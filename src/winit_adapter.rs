use super::{
    command::Action, context::resolve_actions_with, descriptor::WindowSnapshotSeed,
    event::EventKind, winit_mapping, *,
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

const REDRAW_RETRY: Duration = Duration::from_millis(16);

pub(crate) struct WinitRunner<H> {
    handler: H,
    registry: Registry,
    draw: DrawScheduler,
    capabilities: HostCapabilities,
    _clipboard: Box<dyn Clipboard>,
    pub(crate) commands: Vec<Command>,
    pub(crate) startup: Vec<Command>,
    windows: HashMap<winit::window::WindowId, Id>,
    hovered_files: HashMap<Id, Vec<String>>,
    modifiers: ModifierState,
    pub(crate) pointer_positions: HashMap<PointerPositionKey, Point>,
    cursor_state: HashMap<Id, Cursor>,
    pending_draws: HashSet<Id>,
    #[cfg(feature = "accessibility")]
    accessibility: HashMap<Id, accesskit_winit::Adapter>,
    pub(crate) proxy: Option<Proxy>,
    startup_applied: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeTransitionRoute {
    Event,
    Input,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PointerPositionKey {
    window: Id,
    pointer: Option<u64>,
}

impl PointerPositionKey {
    pub(crate) const fn mouse(window: Id) -> Self {
        Self {
            window,
            pointer: None,
        }
    }

    pub(crate) const fn touch(window: Id, pointer: u64) -> Self {
        Self {
            window,
            pointer: Some(pointer),
        }
    }
}

impl<H> WinitRunner<H> {
    pub(crate) fn from_loop(window_loop: Loop<H>) -> Self {
        Self {
            handler: window_loop.handler,
            registry: window_loop.registry,
            draw: window_loop.draw,
            capabilities: HostCapabilities::winit_default(),
            _clipboard: window_loop.clipboard,
            commands: window_loop.commands,
            startup: window_loop.startup,
            windows: HashMap::new(),
            hovered_files: HashMap::new(),
            modifiers: ModifierState::default(),
            pointer_positions: HashMap::new(),
            cursor_state: HashMap::new(),
            pending_draws: HashSet::new(),
            #[cfg(feature = "accessibility")]
            accessibility: HashMap::new(),
            proxy: None,
            startup_applied: false,
        }
    }

    #[must_use]
    pub(crate) fn capabilities(&self) -> &HostCapabilities {
        &self.capabilities
    }

    #[cfg(test)]
    pub(crate) fn plan_command_for_test(&self, command: Command) -> Result<HostCommandPlan> {
        HostCommandPlan::from_command(command, &self.capabilities)
    }
}

impl<H: Handler> WinitRunner<H> {
    pub(crate) fn stage_startup(&mut self) {
        if self.startup_applied {
            return;
        }
        self.startup_applied = true;
        self.commands.append(&mut self.startup);
    }

    fn apply_action(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, action: Action) {
        match action {
            Action::Exit => event_loop.exit(),
            Action::DrawNow(id) => {
                self.draw.next.remove(&id);
                self.draw.delayed.remove(&id);
                if let Ok(handle) = self.handle(id) {
                    self.pending_draws.insert(id);
                    handle.request_draw();
                }
            }
            Action::Batch(actions) => {
                for action in actions {
                    self.apply_action(event_loop, action);
                }
            }
            Action::CloseRequested(id) => {
                self.deliver_close(event_loop, id);
            }
            other => self.draw.request(&other),
        }
    }

    fn request_ready_draws(&mut self) {
        for id in self.draw.take_ready(Instant::now()) {
            self.pending_draws.insert(id);
        }
        self.request_pending_draws();
    }

    fn request_pending_draws(&mut self) {
        for id in self.pending_draws.clone() {
            if !self.can_request_draw(id) {
                continue;
            }
            if let Ok(handle) = self.handle(id) {
                handle.request_draw();
            }
        }
    }

    fn can_request_draw(&self, id: Id) -> bool {
        self.registry
            .get(id)
            .map(|instance| {
                instance.instance.state.is_visible() && !instance.instance.state.is_occluded()
            })
            .unwrap_or(false)
    }

    fn control_flow(&self) -> winit::event_loop::ControlFlow {
        if self
            .pending_draws
            .iter()
            .any(|id| self.can_request_draw(*id))
        {
            return winit::event_loop::ControlFlow::WaitUntil(Instant::now() + REDRAW_RETRY);
        }
        winit_mapping::control_flow_from_draw_scheduler(&self.draw)
    }

    fn call_with_context(
        &mut self,
        call: impl FnOnce(&mut H, &mut Context<'_>) -> Result<()>,
    ) -> Result<Action> {
        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let mut context = Context::new(
                &mut self.registry,
                &mut commands,
                &mut actions,
                self.proxy.clone(),
            );
            call(&mut self.handler, &mut context)?;
            context.resolved_action()
        };
        self.commands.extend(commands);
        Ok(action)
    }

    fn call_with_ready(&mut self, id: Id) -> Result<Action> {
        let mut commands = Vec::new();
        let mut actions = Vec::new();
        {
            let context = Context::new(
                &mut self.registry,
                &mut commands,
                &mut actions,
                self.proxy.clone(),
            );
            let mut ready = Ready::new(id, context);
            self.handler.ready(&mut ready)?;
        }
        self.commands.extend(commands);
        Ok(resolve_actions_with(&actions, Action::DrawNext(id)))
    }

    fn call_with_resize(&mut self, id: Id) -> Result<Action> {
        let mut commands = Vec::new();
        let mut actions = Vec::new();
        {
            let context = Context::new(
                &mut self.registry,
                &mut commands,
                &mut actions,
                self.proxy.clone(),
            );
            let mut resize = Resize::new(id, context);
            self.handler.resize(&mut resize)?;
        }
        self.commands.extend(commands);
        Ok(resolve_actions_with(&actions, Action::DrawNext(id)))
    }

    fn call_with_input(&mut self, input: InputEvent) -> Result<Action> {
        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(
                &mut self.registry,
                &mut commands,
                &mut actions,
                self.proxy.clone(),
            );
            let mut input = Input::new(input, context);
            self.handler.input(&mut input)?;
            input.context_mut().resolved_action()
        };
        self.commands.extend(commands);
        Ok(action)
    }

    fn call_with_event(&mut self, event: EventKind) -> Result<Action> {
        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(
                &mut self.registry,
                &mut commands,
                &mut actions,
                self.proxy.clone(),
            );
            let mut event = Event::new(event, context);
            self.handler.event(&mut event)?;
            event.context_mut().resolved_action()
        };
        self.commands.extend(commands);
        Ok(action)
    }

    #[cfg(test)]
    pub(crate) fn call_with_event_for_test(&mut self, event: EventKind) -> Result<Action> {
        self.call_with_event(event)
    }

    #[cfg(test)]
    pub(crate) fn deliver_created_then_ready_for_test(
        &mut self,
        state: WindowSnapshot,
    ) -> Result<()> {
        let id = state.id();
        let action = self.call_with_event(EventKind::Created(state))?;
        self.apply_action_for_test(action);
        if self.registry.contains(id) {
            let action = self.call_with_ready(id)?;
            self.apply_action_for_test(action);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn registry_contains_for_test(&self, id: Id) -> bool {
        self.registry.contains(id)
    }

    #[cfg(test)]
    fn apply_action_for_test(&mut self, action: Action) {
        match action {
            Action::CloseRequested(id) => {
                self.close(id);
            }
            Action::Batch(actions) => {
                for action in actions {
                    self.apply_action_for_test(action);
                }
            }
            other => self.draw.request(&other),
        }
    }

    fn call_with_close(&mut self, id: Id) -> Result<Action> {
        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(
                &mut self.registry,
                &mut commands,
                &mut actions,
                self.proxy.clone(),
            );
            let mut close = Close::new(id, context);
            self.handler.close(&mut close)?;
            close.context_mut().resolved_action()
        };
        self.commands.extend(commands);
        Ok(action)
    }

    fn call_with_closed(&mut self, state: WindowSnapshot) -> Result<Action> {
        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(
                &mut self.registry,
                &mut commands,
                &mut actions,
                self.proxy.clone(),
            );
            let mut closed = Closed::new(state, context);
            self.handler.closed(&mut closed)?;
            closed.context_mut().resolved_action()
        };
        self.commands.extend(commands);
        Ok(action)
    }

    fn call_with_frame(&mut self, id: Id) -> Result<Action> {
        let mut commands = Vec::new();
        let mut actions = Vec::new();
        let action = {
            let context = Context::new(
                &mut self.registry,
                &mut commands,
                &mut actions,
                self.proxy.clone(),
            );
            let mut frame = Frame::new(id, context);
            self.handler.draw(&mut frame)?;
            frame.action().clone()
        };
        self.commands.extend(commands);
        Ok(action)
    }

    fn finish_callback(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        action: Result<Action>,
    ) {
        match action {
            Ok(action) => {
                if let Err(error) = self.apply_commands(event_loop) {
                    eprintln!("{error}");
                    event_loop.exit();
                    return;
                }
                self.apply_action(event_loop, action);
                self.request_ready_draws();
            }
            Err(error) => {
                eprintln!("{error}");
                event_loop.exit();
            }
        }
    }

    fn deliver_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        id: Id,
        event: EventKind,
    ) {
        debug_assert_eq!(event.id(), id);
        let action = self.call_with_event(event);
        self.finish_callback(event_loop, action);
    }

    fn apply_native_transition(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        id: Id,
        transition: NativeEventTransition,
    ) {
        if let Some(patch) = transition.patch().cloned()
            && let Some(instance) = self.registry.get_mut(id)
        {
            let _ = patch.apply(instance.state_mut());
        }
        if let Some(event) = transition.into_event() {
            match native_transition_route(&event) {
                NativeTransitionRoute::Event => self.deliver_event(event_loop, id, event),
                NativeTransitionRoute::Input => {
                    let EventKind::Input(input) = event else {
                        unreachable!("native transition route must match event kind")
                    };
                    self.deliver_input(event_loop, input);
                }
            }
        }
    }

    fn deliver_ready(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, id: Id) {
        let action = self.call_with_ready(id);
        self.finish_callback(event_loop, action);
    }

    fn deliver_resize(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, id: Id) {
        let action = self.call_with_resize(id);
        self.finish_callback(event_loop, action);
    }

    fn deliver_input(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        input: InputEvent,
    ) {
        let action = self.call_with_input(input);
        self.finish_callback(event_loop, action);
    }

    fn deliver_close(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, id: Id) {
        let action = self.call_with_close(id);
        self.finish_callback(event_loop, action);
    }

    fn deliver_closed(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        state: WindowSnapshot,
    ) {
        let action = self.call_with_closed(state);
        match action {
            Ok(action) => {
                if let Err(error) = self.apply_commands(event_loop) {
                    eprintln!("{error}");
                    event_loop.exit();
                    return;
                }
                self.apply_action(event_loop, action);
                self.request_ready_draws();
            }
            Err(error) => {
                eprintln!("{error}");
                event_loop.exit();
            }
        }
        if self.registry.is_empty() {
            event_loop.exit();
        }
    }

    fn apply_commands(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<()> {
        let commands = std::mem::take(&mut self.commands);
        for command in commands {
            let plan = HostCommandPlan::from_command(command, self.capabilities())?;
            self.apply_host_command(event_loop, plan.into_command())?;
        }
        Ok(())
    }

    fn apply_host_command(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        command: Command,
    ) -> Result<()> {
        match command {
            Command::Open { request } => {
                validate_name(&self.registry, request.name())?;
                #[cfg(feature = "accessibility")]
                let requested_visible = request.visible();
                #[cfg(feature = "accessibility")]
                let native_request = {
                    let mut native_request = request.clone();
                    native_request.set_visible(false);
                    native_request
                };
                #[cfg(not(feature = "accessibility"))]
                let native_request = request.clone();
                let window = Arc::new(
                    event_loop
                        .create_window(winit_mapping::window_attributes_from_request(
                            &native_request,
                        )?)
                        .map_err(|source| {
                            Error::new(ErrorCode::WindowCreateFailed, "failed to create window")
                                .with_source(source)
                        })?,
                );
                let id = self.registry.reserve_id();
                let handle = Handle::from_winit(window.clone());
                let state = state_from_winit(id, &request, &window);
                self.windows.insert(window.id(), id);
                self.registry
                    .insert(Instance::with_handle(id, state.clone(), handle));
                #[cfg(feature = "accessibility")]
                {
                    let proxy = self.proxy.as_ref().ok_or_else(|| {
                        Error::new(
                            ErrorCode::AccessibilityAdapterFailed,
                            "event loop proxy unavailable for accessibility adapter",
                        )
                        .with_id(id)
                    })?;
                    let adapter = accesskit_winit::Adapter::with_event_loop_proxy(
                        event_loop,
                        &window,
                        proxy.inner.clone(),
                    );
                    self.accessibility.insert(id, adapter);
                    if requested_visible {
                        window.set_visible(true);
                    }
                }
                self.deliver_event(event_loop, id, EventKind::Created(state));
                if self.registry.contains(id) {
                    self.deliver_ready(event_loop, id);
                }
            }
            Command::SetTitle { id, title } => {
                let handle = self.handle(id)?;
                handle.winit().set_title(&title);
                self.apply_patch(WindowStatePatch::title(id, title))?;
            }
            Command::SetPosition { id, position } => {
                self.handle(id)?
                    .winit()
                    .set_outer_position(winit::dpi::LogicalPosition::new(position.x, position.y));
            }
            Command::SetVisible { id, visible } => {
                let handle = self.handle(id)?;
                handle.winit().set_visible(visible);
                self.apply_patch(WindowStatePatch::visible(id, visible))?;
                if visible {
                    self.request_pending_draws();
                }
            }
            Command::SetResizable { id, resizable } => {
                self.handle(id)?.winit().set_resizable(resizable);
            }
            Command::SetControls { id, controls } => {
                self.handle(id)?
                    .winit()
                    .set_enabled_buttons(controls.into());
            }
            Command::SetDecorations { id, decorations } => {
                self.handle(id)?.winit().set_decorations(decorations);
            }
            Command::SetTransparent { id, transparent } => {
                self.handle(id)?.winit().set_transparent(transparent);
            }
            Command::SetInnerSize { id, size } => {
                let _ = self
                    .handle(id)?
                    .winit()
                    .request_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
            }
            Command::SetMinInnerSize { id, size } => {
                self.handle(id)?.winit().set_min_inner_size(
                    size.map(|size| winit::dpi::LogicalSize::new(size.width, size.height)),
                );
            }
            Command::SetMaxInnerSize { id, size } => {
                self.handle(id)?.winit().set_max_inner_size(
                    size.map(|size| winit::dpi::LogicalSize::new(size.width, size.height)),
                );
            }
            Command::SetFullscreen { id, fullscreen } => {
                let fullscreen = match fullscreen {
                    Fullscreen::None => None,
                    Fullscreen::Borderless => Some(winit::window::Fullscreen::Borderless(None)),
                    Fullscreen::Exclusive => {
                        unreachable!("exclusive fullscreen is rejected during command planning")
                    }
                };
                let is_fullscreen = fullscreen.is_some();
                self.handle(id)?.winit().set_fullscreen(fullscreen);
                self.apply_patch(WindowStatePatch::Fullscreen {
                    id,
                    fullscreen: is_fullscreen,
                })?;
            }
            Command::SetLevel { id, level } => {
                self.handle(id)?.winit().set_window_level(level.into());
            }
            Command::SetTheme { id, theme } => {
                self.handle(id)?.winit().set_theme(theme.map(Into::into));
                if let Some(event) = self.apply_patch(WindowStatePatch::Theme { id, theme })? {
                    self.deliver_event(event_loop, id, event);
                }
            }
            Command::SetCursor { id, cursor } => {
                if self.cursor_state.get(&id) == Some(&cursor) {
                    return Ok(());
                }
                let window = self.handle(id)?;
                match cursor {
                    Cursor::Icon(icon) => {
                        window.winit().set_cursor_visible(true);
                        window.winit().set_cursor(icon);
                        self.cursor_state.insert(id, Cursor::Icon(icon));
                    }
                    Cursor::Hidden => {
                        window.winit().set_cursor_visible(false);
                        self.cursor_state.insert(id, Cursor::Hidden);
                    }
                    Cursor::Custom(_) => {
                        unreachable!("custom cursors are rejected during command planning")
                    }
                }
            }
            Command::SetCursorGrab { id, grab } => {
                let mode = match grab {
                    CursorGrab::None => winit::window::CursorGrabMode::None,
                    CursorGrab::Confined => winit::window::CursorGrabMode::Confined,
                    CursorGrab::Locked => winit::window::CursorGrabMode::Locked,
                };
                self.handle(id)?
                    .winit()
                    .set_cursor_grab(mode)
                    .map_err(|source| {
                        Error::new(ErrorCode::CursorRequestFailed, "cursor grab failed")
                            .with_id(id)
                            .with_source(source)
                    })?;
            }
            Command::SetIme { id, request } => {
                self.apply_ime(id, request)?;
            }
            Command::RequestUserAttention { id } => {
                self.handle(id)?
                    .winit()
                    .request_user_attention(Some(winit::window::UserAttentionType::Informational));
            }
            Command::RequestDraw { id } => {
                self.pending_draws.insert(id);
                self.handle(id)?.request_draw();
            }
            Command::Destroy { id } => {
                if let Some(state) = self.close(id) {
                    self.deliver_closed(event_loop, state);
                }
            }
        }
        Ok(())
    }

    fn handle(&self, id: Id) -> Result<Handle> {
        self.registry
            .get(id)
            .ok_or_else(|| Error::new(ErrorCode::HandleUnavailable, "unknown window").with_id(id))?
            .handle()
    }

    fn close(&mut self, id: Id) -> Option<WindowSnapshot> {
        let state = if let Some(instance) = self.registry.remove(id) {
            if let Some(handle) = instance.handle {
                self.windows.remove(&handle.winit().id());
            }
            self.pending_draws.remove(&id);
            Some(instance.state)
        } else {
            None
        };
        if state.is_some() {
            self.cursor_state.remove(&id);
            #[cfg(feature = "accessibility")]
            self.accessibility.remove(&id);
        }
        state
    }

    fn apply_ime(&mut self, id: Id, request: ImeRequest) -> Result<()> {
        let handle = self.handle(id)?;
        match request {
            ImeRequest::Disable => handle.winit().set_ime_allowed(false),
            ImeRequest::Enable(config) | ImeRequest::Update(config) => {
                handle.winit().set_ime_allowed(true);
                handle.winit().set_ime_purpose(config.purpose.into());
                if let Some(cursor_area) = config.cursor_area {
                    handle.winit().set_ime_cursor_area(
                        winit::dpi::LogicalPosition::new(
                            cursor_area.origin.x,
                            cursor_area.origin.y,
                        ),
                        winit::dpi::LogicalSize::new(
                            cursor_area.size.width,
                            cursor_area.size.height,
                        ),
                    );
                }
            }
            ImeRequest::Restart(config) => {
                handle.winit().set_ime_allowed(false);
                handle.winit().set_ime_allowed(true);
                handle.winit().set_ime_purpose(config.purpose.into());
            }
        }
        Ok(())
    }

    fn apply_patch(&mut self, patch: WindowStatePatch) -> Result<Option<EventKind>> {
        let id = patch.id();
        let instance = self
            .registry
            .get_mut(id)
            .ok_or_else(|| Error::new(ErrorCode::CommandFailed, "unknown window").with_id(id))?;
        patch.apply(&mut instance.state)
    }

    fn id_for_winit(&self, window_id: winit::window::WindowId) -> Option<Id> {
        self.windows.get(&window_id).copied()
    }
}

impl<H: Handler> winit::application::ApplicationHandler<UserEvent> for WinitRunner<H> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.stage_startup();
        for id in self.live_ids() {
            self.deliver_event(event_loop, id, EventKind::Resumed(id));
        }
        let action = self.call_with_context(|handler, context| handler.resume(context));
        self.finish_callback(event_loop, action);
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        for id in self.live_ids() {
            self.deliver_event(event_loop, id, EventKind::Suspended(id));
        }
        let action = self.call_with_context(|handler, context| handler.suspend(context));
        self.finish_callback(event_loop, action);
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Action(action) => self.apply_action(event_loop, action),
            UserEvent::Command(command) => {
                let result = HostCommandPlan::from_command(command, self.capabilities())
                    .and_then(|plan| self.apply_host_command(event_loop, plan.into_command()));
                if let Err(error) = result {
                    eprintln!("{error}");
                    event_loop.exit();
                }
            }
            #[cfg(feature = "accessibility")]
            UserEvent::Accessibility(event) => {
                if let Some(id) = self.id_for_winit(event.window_id) {
                    let event = accessibility_event_from_winit(id, event.window_event);
                    self.deliver_event(event_loop, id, EventKind::Accessibility(event));
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(id) = self.id_for_winit(window_id) else {
            return;
        };
        #[cfg(feature = "accessibility")]
        if let Ok(handle) = self.handle(id)
            && let Some(adapter) = self.accessibility.get_mut(&id)
        {
            adapter.process_event(handle.winit(), &event);
        }
        match event {
            winit::event::WindowEvent::CloseRequested => {
                self.deliver_close(event_loop, id);
            }
            winit::event::WindowEvent::Destroyed => {
                if let Some(state) = self.close(id) {
                    self.deliver_closed(event_loop, state);
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                self.pending_draws.remove(&id);
                let action = self.call_with_frame(id);
                self.finish_callback(event_loop, action);
            }
            winit::event::WindowEvent::Resized(size) => {
                if self.apply_metrics_transition(
                    event_loop,
                    id,
                    size.width,
                    size.height,
                    MetricsEvent::Resized,
                ) {
                    self.deliver_resize(event_loop, id);
                }
            }
            winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if self.apply_scale_factor_transition(event_loop, id, scale_factor) {
                    self.deliver_resize(event_loop, id);
                }
            }
            winit::event::WindowEvent::Moved(position) => {
                let point = Point {
                    x: f64::from(position.x),
                    y: f64::from(position.y),
                };
                let transition = NativeEventTransition::moved(id, point);
                self.apply_native_transition(event_loop, id, transition);
            }
            winit::event::WindowEvent::Focused(focused) => {
                let transition = NativeEventTransition::focused(id, focused);
                self.apply_native_transition(event_loop, id, transition);
            }
            winit::event::WindowEvent::ThemeChanged(theme) => {
                let theme = Some(theme.into());
                let transition = NativeEventTransition::theme_changed(id, theme);
                self.apply_native_transition(event_loop, id, transition);
            }
            winit::event::WindowEvent::Occluded(occluded) => {
                if !occluded {
                    self.request_pending_draws();
                }
                let transition = NativeEventTransition::occluded(id, occluded);
                self.apply_native_transition(event_loop, id, transition);
            }
            winit::event::WindowEvent::HoveredFile(path) => {
                let entered = !self.hovered_files.contains_key(&id);
                let paths = self.record_hovered_file(id, path);
                let position = self.last_mouse_position(id);
                let event = if entered {
                    FileDragEvent::Entered { id, paths }
                } else {
                    FileDragEvent::Hovered {
                        id,
                        paths,
                        position,
                    }
                };
                self.deliver_event(event_loop, id, EventKind::FileDrag(event));
            }
            winit::event::WindowEvent::DroppedFile(path) => {
                let paths = vec![path_to_string(path)];
                let position = self.last_mouse_position(id);
                self.hovered_files.remove(&id);
                self.deliver_event(
                    event_loop,
                    id,
                    EventKind::FileDrag(FileDragEvent::Dropped {
                        id,
                        paths,
                        position,
                    }),
                );
            }
            winit::event::WindowEvent::HoveredFileCancelled => {
                self.hovered_files.remove(&id);
                self.deliver_event(
                    event_loop,
                    id,
                    EventKind::FileDrag(FileDragEvent::Cancelled { id }),
                );
            }
            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state().into();
                self.deliver_input(
                    event_loop,
                    InputEvent::Modifiers {
                        id,
                        modifiers: self.modifiers,
                    },
                );
            }
            winit::event::WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                let input = InputEvent::Key(key_event_from_winit(
                    id,
                    &event,
                    self.modifiers,
                    is_synthetic,
                ));
                self.deliver_input(event_loop, input);
            }
            winit::event::WindowEvent::Ime(ime) => {
                let input = InputEvent::Ime(ime_event_from_winit(id, ime));
                self.deliver_input(event_loop, input);
            }
            winit::event::WindowEvent::CursorEntered { .. } => {
                let input = InputEvent::Pointer(PointerEvent {
                    id,
                    phase: PointerPhase::Entered,
                    kind: PointerKind::Mouse,
                    pointer_id: None,
                    position: self
                        .pointer_positions
                        .get(&PointerPositionKey::mouse(id))
                        .copied(),
                    physical_position: None,
                    delta: None,
                    button: None,
                    modifiers: self.modifiers,
                    device: PointerDeviceData::default(),
                    timestamp: Some(Instant::now()),
                });
                self.deliver_input(event_loop, input);
            }
            winit::event::WindowEvent::CursorLeft { .. } => {
                let position = self
                    .pointer_positions
                    .remove(&PointerPositionKey::mouse(id));
                let input = InputEvent::Pointer(PointerEvent {
                    id,
                    phase: PointerPhase::Left,
                    kind: PointerKind::Mouse,
                    pointer_id: None,
                    position,
                    physical_position: None,
                    delta: None,
                    button: None,
                    modifiers: self.modifiers,
                    device: PointerDeviceData::default(),
                    timestamp: Some(Instant::now()),
                });
                self.deliver_input(event_loop, input);
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                let point = self.logical_point(id, position.x, position.y);
                let previous = self
                    .pointer_positions
                    .insert(PointerPositionKey::mouse(id), point);
                let physical_position = PhysicalPoint {
                    x: position.x.round() as i32,
                    y: position.y.round() as i32,
                };
                let delta = previous.map(|previous| Point {
                    x: point.x - previous.x,
                    y: point.y - previous.y,
                });
                let transition = NativeEventTransition::mouse_moved(
                    id,
                    point,
                    physical_position,
                    delta,
                    self.modifiers,
                );
                self.apply_native_transition(event_loop, id, transition);
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let input = InputEvent::Pointer(PointerEvent {
                    id,
                    phase: pointer_phase_from_element_state(state),
                    kind: PointerKind::Mouse,
                    pointer_id: None,
                    position: self
                        .pointer_positions
                        .get(&PointerPositionKey::mouse(id))
                        .copied(),
                    physical_position: None,
                    delta: None,
                    button: Some(button.into()),
                    modifiers: self.modifiers,
                    device: PointerDeviceData::default(),
                    timestamp: Some(Instant::now()),
                });
                self.deliver_input(event_loop, input);
            }
            winit::event::WindowEvent::MouseWheel { delta, phase, .. } => {
                let input = InputEvent::Wheel(WheelEvent {
                    id,
                    delta: delta.into(),
                    phase: phase.into(),
                    position: self
                        .pointer_positions
                        .get(&PointerPositionKey::mouse(id))
                        .copied(),
                    modifiers: self.modifiers,
                    timestamp: Some(Instant::now()),
                });
                self.deliver_input(event_loop, input);
            }
            winit::event::WindowEvent::Touch(touch) => {
                let point = self.logical_point(id, touch.location.x, touch.location.y);
                let key = PointerPositionKey::touch(id, touch.id);
                let previous = self.pointer_positions.insert(key, point);
                if matches!(
                    touch.phase,
                    winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled
                ) {
                    self.pointer_positions.remove(&key);
                }
                let input = InputEvent::Pointer(PointerEvent {
                    id,
                    phase: touch_phase_as_pointer_phase(touch.phase),
                    kind: PointerKind::Touch,
                    pointer_id: Some(touch.id),
                    position: Some(point),
                    physical_position: Some(PhysicalPoint {
                        x: touch.location.x.round() as i32,
                        y: touch.location.y.round() as i32,
                    }),
                    delta: previous.map(|previous| Point {
                        x: point.x - previous.x,
                        y: point.y - previous.y,
                    }),
                    button: None,
                    modifiers: self.modifiers,
                    device: PointerDeviceData {
                        force: touch.force.map(|force| force.normalized()),
                        pressure: touch.force.map(|force| force.normalized()),
                        altitude: touch.force.and_then(|force| match force {
                            winit::event::Force::Calibrated { altitude_angle, .. } => {
                                altitude_angle
                            }
                            winit::event::Force::Normalized(_) => None,
                        }),
                        ..PointerDeviceData::default()
                    },
                    timestamp: Some(Instant::now()),
                });
                self.deliver_input(event_loop, input);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.handler.wants_idle() {
            let action = self.call_with_context(|handler, context| handler.idle(context));
            self.finish_callback(event_loop, action);
        }

        self.request_ready_draws();
        event_loop.set_control_flow(self.control_flow());
    }
}

pub(crate) fn validate_name(registry: &Registry, name: Option<&str>) -> Result<()> {
    let Some(name) = name else {
        return Ok(());
    };
    if name.is_empty() || registry.window_id(name).is_none() {
        return Ok(());
    }
    Err(Error::new(
        ErrorCode::CommandFailed,
        format!("duplicate window name '{name}'"),
    ))
}

pub(crate) const fn native_transition_route(event: &EventKind) -> NativeTransitionRoute {
    match event {
        EventKind::Input(_) => NativeTransitionRoute::Input,
        _ => NativeTransitionRoute::Event,
    }
}

#[cfg(feature = "accessibility")]
pub(crate) fn accessibility_event_from_winit(
    id: Id,
    event: accesskit_winit::WindowEvent,
) -> AccessibilityEvent {
    match event {
        accesskit_winit::WindowEvent::InitialTreeRequested => {
            AccessibilityEvent::InitialTreeRequested(id)
        }
        accesskit_winit::WindowEvent::ActionRequested(request) => {
            AccessibilityEvent::ActionRequested(AccessibilityActionRequest {
                id,
                action: format!("{:?}", request.action),
            })
        }
        accesskit_winit::WindowEvent::AccessibilityDeactivated => {
            AccessibilityEvent::Deactivated(id)
        }
    }
}

impl<H: Handler> WinitRunner<H> {
    fn live_ids(&self) -> Vec<Id> {
        self.windows.values().copied().collect()
    }

    fn apply_metrics_transition(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        id: Id,
        width: u32,
        height: u32,
        event: MetricsEvent,
    ) -> bool {
        let Some(existing) = self.registry.get(id).map(|window| window.metrics()) else {
            eprintln!(
                "{}",
                Error::new(ErrorCode::CommandFailed, "unknown window").with_id(id)
            );
            event_loop.exit();
            return false;
        };
        let scale = existing.scale_factor;
        let metrics = Metrics::from_physical_size(id, PhysicalSize { width, height }, scale)
            .with_outer_geometry(existing.outer_position, existing.outer_size);
        let transition = match event {
            MetricsEvent::Resized => NativeEventTransition::resized(metrics),
            MetricsEvent::ScaleFactorChanged => {
                NativeEventTransition::scale_factor_changed(metrics)
            }
        };
        self.apply_native_transition_patch(event_loop, transition)
    }

    fn apply_scale_factor_transition(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        id: Id,
        scale_factor: f64,
    ) -> bool {
        let Some(existing) = self.registry.get(id).map(|window| window.metrics()) else {
            eprintln!(
                "{}",
                Error::new(ErrorCode::CommandFailed, "unknown window").with_id(id)
            );
            event_loop.exit();
            return false;
        };
        let physical_size = existing.physical_size;
        let metrics = Metrics::from_physical_size(id, physical_size, scale_factor)
            .with_outer_geometry(existing.outer_position, existing.outer_size);
        let transition = NativeEventTransition::scale_factor_changed(metrics);
        self.apply_native_transition_patch(event_loop, transition)
    }

    fn apply_native_transition_patch(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        transition: NativeEventTransition,
    ) -> bool {
        let Some(patch) = transition.patch().cloned() else {
            return true;
        };
        match self.apply_patch(patch) {
            Ok(_) => true,
            Err(error) => {
                eprintln!("{error}");
                event_loop.exit();
                false
            }
        }
    }

    fn record_hovered_file(&mut self, id: Id, path: PathBuf) -> Vec<String> {
        let files = self.hovered_files.entry(id).or_default();
        files.push(path_to_string(path));
        files.clone()
    }

    pub(crate) fn last_mouse_position(&self, id: Id) -> Option<Point> {
        self.pointer_positions
            .get(&PointerPositionKey::mouse(id))
            .copied()
    }

    fn logical_point(&self, id: Id, physical_x: f64, physical_y: f64) -> Point {
        let scale = self
            .registry
            .get(id)
            .map(|window| window.metrics().scale_factor)
            .unwrap_or(1.0);
        Point {
            x: physical_x / scale,
            y: physical_y / scale,
        }
    }
}

pub(crate) fn state_from_winit(
    id: Id,
    request: &WindowRequest,
    window: &winit::window::Window,
) -> WindowSnapshot {
    let scale_factor = window.scale_factor();
    let inner_size = window.inner_size();
    let outer_position = window.outer_position().ok().map(|position| Point {
        x: f64::from(position.x),
        y: f64::from(position.y),
    });
    let outer_size = {
        let size = window.outer_size();
        Some(Size {
            width: f64::from(size.width) / scale_factor,
            height: f64::from(size.height) / scale_factor,
        })
    };
    let metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: inner_size.width,
            height: inner_size.height,
        },
        scale_factor,
    )
    .with_outer_geometry(outer_position, outer_size);
    WindowSnapshot::from_seed(WindowSnapshotSeed {
        id,
        title: request.title().to_owned(),
        name: request.name().map(str::to_owned),
        position: outer_position,
        focused: window.has_focus(),
        visible: Some(request.visible()),
        minimized: None,
        maximized: window.is_maximized(),
        occluded: None,
        fullscreen: window.fullscreen().is_some(),
        theme: request.theme(),
        role: request.role().clone(),
        metrics,
    })
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

