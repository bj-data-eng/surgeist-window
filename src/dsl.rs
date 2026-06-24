use super::{
    Command, Context, Controls, Cursor, CursorGrab, Error, ErrorCode, Fullscreen, Handle, Id,
    ImeRequest, InputEvent, Level, Metrics, Modality, Point, Proxy, Rect, Ref, Result, Role, Size,
    Theme, WindowRequest, WindowRequestBuilder, WindowSnapshot, command::Action,
};
use std::{collections::HashSet, time::Instant};

#[must_use]
pub fn point(x: impl Into<f64>, y: impl Into<f64>) -> Point {
    Point {
        x: x.into(),
        y: y.into(),
    }
}

#[must_use]
pub fn size(width: impl Into<f64>, height: impl Into<f64>) -> Size {
    Size {
        width: width.into(),
        height: height.into(),
    }
}

#[must_use]
pub fn rect(
    x: impl Into<f64>,
    y: impl Into<f64>,
    width: impl Into<f64>,
    height: impl Into<f64>,
) -> Rect {
    Rect {
        origin: point(x, y),
        size: size(width, height),
    }
}

#[must_use]
pub fn controls() -> ControlsBuilder {
    ControlsBuilder::default()
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ControlsBuilder {
    controls: Controls,
}

impl ControlsBuilder {
    #[must_use]
    pub fn close(mut self, enabled: bool) -> Self {
        self.controls.close = enabled;
        self
    }

    #[must_use]
    pub fn minimize(mut self, enabled: bool) -> Self {
        self.controls.minimize = enabled;
        self
    }

    #[must_use]
    pub fn maximize(mut self, enabled: bool) -> Self {
        self.controls.maximize = enabled;
        self
    }

    #[must_use]
    pub fn all(mut self, enabled: bool) -> Self {
        self.controls = Controls {
            close: enabled,
            minimize: enabled,
            maximize: enabled,
        };
        self
    }

    #[must_use]
    pub fn build(self) -> Controls {
        self.controls
    }
}

impl From<ControlsBuilder> for Controls {
    fn from(builder: ControlsBuilder) -> Self {
        builder.build()
    }
}

#[must_use]
pub fn open(name: impl Into<String>) -> Open {
    Open::unnamed().name(name)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Open {
    builder: WindowRequestBuilder,
}

impl Open {
    #[must_use]
    pub fn unnamed() -> Self {
        Self {
            builder: WindowRequestBuilder {
                request: WindowRequest::default(),
            },
        }
    }

    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            builder: WindowRequest::builder(name),
        }
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.builder = self.builder.name(name);
        self
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.builder = self.builder.title(title);
        self
    }

    #[must_use]
    pub fn position(mut self, point: impl Into<Point>) -> Self {
        self.builder = self.builder.position(point);
        self
    }

    #[must_use]
    pub fn at(self, point: impl Into<Point>) -> Self {
        self.position(point)
    }

    #[must_use]
    pub fn inner_size(mut self, size: impl Into<Size>) -> Self {
        self.builder = self.builder.inner_size(size);
        self
    }

    #[must_use]
    pub fn size(self, size: impl Into<Size>) -> Self {
        self.inner_size(size)
    }

    #[must_use]
    pub fn min_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.builder = self.builder.min_inner_size(size);
        self
    }

    #[must_use]
    pub fn min(mut self, size: impl Into<Option<Size>>) -> Self {
        self.builder.request.set_min_inner_size(size.into());
        self
    }

    #[must_use]
    pub fn max_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.builder = self.builder.max_inner_size(size);
        self
    }

    #[must_use]
    pub fn max(mut self, size: impl Into<Option<Size>>) -> Self {
        self.builder.request.set_max_inner_size(size.into());
        self
    }

    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.builder = self.builder.resizable(resizable);
        self
    }

    #[must_use]
    pub fn fixed(mut self) -> Self {
        self.builder = self.builder.fixed();
        self
    }

    #[must_use]
    pub fn controls(mut self, controls: impl Into<Controls>) -> Self {
        self.builder = self.builder.controls(controls);
        self
    }

    #[must_use]
    pub fn decorations(mut self, enabled: bool) -> Self {
        self.builder = self.builder.decorations(enabled);
        self
    }

    #[must_use]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.builder = self.builder.transparent(transparent);
        self
    }

    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.builder = self.builder.visible(visible);
        self
    }

    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.builder = self.builder.hidden();
        self
    }

    #[must_use]
    pub fn fullscreen(mut self, fullscreen: impl Into<Fullscreen>) -> Self {
        self.builder = self.builder.fullscreen(fullscreen);
        self
    }

    #[must_use]
    pub fn borderless(mut self) -> Self {
        self.builder = self.builder.borderless();
        self
    }

    #[must_use]
    pub fn level(mut self, level: Level) -> Self {
        self.builder = self.builder.level(level);
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: impl Into<Option<Theme>>) -> Self {
        self.builder = self.builder.theme(theme);
        self
    }

    #[must_use]
    pub fn role(mut self, role: Role) -> Self {
        self.builder = self.builder.role(role);
        self
    }

    #[must_use]
    pub fn root(mut self) -> Self {
        self.builder = self.builder.root();
        self
    }

    #[must_use]
    pub fn dialog(mut self, parent: Id) -> Self {
        self.builder = self.builder.dialog(parent);
        self
    }

    #[must_use]
    pub fn modal(mut self, modality: Modality) -> Self {
        if let Role::Dialog { parent, .. } = self.builder.request.role().clone() {
            self.builder = self.builder.modal(parent, modality);
        }
        self
    }

    #[must_use]
    pub fn tool(mut self, parent: Option<Id>) -> Self {
        self.builder = self.builder.tool(parent);
        self
    }

    #[must_use]
    pub fn popup(mut self, parent: Id) -> Self {
        self.builder = self.builder.popup(parent);
        self
    }

    #[must_use]
    pub fn request(&self) -> &WindowRequest {
        &self.builder.request
    }

    #[must_use]
    pub fn into_request(self) -> WindowRequest {
        self.builder.build()
    }

    #[must_use]
    pub fn build(self) -> WindowRequest {
        self.into_request()
    }

    #[must_use]
    pub fn into_command(self) -> Command {
        Command::Open {
            request: self.into_request(),
        }
    }
}

