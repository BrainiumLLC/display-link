#![cfg(target_os = "ios")]

macro_rules! foreign_obj_type {
    {type CType = $raw_ident:ident;
    fn drop = $drop_func:ident;
    pub struct $owned_ident:ident;
    pub struct $ref_ident:ident;
    } => {
        foreign_types::foreign_type! {
            type CType = $raw_ident;
            fn drop = $drop_func;
            pub struct $owned_ident;
            pub struct $ref_ident;
        }

        unsafe impl ::objc::Message for $raw_ident {
        }
        unsafe impl ::objc::Message for $ref_ident {
        }

        impl ::std::fmt::Debug for $ref_ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                unsafe {
                    use ::objc_foundation::INSString;
                    // TODO: might leak, not 100% sure...
                    let string: &::objc_foundation::NSString = msg_send![self, debugDescription];
                    write!(f, "{}", string.as_str())
                }
            }
        }

        impl ::std::fmt::Debug for $owned_ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                ::std::ops::Deref::deref(self).fmt(f)
            }
        }
    };
}

pub mod cadisplaylink;

use crate::{ios::cadisplaylink::DisplayLink as RawDisplayLink, PauseError, ResumeError};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Object, Sel, NO, YES},
    sel, sel_impl,
};
use std::{
    ffi::c_void,
    panic, process, ptr,
    sync::Once,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct DisplayLink {
    display_link:  RawDisplayLink,
    raw_callback:  *mut c_void,
    drop_callback: unsafe fn(*mut c_void),
}

impl Drop for DisplayLink {
    fn drop(&mut self) {
        unsafe { (self.drop_callback)(self.raw_callback) }
    }
}

extern "C" fn run_callback<F: 'static + FnMut(Instant)>(
    this: &Object,
    _: Sel,
    display_link: *mut Object,
) {
    match panic::catch_unwind(|| unsafe {
        let callback: *mut c_void = *this.get_ivar("_data");
        let callback = &mut *(callback as *mut Callback<F>);

        let t: f64 = msg_send![display_link, timestamp];
        let duration: f64 = msg_send![display_link, duration];

        let (start_os, start_rust) = match callback.start_time {
            Some((start_os, start_rust)) => (start_os, start_rust),
            None => {
                let os_cur_time = cadisplaylink::CACurrentMediaTime();
                let rust_cur_time = Instant::now();
                let start_os = t;
                debug_assert!(start_os <= os_cur_time);
                let d = os_cur_time - start_os;
                let d = from_secs_f64(d);
                let start_rust = rust_cur_time - d;
                callback.start_time = Some((start_os, start_rust));
                (start_os, start_rust)
            }
        };
        let t = t + duration;

        let diff = from_secs_f64(t - start_os);
        let instant = start_rust + diff;
        (callback.f)(instant)
    }) {
        Err(_) => process::abort(),
        _ => {}
    }
}

impl DisplayLink {
    /// Creates a new iOS `DisplayLink` instance.
    ///
    /// iOS does _not_ require the callback to be `Send`.
    pub fn new<F>(callback: F) -> Option<Self>
    where
        F: 'static + FnMut(Instant),
    {
        static CALLBACK_CLASS_CREATOR: Once = Once::new();
        CALLBACK_CLASS_CREATOR.call_once(|| {
            let mut decl = ClassDecl::new("DisplayLinkCallbackHolder", class!(NSObject)).unwrap();
            decl.add_ivar::<*mut c_void>("_data");
            unsafe {
                decl.add_method(
                    sel!(call:),
                    run_callback::<F> as extern "C" fn(&Object, Sel, *mut Object),
                );
            }
            decl.register();
        });

        let raw_callback;
        let mut display_link = unsafe {
            let callback = {
                let dl_callback: *mut Object = msg_send![class!(DisplayLinkCallbackHolder), alloc];
                let dl_callback: *mut Object = msg_send![dl_callback, init];

                let callback = Callback {
                    start_time: None,
                    f:          callback,
                };

                let dl_callback: &mut Object = &mut *dl_callback;
                raw_callback = Box::into_raw(Box::new(callback)) as *mut _;
                dl_callback.set_ivar::<*mut c_void>("_data", raw_callback);
                dl_callback
            };
            let dl = RawDisplayLink::with_target_selector(callback, sel!(call:));
            // let () = msg_send![callback, release]; // retained by displaylink
            dl
        };
        unsafe {
            display_link.set_paused(YES);
            display_link.add_to_current();
        }

        unsafe fn drop_callback<F: 'static + FnMut(Instant)>(callback: *mut c_void) {
            ptr::drop_in_place::<Callback<F>>(callback as _)
        }

        Some(DisplayLink {
            display_link,
            raw_callback,
            drop_callback: drop_callback::<F>,
        })
    }

    pub fn is_paused(&self) -> bool {
        NO != unsafe { self.display_link.is_paused() }
    }

    pub fn pause(&mut self) -> Result<(), PauseError> {
        if self.is_paused() {
            Err(PauseError::AlreadyPaused)
        } else {
            unsafe {
                self.display_link.set_paused(YES);
            }
            Ok(())
        }
    }

    pub fn resume(&mut self) -> Result<(), ResumeError> {
        if !self.is_paused() {
            Err(ResumeError::AlreadyRunning)
        } else {
            unsafe {
                self.display_link.set_paused(NO);
            }
            Ok(())
        }
    }
}

struct Callback<F: 'static + FnMut(Instant)> {
    start_time: Option<(f64, Instant)>,
    f:          F,
}

// https://doc.rust-lang.org/std/time/struct.Duration.html#method.from_secs_f64
fn from_secs_f64(secs: f64) -> Duration {
    const NANOS_PER_SEC: u32 = 1_000_000_000;
    const MAX_NANOS_F64: f64 = ((std::u64::MAX as u128 + 1) * (NANOS_PER_SEC as u128)) as f64;
    let nanos = secs * (NANOS_PER_SEC as f64);
    if !nanos.is_finite() {
        panic!("got non-finite value when converting float to duration");
    }
    if nanos >= MAX_NANOS_F64 {
        panic!("overflow when converting float to duration");
    }
    if nanos < 0.0 {
        panic!("underflow when converting float to duration");
    }
    let nanos = nanos as u128;
    Duration::new(
        (nanos / (NANOS_PER_SEC as u128)) as u64,
        (nanos % (NANOS_PER_SEC as u128)) as u32,
    )
}