impl From<winit::window::Theme> for Theme {
    fn from(theme: winit::window::Theme) -> Self {
        match theme {
            winit::window::Theme::Light => Self::Light,
            winit::window::Theme::Dark => Self::Dark,
        }
    }
}

impl From<winit::keyboard::ModifiersState> for ModifierState {
    fn from(modifiers: winit::keyboard::ModifiersState) -> Self {
        Self {
            shift: modifiers.shift_key(),
            control: modifiers.control_key(),
            alt: modifiers.alt_key(),
            super_key: modifiers.super_key(),
        }
    }
}

impl From<winit::event::MouseButton> for PointerButton {
    fn from(button: winit::event::MouseButton) -> Self {
        match button {
            winit::event::MouseButton::Left => Self::Primary,
            winit::event::MouseButton::Right => Self::Secondary,
            winit::event::MouseButton::Middle => Self::Middle,
            winit::event::MouseButton::Back => Self::Back,
            winit::event::MouseButton::Forward => Self::Forward,
            winit::event::MouseButton::Other(button) => Self::Other(button),
        }
    }
}

impl From<winit::event::TouchPhase> for TouchPhase {
    fn from(phase: winit::event::TouchPhase) -> Self {
        match phase {
            winit::event::TouchPhase::Started => Self::Started,
            winit::event::TouchPhase::Moved => Self::Moved,
            winit::event::TouchPhase::Ended => Self::Ended,
            winit::event::TouchPhase::Cancelled => Self::Cancelled,
        }
    }
}

