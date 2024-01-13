use anyhow::Result;
use std::{
    ffi::OsStr,
    path::PathBuf,
    process::{Command, Stdio},
};
use which::which;

pub trait Notifier: Send {
    fn notify(&self, message: String) -> Result<()>;
}

pub fn get_notifier(custom_command: Option<String>) -> Box<dyn Notifier> {
    if let Some(command) = custom_command {
        Box::new(CustomNotifier(command))
    } else {
        LibNotifyNotifier::get()
            .map(|n| Box::new(n) as Box<dyn Notifier>)
            .or_else(|| AppleScriptNotifier::get().map(|n| Box::new(n) as Box<dyn Notifier>))
            .unwrap_or_else(|| Box::new(NullNotifier))
    }
}

pub struct LibNotifyNotifier(PathBuf);

impl LibNotifyNotifier {
    fn get() -> Option<Self> {
        which("notify-send").ok().map(LibNotifyNotifier)
    }
}

impl Notifier for LibNotifyNotifier {
    fn notify(&self, message: String) -> Result<()> {
        exec(&mut Command::new(&self.0), &["asciinema", &message])
    }
}

pub struct AppleScriptNotifier(PathBuf);

impl AppleScriptNotifier {
    fn get() -> Option<Self> {
        which("osascript").ok().map(AppleScriptNotifier)
    }
}

impl Notifier for AppleScriptNotifier {
    fn notify(&self, message: String) -> Result<()> {
        let text = message.replace('\"', "\\\"");
        let script = format!("display notification \"{text}\" with title \"asciinema\"");

        exec(&mut Command::new(&self.0), &["-e", &script])
    }
}

pub struct CustomNotifier(String);

impl Notifier for CustomNotifier {
    fn notify(&self, text: String) -> Result<()> {
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
    fn notify(&self, _text: String) -> Result<()> {
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
