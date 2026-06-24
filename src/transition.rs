use super::{
    Command, Controls, Cursor, CursorGrab, Error, ErrorCode, EventKind, Fullscreen,
    HostCapabilities, Id, ImeRequest, Level, Metrics, Point, Size, Theme, WindowRequest,
    WindowSnapshot,
};

#[derive(Clone, Debug, PartialEq)]
pub enum HostCommand {
    Open { request: WindowRequest },
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

#[derive(Clone, Debug, PartialEq)]
pub struct HostCommandPlan {
    command: HostCommand,
}

impl HostCommandPlan {
    pub fn from_command(command: Command, capabilities: &HostCapabilities) -> Result<Self, Error> {
        let command = match command {
            Command::Open { request } => {
                capabilities.require_role(request.role().kind())?;
                capabilities.require_fullscreen(request.fullscreen().mode())?;
                HostCommand::Open { request }
            }
            Command::SetFullscreen { id, fullscreen } => {
                capabilities.require_fullscreen(fullscreen.mode())?;
                HostCommand::SetFullscreen { id, fullscreen }
            }
            Command::SetCursor { id, cursor } => {
                capabilities.require_cursor(cursor.capability())?;
                HostCommand::SetCursor { id, cursor }
            }
            Command::SetTitle { id, title } => HostCommand::SetTitle { id, title },
            Command::SetPosition { id, position } => HostCommand::SetPosition { id, position },
            Command::SetVisible { id, visible } => HostCommand::SetVisible { id, visible },
            Command::SetResizable { id, resizable } => HostCommand::SetResizable { id, resizable },
            Command::SetControls { id, controls } => HostCommand::SetControls { id, controls },
            Command::SetDecorations { id, decorations } => {
                HostCommand::SetDecorations { id, decorations }
            }
            Command::SetTransparent { id, transparent } => {
                HostCommand::SetTransparent { id, transparent }
            }
            Command::SetInnerSize { id, size } => HostCommand::SetInnerSize { id, size },
            Command::SetMinInnerSize { id, size } => HostCommand::SetMinInnerSize { id, size },
            Command::SetMaxInnerSize { id, size } => HostCommand::SetMaxInnerSize { id, size },
            Command::SetLevel { id, level } => HostCommand::SetLevel { id, level },
            Command::SetTheme { id, theme } => HostCommand::SetTheme { id, theme },
            Command::SetCursorGrab { id, grab } => HostCommand::SetCursorGrab { id, grab },
            Command::SetIme { id, request } => HostCommand::SetIme { id, request },
            Command::RequestUserAttention { id } => HostCommand::RequestUserAttention { id },
            Command::RequestDraw { id } => HostCommand::RequestDraw { id },
            Command::Destroy { id } => HostCommand::Destroy { id },
        };

        Ok(Self { command })
    }

    #[must_use]
    pub fn command(&self) -> &HostCommand {
        &self.command
    }

    #[must_use]
    pub fn into_command(self) -> HostCommand {
        self.command
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowStatePatch {
    Title {
        id: Id,
        title: String,
    },
    Position {
        id: Id,
        position: Point,
    },
    Visible {
        id: Id,
        visible: bool,
    },
    Metrics {
        metrics: Metrics,
        event: MetricsEvent,
    },
    Focused {
        id: Id,
        focused: bool,
    },
    Theme {
        id: Id,
        theme: Option<Theme>,
    },
    Occluded {
        id: Id,
        occluded: bool,
    },
    Fullscreen {
        id: Id,
        fullscreen: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricsEvent {
    Resized,
    ScaleFactorChanged,
}

impl WindowStatePatch {
    #[must_use]
    pub fn title(id: Id, title: impl Into<String>) -> Self {
        Self::Title {
            id,
            title: title.into(),
        }
    }

    #[must_use]
    pub const fn visible(id: Id, visible: bool) -> Self {
        Self::Visible { id, visible }
    }

    #[must_use]
    pub const fn metrics(metrics: Metrics, event: MetricsEvent) -> Self {
        Self::Metrics { metrics, event }
    }

    #[must_use]
    pub const fn id(&self) -> Id {
        match self {
            Self::Title { id, .. }
            | Self::Position { id, .. }
            | Self::Visible { id, .. }
            | Self::Focused { id, .. }
            | Self::Theme { id, .. }
            | Self::Occluded { id, .. }
            | Self::Fullscreen { id, .. } => *id,
            Self::Metrics { metrics, .. } => metrics.id(),
        }
    }

    pub fn apply(self, snapshot: &mut WindowSnapshot) -> Result<Option<EventKind>, Error> {
        let id = self.id();
        if snapshot.id() != id {
            return Err(Error::new(
                ErrorCode::CommandFailed,
                "patch target does not match window",
            )
            .with_id(id));
        }

        match self {
            Self::Title { title, .. } => {
                snapshot.set_title(title);
                Ok(None)
            }
            Self::Position { position, .. } => {
                snapshot.set_position(Some(position));
                Ok(Some(EventKind::Moved { id, position }))
            }
            Self::Visible { visible, .. } => {
                snapshot.set_visible(Some(visible));
                Ok(None)
            }
            Self::Metrics { metrics, event } => {
                snapshot.set_metrics(metrics.clone());
                let event = match event {
                    MetricsEvent::Resized => EventKind::Resized(metrics),
                    MetricsEvent::ScaleFactorChanged => EventKind::ScaleFactorChanged(metrics),
                };
                Ok(Some(event))
            }
            Self::Focused { focused, .. } => {
                snapshot.set_focused(focused);
                Ok(Some(EventKind::Focused { id, focused }))
            }
            Self::Theme { theme, .. } => {
                snapshot.set_theme(theme);
                Ok(Some(EventKind::ThemeChanged { id, theme }))
            }
            Self::Occluded { occluded, .. } => {
                snapshot.set_occluded(Some(occluded));
                Ok(Some(EventKind::Occluded { id, occluded }))
            }
            Self::Fullscreen { fullscreen, .. } => {
                snapshot.set_fullscreen(fullscreen);
                Ok(None)
            }
        }
    }
}
