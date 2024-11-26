use super::Command;
use crate::asciicast;
use crate::asciicast::Header;
use crate::cli;
use crate::config::Config;
use crate::encoder::{AsciicastEncoder, Encoder, RawEncoder, TextEncoder};
use crate::locale;
use crate::logger;
use crate::notifier;
use crate::pty;
use crate::recorder::Output;
use crate::recorder::{self, KeyBindings};
use crate::tty::{self, FixedSizeTty};
use anyhow::{bail, Result};
use cli::Format;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

impl Command for cli::Record {
    fn run(mut self, config: &Config) -> Result<()> {
        locale::check_utf8_locale()?;

        self.ensure_filename(config)?;
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
        let output = self.get_output(file, format, append, time_offset, config);

        logger::info!("Recording session started, writing to {}", self.path);

        if command.is_none() {
            logger::info!("Press <ctrl+d> or type 'exit' to end");
        }

        let notifier = notifier::threaded(notifier);

        {
            let mut tty = self.get_tty()?;
            let mut recorder = recorder::Recorder::new(output, record_input, keys, notifier);
            pty::exec(&exec_command, &exec_extra_env, &mut tty, &mut recorder)?;
        }

        logger::info!("Recording session ended");

        Ok(())
    }
}

impl cli::Record {
    fn ensure_filename(&mut self, config: &Config) -> Result<()> {
        let mut path = PathBuf::from(&self.path);

        if path.exists() && fs::metadata(&path)?.is_dir() {
            let mut tpl = self.filename.clone().unwrap_or(config.cmd_rec_filename());

            if tpl.contains("{pid}") {
                let pid = process::id().to_string();
                tpl = tpl.replace("{pid}", &pid);
            }

            if tpl.contains("{user}") {
                let user = env::var("USER").ok().unwrap_or("unknown".to_owned());
                tpl = tpl.replace("{user}", &user);
            }

            if tpl.contains("{hostname}") {
                let hostname = hostname::get()
                    .ok()
                    .and_then(|h| h.into_string().ok())
                    .unwrap_or("unknown".to_owned());

                tpl = tpl.replace("{hostname}", &hostname);
            }

            let filename = chrono::Local::now().format(&tpl).to_string();
            path.push(Path::new(&filename));

            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }

            self.path = path.to_string_lossy().to_string();
        }

        Ok(())
    }

    fn get_mode(&self) -> Result<(bool, bool)> {
        let mut overwrite = self.overwrite;
        let mut append = self.append;
        let path = Path::new(&self.path);

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
            .open(&self.path)?;

        Ok(file)
    }

    fn get_format(&self) -> Format {
        self.format.unwrap_or_else(|| {
            if self.raw {
                Format::Raw
            } else if self.path.to_lowercase().ends_with(".txt") {
                Format::Txt
            } else {
                Format::Asciicast
            }
        })
    }

    fn get_time_offset(&self, append: bool, format: Format) -> Result<u64> {
        if append && format == Format::Asciicast {
            asciicast::get_duration(&self.path)
        } else {
            Ok(0)
        }
    }

    fn get_tty(&self) -> Result<FixedSizeTty> {
        let (cols, rows) = self.tty_size.unwrap_or((None, None));
        let cols = cols.or(self.cols);
        let rows = rows.or(self.rows);

        if self.headless {
            Ok(FixedSizeTty::new(tty::NullTty::open()?, cols, rows))
        } else if let Ok(dev_tty) = tty::DevTty::open() {
            Ok(FixedSizeTty::new(dev_tty, cols, rows))
        } else {
            logger::info!("TTY not available, recording in headless mode");
            Ok(FixedSizeTty::new(tty::NullTty::open()?, cols, rows))
        }
    }

    fn get_output(
        &self,
        file: fs::File,
        format: Format,
        append: bool,
        time_offset: u64,
        config: &Config,
    ) -> Box<dyn recorder::Output + Send> {
        let metadata = self.build_asciicast_metadata(config);

        match format {
            Format::Asciicast => {
                let writer = io::LineWriter::new(file);
                let encoder = AsciicastEncoder::new(append, time_offset);

                Box::new(FileOutput {
                    writer,
                    encoder,
                    metadata,
                })
            }

            Format::Raw => Box::new(FileOutput {
                writer: file,
                encoder: RawEncoder::new(append),
                metadata,
            }),

            Format::Txt => Box::new(FileOutput {
                writer: file,
                encoder: TextEncoder::new(),
                metadata,
            }),
        }
    }

    fn get_command(&self, config: &Config) -> Option<String> {
        self.command.as_ref().cloned().or(config.cmd_rec_command())
    }

    fn build_asciicast_metadata(&self, config: &Config) -> Metadata {
        let idle_time_limit = self.idle_time_limit.or(config.cmd_rec_idle_time_limit());
        let command = self.get_command(config);

        let env = self
            .env
            .as_ref()
            .cloned()
            .or(config.cmd_rec_env())
            .unwrap_or(String::from("TERM,SHELL"));

        Metadata {
            idle_time_limit,
            command,
            title: self.title.clone(),
            env: Some(capture_env(&env)),
        }
    }
}

struct FileOutput<W: Write, E: Encoder> {
    writer: W,
    encoder: E,
    metadata: Metadata,
}

pub struct Metadata {
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

impl<W: Write, E: Encoder> Output for FileOutput<W, E> {
    fn header(
        &mut self,
        time: SystemTime,
        tty_size: tty::TtySize,
        theme: Option<tty::Theme>,
    ) -> io::Result<()> {
        let timestamp = time.duration_since(UNIX_EPOCH).unwrap().as_secs();

        let header = Header {
            cols: tty_size.0,
            rows: tty_size.1,
            timestamp: Some(timestamp),
            theme,
            idle_time_limit: self.metadata.idle_time_limit,
            command: self.metadata.command.as_ref().cloned(),
            title: self.metadata.title.as_ref().cloned(),
            env: self.metadata.env.as_ref().cloned(),
        };

        self.writer.write_all(&self.encoder.header(&header))
    }

    fn event(&mut self, event: asciicast::Event) -> io::Result<()> {
        self.writer.write_all(&self.encoder.event(event))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.write_all(&self.encoder.flush())
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
