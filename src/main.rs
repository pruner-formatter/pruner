use anyhow::Result;
use clap::Parser as ClapParser;

mod api;
mod cli;
mod commands;
mod config;

fn main() -> Result<()> {
  let cli = cli::Cli::parse();

  let mut log_builder = env_logger::builder();
  log_builder
    .format_timestamp(None)
    .format_target(false)
    .filter_level(cli.global_opts.log_level.unwrap_or(log::LevelFilter::Info));
  log_builder.init();

  match cli.command {
    cli::Commands::Format(args) => {
      commands::format::handle(args, cli.global_opts)?;
    }
  }

  Ok(())
}
