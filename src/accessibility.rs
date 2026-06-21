use super::{Id, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum AccessibilityEvent {
    InitialTreeRequested(Id),
    ActionRequested(AccessibilityActionRequest),
    Deactivated(Id),
}

impl AccessibilityEvent {
    #[must_use]
    pub const fn id(&self) -> Id {
        match self {
            Self::InitialTreeRequested(id) | Self::Deactivated(id) => *id,
            Self::ActionRequested(request) => request.id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibilityActionRequest {
    pub id: Id,
    pub action: String,
}

/// Receives native accessibility adapter events.
pub trait AccessibilityBridge {
    fn accessibility_event(&mut self, event: AccessibilityEvent) -> Result<()>;
}
