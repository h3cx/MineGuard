use std::fmt::{self, Display};

/// Identifies which process stream produced a line of output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamSource {
    Stdout,
    Stderr,
    #[cfg(feature = "events")]
    Event,
}

/// Captures a single line of process output along with its origin stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamLine {
    line: String,
    source: StreamSource,
}

#[cfg(feature = "events")]
pub struct InstanceEvent {}

#[cfg(feature = "events")]
pub enum Events {}

impl StreamLine {
    pub fn new<S: Into<String>>(line: S, source: StreamSource) -> Self {
        Self {
            line: line.into(),
            source,
        }
    }

    pub fn stdout<S: Into<String>>(line: S) -> Self {
        Self {
            line: line.into(),
            source: StreamSource::Stdout,
        }
    }

    pub fn stderr<S: Into<String>>(line: S) -> Self {
        Self {
            line: line.into(),
            source: StreamSource::Stderr,
        }
    }
}

impl Display for StreamLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line)
    }
}
