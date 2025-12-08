use std::fmt::{self, Display};

use regex::Regex;

#[cfg(feature = "events")]
use chrono::{DateTime, Local, NaiveTime, TimeZone, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamSource {
    Stdout,
    Stderr,
    #[cfg(feature = "events")]
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamLine {
    pub line: String,
    pub source: StreamSource,
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
        Self {
            line,
            source: StreamSource::Stdout,
        }
    }

    pub fn stderr<S: Into<String>>(line: S) -> Self {
        let line = line.into();
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

impl Display for StreamLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line)
    }
}
