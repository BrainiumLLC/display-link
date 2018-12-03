#![cfg(target_os = "ios")]

macro_rules! foreign_obj_type {
    {type CType = $raw_ident:ident;
    pub struct $owned_ident:ident;
    pub struct $ref_ident:ident;
    type ParentType = $parent_ref:ident;
    } => {
        foreign_obj_type! {
            type CType = $raw_ident;
            pub struct $owned_ident;
            pub struct $ref_ident;
        }

        impl ::std::ops::Deref for $ref_ident {
            type Target = $parent_ref;

            fn deref(&self) -> &$parent_ref {
                unsafe { &*(self as *const $ref_ident as *const $parent_ref)  }
            }
        }
    };
    {type CType = $raw_ident:ident;
    pub struct $owned_ident:ident;
    pub struct $ref_ident:ident;
    } => {
        foreign_types::foreign_type! {
            type CType = $raw_ident;
            fn drop = $crate::ios::obj_drop;
            fn clone = $crate::ios::obj_clone;
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
    runtime::{Object, Sel},
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
    is_paused:     bool,
    display_link:  RawDisplayLink,
    raw_callback:  *mut c_void,
    drop_callback: unsafe fn(*mut c_void),
}

impl Drop for DisplayLink {
    fn drop(&mut self) {
        if !self.is_paused {
            unsafe {
                self.display_link.invalidate();
                (self.drop_callback)(self.raw_callback)
            }
        }
    }
}

extern "C" fn run_callback<F: 'static + FnMut(Instant) + Send>(
    this: &Object,
    _: Sel,
    display_link: *mut Object,
) {
    match panic::catch_unwind(|| unsafe {
        let callback: *mut c_void = *this.get_ivar("_data");
        let callback = &mut *(callback as *mut Callback<F>);

        let t: f64 = msg_send![display_link, timestamp];
        let duration: f64 = msg_send![display_link, duration];
        let t = t + duration;

        let (start_os, start_rust) = match callback.start_time {
            Some((start_os, start_rust)) => (start_os, start_rust),
            None => {
                let os_cur_time = cadisplaylink::CACurrentMediaTime();
                let rust_cur_time = Instant::now();
                let start_os = t;
                debug_assert!(start_os >= os_cur_time);
                let d = start_os - os_cur_time;
                let d = Duration::from_float_secs(d);
                let start_rust = rust_cur_time + d;
                callback.start_time = Some((start_os, start_rust));
                (start_os, start_rust)
            }
        };

        let diff = Duration::from_float_secs(t - start_os);
        let instant = start_rust + diff;
        (callback.f)(instant)
    }) {
        Err(_) => process::abort(),
        _ => {}
    }
}

impl DisplayLink {
    pub fn new<F>(callback: F) -> Option<Self>
    where
        F: 'static + FnMut(Instant) + Send,
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
        let display_link = unsafe {
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
            RawDisplayLink::with_target_selector(callback, sel!(call:))
        };

        unsafe fn drop_callback<F: 'static + FnMut(Instant) + Send>(callback: *mut c_void) {
            ptr::drop_in_place::<Callback<F>>(callback as _)
        }

        Some(DisplayLink {
            is_paused: true,
            display_link,
            raw_callback,
            drop_callback: drop_callback::<F>,
        })
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn pause(&mut self) -> Result<(), PauseError> {
        if self.is_paused {
            Err(PauseError::AlreadyPaused)
        } else {
            unsafe {
                self.display_link.invalidate();
            }
            self.is_paused = true;
            Ok(())
        }
    }

    pub fn resume(&mut self) -> Result<(), ResumeError> {
        if !self.is_paused {
            Err(ResumeError::AlreadyRunning)
        } else {
            unsafe {
                self.display_link.add_to_current();
            }
            self.is_paused = false;
            Ok(())
        }
    }
}

struct Callback<F: 'static + FnMut(Instant) + Send> {
    start_time: Option<(f64, Instant)>,
    f:          F,
}

#[inline]
unsafe fn obj_drop<T>(p: *mut T) {
    msg_send![(p as *mut Object), release];
}

#[inline]
unsafe fn obj_clone<T: 'static>(p: *mut T) -> *mut T {
    msg_send![(p as *mut Object), retain]
}