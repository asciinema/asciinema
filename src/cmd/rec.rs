use crate::config::Config;
use crate::format::{asciicast, raw};
use crate::locale;
use crate::notifier;
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

    /// Append to an existing asciicast file
    #[arg(short, long)]
    append: bool,

    /// Recording file format
    #[arg(short, long, value_enum, default_value_t = Format::Asciicast)]
    format: Format,

    #[arg(long, hide = true)]
    raw: bool,

    /// Overwrite target file if it already exists
    #[arg(long, conflicts_with = "append")]
    overwrite: bool,

    /// Command to record [default: $SHELL]
    #[arg(short, long)]
    command: Option<String>,

    /// List of env vars to save
    #[arg(short, long, default_value_t = String::from("SHELL,TERM"))]
    env: String,

    /// Title of the recording
    #[arg(short, long)]
    title: Option<String>,

    /// Limit idle time to a given number of seconds
    #[arg(short, long, value_name = "SECS")]
    idle_time_limit: Option<f64>,

    /// Override terminal size for the recorded command
    #[arg(long, short = 's', value_parser = parse_tty_size, value_name = "COLSxROWS")]
    tty_size: Option<TtySize>,

    #[arg(long, hide = true)]
    cols: Option<u16>,

    #[arg(long, hide = true)]
    rows: Option<u16>,

    /// Quiet mode - suppress all notices/warnings
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Format {
    Asciicast,
    Raw,
}

#[derive(Clone, Debug)]
struct TtySize((Option<u16>, Option<u16>));

impl Cli {
    pub fn run(self, config: &Config) -> Result<()> {
        locale::check_utf8_locale()?;

        let mut overwrite = self.overwrite;
        let mut append = self.append;

        let path = Path::new(&self.filename);

        if path.exists() {
            let metadata = fs::metadata(path)?;

            if metadata.len() == 0 {
                overwrite = true;
                append = false;
            }
            // TODO if !append && !overwrite - error message
        } else {
            append = false;
        }

        let file = fs::OpenOptions::new()
            .write(true)
            .append(append)
            .create(overwrite)
            .create_new(!overwrite && !append)
            .truncate(overwrite)
            .open(&self.filename)?;

        let format = if self.raw { Format::Raw } else { self.format };

        let writer: Box<dyn recorder::EventWriter + Send> = match format {
            Format::Asciicast => {
                let time_offset = if append {
                    asciicast::get_duration(&self.filename)?
                } else {
                    0
                };

                Box::new(asciicast::Writer::new(file, time_offset))
            }

            Format::Raw => Box::new(raw::Writer::new(file)),
        };

        let command = self.get_command(config);
        let metadata = self.build_metadata(command.as_ref().cloned());
        let keys = get_key_bindings(config)?;
        let notifier = get_notifier(config);

        let mut recorder =
            recorder::Recorder::new(writer, append, self.input, metadata, keys, notifier);

        let exec_command = build_exec_command(command);
        let exec_extra_env = build_exec_extra_env();
        let tty_size = self.get_tty_size();

        println!("asciinema: recording asciicast to {}", self.filename);
        println!("asciinema: press <ctrl+d> or type \"exit\" when you're done");

        {
            let mut tty: Box<dyn tty::Tty> = if let Ok(dev_tty) = tty::DevTty::open() {
                Box::new(dev_tty)
            } else {
                println!("asciinema: TTY not available, recording in headless mode");
                Box::new(tty::NullTty::open()?)
            };

            pty::exec(
                &exec_command,
                &exec_extra_env,
                &mut *tty,
                tty_size,
                &mut recorder,
            )?;
        }

        println!("asciinema: recording finished");
        println!("asciinema: asciicast saved to {}", self.filename);

        Ok(())
    }

    fn get_command(&self, config: &Config) -> Option<String> {
        self.command.as_ref().cloned().or(config.cmd_rec_command())
    }

    fn build_metadata(&self, command: Option<String>) -> recorder::Metadata {
        recorder::Metadata {
            idle_time_limit: self.idle_time_limit,
            command,
            title: self.title.clone(),
            env: capture_env(&self.env),
        }
    }

    fn get_tty_size(&self) -> (Option<u16>, Option<u16>) {
        self.tty_size
            .as_ref()
            .map(|s| s.0)
            .unwrap_or((self.cols, self.rows))
    }
}

fn parse_tty_size(s: &str) -> Result<TtySize> {
    match s.split_once('x') {
        Some((cols, "")) => {
            let cols: u16 = cols.parse()?;

            Ok(TtySize((Some(cols), None)))
        }

        Some(("", rows)) => {
            let rows: u16 = rows.parse()?;

            Ok(TtySize((None, Some(rows))))
        }

        Some((cols, rows)) => {
            let cols: u16 = cols.parse()?;
            let rows: u16 = rows.parse()?;

            Ok(TtySize((Some(cols), Some(rows))))
        }

        None => {
            bail!("{s}")
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

fn get_notifier(config: &Config) -> Box<dyn notifier::Notifier> {
    if config.notifications.enabled {
        notifier::get_notifier(config.notifications.command.clone())
    } else {
        Box::new(notifier::NullNotifier)
    }
}

fn capture_env(vars: &str) -> HashMap<String, String> {
    let vars = vars.split(',').collect::<HashSet<_>>();

    env::vars()
        .filter(|(k, _v)| vars.contains(&k.as_str()))
        .collect::<HashMap<_, _>>()
}

fn build_exec_command(command: Option<String>) -> Vec<String> {
    let command = command
        .or(env::var("SHELL").ok())
        .unwrap_or("/bin/sh".to_owned());

    vec!["/bin/sh".to_owned(), "-c".to_owned(), command]
}

fn build_exec_extra_env() -> HashMap<String, String> {
    let mut env = HashMap::new();

    env.insert("ASCIINEMA_REC".to_owned(), "1".to_owned());

    env
}
