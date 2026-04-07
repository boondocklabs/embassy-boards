//! PMOD

use embassy_stm32::{
    exti::ExtiInput,
    gpio::Output,
    mode::Async,
    spi::{Spi, mode::Master},
};
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;

pub struct Pmod {
    pub bus: ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>,
    pub int: ExtiInput<'static, Async>,
    pub reset: Output<'static>,
}

impl Pmod {
    pub fn new(
        bus: ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>,
        int: ExtiInput<'static, Async>,
        reset: Output<'static>,
    ) -> Self {
        Self { bus, int, reset }
    }
}
