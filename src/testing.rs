use super::{
    command::Action, context::resolve_actions_with, descriptor::WindowSnapshotSeed,
    winit_adapter::validate_name, *,
};
use std::{collections::HashMap, time::Instant};

// Test utilities for exercising the window contract without opening native windows.

/// Lifecycle event recorded by the fake host.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Created(WindowSnapshot),
    Destroyed(Id),
    Suspended(Id),
    Resumed(Id),
    CloseRequested(Id),
    Focused { id: Id, focused: bool },
    Resized(Metrics),
    ScaleFactorChanged(Metrics),
    Moved { id: Id, position: Point },
    Occluded { id: Id, occluded: bool },
    ThemeChanged { id: Id, theme: Option<Theme> },
    FileDrag(FileDragEvent),
    Input(InputEvent),
    Accessibility(AccessibilityEvent),
}

impl Event {
    #[must_use]
    pub fn id(&self) -> Id {
        match self {
            Self::Created(state) => state.id(),
            Self::Destroyed(id)
            | Self::Suspended(id)
            | Self::Resumed(id)
            | Self::CloseRequested(id) => *id,
            Self::Focused { id, .. }
            | Self::Moved { id, .. }
            | Self::Occluded { id, .. }
            | Self::ThemeChanged { id, .. } => *id,
            Self::Resized(metrics) | Self::ScaleFactorChanged(metrics) => metrics.id,
            Self::FileDrag(event) => event.id(),
            Self::Input(event) => event.id(),
            Self::Accessibility(event) => event.id(),
        }
    }
}

impl From<EventKind> for Event {
    fn from(event: EventKind) -> Self {
        match event {
            EventKind::Created(state) => Self::Created(state),
            EventKind::Destroyed(id) => Self::Destroyed(id),
            EventKind::Suspended(id) => Self::Suspended(id),
            EventKind::Resumed(id) => Self::Resumed(id),
            EventKind::CloseRequested(id) => Self::CloseRequested(id),
            EventKind::Focused { id, focused } => Self::Focused { id, focused },
            EventKind::Resized(metrics) => Self::Resized(metrics),
            EventKind::ScaleFactorChanged(metrics) => Self::ScaleFactorChanged(metrics),
            EventKind::Moved { id, position } => Self::Moved { id, position },
            EventKind::Occluded { id, occluded } => Self::Occluded { id, occluded },
            EventKind::ThemeChanged { id, theme } => Self::ThemeChanged { id, theme },
            EventKind::FileDrag(event) => Self::FileDrag(event),
            EventKind::Input(event) => Self::Input(event),
            EventKind::Accessibility(event) => Self::Accessibility(event),
        }
    }
}

/// Lifecycle effect produced by a handler dispatch in the fake host.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Wait,
    Draw(Id),
    Again(Id),
    At { id: Id, time: Instant },
    CloseRequested(Id),
    Exit,
    Batch(Vec<Effect>),
}

impl From<Action> for Effect {
    fn from(action: Action) -> Self {
        match action {
            Action::Wait => Self::Wait,
            Action::DrawNow(id) => Self::Again(id),
            Action::DrawNext(id) => Self::Draw(id),
            Action::DrawAt { id, time } => Self::At { id, time },
            Action::CloseRequested(id) => Self::CloseRequested(id),
            Action::Exit => Self::Exit,
            Action::Batch(actions) => Self::Batch(actions.into_iter().map(Into::into).collect()),
        }
    }
}

/// Fake native host for command and event contract tests.
#[derive(Debug)]
pub struct Host {
    registry: Registry,
    draw: DrawScheduler,
    capabilities: HostCapabilities,
    events: Vec<Event>,
    commands: Vec<Command>,
    closed: HashMap<Id, WindowSnapshot>,
    cursors: HashMap<Id, Cursor>,
    cursor_updates: Vec<(Id, Cursor)>,
    ime_requests: Vec<(Id, ImeRequest)>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
            draw: DrawScheduler::default(),
            capabilities: HostCapabilities::winit_default(),
            events: Vec::new(),
            commands: Vec::new(),
            closed: HashMap::new(),
            cursors: HashMap::new(),
            cursor_updates: Vec::new(),
            ime_requests: Vec::new(),
        }
    }
}

impl Host {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    #[must_use]
    pub fn capabilities(&self) -> &HostCapabilities {
        &self.capabilities
    }

    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    #[must_use]
    pub fn cursor_updates(&self) -> &[(Id, Cursor)] {
        &self.cursor_updates
    }

    #[must_use]
    pub fn ime_requests(&self) -> &[(Id, ImeRequest)] {
        &self.ime_requests
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.commands.clear();
        self.cursor_updates.clear();
        self.ime_requests.clear();
    }

    pub fn apply(&mut self, command: impl Into<Command>) -> Result<()> {
        let command = command.into();
        self.commands.push(command.clone());
        let plan = HostCommandPlan::from_command(command, &self.capabilities)?;
        self.apply_plan(plan)
    }

