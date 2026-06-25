use super::winit_adapter::WinitRunner;
use super::winit_mapping;
use super::{
    Clipboard, Command, DrawScheduler, Error, ErrorCode, Handler, MemoryClipboard, Proxy, Registry,
    Result, UserEvent,
};
#[cfg(test)]
use super::{Context, command::Action};

/// Native event-loop owner.
pub struct Loop<H> {
    pub(crate) handler: H,
    pub(crate) registry: Registry,
    pub(crate) draw: DrawScheduler,
    pub(crate) clipboard: Box<dyn Clipboard>,
    pub(crate) commands: Vec<Command>,
    #[cfg(test)]
    pub(crate) actions: Vec<Action>,
    pub(crate) startup: Vec<Command>,
}

impl<H> Loop<H> {
    #[must_use]
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            registry: Registry::new(),
            draw: DrawScheduler::new(),
            clipboard: Box::new(MemoryClipboard::new()),
            commands: Vec::new(),
            #[cfg(test)]
            actions: Vec::new(),
            startup: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_clipboard(mut self, clipboard: Box<dyn Clipboard>) -> Self {
        self.clipboard = clipboard;
        self
    }

    #[must_use]
    pub fn handler(&self) -> &H {
        &self.handler
    }

    pub fn handler_mut(&mut self) -> &mut H {
        &mut self.handler
    }

    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn clipboard_mut(&mut self) -> &mut dyn Clipboard {
        self.clipboard.as_mut()
    }

    #[cfg(test)]
    pub(crate) fn context(&mut self) -> Context<'_> {
        Context::new(
            &mut self.registry,
            &mut self.commands,
            &mut self.actions,
            None,
        )
    }
}

impl<H: Handler + 'static> Loop<H> {
    /// Run the native winit event loop.
    pub fn run(self) -> Result<()> {
        let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event()
            .build()
            .map_err(|source| {
                Error::new(
                    ErrorCode::EventLoopCreateFailed,
                    "failed to create native event loop",
                )
                .with_source(source)
            })?;
        let proxy = Proxy {
            inner: event_loop.create_proxy(),
        };
        let mut runner = WinitRunner::from_loop(self);
        runner.proxy = Some(proxy);
        event_loop.set_control_flow(winit_mapping::native_control_flow(
            winit::event_loop::ControlFlow::Wait,
        ));
        event_loop.run_app(&mut runner).map_err(|source| {
            Error::new(ErrorCode::UnknownNativeError, "native event loop failed")
                .with_source(source)
        })
    }
}
