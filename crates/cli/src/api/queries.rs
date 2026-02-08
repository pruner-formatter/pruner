use anyhow::Result;
use std::{fs, path::PathBuf};
use tree_sitter::{Language, Query};

fn read_files(paths: &[PathBuf]) -> Result<String> {
  let mut out = String::new();
  for (i, p) in paths.iter().enumerate() {
    let contents = fs::read_to_string(p)
      .map_err(|e| anyhow::format_err!("Failed to read {}: {e}", p.display()))?;
    if i > 0 {
      out.push('\n');
    }
    out.push_str(&contents);
  }
  Ok(out)
}

fn merge_queries(base: &str, overlay: &str) -> String {
  if base.is_empty() {
    return overlay.to_owned();
  }

  if overlay.is_empty() {
    return base.to_owned();
  }

  let mut merged = String::with_capacity(base.len() + overlay.len() + 1);
  merged.push_str(base);
  if !base.ends_with('\n') {
    merged.push('\n');
  }
  merged.push_str(overlay);

  merged
}

fn is_extending(contents: &str) -> bool {
  contents
    .lines()
    .next()
    .map(|line| line.trim_start().starts_with(";; extends"))
    .unwrap_or(false)
}

fn read_query(queries_dirs: &[PathBuf], name: &str, filename: &str, base: &str) -> Result<String> {
  let mut result = base.to_owned();

  for dir in queries_dirs {
    let path = dir.join(name).join(filename);
    if path.is_file() {
      let contents = fs::read_to_string(&path)
        .map_err(|e| anyhow::format_err!("Failed to read {}: {e}", path.display()))?;

      if is_extending(&contents) {
        result = merge_queries(&result, &contents);
      } else {
        result = contents;
      }
    }
  }

  Ok(result)
}

pub fn load_injections_query(
  lang: &Language,
  name: &str,
  base_files: &[PathBuf],
  search_paths: &[PathBuf],
) -> Result<Query> {
  let base_queries = read_files(base_files)?;
  let query_content = read_query(search_paths, name, "injections.scm", &base_queries)?;
  Query::new(lang, &query_content).map_err(|err| anyhow::format_err!("{err:?}"))
}

pub fn load_optional_query(
  lang: &Language,
  name: &str,
  filename: &str,
  search_paths: &[PathBuf],
) -> Result<Option<Query>> {
  let query_content = read_query(search_paths, name, filename, "")?;
  if query_content.trim().is_empty() {
    return Ok(None);
  }

  let query = Query::new(lang, &query_content).map_err(|err| anyhow::format_err!("{err:?}"))?;
  Ok(Some(query))
}
