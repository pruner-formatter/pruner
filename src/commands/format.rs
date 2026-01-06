use anyhow::{Context, Result};
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
  /// The language name of the root document. Regions containing injected languages will be
  /// dynamically discovered from queries.
  #[arg(long)]
  lang: String,

  /// The desired print-width of the document after which text should wrap. This value specifies the
  /// starting point and will be dynamically adjusted for injected language regions.
  #[arg(long, short('w'), default_value_t = 80)]
  print_width: u32,

  /// Specifying this will skip formatting the document root. This means only regions within the
  /// document containing language injections will be formatted. If you only want to use pruner to
  /// format injected regions, then this is the option to use.
  ///
  /// This can be especially useful in an editor context where you might want to use your LSP to
  /// format your document root, and then run pruner on the result to format injected regions.
  #[arg(
    long,
    short('R'),
    default_value_t = false,
    num_args = 0..=1,
    default_missing_value = "true",
    value_parser = clap::builder::BoolValueParser::new()
  )]
  skip_root: bool,

  /// The current working directory. Only used when formatting files.
  #[arg(long, short('d'))]
  dir: Option<PathBuf>,

  /// Specify a file exclusion pattern as a glob. Any files matching this pattern will not be
  /// formatted. Can be specified multiple times.
  #[arg(long, short('e'))]
  exclude: Option<Vec<String>>,

  /// Setting this to true will result in no files being modified on disk. If any files are
  /// considered 'dirty' meaning, meaning they are not correctly formatted, then pruner will exit
  /// with a non-0 exit code.
  #[arg(
    long,
    short('c'),
    default_value_t = false,
    num_args = 0..=1,
    default_missing_value = "true",
    value_parser = clap::builder::BoolValueParser::new()
  )]
  check: bool,

  /// A file pattern, in glob format, describing files on disk to be formatted.
  ///
  /// If this is specified then pruner will recursively format all files in the cwd (or --dir if
  /// set) that match this pattern.
  ///
  /// If this is _not_ set then pruner will expect source code to be provided via stdin and the
  /// formatted result will be outputted over stdout.
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
    args.skip_root,
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
    args.skip_root,
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
