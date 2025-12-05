use std::fmt::{self, Display};

#[cfg(feature = "events")]
use chrono::{DateTime, Utc};
use chrono::{Local, NaiveTime, TimeZone};
use regex::Regex;
#[cfg(feature = "events")]
use uuid::Uuid;

#[cfg(feature = "mc-vanilla")]
use crate::error::ParserError;
use crate::instance::InstanceStatus;

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

#[cfg(feature = "mc-vanilla")]
pub struct LogMeta {
    time: String,
    thread: String,
    level: LogLevel,
    msg: String,
}

#[cfg(feature = "mc-vanilla")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Other,
}

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

    pub timestamp: DateTime<Utc>,

    pub payload: EventPayload,
}

#[cfg(feature = "mc-vanilla")]
impl LogMeta {
    pub fn new<S: Into<String>>(line: S) -> Result<Option<Self>, ParserError> {
        let line: String = line.into();
        let line = line.trim();

        if !line.starts_with('[') {
            return Ok(None);
        }

        let time_end = match line.find(']') {
            Some(i) => i,
            None => return Ok(None),
        };
        let time = line[1..time_end].to_string();

        let meta_start = match line[time_end + 1..].find('[') {
            Some(j) => time_end + 1 + j,
            None => return Ok(None),
        };

        let msg_sep = match line[meta_start..].find("]: ") {
            Some(k) => meta_start + k,
            None => return Ok(None),
        };

        let meta = &line[(meta_start + 1)..msg_sep]; // inside the brackets
        let msg = line[(msg_sep + 3)..].to_string(); // after "]: "

        let mut thread_level = meta.splitn(2, '/');
        let thread = thread_level
            .next()
            .ok_or(ParserError::ParserError)?
            .to_string();
        let level_str = thread_level
            .next()
            .ok_or(ParserError::ParserError)?
            .trim_end_matches(']'); // just in case

        let level = match level_str {
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => LogLevel::Other,
        };

        Ok(Some(LogMeta {
            time,
            thread,
            level,
            msg,
        }))
    }
}

#[cfg(feature = "mc-vanilla")]
impl Display for LogMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line = format!(
            "Time: {}\nThread: {}\nLevel: {}\nMessage: {}",
            self.time, self.thread, self.level, self.msg
        );

        write!(f, "{}", line)
    }
}

#[cfg(feature = "mc-vanilla")]
impl Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Other => write!(f, "OTHER"),
        }
    }
}

impl StreamLine {
    pub fn new<S: Into<String>>(line: S, source: StreamSource) -> Self {
        let line = line.into();
        let re = Regex::new(r#"^\[[^\]]*\]\s*\[[^\]]*\]:\s*"#).unwrap();
        let line = re.replace(&line, "").to_string();
        Self { line, source }
    }

    pub fn stdout<S: Into<String>>(line: S) -> Self {
        let line = line.into();
        let re = Regex::new(r#"^\[[^\]]*\]\s*\[[^\]]*\]:\s*"#).unwrap();
        let line = re.replace(&line, "").to_string();
        Self {
            line,
            source: StreamSource::Stdout,
        }
    }

    pub fn stderr<S: Into<String>>(line: S) -> Self {
        let line = line.into();
        let re = Regex::new(r#"^\[[^\]]*\]\s*\[[^\]]*\]:\s*"#).unwrap();
        let line = re.replace(&line, "").to_string();
        Self {
            line,
            source: StreamSource::Stderr,
        }
    }

    pub fn msg(&self) -> String {
        self.line.clone()
    }

    pub fn extract_timestamp(&self) -> Option<DateTime<Utc>> {
        let input = self.line.as_str();
        let re = Regex::new(r"\[(.*?)\]").unwrap();
        let time_s = re.captures(input).map(|v| v[1].to_string());
        time_s.as_ref()?;
        let time = NaiveTime::parse_from_str(&time_s.unwrap(), "%H:%M:%S").ok()?;

        let today = Local::now().date_naive();
        let naive_dt = today.and_time(time);

        let local_dt = Local.from_local_datetime(&naive_dt).unwrap();

        let utc_dt = local_dt.with_timezone(&Utc);

        Some(utc_dt)
    }
}

impl InstanceEvent {
    pub fn stdout<S: Into<String>>(line: S) -> Self {
        let line = line.into();
        let s_line = StreamLine::stdout(line);
        let timestamp = s_line.extract_timestamp().unwrap_or(Utc::now());
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
        let timestamp = s_line.extract_timestamp().unwrap_or(Utc::now());
        let payload = EventPayload::StdLine { line: s_line };

        Self {
            id: Uuid::new_v4(),
            timestamp,
            payload,
        }
    }
}

impl Display for StreamLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line)
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
