//! Safe Rust over the ArkUI NDK, via `shim.c`.
//!
//! The whole widget tree is built here — created, configured, mounted and
//! event-wired from native code. ArkTS never sees an individual widget; it only
//! hands over one `NodeContent` slot at startup.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

pub type NodeHandle = *mut c_void;
pub type NodeContentHandle = *mut c_void;

extern "C" {
    fn splash_arkui_init() -> c_int;
    fn splash_create_node(ty: c_int) -> NodeHandle;
    fn splash_dispose_node(n: NodeHandle);
    fn splash_add_child(parent: NodeHandle, child: NodeHandle) -> c_int;
    fn splash_set_string(n: NodeHandle, attr: c_int, s: *const c_char) -> c_int;
    fn splash_set_i32_string(n: NodeHandle, attr: c_int, v: i32, s: *const c_char) -> c_int;
    fn splash_set_f32(n: NodeHandle, attr: c_int, v: f32) -> c_int;
    fn splash_set_i32(n: NodeHandle, attr: c_int, v: i32) -> c_int;
    fn splash_set_u32(n: NodeHandle, attr: c_int, v: u32) -> c_int;
    fn splash_set_f32v(n: NodeHandle, attr: c_int, v: *const f32, count: c_int) -> c_int;
    fn splash_content_add(content: NodeContentHandle, root: NodeHandle) -> c_int;
    fn splash_register_event(n: NodeHandle, event_type: c_int, id: i32) -> c_int;
}

/// ArkUI enum values, evaluated by the C++ compiler against the real headers
/// (see `shim.cpp`). Hand-transcribing them produced a SIGSEGV: the enums are
/// mostly implicit and the per-component blocks are `1000 * ARKUI_NODE_X + n`,
/// so a single wrong value silently addresses a different attribute.
mod raw {
    extern "C" {
        pub static splash_a_width: i32;
        pub static splash_a_height: i32;
        pub static splash_a_bg: i32;
        pub static splash_a_padding: i32;
        pub static splash_a_margin: i32;
        pub static splash_a_border_width: i32;
        pub static splash_a_border_radius: i32;
        pub static splash_a_border_color: i32;
        pub static splash_a_checkbox_select: i32;
        pub static splash_a_checkbox_color: i32;
        pub static splash_a_radio_checked: i32;
        pub static splash_a_toggle_value: i32;
        pub static splash_a_toggle_color: i32;
        pub static splash_a_textpicker_range: i32;
        pub static splash_a_image_src: i32;
        pub static splash_a_image_fit: i32;
        pub static splash_a_alignment: i32;
        pub static splash_a_opacity: i32;
        pub static splash_a_visibility: i32;
        pub static splash_a_text_content: i32;
        pub static splash_a_font_color: i32;
        pub static splash_a_font_size: i32;
        pub static splash_a_font_weight: i32;
        pub static splash_a_text_align: i32;
        pub static splash_a_button_label: i32;
        pub static splash_a_progress_value: i32;
        pub static splash_a_progress_total: i32;
        pub static splash_a_input_placeholder: i32;

        pub static splash_t_text: i32;
        pub static splash_t_image: i32;
        pub static splash_t_toggle: i32;
        pub static splash_t_loading: i32;
        pub static splash_t_input: i32;
        pub static splash_t_textarea: i32;
        pub static splash_t_button: i32;
        pub static splash_t_progress: i32;
        pub static splash_t_checkbox: i32;
        pub static splash_t_datepicker: i32;
        pub static splash_t_slider: i32;
        pub static splash_t_radio: i32;
        pub static splash_t_stack: i32;
        pub static splash_t_scroll: i32;
        pub static splash_t_list: i32;
        pub static splash_t_column: i32;
        pub static splash_t_row: i32;
        pub static splash_t_flex: i32;
        pub static splash_t_timepicker: i32;
        pub static splash_t_textpicker: i32;
        pub static splash_t_swiper: i32;
        pub static splash_t_grid: i32;
        pub static splash_t_waterflow: i32;
        pub static splash_t_refresh: i32;
        pub static splash_e_click: i32;
    }
}

