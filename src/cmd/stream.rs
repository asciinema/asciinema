use super::Command;
use crate::api;
use crate::cli;
use crate::config::Config;
use crate::locale;
use crate::logger;
use crate::notifier;
use crate::pty;
use crate::streamer::{self, KeyBindings};
use crate::tty::{self, FixedSizeTty};
use crate::util;
use anyhow::bail;
use anyhow::{anyhow, Context, Result};
use cli::{RelayTarget, DEFAULT_LISTEN_ADDR};
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::fs;
use std::net::TcpListener;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use url::{form_urlencoded, Url};

#[derive(Debug)]
struct Relay {
    ws_producer_url: Url,
    url: Option<Url>,
}

impl Command for cli::Stream {
    fn run(mut self, config: &Config) -> Result<()> {
        locale::check_utf8_locale()?;

        if self.serve.is_none() && self.relay.is_none() {
            self.serve = Some(DEFAULT_LISTEN_ADDR.parse().unwrap());
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

        let notifier = notifier::threaded(notifier);

        {
            let mut tty = self.get_tty()?;

            let mut streamer = streamer::Streamer::new(
                listener,
                relay.map(|e| e.ws_producer_url),
                record_input,
                keys,
                notifier,
            );

            self.init_logging()?;
            pty::exec(&exec_command, &exec_extra_env, &mut tty, &mut streamer)?;
        }

        logger::info!("Streaming session ended");

        Ok(())
    }
}

impl cli::Stream {
    fn get_command(&self, config: &Config) -> Option<String> {
        self.command
            .as_ref()
            .cloned()
            .or(config.cmd_stream_command())
    }

    fn get_relay(&mut self, config: &Config) -> Result<Option<Relay>> {
        match self.relay.take() {
            Some(RelayTarget::StreamId(id)) => {
                let stream = api::create_user_stream(id, config)?;
                let ws_producer_url = self.build_producer_url(&stream.ws_producer_url)?;

                Ok(Some(Relay {
                    ws_producer_url,
                    url: Some(stream.url.parse()?),
                }))
            }

            Some(RelayTarget::WsProducerUrl(url)) => Ok(Some(Relay {
                ws_producer_url: url,
                url: None,
            })),

            None => Ok(None),
        }
    }

    fn build_producer_url(&self, url: &str) -> Result<Url> {
        let mut url: Url = url.parse()?;
        let term = env::var("TERM").ok().unwrap_or_default();
        let shell = env::var("SHELL").ok().unwrap_or_default();

        let params = vec![
            ("term[type]", term.clone()),
            ("shell", shell.clone()),
            ("env[TERM]", term),
            ("env[SHELL]", shell),
        ]
        .into_iter()
        .filter(|(_k, v)| v != "");

        let query = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();

        url.set_query(Some(&query));

        Ok(url)
    }

    fn get_listener(&self) -> Result<Option<TcpListener>> {
        if let Some(addr) = self.serve {
            return Ok(Some(
                TcpListener::bind(addr).context("cannot start listener")?,
            ));
        }

        Ok(None)
    }

    fn get_tty(&self) -> Result<FixedSizeTty> {
        let (cols, rows) = self.tty_size.unwrap_or((None, None));

        if self.headless {
            Ok(FixedSizeTty::new(tty::NullTty::open()?, cols, rows))
        } else if let Ok(dev_tty) = tty::DevTty::open() {
            Ok(FixedSizeTty::new(dev_tty, cols, rows))
        } else {
            logger::info!("TTY not available, streaming in headless mode");
            Ok(FixedSizeTty::new(tty::NullTty::open()?, cols, rows))
        }
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

fn get_key_bindings(config: &Config) -> Result<KeyBindings> {
    let mut keys = KeyBindings::default();

    if let Some(key) = config.cmd_stream_prefix_key()? {
        keys.prefix = key;
    }

    if let Some(key) = config.cmd_stream_pause_key()? {
        keys.pause = key;
    }

    if let Some(key) = config.cmd_stream_add_marker_key()? {
        keys.add_marker = key;
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
