use chrono::{DateTime, Utc};

static mut SAVED_NOW: Option<DateTime<Utc>> = None;

pub fn get_now() -> DateTime<Utc> {
    unsafe { SAVED_NOW.unwrap() }
}

pub fn set_now(now: DateTime<Utc>) {
    unsafe { SAVED_NOW = Some(now) }
}
