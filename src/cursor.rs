use super::{CursorCapability, CursorIcon};

#[derive(Clone, Debug, PartialEq)]
pub enum Cursor {
    Icon(CursorIcon),
    Hidden,
    Custom(CustomCursorId),
}

impl Cursor {
    #[must_use]
    pub const fn capability(&self) -> CursorCapability {
        match self {
            Self::Icon(_) => CursorCapability::Icon,
            Self::Hidden => CursorCapability::Hidden,
            Self::Custom(_) => CursorCapability::Custom,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CustomCursorId(u64);

impl CustomCursorId {
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CursorGrab {
    #[default]
    None,
    Confined,
    Locked,
}
