use super::Id;
use std::{error, fmt};

/// Module result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Stable diagnostic error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    pub id: Option<Id>,
    pub source: Option<Box<dyn error::Error + Send + Sync>>,
}

impl Error {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            id: None,
            source: None,
        }
    }

    #[must_use]
    pub fn with_id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    #[must_use]
    pub fn with_source(mut self, source: impl error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = self.id {
            write!(
                f,
                "{:?} for window {}: {}",
                self.code,
                id.as_u64(),
                self.message
            )
        } else {
            write!(f, "{:?}: {}", self.code, self.message)
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn error::Error + 'static))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    EventLoopCreateFailed,
    WindowCreateFailed,
    HandleUnavailable,
    ImeUnsupported,
    ImeRequestFailed,
    ClipboardUnavailable,
    ClipboardReadFailed,
    ClipboardWriteFailed,
    CursorRequestFailed,
    CommandFailed,
    UnsupportedFeature,
    AccessibilityAdapterFailed,
    UnknownNativeError,
}
