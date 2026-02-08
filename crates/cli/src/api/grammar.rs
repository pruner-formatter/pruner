use anyhow::{Context, Result};
use rayon::prelude::*;
use std::{collections::HashMap, fs, path::Path, path::PathBuf};
use tree_sitter::{Language, Query};
use tree_sitter_loader::{CompileConfig, Loader};

use super::queries;

#[derive(Debug)]
pub struct Grammar {
  #[allow(dead_code)]
  pub name: String,
  pub lang: Language,
  pub injections: Query,
  pub pruner_ignore: Option<Query>,
}

pub type Grammars = HashMap<String, Grammar>;

fn load_grammars_from_path(
  grammar_path: &Path,
  query_search_paths: &[PathBuf],
  lib_dir: &Option<PathBuf>,
) -> Result<Grammars> {
  let mut loader = match lib_dir {
    Some(dir) => Loader::with_parser_lib_path(dir.clone()),
    None => Loader::new()?,
  };

  loader
    .find_language_configurations_at_path(grammar_path, false)
    .with_context(|| {
      format!(
        "Failed to load language configuration from {:?}",
        grammar_path
      )
    })?;

  let mut languages = HashMap::new();

  for (config, path) in loader.get_all_language_configurations() {
    let src_path = path.join("src");

    let language = loader
      .load_language_at_path(CompileConfig::new(&src_path, None, None))
      .with_context(|| format!("Failed to load language {}", config.language_name))?;

    let injections = config
      .injections_filenames
      .clone()
      .unwrap_or_default()
      .iter()
      .map(|path| config.root_path.join(path))
      .collect::<Vec<_>>();

    let injections_query = queries::load_injections_query(
      &language,
      &config.language_name,
      &injections,
      query_search_paths,
    )?;

    let pruner_ignore = queries::load_optional_query(
      &language,
      &config.language_name,
      "pruner/ignore.scm",
      query_search_paths,
    )?;

    languages.insert(
      config.language_name.clone(),
      Grammar {
        name: config.language_name.clone(),
        lang: language,
        injections: injections_query,
        pruner_ignore,
      },
    );
  }

  Ok(languages)
}

pub fn load_grammars(
  grammar_search_paths: &[PathBuf],
  query_search_paths: &[PathBuf],
  lib_dir: Option<PathBuf>,
) -> Result<Grammars> {
  let mut grammar_paths = grammar_search_paths
    .par_iter()
    .map(|dir| {
      let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory {:?}", dir))?
        .filter_map(|entry| match entry {
          Ok(entry) => {
            let path = entry.path();
            if path.is_dir() {
              Some(path)
            } else {
              None
            }
          }
          Err(_) => None,
        });
      Ok(entries.collect::<Vec<_>>())
    })
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

  let mut languages = HashMap::new();
  grammar_paths.sort();

  let results = grammar_paths
    .par_iter()
    .map(|path| load_grammars_from_path(path, query_search_paths, &lib_dir))
    .collect::<Result<Vec<_>>>()?;

  for result in results {
    languages.extend(result);
  }

  Ok(languages)
}
