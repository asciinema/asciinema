use clap::CommandFactory;
use clap::ValueEnum;
use std::env;
use std::fs::create_dir_all;
use std::path::Path;
use std::path::PathBuf;

mod cli {
    include!("src/cli.rs");
}

const ENV_KEY: &str = "ASCIINEMA_GEN_DIR";

fn main() -> std::io::Result<()> {
    if let Some(dir) = env::var_os(ENV_KEY).or(env::var_os("OUT_DIR")) {
        let mut cmd = cli::Cli::command();
        let base_dir = PathBuf::from(dir);

        let man_dir = Path::join(&base_dir, "man");
        create_dir_all(&man_dir)?;
        clap_mangen::generate_to(cmd.clone(), &man_dir)?;

        let completion_dir = Path::join(&base_dir, "completion");
        create_dir_all(&completion_dir)?;

        for shell in clap_complete::Shell::value_variants() {
            clap_complete::generate_to(*shell, &mut cmd, "asciinema", &completion_dir)?;
        }
    }

    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rerun-if-env-changed={ENV_KEY}");

    Ok(())
}
