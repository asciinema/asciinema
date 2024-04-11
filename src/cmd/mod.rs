pub mod auth;
pub mod cat;
pub mod convert;
pub mod play;
pub mod rec;
pub mod stream;
pub mod upload;
use crate::config::Config;
use crate::notifier;
use std::collections::HashMap;
use std::env;

fn get_notifier(config: &Config) -> Box<dyn notifier::Notifier> {
    if config.notifications.enabled {
        notifier::get_notifier(config.notifications.command.clone())
    } else {
        Box::new(notifier::NullNotifier)
    }
}

fn build_exec_command(command: Option<String>) -> Vec<String> {
    let command = command
        .or(env::var("SHELL").ok())
        .unwrap_or("/bin/sh".to_owned());

    vec!["/bin/sh".to_owned(), "-c".to_owned(), command]
}

fn build_exec_extra_env(vars: &[(String, String)]) -> HashMap<String, String> {
    let mut env = HashMap::new();

    env.insert("ASCIINEMA_REC".to_owned(), "1".to_owned());

    for (k, v) in vars {
        env.insert(k.clone(), v.clone());
    }

    env
}
