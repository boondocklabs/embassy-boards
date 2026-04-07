#[cfg(feature = "touch")]
pub mod touch;

#[cfg(feature = "terminal")]
pub mod terminal;

#[cfg(feature = "led")]
pub mod led;

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

    #[cfg(feature = "dual-core")]
    type Sender;

    #[cfg(feature = "dual-core")]
    type Receiver;
}
