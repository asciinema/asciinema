use crate::format;
use crate::format::{asciicast, raw};
use crate::locale;
use crate::pty;
use crate::recorder;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::{CString, OsString};
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

pub fn run(
    filename: String,
    stdin: bool,
    mut append: bool,
    raw: bool,
    mut overwrite: bool,
    command: Option<String>,
    env: String,
    title: Option<String>,
    idle_time_limit: Option<f32>,
    cols: Option<u16>,
    rows: Option<u16>,
    quiet: bool,
) -> Result<()> {
    locale::check_utf8_locale()?;

    let path = Path::new(&filename);

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
        .open(&filename)?;

    let writer: Box<dyn format::Writer + Send> = if raw {
        Box::new(raw::Writer::new(file))
    } else {
        let time_offset = if append {
            asciicast::get_duration(&filename)?
        } else {
            0.0
        };

        Box::new(asciicast::Writer::new(file, time_offset))
    };

    let mut recorder = recorder::Recorder::new(
        writer,
        append,
        stdin,
        idle_time_limit,
        command.clone(),
        title,
        capture_env(&env),
    );

    let exec_args = build_exec_args(command);
    let exec_env = build_exec_env();

    pty::exec(&exec_args, &exec_env, (cols, rows), &mut recorder)?;

    Ok(())
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
