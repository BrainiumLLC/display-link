//! Apple docs: [CADisplayLink](https://developer.apple.com/documentation/quartzcore/cadisplaylink?language=objc)

use objc::{
    class, msg_send,
    runtime::{Object, Sel},
    sel, sel_impl,
};
use objc_foundation::NSString;

#[link(name = "Foundation", kind = "framework")]
#[link(name = "UIKit", kind = "framework")]
extern "C" {
    #[allow(improper_ctypes)]
    static NSRunLoopCommonModes: *mut NSString;
    pub fn CACurrentMediaTime() -> f64;
}

pub enum CADisplayLink {}

foreign_obj_type! {
    type CType = CADisplayLink;
    pub struct DisplayLink;
    pub struct DisplayLinkRef;
}

impl DisplayLink {
    /// Apple docs: [displayLinkWithTarget:selector:](https://developer.apple.com/documentation/quartzcore/cadisplaylink/1621228-displaylinkwithtarget?language=objc)
    pub unsafe fn with_target_selector(object: *mut Object, selector: Sel) -> Self {
        msg_send![class!(CADisplayLink), displayLinkWithTarget:object selector:selector]
    }
}

impl DisplayLinkRef {
    /// Apple docs: [addToRunLoop:forMode:](https://developer.apple.com/documentation/quartzcore/cadisplaylink/1621323-addtorunloop?language=objc)
    pub unsafe fn add_to_run_loop_for_mode(&mut self, run_loop: *mut Object, mode: *mut NSString) {
        msg_send![
            self,
            addToRunLoop: run_loop
            forMode: mode
        ];
    }

    /// Calls `self.add_to_run_loop_for_mode([NSRunLoop currentRunLoop], NSRunLoopCommonModes)`
    pub unsafe fn add_to_current(&mut self) {
        self.add_to_run_loop_for_mode(
            msg_send![class!(NSRunLoop), currentRunLoop],
            NSRunLoopCommonModes,
        )
    }

    /// Apple docs: [invalidate](https://developer.apple.com/documentation/quartzcore/cadisplaylink/1621293-invalidate?language=objc)
    pub unsafe fn invalidate(&mut self) {
        msg_send![self, invalidate];
    }
}
