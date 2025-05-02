use std::collections::HashMap;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use regex::Regex;
use std::fs;
use std::path::PathBuf;

use tokio::net::UnixStream;
use tokio::runtime::Handle;
use tokio::sync::{Mutex, mpsc};
use tokio::io::AsyncWriteExt;

use crate::asciicast;
use crate::encoder;
use crate::session;
use crate::tty::{TtySize, TtyTheme};
use crate::status;

pub struct SocketWriterStarter {
    pub socket_path: String,
    pub encoder: Box<dyn encoder::Encoder + Send>,
    pub metadata: Metadata,
    pub handle: Handle,
}

pub struct SocketWriter {
    sender: mpsc::Sender<Vec<u8>>,
    encoder: Box<dyn encoder::Encoder + Send>,
    filters: Vec<(Regex, String)>,
}

pub struct Metadata {
    pub term_type: Option<String>,
    pub term_version: Option<String>,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub username: Option<String>,
    pub directory: Option<String>,
    pub shell: Option<String>,
}

fn load_regex_filters() -> Vec<(Regex, String)> {
    let mut filters = Vec::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let mut path = PathBuf::from(home);
    path.push(".focusbase/regexfilters.txt");
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some((pat, name)) = line.split_once(' ') {
                if let Ok(re) = Regex::new(pat) {
                    filters.push((re, name.trim().to_string()));
                }
            }
        }
    }
    filters
}

fn redact_all_filters(s: &str, filters: &[(Regex, String)]) -> String {
    let mut result = s.to_string();
    for (re, name) in filters {
        result = re.replace_all(&result, format!("[redact-{}]", name)).to_string();
    }
    result
}

impl session::OutputStarter for SocketWriterStarter {
    fn start(
        self: Box<Self>,
        time: SystemTime,
        tty_size: TtySize,
        theme: Option<TtyTheme>,
        child_pid: u32,
    ) -> io::Result<Box<dyn session::Output>> {
        let timestamp = time.duration_since(UNIX_EPOCH).unwrap().as_secs();
        let header = asciicast::Header {
            term_cols: tty_size.0,
            term_rows: tty_size.1,
            term_type: self.metadata.term_type,
            term_version: self.metadata.term_version,
            term_theme: theme,
            timestamp: Some(timestamp),
            idle_time_limit: self.metadata.idle_time_limit,
            command: self.metadata.command.as_ref().cloned(),
            title: self.metadata.title.as_ref().cloned(),
            env: self.metadata.env.as_ref().cloned(),
            child_pid: Some(child_pid),
            username: self.metadata.username.clone(),
            directory: self.metadata.directory.clone(),
            shell: self.metadata.shell.clone(),
        };
        let mut encoder = self.encoder;
        let socket_path = self.socket_path.clone();
        let handle = self.handle.clone();
        let header_bytes = encoder.header(&header);
        let (sender, mut receiver) = mpsc::channel::<Vec<u8>>(100); // buffer size 100, adjust as needed
        let filters = load_regex_filters();
        // Spawn background async task for socket writing
        handle.spawn(async move {
            let mut stream: Option<UnixStream> = None;
            // Try initial connect and send header
            loop {
                if stream.is_none() {
                    match UnixStream::connect(socket_path.clone()).await {
                        Ok(mut s) => {
                            if s.write_all(&header_bytes).await.is_ok() {
                                stream = Some(s);
                            }
                        }
                        Err(_) => {
                            // Could not connect, will retry
                        }
                    }
                }
                // If connected, try to write events
                if let Some(s) = stream.as_mut() {
                    tokio::select! {
                        Some(bytes) = receiver.recv() => {
                            if let Err(_) = s.write_all(&bytes).await {
                                // Drop connection on error
                                stream = None;
                            }
                        }
                        else => {
                            // Channel closed, exit
                            break;
                        }
                    }
                } else {
                    // Not connected, wait and retry
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        });
        Ok(Box::new(SocketWriter {
            sender,
            encoder,
            filters,
        }) as Box<dyn session::Output>)
    }
}

impl session::Output for SocketWriter {
    fn event(&mut self, event: session::Event) -> io::Result<()> {
        let event = match event {
            session::Event::Output(time, text, pid) =>
                session::Event::Output(time, redact_all_filters(&text, &self.filters), pid),
            other => other,
        };
        let bytes = self.encoder.event(event.into());
        let _ = self.sender.try_send(bytes);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        let bytes = self.encoder.flush();
        let _ = self.sender.try_send(bytes);
        Ok(())
    }
} 