mod alis;
mod api;
mod asciicast;
mod cli;
mod cmd;
mod config;
mod encoder;
mod file_writer;
mod forwarder;
mod io;
mod leb128;
mod locale;
mod notifier;
mod player;
mod pty;
mod server;
mod session;
mod status;
mod stream;
mod tty;
mod util;
mod socket_writer;

use clap::Parser;

use self::cli::{Cli, Commands, Session};
use self::config::Config;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::new(cli.server_url.clone())?;

    if cli.quiet {
        status::disable();
    }

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    match cli.command {
        Commands::Rec(cmd) => {
            let cmd = Session {
                output: Some(cmd.path),
                input: cmd.input,
                append: cmd.append,
                format: cmd.format,
                overwrite: cmd.overwrite,
                command: cmd.command,
                filename: cmd.filename,
                env: cmd.env,
                title: cmd.title,
                idle_time_limit: cmd.idle_time_limit,
                headless: cmd.headless,
                tty_size: cmd.tty_size,
                serve: None,
                relay: None,
                log_file: None,
                socket_path: None,
            };

            cmd.run(&config, &config.cmd_rec())
        }

        Commands::Stream(stream) => {
            let cmd = Session {
                output: None,
                input: stream.input,
                append: false,
                format: None,
                overwrite: false,
                command: stream.command,
                filename: None,
                env: stream.env,
                title: None,
                idle_time_limit: None,
                headless: stream.headless,
                tty_size: stream.tty_size,
                serve: stream.serve,
                relay: stream.relay,
                log_file: stream.log_file,
                socket_path: None,
            };

            cmd.run(&config, &config.cmd_stream())
        }

        Commands::Session(cmd) => cmd.run(&config, &config.cmd_session()),
        Commands::Play(cmd) => cmd.run(&config),
        Commands::Cat(cmd) => cmd.run(&config),
        Commands::Convert(cmd) => cmd.run(&config),
        Commands::Upload(cmd) => cmd.run(&config),
        Commands::Auth(cmd) => cmd.run(&config),
    }
}
