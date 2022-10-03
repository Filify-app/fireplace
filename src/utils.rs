use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;

pub fn get_unix_time() -> Result<u64, anyhow::Error> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("Failed to get system time")?;
    Ok(duration.as_secs())
}
