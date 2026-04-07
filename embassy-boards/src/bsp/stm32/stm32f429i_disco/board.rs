#![allow(static_mut_refs)]

#[cfg(feature = "terminal")]
use crate::drivers::terminal::RenderServer;
use crate::memory::BoardMemory;
use defmt::error;
use embassy_stm32::Config;
use embassy_stm32::bind_interrupts;
use embassy_stm32::dma2d;
use embassy_stm32::dma2d::Buffer2D;
use embassy_stm32::dma2d::Dma2d;
use embassy_stm32::dma2d::PixelFormat;
use embassy_stm32::fmc::Fmc;
use embassy_stm32::gpio::Level;
use embassy_stm32::gpio::Output;
use embassy_stm32::gpio::Speed;
use embassy_stm32::ltdc;
use embassy_stm32::ltdc::Ltdc;
use embassy_stm32::ltdc::LtdcConfiguration;
use embassy_stm32::ltdc::LtdcLayer;
use embassy_stm32::ltdc::LtdcLayerConfig;
use embassy_stm32::peripherals;
use embassy_stm32::spi;
use embassy_stm32::time::Hertz;
use embassy_time::Delay;
#[cfg(feature = "terminal")]
use ratatui::Terminal;
use rtt_target::rtt_init;

use super::Board;
use super::Memory;

use crate::display::texture::Texture;
use crate::drivers::lcd::panel::Panel;
use crate::drivers::lcd::panel::SpiPanel;
use crate::drivers::led::Led;
use crate::{BoardConfig, drivers::BoardDrivers};
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

extern crate alloc;

bind_interrupts!(
    struct Irqs {
        LTDC => ltdc::InterruptHandler<peripherals::LTDC>;
        DMA2D => dma2d::InterruptHandler<peripherals::DMA2D>;
    }
);

const WIDTH: u16 = 240;
const HEIGHT: u16 = 320;
const PIXELS: usize = WIDTH as usize * HEIGHT as usize;

#[unsafe(link_section = ".fb0")]
static mut FB0: [u32; PIXELS] = [0; PIXELS];

#[unsafe(link_section = ".fb1")]
static mut FB1: [u32; PIXELS] = [0; PIXELS];

/// Board driver types
impl BoardDrivers for Board {
    type Led = Led<2, false>;
    #[cfg(feature = "terminal")]
    type Terminal = Terminal<RenderServer<ltdc::Rgb666, WIDTH, HEIGHT>>;
}

pub struct Devices {
    pub led: <Board as BoardDrivers>::Led,
    #[cfg(feature = "terminal")]
    pub terminal: <Board as BoardDrivers>::Terminal,
}

impl BoardConfig for Board {
    const NAME: &str = "STM32F429I-DISCO";
    const VENDOR: &str = "ST";

    type Layout = Memory;
    type Devices = Devices;

