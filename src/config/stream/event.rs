use std::fmt::{self, Display};

use uuid::Uuid;

use crate::instance::InstanceStatus;

use super::line::StreamLine;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventPayload {
    #[cfg(feature = "events")]
    StateChange {
        old: InstanceStatus,
        new: InstanceStatus,
    },

    StdLine {
        line: StreamLine,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceEvent {
    pub id: Uuid,

    pub timestamp: chrono::DateTime<chrono::Utc>,

    pub payload: EventPayload,
}

impl InstanceEvent {
    pub fn stdout<S: Into<String>>(line: S) -> Self {
        let line = line.into();
        let s_line = StreamLine::stdout(line);
        let timestamp = s_line.extract_timestamp().unwrap_or(chrono::Utc::now());
        let payload = EventPayload::StdLine { line: s_line };

        Self {
            id: Uuid::new_v4(),
            timestamp,
            payload,
        }
    }

    pub fn stderr<S: Into<String>>(line: S) -> Self {
        let line = line.into();
        let s_line = StreamLine::stderr(line);
        let timestamp = s_line.extract_timestamp().unwrap_or(chrono::Utc::now());
        let payload = EventPayload::StdLine { line: s_line };

        Self {
            id: Uuid::new_v4(),
            timestamp,
            payload,
        }
    }
}

impl Display for InstanceEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let head = format!(
            "UUID: {}\nTimestamp:{}\nPayload:\n",
            self.id, self.timestamp
        );
        match self.payload.clone() {
            EventPayload::StdLine { line } => {
                let full = format!("{}{}", head, line);
                writeln!(f, "{}", full)
            }

            #[cfg(feature = "events")]
            EventPayload::StateChange { old, new } => {
                let full = format!("{}State changed: {:?} -> {:?}", head, old, new);
                writeln!(f, "{}", full)
            }
        }
    }
}
