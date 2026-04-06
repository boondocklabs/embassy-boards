#[cfg(feature = "touch")]
pub mod touch;

#[cfg(feature = "terminal")]
pub mod terminal;

/// Board driver types
pub trait BoardDrivers {
    /// Touchscreen driver
    #[cfg(feature = "touch")]
    type Touch;

    #[cfg(feature = "terminal")]
    type Terminal;
}
