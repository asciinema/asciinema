use crate::config::Config;
use crate::format::{asciicast, raw};
use crate::locale;
use crate::notifier;
use crate::pty;
use crate::recorder::{self, KeyBindings};
use crate::tty;
use anyhow::Result;
use clap::Args;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Args)]
pub struct Cli {
    filename: String,

    /// Enable input recording
    #[arg(long, alias = "stdin")]
    input: bool,

    /// Append to an existing asciicast file
    #[arg(long)]
    append: bool,

    /// Save raw output only
    #[arg(long)]
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

    /// Override terminal width (columns) for the recorded command
    #[arg(long)]
    cols: Option<u16>,

    /// Override terminal height (rows) for the recorded command
    #[arg(long)]
    rows: Option<u16>,

    /// Quiet mode - suppress all notices/warnings
    #[arg(short, long)]
    quiet: bool,
}

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

        let writer: Box<dyn recorder::EventWriter + Send> = if self.raw {
            Box::new(raw::Writer::new(file))
        } else {
            let time_offset = if append {
                asciicast::get_duration(&self.filename)?
            } else {
                0
            };

            Box::new(asciicast::Writer::new(file, time_offset))
        };

        let metadata = recorder::Metadata {
            idle_time_limit: self.idle_time_limit,
            command: self.command.clone(),
            title: self.title,
            env: capture_env(&self.env),
        };

        let keys = get_key_bindings(config)?;
        let notifier = get_notifier(config);

        let mut recorder =
            recorder::Recorder::new(writer, append, self.input, metadata, keys, notifier);

        let exec_command = build_exec_command(self.command);
        let exec_extra_env = build_exec_extra_env();

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
                (self.cols, self.rows),
                &mut recorder,
            )?;
        }

        println!("asciinema: recording finished");
        println!("asciinema: asciicast saved to {}", self.filename);

        Ok(())
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
