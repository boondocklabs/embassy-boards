#![allow(unexpected_cfgs)]

#[cfg(feature = "touch")]
pub mod touch;

#[cfg(feature = "terminal")]
pub mod terminal;

#[cfg(feature = "led")]
pub mod led;

#[cfg(feature = "pmod")]
pub mod pmod;

#[cfg(lcd)]
pub mod lcd;

/// Board driver types
pub trait BoardDrivers {
    /// Touchscreen driver
    #[cfg(feature = "touch")]
    type Touch;

    #[cfg(feature = "terminal")]
    type Terminal;

    #[cfg(feature = "led")]
    type Led;

    #[cfg(feature = "crypto")]
    type Crypto;

    #[cfg(feature = "hash")]
    type Hash;

    #[cfg(feature = "pmod")]
    type Pmod;

    //#[cfg(lcd)]
    //type Lcd;

    #[cfg(feature = "dual-core")]
    type Sender;

    #[cfg(feature = "dual-core")]
    type Receiver;

    #[cfg(feature = "net")]
    type Network;
}
