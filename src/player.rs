use crate::format::asciicast;
use anyhow::Result;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

pub fn play(input: impl io::Read) -> Result<()> {
    let reader = io::BufReader::new(input);
    let (_, events) = asciicast::open(reader)?;
    let output = asciicast::output(events);
    let mut stdout = io::stdout();
    let epoch = Instant::now();

    for o in output {
        let (time, data) = o?;
        let diff = time as i64 - epoch.elapsed().as_micros() as i64;

        if diff > 0 {
            stdout.flush().unwrap();
            thread::sleep(Duration::from_micros(diff as u64));
        }

        stdout.write(data.as_bytes())?;
    }

    Ok(())
}
