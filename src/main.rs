mod asciicast;
mod pty;
mod recorder;
use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "asciinema")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Record terminal session
    #[command(name = "rec")]
    Record {
        filename: String,

        /// Enable input recording
        #[arg(long)]
        stdin: bool,

        /// Append to existing asciicast file
        #[arg(long)]
        append: bool,

        /// Save raw output only
        #[arg(long)]
        raw: bool,

        /// Overwrite target file if it already exists
        #[arg(long)]
        overwrite: bool,

        /// Command to record
        #[arg(short, long, default_value_t = String::from("$SHELL"))]
        command: String,

        /// List of env vars to save
        #[arg(short, long, default_value_t = String::from("SHELL,TERM"))]
        env: String,

        /// Title of the recording
        #[arg(short, long)]
        title: Option<String>,

        /// Limit idle time to given number of seconds
        #[arg(short, long, value_name = "SECS")]
        idle_time_limit: Option<f64>,

        /// Override terminal width (columns) for recorded command
        #[arg(long)]
        cols: Option<u16>,

        /// Override terminal height (rows) for recorded command
        #[arg(long)]
        rows: Option<u16>,

        /// Quiet mode - suppress all notices/warnings
        #[arg(short, long)]
        quiet: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Record {
            filename,
            stdin,
            append,
            raw,
            overwrite,
            command,
            env,
            title,
            idle_time_limit,
            cols,
            rows,
            quiet,
        } => {
            let format = if raw {
                recorder::Format::Raw
            } else {
                recorder::Format::Asciicast
            };

            let mut recorder = recorder::new(filename, format, append, stdin)?;
            pty::exec(&["/bin/bash"], &mut recorder)?;
        }
    }

    Ok(())
}
