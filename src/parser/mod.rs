use regex::Regex;

use crate::{
    config::{
        LogMeta,
        stream::{EventPayload, InstanceEvent, InternalEvent, LogLevel},
    },
    error::ParserError,
};

impl LogMeta {
    pub fn parse_event(&self) -> Result<Option<InternalEvent>, ParserError> {
        if self.thread == "Server thread" && self.level == LogLevel::Info {
            return self.parse_server_thread_info_lv2();
        }
        Ok(None)
    }

    fn parse_server_thread_info_lv2(&self) -> Result<Option<InternalEvent>, ParserError> {
        let re = Regex::new(r"Done \([0-9.]+s\)!").unwrap();
        if re.is_match(&self.msg) {
            return Ok(Some(InternalEvent::ServerStarted));
        }
        Ok(None)
    }
}
