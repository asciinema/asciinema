use super::Command;
use crate::asciicast::{self, Header};
use crate::cli::{self, Format};
use crate::config::Config;
use crate::encoder::{self, EncoderExt};
use crate::util;
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

impl Command for cli::Convert {
    fn run(self, _config: &Config) -> Result<()> {
        let path = util::get_local_path(&self.input_filename)?;
        let input = asciicast::open_from_path(&*path)?;
        let mut output = self.get_output(&input.header)?;

        output.encode(input)
    }
}

impl cli::Convert {
    fn get_output(&self, header: &Header) -> Result<Box<dyn encoder::Encoder>> {
        let file = self.open_file()?;

        let format = self.format.unwrap_or_else(|| {
            if self.output_filename.to_lowercase().ends_with(".txt") {
                Format::Txt
            } else {
                Format::Asciicast
            }
        });

        match format {
            Format::Asciicast => Ok(Box::new(encoder::AsciicastEncoder::new(
                file,
                false,
                0,
                header.into(),
            ))),

            Format::Raw => Ok(Box::new(encoder::RawEncoder::new(file, false))),
            Format::Txt => Ok(Box::new(encoder::TextEncoder::new(file))),
        }
    }

    fn open_file(&self) -> Result<fs::File> {
        let overwrite = self.get_mode()?;

        let file = fs::OpenOptions::new()
            .write(true)
            .create(overwrite)
            .create_new(!overwrite)
            .truncate(overwrite)
            .open(&self.output_filename)?;

        Ok(file)
    }

    fn get_mode(&self) -> Result<bool> {
        let mut overwrite = self.overwrite;
        let path = Path::new(&self.output_filename);

        if path.exists() {
            let metadata = fs::metadata(path)?;

            if metadata.len() == 0 {
                overwrite = true;
            }

            if !overwrite {
                bail!("file exists, use --overwrite option to overwrite the file");
            }
        }

        Ok(overwrite)
    }
}
