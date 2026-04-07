//! STM32H747I-DISCO CPU0 (Cortex-M7)
//!
//! The primary core initializes and provides device handles for:
//! * 32MB SDRAM
//! * Graphics framebuffers and texture buffers in SDRAM
//! * Remaining SDRAM used as heap with write back D-cache
//! * DSI display
//! * DMA2D
//! * Touchscreen
//! * SDMMC
//! * 128MB QSPI Flash

const TOUCH_ADDR: u8 = 0x38;

extern crate alloc;

use crate::bsp::stm32::runtime::dualcore::Mailbox;
use crate::bsp::stm32::runtime::mpu::init_mpu;
use crate::bsp::stm32::shared_queue::{Receiver, Sender};
use crate::bsp::stm32::stm32h747i_disco::{Stm32h747iCm7Memory, cm7::display::init_display};
use crate::drivers::BoardDrivers;
use crate::drivers::lcd::panel::Panel;
use crate::drivers::terminal::RenderServer;
use crate::drivers::touch::Ft5316;
use crate::memory::BoardMemory;
use core::slice;

use crate::bsp::stm32::runtime::SHARED_DATA;
use cortex_m::Peripherals;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Pull;
use embassy_stm32::hsem::{HardwareSemaphore, HardwareSemaphoreChannel};
use embassy_stm32::i2c::I2c;
use embassy_stm32::{
    Config, bind_interrupts, dma,
    dma2d::{self, Dma2d, PixelFormat},
    dsihost::{self, DsiHost},
    exti,
    fmc::Fmc,
    gpio::Output,
    hsem, i2c, interrupt,
    ltdc::{self, Ltdc},
    mode::Async,
    peripherals,
    qspi::{self, Qspi},
    rcc::{DsiHostPllConfig, DsiPllInput, DsiPllOutput, Hse, Pll},
    sdmmc::{self, Sdmmc},
    time::Hertz,
};
use embassy_time::{Delay, Timer};
use ratatui::Terminal;
use rtt_target::rtt_init;
use stm32_fmc::{Sdram, devices::is42s32800g_6::Is42s32800g};

use super::Board;
use crate::BoardConfig;
use crate::display::framebuffer::Framebuffer;
use crate::display::texture::Texture;

use embedded_alloc::LlffHeap as Heap;

bind_interrupts!(
    struct Irqs {
        HSEM1 => hsem::HardwareSemaphoreInterruptHandler<peripherals::HSEM>;
        I2C4_EV => i2c::EventInterruptHandler<peripherals::I2C4>;
        I2C4_ER => i2c::ErrorInterruptHandler<peripherals::I2C4>;
        BDMA_CHANNEL0 => dma::InterruptHandler<peripherals::BDMA_CH0>;
        BDMA_CHANNEL1 => dma::InterruptHandler<peripherals::BDMA_CH1>;

        LTDC => ltdc::InterruptHandler<peripherals::LTDC>;
        DSI => dsihost::InterruptHandler<peripherals::DSIHOST>;
        DMA2D => dma2d::InterruptHandler<peripherals::DMA2D>;
        EXTI0 => exti::InterruptHandler<interrupt::typelevel::EXTI0>;
        EXTI1 => exti::InterruptHandler<interrupt::typelevel::EXTI1>;
        EXTI2 => exti::InterruptHandler<interrupt::typelevel::EXTI2>;
        EXTI3 => exti::InterruptHandler<interrupt::typelevel::EXTI3>;
        EXTI4 => exti::InterruptHandler<interrupt::typelevel::EXTI4>;
        EXTI9_5 => exti::InterruptHandler<interrupt::typelevel::EXTI9_5>;
        EXTI15_10 => exti::InterruptHandler<interrupt::typelevel::EXTI15_10>;
        SDMMC1 => sdmmc::InterruptHandler<peripherals::SDMMC1>;
        QUADSPI => qspi::InterruptHandler<embassy_stm32::peripherals::QUADSPI>;
        MDMA => dma::InterruptHandler<embassy_stm32::peripherals::MDMA_CH0>;

    }
);