    async fn init() -> Self::Devices {
        let channels = rtt_init! {
            up: {
                0: {
                    size: 1024,
                    name: "defmt",
                }
            }
            section_cb: ".rtt"
        };

        rtt_target::set_defmt_channel(channels.up.0);

        let mut config = Config::default();
        {
            use embassy_stm32::rcc::*;
            config.rcc.hse = Some(Hse {
                freq: Hertz(8_000_000),
                mode: HseMode::Oscillator,
            });
            config.rcc.pll_src = PllSource::HSE;
            config.rcc.pll = Some(Pll {
                prediv: PllPreDiv::DIV8,
                mul: PllMul::MUL360,
                divp: Some(PllPDiv::DIV2), // 8mhz / 8 * 360 / 2 = 180Mhz.
                divq: Some(PllQDiv::DIV7),
                divr: None,
            });
            config.rcc.ahb_pre = AHBPrescaler::DIV1;
            config.rcc.apb1_pre = APBPrescaler::DIV4;
            config.rcc.apb2_pre = APBPrescaler::DIV2;
            config.rcc.sys = Sysclk::PLL1_P;

            // Configure SAI PLL for LTDC
            config.rcc.pllsai = Some(Pll {
                prediv: PllPreDiv::DIV8,
                mul: PllMul::MUL192,
                divp: None,
                divq: None,
                divr: Some(PllRDiv::DIV4), // 8mhz / 8 * 192 / 4 = 48MHz
            });

            // Set LCD dot clock divisor to 8
            // F = PLLSAI.R / 8 = 48 / 8 = 6MHz
            // F = PLLSAI.R / 8 = 48 / 4 = 12MHz
            config.rcc.lcd_div = Some(ltdc::LcdClockDiv::Div4);
        }
        let p = embassy_stm32::init(config);

        let mut fmc = Fmc::sdram_a12bits_d16bits_4banks_bank2(
            p.FMC,  // A pins
            p.PF0,  // A0
            p.PF1,  // A1
            p.PF2,  // A2
            p.PF3,  // A3
            p.PF4,  // A4
            p.PF5,  // A5
            p.PF12, // A6
            p.PF13, // A7
            p.PF14, // A8
            p.PF15, // A9
            p.PG0,  // A10
            p.PG1,  // A11
            // BA
            p.PG4, // BA0
            p.PG5, // BA1
            // D pins
            p.PD14, // D0
            p.PD15, // D1
            p.PD0,  // D2
            p.PD1,  // D3
            p.PE7,  // D4
            p.PE8,  // D5
            p.PE9,  // D6
            p.PE10, // D7
            p.PE11, // D8
            p.PE12, // D9
            p.PE13, // D10
            p.PE14, // D11
            p.PE15, // D12
            p.PD8,  // D13
            p.PD9,  // D14
            p.PD10, // D15
            // control
            p.PE0,  // NBL0
            p.PE1,  // NBL1
            p.PB5,  // SDCKE1
            p.PG8,  // SDCLK
            p.PG15, // SDNCAS
            p.PB6,  // SDNE1
            p.PF11, //
            p.PC0,  //
            stm32_fmc::devices::is42s16400j_7::Is42s16400j {},
        );

        let _base = fmc.init(&mut Delay);

        let heap_section = <Board as BoardConfig>::Layout::MEMORY
            .section("SDRAM", "heap")
            .unwrap();

        unsafe {
            HEAP.init(heap_section.origin, heap_section.length);
        }

        let led1 = Output::new(
            p.PG13,
            embassy_stm32::gpio::Level::High,
            embassy_stm32::gpio::Speed::Low,
        );

        let led2 = Output::new(
            p.PG14,
            embassy_stm32::gpio::Level::High,
            embassy_stm32::gpio::Speed::Low,
        );

        let led = Led { pins: [led1, led2] };

        let mut lcd_cs = Output::new(p.PC2, Level::High, Speed::VeryHigh);
        let mut lcd_dc = Output::new(p.PD13, Level::High, Speed::VeryHigh);

        let mut spi_cfg = spi::Config::default();
        spi_cfg.frequency = Hertz(10_000_000);
        spi_cfg.mode = spi::MODE_0;

        let mut lcd_spi = spi::Spi::new_blocking_txonly(
            p.SPI5, p.PF7, // SCK
            p.PF9, // MOSI
            spi_cfg,
        );

        Panel::init(&mut lcd_spi, &mut lcd_cs, &mut lcd_dc).await;

        let b2 = p.PD6;
        let b3 = p.PG11;
        let b4 = p.PG12;
        let b5 = p.PA3;
        let b6 = p.PB8;
        let b7 = p.PB9;

        let g2 = p.PA6;
        let g3 = p.PG10;
        let g4 = p.PB10;
        let g5 = p.PB11;
        let g6 = p.PC7;
        let g7 = p.PD3;

        let r2 = p.PC10;
        let r3 = p.PB0;
        let r4 = p.PA11;
        let r5 = p.PA12;
        let r6 = p.PB1;
        let r7 = p.PG6;

        let mut ltdc = Ltdc::<_, ltdc::Rgb666>::new_with_pins(
            p.LTDC, Irqs, p.PG7, p.PC6, p.PA4, p.PF10, b2, b3, b4, b5, b6, b7, g2, g3, g4, g5, g6,
            g7, r2, r3, r4, r5, r6, r7,
        );

        ltdc.init(&LtdcConfiguration {
            active_width: WIDTH as u16,
            active_height: HEIGHT as u16,
            h_back_porch: 20,
            h_front_porch: 10,
            v_back_porch: 2,
            v_front_porch: 4,
            h_sync: 10,
            v_sync: 2,
            h_sync_polarity: ltdc::PolarityActive::ActiveLow,
            v_sync_polarity: ltdc::PolarityActive::ActiveLow,
            data_enable_polarity: ltdc::PolarityActive::ActiveLow,
            pixel_clock_polarity: ltdc::PolarityEdge::FallingEdge,
        });

        ltdc.disable();

        ltdc.init_layer(
            &LtdcLayerConfig {
                layer: ltdc::LtdcLayer::Layer1,
                pixel_format: ltdc::PixelFormat::ARGB8888,
                window_x0: 0,
                window_x1: WIDTH as u16,
                window_y0: 0,
                window_y1: HEIGHT as u16,
            },
            None,
        );

        ltdc.init_layer(
            &LtdcLayerConfig {
                layer: ltdc::LtdcLayer::Layer2,
                pixel_format: ltdc::PixelFormat::ARGB8888,
                window_x0: 0,
                window_x1: WIDTH as u16,
                window_y0: 0,
                window_y1: HEIGHT as u16,
            },
            None,
        );

        ltdc.enable();

        ltdc.set_buffer(ltdc::LtdcLayer::Layer1, unsafe { FB0.as_ptr() as *const _ })
            .await
            .unwrap();

        ltdc.set_buffer(ltdc::LtdcLayer::Layer2, unsafe { FB1.as_ptr() as *const _ })
            .await
            .unwrap();

        // Set the framebuffers for each layer
        ltdc.init_buffer(LtdcLayer::Layer2, unsafe { FB0.as_ptr() as *const _ });
        ltdc.init_buffer(LtdcLayer::Layer1, unsafe { FB1.as_ptr() as *const _ });

        // Reload the shadow registers
        ltdc.reload().await.unwrap();

        let mut dma2d = Dma2d::new(p.DMA2D, Irqs);
        let buf0 = Buffer2D::new(
            unsafe { FB0.as_ptr() as *mut u8 },
            dma2d::PixelFormat::Argb8888,
            WIDTH,
            WIDTH,
            HEIGHT,
        );

        let buf1 = Buffer2D::new(
            unsafe { FB1.as_ptr() as *mut u8 },
            dma2d::PixelFormat::Argb8888,
            WIDTH,
            WIDTH,
            HEIGHT,
        );

        if let Err(e) = dma2d
            .fill(&buf0.region(0, 0, WIDTH as u16, HEIGHT as u16), 0)
            .await
        {
            error!("DMA2D {}", e);
        }

        if let Err(e) = dma2d
            .fill(&buf1.region(0, 0, WIDTH as u16, HEIGHT as u16), 0)
            .await
        {
            error!("DMA2D {}", e);
        }

        #[cfg(feature = "terminal")]
        let terminal = {
            let tex_section = <Board as BoardConfig>::Layout::MEMORY
                .section("SDRAM", "tex")
                .unwrap();

            let font_texture =
                unsafe { Texture::new(tex_section.origin as *mut u8, 1600, 200, PixelFormat::A8) };

            let backend = RenderServer::<ltdc::Rgb666, WIDTH, HEIGHT>::new(
                dma2d,
                ltdc,
                buf0,
                buf1,
                font_texture,
            )
            .await;
            Terminal::new(backend).unwrap()
        };

        Devices {
            led,
            #[cfg(feature = "terminal")]
            terminal,
        }
    }

    #[cfg(feature = "heap")]
    fn heap_free() -> usize {
        HEAP.free()
    }

    #[cfg(feature = "heap")]
    fn heap_used() -> usize {
        HEAP.used()
    }

    #[cfg(feature = "heap")]
    fn heap_size() -> usize {
        let heap_section = <Board as BoardConfig>::Layout::MEMORY
            .section("SDRAM", "heap")
            .unwrap();
        heap_section.length
    }
}
