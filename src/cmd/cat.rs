use crate::format::asciicast;
use anyhow::Result;
use clap::Args;
use std::{fs, io};

#[derive(Debug, Args)]
pub struct Cli {
    #[arg(required = true)]
    filename: Vec<String>,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let mut writer = asciicast::Writer::new(io::stdout(), false, 0);
        let mut time_offset: u64 = 0;
        let mut first = true;

        for path in self.filename.iter() {
            let reader = io::BufReader::new(fs::File::open(path)?);
            let recording = asciicast::open(reader)?;
            let mut time = time_offset;

            if first {
                writer.write_header(&recording.header)?;
                first = false;
            }

            for event in recording.events {
                let mut event = event?;
                time = time_offset + event.time;
                event.time = time;
                writer.write_event(event)?;
            }

            time_offset = time;
        }

        Ok(())
    }
}