impl From<winit::event::MouseScrollDelta> for WheelDelta {
    fn from(delta: winit::event::MouseScrollDelta) -> Self {
        match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => Self::Lines {
                x: f64::from(x),
                y: f64::from(y),
            },
            winit::event::MouseScrollDelta::PixelDelta(position) => Self::Pixels {
                x: position.x,
                y: position.y,
            },
        }
    }
}

fn pointer_phase_from_element_state(state: winit::event::ElementState) -> PointerPhase {
    match state {
        winit::event::ElementState::Pressed => PointerPhase::Pressed,
        winit::event::ElementState::Released => PointerPhase::Released,
    }
}

fn touch_phase_as_pointer_phase(phase: winit::event::TouchPhase) -> PointerPhase {
    match phase {
        winit::event::TouchPhase::Started => PointerPhase::Pressed,
        winit::event::TouchPhase::Moved => PointerPhase::Moved,
        winit::event::TouchPhase::Ended => PointerPhase::Released,
        winit::event::TouchPhase::Cancelled => PointerPhase::Cancelled,
    }
}

pub(crate) fn ime_event_from_winit(id: Id, ime: winit::event::Ime) -> ImeEvent {
    match ime {
        winit::event::Ime::Enabled => ImeEvent::Enabled { id },
        winit::event::Ime::Disabled => ImeEvent::Disabled { id },
        winit::event::Ime::Preedit(text, cursor) => ImeEvent::Preedit { id, text, cursor },
        winit::event::Ime::Commit(text) => ImeEvent::Commit { id, text },
    }
}

