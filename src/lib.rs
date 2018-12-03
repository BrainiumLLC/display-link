#![feature(duration_float)]

mod ios;
mod macos;

use failure::Fail;
use std::time::Instant;

#[cfg(target_os = "ios")]
pub use crate::ios::cadisplaylink;
#[cfg(target_os = "macos")]
pub use crate::macos::cvdisplaylink;

#[cfg(target_os = "ios")]
use crate::ios::DisplayLink as PlatformDisplayLink;
#[cfg(target_os = "macos")]
use crate::macos::DisplayLink as PlatformDisplayLink;

#[derive(Debug, Fail)]
pub enum PauseError {
    #[fail(display = "already paused")]
    AlreadyPaused,
}

#[derive(Debug, Fail)]
pub enum ResumeError {
    #[fail(display = "already running")]
    AlreadyRunning,
}

/// `DisplayLink` is a timer object used to synchronize drawing with the refresh rate of the
/// display.
#[derive(Debug)]
pub struct DisplayLink(PlatformDisplayLink);

impl DisplayLink {
    /// Creates a new `DisplayLink` with a callback that will be invoked with the `Instant` the
    /// screen will next refresh.
    ///
    /// The returned `DisplayLink` will be in a paused state. Returns `None` if a `DisplayLink`
    /// could not be created.
    ///
    /// ## Panic
    ///
    /// If the callback panics, the process will be aborted.
    pub fn new<F>(callback: F) -> Option<Self>
    where
        F: 'static + FnMut(Instant) + Send,
    {
        PlatformDisplayLink::new(callback).map(DisplayLink)
    }

    /// Returns `true` if the `DisplayLink` is currently paused.
    pub fn is_paused(&self) -> bool {
        self.0.is_paused()
    }

    /// Pauses the `DisplayLink`.
    ///
    /// A paused `DisplayLink` will not invoke it's callback. On iOS, it is necessary to pause the
    /// `DisplayLink` in response to events like backgrounding.
    pub fn pause(&mut self) -> Result<(), PauseError> {
        self.0.pause()
    }

    /// Resumes the `DisplayLink`.
    pub fn resume(&mut self) -> Result<(), ResumeError> {
        self.0.resume()
    }
}