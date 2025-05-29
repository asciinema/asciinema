use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::LineWriter;
use std::net::TcpListener;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tokio::runtime::Runtime;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use url::{form_urlencoded, Url};

use crate::api;
use crate::asciicast::{self, Version};
use crate::cli::{self, Format, RelayTarget};
use crate::config::{self, Config};
use crate::encoder::{AsciicastV2Encoder, AsciicastV3Encoder, RawEncoder, TextEncoder};
use crate::file_writer::{FileWriterStarter, Metadata};
use crate::forwarder;
use crate::hash;
use crate::locale;
use crate::notifier::{self, Notifier, NullNotifier};
use crate::pty;
use crate::server;
use crate::session::{self, KeyBindings, SessionStarter};
use crate::status;
use crate::stream::Stream;
use crate::tty::{DevTty, FixedSizeTty, NullTty, Tty};

impl cli::Session {
    pub fn run(mut self) -> Result<ExitCode> {
        locale::check_utf8_locale()?;

        let config = Config::new(self.server_url.clone())?;
        let runtime = Runtime::new()?;
        let command = self.get_command(&config.recording);
        let keys = get_key_bindings(&config.recording)?;
        let notifier = notifier::threaded(get_notifier(&config));
        let record_input = self.rec_input || config.recording.rec_input;
        let term_type = self.get_term_type();
        let term_version = self.get_term_version()?;
        let env = capture_env(self.rec_env.take(), &config.recording);

        let file_writer = self
            .output_file
            .as_ref()
            .map(|path| {
                self.get_file_writer(
                    path,
                    &config.recording,
                    term_type.clone(),
                    term_version.clone(),
                    &env,
                    notifier.clone(),
                )
            })
            .transpose()?;

        let mut listener = self
            .stream_local
            .take()
            .map(TcpListener::bind)
            .transpose()
            .context("cannot start listener")?;

        let mut relay = self
            .stream_remote
            .take()
            .map(|target| {
                get_relay(
                    target,
                    &config,
                    term_type,
                    term_version,
                    self.title.take(),
                    &env,
                )
            })
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

        status::info!("asciinema session started");

        if let Some(path) = self.output_file.as_ref() {
            status::info!("Recording to {}", path);
        }

        if let Some(listener) = &listener {
            status::info!(
                "Live streaming at http://{}",
                listener.local_addr().unwrap()
            );
        }

        if let Some(Relay { url: Some(url), .. }) = &relay {
            status::info!("Live streaming at {}", url);
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

        let mut outputs: Vec<Box<dyn session::OutputStarter>> = Vec::new();

        if server.is_some() || forwarder.is_some() {
            let output = stream.start(runtime.handle().clone());
            outputs.push(Box::new(output));
        }

        if let Some(output) = file_writer {
            outputs.push(Box::new(output));
        }

        if outputs.is_empty() {
            status::warning!("No outputs enabled, consider using -o, -l, or -r");
        }

        if command.is_none() {
            status::info!("Press <ctrl+d> or type 'exit' to end");
        }

        let exec_command = build_exec_command(command.as_ref().cloned());
        let exec_extra_env = build_exec_extra_env(relay_id.as_ref());

        let (exit_status, _) = {
            let starter = SessionStarter::new(outputs, record_input, keys, notifier);
            let mut tty = self.get_tty(true)?;
            pty::exec(&exec_command, &exec_extra_env, &mut tty, starter)?
        };

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

        status::info!("asciinema session ended");

        if !self.return_ || exit_status == 0 {
            Ok(ExitCode::from(0))
        } else if exit_status > 0 {
            Ok(ExitCode::from(exit_status as u8))
        } else {
            Ok(ExitCode::from(1))
        }
    }

    fn get_file_writer<N: Notifier + 'static>(
        &self,
        path: &str,
        config: &config::Recording,
        term_type: Option<String>,
        term_version: Option<String>,
        env: &HashMap<String, String>,
        notifier: N,
    ) -> Result<FileWriterStarter> {
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

        let format = self.output_format.map(Ok).unwrap_or_else(|| {
            if path.extension().is_some_and(|ext| ext == "txt") {
                Ok(Format::Txt)
            } else if append {
                match asciicast::open_from_path(path) {
                    Ok(cast) => match cast.version {
                        Version::One => bail!("appending to asciicast v1 files is not supported"),
                        Version::Two => Ok(Format::AsciicastV2),
                        Version::Three => Ok(Format::AsciicastV3),
                    },

                    Err(e) => bail!("can't append: {e}"),
                }
            } else {
                Ok(Format::AsciicastV3)
            }
        })?;

        let file = OpenOptions::new()
            .write(true)
            .append(append)
            .create(overwrite)
            .create_new(!overwrite && !append)
            .truncate(overwrite)
            .open(path)?;

        let metadata = self.build_asciicast_metadata(term_type, term_version, env, config);
        let notifier = Box::new(notifier);

        let writer = match format {
            Format::AsciicastV3 => {
                let writer = Box::new(LineWriter::new(file));
                let encoder = Box::new(AsciicastV3Encoder::new(append));

                FileWriterStarter {
                    writer,
                    encoder,
                    metadata,
                    notifier,
                }
            }

            Format::AsciicastV2 => {
                let time_offset = if append {
                    asciicast::get_duration(path)?
                } else {
                    0
                };

                let writer = Box::new(LineWriter::new(file));
                let encoder = Box::new(AsciicastV2Encoder::new(append, time_offset));

                FileWriterStarter {
                    writer,
                    encoder,
                    metadata,
                    notifier,
                }
            }

            Format::Raw => {
                let writer = Box::new(file);
                let encoder = Box::new(RawEncoder::new(append));

                FileWriterStarter {
                    writer,
                    encoder,
                    metadata,
                    notifier,
                }
            }

            Format::Txt => {
                let writer = Box::new(file);
                let encoder = Box::new(TextEncoder::new());

                FileWriterStarter {
                    writer,
                    encoder,
                    metadata,
                    notifier,
                }
            }
        };

        Ok(writer)
    }

