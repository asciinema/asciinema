use crate::format::asciicast;
use anyhow::Result;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

pub fn play(input: impl io::Read, speed: f64, idle_time_limit: Option<f64>) -> Result<()> {
    let reader = io::BufReader::new(input);
    let (header, events) = asciicast::open(reader)?;
    let mut stdout = io::stdout();

    let idle_time_limit = idle_time_limit
        .or(header.idle_time_limit)
        .unwrap_or(f64::MAX);

    let events = asciicast::limit_idle_time(events, idle_time_limit);
    let events = asciicast::accelerate(events, speed);
    let output = asciicast::output(events);
    let epoch = Instant::now();

    for o in output {
        let (time, data) = o?;
        let diff = time as i64 - epoch.elapsed().as_micros() as i64;

        if diff > 0 {
            stdout.flush().unwrap();
            thread::sleep(Duration::from_micros(diff as u64));
        }

        stdout.write_all(data.as_bytes())?;
    }

    Ok(())
}
