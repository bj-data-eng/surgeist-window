use super::{AccessibilityEvent, Id, Metrics, PhysicalPoint, Point, Rect, State, Theme};
use std::time::Instant;

/// Native window event payload emitted by this crate.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    Created(State),
    Destroyed(Id),
    Suspended(Id),
    Resumed(Id),
    CloseRequested(Id),
    Focused { id: Id, focused: bool },
    Resized(Metrics),
    ScaleFactorChanged(Metrics),
    Moved { id: Id, position: Point },
    Occluded { id: Id, occluded: bool },
    ThemeChanged { id: Id, theme: Option<Theme> },
    FileDrag(FileDragEvent),
    Input(InputEvent),
    Accessibility(AccessibilityEvent),
}

impl EventKind {
    #[must_use]
    pub fn id(&self) -> Id {
        match self {
            Self::Created(state) => state.id(),
            Self::Destroyed(id)
            | Self::Suspended(id)
            | Self::Resumed(id)
            | Self::CloseRequested(id) => *id,
            Self::Focused { id, .. }
            | Self::Moved { id, .. }
            | Self::Occluded { id, .. }
            | Self::ThemeChanged { id, .. } => *id,
            Self::Resized(metrics) | Self::ScaleFactorChanged(metrics) => metrics.id,
            Self::FileDrag(event) => event.id(),
            Self::Input(event) => event.id(),
            Self::Accessibility(event) => event.id(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileDragEvent {
    Entered {
        id: Id,
        paths: Vec<String>,
    },
    Hovered {
        id: Id,
        paths: Vec<String>,
        position: Option<Point>,
    },
    Dropped {
        id: Id,
        paths: Vec<String>,
        position: Option<Point>,
    },
    Cancelled {
        id: Id,
    },
}

impl FileDragEvent {
    #[must_use]
    pub fn id(&self) -> Id {
        match self {
            Self::Entered { id, .. }
            | Self::Hovered { id, .. }
            | Self::Dropped { id, .. }
            | Self::Cancelled { id } => *id,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    Pointer(PointerEvent),
    Wheel(WheelEvent),
    Key(KeyEvent),
    Modifiers { id: Id, modifiers: ModifierState },
    Ime(ImeEvent),
    StandardKeyBinding(StandardKeyBindingEvent),
}

impl InputEvent {
    #[must_use]
    pub fn id(&self) -> Id {
        match self {
            Self::Pointer(event) => event.id,
            Self::Wheel(event) => event.id,
            Self::Key(event) => event.id,
            Self::Modifiers { id, .. } => *id,
            Self::Ime(event) => event.id(),
            Self::StandardKeyBinding(event) => event.id,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerPhase {
    Entered,
    Moved,
    Pressed,
    Released,
    Left,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerKind {
    Mouse,
    Touch,
    Pen,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointerEvent {
    pub id: Id,
    pub phase: PointerPhase,
    pub kind: PointerKind,
    pub pointer_id: Option<u64>,
    pub position: Option<Point>,
    pub physical_position: Option<PhysicalPoint>,
    pub delta: Option<Point>,
    pub button: Option<PointerButton>,
    pub modifiers: ModifierState,
    pub device: PointerDeviceData,
    pub timestamp: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerDeviceData {
    pub force: Option<f64>,
    pub pressure: Option<f64>,
    pub tangential_pressure: Option<f64>,
    pub tilt_x: Option<f64>,
    pub tilt_y: Option<f64>,
    pub twist: Option<f64>,
    pub altitude: Option<f64>,
    pub azimuth: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WheelEvent {
    pub id: Id,
    pub delta: WheelDelta,
    pub phase: TouchPhase,
    pub position: Option<Point>,
    pub modifiers: ModifierState,
    pub timestamp: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelDelta {
    Lines { x: f64, y: f64 },
    Pixels { x: f64, y: f64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyEvent {
    pub id: Id,
    pub logical_key: keyboard_types::Key,
    pub physical_key: keyboard_types::Code,
    pub location: keyboard_types::Location,
    pub state: KeyState,
    pub repeat: bool,
    pub synthetic: bool,
    pub modifiers: ModifierState,
    pub timestamp: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModifierState {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImeEvent {
    Enabled {
        id: Id,
    },
    Disabled {
        id: Id,
    },
    Preedit {
        id: Id,
        text: String,
        cursor: Option<(usize, usize)>,
    },
    Commit {
        id: Id,
        text: String,
    },
    DeleteSurrounding {
        id: Id,
        before: usize,
        after: usize,
    },
}

impl ImeEvent {
    #[must_use]
    pub fn id(&self) -> Id {
        match self {
            Self::Enabled { id }
            | Self::Disabled { id }
            | Self::Preedit { id, .. }
            | Self::Commit { id, .. }
            | Self::DeleteSurrounding { id, .. } => *id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StandardKeyBindingEvent {
    pub id: Id,
    pub binding: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImeRequest {
    Disable,
    Enable(ImeConfig),
    Update(ImeConfig),
    Restart(ImeConfig),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImeConfig {
    pub purpose: ImePurpose,
    pub hint: ImeHint,
    pub cursor_area: Option<Rect>,
    pub surrounding_text: Option<ImeSurroundingText>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ImePurpose {
    #[default]
    Normal,
    Password,
    Number,
    Email,
    Url,
    Terminal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ImeHint {
    #[default]
    None,
    Spellcheck,
    NoSpellcheck,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImeSurroundingText {
    pub text: String,
    pub cursor: usize,
    pub anchor: usize,
}