impl From<Open> for WindowRequest {
    fn from(open: Open) -> Self {
        open.into_request()
    }
}

impl From<Open> for Command {
    fn from(open: Open) -> Self {
        open.into_command()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selector {
    Id(Id),
    Name(String),
}

impl From<Id> for Selector {
    fn from(id: Id) -> Self {
        Self::Id(id)
    }
}

impl From<&str> for Selector {
    fn from(name: &str) -> Self {
        Self::Name(name.to_owned())
    }
}

impl From<String> for Selector {
    fn from(name: String) -> Self {
        Self::Name(name)
    }
}

pub struct Target<'a> {
    commands: &'a mut Vec<Command>,
    actions: &'a mut Vec<Action>,
    action: &'a mut Action,
    id: Id,
}

impl<'a> Target<'a> {
    pub(crate) fn new(
        id: Id,
        commands: &'a mut Vec<Command>,
        actions: &'a mut Vec<Action>,
        action: &'a mut Action,
    ) -> Self {
        Self {
            commands,
            actions,
            action,
            id,
        }
    }

    fn send(&mut self, command: Command) {
        self.commands.push(command);
    }

    fn request(&mut self, action: Action) {
        self.actions.push(action);
        *self.action = super::context::resolve_actions(self.actions);
    }

    pub fn title(&mut self, title: impl Into<String>) -> &mut Self {
        self.send(Command::SetTitle {
            id: self.id,
            title: title.into(),
        });
        self
    }

    pub fn at(&mut self, point: impl Into<Point>) -> &mut Self {
        self.send(Command::SetPosition {
            id: self.id,
            position: point.into(),
        });
        self
    }

