use super::{Command, Error, ErrorCode, Id, Metrics, Result, WindowSnapshot, command::Action};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{collections::HashMap, sync::Arc};

/// Owned live native window entry.
#[derive(Debug)]
pub struct Instance {
    pub(crate) id: Id,
    pub(crate) state: WindowSnapshot,
    pub(crate) handle: Option<Handle>,
}

impl Instance {
    #[must_use]
    pub fn new(id: Id, state: WindowSnapshot) -> Self {
        Self {
            id,
            state,
            handle: None,
        }
    }

    #[must_use]
    pub fn with_handle(id: Id, state: WindowSnapshot, handle: Handle) -> Self {
        Self {
            id,
            state,
            handle: Some(handle),
        }
    }

    #[must_use]
    pub const fn id(&self) -> Id {
        self.id
    }

    #[must_use]
    pub const fn state(&self) -> &WindowSnapshot {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut WindowSnapshot {
        &mut self.state
    }

    #[must_use]
    pub fn as_ref(&self) -> Ref<'_> {
        Ref { instance: self }
    }
}

/// Borrowed native window access.
#[derive(Clone, Copy, Debug)]
pub struct Ref<'a> {
    pub(crate) instance: &'a Instance,
}

/// Cloneable owner-backed native handle token.
#[derive(Clone, Debug)]
pub struct Handle {
    window: Arc<winit::window::Window>,
}

impl Handle {
    #[must_use]
    pub(crate) fn from_winit(window: Arc<winit::window::Window>) -> Self {
        Self { window }
    }

    #[must_use]
    pub(crate) fn winit(&self) -> &Arc<winit::window::Window> {
        &self.window
    }

    pub(crate) fn request_draw(&self) {
        self.window.request_redraw();
    }
}

impl raw_window_handle::HasWindowHandle for Handle {
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        self.window.window_handle()
    }
}

impl raw_window_handle::HasDisplayHandle for Handle {
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        self.window.display_handle()
    }
}

/// Cross-thread handle for sending typed window commands and event-loop actions.
///
/// Obtain a proxy from `Context::proxy()` inside a handler callback, clone it,
/// and move it to another thread when external work needs to wake the window
/// loop. Public command helpers are implemented on `Proxy` by the app-facing
/// DSL module: `send`, `open`, `close`, `draw`, `again`, `at`, and `exit`.
#[derive(Clone, Debug)]
pub struct Proxy {
    pub(crate) inner: winit::event_loop::EventLoopProxy<UserEvent>,
}

impl Proxy {
    pub(crate) fn command(&self, command: Command) -> Result<()> {
        self.send_user_event(UserEvent::Command(command))
    }

    pub(crate) fn request_action(&self, action: Action) -> Result<()> {
        self.send_user_event(UserEvent::Action(action))
    }

    /// Enqueue an event-loop exit action on the native loop.
    ///
    /// Returns [`ErrorCode::CommandFailed`] if the event loop is closed.
    pub fn exit(&self) -> Result<()> {
        self.send_user_event(UserEvent::Action(Action::Exit))
    }

    fn send_user_event(&self, event: UserEvent) -> Result<()> {
        self.inner
            .send_event(event)
            .map_err(|_| Error::new(ErrorCode::CommandFailed, "event loop is closed"))
    }
}

#[derive(Debug)]
pub(crate) enum UserEvent {
    Action(Action),
    Command(Command),
    #[cfg(feature = "accessibility")]
    Accessibility(accesskit_winit::Event),
}

#[cfg(feature = "accessibility")]
impl From<accesskit_winit::Event> for UserEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::Accessibility(event)
    }
}

/// Borrowed native window capabilities.
pub trait Access {
    fn id(&self) -> Id;
    fn metrics(&self) -> Metrics;
    fn handle(&self) -> Result<Handle>;
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>>;
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>>;
}

impl Access for Ref<'_> {
    fn id(&self) -> Id {
        self.instance.id
    }

    fn metrics(&self) -> Metrics {
        self.instance.state.metrics().clone()
    }

    fn handle(&self) -> Result<Handle> {
        self.instance.handle.clone().ok_or_else(|| {
            Error::new(ErrorCode::HandleUnavailable, "native handle unavailable").with_id(self.id())
        })
    }

    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>> {
        self.instance
            .handle
            .as_ref()
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::HandleUnavailable,
                    "native window handle unavailable",
                )
                .with_id(self.id())
            })?
            .window_handle()
            .map_err(|source| {
                Error::new(
                    ErrorCode::HandleUnavailable,
                    "native window handle unavailable",
                )
                .with_id(self.id())
                .with_source(source)
            })
    }

    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>> {
        self.instance
            .handle
            .as_ref()
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::HandleUnavailable,
                    "native display handle unavailable",
                )
                .with_id(self.id())
            })?
            .display_handle()
            .map_err(|source| {
                Error::new(
                    ErrorCode::HandleUnavailable,
                    "native display handle unavailable",
                )
                .with_id(self.id())
                .with_source(source)
            })
    }
}

/// Live native windows keyed by `Id`.
#[derive(Debug, Default)]
pub struct Registry {
    next: u64,
    instances: HashMap<Id, Instance>,
}

impl Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn reserve_id(&mut self) -> Id {
        self.next += 1;
        Id::from_u64(self.next)
    }

    pub fn insert(&mut self, instance: Instance) -> Option<Instance> {
        self.instances.insert(instance.id(), instance)
    }

    pub fn remove(&mut self, id: Id) -> Option<Instance> {
        self.instances.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: Id) -> Option<Ref<'_>> {
        self.instances.get(&id).map(Instance::as_ref)
    }

    #[must_use]
    pub fn window_id(&self, name: impl AsRef<str>) -> Option<Id> {
        let name = name.as_ref();
        self.instances
            .values()
            .find(|instance| instance.state.name() == Some(name))
            .map(Instance::id)
    }

    pub(crate) fn get_mut(&mut self, id: Id) -> Option<&mut Instance> {
        self.instances.get_mut(&id)
    }

    #[must_use]
    pub fn contains(&self, id: Id) -> bool {
        self.instances.contains_key(&id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}