pub struct Devices<M: 'static> {
    pub dsi: DsiHost<'static, peripherals::DSIHOST>,

    #[cfg(not(feature = "terminal"))]
    pub buffers: Buffers,

    #[cfg(not(feature = "terminal"))]
    pub ltdc: Ltdc<'static, peripherals::LTDC, ltdc::DSI>,

    #[cfg(not(feature = "terminal"))]
    pub dma2d: Dma2d<'static, peripherals::DMA2D>,

    #[cfg(feature = "terminal")]
    pub terminal: <Board<M> as BoardDrivers>::Terminal,

    pub sdmmc: Sdmmc<'static>,
    pub qspi: Qspi<'static, peripherals::QUADSPI, Async>,
    pub touch: <Board<M> as BoardDrivers>::Touch,

    _mailbox: Mailbox<M, 128>,

    /// Sender to send messages to secdonary CM4 core
    pub sender: <Board<M> as BoardDrivers>::Sender,

    /// Receiver to receive messages from secondary CM4 core
    pub receiver: <Board<M> as BoardDrivers>::Receiver,
}

#[global_allocator]
static HEAP: Heap = Heap::empty();

/// SDRAM Buffers
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Buffers {
    pub fb0: Framebuffer<480, 800>,
    pub fb1: Framebuffer<480, 800>,
    pub font_texture: Texture,
}

impl<M: 'static> BoardDrivers for Board<M> {
    type Touch = Ft5316<I2c<'static, Async, i2c::Master>, TOUCH_ADDR>;
    type Terminal = Terminal<RenderServer<ltdc::DSI, 480, 800>>;
    type Sender = Sender<'static, M, 128>;
    type Receiver = Receiver<'static, M, 128>;
}