    pub fn visible(&mut self, visible: bool) -> &mut Self {
        self.send(Command::SetVisible {
            id: self.id,
            visible,
        });
        self
    }

    pub fn show(&mut self) -> &mut Self {
        self.visible(true)
    }

    pub fn hide(&mut self) -> &mut Self {
        self.visible(false)
    }

    pub fn resizable(&mut self, resizable: bool) -> &mut Self {
        self.send(Command::SetResizable {
            id: self.id,
            resizable,
        });
        self
    }

    pub fn controls(&mut self, controls: impl Into<Controls>) -> &mut Self {
        self.send(Command::SetControls {
            id: self.id,
            controls: controls.into(),
        });
        self
    }

    pub fn decorations(&mut self, enabled: bool) -> &mut Self {
        self.send(Command::SetDecorations {
            id: self.id,
            decorations: enabled,
        });
        self
    }

    pub fn transparent(&mut self, transparent: bool) -> &mut Self {
        self.send(Command::SetTransparent {
            id: self.id,
            transparent,
        });
        self
    }

    pub fn size(&mut self, size: impl Into<Size>) -> &mut Self {
        self.send(Command::SetInnerSize {
            id: self.id,
            size: size.into(),
        });
        self
    }

    pub fn min(&mut self, size: impl Into<Option<Size>>) -> &mut Self {
        self.send(Command::SetMinInnerSize {
            id: self.id,
            size: size.into(),
        });
        self
    }

    pub fn max(&mut self, size: impl Into<Option<Size>>) -> &mut Self {
        self.send(Command::SetMaxInnerSize {
            id: self.id,
            size: size.into(),
        });
        self
    }

    pub fn fullscreen(&mut self, fullscreen: Fullscreen) -> &mut Self {
        self.send(Command::SetFullscreen {
            id: self.id,
            fullscreen,
        });
        self
    }

    pub fn level(&mut self, level: Level) -> &mut Self {
        self.send(Command::SetLevel { id: self.id, level });
        self
    }

    pub fn theme(&mut self, theme: impl Into<Option<Theme>>) -> &mut Self {
        self.send(Command::SetTheme {
            id: self.id,
            theme: theme.into(),
        });
        self
    }

    pub fn cursor(&mut self, cursor: Cursor) -> &mut Self {
        self.send(Command::SetCursor {
            id: self.id,
            cursor,
        });
        self
    }

    pub fn cursor_grab(&mut self, grab: CursorGrab) -> &mut Self {
        self.send(Command::SetCursorGrab { id: self.id, grab });
        self
    }

    pub fn ime(&mut self, request: ImeRequest) -> &mut Self {
        self.send(Command::SetIme {
            id: self.id,
            request,
        });
        self
    }

    pub fn attention(&mut self) -> &mut Self {
        self.send(Command::RequestUserAttention { id: self.id });
        self
    }

    pub fn draw(&mut self) -> &mut Self {
        self.request(Action::DrawNext(self.id));
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.request(Action::CloseRequested(self.id));
        self
    }
}

pub struct Frame<'a> {
    id: Id,
    context: Context<'a>,
}

pub struct Ready<'a> {
    id: Id,
    context: Context<'a>,
}

pub struct Resize<'a> {
    id: Id,
    context: Context<'a>,
}

pub struct Input<'a> {
    event: InputEvent,
    context: Context<'a>,
}

pub struct Close<'a> {
    id: Id,
    context: Context<'a>,
    accepted: bool,
}

pub struct Closed<'a> {
    state: WindowSnapshot,
    context: Context<'a>,
}

