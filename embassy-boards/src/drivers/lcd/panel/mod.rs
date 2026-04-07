use embedded_hal::{digital::OutputPin, spi::SpiBus};

#[cfg(panel = "nt35510")]
mod nt35510;
#[cfg(panel = "nt35510")]
pub use nt35510::Glass as Panel;

#[cfg(panel = "ili9341")]
mod ili9341;
#[cfg(panel = "ili9341")]
pub use ili9341::Ili9341 as Panel;

#[allow(async_fn_in_trait)]
pub trait SpiPanel {
    /// Initialize the panel
    async fn init<Bus: SpiBus, CS: OutputPin, DC: OutputPin>(
        bus: &mut Bus,
        cs: &mut CS,
        dc: &mut DC,
    );
}
