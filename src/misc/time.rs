use anyhow::Result;
use std::{fmt::Write, write};

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn ppr_time_diff(mut w: impl Write, timestamp_ms: u64) -> Result<()> {
    let now = now();
    let t_diff_secs = (now - timestamp_ms) / 1000;

    if t_diff_secs <= 1 {
        w.write_str("1 second")?;
    } else if t_diff_secs < 100 {
        write!(w, "{} seconds ago", t_diff_secs)?;
    } else {
        let hours = t_diff_secs / 3600;
        let mins = (t_diff_secs % 3600) / 60;
        let secs = (t_diff_secs % 3600) % 60;
        write!(w, "{}:{}:{}", hours, mins, secs)?;
    }

    Ok(())
}
