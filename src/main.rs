mod pty;
use anyhow::Result;

fn main() -> Result<()> {
    pty::exec(&["/bin/bash"])?;

    Ok(())
}
