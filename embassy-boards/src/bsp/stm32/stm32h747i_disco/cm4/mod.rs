use crate::memory::BoardMemory;
use crate::{bsp::stm32::runtime::dualcore::Mailbox, drivers::pmod::Pmod};
use core::marker::PhantomData;

use crate::{
    bsp::stm32::{
        runtime::SHARED_DATA,
        shared_queue::{Receiver, Sender},
    },
    drivers::{BoardDrivers, led::Led},
};
use embassy_stm32::{
    bind_interrupts, dma,
    exti::{self, ExtiInput},
    gpio::{Level, Output, Pull, Speed},
    hsem::{self, HardwareSemaphore},
    interrupt, peripherals,
    spi::{self, Spi},
};
use embedded_alloc::LlffHeap as Heap;
use embedded_hal_bus::spi::ExclusiveDevice;

use crate::BoardConfig;

bind_interrupts!(
    struct Irqs {
        HSEM2 => hsem::HardwareSemaphoreInterruptHandler<peripherals::HSEM>;
        DMA1_STREAM3 => dma::InterruptHandler<peripherals::DMA1_CH3>;
        DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
        EXTI9_5 => exti::InterruptHandler<interrupt::typelevel::EXTI9_5>;
        EXTI4 => exti::InterruptHandler<interrupt::typelevel::EXTI4>;
        EXTI3 => exti::InterruptHandler<interrupt::typelevel::EXTI3>;
    }
);

#[global_allocator]
static HEAP: Heap = Heap::empty();

pub struct Board<M> {
    _message: PhantomData<M>,
}

impl<M> Board<M> {}

/// Devices returned from board init
pub struct Devices<M: 'static> {
    pub led: <Board<M> as BoardDrivers>::Led,

    #[cfg(feature = "pmod")]
    pub pmod: <Board<M> as BoardDrivers>::Pmod,

    /// Private internal Mailbox for Sender and Receiver
    _mailbox: Mailbox<M, 128>,

    pub sender: <Board<M> as BoardDrivers>::Sender,
    pub receiver: <Board<M> as BoardDrivers>::Receiver,
}

/// Board driver types
impl<M: 'static> BoardDrivers for Board<M> {
    type Led = Led<4, true>;
    type Sender = Sender<'static, M, 128>;
    type Receiver = Receiver<'static, M, 128>;
    type Pmod = Pmod;
}

impl<M: 'static> BoardConfig for Board<M> {
    const NAME: &str = "STM32H747i-DISCO CPU1 (Cortex-M4)";
    const VENDOR: &str = "ST";
    type Devices = Devices<M>;
    type Layout = super::Stm32h747iCm4Memory;
    type Message = M;

    async fn init() -> Devices<M> {
        let p = embassy_stm32::init_secondary(&SHARED_DATA);

        // Initialize heap on SRAM2
        let heap_region = <Board<M> as BoardConfig>::Layout::MEMORY
            .region("SRAM2")
            .unwrap();

        unsafe {
            HEAP.init(heap_region.origin, heap_region.length);
        }

        let led1 = Output::new(p.PI12, Level::High, Speed::High);
        let led2 = Output::new(p.PI13, Level::High, Speed::High);
        let led3 = Output::new(p.PI14, Level::High, Speed::High);
        let led4 = Output::new(p.PI15, Level::High, Speed::High);

        let led = Led {
            pins: [led1, led2, led3, led4],
        };

        let [hsem1, hsem2, mut hsem3, _hsem4, _hsem5, _hsem6] =
            HardwareSemaphore::new(p.HSEM, Irqs).split();

        // Wait for the primary core to initialize the mailbox, signalled on hsem3
        hsem3.blocking_listen();

        let mailbox = Mailbox::new(false);
        let sender = unsafe {
            mailbox
                .cm4_to_cm7
                .as_ptr()
                .as_ref()
                .expect("CM4 to CM7 mailbox")
                .sender(hsem2)
        };

        let receiver = unsafe {
            mailbox
                .cm7_to_cm4
                .as_ptr()
                .as_ref()
                .expect("CM7 to CM4 mailbox")
                .receiver(hsem1)
        };

        #[cfg(feature = "pmod")]
        let pmod = {
            use embassy_stm32::time::Hertz;
            use embassy_time::Delay;

            // TODO: This should be configurable by a caller
            let mut spi_config = spi::Config::default();
            spi_config.frequency = Hertz::mhz(10);
            spi_config.gpio_speed = embassy_stm32::gpio::Speed::Low;
            spi_config.mode = spi::MODE_0;
            spi_config.nss_output_disable = true;
            spi_config.bit_order = spi::BitOrder::MsbFirst;

            let pmod_spi = Spi::new(
                p.SPI2, p.PA12, p.PC3, p.PC2, p.DMA1_CH3, p.DMA1_CH4, Irqs, spi_config,
            );

            let pmod_int = ExtiInput::new(p.PC6, p.EXTI6, Pull::None, Irqs);
            let pmod_cs = Output::new(
                p.PA11,
                embassy_stm32::gpio::Level::High,
                embassy_stm32::gpio::Speed::High,
            );
            let pmod_reset = Output::new(
                p.PJ13,
                embassy_stm32::gpio::Level::Low,
                embassy_stm32::gpio::Speed::Low,
            );
            let bus = ExclusiveDevice::new(pmod_spi, pmod_cs, Delay).unwrap();
            Pmod::new(bus, pmod_int, pmod_reset)
        };

        Devices {
            led,
            pmod,
            _mailbox: mailbox,
            sender,
            receiver,
        }
    }
}
