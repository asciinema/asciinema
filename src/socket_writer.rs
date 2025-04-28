use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::net::UnixStream;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;

use crate::asciicast;
use crate::encoder;
use crate::notifier::Notifier;
use crate::session;
use crate::tty::{TtySize, TtyTheme};

pub struct SocketWriterStarter {
    pub socket_path: String,
    pub encoder: Box<dyn encoder::Encoder + Send>,
    pub metadata: Metadata,
    pub notifier: Box<dyn Notifier>,
    pub handle: Handle,
}

pub struct SocketWriter {
    stream: Arc<Mutex<UnixStream>>,
    encoder: Box<dyn encoder::Encoder + Send>,
    #[allow(dead_code)]
    notifier: Box<dyn Notifier>,
    handle: Handle,
}

pub struct Metadata {
    pub term_type: Option<String>,
    pub term_version: Option<String>,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

impl session::OutputStarter for SocketWriterStarter {
    fn start(
        self: Box<Self>,
        time: SystemTime,
        tty_size: TtySize,
        theme: Option<TtyTheme>,
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
            child_pid: None,
        };
        let mut encoder = self.encoder;
        let mut notifier = self.notifier;
        let socket_path = self.socket_path.clone();
        let handle = self.handle.clone();
        let header_bytes = encoder.header(&header);
        let fut = async move {
            let stream = UnixStream::connect(socket_path).await;
            match stream {
                Ok(mut stream) => {
                    if let Err(e) = stream.write_all(&header_bytes).await {
                        let _ = notifier.notify("Socket write error, session won't be recorded".to_owned());
                        return Err(io::Error::new(io::ErrorKind::Other, e));
                    }
                    Ok(Box::new(SocketWriter {
                        stream: Arc::new(Mutex::new(stream)),
                        encoder,
                        notifier,
                        handle,
                    }) as Box<dyn session::Output>)
                }
                Err(e) => {
                    let _ = notifier.notify("Socket connect error, session won't be recorded".to_owned());
                    Err(io::Error::new(io::ErrorKind::Other, e))
                }
            }
        };
        self.handle.block_on(fut)
    }
}

impl session::Output for SocketWriter {
    fn event(&mut self, event: session::Event) -> io::Result<()> {
        let bytes = self.encoder.event(event.into());
        let stream = self.stream.clone();
        let fut = async move {
            let mut stream = stream.lock().await;
            stream.write_all(&bytes).await
        };
        self.handle.block_on(fut)
    }

    fn flush(&mut self) -> io::Result<()> {
        let bytes = self.encoder.flush();
        let stream = self.stream.clone();
        let fut = async move {
            let mut stream = stream.lock().await;
            stream.write_all(&bytes).await
        };
        self.handle.block_on(fut)
    }
} 