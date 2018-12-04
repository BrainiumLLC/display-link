#![cfg(target_os = "macos")]

pub mod cvdisplaylink;

use crate::{
    macos::cvdisplaylink::{CVDisplayLink, CVTimeStamp, DisplayLink as RawDisplayLink},
    PauseError, ResumeError,
};
use std::{any::Any, ffi::c_void, mem, panic, process, time::Instant};

unsafe extern "C" fn render<F>(
    _: *mut CVDisplayLink,
    _: *const CVTimeStamp,
    in_out_timestamp: *const CVTimeStamp,
    _: i64,
    _: *mut i64,
    display_link_context: *mut c_void,
) -> i32
where
    F: FnMut(Instant),
{
    match panic::catch_unwind(|| {
        let in_out_timestamp = &*in_out_timestamp;
        let time = mem::transmute(in_out_timestamp.host_time);
        let f = &mut *(display_link_context as *mut F);
        f(time);
        0
    }) {
        Ok(o) => o,
        _ => process::abort(),
    }
}

#[derive(Debug)]
pub struct DisplayLink {
    is_paused:    bool,
    func:         Box<Any>,
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
    pub fn new<F>(callback: F) -> Option<Self>
    where
        F: 'static + FnMut(Instant) + Send,
    {
        let func = Box::new(callback);
        unsafe {
            let raw = Box::into_raw(func);
            let func = Box::from_raw(raw);
            let mut display_link = RawDisplayLink::new()?;
            display_link.set_output_callback(render::<F>, raw as *mut c_void);
            Some(DisplayLink {
                is_paused: true,
                func,
                display_link,
            })
        }
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
