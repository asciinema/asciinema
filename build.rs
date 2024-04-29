use clap::CommandFactory;
use clap_mangen::Man;
use std::env;
use std::fs::File;
use std::io;
use std::path::PathBuf;

mod cli {
    include!("src/cli.rs");
}

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or(io::ErrorKind::NotFound)?);
    let cmd = cli::Cli::command();
    let man = Man::new(cmd);
    man.render(&mut File::create(out_dir.join("asciinema.1"))?)?;

    Ok(())
}
