use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone, Timelike, Utc};
use regex::Regex;

pub fn extract_timestamp(input: &str) -> Option<DateTime<Utc>> {
    let re = Regex::new(r"\[(.*?)\]").unwrap();
    let time_s = re.captures(input).map(|v| v[1].to_string());
    time_s.as_ref()?;
    let time = NaiveTime::parse_from_str(&time_s.unwrap(), "%H:%M:%S").ok()?;

    let today = Local::now().date_naive();

    let local_dt = Local
        .with_ymd_and_hms(
            today.year(),
            today.month(),
            today.day(),
            time.hour(),
            time.minute(),
            time.second(),
        )
        .single()?;

    Some(local_dt.with_timezone(&Utc))
}
