use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Result};

use crate::asciicast;
use crate::cli::{self, Format};
use crate::encoder::{
    self, AsciicastV2Encoder, AsciicastV3Encoder, EncoderExt, RawEncoder, TextEncoder,
};
use crate::util;

impl cli::Convert {
    pub fn run(self) -> Result<()> {
        let input_path = self.get_input_path()?;
        let output_path = self.get_output_path();
        let cast = asciicast::open_from_path(&*input_path)?;
        let mut encoder = self.get_encoder();
        let mut output_file = self.open_output_file(output_path)?;

        encoder.encode_to_file(cast, &mut output_file)
    }

    fn get_encoder(&self) -> Box<dyn encoder::Encoder> {
        let format = self.output_format.unwrap_or_else(|| {
            if self.output.to_lowercase().ends_with(".txt") {
                Format::Txt
            } else {
                Format::AsciicastV3
            }
        });

        match format {
            Format::AsciicastV3 => Box::new(AsciicastV3Encoder::new(false)),
            Format::AsciicastV2 => {
                Box::new(AsciicastV2Encoder::new(false, Duration::from_micros(0)))
            }
            Format::Raw => Box::new(RawEncoder::new()),
            // pass the CLI flag into the TextEncoder
            Format::Txt => Box::new(TextEncoder::new(self.timestamp)),
        }
    }

    fn get_input_path(&self) -> Result<Box<dyn AsRef<Path>>> {
        if self.input == "-" {
            Ok(Box::new(Path::new("/dev/stdin")))
        } else {
            util::get_local_path(&self.input)
        }
    }

    fn get_output_path(&self) -> String {
        if self.output == "-" {
            "/dev/stdout".to_owned()
        } else {
            self.output.clone()
        }
    }

    fn open_output_file(&self, path: String) -> Result<fs::File> {
        let overwrite = self.get_mode(&path)?;

        let file = fs::OpenOptions::new()
            .write(true)
            .create(overwrite)
            .create_new(!overwrite)
            .truncate(overwrite)
            .open(&path)?;

        Ok(file)
    }

    fn get_mode(&self, path: &str) -> Result<bool> {
        let mut overwrite = self.overwrite;
        let path = Path::new(path);

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
