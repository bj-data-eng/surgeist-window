use super::{Error, ErrorCode, Id, Insets, PhysicalPoint, PhysicalSize, Point, Result, Size};
use crate::{FullscreenMode, RoleKind};

/// Native fullscreen intent.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Fullscreen {
    #[default]
    None,
    Borderless,
    Exclusive,
}

impl Fullscreen {
    #[must_use]
    pub const fn mode(&self) -> FullscreenMode {
        match self {
            Self::None => FullscreenMode::None,
            Self::Borderless => FullscreenMode::Borderless,
            Self::Exclusive => FullscreenMode::Exclusive,
        }
    }
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

impl Role {
    #[must_use]
    pub const fn kind(&self) -> RoleKind {
        match self {
            Self::Root => RoleKind::Root,
            Self::Dialog { .. } => RoleKind::Dialog,
            Self::Tool { .. } => RoleKind::Tool,
            Self::Popup { .. } => RoleKind::Popup,
        }
    }
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
pub struct WindowRequest {
    title: String,
    name: Option<String>,
    position: Option<Point>,
    inner_size: Option<Size>,
    min_inner_size: Option<Size>,
    max_inner_size: Option<Size>,
    resizable: bool,
    controls: Controls,
    decorations: bool,
    transparent: bool,
    visible: bool,
    fullscreen: Fullscreen,
    level: Level,
    theme: Option<Theme>,
    role: Role,
}

pub type Descriptor = WindowRequest;

#[derive(Clone, Debug, PartialEq)]
pub struct WindowRequestBuilder {
    pub(crate) request: WindowRequest,
}

impl Default for WindowRequest {
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

impl WindowRequest {
    #[must_use]
    pub fn builder(name: impl Into<String>) -> WindowRequestBuilder {
        WindowRequestBuilder {
            request: Self {
                name: Some(name.into()),
                ..Self::default()
            },
        }
    }

