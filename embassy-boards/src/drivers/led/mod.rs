//! LED Driver

use embedded_hal::digital::{PinState, StatefulOutputPin};

//use embassy_stm32::gpio::{Level, Output};

pub struct Led<T: StatefulOutputPin, const N: usize, const INVERTED: bool = true> {
    pub(crate) pins: [T; N],
}

impl<T: StatefulOutputPin, const N: usize, const INVERTED: bool> Led<T, N, INVERTED> {
    pub fn set(&mut self, index: usize, level: bool) {
        if index < N {
            let level = if INVERTED { !level } else { level };

            let state = match level {
                true => PinState::Low,
                false => PinState::High,
            };

            self.pins[index].set_state(state).ok();
        }
    }

    pub fn toggle(&mut self, index: usize) {
        if index < N {
            self.pins[index].toggle().ok();
        }
    }

    pub fn mask(&mut self, mask: u32) {
        for i in 0..N {
            let state = PinState::from((mask >> i) & 1 != 0);
            self.pins[i].set_state(state).ok();
        }
    }
}
