//! Route AppKit terminate: (menu, Dock and Cmd+Q) through asynchronous Hub shutdown.
//! Iced/Winit's normal termination otherwise bypasses CloseRequested.
use std::sync::{Arc, OnceLock};
use tokio::sync::Notify;
static REQUESTS: OnceLock<Arc<Notify>> = OnceLock::new();
pub fn requests() -> Arc<Notify> {
    REQUESTS.get_or_init(|| Arc::new(Notify::new())).clone()
}
#[cfg(target_os = "macos")]
pub fn install() {
    use std::ffi::{c_char, c_void};
    type Obj = *mut c_void;
    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Obj;
        fn sel_registerName(name: *const c_char) -> Obj;
        fn class_getInstanceMethod(class: Obj, selector: Obj) -> Obj;
        fn method_setImplementation(
            method: Obj,
            implementation: unsafe extern "C" fn(Obj, Obj, Obj),
        ) -> *mut c_void;
    }
    unsafe extern "C" fn request_quit(_: Obj, _: Obj, _: Obj) {
        // Never block AppKit, unwind across FFI, or terminate from the callback.
        if let Some(requests) = REQUESTS.get() {
            requests.notify_one();
        }
    }
    requests();
    // Called once from main before running Iced. terminate: has ABI void(id, SEL, id).
    // Replace only this application's method; Winit's delegate/sendEvent stay intact.
    // Successful shutdown exits via Iced's event loop, not terminate: recursively.
    unsafe {
        let class = objc_getClass(c"NSApplication".as_ptr());
        assert!(!class.is_null(), "AppKit NSApplication unavailable");
        let method = class_getInstanceMethod(class, sel_registerName(c"terminate:".as_ptr()));
        assert!(!method.is_null(), "AppKit terminate: unavailable");
        method_setImplementation(method, request_quit);
    }
}
#[cfg(not(target_os = "macos"))]
pub fn install() {}
