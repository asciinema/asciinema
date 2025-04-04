use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::LineWriter;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Local;
use tokio::runtime::Runtime;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use url::{form_urlencoded, Url};

use crate::api;
use crate::asciicast;
use crate::cli::{self, Format, RelayTarget};
use crate::config::{self, Config};
use crate::encoder::{AsciicastEncoder, RawEncoder, TextEncoder};
use crate::file_writer::{FileWriter, Metadata};
use crate::forwarder;
use crate::locale;
use crate::logger;
use crate::notifier::{self, Notifier, NullNotifier};
use crate::pty;
use crate::server;
use crate::session::{self, KeyBindings, Session};
use crate::stream::Stream;
use crate::tty::{DevTty, FixedSizeTty, NullTty};
use crate::util;

impl cli::Session {
    pub fn run(mut self, config: &Config, cmd_config: &config::Session) -> Result<()> {
        locale::check_utf8_locale()?;

        let runtime = Runtime::new()?;
        let command = self.get_command(&cmd_config);
        let keys = get_key_bindings(&cmd_config)?;
        let notifier = notifier::threaded(get_notifier(config));
        let record_input = self.input || cmd_config.input;
        let env = capture_env(self.env.clone(), &cmd_config);

        let path = self
            .output
            .take()
            .map(|path| self.ensure_filename(path, &cmd_config))
            .transpose()?;

        let file_writer = path
            .as_ref()
            .map(|path| self.get_file_writer(path, &cmd_config, &env))
            .transpose()?;

        let mut listener = self
            .serve
            .take()
            .map(TcpListener::bind)
            .transpose()
            .context("cannot start listener")?;

        let mut relay = self
            .relay
            .take()
            .map(|target| get_relay(target, config, &env))
            .transpose()?;

        let relay_id = relay.as_ref().map(|r| r.id());
        let parent_session_relay_id = get_parent_session_relay_id();

        if relay_id.is_some()
            && parent_session_relay_id.is_some()
            && relay_id == parent_session_relay_id
        {
            if let Some(Relay { url: Some(url), .. }) = relay {
                bail!("This shell is already being streamed at {url}");
            } else {
                bail!("This shell is already being streamed");
            }
        }

        if listener.is_some() || relay.is_some() {
            self.init_logging()?;
        }

        logger::info!("asciinema session started");

        if let Some(path) = path {
            logger::info!("Recording to {}", path);
        }

        if let Some(listener) = &listener {
            logger::info!(
                "Live streaming at http://{}",
                listener.local_addr().unwrap()
            );
        }

        if let Some(Relay { url: Some(url), .. }) = &relay {
            logger::info!("Live streaming at {}", url);
        }

        if command.is_none() {
            logger::info!("Press <ctrl+d> or type 'exit' to end");
        }

        let stream = Stream::new();
        let shutdown_token = CancellationToken::new();

        let server = listener.take().map(|listener| {
            runtime.spawn(server::serve(
                listener,
                stream.subscriber(),
                shutdown_token.clone(),
            ))
        });

        let forwarder = relay.take().map(|relay| {
            runtime.spawn(forwarder::forward(
                relay.ws_producer_url,
                stream.subscriber(),
                notifier.clone(),
                shutdown_token.clone(),
            ))
        });

        let mut outputs: Vec<Box<dyn session::Output + Send>> = Vec::new();

        if server.is_some() || forwarder.is_some() {
            let output = stream.start(&runtime);
            outputs.push(Box::new(output));
        }

        if let Some(output) = file_writer {
            outputs.push(Box::new(output));
        }

        let exec_command = build_exec_command(command.as_ref().cloned());
        let exec_extra_env = build_exec_extra_env(relay_id.as_ref());

        {
            let mut session = Session::new(outputs, record_input, keys, notifier);
            let mut tty = self.get_tty()?;
            pty::exec(&exec_command, &exec_extra_env, &mut tty, &mut session)?;
        }

        runtime.block_on(async {
            debug!("session shutting down...");
            shutdown_token.cancel();

            if let Some(task) = server {
                debug!("waiting for server shutdown...");
                let _ = time::timeout(Duration::from_secs(5), task).await;
            }

            if let Some(task) = forwarder {
                debug!("waiting for forwarder shutdown...");
                let _ = time::timeout(Duration::from_secs(5), task).await;
            }

            debug!("shutdown complete");
        });

        logger::info!("asciinema session ended");

        Ok(())
    }