pub trait Scope<'a> {
    fn id(&self) -> Id;
    fn state(&self) -> &WindowSnapshot;

    #[must_use]
    fn metrics(&self) -> &Metrics {
        self.state().metrics()
    }

    #[must_use]
    fn size(&self) -> Size {
        self.metrics().logical_size
    }

    #[must_use]
    fn scale(&self) -> f64 {
        self.metrics().scale_factor
    }

    #[must_use]
    fn is_focused(&self) -> bool {
        self.state().is_focused()
    }

    #[must_use]
    fn is_visible(&self) -> bool {
        self.state().is_visible()
    }

    #[must_use]
    fn is_occluded(&self) -> bool {
        self.state().is_occluded()
    }

    #[must_use]
    fn is_resizing(&self) -> bool {
        false
    }

    fn access(&self) -> Result<Ref<'_>>;
    fn handle(&self) -> Result<Handle>;
    fn context_mut(&mut self) -> &mut Context<'a>;
    fn target(&mut self) -> Target<'_>;
    fn draw(&mut self) -> &mut Self;
    fn again(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        let id = self.id();
        self.context_mut().again(id);
        self
    }
    fn at(&mut self, time: Instant) -> &mut Self
    where
        Self: Sized,
    {
        let id = self.id();
        self.context_mut().at(id, time);
        self
    }
    fn close(&mut self) -> &mut Self;
    fn exit(&mut self) -> &mut Self;
}

impl<'a> Ready<'a> {
    pub(crate) fn new(id: Id, context: Context<'a>) -> Self {
        Self { id, context }
    }

    #[must_use]
    pub fn id(&self) -> Id {
        self.id
    }

    #[must_use]
    pub fn state(&self) -> &WindowSnapshot {
        self.context
            .state(self.id)
            .expect("ready scope always targets a live window")
    }

    pub fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    pub fn window(&mut self) -> Target<'_> {
        self.context.window(self.id)
    }

    pub fn access(&self) -> Result<Ref<'_>> {
        self.context.access(self.id)
    }

    pub fn handle(&self) -> Result<Handle> {
        self.context.handle(self.id)
    }

    pub fn metrics(&self) -> &Metrics {
        self.state().metrics()
    }

    pub fn draw(&mut self) -> &mut Self {
        self.context.draw(self.id);
        self
    }

    pub fn again(&mut self) -> &mut Self {
        self.context.again(self.id);
        self
    }

    pub fn at(&mut self, time: Instant) -> &mut Self {
        self.context.at(self.id, time);
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.context.close(self.id);
        self
    }

    pub fn exit(&mut self) -> &mut Self {
        self.context.request(Action::Exit);
        self
    }
}

impl<'a> Resize<'a> {
    pub(crate) fn new(id: Id, context: Context<'a>) -> Self {
        Self { id, context }
    }

    #[must_use]
    pub fn id(&self) -> Id {
        self.id
    }

    #[must_use]
    pub fn state(&self) -> &WindowSnapshot {
        self.context
            .state(self.id)
            .expect("resize scope always targets a live window")
    }

    #[must_use]
    pub fn metrics(&self) -> &Metrics {
        self.state().metrics()
    }

    #[must_use]
    pub fn size(&self) -> Size {
        self.metrics().logical_size
    }

    #[must_use]
    pub fn scale(&self) -> f64 {
        self.metrics().scale_factor
    }

    pub fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    pub fn access(&self) -> Result<Ref<'_>> {
        self.context.access(self.id)
    }

    pub fn handle(&self) -> Result<Handle> {
        self.context.handle(self.id)
    }

    pub fn window(&mut self) -> Target<'_> {
        self.context.window(self.id)
    }

    pub fn draw(&mut self) -> &mut Self {
        self.context.draw(self.id);
        self
    }

    pub fn again(&mut self) -> &mut Self {
        self.context.again(self.id);
        self
    }

    pub fn at(&mut self, time: Instant) -> &mut Self {
        self.context.at(self.id, time);
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.context.close(self.id);
        self
    }

    pub fn exit(&mut self) -> &mut Self {
        self.context.request(Action::Exit);
        self
    }
}

impl<'a> Input<'a> {
    pub(crate) fn new(event: InputEvent, context: Context<'a>) -> Self {
        Self { event, context }
    }

    #[must_use]
    pub fn id(&self) -> Id {
        self.event.id()
    }

