use embassy_stm32::{hsem::HardwareSemaphoreChannel, peripherals::HSEM};
use embassy_time::{Duration, WithTimeout};

use super::SharedQueue;

pub struct Receiver<'a, T, const N: usize> {
    queue: &'a SharedQueue<T, N>,
    hsem_channel: HardwareSemaphoreChannel<'a, HSEM>,
}

impl<'a, T, const N: usize> Receiver<'a, T, N> {
    pub(super) fn new(
        queue: &'a SharedQueue<T, N>,
        hsem_channel: HardwareSemaphoreChannel<'a, HSEM>,
    ) -> Self {
        Self {
            queue,
            hsem_channel,
        }
    }

    pub async fn recv(&mut self) -> Option<T> {
        while self.queue.is_empty() {
            self.hsem_channel.listen().await;
        }
        self.queue.pop()
    }
}
