use crate::config::Config;
use crate::locale;
use crate::logger;
use crate::pty;
use crate::streamer::{self, KeyBindings};
use crate::tty;
use crate::util;
use anyhow::bail;
use anyhow::{anyhow, Context, Result};
use clap::Args;
use reqwest::blocking::Client;
use reqwest::header;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::fs;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use url::Url;

const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8080";

#[derive(Debug, Args)]
pub struct Cli {
    /// Enable input capture
    #[arg(long, short = 'I', alias = "stdin")]
    input: bool,

    /// Command to stream [default: $SHELL]
    #[arg(short, long)]
    command: Option<String>,

    /// Serve the stream via local HTTP server
    #[clap(short, long, value_name = "IP:PORT", default_missing_value = DEFAULT_LISTEN_ADDR, num_args = 0..=1)]
    listen: Option<SocketAddr>,

    /// Forward the stream to a relay, e.g. asciinema server
    #[clap(short, long, value_name = "STREAM-ID|WS-URL", default_missing_value = "", num_args = 0..=1, value_parser = validate_forward_target)]
    forward: Option<ForwardTarget>,

    /// Override terminal size for the session
    #[arg(long, value_name = "COLSxROWS")]
    tty_size: Option<pty::WinsizeOverride>,

    /// Log file path
    #[arg(long)]
    log_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
enum ForwardTarget {
    StreamId(String),
    WsProducerUrl(url::Url),
}

#[derive(Debug, Deserialize)]
struct GetStreamResponse {
    ws_producer_url: String,
    url: String,
}

#[derive(Debug)]
struct Relay {
    ws_producer_url: Url,
    url: Option<Url>,
}

fn validate_forward_target(s: &str) -> Result<ForwardTarget, String> {
    let s = s.trim();

    match url::Url::parse(s) {
        Ok(url) => {
            let scheme = url.scheme();

            if scheme == "ws" || scheme == "wss" {
                Ok(ForwardTarget::WsProducerUrl(url))
            } else {
                Err("must be a WebSocket URL (ws:// or wss://)".to_owned())
            }
        }

        Err(url::ParseError::RelativeUrlWithoutBase) => Ok(ForwardTarget::StreamId(s.to_owned())),
        Err(e) => Err(e.to_string()),
    }
}

impl Cli {
    pub fn run(mut self, config: &Config) -> Result<()> {
        locale::check_utf8_locale()?;

        if self.listen.is_none() && self.forward.is_none() {
            self.listen = Some(DEFAULT_LISTEN_ADDR.parse().unwrap());
        }

        let command = self.get_command(config);
        let keys = get_key_bindings(config)?;
        let notifier = super::get_notifier(config);
        let record_input = self.input || config.cmd_stream_input();
        let exec_command = super::build_exec_command(command.as_ref().cloned());
        let listener = self.get_listener()?;
        let relay = self.get_relay(config)?;
        let relay_id = relay.as_ref().map(|r| r.id());
        let exec_extra_env = build_exec_extra_env(relay_id.as_ref());

        if let (Some(id), Some(parent_id)) = (relay_id, parent_session_relay_id()) {
            if id == parent_id {
                if let Some(Relay { url: Some(url), .. }) = relay {
                    bail!("This shell is already being streamed at {url}");
                } else {
                    bail!("This shell is already being streamed");
                }
            }
        }

        logger::info!("Streaming session started");

        if let Some(listener) = &listener {
            logger::info!(
                "Live stream available at http://{}",
                listener.local_addr().unwrap()
            );
        }

        if let Some(Relay { url: Some(url), .. }) = &relay {
            logger::info!("Live stream available at {}", url);
        }

        if command.is_none() {
            logger::info!("Press <ctrl+d> or type 'exit' to end");
        }

        {
            let mut tty: Box<dyn tty::Tty> = if let Ok(dev_tty) = tty::DevTty::open() {
                Box::new(dev_tty)
            } else {
                logger::info!("TTY not available, streaming in headless mode");
                Box::new(tty::NullTty::open()?)
            };

            let mut streamer = streamer::Streamer::new(
                listener,
                relay.map(|e| e.ws_producer_url),
                record_input,
                keys,
                notifier,
                tty.get_theme(),
            );

            self.init_logging()?;

            pty::exec(
                &exec_command,
                &exec_extra_env,
                &mut *tty,
                self.tty_size,
                &mut streamer,
            )?;
        }

        logger::info!("Streaming session ended");

        Ok(())
    }

    fn get_command(&self, config: &Config) -> Option<String> {
        self.command
            .as_ref()
            .cloned()
            .or(config.cmd_stream_command())
    }

    fn get_relay(&mut self, config: &Config) -> Result<Option<Relay>> {
        match self.forward.take() {
            Some(ForwardTarget::StreamId(id)) => {
                let stream = get_server_stream(id, config)?;

                Ok(Some(Relay {
                    ws_producer_url: stream.ws_producer_url.parse()?,
                    url: Some(stream.url.parse()?),
                }))
            }

            Some(ForwardTarget::WsProducerUrl(url)) => Ok(Some(Relay {
                ws_producer_url: url,
                url: None,
            })),

            None => Ok(None),
        }
    }

    fn get_listener(&self) -> Result<Option<TcpListener>> {
        if let Some(addr) = self.listen {
            return Ok(Some(
                TcpListener::bind(addr).context("couldn't start listener")?,
            ));
        }

        Ok(None)
    }

    fn init_logging(&self) -> Result<()> {
        let log_file = self.log_file.as_ref().cloned();

        if let Some(path) = &log_file {
            let file = fs::OpenOptions::new()
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

impl Relay {
    fn id(&self) -> String {
        util::sha2_digest(self.ws_producer_url.as_ref())
    }
}

fn get_server_stream(stream_id: String, config: &Config) -> Result<GetStreamResponse> {
    let response = Client::new()
        .get(stream_api_url(&config.get_server_url()?, stream_id))
        .basic_auth("", Some(config.get_install_id()?))
        .header(header::ACCEPT, "application/json")
        .send()?;

    response.error_for_status_ref()?;

    let json = response.json::<GetStreamResponse>()?;

    Ok(json)
}

fn stream_api_url(server_url: &Url, stream_id: String) -> Url {
    let mut url = server_url.clone();

    if stream_id.is_empty() {
        url.set_path("api/user/stream");
    } else {
        url.set_path(&format!("api/user/streams/{stream_id}"));
    }

    url
}

fn get_key_bindings(config: &Config) -> Result<KeyBindings> {
    let mut keys = KeyBindings::default();

    if let Some(key) = config.cmd_stream_prefix_key()? {
        keys.prefix = key;
    }

    if let Some(key) = config.cmd_stream_pause_key()? {
        keys.pause = key;
    }

    Ok(keys)
}

fn build_exec_extra_env(relay_id: Option<&String>) -> HashMap<String, String> {
    match relay_id {
        Some(id) => super::build_exec_extra_env(&[("ASCIINEMA_RELAY_ID".to_string(), id.clone())]),
        None => super::build_exec_extra_env(&[]),
    }
}

fn parent_session_relay_id() -> Option<String> {
    env::var("ASCIINEMA_RELAY_ID").ok()
}
