use std::fmt::{self, Display};

use crate::error::ParserError;

#[cfg(feature = "mc-vanilla")]
pub struct LogMeta {
    pub time: String,
    pub thread: String,
    pub level: LogLevel,
    pub msg: String,
}

#[cfg(feature = "mc-vanilla")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Other,
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
