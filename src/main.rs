mod cmd;
mod format;
mod locale;
mod pty;
mod recorder;
mod util;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
#[command(name = "asciinema")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Record terminal session
    #[command(name = "rec")]
    Record(cmd::record::Cli),

    /// Play terminal session
    Play(cmd::play::Cli),

    /// Print full output of terminal sessions
    Cat(cmd::cat::Cli),

    /// Upload recording to asciinema.org
    Upload(cmd::upload::Cli),

    /// Link this system to asciinema.org account
    Auth(cmd::auth::Cli),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Record(cli) => cli.run(),
        Commands::Play(cli) => cli.run(),
        Commands::Cat(cli) => cli.run(),
        Commands::Upload(cli) => cli.run(),
        Commands::Auth(cli) => cli.run(),
    }
}
