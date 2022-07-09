use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_current_unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs()
        .try_into()
        .expect("time should not be larger than 2^63-1")
}