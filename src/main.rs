mod asciicast;
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

    /// Quiet mode, i.e. suppress diagnostic messages
    #[clap(short, long, global = true)]
    quiet: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Record a terminal session
    Rec(cmd::rec::Cli),

    /// Replay a terminal session
    Play(cmd::play::Cli),

    /// Stream a terminal session
    Stream(cmd::stream::Cli),

    /// Concatenate multiple recordings
    Cat(cmd::cat::Cli),

    /// Convert a recording into another format
    Convert(cmd::convert::Cli),

    /// Upload a recording to an asciinema server
    Upload(cmd::upload::Cli),

    /// Authenticate this CLI with an asciinema server account
    Auth(cmd::auth::Cli),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::new(cli.server_url.clone())?;

    if cli.quiet {
        logger::disable();
    }

    match cli.command {
        Commands::Rec(record) => record.run(&config),
        Commands::Play(play) => play.run(&config),
        Commands::Stream(stream) => stream.run(&config),
        Commands::Cat(cat) => cat.run(),
        Commands::Convert(convert) => convert.run(),
        Commands::Upload(upload) => upload.run(&config),
        Commands::Auth(auth) => auth.run(&config),
    }
}