fn key_event_from_winit(
    id: Id,
    event: &winit::event::KeyEvent,
    modifiers: ModifierState,
    synthetic: bool,
) -> KeyEvent {
    KeyEvent {
        id,
        logical_key: key_from_winit(&event.logical_key),
        physical_key: code_from_winit(&event.physical_key),
        location: location_from_winit(event.location),
        state: key_state_from_winit(event.state),
        repeat: event.repeat,
        synthetic,
        modifiers,
        timestamp: Some(Instant::now()),
    }
}

fn key_state_from_winit(state: winit::event::ElementState) -> KeyState {
    match state {
        winit::event::ElementState::Pressed => KeyState::Pressed,
        winit::event::ElementState::Released => KeyState::Released,
    }
}

pub(crate) fn location_from_winit(
    location: winit::keyboard::KeyLocation,
) -> keyboard_types::Location {
    match location {
        winit::keyboard::KeyLocation::Standard => keyboard_types::Location::Standard,
        winit::keyboard::KeyLocation::Left => keyboard_types::Location::Left,
        winit::keyboard::KeyLocation::Right => keyboard_types::Location::Right,
        winit::keyboard::KeyLocation::Numpad => keyboard_types::Location::Numpad,
    }
}

pub(crate) fn key_from_winit(key: &winit::keyboard::Key) -> keyboard_types::Key {
    match key {
        winit::keyboard::Key::Character(character) => {
            keyboard_types::Key::Character(character.to_string())
        }
        winit::keyboard::Key::Named(named) => format!("{named:?}")
            .parse()
            .unwrap_or(keyboard_types::Key::Unidentified),
        winit::keyboard::Key::Dead(_) | winit::keyboard::Key::Unidentified(_) => {
            keyboard_types::Key::Unidentified
        }
    }
}

pub(crate) fn code_from_winit(physical_key: &winit::keyboard::PhysicalKey) -> keyboard_types::Code {
    match physical_key {
        winit::keyboard::PhysicalKey::Code(code) => format!("{code:?}")
            .parse()
            .unwrap_or(keyboard_types::Code::Unidentified),
        winit::keyboard::PhysicalKey::Unidentified(_) => keyboard_types::Code::Unidentified,
    }
}

impl From<ImePurpose> for winit::window::ImePurpose {
    fn from(purpose: ImePurpose) -> Self {
        match purpose {
            ImePurpose::Normal => Self::Normal,
            ImePurpose::Password => Self::Password,
            ImePurpose::Number | ImePurpose::Email | ImePurpose::Url => Self::Normal,
            ImePurpose::Terminal => Self::Terminal,
        }
    }
}
