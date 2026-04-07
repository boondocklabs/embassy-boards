use core::mem::MaybeUninit;
use embassy_stm32::SharedData;
use panic_probe as _;

pub mod mpu;

/// SharedData used for synchronizing dual core boards
#[unsafe(link_section = ".shared_data")]
#[unsafe(no_mangle)]
pub static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[cfg(feature = "dual-core")]
pub mod dualcore;
