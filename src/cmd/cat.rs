use std::io;
use std::io::Write;

use anyhow::{anyhow, Result};

use crate::asciicast;
use crate::cli;
use crate::config::Config;

impl cli::Cat {
    pub fn run(self, _config: &Config) -> Result<()> {
        let mut stdout = io::stdout();
        let mut time_offset: u64 = 0;
        let mut first = true;

        let casts = self
            .filename
            .iter()
            .map(asciicast::open_from_path)
            .collect::<Result<Vec<_>>>()?;

        let version = casts[0].version;

        let mut encoder = asciicast::encoder(version)
            .ok_or(anyhow!("asciicast v{version} files can't be concatenated"))?;

        for path in self.filename.iter() {
            let recording = asciicast::open_from_path(path)?;
            let mut time = time_offset;

            if first {
                stdout.write_all(&encoder.header(&recording.header))?;
                first = false;
            }

            for event in recording.events {
                let mut event = event?;
                time = time_offset + event.time;
                event.time = time;
                stdout.write_all(&encoder.event(&event))?;
            }

            time_offset = time;
        }

        Ok(())
    }
}