    #[must_use]
    pub fn event(&self) -> &InputEvent {
        &self.event
    }

    #[must_use]
    pub fn state(&self) -> &WindowSnapshot {
        self.context
            .state(self.id())
            .expect("input scope always targets a live window")
    }

    #[must_use]
    pub fn metrics(&self) -> &Metrics {
        self.state().metrics()
    }

    #[must_use]
    pub fn size(&self) -> Size {
        self.metrics().logical_size
    }

    #[must_use]
    pub fn scale(&self) -> f64 {
        self.metrics().scale_factor
    }

    #[must_use]
    pub fn key_pressed(&self, code: keyboard_types::Code) -> bool {
        matches!(
            &self.event,
            InputEvent::Key(event)
                if event.physical_key == code && event.state == super::KeyState::Pressed
        )
    }

    #[must_use]
    pub fn pointer_pressed(&self, button: super::PointerButton) -> bool {
        matches!(
            &self.event,
            InputEvent::Pointer(event)
                if event.button == Some(button) && event.phase == super::PointerPhase::Pressed
        )
    }

    #[must_use]
    pub fn position(&self) -> Option<Point> {
        match &self.event {
            InputEvent::Pointer(event) => event.position,
            InputEvent::Wheel(event) => event.position,
            _ => None,
        }
    }

    #[must_use]
    pub fn modifiers(&self) -> super::ModifierState {
        match &self.event {
            InputEvent::Pointer(event) => event.modifiers,
            InputEvent::Wheel(event) => event.modifiers,
            InputEvent::Key(event) => event.modifiers,
            InputEvent::Modifiers { modifiers, .. } => *modifiers,
            _ => super::ModifierState::default(),
        }
    }

    pub fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    pub fn window(&mut self) -> Target<'_> {
        self.context.window(self.id())
    }

    pub fn draw(&mut self) -> &mut Self {
        self.context.draw(self.id());
        self
    }

    pub fn again(&mut self) -> &mut Self {
        self.context.again(self.id());
        self
    }

    pub fn at(&mut self, time: Instant) -> &mut Self {
        self.context.at(self.id(), time);
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.context.request(Action::CloseRequested(self.id()));
        self
    }

    pub fn exit(&mut self) -> &mut Self {
        self.context.request(Action::Exit);
        self
    }
}

impl<'a> Close<'a> {
    pub(crate) fn new(id: Id, context: Context<'a>) -> Self {
        Self {
            id,
            context,
            accepted: false,
        }
    }

    #[must_use]
    pub fn id(&self) -> Id {
        self.id
    }

    #[must_use]
    pub fn state(&self) -> &WindowSnapshot {
        self.context
            .state(self.id)
            .expect("close scope always targets a live window")
    }

    #[must_use]
    pub fn metrics(&self) -> &Metrics {
        self.state().metrics()
    }

    pub fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    pub fn draw(&mut self) -> &mut Self {
        self.context.draw(self.id);
        self
    }

    pub fn again(&mut self) -> &mut Self {
        self.context.again(self.id);
        self
    }

    pub fn at(&mut self, time: Instant) -> &mut Self {
        self.context.at(self.id, time);
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.accepted = true;
        self.context.send(Command::Destroy { id: self.id });
        self
    }

    pub fn cancel(&mut self) -> &mut Self {
        self.accepted = false;
        self
    }

    #[must_use]
    pub fn is_accepted(&self) -> bool {
        self.accepted
    }
}

impl<'a> Closed<'a> {
    pub(crate) fn new(state: WindowSnapshot, context: Context<'a>) -> Self {
        Self { state, context }
    }

    #[must_use]
    pub fn id(&self) -> Id {
        self.state.id()
    }

    #[must_use]
    pub fn state(&self) -> &WindowSnapshot {
        &self.state
    }

    #[must_use]
    pub fn metrics(&self) -> &Metrics {
        self.state.metrics()
    }

    pub fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    pub fn exit(&mut self) -> &mut Self {
        self.context.request(Action::Exit);
        self
    }
}

