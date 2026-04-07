//! Dual core shared SPSC queue with hardware semaphore notifications
//! The shared queue must be placed at a stable address accessible from both cores.

use core::{cell::UnsafeCell, mem::MaybeUninit, ptr};
use embassy_stm32::{hsem::HardwareSemaphoreChannel, peripherals::HSEM};

mod receiver;
mod sender;

pub use receiver::Receiver;
pub use sender::Sender;

#[repr(C)]
pub struct SharedQueue<T, const N: usize> {
    head: UnsafeCell<u32>,
    tail: UnsafeCell<u32>,
    slots: UnsafeCell<[MaybeUninit<T>; N]>,
}

unsafe impl<T: Send, const N: usize> Sync for SharedQueue<T, N> {}

#[allow(unused)]
impl<T, const N: usize> SharedQueue<T, N> {
    pub const fn new() -> Self {
        Self {
            head: UnsafeCell::new(0),
            tail: UnsafeCell::new(0),
            slots: UnsafeCell::new([const { MaybeUninit::uninit() }; N]),
        }
    }

    #[inline]
    unsafe fn head(&self) -> u32 {
        unsafe { ptr::read_volatile(self.head.get()) }
    }

    #[inline]
    unsafe fn tail(&self) -> u32 {
        unsafe { ptr::read_volatile(self.tail.get()) }
    }

    #[inline]
    unsafe fn set_head(&self, v: u32) {
        unsafe {
            ptr::write_volatile(self.head.get(), v);
        }
    }

    #[inline]
    unsafe fn set_tail(&self, v: u32) {
        unsafe {
            ptr::write_volatile(self.tail.get(), v);
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        unsafe { self.head() == self.tail() }
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        unsafe {
            let head = self.head();
            let next = (head + 1) % N as u32;
            next == self.tail()
        }
    }

    pub fn push(&self, value: T) -> Result<(), PushError> {
        unsafe {
            let head = self.head();
            let tail = self.tail();
            let next = (head + 1) % N as u32;

            if next == tail {
                return Err(PushError::Full);
            }

            let slots = &mut *self.slots.get();

            slots[head as usize].write(value);

            // Ensure payload is visible before publishing head.
            cortex_m::asm::dmb();

            self.set_head(next);
            Ok(())
        }
    }

    pub fn pop(&self) -> Option<T> {
        unsafe {
            let head = self.head();
            let tail = self.tail();

            if head == tail {
                return None;
            }

            // Ensure we see slot contents after observing head.
            cortex_m::asm::dmb();

            let slots = &mut *self.slots.get();
            let value = slots[tail as usize].assume_init_read();

            let next = (tail + 1) % N as u32;
            self.set_tail(next);
            Some(value)
        }
    }

    pub fn sender<'a>(
        &'a self,
        hsem_channel: HardwareSemaphoreChannel<'a, HSEM>,
    ) -> Sender<'a, T, N> {
        Sender::new(self, hsem_channel)
    }

    pub fn receiver<'a>(
        &'a self,
        hsem_channel: HardwareSemaphoreChannel<'a, HSEM>,
    ) -> Receiver<'a, T, N> {
        Receiver::new(self, hsem_channel)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PushError {
    Full,
}