    #[must_use]
    pub fn take_ready_draws(&mut self, now: Instant) -> Vec<Id> {
        self.draw.take_ready(now)
    }

    pub fn resume(&mut self, id: Id) -> Result<()> {
        self.require_window(id)?;
        self.events.push(Event::Resumed(id));
        Ok(())
    }

    pub fn suspend(&mut self, id: Id) -> Result<()> {
        self.require_window(id)?;
        self.events.push(Event::Suspended(id));
        Ok(())
    }

    pub fn accessibility(&mut self, event: AccessibilityEvent) -> Result<()> {
        self.require_window(event.id())?;
        self.events.push(Event::Accessibility(event));
        Ok(())
    }

    #[must_use]
    pub fn window_id(&self, name: impl AsRef<str>) -> Option<Id> {
        self.registry.window_id(name)
    }

    pub fn dispatch_ready<H: Handler>(&mut self, handler: &mut H, id: Id) -> Result<Effect> {
        self.require_window(id)?;

        let mut commands = Vec::new();
        let mut actions = Vec::new();
        {
            let context = Context::new(&mut self.registry, &mut commands, &mut actions, None);
            let mut ready = Ready::new(id, context);
            handler.ready(&mut ready)?;
        }

        self.draw.request(&Action::DrawNext(id));
        for command in commands {
            self.apply(command)?;
        }

        Ok(resolve_actions_with(&actions, Action::DrawNext(id)).into())
    }

    pub fn dispatch_draw<H: Handler>(&mut self, handler: &mut H, id: Id) -> Result<Effect> {
        self.require_window(id)?;

        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(&mut self.registry, &mut commands, &mut actions, None);
            let mut frame = Frame::new(id, context);
            handler.draw(&mut frame)?;
            frame.action().clone()
        };

        for command in commands {
            self.apply(command)?;
        }

