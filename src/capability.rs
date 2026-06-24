use super::{Error, ErrorCode};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RoleKind {
    Root,
    Dialog,
    Tool,
    Popup,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FullscreenMode {
    None,
    Borderless,
    Exclusive,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CursorCapability {
    Icon,
    Hidden,
    Custom,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostCapabilities {
    root_role: bool,
    dialog_role: bool,
    tool_role: bool,
    popup_role: bool,
    borderless_fullscreen: bool,
    exclusive_fullscreen: bool,
    icon_cursor: bool,
    hidden_cursor: bool,
    custom_cursor: bool,
}

impl HostCapabilities {
    #[must_use]
    pub const fn winit_default() -> Self {
        Self {
            root_role: true,
            dialog_role: false,
            tool_role: false,
            popup_role: false,
            borderless_fullscreen: true,
            exclusive_fullscreen: false,
            icon_cursor: true,
            hidden_cursor: true,
            custom_cursor: false,
        }
    }

    #[must_use]
    pub const fn supports_role(&self, role: RoleKind) -> bool {
        match role {
            RoleKind::Root => self.root_role,
            RoleKind::Dialog => self.dialog_role,
            RoleKind::Tool => self.tool_role,
            RoleKind::Popup => self.popup_role,
        }
    }

    pub fn require_role(&self, role: RoleKind) -> Result<(), Error> {
        self.supports_role(role).then_some(()).ok_or_else(|| {
            Error::new(
                ErrorCode::UnsupportedFeature,
                format!("native host does not support {role:?} window role"),
            )
        })
    }

    #[must_use]
    pub const fn supports_fullscreen(&self, fullscreen: FullscreenMode) -> bool {
        match fullscreen {
            FullscreenMode::None => true,
            FullscreenMode::Borderless => self.borderless_fullscreen,
            FullscreenMode::Exclusive => self.exclusive_fullscreen,
        }
    }

    pub fn require_fullscreen(&self, fullscreen: FullscreenMode) -> Result<(), Error> {
        self.supports_fullscreen(fullscreen)
            .then_some(())
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::UnsupportedFeature,
                    format!("native host does not support {fullscreen:?} fullscreen"),
                )
            })
    }

    #[must_use]
    pub const fn supports_cursor(&self, cursor: CursorCapability) -> bool {
        match cursor {
            CursorCapability::Icon => self.icon_cursor,
            CursorCapability::Hidden => self.hidden_cursor,
            CursorCapability::Custom => self.custom_cursor,
        }
    }

    pub fn require_cursor(&self, cursor: CursorCapability) -> Result<(), Error> {
        self.supports_cursor(cursor).then_some(()).ok_or_else(|| {
            Error::new(
                ErrorCode::UnsupportedFeature,
                format!("native host does not support {cursor:?} cursor"),
            )
        })
    }
}
