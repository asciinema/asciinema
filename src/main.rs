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
                output_file: Some(cmd.output_path),
                rec_input: cmd.rec_input,
                append: cmd.append,
                output_format: cmd.output_format,
                overwrite: cmd.overwrite,
                command: cmd.command,
                rec_env: cmd.rec_env,
                title: cmd.title,
                idle_time_limit: cmd.idle_time_limit,
                headless: cmd.headless,
                tty_size: cmd.tty_size,
                serve: None,
                relay: None,
                log_file: None,
            };

            cmd.run(&config, &config.cmd_rec())
        }

        Commands::Stream(stream) => {
            let cmd = Session {
                output_file: None,
                rec_input: stream.rec_input,
                append: false,
                output_format: None,
                overwrite: false,
                command: stream.command,
                rec_env: stream.rec_env,
                title: None,
                idle_time_limit: None,
                headless: stream.headless,
                tty_size: stream.tty_size,
                serve: stream.serve,
                relay: stream.relay,
                log_file: stream.log_file,
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
