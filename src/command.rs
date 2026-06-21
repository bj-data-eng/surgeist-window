use super::{
    Controls, Cursor, CursorGrab, Descriptor, Fullscreen, Id, ImeRequest, Level, Point, Size, Theme,
};
use std::time::Instant;

/// Handler result telling the event loop how to continue.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Wait,
    DrawNow(Id),
    DrawNext(Id),
    DrawAt { id: Id, time: Instant },
    CloseRequested(Id),
    Exit,
    Batch(Vec<Action>),
}

/// Native window command consumed by this crate.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Open { descriptor: Descriptor },
    SetTitle { id: Id, title: String },
    SetPosition { id: Id, position: Point },
    SetVisible { id: Id, visible: bool },
    SetResizable { id: Id, resizable: bool },
    SetControls { id: Id, controls: Controls },
    SetDecorations { id: Id, decorations: bool },
    SetTransparent { id: Id, transparent: bool },
    SetInnerSize { id: Id, size: Size },
    SetMinInnerSize { id: Id, size: Option<Size> },
    SetMaxInnerSize { id: Id, size: Option<Size> },
    SetFullscreen { id: Id, fullscreen: Fullscreen },
    SetLevel { id: Id, level: Level },
    SetTheme { id: Id, theme: Option<Theme> },
    SetCursor { id: Id, cursor: Cursor },
    SetCursorGrab { id: Id, grab: CursorGrab },
    SetIme { id: Id, request: ImeRequest },
    RequestUserAttention { id: Id },
    RequestDraw { id: Id },
    Destroy { id: Id },
}