    fn get_term_type(&self) -> Option<String> {
        env::var("TERM").ok()
    }

    fn get_term_version(&self) -> Result<Option<String>> {
        self.get_tty(false).map(|tty| tty.get_version())
    }

    fn get_command(&self, config: &config::Recording) -> Option<String> {
        self.command.as_ref().cloned().or(config.command.clone())
    }

    fn build_asciicast_metadata(
        &self,
        term_type: Option<String>,
        term_version: Option<String>,
        env: &HashMap<String, String>,
        config: &config::Recording,
    ) -> Metadata {
        let idle_time_limit = self.idle_time_limit.or(config.idle_time_limit);
        let command = self.get_command(config);

        Metadata {
            term_type,
            term_version,
            idle_time_limit,
            command,
            title: self.title.clone(),
            env: Some(env.clone()),
        }
    }

    fn get_tty(&self, quiet: bool) -> Result<impl Tty> {
        let (cols, rows) = self.window_size.unwrap_or((None, None));

        if self.headless {
            Ok(FixedSizeTty::new(NullTty::open()?, cols, rows))
        } else if let Ok(dev_tty) = DevTty::open() {
            Ok(FixedSizeTty::new(dev_tty, cols, rows))
        } else {
            if !quiet {
                status::info!("TTY not available, recording in headless mode");
            }

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
        format!("{:x}", hash::fnv1a_128(self.ws_producer_url.as_ref()))
    }
}

fn get_relay(
    target: RelayTarget,
    config: &Config,
    term_type: Option<String>,
    term_version: Option<String>,
    title: Option<String>,
    env: &HashMap<String, String>,
) -> Result<Relay> {
    match target {
        RelayTarget::StreamId(id) => {
            let stream = api::create_user_stream(id, config)?;
            let ws_producer_url =
                build_producer_url(&stream.ws_producer_url, term_type, term_version, title, env)?;

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

fn build_producer_url(
    url: &str,
    term_type: Option<String>,
    term_version: Option<String>,
    title: Option<String>,
    env: &HashMap<String, String>,
) -> Result<Url> {
    let mut url: Url = url.parse()?;
    let mut params = Vec::new();

    if let Some(type_) = term_type {
        params.push(("term[type]".to_string(), type_));
    }

    if let Some(version) = term_version {
        params.push(("term[version]".to_string(), version));
    }

    if let Ok(shell) = env::var("SHELL") {
        params.push(("shell".to_string(), shell));
    }

    if let Some(title) = title {
        params.push(("title".to_string(), title));
    }

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

fn get_key_bindings(config: &config::Recording) -> Result<KeyBindings> {
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

fn capture_env(var_names: Option<String>, config: &config::Recording) -> HashMap<String, String> {
    let var_names = var_names
        .or(config.rec_env.clone())
        .unwrap_or(String::from("SHELL"));

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
