use crate::asciicast;
use crate::config::Config;
use crate::encoder;
use crate::locale;
use crate::logger;
use crate::pty;
use crate::recorder::{self, KeyBindings};
use crate::tty;
use anyhow::{bail, Result};
use clap::{Args, ValueEnum};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Args)]
pub struct Cli {
    filename: String,

    /// Enable input recording
    #[arg(long, short = 'I', alias = "stdin")]
    input: bool,

    /// Append to an existing recording file
    #[arg(short, long)]
    append: bool,

    /// Recording file format [default: asciicast]
    #[arg(short, long, value_enum)]
    format: Option<Format>,

    #[arg(long, hide = true)]
    raw: bool,

    /// Overwrite target file if it already exists
    #[arg(long, conflicts_with = "append")]
    overwrite: bool,

    /// Command to record [default: $SHELL]
    #[arg(short, long)]
    command: Option<String>,

    /// List of env vars to save [default: TERM,SHELL]
    #[arg(long)]
    env: Option<String>,

    /// Title of the recording
    #[arg(short, long)]
    title: Option<String>,

    /// Limit idle time to a given number of seconds
    #[arg(short, long, value_name = "SECS")]
    idle_time_limit: Option<f64>,

    /// Override terminal size for the recorded command
    #[arg(long, value_name = "COLSxROWS")]
    tty_size: Option<pty::WinsizeOverride>,

    #[arg(long, hide = true)]
    cols: Option<u16>,

    #[arg(long, hide = true)]
    rows: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
enum Format {
    Asciicast,
    Raw,
    Txt,
}

impl Cli {
    pub fn run(self, config: &Config) -> Result<()> {
        locale::check_utf8_locale()?;

        let format = self.get_format();
        let (append, overwrite) = self.get_mode()?;
        let file = self.open_file(append, overwrite)?;
        let time_offset = self.get_time_offset(append, format)?;
        let command = self.get_command(config);
        let keys = get_key_bindings(config)?;
        let notifier = super::get_notifier(config);
        let record_input = self.input || config.cmd_rec_input();
        let exec_command = super::build_exec_command(command.as_ref().cloned());
        let exec_extra_env = super::build_exec_extra_env(&[]);

        logger::info!("Recording session started, writing to {}", self.filename);

        if command.is_none() {
            logger::info!("Press <ctrl+d> or type 'exit' to end");
        }

        {
            let mut tty: Box<dyn tty::Tty> = if let Ok(dev_tty) = tty::DevTty::open() {
                Box::new(dev_tty)
            } else {
                logger::info!("TTY not available, recording in headless mode");
                Box::new(tty::NullTty::open()?)
            };

            let theme = tty.get_theme();
            let output = self.get_output(file, format, append, time_offset, theme, config);
            let mut recorder = recorder::Recorder::new(output, record_input, keys, notifier);

            pty::exec(
                &exec_command,
                &exec_extra_env,
                &mut *tty,
                self.tty_size,
                &mut recorder,
            )?;
        }

        logger::info!("Recording session ended");

        Ok(())
    }

    fn get_mode(&self) -> Result<(bool, bool)> {
        let mut overwrite = self.overwrite;
        let mut append = self.append;
        let path = Path::new(&self.filename);

        if path.exists() {
            let metadata = fs::metadata(path)?;

            if metadata.len() == 0 {
                overwrite = true;
                append = false;
            }

            if !append && !overwrite {
                bail!("file exists, use --overwrite or --append");
            }
        } else {
            append = false;
        }

        Ok((append, overwrite))
    }

    fn open_file(&self, append: bool, overwrite: bool) -> Result<fs::File> {
        let file = fs::OpenOptions::new()
            .write(true)
            .append(append)
            .create(overwrite)
            .create_new(!overwrite && !append)
            .truncate(overwrite)
            .open(&self.filename)?;

        Ok(file)
    }

    fn get_format(&self) -> Format {
        self.format.unwrap_or_else(|| {
            if self.raw {
                Format::Raw
            } else if self.filename.to_lowercase().ends_with(".txt") {
                Format::Txt
            } else {
                Format::Asciicast
            }
        })
    }

    fn get_time_offset(&self, append: bool, format: Format) -> Result<u64> {
        if append && format == Format::Asciicast {
            asciicast::get_duration(&self.filename)
        } else {
            Ok(0)
        }
    }

    fn get_output(
        &self,
        file: fs::File,
        format: Format,
        append: bool,
        time_offset: u64,
        theme: Option<tty::Theme>,
        config: &Config,
    ) -> Box<dyn recorder::Output + Send> {
        match format {
            Format::Asciicast => {
                let metadata = self.build_asciicast_metadata(theme, config);

                Box::new(encoder::AsciicastEncoder::new(
                    file,
                    append,
                    time_offset,
                    metadata,
                ))
            }

            Format::Raw => Box::new(encoder::RawEncoder::new(file, append)),
            Format::Txt => Box::new(encoder::TextEncoder::new(file)),
        }
    }

    fn get_command(&self, config: &Config) -> Option<String> {
        self.command.as_ref().cloned().or(config.cmd_rec_command())
    }

    fn build_asciicast_metadata(
        &self,
        theme: Option<tty::Theme>,
        config: &Config,
    ) -> encoder::Metadata {
        let idle_time_limit = self.idle_time_limit.or(config.cmd_rec_idle_time_limit());
        let command = self.get_command(config);

        let env = self
            .env
            .as_ref()
            .cloned()
            .or(config.cmd_rec_env())
            .unwrap_or(String::from("TERM,SHELL"));

        encoder::Metadata {
            idle_time_limit,
            command,
            title: self.title.clone(),
            env: Some(capture_env(&env)),
            theme,
        }
    }
}

fn get_key_bindings(config: &Config) -> Result<KeyBindings> {
    let mut keys = KeyBindings::default();

    if let Some(key) = config.cmd_rec_prefix_key()? {
        keys.prefix = key;
    }

    if let Some(key) = config.cmd_rec_pause_key()? {
        keys.pause = key;
    }

    if let Some(key) = config.cmd_rec_add_marker_key()? {
        keys.add_marker = key;
    }

    Ok(keys)
}

fn capture_env(vars: &str) -> HashMap<String, String> {
    let vars = vars.split(',').collect::<HashSet<_>>();

    env::vars()
        .filter(|(k, _v)| vars.contains(&k.as_str()))
        .collect::<HashMap<_, _>>()
}
