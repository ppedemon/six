use anyhow::{Context, Result};
use chrono::{Datelike, Local, TimeZone, Timelike};
use std::{fmt::Write, write};

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn ppr_timestamp_ms(mut w: impl Write, timestamp_ms: u64) -> Result<()> {
    let now = now();
    let t_diff_secs = (now - timestamp_ms) / 1000;

    if t_diff_secs <= 1 {
        w.write_str("1 second")?;
    } else if t_diff_secs < 100 {
        write!(w, "{} seconds ago", t_diff_secs)?;
    } else {
        let local_time = Local
            .timestamp_millis_opt(i64::try_from(timestamp_ms)?)
            .single()
            .context("timestamp is outside the supported date range")?;
        if local_time.date_naive() != Local::now().date_naive() {
            write!(
                w,
                "{:04}/{:02}/{:02} ",
                local_time.year(),
                local_time.month(),
                local_time.day()
            )?;
        }
        let hours = local_time.hour();
        let mins = local_time.minute();
        let secs = local_time.second();
        write!(w, "{hours:02}:{mins:02}:{secs:02}")?;
    }

    Ok(())
}
