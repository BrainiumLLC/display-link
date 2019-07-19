//! Apple docs: [CVDisplayLink](https://developer.apple.com/documentation/corevideo/cvdisplaylinkoutputcallback?language=objc)

use foreign_types::{foreign_type, ForeignType};
use std::{
    ffi::c_void,
    fmt::{Debug, Formatter, Result},
};

#[derive(Debug)]
pub enum CVDisplayLink {}

foreign_type! {
    type CType = CVDisplayLink;
    fn drop = CVDisplayLinkRelease;
    fn clone = CVDisplayLinkRetain;
    pub struct DisplayLink;
    pub struct DisplayLinkRef;
}

impl Debug for DisplayLink {
    fn fmt(&self, formatter: &mut Formatter) -> Result {
        formatter
            .debug_tuple("DisplayLink")
            .field(&self.as_ptr())
            .finish()
    }
}

#[doc(hidden)]
pub enum Unimplemented {}

#[repr(C)]
pub struct CVTimeStamp {
    pub version:              u32,
    pub video_timescale:      i32,
    pub video_time:           i64,
    pub host_time:            u64,
    pub rate_scalar:          f64,
    pub video_refresh_period: i64,

    #[doc(hidden)]
    _unimplemented: Unimplemented,
}

pub type CVDisplayLinkOutputCallback = unsafe extern "C" fn(
    display_link_out: *mut CVDisplayLink,
    in_now_timestamp: *const CVTimeStamp,
    in_output_timestamp: *const CVTimeStamp,
    flags_in: i64,
    flagsOut: *mut i64,
    display_link_context: *mut c_void,
) -> i32;

#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "CoreVideo", kind = "framework")]
#[allow(improper_ctypes)]
extern "C" {
    pub fn CVDisplayLinkCreateWithActiveCGDisplays(
        display_link_out: *mut *mut CVDisplayLink,
    ) -> i32;
    pub fn CVDisplayLinkCreateWithCGDisplay(
        display_id: u32,
        display_link_out: *mut *mut CVDisplayLink,
    ) -> i32;
    pub fn CVDisplayLinkSetOutputCallback(
        display_link: &mut DisplayLinkRef,
        callback: CVDisplayLinkOutputCallback,
        user_info: *mut c_void,
    ) -> i32;
    pub fn CVDisplayLinkSetCurrentCGDisplay(
        display_link: &mut DisplayLinkRef,
        display_id: u32,
    ) -> i32;
    pub fn CVDisplayLinkStart(display_link: &mut DisplayLinkRef) -> i32;
    pub fn CVDisplayLinkStop(display_link: &mut DisplayLinkRef) -> i32;
    pub fn CVDisplayLinkRelease(display_link: *mut CVDisplayLink);
    pub fn CVDisplayLinkRetain(display_link: *mut CVDisplayLink) -> *mut CVDisplayLink;
}

impl DisplayLink {
    /// Apple docs: [CVDisplayLinkCreateWithActiveCGDisplays](https://developer.apple.com/documentation/corevideo/1456863-cvdisplaylinkcreatewithactivecgd?language=objc)
    pub unsafe fn new() -> Option<Self> {
        let mut display_link: *mut CVDisplayLink = 0 as _;
        let code = CVDisplayLinkCreateWithActiveCGDisplays(&mut display_link);
        if code == 0 {
            Some(DisplayLink::from_ptr(display_link))
        } else {
            None
        }
    }

    /// Apple docs: [CVDisplayLinkCreateWithCGDisplay](https://developer.apple.com/documentation/corevideo/1456981-cvdisplaylinkcreatewithcgdisplay?language=objc)
    pub unsafe fn on_display(display_id: u32) -> Option<Self> {
        let mut display_link: *mut CVDisplayLink = 0 as _;
        let code = CVDisplayLinkCreateWithCGDisplay(display_id, &mut display_link);
        if code == 0 {
            Some(DisplayLink::from_ptr(display_link))
        } else {
            None
        }
    }
}

impl DisplayLinkRef {
    /// Apple docs: [CVDisplayLinkSetOutputCallback](https://developer.apple.com/documentation/corevideo/1457096-cvdisplaylinksetoutputcallback?language=objc)
    pub unsafe fn set_output_callback(
        &mut self,
        callback: CVDisplayLinkOutputCallback,
        user_info: *mut c_void,
    ) {
        assert_eq!(CVDisplayLinkSetOutputCallback(self, callback, user_info), 0);
    }

    /// Apple docs: [CVDisplayLinkSetCurrentCGDisplay](https://developer.apple.com/documentation/corevideo/1456768-cvdisplaylinksetcurrentcgdisplay?language=objc)
    pub unsafe fn set_current_display(&mut self, display_id: u32) {
        assert_eq!(CVDisplayLinkSetCurrentCGDisplay(self, display_id), 0);
    }

    /// Apple docs: [CVDisplayLinkStart](https://developer.apple.com/documentation/corevideo/1457193-cvdisplaylinkstart?language=objc)
    pub unsafe fn start(&mut self) {
        assert_eq!(CVDisplayLinkStart(self), 0);
    }

    /// Apple docs: [CVDisplayLinkStop](https://developer.apple.com/documentation/corevideo/1457281-cvdisplaylinkstop?language=objc)
    pub unsafe fn stop(&mut self) {
        assert_eq!(CVDisplayLinkStop(self), 0);
    }
}
