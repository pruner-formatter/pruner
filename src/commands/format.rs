use anyhow::{Context, Result};
use clap::ArgAction;
use std::{
  fs,
  io::Read,
  path::{Path, PathBuf},
  process::exit,
  time::Instant,
};

use crate::{
  api::{
    self,
    format::{self, FormatContext, FormatOpts},
  },
  cli::GlobalOpts,
  config::PrunerConfig,
};

#[derive(clap::Args, Debug)]
pub struct FormatArgs {
  #[arg(long)]
  lang: String,

  #[arg(long, default_value_t = 80)]
  print_width: u32,

  #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
  injected_regions_only: bool,

  #[arg(long)]
  dir: Option<PathBuf>,

  #[arg(long)]
  exclude: Option<Vec<String>>,

  #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
  check: bool,

  include_glob: Option<String>,
}

fn paths_relative_to(root: &Path, paths: &[PathBuf]) -> Vec<PathBuf> {
  paths
    .iter()
    .cloned()
    .map(|entry| root.join(entry))
    .collect::<Vec<_>>()
}

fn format_stdin(args: &FormatArgs, context: &FormatContext) -> Result<()> {
  let input = {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;
    buf
  };

  let start = Instant::now();
  let result = format::format(
    &input,
    &FormatOpts {
      printwidth: args.print_width,
      language: &args.lang,
    },
    args.injected_regions_only,
    context,
  )?;
  log::debug!(
    "Format time total: {:?}",
    Instant::now().duration_since(start)
  );

  print!("{}", String::from_utf8(result).unwrap());

  Ok(())
}

fn format_files(args: &FormatArgs, context: &FormatContext) -> Result<()> {
  let cwd = std::env::current_dir()?;

  let paths = format::format_files(
    &args.dir.clone().unwrap_or(cwd),
    &args.include_glob.clone().unwrap(),
    args.exclude.clone(),
    !args.check,
    &FormatOpts {
      printwidth: args.print_width,
      language: &args.lang,
    },
    args.injected_regions_only,
    context,
  )?;

  if args.check {
    if !paths.is_empty() {
      log::error!("{} dirty files", paths.len());
      exit(1);
    }
  } else {
    log::info!("formatted {} files", paths.len());
  }

  Ok(())
}

pub fn handle(args: FormatArgs, global: GlobalOpts) -> Result<()> {
  let xdg_dirs = xdg::BaseDirectories::with_prefix("pruner");
  let config_path = global.config.or(xdg_dirs.find_config_file("config.toml"));
  let pruner_config = match config_path.as_deref() {
    Some(config_path) => PrunerConfig::from_file(config_path)
      .with_context(|| format!("Failed to load config {:?}", config_path))?,
    None => PrunerConfig::default(),
  };

  let cwd = std::env::current_dir()?;
  let repos_dir = cwd.join(
    pruner_config
      .grammar_download_dir
      .clone()
      .unwrap_or(xdg_dirs.place_data_file("grammars")?),
  );
  let lib_dir = cwd.join(
    pruner_config
      .grammar_build_dir
      .clone()
      .unwrap_or(xdg_dirs.place_data_file("build")?),
  );

  fs::create_dir_all(&repos_dir)?;
  fs::create_dir_all(&lib_dir)?;

  let grammars = pruner_config.grammars.clone().unwrap_or_default();

  let start = Instant::now();
  api::git::clone_all_grammars(&repos_dir, &grammars)?;
  log::debug!(
    "Grammar clone duration: {:?}",
    Instant::now().duration_since(start)
  );

  let config_relative_path = config_path
    .and_then(|path| path.parent().map(PathBuf::from))
    .unwrap_or(cwd.clone());
  let mut grammar_paths = paths_relative_to(
    &config_relative_path,
    &pruner_config.grammar_paths.unwrap_or_default(),
  );
  grammar_paths.push(repos_dir);

  let query_paths = paths_relative_to(
    &config_relative_path,
    &pruner_config.query_paths.unwrap_or_default(),
  );

  let start = Instant::now();
  let grammars = api::grammar::load_grammars(&grammar_paths, &query_paths, Some(lib_dir))
    .context("Failed to load grammars")?;
  log::debug!(
    "Grammar load duration: {:?}",
    Instant::now().duration_since(start)
  );

  let context = FormatContext {
    grammars: &grammars,
    languages: &pruner_config.languages.unwrap_or_default(),
    formatters: &pruner_config.formatters.unwrap_or_default(),
  };

  if args.include_glob.is_some() {
    format_files(&args, &context)?;
  } else {
    format_stdin(&args, &context)?;
  }

  Ok(())
}
