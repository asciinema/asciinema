mod alis;
mod api;
mod asciicast;
mod cli;
mod cmd;
mod config;
mod encoder;
mod fd;
mod file_writer;
mod forwarder;
mod hash;
mod html;
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

use std::process::{ExitCode, Termination};

use clap::Parser;

use self::cli::{Cli, Commands, Session};

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.quiet {
        status::disable();
    }

    let _ = rustls::crypto::ring::default_provider().install_default();

    crate::config::check_legacy_config_file();

    match cli.command {
        Commands::Record(cmd) => {
            let cmd = Session {
                output_file: Some(cmd.file),
                rec_input: cmd.rec_input,
                append: cmd.append,
                output_format: cmd.output_format,
                overwrite: cmd.overwrite,
                command: cmd.command,
                rec_env: cmd.rec_env,
                title: cmd.title,
                idle_time_limit: cmd.idle_time_limit,
                headless: cmd.headless,
                window_size: cmd.window_size,
                stream_local: None,
                stream_remote: None,
                return_: cmd.return_,
                log_file: cmd.log_file,
                server_url: None,
            };

            cmd.run().report()
        }

        Commands::Stream(cmd) => {
            let cmd = Session {
                output_file: None,
                rec_input: cmd.rec_input,
                append: false,
                output_format: None,
                overwrite: false,
                command: cmd.command,
                rec_env: cmd.rec_env,
                title: cmd.title,
                idle_time_limit: None,
                headless: cmd.headless,
                window_size: cmd.window_size,
                stream_local: cmd.local,
                stream_remote: cmd.remote,
                return_: cmd.return_,
                log_file: cmd.log_file,
                server_url: cmd.server_url,
            };

            cmd.run().report()
        }

        Commands::Session(cmd) => cmd.run().report(),
        Commands::Play(cmd) => cmd.run().report(),
        Commands::Cat(cmd) => cmd.run().report(),
        Commands::Convert(cmd) => cmd.run().report(),
        Commands::Upload(cmd) => cmd.run().report(),
        Commands::Auth(cmd) => cmd.run().report(),
    }
}
