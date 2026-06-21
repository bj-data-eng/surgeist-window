use super::{Error, ErrorCode, Id, Insets, PhysicalPoint, PhysicalSize, Point, Result, Size};

/// Native fullscreen intent.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Fullscreen {
    #[default]
    None,
    Borderless,
    Exclusive,
}

/// Native window stacking intent.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Level {
    #[default]
    Normal,
    AlwaysOnTop,
    AlwaysOnBottom,
}

/// Native window control availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Controls {
    pub close: bool,
    pub minimize: bool,
    pub maximize: bool,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            close: true,
            minimize: true,
            maximize: true,
        }
    }
}

/// Observed or requested native appearance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

/// Native window role and parent relationship.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Role {
    #[default]
    Root,
    Dialog {
        parent: Id,
        modality: Modality,
    },
    Tool {
        parent: Option<Id>,
    },
    Popup {
        parent: Id,
    },
}

/// Dialog blocking intent.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Modality {
    Window,
    App,
    #[default]
    Modeless,
}

/// Requested native window configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct Descriptor {
    pub title: String,
    pub name: Option<String>,
    pub position: Option<Point>,
    pub inner_size: Option<Size>,
    pub min_inner_size: Option<Size>,
    pub max_inner_size: Option<Size>,
    pub resizable: bool,
    pub controls: Controls,
    pub decorations: bool,
    pub transparent: bool,
    pub visible: bool,
    pub fullscreen: Fullscreen,
    pub level: Level,
    pub theme: Option<Theme>,
    pub role: Role,
}

impl Default for Descriptor {
    fn default() -> Self {
        Self {
            title: String::from("Surgeist"),
            name: None,
            position: None,
            inner_size: None,
            min_inner_size: None,
            max_inner_size: None,
            resizable: true,
            controls: Controls::default(),
            decorations: true,
            transparent: false,
            visible: true,
            fullscreen: Fullscreen::None,
            level: Level::Normal,
            theme: None,
            role: Role::Root,
        }
    }
}

impl Descriptor {
    /// Convert this stable descriptor into `winit` window attributes.
    ///
    /// This is intentionally explicit rather than a blanket `From` impl because
    /// some Surgeist intents may need diagnostics when a native backend cannot
    /// express them.
    pub(crate) fn to_winit_attributes(&self) -> Result<winit::window::WindowAttributes> {
        if !matches!(self.role, Role::Root) {
            return Err(Error::new(
                ErrorCode::UnsupportedFeature,
                "native window roles require parent and modality wiring",
            ));
        }

        let mut attributes = winit::window::Window::default_attributes()
            .with_title(self.title.clone())
            .with_resizable(self.resizable)
            .with_enabled_buttons(self.controls.into())
            .with_decorations(self.decorations)
            .with_transparent(self.transparent)
            .with_visible(self.visible)
            .with_window_level(self.level.into())
            .with_theme(self.theme.map(Into::into));

        if let Some(position) = self.position {
            attributes =
                attributes.with_position(winit::dpi::LogicalPosition::new(position.x, position.y));
        }
        if let Some(size) = self.inner_size {
            attributes =
                attributes.with_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
        }
        if let Some(size) = self.min_inner_size {
            attributes = attributes
                .with_min_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
        }
        if let Some(size) = self.max_inner_size {
            attributes = attributes
                .with_max_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
        }

        attributes = match self.fullscreen {
            Fullscreen::None => attributes.with_fullscreen(None),
            Fullscreen::Borderless => {
                attributes.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
            }
            Fullscreen::Exclusive => {
                return Err(Error::new(
                    ErrorCode::CommandFailed,
                    "exclusive fullscreen requires a native video mode",
                ));
            }
        };

        Ok(attributes)
    }
}

impl From<Controls> for winit::window::WindowButtons {
    fn from(controls: Controls) -> Self {
        let mut buttons = Self::empty();
        if controls.close {
            buttons |= Self::CLOSE;
        }
        if controls.minimize {
            buttons |= Self::MINIMIZE;
        }
        if controls.maximize {
            buttons |= Self::MAXIMIZE;
        }
        buttons
    }
}

impl From<Level> for winit::window::WindowLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::Normal => Self::Normal,
            Level::AlwaysOnTop => Self::AlwaysOnTop,
            Level::AlwaysOnBottom => Self::AlwaysOnBottom,
        }
    }
}

impl From<Theme> for winit::window::Theme {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::Light,
            Theme::Dark => Self::Dark,
        }
    }
}

/// Observed native window state.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub id: Id,
    pub title: String,
    pub name: Option<String>,
    pub metrics: Metrics,
    pub position: Option<Point>,
    pub focused: bool,
    pub visible: Option<bool>,
    pub minimized: Option<bool>,
    pub maximized: bool,
    pub occluded: Option<bool>,
    pub fullscreen: bool,
    pub theme: Option<Theme>,
    pub role: Role,
}

/// Observed native geometry and scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Metrics {
    pub id: Id,
    pub logical_size: Size,
    pub physical_size: PhysicalSize,
    pub outer_position: Option<Point>,
    pub outer_size: Option<Size>,
    pub scale_factor: f64,
    pub safe_area: Insets,
}

impl Metrics {
    #[must_use]
    pub fn from_physical_size(id: Id, physical_size: PhysicalSize, scale_factor: f64) -> Self {
        let scale = if scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        Self {
            id,
            logical_size: Size {
                width: f64::from(physical_size.width) / scale,
                height: f64::from(physical_size.height) / scale,
            },
            physical_size,
            outer_position: None,
            outer_size: None,
            scale_factor: scale,
            safe_area: Insets::default(),
        }
    }

    #[must_use]
    pub fn with_outer_geometry(
        mut self,
        outer_position: Option<Point>,
        outer_size: Option<Size>,
    ) -> Self {
        self.outer_position = outer_position;
        self.outer_size = outer_size;
        self
    }

    #[must_use]
    pub fn logical_to_physical_point(&self, point: Point) -> PhysicalPoint {
        PhysicalPoint {
            x: (point.x * self.scale_factor).round() as i32,
            y: (point.y * self.scale_factor).round() as i32,
        }
    }

    #[must_use]
    pub fn physical_to_logical_point(&self, point: PhysicalPoint) -> Point {
        Point {
            x: f64::from(point.x) / self.scale_factor,
            y: f64::from(point.y) / self.scale_factor,
        }
    }
}