    #[must_use]
    pub fn title(&self) -> &str {
        self.title.as_str()
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub const fn position(&self) -> Option<Point> {
        self.position
    }

    #[must_use]
    pub const fn inner_size(&self) -> Option<Size> {
        self.inner_size
    }

    #[must_use]
    pub const fn min_inner_size(&self) -> Option<Size> {
        self.min_inner_size
    }

    #[must_use]
    pub const fn max_inner_size(&self) -> Option<Size> {
        self.max_inner_size
    }

    #[must_use]
    pub const fn resizable(&self) -> bool {
        self.resizable
    }

    #[must_use]
    pub const fn controls(&self) -> Controls {
        self.controls
    }

    #[must_use]
    pub const fn decorations(&self) -> bool {
        self.decorations
    }

    #[must_use]
    pub const fn transparent(&self) -> bool {
        self.transparent
    }

    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    #[must_use]
    pub fn fullscreen(&self) -> Fullscreen {
        self.fullscreen.clone()
    }

    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    #[must_use]
    pub const fn theme(&self) -> Option<Theme> {
        self.theme
    }

    #[must_use]
    pub const fn role(&self) -> &Role {
        &self.role
    }

    #[cfg(feature = "accessibility")]
    pub(crate) const fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub(crate) const fn set_min_inner_size(&mut self, size: Option<Size>) {
        self.min_inner_size = size;
    }

    pub(crate) const fn set_max_inner_size(&mut self, size: Option<Size>) {
        self.max_inner_size = size;
    }

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

impl WindowRequestBuilder {
    #[must_use]
    pub fn build(self) -> WindowRequest {
        self.request
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.request.title = title.into();
        self
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.request.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn position(mut self, point: impl Into<Point>) -> Self {
        self.request.position = Some(point.into());
        self
    }

    #[must_use]
    pub fn inner_size(mut self, size: impl Into<Size>) -> Self {
        self.request.inner_size = Some(size.into());
        self
    }

    #[must_use]
    pub fn min_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.request.min_inner_size = Some(size.into());
        self
    }

    #[must_use]
    pub fn max_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.request.max_inner_size = Some(size.into());
        self
    }

    #[must_use]
    pub const fn resizable(mut self, resizable: bool) -> Self {
        self.request.resizable = resizable;
        self
    }

    #[must_use]
    pub const fn fixed(self) -> Self {
        self.resizable(false)
    }

    #[must_use]
    pub fn controls(mut self, controls: impl Into<Controls>) -> Self {
        self.request.controls = controls.into();
        self
    }

    #[must_use]
    pub const fn decorations(mut self, enabled: bool) -> Self {
        self.request.decorations = enabled;
        self
    }

    #[must_use]
    pub const fn transparent(mut self, transparent: bool) -> Self {
        self.request.transparent = transparent;
        self
    }

    #[must_use]
    pub const fn visible(mut self, visible: bool) -> Self {
        self.request.visible = visible;
        self
    }

    #[must_use]
    pub const fn hidden(self) -> Self {
        self.visible(false)
    }

    #[must_use]
    pub fn fullscreen(mut self, fullscreen: impl Into<Fullscreen>) -> Self {
        self.request.fullscreen = fullscreen.into();
        self
    }

    #[must_use]
    pub fn borderless(mut self) -> Self {
        self.request.fullscreen = Fullscreen::Borderless;
        self
    }

    #[must_use]
    pub const fn level(mut self, level: Level) -> Self {
        self.request.level = level;
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: impl Into<Option<Theme>>) -> Self {
        self.request.theme = theme.into();
        self
    }

    #[must_use]
    pub const fn role(mut self, role: Role) -> Self {
        self.request.role = role;
        self
    }

    #[must_use]
    pub const fn root(self) -> Self {
        self.role(Role::Root)
    }

    #[must_use]
    pub const fn dialog(self, parent: Id) -> Self {
        self.role(Role::Dialog {
            parent,
            modality: Modality::Window,
        })
    }

    #[must_use]
    pub const fn modal(self, parent: Id, modality: Modality) -> Self {
        self.role(Role::Dialog { parent, modality })
    }

    #[must_use]
    pub const fn tool(self, parent: Option<Id>) -> Self {
        self.role(Role::Tool { parent })
    }

    #[must_use]
    pub const fn popup(self, parent: Id) -> Self {
        self.role(Role::Popup { parent })
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
pub struct WindowSnapshot {
    id: Id,
    title: String,
    name: Option<String>,
    metrics: Metrics,
    position: Option<Point>,
    focused: bool,
    visible: Option<bool>,
    minimized: Option<bool>,
    maximized: bool,
    occluded: Option<bool>,
    fullscreen: bool,
    theme: Option<Theme>,
    role: Role,
}

pub type State = WindowSnapshot;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WindowSnapshotSeed {
    pub(crate) id: Id,
    pub(crate) title: String,
    pub(crate) name: Option<String>,
    pub(crate) metrics: Metrics,
    pub(crate) position: Option<Point>,
    pub(crate) focused: bool,
    pub(crate) visible: Option<bool>,
    pub(crate) minimized: Option<bool>,
    pub(crate) maximized: bool,
    pub(crate) occluded: Option<bool>,
    pub(crate) fullscreen: bool,
    pub(crate) theme: Option<Theme>,
    pub(crate) role: Role,
}

impl WindowSnapshot {
    #[must_use]
    pub fn new(id: Id, title: impl Into<String>, metrics: Metrics) -> Self {
        Self {
            id,
            title: title.into(),
            name: None,
            metrics,
            position: None,
            focused: false,
            visible: None,
            minimized: None,
            maximized: false,
            occluded: None,
            fullscreen: false,
            theme: None,
            role: Role::Root,
        }
    }

    #[must_use]
    pub(crate) fn from_seed(seed: WindowSnapshotSeed) -> Self {
        Self {
            id: seed.id,
            title: seed.title,
            name: seed.name,
            metrics: seed.metrics,
            position: seed.position,
            focused: seed.focused,
            visible: seed.visible,
            minimized: seed.minimized,
            maximized: seed.maximized,
            occluded: seed.occluded,
            fullscreen: seed.fullscreen,
            theme: seed.theme,
            role: seed.role,
        }
    }

    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    #[must_use]
    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    #[must_use]
    pub const fn id(&self) -> Id {
        self.id
    }

    #[must_use]
    pub const fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    #[must_use]
    pub const fn position(&self) -> Option<Point> {
        self.position
    }

    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    #[must_use]
    pub fn title(&self) -> &str {
        self.title.as_str()
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub const fn visible(&self) -> Option<bool> {
        self.visible
    }

    #[must_use]
    pub const fn theme(&self) -> Option<Theme> {
        self.theme
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible.unwrap_or(true)
    }

    #[must_use]
    pub fn is_occluded(&self) -> bool {
        self.occluded.unwrap_or(false)
    }

    #[must_use]
    pub const fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub(crate) const fn set_visible(&mut self, visible: Option<bool>) {
        self.visible = visible;
    }

    pub(crate) fn set_metrics(&mut self, metrics: Metrics) {
        self.metrics = metrics;
    }

    pub(crate) const fn set_position(&mut self, position: Option<Point>) {
        self.position = position;
    }

    pub(crate) const fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub(crate) const fn set_theme(&mut self, theme: Option<Theme>) {
        self.theme = theme;
    }

    pub(crate) const fn set_occluded(&mut self, occluded: Option<bool>) {
        self.occluded = occluded;
    }

    pub(crate) const fn set_fullscreen(&mut self, fullscreen: bool) {
        self.fullscreen = fullscreen;
    }
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
    pub const fn id(&self) -> Id {
        self.id
    }

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
