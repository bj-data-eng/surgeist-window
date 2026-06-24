//! Native window boundary for Surgeist.
//!
//! This module owns the stable app-facing contract around `winit`: native window
//! identity, events, commands, metrics, draw scheduling, clipboard fallback,
//! and live native handles for renderer integration. It does not own renderers,
//! surfaces, UI semantics, layout, hit testing, or application behavior.

#[cfg(feature = "accessibility")]
pub use accesskit_winit;
pub use cursor_icon::CursorIcon;
pub use keyboard_types;
pub use keyboard_types::Code;
pub use raw_window_handle;

mod accessibility;
mod capability;
mod clipboard;
mod command;
mod context;
mod cursor;
mod descriptor;
mod dsl;
mod error;
mod event;
mod geometry;
mod handler;
mod loop_;
mod registry;
mod scheduler;
pub mod testing;
mod transition;
mod winit_adapter;

pub use accessibility::{AccessibilityActionRequest, AccessibilityBridge, AccessibilityEvent};
pub use capability::{CursorCapability, FullscreenMode, HostCapabilities, RoleKind};
pub use clipboard::{Clipboard, ClipboardImage, ClipboardImageRef, MemoryClipboard};
pub use command::Command;
pub use context::Context;
pub use cursor::{Cursor, CursorGrab, CustomCursorId};
pub use descriptor::{
    Controls, Descriptor, Fullscreen, Level, Metrics, Modality, Role, State, Theme, WindowRequest,
    WindowRequestBuilder, WindowSnapshot,
};
pub use dsl::{
    App, Close, Closed, ControlsBuilder, Frame, Input, Open, Ready, Resize, Scope, Selector,
    Target, app, controls, open, point, rect, size,
};
pub use error::{Error, ErrorCode, Result};
pub use event::{
    EventKind, FileDragEvent, ImeConfig, ImeEvent, ImeHint, ImePurpose, ImeRequest,
    ImeSurroundingText, InputEvent, KeyEvent, KeyState, ModifierState, PointerButton,
    PointerDeviceData, PointerEvent, PointerKind, PointerPhase, StandardKeyBindingEvent,
    TouchPhase, WheelDelta, WheelEvent,
};
pub use geometry::{Id, Insets, PhysicalPoint, PhysicalSize, Point, Rect, Size};
pub use handler::Handler;
pub use loop_::Loop;
pub use registry::{Access, Handle, Instance, Proxy, Ref, Registry};
pub use transition::{
    HostCommand, HostCommandPlan, MetricsEvent, NativeEventTransition, WindowStatePatch,
};

pub(crate) use registry::UserEvent;
pub(crate) use scheduler::{DrawScheduler, native_control_flow};

#[cfg(test)]
mod tests;
