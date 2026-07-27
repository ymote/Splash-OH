//! Splash DSL → native ArkUI widgets.
//!
//! This is the piece that makes the repo's claim true: the widget tree is not
//! written in Rust, it is **evaluated by the Splash VM** (`makepad-script`) at
//! runtime and then walked into ArkUI nodes. The VM is renderer-free — it has
//! no dependency on makepad-platform, makepad-draw or any widget crate — so
//! nothing about makepad's renderer comes along for the ride.
//!
//! ## The tree the DSL produces
//!
//! A node is a plain object with a `t` type tag, optional attributes, and an
//! optional `c` array of children:
//!
//! ```text
//! {t: "column", bg: 0xFFF2F2F7, c: [
//!     {t: "text", text: "Components", size: 28, weight: 7},
//!     {t: "button", label: "Tap me", h: 44},
//! ]}
//! ```
//!
//! Deliberately plain data rather than makepad's `Button{...}` component
//! syntax: that syntax resolves through makepad's *widget registry*, which is
//! exactly the coupling this repo exists to avoid. Everything that makes the
//! DSL worth having is still available above this layer — variables, `fn`,
//! loops, conditionals, string building — because a real VM evaluates it. The
//! catalog leans on that heavily (see `assets/catalog.splash`).

use crate::arkui::{attr, event, ty, Node};
use makepad_script::apply::*;
use makepad_script::makepad_live_id::*;
use makepad_script::array::ScriptArrayStorage;
use makepad_script::traits::*;
use makepad_script::*;

/// Evaluate the catalog for a given screen.
///
/// `screen` and `bench` are bound as ordinary `let`s prepended to the source —
/// the simplest way to pass host state into a script without a scope object.
pub fn build_screen(screen: &str, bench: Option<&str>) -> Option<Node> {
    const CATALOG: &str = include_str!("../assets/catalog.splash");
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    let src = format!(
        "let screen = \"{}\"\nlet bench = \"{}\"\n{}",
        esc(screen),
        esc(bench.unwrap_or("")),
        CATALOG
    );
    build(&src)
}

/// Evaluate Splash source and build the native tree it describes.
pub fn build(src: &str) -> Option<Node> {
    let mut std_slot = 0;
    let mut host = 0;
    let vm = &mut ScriptVm {
        host: &mut host,
        std: &mut std_slot,
        bx: Box::new(ScriptVmBase::new()),
    };

    let value = vm.eval(ScriptMod {
        cargo_manifest_path: String::new(),
        module_path: String::from("splash_oh"),
        file: String::from("catalog.splash"),
        line: 0,
        column: 0,
        code: src.to_string(),
        values: Vec::new(),
    });

    if value.is_nil() {
        crate::log("dsl: script evaluated to nil (parse or runtime error)");
        return None;
    }
    walk(vm, value, 0)
}