macro_rules! arkui_consts {
    ($($fn_name:ident => $sym:ident),* $(,)?) => {
        $(#[inline] pub fn $fn_name() -> i32 { unsafe { raw::$sym } })*
    };
}

/// Node types.
pub mod ty {
    use super::raw;
    arkui_consts! {
        text => splash_t_text, image => splash_t_image, toggle => splash_t_toggle,
        loading => splash_t_loading, input => splash_t_input, textarea => splash_t_textarea,
        button => splash_t_button, progress => splash_t_progress, checkbox => splash_t_checkbox,
        datepicker => splash_t_datepicker, slider => splash_t_slider, radio => splash_t_radio,
        stack => splash_t_stack, scroll => splash_t_scroll, list => splash_t_list,
        column => splash_t_column, row => splash_t_row, flex => splash_t_flex,
        timepicker => splash_t_timepicker, textpicker => splash_t_textpicker,
        swiper => splash_t_swiper, grid => splash_t_grid,
        waterflow => splash_t_waterflow, refresh => splash_t_refresh,
    }
}

/// Attribute ids.
pub mod attr {
    use super::raw;
    arkui_consts! {
        width => splash_a_width, height => splash_a_height, bg => splash_a_bg,
        padding => splash_a_padding, margin => splash_a_margin,
        border_width => splash_a_border_width, border_radius => splash_a_border_radius,
        border_color => splash_a_border_color, alignment => splash_a_alignment,
        opacity => splash_a_opacity, visibility => splash_a_visibility,
        text_content => splash_a_text_content, font_color => splash_a_font_color,
        font_size => splash_a_font_size, font_weight => splash_a_font_weight,
        text_align => splash_a_text_align, button_label => splash_a_button_label,
        progress_value => splash_a_progress_value, progress_total => splash_a_progress_total,
        input_placeholder => splash_a_input_placeholder,
        checkbox_select => splash_a_checkbox_select,
        checkbox_color => splash_a_checkbox_color,
        radio_checked => splash_a_radio_checked,
        toggle_value => splash_a_toggle_value,
        toggle_color => splash_a_toggle_color,
        image_src => splash_a_image_src,
        image_fit => splash_a_image_fit,
        textpicker_range => splash_a_textpicker_range,
    }
}

/// Event ids.
pub mod event {
    use super::raw;
    arkui_consts! { click => splash_e_click }
}

/// Initialise the node API. Safe to call repeatedly.
pub fn init() -> Result<(), &'static str> {
    if unsafe { splash_arkui_init() } == 0 {
        Ok(())
    } else {
        Err("OH_ArkUI_GetModuleInterface(ARKUI_NATIVE_NODE) failed")
    }
}

/// An ArkUI component owned by native code.
pub struct Node {
    raw: NodeHandle,
    /// Children are kept alive here — ArkUI does not own them for us, and
    /// dropping a parent must not leave dangling child handles.
    children: Vec<Node>,
}

impl Node {
    pub fn new(ty: i32) -> Option<Self> {
        let raw = unsafe { splash_create_node(ty as c_int) };
        if raw.is_null() {
            return None;
        }
        Some(Self {
            raw,
            children: Vec::new(),
        })
    }

    pub fn raw(&self) -> NodeHandle {
        self.raw
    }

    pub fn child(mut self, c: Node) -> Self {
        unsafe { splash_add_child(self.raw, c.raw) };
        self.children.push(c);
        self
    }

    pub fn text(self, s: &str) -> Self {
        self.string_attr(attr::text_content(), s)
    }

    pub fn label(self, s: &str) -> Self {
        self.string_attr(attr::button_label(), s)
    }

    pub fn string_attr(self, a: i32, s: &str) -> Self {
        if let Ok(c) = CString::new(s) {
            unsafe { splash_set_string(self.raw, a, c.as_ptr()) };
        }
        self
    }

