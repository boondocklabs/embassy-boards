//! STM32F429I-DISCO Board

mod memory;
pub struct Memory {}

#[cfg(feature = "_runtime")]
mod board;
#[cfg(feature = "_runtime")]
pub struct Board {}