impl<M: 'static> BoardConfig for Board<M> {
    const NAME: &str = "STM32H747i-DISCO CPU0 (Cortex-M7)";
    const VENDOR: &str = "ST";

    type Devices = Devices<M>;
    type Layout = Stm32h747iCm7Memory;
    type Message = M;

    async fn init() -> Self::Devices {
        let channels = rtt_init! {
            up: {
                0: {
                    size: 1024,
                    name: "defmt",
                }
                1: {
                    size: 1024,
                    name: "core2",
                }
            }
            section_cb: ".rtt"
        };

        rtt_target::set_defmt_channel(channels.up.0);

        let mut cp = Peripherals::take().unwrap();
        init_mpu::<Board<M>>(&mut cp);

        // DSI PLL configuration for 500MHz PHY clock.
        // The PLL input is hardwired to HSE, which on the STM32G747i-DISCO is 25MHz
        // VCO = 25 / idiv * 2 * ndiv = 25 / 5 * 2 * 100 = 1GHz
        // PLL_DSI = VCO / 2 / odiv = 1GHz / 2 / 1 = 500Mhz
        let dsi_pll = DsiHostPllConfig::new(100, DsiPllInput::Div5, DsiPllOutput::Div1);

        let mut config = Config::default();
        config.rcc.supply_config = embassy_stm32::rcc::SupplyConfig::DirectSMPS;
        config.rcc.voltage_scale = embassy_stm32::rcc::VoltageScale::Scale0;
        config.rcc.sys = embassy_stm32::rcc::Sysclk::PLL1_P;
        config.rcc.hse = Some(Hse {
            freq: Hertz::mhz(25),
            mode: embassy_stm32::rcc::HseMode::Bypass,
        });
        config.rcc.pll1 = Some(Pll {
            source: embassy_stm32::rcc::PllSource::HSE,
            prediv: embassy_stm32::rcc::PllPreDiv::DIV5,
            mul: embassy_stm32::rcc::PllMul::MUL192,
            divp: Some(embassy_stm32::rcc::PllDiv::DIV2),
            divq: Some(embassy_stm32::rcc::PllDiv::DIV8), // 120MHz
            divr: None,
        });

        config.rcc.pll2 = Some(Pll {
            source: embassy_stm32::rcc::PllSource::HSE,
            prediv: embassy_stm32::rcc::PllPreDiv::DIV5,
            mul: embassy_stm32::rcc::PllMul::MUL120,
            divp: Some(embassy_stm32::rcc::PllDiv::DIV3),
            divq: Some(embassy_stm32::rcc::PllDiv::DIV2),
            divr: Some(embassy_stm32::rcc::PllDiv::DIV3), // 200MHz SDRAM clock
        });

        config.rcc.pll3 = Some(Pll {
            source: embassy_stm32::rcc::PllSource::HSE,
            prediv: embassy_stm32::rcc::PllPreDiv::DIV5,
            mul: embassy_stm32::rcc::PllMul::MUL100,
            divp: Some(embassy_stm32::rcc::PllDiv::DIV2),
            divq: Some(embassy_stm32::rcc::PllDiv::DIV2),

            // LTDC is clocked from PLL3R
            divr: Some(embassy_stm32::rcc::PllDiv::DIV20),
        });
        config.rcc.d1c_pre = embassy_stm32::rcc::AHBPrescaler::DIV1;
        config.rcc.ahb_pre = embassy_stm32::rcc::AHBPrescaler::DIV2;
        config.rcc.apb1_pre = embassy_stm32::rcc::APBPrescaler::DIV2;
        config.rcc.apb2_pre = embassy_stm32::rcc::APBPrescaler::DIV2;
        config.rcc.apb3_pre = embassy_stm32::rcc::APBPrescaler::DIV2;
        config.rcc.apb4_pre = embassy_stm32::rcc::APBPrescaler::DIV2;
        config.rcc.hsi48 = Some(Default::default());
        config.rcc.csi = true;
        config.rcc.mux.fmcsel = embassy_stm32::rcc::mux::Fmcsel::PLL2_R; // Use PLL2_R for SDRAM (100MHz)
        config.rcc.mux.dsisel = embassy_stm32::rcc::mux::Dsisel::DSI_PHY_DIV_8; // Use DSI PHY / 8 for byte lane clock (62.5MHz lane byte clock)
        config.rcc.mux.quadspisel = embassy_stm32::rcc::mux::Fmcsel::PLL1_Q;
        config.rcc.dsi = Some(dsi_pll);

        let p = embassy_stm32::init_primary(config, &SHARED_DATA);

        let sdram = Fmc::sdram_a12bits_d32bits_4banks_bank2(
            p.FMC,
            // A0-A11
            p.PF0,
            p.PF1,
            p.PF2,
            p.PF3,
            p.PF4,
            p.PF5,
            p.PF12,
            p.PF13,
            p.PF14,
            p.PF15,
            p.PG0,
            p.PG1,
            // BA0-BA1
            p.PG4,
            p.PG5,
            // D0-D31
            p.PD14,
            p.PD15,
            p.PD0,
            p.PD1,
            p.PE7,
            p.PE8,
            p.PE9,
            p.PE10,
            p.PE11,
            p.PE12,
            p.PE13,
            p.PE14,
            p.PE15,
            p.PD8,
            p.PD9,
            p.PD10,
            p.PH8,
            p.PH9,
            p.PH10,
            p.PH11,
            p.PH12,
            p.PH13,
            p.PH14,
            p.PH15,
            p.PI0,
            p.PI1,
            p.PI2,
            p.PI3,
            p.PI6,
            p.PI7,
            p.PI9,
            p.PI10,
            // NBL0 - NBL3
            p.PE0,
            p.PE1,
            p.PI4,
            p.PI5,  // Control signals
            p.PH7,  // SDCKE1
            p.PG8,  // SDCLK
            p.PG15, // SDNCAS
            p.PH6,  // SDNE1 (!CS)
            p.PF11, // SDRAS
            p.PH5,  // SDNWE
            stm32_fmc::devices::is42s32800g_6::Is42s32800g {},
        );

        let buffers = init_memory::<M>(sdram);

        #[cfg(feature = "defmt")]
        defmt::info!("{}", buffers);

        // Reset display
        let mut dsi_reset = Output::new(
            p.PG3,
            embassy_stm32::gpio::Level::Low,
            embassy_stm32::gpio::Speed::Low,
        );
        Timer::after_millis(120).await;
        dsi_reset.set_high();

        // Create DSI host using PJ2 as tearing input
        let mut dsi = DsiHost::new(p.DSIHOST, p.PJ2);
        let mut ltdc = Ltdc::new(p.LTDC);
        let dma2d = Dma2d::new(p.DMA2D, Irqs);

        init_display(&mut dsi, &mut ltdc, &buffers).await;

        let mut i2c4_config = i2c::Config::default();
        i2c4_config.frequency = Hertz::khz(400);

        let i2c4 = I2c::new(
            p.I2C4,
            p.PD12,
            p.PD13,
            p.BDMA_CH0,
            p.BDMA_CH1,
            Irqs,
            i2c4_config,
        );

        let touch_int = ExtiInput::new(p.PK7, p.EXTI7, Pull::Up, Irqs);
        let touch = Ft5316::<_, TOUCH_ADDR>::new(i2c4, touch_int);

        let sdmmc = Sdmmc::new_4bit(
            p.SDMMC1,
            Irqs,
            p.PC12, // CLK
            p.PD2,  // CMD
            p.PC8,  // D0
            p.PC9,  // D1
            p.PC10, // D2
            p.PC11, // D3
            Default::default(),
        );

        let mut qspi_config = qspi::Config::default();

        qspi_config.memory_size = qspi::enums::MemorySize::_64MiB;
        qspi_config.address_size = qspi::enums::AddressSize::_24bit;
        qspi_config.prescaler = 1;
        qspi_config.cs_high_time = qspi::enums::ChipSelectHighTime::_2Cycle;
        qspi_config.fifo_threshold = qspi::enums::FIFOThresholdLevel::_1Bytes;

        let qspi = Qspi::new_dual_bank(
            p.QUADSPI,
            p.PD11, // BK1 IO0
            p.PF9,  // BK1 IO1
            p.PF7,  // BK1 IO2
            p.PF6,  // BK1 IO3
            p.PH2,  // BK2 IO0
            p.PH3,  // BK2 IO1
            p.PG9,  // BK2 IO2
            p.PG14, // BK2 IO3
            p.PB2,  // CLK
            p.PG6,  // NSS
            p.MDMA_CH0,
            Irqs,
            qspi_config,
        );

        let backend = RenderServer::new(
            dma2d,
            ltdc,
            buffers.fb0.as_buffer2d(PixelFormat::Argb8888),
            buffers.fb1.as_buffer2d(PixelFormat::Argb8888),
            buffers.font_texture,
        )
        .await;

        let terminal = Terminal::new(backend).unwrap();

        let [hsem1, hsem2, mut hsem3, hsem4, hsem5, hsem6] =
            HardwareSemaphore::new(p.HSEM, Irqs).split();

        let mailbox = Mailbox::new(true);
        let sender = unsafe {
            mailbox
                .cm7_to_cm4
                .as_ptr()
                .as_ref()
                .expect("CM7 to CM4 mailbox")
                .sender(hsem1)
        };

        let receiver = unsafe {
            mailbox
                .cm4_to_cm7
                .as_ptr()
                .as_ref()
                .expect("CM4 to CM7 mailbox")
                .receiver(hsem2)
        };

        hsem3.blocking_notify();

        Devices {
            dsi,
            terminal,
            sdmmc,
            qspi,
            touch,
            _mailbox: mailbox,
            sender,
            receiver,
        }
    }

    fn heap_free() -> usize {
        HEAP.free()
    }

    fn heap_used() -> usize {
        HEAP.used()
    }

    fn heap_size() -> usize {
        <Board<M> as BoardConfig>::Layout::MEMORY
            .section("SDRAM", "heap")
            .expect("heap section")
            .length
    }
}

