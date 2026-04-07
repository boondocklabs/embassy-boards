use embassy_stm32::{hsem::HardwareSemaphoreChannel, peripherals::HSEM};

use super::{PushError, SharedQueue};

pub struct Sender<'a, T, const N: usize> {
    queue: &'a SharedQueue<T, N>,
    hsem_channel: HardwareSemaphoreChannel<'a, HSEM>,
}

impl<'a, T, const N: usize> Sender<'a, T, N> {
    pub(super) fn new(
        queue: &'a SharedQueue<T, N>,
        hsem_channel: HardwareSemaphoreChannel<'a, HSEM>,
    ) -> Self {
        Self {
            queue,
            hsem_channel,
        }
    }

    /// Send a message to the other core
    pub async fn send(&mut self, value: T) -> Result<(), PushError> {
        self.queue.push(value)?;
        self.hsem_channel.blocking_notify();
        Ok(())
    }
}
