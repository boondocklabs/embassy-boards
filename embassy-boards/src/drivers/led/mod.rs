//! LED Driver

use embassy_stm32::gpio::{Level, Output};

pub struct Led<const N: usize, const INVERTED: bool = true> {
    pub(crate) pins: [Output<'static>; N],
}

impl<const N: usize, const INVERTED: bool> Led<N, INVERTED> {
    pub fn set(&mut self, index: usize, level: Level) {
        if index < N {
            let level = if INVERTED {
                match level {
                    Level::Low => Level::High,
                    Level::High => Level::Low,
                }
            } else {
                level
            };
            self.pins[index].set_level(level);
        }
    }

    pub fn toggle(&mut self, index: usize) {
        if index < N {
            self.pins[index].toggle();
        }
    }

    pub fn mask(&mut self, mask: u32) {
        for i in 0..N {
            let level = Level::from((mask >> i) & 1 != 0);
            self.set(i, level);
        }
    }
}