/// One DSL object → one ArkUI node, recursing into `c`.
fn walk(vm: &mut ScriptVm, value: ScriptValue, depth: usize) -> Option<Node> {
    // A malformed script should not be able to blow the native stack.
    if depth > 32 {
        return None;
    }
    let tag = string_prop(vm, value, id!(t)).unwrap_or_default();
    let node_ty = match tag.as_str() {
        "column" => ty::column(),
        "timepicker" => ty::timepicker(),
        "textpicker" => ty::textpicker(),
        "swiper" => ty::swiper(),
        "grid" => ty::grid(),
        "waterflow" => ty::waterflow(),
        "refresh" => ty::refresh(),
        "list" => ty::list(),
        "row" => ty::row(),
        "stack" => ty::stack(),
        "scroll" => ty::scroll(),
        "text" => ty::text(),
        "button" => ty::button(),
        "toggle" => ty::toggle(),
        "checkbox" => ty::checkbox(),
        "radio" => ty::radio(),
        "slider" => ty::slider(),
        "progress" => ty::progress(),
        "loading" => ty::loading(),
        "input" => ty::input(),
        "textarea" => ty::textarea(),
        "datepicker" => ty::datepicker(),
        "image" => ty::image(),
        other => {
            crate::log(&format!("dsl: unknown node type {other:?}"));
            return None;
        }
    };

    let mut node = Node::new(node_ty)?;

    // --- attributes -------------------------------------------------------
    if let Some(s) = string_prop(vm, value, id!(text)) {
        node = node.text(&s);
    }
    if let Some(s) = string_prop(vm, value, id!(label)) {
        node = node.label(&s);
    }
    if let Some(s) = string_prop(vm, value, id!(placeholder)) {
        node = node.string_attr(attr::input_placeholder(), &s);
    }
    if let Some(v) = num_prop(vm, value, id!(w)) {
        node = node.width(v as f32);
    }
    if let Some(v) = num_prop(vm, value, id!(h)) {
        node = node.height(v as f32);
    }
    if let Some(v) = num_prop(vm, value, id!(size)) {
        node = node.font_size(v as f32);
    }
    if let Some(v) = num_prop(vm, value, id!(weight)) {
        node = node.font_weight(v as i32);
    }
    if let Some(v) = num_prop(vm, value, id!(color)) {
        node = node.font_color(v as u32);
    }
    if let Some(v) = num_prop(vm, value, id!(bg)) {
        node = node.bg(v as u32);
    }
    if let Some(v) = num_prop(vm, value, id!(radius)) {
        node = node.radius(v as f32);
    }
    if let Some(v) = num_prop(vm, value, id!(pad)) {
        node = node.padding(v as f32);
    }
    if let Some(v) = num_prop(vm, value, id!(margin)) {
        node = node.margin(v as f32);
    }
    if let Some(v) = num_prop(vm, value, id!(border)) {
        node = node.f32v_attr(attr::border_width(), &[v as f32; 4]);
    }
    if let Some(v) = num_prop(vm, value, id!(bordercolor)) {
        node = node.u32_attr(attr::border_color(), v as u32);
    }
    if let Some(v) = num_prop(vm, value, id!(value)) {
        node = node.f32_attr(attr::progress_value(), v as f32);
    }
    if let Some(v) = num_prop(vm, value, id!(total)) {
        node = node.f32_attr(attr::progress_total(), v as f32);
    }
    // `align: n` is ArkUI_Alignment. A Scroll whose content is shorter than its
    // own height centres it by default, which drops the page half way down the
    // screen; `align: 1` (TOP) is what a page wants.
    if let Some(v) = num_prop(vm, value, id!(align)) {
        node = node.i32_attr(attr::alignment(), v as i32);
    }

    // `on: 1` is the selected state. Which attribute that maps to depends on
    // the control, so it is resolved from the tag rather than the DSL naming
    // three different keys for one concept.
    if let Some(v) = num_prop(vm, value, id!(on)) {
        let on = v as i32;
        match tag.as_str() {
            "checkbox" => {
                node = node
                    .i32_attr(attr::checkbox_select(), on)
                    .u32_attr(attr::checkbox_color(), 0xFF6750A4);
            }
            "radio" => node = node.i32_attr(attr::radio_checked(), on),
            "toggle" => {
                node = node
                    .i32_attr(attr::toggle_value(), on)
                    .u32_attr(attr::toggle_color(), 0xFF6750A4);
            }
            _ => {}
        }
    }

    // `tap: <id>` wires a click straight back to Rust.
    if let Some(v) = num_prop(vm, value, id!(tap)) {
        node = node.on_event(event::click(), v as i32);
    }

    // --- children ---------------------------------------------------------
    // `c` is a ScriptArray, NOT an object with a vec — arrays are their own
    // heap type in this VM, so `as_object()` on one yields None and the whole
    // subtree silently disappears.
    for kid in children_of(vm, value) {
        if let Some(child) = walk(vm, kid, depth + 1) {
            node = node.child(child);
        }
    }

    Some(node)
}

/// The `c` array's members, copied out so the walk can borrow the vm again.
fn children_of(vm: &mut ScriptVm, value: ScriptValue) -> Vec<ScriptValue> {
    let Some(c) = prop(vm, value, id!(c)) else {
        return Vec::new();
    };
    let Some(arr) = c.as_array() else {
        return Vec::new();
    };
    match vm.bx.heap.array_storage(arr) {
        ScriptArrayStorage::ScriptValue(v) => v.iter().copied().collect(),
        _ => Vec::new(),
    }
}

fn prop(vm: &mut ScriptVm, obj: ScriptValue, key: LiveId) -> Option<ScriptValue> {
    vm.bx.heap.value_for_apply(obj, key.into(), &Apply::Eval)
}

fn string_prop(vm: &mut ScriptVm, obj: ScriptValue, key: LiveId) -> Option<String> {
    let v = prop(vm, obj, key)?;
    vm.string_with(v, |_vm, s| s.to_string())
}

/// Numbers come back through the VM's own coercion rather than bit-poking the
/// NaN-boxed representation, so ints, floats and colour literals all work.
fn num_prop(vm: &mut ScriptVm, obj: ScriptValue, key: LiveId) -> Option<f64> {
    let v = prop(vm, obj, key)?;
    if v.is_nil() {
        return None;
    }
    let mut out: f64 = 0.0;
    <f64 as ScriptApply>::script_apply(&mut out, vm, &Apply::Eval, &mut Scope::default(), v);
    Some(out)
}
