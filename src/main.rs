mod format;
mod pty;
mod recorder;
use anyhow::Result;
use clap::{Parser, Subcommand};
use format::{asciicast, raw};
use std::env;
use std::fs;
use std::path;

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
        #[arg(long, conflicts_with = "append")]
        overwrite: bool,

        /// Command to record [default: $SHELL]
        #[arg(short, long)]
        command: Option<String>,

        /// List of env vars to save
        #[arg(short, long, default_value_t = String::from("SHELL,TERM"))]
        env: String,

        /// Title of the recording
        #[arg(short, long)]
        title: Option<String>,

        /// Limit idle time to given number of seconds
        #[arg(short, long, value_name = "SECS")]
        idle_time_limit: Option<f32>,

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

    /// Play terminal session
    Play {
        filename: String,

        /// Limit idle time to given number of seconds
        #[arg(short, long, value_name = "SECS")]
        idle_time_limit: Option<f64>,

        /// Set playback speed
        #[arg(short, long)]
        speed: Option<f64>,

        /// Loop loop loop loop
        #[arg(short, long, name = "loop")]
        loop_: bool,

        /// Automatically pause on markers
        #[arg(short = 'm', long)]
        pause_on_markers: bool,
    },

    /// Print full output of terminal sessions
    Cat {
        #[arg(required = true)]
        filename: Vec<String>,
    },

    /// Upload recording to asciinema.org
    Upload {
        /// Filename/path of asciicast to upload
        filename: String,
    },

    /// Link this system to asciinema.org account
    Auth,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Record {
            filename,
            stdin,
            mut append,
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
            let exists = path::Path::new(&filename).exists();
            append = append && exists;
            let mut opts = fs::OpenOptions::new();

            opts.write(true)
                .append(append)
                .create_new(!overwrite && !append)
                .truncate(overwrite);

            let writer: Box<dyn format::Writer> = if raw {
                let file = opts.open(&filename)?;

                Box::new(raw::Writer::new(file))
            } else {
                let writer = if append {
                    let time_offset = asciicast::get_duration(&filename)?;
                    let file = opts.open(&filename)?;
                    asciicast::Writer::new(file, time_offset)
                } else {
                    let file = opts.open(&filename)?;
                    asciicast::Writer::new(file, 0.0)
                };

                Box::new(writer)
            };

            let mut recorder = recorder::Recorder::new(
                writer,
                append,
                stdin,
                idle_time_limit,
                command.clone(),
                title.clone(),
            );

            let command = command
                .or(env::var("SHELL").ok())
                .unwrap_or("/bin/sh".to_owned());

            pty::exec(&["/bin/sh", "-c", &command], &mut recorder)?;
        }

        Commands::Play {
            filename,
            idle_time_limit,
            speed,
            loop_,
            pause_on_markers,
        } => todo!(),

        Commands::Cat { filename } => todo!(),

        Commands::Upload { filename } => todo!(),

        Commands::Auth => todo!(),
    }

    Ok(())
}