impl<'a> Frame<'a> {
    pub(crate) fn new(id: Id, context: Context<'a>) -> Self {
        Self { id, context }
    }

    #[must_use]
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    pub fn window(&mut self) -> Target<'_> {
        self.context.window(self.id)
    }

    pub fn access(&self) -> Result<Ref<'_>> {
        self.context.access(self.id)
    }

    pub fn handle(&self) -> Result<Handle> {
        self.context.handle(self.id)
    }

    #[must_use]
    pub fn state(&self) -> &WindowSnapshot {
        self.context
            .state(self.id)
            .expect("frame scope always targets a live window")
    }

    pub fn metrics(&self) -> &Metrics {
        self.state().metrics()
    }

    #[must_use]
    pub fn size(&self) -> Size {
        self.context
            .state(self.id)
            .expect("frame scope always targets a live window")
            .metrics()
            .logical_size
    }

    #[must_use]
    pub fn scale(&self) -> f64 {
        self.context
            .state(self.id)
            .expect("frame scope always targets a live window")
            .metrics()
            .scale_factor
    }

    pub fn draw(&mut self) -> &mut Self {
        self.context.draw(self.id);
        self
    }

    pub fn again(&mut self) -> &mut Self {
        self.context.again(self.id);
        self
    }

    pub fn at(&mut self, time: Instant) -> &mut Self {
        self.context.at(self.id, time);
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.context.close(self.id);
        self
    }

    pub fn exit(&mut self) -> &mut Self {
        self.context.request(Action::Exit);
        self
    }

    #[must_use]
    pub(crate) fn action(&self) -> &Action {
        self.context.action()
    }
}

impl<'a> Scope<'a> for Ready<'a> {
    fn id(&self) -> Id {
        Ready::id(self)
    }

    fn state(&self) -> &WindowSnapshot {
        Ready::state(self)
    }

    fn access(&self) -> Result<Ref<'_>> {
        Ready::access(self)
    }

    fn handle(&self) -> Result<Handle> {
        Ready::handle(self)
    }

    fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    fn target(&mut self) -> Target<'_> {
        self.window()
    }

    fn draw(&mut self) -> &mut Self {
        Ready::draw(self)
    }

    fn close(&mut self) -> &mut Self {
        Ready::close(self)
    }

    fn exit(&mut self) -> &mut Self {
        Ready::exit(self)
    }
}

impl<'a> Scope<'a> for Resize<'a> {
    fn id(&self) -> Id {
        Resize::id(self)
    }

    fn state(&self) -> &WindowSnapshot {
        Resize::state(self)
    }

    fn access(&self) -> Result<Ref<'_>> {
        Resize::access(self)
    }

    fn handle(&self) -> Result<Handle> {
        Resize::handle(self)
    }

    fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    fn target(&mut self) -> Target<'_> {
        self.window()
    }

    fn draw(&mut self) -> &mut Self {
        Resize::draw(self)
    }

    fn close(&mut self) -> &mut Self {
        Resize::close(self)
    }

    fn exit(&mut self) -> &mut Self {
        Resize::exit(self)
    }
}

impl<'a> Scope<'a> for Input<'a> {
    fn id(&self) -> Id {
        Input::id(self)
    }

    fn state(&self) -> &WindowSnapshot {
        Input::state(self)
    }

    fn access(&self) -> Result<Ref<'_>> {
        self.context.access(self.id())
    }

    fn handle(&self) -> Result<Handle> {
        self.context.handle(self.id())
    }

    fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    fn target(&mut self) -> Target<'_> {
        self.window()
    }

    fn draw(&mut self) -> &mut Self {
        Input::draw(self)
    }

    fn close(&mut self) -> &mut Self {
        Input::close(self)
    }

    fn exit(&mut self) -> &mut Self {
        Input::exit(self)
    }
}