    /// For attributes that carry a number *and* a string in the same item —
    /// the text picker's range being the one this exists for.
    pub fn i32_string_attr(self, a: i32, v: i32, s: &str) -> Self {
        if let Ok(c) = CString::new(s) {
            unsafe { splash_set_i32_string(self.raw, a, v, c.as_ptr()) };
        }
        self
    }

    /// Set a float attribute on a raw handle, bypassing the builder. Only for
    /// the benchmark, which needs to time `setAttribute` on its own without a
    /// `Node` move in the loop.
    ///
    /// # Safety
    /// `raw` must be a live node handle owned by the caller.
    pub unsafe fn set_f32_attr_raw(raw: NodeHandle, a: i32, v: f32) {
        unsafe { splash_set_f32(raw, a, v) };
    }

    pub fn f32_attr(self, a: i32, v: f32) -> Self {
        unsafe { splash_set_f32(self.raw, a, v) };
        self
    }

    pub fn i32_attr(self, a: i32, v: i32) -> Self {
        unsafe { splash_set_i32(self.raw, a, v) };
        self
    }

    pub fn u32_attr(self, a: i32, v: u32) -> Self {
        unsafe { splash_set_u32(self.raw, a, v) };
        self
    }

    pub fn f32v_attr(self, a: i32, v: &[f32]) -> Self {
        unsafe { splash_set_f32v(self.raw, a, v.as_ptr(), v.len() as c_int) };
        self
    }

    // --- conveniences the backend leans on ---
    pub fn width(self, v: f32) -> Self {
        self.f32_attr(attr::width(), v)
    }
    pub fn height(self, v: f32) -> Self {
        self.f32_attr(attr::height(), v)
    }
    pub fn bg(self, argb: u32) -> Self {
        self.u32_attr(attr::bg(), argb)
    }
    pub fn font_color(self, argb: u32) -> Self {
        self.u32_attr(attr::font_color(), argb)
    }
    pub fn font_size(self, v: f32) -> Self {
        self.f32_attr(attr::font_size(), v)
    }
    pub fn font_weight(self, w: i32) -> Self {
        self.i32_attr(attr::font_weight(), w)
    }
    pub fn padding(self, p: f32) -> Self {
        self.f32v_attr(attr::padding(), &[p, p, p, p])
    }
    pub fn radius(self, r: f32) -> Self {
        self.f32v_attr(attr::border_radius(), &[r, r, r, r])
    }
    pub fn margin(self, m: f32) -> Self {
        self.f32v_attr(attr::margin(), &[m, m, m, m])
    }

    pub fn on_event(self, event_type: i32, id: i32) -> Self {
        unsafe { splash_register_event(self.raw, event_type, id) };
        self
    }

    /// Mount and hand the node back, so the caller can detach it later on a
    /// rebuild. `mount` forgets the tree; this keeps ownership.
    ///
    /// # Safety
    /// `content` must be the live `NodeContent` handle ArkTS passed to `mount`.
    pub unsafe fn mount_keep(self, content: NodeContentHandle) -> Result<Node, &'static str> {
        let r = unsafe { splash_content_add(content, self.raw) };
        if r == 0 {
            Ok(self)
        } else {
            Err("OH_ArkUI_NodeContent_AddNode failed")
        }
    }

    /// Mount as the root of an ArkTS-provided slot. Consumes self and leaks it
    /// deliberately: the tree must outlive this call for the page's lifetime.
    ///
    /// # Safety
    /// `content` must be the live `NodeContent` handle ArkTS passed to `mount`.
    pub unsafe fn mount(self, content: NodeContentHandle) -> Result<(), &'static str> {
        let r = unsafe { splash_content_add(content, self.raw) };
        std::mem::forget(self);
        if r == 0 {
            Ok(())
        } else {
            Err("OH_ArkUI_NodeContent_AddNode failed")
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        // Children first — ArkUI does not cascade.
        self.children.clear();
        unsafe { splash_dispose_node(self.raw) };
    }
}
