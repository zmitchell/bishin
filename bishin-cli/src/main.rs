use clap::Parser;
use cmd::Cli;

mod cmd;

fn main() {
    let _args = Cli::parse();
    println!("Hello, world!");
}
