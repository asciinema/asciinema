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
    stream: Arc<Mutex<Option<UnixStream>>>,
    encoder: Box<dyn encoder::Encoder + Send>,
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
        };
        let mut encoder = self.encoder;
        let socket_path = self.socket_path.clone();
        let handle = self.handle.clone();
        let header_bytes = encoder.header(&header);
        let fut = async move {
            let stream = UnixStream::connect(socket_path.clone()).await;
            let shared_stream = Arc::new(Mutex::new(None));
            match stream {
                Ok(mut s) => {
                    if let Err(e) = s.write_all(&header_bytes).await {
                        //status::warning!("Socket write error, session won't be recorded: {e}");
                    } else {
                      //  status::info!("SocketWriter: initial connection established");
                        *shared_stream.lock().await = Some(s);
                    }
                }
                Err(e) => {
                    //status::warning!("Socket connect error, session won't be recorded: {e}");
                }
            }
            // Spawn background reconnection task
            let stream_clone = shared_stream.clone();
            let socket_path_clone = socket_path.clone();
            let header_bytes_clone = header_bytes.clone();
            handle.spawn(async move {
                loop {
                    let mut guard = stream_clone.lock().await;
                    if guard.is_none() {
                        match UnixStream::connect(socket_path_clone.clone()).await {
                            Ok(mut s) => {
                                // Try to send header
                                match s.write_all(&header_bytes_clone).await {
                                    Ok(_) => {
                                        //status::info!("SocketWriter: reconnected and header sent");
                                        *guard = Some(s);
                                    }
                                    Err(e) => {
                                        //status::warning!("Socket write error on reconnect: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                //status::warning!("Socket reconnect failed: {e}");
                            }
                        }
                    }
                    drop(guard);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });
            Ok(Box::new(SocketWriter {
                stream: shared_stream,
                encoder,
                handle,
            }) as Box<dyn session::Output>)
        };
        self.handle.block_on(fut)
    }
}

impl session::Output for SocketWriter {
    fn event(&mut self, event: session::Event) -> io::Result<()> {
        let bytes = self.encoder.event(event.into());
        let stream = self.stream.clone();
        let fut = async move {
            let mut guard = stream.lock().await;
            if let Some(ref mut s) = *guard {
                match s.write_all(&bytes).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        // status::warning!("SocketWriter: event write failed, dropping connection: {e}");
                        *guard = None;
                        Ok(())
                    }
                }
            } else {
                // If no connection, drop the event
                Ok(())
            }
        };
        self.handle.block_on(fut)
    }

    fn flush(&mut self) -> io::Result<()> {
        let bytes = self.encoder.flush();
        let stream = self.stream.clone();
        let fut = async move {
            let mut guard = stream.lock().await;
            if let Some(ref mut s) = *guard {
                match s.write_all(&bytes).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        //status::warning!("SocketWriter: flush write failed, dropping connection: {e}");
                        *guard = None;
                        Ok(())
                    }
                }
            } else {
                // If no connection, drop the flush
                Ok(())
            }
        };
        self.handle.block_on(fut)
    }
} 