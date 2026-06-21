use super::CursorIcon;

#[derive(Clone, Debug, PartialEq)]
pub enum Cursor {
    Icon(CursorIcon),
    Hidden,
    Custom(CustomCursorId),
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
