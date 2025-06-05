use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::LineWriter;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

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
use crate::file_writer::FileWriter;
use crate::forwarder;
use crate::hash;
use crate::locale;
use crate::notifier::{self, Notifier, NullNotifier};
use crate::pty;
use crate::server;
use crate::session::{self, KeyBindings, Metadata, Session, TermInfo};
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
        let signal_fd = pty::open_signal_fd()?;
        let metadata = self.get_session_metadata(&config.recording)?;
        let file_writer = self.get_file_writer(&metadata, notifier.clone())?;
        let listener = self.get_listener()?;
        let relay = self.get_relay(&metadata, &config)?;
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

        let mut no_outputs = true;

        if let Some(path) = self.output_file.as_ref() {
            status::info!("Recording to {}", path);
            no_outputs = false;
        }

        if let Some(listener) = &listener {
            status::info!(
                "Live streaming at http://{}",
                listener.local_addr().unwrap()
            );

            no_outputs = false;
        }

        if let Some(Relay { url: Some(url), .. }) = &relay {
            status::info!("Live streaming at {}", url);
            no_outputs = false;
        }

        if no_outputs {
            status::warning!("No outputs enabled, consider using -o, -l, or -r");
        }

        if command.is_none() {
            status::info!("Press <ctrl+d> or type 'exit' to end");
        }

        let stream = Stream::new();
        let shutdown_token = CancellationToken::new();
        let mut outputs: Vec<Box<dyn session::Output>> = Vec::new();

        if let Some(writer) = file_writer {
            let output = writer.start()?;
            outputs.push(Box::new(output));
        }

        let server = listener.map(|listener| {
            runtime.spawn(server::serve(
                listener,
                stream.subscriber(),
                shutdown_token.clone(),
            ))
        });

        let forwarder = relay.map(|relay| {
            runtime.spawn(forwarder::forward(
                relay.ws_producer_url,
                stream.subscriber(),
                notifier.clone(),
                shutdown_token.clone(),
            ))
        });

        if server.is_some() || forwarder.is_some() {
            let output = stream.start(runtime.handle().clone(), &metadata);
            outputs.push(Box::new(output));
        }

        let exit_status = {
            let mut tty = self.get_tty(true)?;

            let mut session = Session::new(
                outputs,
                metadata.term.size,
                self.rec_input || config.recording.rec_input,
                keys,
                notifier,
            );

            pty::exec(
                &build_exec_command(command.as_ref().cloned()),
                &build_exec_extra_env(relay_id.as_ref()),
                metadata.term.size,
                &mut tty,
                &mut session,
                signal_fd,
            )?
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

    fn get_command(&self, config: &config::Recording) -> Option<String> {
        self.command.as_ref().cloned().or(config.command.clone())
    }

    fn get_session_metadata(&self, config: &config::Recording) -> Result<Metadata> {
        Ok(Metadata {
            time: SystemTime::now(),
            term: self.get_term_info()?,
            idle_time_limit: self.idle_time_limit.or(config.idle_time_limit),
            command: self.get_command(config),
            title: self.title.clone(),
            env: capture_env(self.rec_env.clone(), config),
        })
    }

    fn get_term_info(&self) -> Result<TermInfo> {
        let tty = self.get_tty(false)?;

        Ok(TermInfo {
            type_: env::var("TERM").ok(),
            version: tty.get_version(),
            size: tty.get_size().into(),
            theme: tty.get_theme(),
        })
    }

    fn get_file_writer<N: Notifier + 'static>(
        &self,
        metadata: &Metadata,
        notifier: N,
    ) -> Result<Option<FileWriter>> {
        let Some(path) = self.output_file.as_ref() else {
            return Ok(None);
        };

        let path = Path::new(path);
        let (overwrite, append) = self.get_file_mode(path)?;
        let file = self.open_output_file(path, overwrite, append)?;
        let format = self.get_file_format(path, append)?;
        let notifier = Box::new(notifier);

        let file_writer = match format {
            Format::AsciicastV3 => {
                let writer = Box::new(LineWriter::new(file));
                let encoder = Box::new(AsciicastV3Encoder::new(append));

                FileWriter::new(writer, encoder, notifier, metadata.clone())
            }

            Format::AsciicastV2 => {
                let time_offset = if append {
                    asciicast::get_duration(path)?
                } else {
                    0
                };

                let writer = Box::new(LineWriter::new(file));
                let encoder = Box::new(AsciicastV2Encoder::new(append, time_offset));

                FileWriter::new(writer, encoder, notifier, metadata.clone())
            }

            Format::Raw => {
                let writer = Box::new(file);
                let encoder = Box::new(RawEncoder::new());

                FileWriter::new(writer, encoder, notifier, metadata.clone())
            }

            Format::Txt => {
                let writer = Box::new(file);
                let encoder = Box::new(TextEncoder::new());

                FileWriter::new(writer, encoder, notifier, metadata.clone())
            }
        };

        Ok(Some(file_writer))
    }

    fn get_file_mode(&self, path: &Path) -> Result<(bool, bool)> {
        let mut overwrite = self.overwrite;
        let mut append = self.append;

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

        Ok((overwrite, append))
    }

    fn get_file_format(&self, path: &Path, append: bool) -> Result<Format> {
        self.output_format.map(Ok).unwrap_or_else(|| {
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
        })
    }

    fn open_output_file(&self, path: &Path, overwrite: bool, append: bool) -> Result<File> {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }

        OpenOptions::new()
            .write(true)
            .append(append)
            .create(overwrite)
            .create_new(!overwrite && !append)
            .truncate(overwrite)
            .open(path)
            .map_err(|e| e.into())
    }

    fn get_listener(&self) -> Result<Option<TcpListener>> {
        let Some(addr) = self.stream_local else {
            return Ok(None);
        };

        TcpListener::bind(addr)
            .map(Some)
            .context("cannot start listener")
    }

    fn get_relay(&mut self, metadata: &Metadata, config: &config::Config) -> Result<Option<Relay>> {
        let Some(target) = &self.stream_remote else {
            return Ok(None);
        };

        let relay = match target {
            RelayTarget::StreamId(id) => {
                let stream = api::create_user_stream(id, config)?;
                let ws_producer_url = build_producer_url(&stream.ws_producer_url, metadata)?;

                Relay {
                    ws_producer_url,
                    url: Some(stream.url.parse()?),
                }
            }

            RelayTarget::WsProducerUrl(url) => Relay {
                ws_producer_url: url.clone(),
                url: None,
            },
        };

        Ok(Some(relay))
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
        let Some(path) = &self.log_file else {
            return Ok(());
        };

        let file = self.open_log_file(path)?;

        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();

        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_env_filter(filter)
            .with_writer(file)
            .init();

        Ok(())
    }

    fn open_log_file(&self, path: &PathBuf) -> Result<File> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| anyhow!("cannot open log file {}: {}", path.to_string_lossy(), e))
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

fn build_producer_url(url: &str, metadata: &Metadata) -> Result<Url> {
    let mut url: Url = url.parse()?;
    let mut params = Vec::new();

    if let Some(type_) = &metadata.term.type_ {
        params.push(("term[type]".to_string(), type_.clone()));
    }

    if let Some(version) = &metadata.term.version {
        params.push(("term[version]".to_string(), version.clone()));
    }

    if let Ok(shell) = env::var("SHELL") {
        params.push(("shell".to_string(), shell));
    }

    if let Some(title) = &metadata.title {
        params.push(("title".to_string(), title.clone()));
    }

    for (k, v) in &metadata.env {
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