impl<'a> Scope<'a> for Close<'a> {
    fn id(&self) -> Id {
        Close::id(self)
    }

    fn state(&self) -> &WindowSnapshot {
        Close::state(self)
    }

    fn access(&self) -> Result<Ref<'_>> {
        self.context.access(self.id)
    }

    fn handle(&self) -> Result<Handle> {
        self.context.handle(self.id)
    }

    fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    fn target(&mut self) -> Target<'_> {
        self.context.window(self.id)
    }

    fn draw(&mut self) -> &mut Self {
        self.context.draw(self.id);
        self
    }

    fn close(&mut self) -> &mut Self {
        Close::close(self)
    }

    fn exit(&mut self) -> &mut Self {
        self.context.request(Action::Exit);
        self
    }
}

impl<'a> Scope<'a> for Frame<'a> {
    fn id(&self) -> Id {
        Frame::id(self)
    }

    fn state(&self) -> &WindowSnapshot {
        Frame::state(self)
    }

    fn access(&self) -> Result<Ref<'_>> {
        Frame::access(self)
    }

    fn handle(&self) -> Result<Handle> {
        Frame::handle(self)
    }

    fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    fn target(&mut self) -> Target<'_> {
        self.window()
    }

    fn draw(&mut self) -> &mut Self {
        Frame::draw(self)
    }

    fn close(&mut self) -> &mut Self {
        Frame::close(self)
    }

    fn exit(&mut self) -> &mut Self {
        Frame::exit(self)
    }
}

pub struct App<H> {
    window_loop: super::Loop<H>,
    startup: Vec<Open>,
}

#[must_use]
pub fn app<H>(handler: H) -> App<H> {
    App::new(handler)
}

impl<H> App<H> {
    #[must_use]
    pub fn new(handler: H) -> Self {
        Self {
            window_loop: super::Loop::new(handler),
            startup: Vec::new(),
        }
    }

    #[must_use]
    pub fn open(mut self, open: Open) -> Self {
        self.startup.push(open);
        self
    }

    #[must_use]
    pub fn with_clipboard(mut self, clipboard: Box<dyn super::Clipboard>) -> Self {
        self.window_loop = self.window_loop.with_clipboard(clipboard);
        self
    }

    #[must_use]
    pub fn handler(&self) -> &H {
        self.window_loop.handler()
    }

    pub fn handler_mut(&mut self) -> &mut H {
        self.window_loop.handler_mut()
    }

    pub(crate) fn validate_startup(&self) -> Result<()> {
        let mut names = HashSet::new();
        for open in &self.startup {
            let request = open.request();
            if !matches!(request.role(), Role::Root) {
                return Err(Error::new(
                    ErrorCode::UnsupportedFeature,
                    "startup windows must be root windows",
                ));
            }
            if let Some(name) = request.name()
                && !name.is_empty()
                && !names.insert(name)
            {
                return Err(Error::new(
                    ErrorCode::CommandFailed,
                    format!("duplicate startup window name '{name}'"),
                ));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn into_loop(mut self) -> super::Loop<H> {
        for open in self.startup {
            self.window_loop.startup.push(open.into_command());
        }
        self.window_loop
    }
}

impl<H: super::Handler + 'static> App<H> {
    pub fn run(self) -> Result<()> {
        self.validate_startup()?;
        self.into_loop().run()
    }
}

impl Proxy {
    pub fn send(&self, command: impl Into<Command>) -> Result<()> {
        self.command(command.into())
    }

    pub fn open(&self, open: Open) -> Result<()> {
        self.send(open)
    }

    pub fn close(&self, id: Id) -> Result<()> {
        self.request(Action::CloseRequested(id))
    }

    pub fn draw(&self, id: Id) -> Result<()> {
        self.request(Action::DrawNext(id))
    }

    pub fn again(&self, id: Id) -> Result<()> {
        self.request(Action::DrawNow(id))
    }

    pub fn at(&self, id: Id, time: Instant) -> Result<()> {
        self.request(Action::DrawAt { id, time })
    }

    pub(crate) fn request(&self, action: impl Into<Action>) -> Result<()> {
        self.request_action(action.into())
    }
}
