#[cfg(feature = "dual-core")]
use core::mem::MaybeUninit;
#[cfg(feature = "dual-core")]
use embassy_stm32::SharedData;
use panic_probe as _;

pub mod mpu;

#[cfg(feature = "dual-core")]
#[unsafe(link_section = ".shared_data")]
#[unsafe(no_mangle)]
pub static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[cfg(feature = "dual-core")]
pub mod dualcore;