        Ok(action.into())
    }

    pub fn dispatch_resize<H: Handler>(
        &mut self,
        handler: &mut H,
        metrics: Metrics,
    ) -> Result<Effect> {
        let id = metrics.id;
        self.state_mut(id)?.set_metrics(metrics);

        let mut commands = Vec::new();
        let mut actions = Vec::new();
        {
            let context = Context::new(&mut self.registry, &mut commands, &mut actions, None);
            let mut resize = Resize::new(id, context);
            handler.resize(&mut resize)?;
        }

        for command in commands {
            self.apply(command)?;
        }

        Ok(resolve_actions_with(&actions, Action::DrawNext(id)).into())
    }

    pub fn dispatch_input<H: Handler>(
        &mut self,
        handler: &mut H,
        input: InputEvent,
    ) -> Result<Effect> {
        let id = input.id();
        self.require_window(id)?;

        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(&mut self.registry, &mut commands, &mut actions, None);
            let mut input = Input::new(input, context);
            handler.input(&mut input)?;
            input.context_mut().resolved_action()
        };

        for command in commands {
            self.apply(command)?;
        }

        Ok(action.into())
    }

    pub fn dispatch_close<H: Handler>(&mut self, handler: &mut H, id: Id) -> Result<Effect> {
        self.require_window(id)?;

        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(&mut self.registry, &mut commands, &mut actions, None);
            let mut close = Close::new(id, context);
            handler.close(&mut close)?;
            close.context_mut().resolved_action()
        };

        for command in commands {
            self.apply(command)?;
        }

        Ok(action.into())
    }

    pub fn dispatch_closed<H: Handler>(&mut self, handler: &mut H, id: Id) -> Result<Effect> {
        let state = self.closed.get(&id).cloned().ok_or_else(|| {
            Error::new(ErrorCode::CommandFailed, "unknown closed window").with_id(id)
        })?;

        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(&mut self.registry, &mut commands, &mut actions, None);
            let mut closed = Closed::new(state, context);
            handler.closed(&mut closed)?;
            closed.context_mut().resolved_action()
        };

        for command in commands {
            self.apply(command)?;
        }

        Ok(action.into())
    }

    pub fn idle<H: Handler>(&mut self, handler: &mut H) -> Result<Option<Effect>> {
        if !handler.wants_idle() {
            return Ok(None);
        }

        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let mut context = Context::new(&mut self.registry, &mut commands, &mut actions, None);
            handler.idle(&mut context)?;
            context.action().clone()
        };

        for command in commands {
            self.apply(command)?;
        }

        Ok(Some(action.into()))
    }

    fn require_window(&self, id: Id) -> Result<()> {
        self.registry
            .contains(id)
            .then_some(())
            .ok_or_else(|| Error::new(ErrorCode::CommandFailed, "unknown window").with_id(id))
    }

    fn state_mut(&mut self, id: Id) -> Result<&mut WindowSnapshot> {
        self.registry
            .get_mut(id)
            .map(Instance::state_mut)
            .ok_or_else(|| Error::new(ErrorCode::CommandFailed, "unknown window").with_id(id))
    }

    fn apply_patch(&mut self, patch: WindowStatePatch) -> Result<()> {
        let id = patch.id();
        let event = {
            let state = self.state_mut(id)?;
            patch.apply(state)?
        };
        if let Some(event) = event {
            self.events.push(event.into());
        }
        Ok(())
    }

    fn apply_plan(&mut self, plan: HostCommandPlan) -> Result<()> {
        match plan.into_command() {
            HostCommand::Open { request } => self.apply_open_request(request),
            HostCommand::SetTitle { id, title } => {
                self.apply_patch(WindowStatePatch::title(id, title))
            }
            HostCommand::SetPosition { id, position } => {
                self.apply_patch(WindowStatePatch::Position { id, position })
            }
            HostCommand::SetVisible { id, visible } => {
                self.apply_patch(WindowStatePatch::visible(id, visible))
            }
            HostCommand::SetInnerSize { id, size } => self.apply_inner_size(id, size),
            HostCommand::SetFullscreen { id, fullscreen } => {
                self.apply_patch(WindowStatePatch::Fullscreen {
                    id,
                    fullscreen: !matches!(fullscreen, Fullscreen::None),
                })
            }
            HostCommand::SetTheme { id, theme } => {
                self.apply_patch(WindowStatePatch::Theme { id, theme })
            }
            HostCommand::SetCursor { id, cursor } => self.apply_cursor(id, cursor),
            HostCommand::SetIme { id, request } => self.apply_ime_request(id, request),
            HostCommand::RequestDraw { id } => self.apply_draw_request(id),
            HostCommand::Destroy { id } => self.apply_destroy(id),
            HostCommand::SetResizable { id, .. }
            | HostCommand::SetControls { id, .. }
            | HostCommand::SetDecorations { id, .. }
            | HostCommand::SetTransparent { id, .. }
            | HostCommand::SetMinInnerSize { id, .. }
            | HostCommand::SetMaxInnerSize { id, .. }
            | HostCommand::SetLevel { id, .. }
            | HostCommand::SetCursorGrab { id, .. }
            | HostCommand::RequestUserAttention { id } => self.require_window(id),
        }
    }

    fn apply_open_request(&mut self, request: WindowRequest) -> Result<()> {
        validate_name(&self.registry, request.name())?;
        let id = self.registry.reserve_id();
        let state = fake_state_from_request(id, &request);
        self.registry.insert(Instance::new(id, state.clone()));
        self.events.push(Event::Created(state));
        Ok(())
    }

    fn apply_inner_size(&mut self, id: Id, size: Size) -> Result<()> {
        let metrics = {
            let state = self.state_mut(id)?;
            let scale = state.metrics().scale_factor;
            Metrics::from_physical_size(
                id,
                PhysicalSize {
                    width: (size.width * scale).round().max(0.0) as u32,
                    height: (size.height * scale).round().max(0.0) as u32,
                },
                scale,
            )
            .with_outer_geometry(state.metrics().outer_position, state.metrics().outer_size)
        };
        self.apply_patch(WindowStatePatch::metrics(metrics, MetricsEvent::Resized))
    }

    fn apply_cursor(&mut self, id: Id, cursor: Cursor) -> Result<()> {
        self.require_window(id)?;
        if self.cursors.get(&id) != Some(&cursor) {
            self.cursors.insert(id, cursor.clone());
            self.cursor_updates.push((id, cursor));
        }
        Ok(())
    }

    fn apply_ime_request(&mut self, id: Id, request: ImeRequest) -> Result<()> {
        self.require_window(id)?;
        self.ime_requests.push((id, request));
        Ok(())
    }

    fn apply_draw_request(&mut self, id: Id) -> Result<()> {
        self.require_window(id)?;
        self.draw.request(&Action::DrawNext(id));
        Ok(())
    }

    fn apply_destroy(&mut self, id: Id) -> Result<()> {
        self.require_window(id)?;
        if let Some(instance) = self.registry.remove(id) {
            self.closed.insert(id, instance.state().clone());
        }
        self.cursors.remove(&id);
        self.events.push(Event::Destroyed(id));
        Ok(())
    }
}

fn fake_state_from_request(id: Id, request: &WindowRequest) -> WindowSnapshot {
    let logical_size = request.inner_size().unwrap_or(Size {
        width: 800.0,
        height: 600.0,
    });
    let metrics = Metrics {
        id,
        logical_size,
        physical_size: PhysicalSize {
            width: logical_size.width.round().max(0.0) as u32,
            height: logical_size.height.round().max(0.0) as u32,
        },
        outer_position: request.position(),
        outer_size: None,
        scale_factor: 1.0,
        safe_area: Insets::default(),
    };
    WindowSnapshot::from_seed(WindowSnapshotSeed {
        id,
        title: request.title().to_owned(),
        name: request.name().map(str::to_owned),
        metrics,
        position: request.position(),
        focused: false,
        visible: Some(request.visible()),
        minimized: Some(false),
        maximized: false,
        occluded: Some(false),
        fullscreen: !matches!(request.fullscreen(), Fullscreen::None),
        theme: request.theme(),
        role: request.role().clone(),
    })
}
