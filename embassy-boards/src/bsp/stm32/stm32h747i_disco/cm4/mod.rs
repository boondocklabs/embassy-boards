use crate::bsp::stm32::runtime::dualcore::Mailbox;
use core::marker::PhantomData;

use crate::{
    bsp::stm32::{
        runtime::SHARED_DATA,
        shared_queue::{Receiver, Sender},
    },
    drivers::{BoardDrivers, led::Led},
};
use embassy_stm32::{
    bind_interrupts, dma, exti,
    gpio::{Level, Output, Speed},
    hsem::{self, HardwareSemaphore},
    interrupt,
    mode::Async,
    peripherals,
};

use crate::BoardConfig;

bind_interrupts!(
    struct Irqs {
        HSEM2 => hsem::HardwareSemaphoreInterruptHandler<peripherals::HSEM>;
        EXTI9_5 => exti::InterruptHandler<interrupt::typelevel::EXTI9_5>;
        EXTI4 => exti::InterruptHandler<interrupt::typelevel::EXTI4>;
        EXTI3 => exti::InterruptHandler<interrupt::typelevel::EXTI3>;
    }
);

pub struct Board<M> {
    _message: PhantomData<M>,
}

impl<M> Board<M> {}

/// Devices returned from board init
pub struct Devices<M: 'static> {
    pub led: <Board<M> as BoardDrivers>::Led,

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
}

impl<M: 'static> BoardConfig for Board<M> {
    const NAME: &str = "STM32H747i-DISCO CPU1 (Cortex-M4)";
    const VENDOR: &str = "ST";
    type Devices = Devices<M>;
    type Layout = super::Stm32h747iCm4Memory;
    type Message = M;

    async fn init() -> Devices<M> {
        let p = embassy_stm32::init_secondary(&SHARED_DATA);

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

        Devices {
            led,
            _mailbox: mailbox,
            sender,
            receiver,
        }
    }
}
