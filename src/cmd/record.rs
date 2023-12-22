use crate::format;
use crate::format::{asciicast, raw};
use crate::locale;
use crate::pty;
use crate::recorder;
use anyhow::Result;
use clap::Args;
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::{CString, OsString};
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

#[derive(Debug, Args)]
pub struct Cli {
    filename: String,

    /// Enable input recording
    #[arg(long)]
    stdin: bool,

    /// Append to existing asciicast file
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

    /// Limit idle time to given number of seconds
    #[arg(short, long, value_name = "SECS")]
    idle_time_limit: Option<f32>,

    /// Override terminal width (columns) for recorded command
    #[arg(long)]
    cols: Option<u16>,

    /// Override terminal height (rows) for recorded command
    #[arg(long)]
    rows: Option<u16>,

    /// Quiet mode - suppress all notices/warnings
    #[arg(short, long)]
    quiet: bool,
}

impl Cli {
    pub fn run(self) -> Result<()> {
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

        let writer: Box<dyn format::Writer + Send> = if self.raw {
            Box::new(raw::Writer::new(file))
        } else {
            let time_offset = if append {
                asciicast::get_duration(&self.filename)?
            } else {
                0.0
            };

            Box::new(asciicast::Writer::new(file, time_offset))
        };

        let mut recorder = recorder::Recorder::new(
            writer,
            append,
            self.stdin,
            self.idle_time_limit,
            self.command.clone(),
            self.title,
            capture_env(&self.env),
        );

        let exec_args = build_exec_args(self.command);
        let exec_env = build_exec_env();

        println!("asciinema: recording asciicast to {}", self.filename);
        println!("asciinema: press <ctrl+d> or type \"exit\" when you're done");

        pty::exec(&exec_args, &exec_env, (self.cols, self.rows), &mut recorder)?;

        println!("asciinema: recording finished");
        println!("asciinema: asciicast saved to {}", self.filename);

        Ok(())
    }
}

fn capture_env(vars: &str) -> HashMap<String, String> {
    let vars = vars.split(',').collect::<HashSet<_>>();

    env::vars()
        .filter(|(k, _v)| vars.contains(&k.as_str()))
        .collect::<HashMap<_, _>>()
}

fn build_exec_args(command: Option<String>) -> Vec<String> {
    let command = command
        .or(env::var("SHELL").ok())
        .unwrap_or("/bin/sh".to_owned());

    vec!["/bin/sh".to_owned(), "-c".to_owned(), command]
}

fn build_exec_env() -> Vec<CString> {
    env::vars_os()
        .map(format_env_var)
        .chain(std::iter::once(CString::new("ASCIINEMA_REC=1").unwrap()))
        .collect()
}

fn format_env_var((key, value): (OsString, OsString)) -> CString {
    let mut key_value = key.into_vec();
    key_value.push(b'=');
    key_value.extend(value.into_vec());

    CString::new(key_value).unwrap()
}
