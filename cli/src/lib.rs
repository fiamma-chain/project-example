use clap::{CommandFactory, Parser, Subcommand};

pub mod subcommands;

#[derive(Debug, Parser)]
#[clap(author, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Subcommands>,
    #[clap(long = "version", short = 'V', help = "Print version info and exit")]
    pub version: bool,
}

#[derive(Debug, Subcommand)]
pub enum Subcommands {}

pub async fn run_command(cli: Cli) -> anyhow::Result<()> {
    match (cli.version, cli.command) {
        (false, None) => Ok(Cli::command().print_help()?),
        (true, _) => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        (false, Some(command)) => match command {},
    }
}
