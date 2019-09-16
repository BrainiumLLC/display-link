pub mod ios;
pub mod macos;

use std::fmt::{self, Display, Formatter};
use time_point::TimePoint;

#[cfg(target_os = "ios")]
use crate::ios::DisplayLink as PlatformDisplayLink;
#[cfg(target_os = "macos")]
use crate::macos::DisplayLink as PlatformDisplayLink;

#[derive(Debug)]
pub enum PauseError {
    AlreadyPaused,
}

impl Display for PauseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PauseError::AlreadyPaused => write!(formatter, "already paused"),
        }
    }
}

#[derive(Debug)]
pub enum ResumeError {
    AlreadyRunning,
}

impl Display for ResumeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ResumeError::AlreadyRunning => write!(formatter, "already running"),
        }
    }
}

/// `DisplayLink` is a timer object used to synchronize drawing with the refresh rate of the
/// display.
#[derive(Debug)]
pub struct DisplayLink(PlatformDisplayLink);

impl DisplayLink {
    /// Creates a new `DisplayLink` with a callback that will be invoked with the `TimePoint` the
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
        F: 'static + FnMut(TimePoint) + Send,
    {
        PlatformDisplayLink::new(callback).map(DisplayLink)
    }

    #[cfg(feature = "winit")]
    pub fn on_monitor<F>(monitor: &winit::monitor::MonitorHandle, callback: F) -> Option<Self>
    where
        F: 'static + FnMut(TimePoint) + Send,
    {
        PlatformDisplayLink::on_monitor(monitor, callback).map(DisplayLink)
    }

    #[cfg(feature = "winit")]
    pub fn set_current_monitor(&mut self, monitor: &winit::monitor::MonitorHandle) {
        self.0.set_current_monitor(monitor)
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
