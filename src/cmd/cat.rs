use std::io;
use std::io::Write;
use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::asciicast::{self, Asciicast, Encoder, Event, EventData, Version};
use crate::cli;
use crate::util;

impl cli::Cat {
    pub fn run(self) -> Result<()> {
        let mut stdout = io::stdout();
        let casts = self.open_input_files()?;
        let mut encoder = self.get_encoder(casts[0].version)?;
        let mut time_offset = Duration::from_micros(0);
        let mut first = true;
        let mut cols = 0;
        let mut rows = 0;

        for cast in casts.into_iter() {
            let mut time = time_offset;

            if first {
                first = false;
                stdout.write_all(&encoder.header(&cast.header))?;
            } else if cast.header.term_cols != cols || cast.header.term_rows != rows {
                let event = Event::resize(time, (cast.header.term_cols, cast.header.term_rows));
                stdout.write_all(&encoder.event(&event))?;
            }

            cols = cast.header.term_cols;
            rows = cast.header.term_rows;

            for event in cast.events {
                let mut event = event?;
                time = time_offset + event.time;
                event.time = time;
                stdout.write_all(&encoder.event(&event))?;

                if let EventData::Resize(cols_, rows_) = event.data {
                    cols = cols_;
                    rows = rows_;
                }
            }

            time_offset = time;
        }

        Ok(())
    }

    fn open_input_files(&self) -> Result<Vec<Asciicast<'_>>> {
        self.file
            .iter()
            .map(|filename| {
                let path = util::get_local_path(filename)?;
                asciicast::open_from_path(&*path)
            })
            .collect()
    }

    fn get_encoder(&self, version: Version) -> Result<Box<dyn Encoder>> {
        asciicast::encoder(version)
            .ok_or(anyhow!("asciicast v{version} files can't be concatenated"))
    }
}