/// Initialize memory
pub fn init_memory<M: 'static>(
    mut sdram: Sdram<Fmc<'_, peripherals::FMC>, Is42s32800g>,
) -> Buffers {
    let _sdram_region = <Board<M> as BoardConfig>::Layout::MEMORY
        .region("SDRAM")
        .unwrap();

    sdram.init(&mut Delay);

    let qspi_region = <Board<M> as BoardConfig>::Layout::MEMORY
        .region("QSPI")
        .unwrap();

    let _qspi_base =
        unsafe { slice::from_raw_parts_mut(qspi_region.origin as *mut u8, qspi_region.length) };

    let fb0_section = <Board<M> as BoardConfig>::Layout::MEMORY
        .section("SDRAM", "fb0")
        .unwrap();
    let fb1_section = <Board<M> as BoardConfig>::Layout::MEMORY
        .section("SDRAM", "fb1")
        .unwrap();

    let fb0 = unsafe {
        Framebuffer::new(
            fb0_section.origin as *mut u32,
            fb0_section.length / size_of::<u32>(),
        )
    };
    let fb1 = unsafe {
        Framebuffer::new(
            fb1_section.origin as *mut u32,
            fb1_section.length / size_of::<u32>(),
        )
    };

    let tex_section = <Board<M> as BoardConfig>::Layout::MEMORY
        .section("SDRAM", "tex")
        .unwrap();

    let font_texture =
        unsafe { Texture::new(tex_section.origin as *mut u8, 1600, 200, PixelFormat::A8) };

    let heap_section = <Board<M> as BoardConfig>::Layout::MEMORY
        .section("SDRAM", "heap")
        .unwrap();

    unsafe {
        HEAP.init(heap_section.origin, heap_section.length);
    }

    Buffers {
        fb0,
        fb1,
        font_texture,
    }
}
