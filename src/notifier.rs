use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::error;
use which::which;

#[async_trait]
pub trait Notifier: Send {
    async fn notify(&mut self, message: String) -> anyhow::Result<()>;
}

pub fn get_notifier(custom_command: Option<String>) -> Box<dyn Notifier> {
    if let Some(command) = custom_command {
        Box::new(CustomNotifier(command))
    } else {
        TmuxNotifier::get()
            .map(|n| Box::new(n) as Box<dyn Notifier>)
            .or_else(|| LibNotifyNotifier::get().map(|n| Box::new(n) as Box<dyn Notifier>))
            .or_else(|| AppleScriptNotifier::get().map(|n| Box::new(n) as Box<dyn Notifier>))
            .unwrap_or_else(|| Box::new(NullNotifier))
    }
}

pub struct TmuxNotifier(PathBuf);

impl TmuxNotifier {
    fn get() -> Option<Self> {
        env::var("TMUX")
            .ok()
            .and_then(|_| which("tmux").ok().map(TmuxNotifier))
    }
}

#[async_trait]
impl Notifier for TmuxNotifier {
    async fn notify(&mut self, message: String) -> anyhow::Result<()> {
        let args = ["display-message", &format!("asciinema: {message}")];

        exec(&mut Command::new(&self.0), &args).await
    }
}

pub struct LibNotifyNotifier(PathBuf);

impl LibNotifyNotifier {
    fn get() -> Option<Self> {
        which("notify-send").ok().map(LibNotifyNotifier)
    }
}

#[async_trait]
impl Notifier for LibNotifyNotifier {
    async fn notify(&mut self, message: String) -> anyhow::Result<()> {
        exec(&mut Command::new(&self.0), &["asciinema", &message]).await
    }
}

pub struct AppleScriptNotifier(PathBuf);

impl AppleScriptNotifier {
    fn get() -> Option<Self> {
        which("osascript").ok().map(AppleScriptNotifier)
    }
}

#[async_trait]
impl Notifier for AppleScriptNotifier {
    async fn notify(&mut self, message: String) -> anyhow::Result<()> {
        let text = message.replace('\"', "\\\"");
        let script = format!("display notification \"{text}\" with title \"asciinema\"");

        exec(&mut Command::new(&self.0), &["-e", &script]).await
    }
}

pub struct CustomNotifier(String);

#[async_trait]
impl Notifier for CustomNotifier {
    async fn notify(&mut self, text: String) -> anyhow::Result<()> {
        exec::<&str>(
            Command::new("/bin/sh")
                .args(["-c", &self.0])
                .env("TEXT", text),
            &[],
        )
        .await
    }
}

pub struct NullNotifier;

#[async_trait]
impl Notifier for NullNotifier {
    async fn notify(&mut self, _text: String) -> anyhow::Result<()> {
        Ok(())
    }
}

async fn exec<S: AsRef<OsStr>>(command: &mut Command, args: &[S]) -> anyhow::Result<()> {
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(args)
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "exit status: {}",
            status.code().unwrap_or(0)
        ))
    }
}

#[derive(Clone)]
pub struct BackgroundNotifier(mpsc::Sender<String>);

pub fn background(mut notifier: Box<dyn Notifier>) -> BackgroundNotifier {
    let (tx, mut rx) = mpsc::channel(16);

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(e) = notifier.notify(message).await {
                error!("notification failed: {e}");
                break;
            }
        }

        while rx.recv().await.is_some() {}
    });

    BackgroundNotifier(tx)
}

#[async_trait]
impl Notifier for BackgroundNotifier {
    async fn notify(&mut self, message: String) -> anyhow::Result<()> {
        self.0.send(message).await?;

        Ok(())
    }
}
