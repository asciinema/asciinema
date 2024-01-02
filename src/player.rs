use crate::format::asciicast;
use anyhow::Result;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

pub fn play(
    recording: impl io::Read,
    mut output: impl io::Write,
    speed: f64,
    idle_time_limit: Option<f64>,
) -> Result<()> {
    let reader = io::BufReader::new(recording);
    let (header, events) = asciicast::open(reader)?;

    let idle_time_limit = idle_time_limit
        .or(header.idle_time_limit)
        .unwrap_or(f64::MAX);

    let events = asciicast::limit_idle_time(events, idle_time_limit);
    let events = asciicast::accelerate(events, speed);
    let events = asciicast::output(events);
    let epoch = Instant::now();

    for event in events {
        let (time, data) = event?;
        let diff = time as i64 - epoch.elapsed().as_micros() as i64;

        if diff > 0 {
            output.flush()?;
            thread::sleep(Duration::from_micros(diff as u64));
        }

        output.write_all(data.as_bytes())?;
    }

    Ok(())
}
