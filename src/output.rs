//! Platform output abstraction.

use bitflags::bitflags;
use thiserror::Error;

// TODO: show every platform in `cargo doc`
#[cfg(all(target_os = "linux"))]
// #[cfg_attr(docsrs, doc(cfg(feature = "x11")))]
pub mod x11;

/// Alias for [`std::result::Result`] with [`PlatformError`] as the error type.
pub type Result<T> = std::result::Result<T, PlatformError>;

bitflags! {
    /// Modifier keys held alongside a [`Key`].
    ///
    /// `Shift` is per-key: it ORs with the [`Key::Char`]'s intrinsic shift need
    /// (e.g. `Char('A')` already implies Shift). Other modifiers stay held only
    /// for the keys they're attached to.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        const CTRL  = 1 << 0;
        const ALT   = 1 << 1;
        const SHIFT = 1 << 2;
        const SUPER = 1 << 3;
    }
}

/// Non-modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Backspace,
    Return,
    Tab,
    Escape,
    Char(char),
}

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

/// Output abstraction.
pub trait Output {
    /// Tap each key with its own modifier set, minimising modifier toggling
    /// between consecutive strokes.
    fn send_keys(&mut self, strokes: &[(Key, Modifiers)]) -> Result<()>;
}

/// Create the keyboard output backend for the current platform.
pub fn new() -> Result<Box<dyn Output>> {
    // TODO: auto-detect X11 and Wayland at runtime
    #[cfg(all(target_os = "linux", feature = "x11"))]
    {
        Ok(Box::new(x11::X11Output::new()?))
    }
    #[cfg(not(all(target_os = "linux", feature = "x11")))]
    {
        Err(PlatformError::Unsupported)
    }
}
