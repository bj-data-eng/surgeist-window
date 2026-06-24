use super::{Error, ErrorCode, EventKind, Id, Metrics, Point, Theme, WindowSnapshot};

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
