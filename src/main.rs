mod api;
mod asciicast;
mod cli;
mod cmd;
mod config;
mod encoder;
mod io;
mod locale;
mod logger;
mod notifier;
mod player;
mod pty;
mod recorder;
mod streamer;
mod tty;
mod util;
use crate::cli::{Cli, Commands};
use crate::config::Config;
use clap::Parser;
use cmd::Command;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::new(cli.server_url.clone())?;

    if cli.quiet {
        logger::disable();
    }

    match cli.command {
        Commands::Rec(record) => record.run(&config),
        Commands::Play(play) => play.run(&config),
        Commands::Stream(stream) => stream.run(&config),
        Commands::Cat(cat) => cat.run(&config),
        Commands::Convert(convert) => convert.run(&config),
        Commands::Upload(upload) => upload.run(&config),
        Commands::Auth(auth) => auth.run(&config),
    }
}
