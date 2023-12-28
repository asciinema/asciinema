mod cmd;
mod config;
mod format;
mod locale;
mod pty;
mod recorder;
use crate::config::Config;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
#[command(name = "asciinema")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// asciinema server URL
    #[arg(long)]
    server_url: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Record a terminal session
    Rec(cmd::rec::Cli),

    /// Replay a terminal session
    Play(cmd::play::Cli),

    /// Concatenate multiple recordings
    Cat(cmd::cat::Cli),

    /// Upload a recording to an asciinema server
    Upload(cmd::upload::Cli),

    /// Authenticate this CLI with an asciinema server account
    Auth(cmd::auth::Cli),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::new(cli.server_url.clone())?;

    match cli.command {
        Commands::Rec(record) => record.run(),
        Commands::Play(play) => play.run(),
        Commands::Cat(cat) => cat.run(),
        Commands::Upload(upload) => upload.run(&config),
        Commands::Auth(auth) => auth.run(&config),
    }
}
