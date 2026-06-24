use super::{
    Command, Error, ErrorCode, Handle, Id, Metrics, Open, Proxy, Ref, Registry, Selector, Target,
    command::Action,
};
use std::time::Instant;

/// Event-loop context passed to handlers.
pub struct Context<'a> {
    pub(crate) registry: &'a mut Registry,
    pub(crate) commands: &'a mut Vec<Command>,
    pub(crate) actions: &'a mut Vec<Action>,
    action: Action,
    proxy: Option<Proxy>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        registry: &'a mut Registry,
        commands: &'a mut Vec<Command>,
        actions: &'a mut Vec<Action>,
        proxy: Option<Proxy>,
    ) -> Self {
        let action = resolve_actions(actions);
        Self {
            registry,
            commands,
            actions,
            action,
            proxy,
        }
    }

    pub fn registry(&self) -> &Registry {
        self.registry
    }

    pub(crate) fn request(&mut self, action: impl Into<Action>) -> &mut Self {
        self.actions.push(action.into());
        self.refresh_action();
        self
    }

    pub(crate) fn send(&mut self, command: impl Into<Command>) -> &mut Self {
        self.commands.push(command.into());
        self
    }

    pub fn open(&mut self, open: Open) -> &mut Self {
        self.send(open)
    }

    pub fn close(&mut self, id: Id) -> &mut Self {
        self.request(Action::CloseRequested(id))
    }

    pub fn draw(&mut self, id: Id) -> &mut Self {
        self.request(Action::DrawNext(id))
    }

    pub fn again(&mut self, id: Id) -> &mut Self {
        self.request(Action::DrawNow(id))
    }

    pub fn at(&mut self, id: Id, time: Instant) -> &mut Self {
        self.request(Action::DrawAt { id, time })
    }

    pub fn exit(&mut self) -> &mut Self {
        self.request(Action::Exit)
    }

    #[must_use]
    pub(crate) fn action(&self) -> &Action {
        &self.action
    }

    #[must_use]
    pub fn window_id(&self, name: impl AsRef<str>) -> Option<Id> {
        self.registry.window_id(name)
    }

    #[must_use]
    pub fn state(&self, target: impl Into<Selector>) -> Option<&super::WindowSnapshot> {
        match target.into() {
            Selector::Id(id) => self.registry.get(id).map(|window| window.instance.state()),
            Selector::Name(name) => self
                .registry
                .window_id(name)
                .and_then(|id| self.registry.get(id))
                .map(|window| window.instance.state()),
        }
    }

    pub fn access(&self, id: Id) -> super::Result<Ref<'_>> {
        self.registry
            .get(id)
            .ok_or_else(|| Error::new(ErrorCode::HandleUnavailable, "unknown window").with_id(id))
    }

    pub fn handle(&self, id: Id) -> super::Result<Handle> {
        use super::Access;

        self.access(id)?.handle()
    }

    pub fn metrics(&self, id: Id) -> super::Result<Metrics> {
        use super::Access;

        Ok(self.access(id)?.metrics())
    }

    pub fn window(&mut self, id: Id) -> Target<'_> {
        Target::new(id, self.commands, self.actions, &mut self.action)
    }

    pub(crate) fn resolved_action(&self) -> Action {
        self.action.clone()
    }

    fn refresh_action(&mut self) {
        self.action = resolve_actions(self.actions);
    }

    #[must_use]
    pub fn proxy(&self) -> Option<Proxy> {
        self.proxy.clone()
    }
}

pub(crate) fn resolve_actions(actions: &[Action]) -> Action {
    match actions {
        [] => Action::Wait,
        [action] => action.clone(),
        actions => Action::Batch(actions.to_vec()),
    }
}

pub(crate) fn resolve_actions_with(actions: &[Action], fallback: Action) -> Action {
    let mut batch = Vec::with_capacity(actions.len() + 1);
    push_unique_action(&mut batch, fallback);
    for action in actions
        .iter()
        .filter(|action| !matches!(action, Action::Wait))
        .cloned()
    {
        push_unique_action(&mut batch, action);
    }

    match batch.as_slice() {
        [] => Action::Wait,
        [action] => action.clone(),
        _ => Action::Batch(batch),
    }
}

fn push_unique_action(actions: &mut Vec<Action>, action: Action) {
    if !actions.iter().any(|stored| stored == &action) {
        actions.push(action);
    }
}
