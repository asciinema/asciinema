use std::fs;
use std::path::Path;

use anyhow::{bail, Result};

use crate::asciicast;
use crate::cli::{self, Format};
use crate::config::Config;
use crate::encoder::{
    self, AsciicastV2Encoder, AsciicastV3Encoder, EncoderExt, RawEncoder, TextEncoder,
};
use crate::util;

impl cli::Convert {
    pub fn run(self, _config: &Config) -> Result<()> {
        let path = util::get_local_path(&self.input_filename)?;
        let cast = asciicast::open_from_path(&*path)?;
        let mut encoder = self.get_encoder();
        let mut file = self.open_file()?;

        encoder.encode_to_file(cast, &mut file)
    }

    fn get_encoder(&self) -> Box<dyn encoder::Encoder> {
        let format = self.format.unwrap_or_else(|| {
            if self.output_filename.to_lowercase().ends_with(".txt") {
                Format::Txt
            } else {
                Format::AsciicastV3
            }
        });

        match format {
            Format::AsciicastV3 => Box::new(AsciicastV3Encoder::new(false)),
            Format::AsciicastV2 => Box::new(AsciicastV2Encoder::new(false, 0)),
            Format::Raw => Box::new(RawEncoder::new(false)),
            Format::Txt => Box::new(TextEncoder::new()),
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
