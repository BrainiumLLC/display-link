#![cfg(target_os = "macos")]

pub mod cvdisplaylink;

use crate::{
    macos::cvdisplaylink::{CVDisplayLink, CVTimeStamp, DisplayLink as RawDisplayLink},
    PauseError, ResumeError,
};
use std::{any::Any, ffi::c_void, mem};
use time_point::TimePoint;

unsafe extern "C" fn render<F>(
    _: *mut CVDisplayLink,
    _: *const CVTimeStamp,
    in_out_timestamp: *const CVTimeStamp,
    _: i64,
    _: *mut i64,
    display_link_context: *mut c_void,
) -> i32
where
    F: FnMut(TimePoint),
{
    let in_out_timestamp = &*in_out_timestamp;
    let time = mem::transmute::<_, std::time::Instant>(in_out_timestamp.host_time);
    let f = &mut *(display_link_context as *mut F);
    f(TimePoint::from_std_instant(time));
    0
}

#[derive(Debug)]
pub struct DisplayLink {
    is_paused:    bool,
    func:         Box<dyn Any>,
    display_link: RawDisplayLink,
}

impl Drop for DisplayLink {
    fn drop(&mut self) {
        if !self.is_paused {
            unsafe {
                self.display_link.stop();
            }
        }
    }
}

impl DisplayLink {
    fn new_impl<R, F>(make_raw: R, callback: F) -> Option<Self>
    where
        R: FnOnce() -> Option<RawDisplayLink>,
        F: 'static + FnMut(TimePoint) + Send,
    {
        let func = Box::new(callback);
        unsafe {
            let raw = Box::into_raw(func);
            let func = Box::from_raw(raw);
            let mut display_link = make_raw()?;
            display_link.set_output_callback(render::<F>, raw as *mut c_void);
            Some(DisplayLink {
                is_paused: true,
                func,
                display_link,
            })
        }
    }

    /// Creates a new iOS `DisplayLink` instance.
    ///
    /// macos _does_ require the callback to be `Send`.
    pub fn new<F>(callback: F) -> Option<Self>
    where
        F: 'static + FnMut(TimePoint) + Send,
    {
        Self::new_impl(|| unsafe { RawDisplayLink::new() }, callback)
    }

    pub fn on_display<F>(display_id: u32, callback: F) -> Option<Self>
    where
        F: 'static + FnMut(TimePoint) + Send,
    {
        Self::new_impl(
            || unsafe { RawDisplayLink::on_display(display_id) },
            callback,
        )
    }

    #[cfg(feature = "winit")]
    pub fn on_monitor<F>(monitor: &winit::monitor::MonitorHandle, callback: F) -> Option<Self>
    where
        F: 'static + FnMut(TimePoint) + Send,
    {
        use winit::platform::macos::MonitorHandleExtMacOS;
        let id = monitor.native_id();
        Self::on_display(id, callback)
    }

    pub fn set_current_display(&mut self, display_id: u32) {
        unsafe { self.display_link.set_current_display(display_id) }
    }

    #[cfg(feature = "winit")]
    pub fn set_current_monitor(&mut self, monitor: &winit::monitor::MonitorHandle) {
        use winit::platform::macos::MonitorHandleExtMacOS;
        let id = monitor.native_id();
        self.set_current_display(id)
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn pause(&mut self) -> Result<(), PauseError> {
        if self.is_paused {
            Err(PauseError::AlreadyPaused)
        } else {
            unsafe {
                self.display_link.stop();
                self.is_paused = true;
                Ok(())
            }
        }
    }

    pub fn resume(&mut self) -> Result<(), ResumeError> {
        if !self.is_paused {
            Err(ResumeError::AlreadyRunning)
        } else {
            unsafe {
                self.display_link.start();
                self.is_paused = false;
                Ok(())
            }
        }
    }
}
