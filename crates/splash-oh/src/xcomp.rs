//! A native render surface, created by Rust with no ArkTS anywhere.
//!
//! This is the answer to "wouldn't ArkTS add overhead" for anything that needs
//! a surface — a camera preview, a video frame, a GL scene.
//!
//! The web slots are stuck with an ArkTS overlay because **there is no
//! `ARKUI_NODE_WEB`**: the NDK exposes no web component at all, so Rust cannot
//! create one and a real `Web` has to be positioned on top of the native tree
//! by ArkTS. Every rebuild therefore crosses into ArkTS to re-sync geometry.
//!
//! `XComponent` is not like that. `ARKUI_NODE_XCOMPONENT` is a first-class node
//! type, so the whole chain stays native:
//!
//! ```text
//! Node::new(ARKUI_NODE_XCOMPONENT)            this file
//! OH_NativeXComponent_GetNativeXComponent()   node handle -> component
//! OH_NativeXComponent_RegisterCallback()      -> OnSurfaceCreated(window)
//! OH_NativeWindow_GetSurfaceId(window, &id)   the id a producer wants
//! ```
//!
//! and a frame producer writes into that surface directly. ArkTS is not on the
//! frame path, and here it is not on the setup path either.

use crate::arkui::Node;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::sync::Mutex;

pub const ARKUI_NODE_XCOMPONENT: i32 = 12;
/// `NODE_XCOMPONENT_TYPE`, from `MAX_NODE_SCOPE_NUM * ARKUI_NODE_XCOMPONENT + 1`.
const NODE_XCOMPONENT_ID: i32 = 1000 * ARKUI_NODE_XCOMPONENT;
const NODE_XCOMPONENT_TYPE: i32 = NODE_XCOMPONENT_ID + 1;
/// `ARKUI_XCOMPONENT_TYPE_SURFACE` — a real producer surface rather than a
/// texture, which is what a camera or codec wants to write into.
const XCOMPONENT_TYPE_SURFACE: i32 = 0;

#[repr(C)]
struct XComponentCallback {
    on_surface_created: extern "C" fn(*mut c_void, *mut c_void),
    on_surface_changed: extern "C" fn(*mut c_void, *mut c_void),
    on_surface_destroyed: extern "C" fn(*mut c_void, *mut c_void),
    dispatch_touch_event: extern "C" fn(*mut c_void, *mut c_void),
}

extern "C" {
    /// In libace_ndk.z, already a hard dependency — the ArkUI tree itself needs
    /// it, so unlike the image kits there is nothing optional about this one.
    fn OH_NativeXComponent_GetNativeXComponent(node: *mut c_void) -> *mut c_void;
    fn OH_NativeXComponent_RegisterCallback(
        component: *mut c_void,
        callback: *mut XComponentCallback,
    ) -> i32;
    fn OH_NativeXComponent_GetXComponentSize(
        component: *mut c_void,
        window: *const c_void,
        w: *mut u64,
        h: *mut u64,
    ) -> i32;
}

/// `OH_NativeWindow_GetSurfaceId` lives in libnative_window, which is resolved
/// at runtime for the reason the image kits are: a `DT_NEEDED` the loader
/// cannot satisfy kills the whole library, and one optional capability has no
/// business being able to do that.
type GetSurfaceId = unsafe extern "C" fn(*mut c_void, *mut u64) -> i32;
static SURFACE_ID_FN: std::sync::OnceLock<Option<GetSurfaceId>> = std::sync::OnceLock::new();

extern "C" {
    fn dlopen(file: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

fn get_surface_id_fn() -> Option<GetSurfaceId> {
    *SURFACE_ID_FN.get_or_init(|| {
        let lib = std::ffi::CString::new("libnative_window.so").ok()?;
        let h = unsafe { dlopen(lib.as_ptr(), 2) };
        if h.is_null() {
            crate::log("xcomp: libnative_window.so did not load");
            return None;
        }
        let name = std::ffi::CString::new("OH_NativeWindow_GetSurfaceId").ok()?;
        let p = unsafe { dlsym(h, name.as_ptr()) };
        if p.is_null() {
            crate::log("xcomp: OH_NativeWindow_GetSurfaceId not exported");
            return None;
        }
        Some(unsafe { std::mem::transmute::<*mut c_void, GetSurfaceId>(p) })
    })
}

/// What the surface callbacks have reported. A page reads this to find out
/// whether a native surface actually materialised, and what its id is.
#[derive(Default, Clone)]
pub struct SurfaceState {
    pub created: bool,
    pub destroyed: bool,
    pub surface_id: u64,
    pub width: u64,
    pub height: u64,
}

static STATE: Mutex<Option<SurfaceState>> = Mutex::new(None);

fn update(f: impl FnOnce(&mut SurfaceState)) {
    if let Ok(mut s) = STATE.lock() {
        let st = s.get_or_insert_with(SurfaceState::default);
        f(st);
    }
}

extern "C" fn on_created(component: *mut c_void, window: *mut c_void) {
    let mut id: u64 = 0;
    if let Some(get) = get_surface_id_fn() {
        unsafe { get(window, &mut id) };
    }
    let (mut w, mut h) = (0u64, 0u64);
    if !component.is_null() {
        unsafe { OH_NativeXComponent_GetXComponentSize(component, window, &mut w, &mut h) };
    }
    crate::log(&format!("xcomp: surface created, id {id}, {w}x{h} px"));
    update(|s| {
        s.created = true;
        s.destroyed = false;
        s.surface_id = id;
        s.width = w;
        s.height = h;
    });
}

extern "C" fn on_changed(_component: *mut c_void, _window: *mut c_void) {}

extern "C" fn on_destroyed(_component: *mut c_void, _window: *mut c_void) {
    crate::log("xcomp: surface destroyed");
    update(|s| {
        s.created = false;
        s.destroyed = true;
    });
}

extern "C" fn on_touch(_component: *mut c_void, _window: *mut c_void) {}

/// The callback table must outlive the component, so it is static rather than a
/// local — registering a stack address here would hand ArkUI a dangling pointer
/// the moment this function returned.
static mut CALLBACKS: XComponentCallback = XComponentCallback {
    on_surface_created: on_created,
    on_surface_changed: on_changed,
    on_surface_destroyed: on_destroyed,
    dispatch_touch_event: on_touch,
};

/// Build a surface node of the given size. `None` if the node type is not
/// available on this build.
pub fn surface(w: f32, h: f32) -> Option<Node> {
    let node = Node::new(ARKUI_NODE_XCOMPONENT)?
        .i32_attr(NODE_XCOMPONENT_TYPE, XCOMPONENT_TYPE_SURFACE)
        .width(w)
        .height(h);

    let component = unsafe { OH_NativeXComponent_GetNativeXComponent(node.raw()) };
    if component.is_null() {
        crate::log("xcomp: node created but no OH_NativeXComponent behind it");
        return Some(node);
    }
    let rc = unsafe { OH_NativeXComponent_RegisterCallback(component, &raw mut CALLBACKS) };
    crate::log(&format!("xcomp: registered surface callbacks ({rc})"));
    Some(node)
}

/// What the surface reported, as JSON.
pub fn state() -> String {
    let s = STATE
        .lock()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_default();
    format!(
        "{{\"created\":{},\"destroyed\":{},\"surfaceId\":\"{}\",\"width\":{},\"height\":{}}}",
        s.created, s.destroyed, s.surface_id, s.width, s.height
    )
}
