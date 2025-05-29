use std::io;
use std::io::Write;

use anyhow::{anyhow, Result};

use crate::asciicast::{self, Asciicast, Encoder, Version};
use crate::cli;

impl cli::Cat {
    pub fn run(self) -> Result<()> {
        let mut stdout = io::stdout();
        let casts = self.open_input_files()?;
        let mut encoder = self.get_encoder(casts[0].version)?;
        let mut time_offset: u64 = 0;
        let mut first = true;

        for cast in casts.into_iter() {
            let mut time = time_offset;

            if first {
                stdout.write_all(&encoder.header(&cast.header))?;
                first = false;
            }

            for event in cast.events {
                let mut event = event?;
                time = time_offset + event.time;
                event.time = time;
                stdout.write_all(&encoder.event(&event))?;
            }

            time_offset = time;
        }

        Ok(())
    }

    fn open_input_files(&self) -> Result<Vec<Asciicast>> {
        self.filename
            .iter()
            .map(asciicast::open_from_path)
            .collect()
    }

    fn get_encoder(&self, version: Version) -> Result<Box<dyn Encoder>> {
        asciicast::encoder(version)
            .ok_or(anyhow!("asciicast v{version} files can't be concatenated"))
    }
}
