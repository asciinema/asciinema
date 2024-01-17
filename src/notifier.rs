use anyhow::Result;
use std::{
    env,
    ffi::OsStr,
    path::PathBuf,
    process::{Command, Stdio},
};
use which::which;

pub trait Notifier: Send {
    fn notify(&mut self, message: String) -> Result<()>;
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

impl Notifier for TmuxNotifier {
    fn notify(&mut self, message: String) -> Result<()> {
        let args = ["display-message", &format!("asciinema: {}", message)];

        exec(&mut Command::new(&self.0), &args)
    }
}

pub struct LibNotifyNotifier {
    binary_path: PathBuf,
    last_notification_id: Option<String>,
}

impl LibNotifyNotifier {
    fn get() -> Option<Self> {
        which("notify-send").ok().map(|path| LibNotifyNotifier {
            binary_path: path,
            last_notification_id: None,
        })
    }
}

impl Notifier for LibNotifyNotifier {
    fn notify(&mut self, message: String) -> Result<()> {
        let mut args: Vec<&str> = Vec::new();

        if let Some(id) = &self.last_notification_id {
            args.extend_from_slice(&["-r", id]);
        }

        args.extend_from_slice(&["-p", "asciinema", &message]);

        let output = Command::new(&self.binary_path).args(args).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let id = stdout.trim();

        if !id.is_empty() {
            self.last_notification_id = Some(id.to_owned())
        }

        Ok(())
    }
}

pub struct AppleScriptNotifier(PathBuf);

impl AppleScriptNotifier {
    fn get() -> Option<Self> {
        which("osascript").ok().map(AppleScriptNotifier)
    }
}

impl Notifier for AppleScriptNotifier {
    fn notify(&mut self, message: String) -> Result<()> {
        let text = message.replace('\"', "\\\"");
        let script = format!("display notification \"{text}\" with title \"asciinema\"");

        exec(&mut Command::new(&self.0), &["-e", &script])
    }
}

pub struct CustomNotifier(String);

impl Notifier for CustomNotifier {
    fn notify(&mut self, text: String) -> Result<()> {
        exec::<&str>(
            Command::new("/bin/sh")
                .args(["-c", &self.0])
                .env("TEXT", text),
            &[],
        )
    }
}

pub struct NullNotifier;

impl Notifier for NullNotifier {
    fn notify(&mut self, _text: String) -> Result<()> {
        Ok(())
    }
}

fn exec<S: AsRef<OsStr>>(command: &mut Command, args: &[S]) -> Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(args)
        .status()?;

    Ok(())
}
