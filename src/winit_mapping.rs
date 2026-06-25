use super::{
    Controls, DrawScheduler, Error, ErrorCode, Fullscreen, Level, Result, Role, Theme,
    WindowRequest,
};
use std::time::Instant;

pub(crate) fn window_attributes_from_request(
    request: &WindowRequest,
) -> Result<winit::window::WindowAttributes> {
    if !matches!(request.role(), Role::Root) {
        return Err(Error::new(
            ErrorCode::UnsupportedFeature,
            "native window roles require parent and modality wiring",
        ));
    }

    let mut attributes = winit::window::Window::default_attributes()
        .with_title(request.title().to_owned())
        .with_resizable(request.resizable())
        .with_enabled_buttons(request.controls().into())
        .with_decorations(request.decorations())
        .with_transparent(request.transparent())
        .with_visible(request.visible())
        .with_window_level(request.level().into())
        .with_theme(request.theme().map(Into::into));

    if let Some(position) = request.position() {
        attributes =
            attributes.with_position(winit::dpi::LogicalPosition::new(position.x, position.y));
    }
    if let Some(size) = request.inner_size() {
        attributes =
            attributes.with_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
    }
    if let Some(size) = request.min_inner_size() {
        attributes =
            attributes.with_min_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
    }
    if let Some(size) = request.max_inner_size() {
        attributes =
            attributes.with_max_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
    }

    attributes = match request.fullscreen() {
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

pub(crate) fn control_flow_from_draw_scheduler(
    draw: &DrawScheduler,
) -> winit::event_loop::ControlFlow {
    native_control_flow(draw.next_deadline().map_or(
        winit::event_loop::ControlFlow::Wait,
        winit::event_loop::ControlFlow::WaitUntil,
    ))
}

pub(crate) fn native_control_flow(
    control_flow: winit::event_loop::ControlFlow,
) -> winit::event_loop::ControlFlow {
    #[cfg(target_os = "macos")]
    {
        if matches!(control_flow, winit::event_loop::ControlFlow::Wait) {
            return winit::event_loop::ControlFlow::WaitUntil(
                Instant::now() + std::time::Duration::from_secs(60 * 60),
            );
        }
    }

    control_flow
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
