//! Backend-agnostic UI node model.
//!
//! The Splash DSL evaluates (in the makepad-script VM) to a tree of plain data
//! objects `{t: "...", <attrs>, c: [...]}`. [`crate::build`] walks that into this
//! `UiNode` tree, which carries **no renderer dependency**. Each backend (ArkUI,
//! makepad, …) turns a `UiNode` tree into its own widgets — that is what makes
//! makepad just *one* render backend rather than *the* renderer.

/// Every node type the DSL can name. A backend maps each to one of its widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Column,
    Row,
    Stack,
    Scroll,
    List,
    Grid,
    Waterflow,
    Refresh,
    Swiper,
    Text,
    Image,
    Button,
    Toggle,
    Checkbox,
    Radio,
    Slider,
    Progress,
    Loading,
    Input,
    Textarea,
    DatePicker,
    TimePicker,
    TextPicker,
}

impl NodeKind {
    /// Parse the DSL `t` tag. Unknown tags yield `None` (the node is dropped).
    pub fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "column" => Self::Column,
            "row" => Self::Row,
            "stack" => Self::Stack,
            "scroll" => Self::Scroll,
            "list" => Self::List,
            "grid" => Self::Grid,
            "waterflow" => Self::Waterflow,
            "refresh" => Self::Refresh,
            "swiper" => Self::Swiper,
            "text" => Self::Text,
            "image" => Self::Image,
            "button" => Self::Button,
            "toggle" => Self::Toggle,
            "checkbox" => Self::Checkbox,
            "radio" => Self::Radio,
            "slider" => Self::Slider,
            "progress" => Self::Progress,
            "loading" => Self::Loading,
            "input" => Self::Input,
            "textarea" => Self::Textarea,
            "datepicker" => Self::DatePicker,
            "timepicker" => Self::TimePicker,
            "textpicker" => Self::TextPicker,
            _ => return None,
        })
    }

    /// Whether this kind lays its children out along the main axis vertically
    /// (column-like) — a convenience for simple backends.
    pub fn is_vertical_stack(self) -> bool {
        matches!(self, Self::Column | Self::Scroll | Self::List)
    }
}

/// All attributes a node can carry. Every field is optional; a backend applies
/// the ones it understands and ignores the rest. Colours are `0xAARRGGBB`.
/// `on` / `tap` are left for the backend to resolve against [`NodeKind`] (e.g.
/// `on` means checkbox-select vs toggle-value depending on the kind).
#[derive(Clone, Default, Debug)]
pub struct Attrs {
    pub text: Option<String>,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    /// Image source: a resource ref or an `https://` URL.
    pub src: Option<String>,
    /// ObjectFit-style enum for images.
    pub fit: Option<i32>,
    pub w: Option<f32>,
    pub h: Option<f32>,
    pub size: Option<f32>,
    pub weight: Option<i32>,
    pub color: Option<u32>,
    pub bg: Option<u32>,
    pub radius: Option<f32>,
    pub pad: Option<f32>,
    pub margin: Option<f32>,
    pub border: Option<f32>,
    pub bordercolor: Option<u32>,
    pub value: Option<f32>,
    pub total: Option<f32>,
    pub align: Option<i32>,
    pub on: Option<i32>,
    pub tap: Option<i32>,
}

/// One node in the backend-agnostic tree.
#[derive(Clone, Debug)]
pub struct UiNode {
    pub kind: NodeKind,
    pub attrs: Attrs,
    pub children: Vec<UiNode>,
}

impl UiNode {
    /// Total node count including self — handy for tests and diagnostics.
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(UiNode::count).sum::<usize>()
    }
}
