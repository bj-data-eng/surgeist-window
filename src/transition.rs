use super::{
    Command, Error, ErrorCode, EventKind, HostCapabilities, Id, InputEvent, Metrics, ModifierState,
    PhysicalPoint, Point, PointerDeviceData, PointerEvent, PointerKind, PointerPhase, Theme,
    WindowSnapshot,
};

#[derive(Clone, Debug, PartialEq)]
pub struct HostCommandPlan {
    command: Command,
}

impl HostCommandPlan {
    pub fn from_command(command: Command, capabilities: &HostCapabilities) -> Result<Self, Error> {
        match &command {
            Command::Open { request } => {
                capabilities.require_role(request.role().kind())?;
                capabilities.require_fullscreen(request.fullscreen().mode())?;
            }
            Command::SetFullscreen { fullscreen, .. } => {
                capabilities.require_fullscreen(fullscreen.mode())?;
            }
            Command::SetCursor { cursor, .. } => {
                capabilities.require_cursor(cursor.capability())?;
            }
            Command::SetTitle { .. }
            | Command::SetPosition { .. }
            | Command::SetVisible { .. }
            | Command::SetResizable { .. }
            | Command::SetControls { .. }
            | Command::SetDecorations { .. }
            | Command::SetTransparent { .. }
            | Command::SetInnerSize { .. }
            | Command::SetMinInnerSize { .. }
            | Command::SetMaxInnerSize { .. }
            | Command::SetLevel { .. }
            | Command::SetTheme { .. }
            | Command::SetCursorGrab { .. }
            | Command::SetIme { .. }
            | Command::RequestUserAttention { .. }
            | Command::RequestDraw { .. }
            | Command::Destroy { .. } => {}
        }

        Ok(Self { command })
    }

    #[must_use]
    pub fn command(&self) -> &Command {
        &self.command
    }

    #[must_use]
    pub fn into_command(self) -> Command {
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

#[derive(Clone, Debug, PartialEq)]
pub struct NativeEventTransition {
    patch: Option<WindowStatePatch>,
    event: Option<EventKind>,
}

impl NativeEventTransition {
    #[must_use]
    pub const fn new(patch: Option<WindowStatePatch>, event: Option<EventKind>) -> Self {
        Self { patch, event }
    }

    #[must_use]
    pub fn focused(id: Id, focused: bool) -> Self {
        let patch = WindowStatePatch::Focused { id, focused };
        let event = EventKind::Focused { id, focused };
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn moved(id: Id, position: Point) -> Self {
        let patch = WindowStatePatch::Position { id, position };
        let event = EventKind::Moved { id, position };
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn theme_changed(id: Id, theme: Option<Theme>) -> Self {
        let patch = WindowStatePatch::Theme { id, theme };
        let event = EventKind::ThemeChanged { id, theme };
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn occluded(id: Id, occluded: bool) -> Self {
        let patch = WindowStatePatch::Occluded { id, occluded };
        let event = EventKind::Occluded { id, occluded };
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn resized(metrics: Metrics) -> Self {
        let event = EventKind::Resized(metrics.clone());
        let patch = WindowStatePatch::metrics(metrics, MetricsEvent::Resized);
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn scale_factor_changed(metrics: Metrics) -> Self {
        let event = EventKind::ScaleFactorChanged(metrics.clone());
        let patch = WindowStatePatch::metrics(metrics, MetricsEvent::ScaleFactorChanged);
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn mouse_moved(
        id: Id,
        position: Point,
        physical_position: PhysicalPoint,
        delta: Option<Point>,
        modifiers: ModifierState,
    ) -> Self {
        Self::new(
            None,
            Some(EventKind::Input(InputEvent::Pointer(PointerEvent {
                id,
                phase: PointerPhase::Moved,
                kind: PointerKind::Mouse,
                pointer_id: None,
                position: Some(position),
                physical_position: Some(physical_position),
                delta,
                button: None,
                modifiers,
                device: PointerDeviceData::default(),
                timestamp: None,
            }))),
        )
    }

    #[must_use]
    pub fn patch(&self) -> Option<&WindowStatePatch> {
        self.patch.as_ref()
    }

    #[must_use]
    pub fn event(&self) -> Option<&EventKind> {
        self.event.as_ref()
    }

    #[must_use]
    pub fn into_event(self) -> Option<EventKind> {
        self.event
    }
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
