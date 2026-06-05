//! Platform output abstraction.

use thiserror::Error;

#[cfg(all(target_os = "linux"))]
// #[cfg_attr(docsrs, doc(cfg(feature = "x11")))]
pub mod x11;

/// Alias for [`std::result::Result`] with [`PlatformError`] as the error type.
pub type Result<T> = std::result::Result<T, PlatformError>;

/// Error produced by the platform output backend.
#[derive(Debug, Error)]
pub enum PlatformError {
    /// A type-erased error from the active backend.
    #[error("backend error: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// No backend is available for this build (platform or feature mismatch).
    #[error("unsupported platform")]
    Unsupported,
}

#[cfg(all(target_os = "linux", feature = "x11"))]
#[cfg_attr(docsrs, doc(cfg(feature = "x11")))]
impl From<x11::X11Error> for PlatformError {
    fn from(e: x11::X11Error) -> Self {
        PlatformError::Backend(Box::new(e))
    }
}

/// Keyboard output abstraction.
pub trait KeyboardOutput {
    fn send_backspaces(&mut self, count: usize) -> Result<()>;
    fn send_string(&mut self, s: &str) -> Result<()>;
    fn send_key_combination(&mut self, key: &str, modifiers: &[&str]) -> Result<()>;
}

/// Create the keyboard output backend for the current platform.
pub fn create_output() -> Result<Box<dyn KeyboardOutput>> {
    #[cfg(all(target_os = "linux", feature = "x11"))]
    {
        Ok(Box::new(x11::X11Output::new()?))
    }
    #[cfg(not(all(target_os = "linux", feature = "x11")))]
    {
        Err(PlatformError::Unsupported)
    }
}
