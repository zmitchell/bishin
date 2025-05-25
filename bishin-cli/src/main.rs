use anyhow::Error;
use clap::Parser;
use cmd::{Cli, handle_args};

mod cmd;

fn main() -> Result<(), Error> {
    let args = Cli::parse();
    handle_args(&args)?;
    Ok(())
}