    fn ensure_filename(&mut self, path_: String, config: &config::Session) -> Result<String> {
        let mut path = PathBuf::from(&path_);

        if path.exists() && fs::metadata(&path)?.is_dir() {
            let mut tpl = self.filename.clone().unwrap_or(config.filename.clone());

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

            let filename = Local::now().format(&tpl).to_string();
            path.push(Path::new(&filename));

            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }

            Ok(path.to_string_lossy().to_string())
        } else {
            Ok(path_)
        }
    }

    fn get_file_writer(
        &self,
        path: &str,
        config: &config::Session,
        env: &HashMap<String, String>,
    ) -> Result<FileWriter> {
        let format = self.format.unwrap_or_else(|| {
            if path.to_lowercase().ends_with(".txt") {
                Format::Txt
            } else {
                Format::Asciicast
            }
        });

        let mut overwrite = self.overwrite;
        let mut append = self.append;
        let path = Path::new(path);

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

        let file = OpenOptions::new()
            .write(true)
            .append(append)
            .create(overwrite)
            .create_new(!overwrite && !append)
            .truncate(overwrite)
            .open(path)?;

        let time_offset = if append && format == Format::Asciicast {
            asciicast::get_duration(path)?
        } else {
            0
        };

        let metadata = self.build_asciicast_metadata(env, config);

        let writer = match format {
            Format::Asciicast => {
                let writer = Box::new(LineWriter::new(file));
                let encoder = Box::new(AsciicastEncoder::new(append, time_offset));

                FileWriter {
                    writer,
                    encoder,
                    metadata,
                }
            }

            Format::Raw => {
                let writer = Box::new(file);
                let encoder = Box::new(RawEncoder::new(append));

                FileWriter {
                    writer,
                    encoder,
                    metadata,
                }
            }

            Format::Txt => {
                let writer = Box::new(file);
                let encoder = Box::new(TextEncoder::new());

                FileWriter {
                    writer,
                    encoder,
                    metadata,
                }
            }
        };

        Ok(writer)
    }

    fn get_command(&self, config: &config::Session) -> Option<String> {
        self.command.as_ref().cloned().or(config.command.clone())
    }

    fn build_asciicast_metadata(
        &self,
        env: &HashMap<String, String>,
        config: &config::Session,
    ) -> Metadata {
        let idle_time_limit = self.idle_time_limit.or(config.idle_time_limit);
        let command = self.get_command(config);

        Metadata {
            idle_time_limit,
            command,
            title: self.title.clone(),
            env: Some(env.clone()),
        }
    }

    fn get_tty(&self) -> Result<FixedSizeTty> {
        let (cols, rows) = self.tty_size.unwrap_or((None, None));

        if self.headless {
            Ok(FixedSizeTty::new(NullTty::open()?, cols, rows))
        } else if let Ok(dev_tty) = DevTty::open() {
            Ok(FixedSizeTty::new(dev_tty, cols, rows))
        } else {
            logger::info!("TTY not available, recording in headless mode");
            Ok(FixedSizeTty::new(NullTty::open()?, cols, rows))
        }
    }

    fn init_logging(&self) -> Result<()> {
        let log_file = self.log_file.as_ref().cloned();

        if let Some(path) = &log_file {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| anyhow!("cannot open log file {}: {}", path.to_string_lossy(), e))?;

            let filter = EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy();

            tracing_subscriber::fmt()
                .with_ansi(false)
                .with_env_filter(filter)
                .with_writer(file)
                .init();
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Relay {
    ws_producer_url: Url,
    url: Option<Url>,
}

impl Relay {
    fn id(&self) -> String {
        util::sha2_digest(self.ws_producer_url.as_ref())
    }
}

fn get_relay(target: RelayTarget, config: &Config, env: &HashMap<String, String>) -> Result<Relay> {
    match target {
        RelayTarget::StreamId(id) => {
            let stream = api::create_user_stream(id, config)?;
            let ws_producer_url = build_producer_url(&stream.ws_producer_url, env)?;

            Ok(Relay {
                ws_producer_url,
                url: Some(stream.url.parse()?),
            })
        }

        RelayTarget::WsProducerUrl(url) => Ok(Relay {
            ws_producer_url: url,
            url: None,
        }),
    }
}

fn build_producer_url(url: &str, env: &HashMap<String, String>) -> Result<Url> {
    let mut url: Url = url.parse()?;
    let term = env::var("TERM").ok().unwrap_or_default();
    let shell = env::var("SHELL").ok().unwrap_or_default();

    let mut params = vec![
        ("term[type]".to_string(), term.clone()),
        ("shell".to_string(), shell.clone()),
    ];

    for (k, v) in env {
        params.push((format!("env[{k}]"), v.to_string()));
    }

    let params = params.into_iter().filter(|(_k, v)| !v.is_empty());

    let query = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish();

    url.set_query(Some(&query));

    Ok(url)
}

fn get_key_bindings(config: &config::Session) -> Result<KeyBindings> {
    let mut keys = KeyBindings::default();

    if let Some(key) = config.prefix_key()? {
        keys.prefix = key;
    }

    if let Some(key) = config.pause_key()? {
        keys.pause = key;
    }

    if let Some(key) = config.add_marker_key()? {
        keys.add_marker = key;
    }

    Ok(keys)
}

fn capture_env(var_names: Option<String>, config: &config::Session) -> HashMap<String, String> {
    let var_names = var_names
        .or(config.env.clone())
        .unwrap_or(String::from("TERM,SHELL"));

    let vars = var_names.split(',').collect::<HashSet<_>>();

    env::vars()
        .filter(|(k, _v)| vars.contains(&k.as_str()))
        .collect::<HashMap<_, _>>()
}

fn get_notifier(config: &Config) -> Box<dyn Notifier> {
    if config.notifications.enabled {
        notifier::get_notifier(config.notifications.command.clone())
    } else {
        Box::new(NullNotifier)
    }
}

fn build_exec_command(command: Option<String>) -> Vec<String> {
    let command = command
        .or(env::var("SHELL").ok())
        .unwrap_or("/bin/sh".to_owned());

    vec!["/bin/sh".to_owned(), "-c".to_owned(), command]
}

fn build_exec_extra_env(relay_id: Option<&String>) -> HashMap<String, String> {
    let mut env = HashMap::new();

    env.insert("ASCIINEMA_REC".to_owned(), "1".to_owned());

    if let Some(id) = relay_id {
        env.insert("ASCIINEMA_RELAY_ID".to_owned(), id.clone());
    }

    env
}

fn get_parent_session_relay_id() -> Option<String> {
    env::var("ASCIINEMA_RELAY_ID").ok()
}
