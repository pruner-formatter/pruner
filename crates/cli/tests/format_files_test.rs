use anyhow::Result;
use rayon::prelude::*;
use std::{
  collections::BTreeMap,
  fs,
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use pruner::{
  api::{
    diff,
    format::{self, FormatContext, FormatOpts},
  },
  wasm::formatter::WasmFormatter,
};

mod common;

#[test]
fn format_files() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let language_aliases = common::language_aliases();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let input_dir = PathBuf::from("tests/fixtures/tests/format_files/input");
  let output_dir = PathBuf::from("tests/fixtures/tests/format_files/output");
  let temp_dir = create_temp_dir("pruner-format-files")?;
  let opts = FormatOpts {
    printwidth: 80,
    language: "clojure",
    source_file: None,
  };
  let format_context = FormatContext {
    grammars: &grammars,
    languages: &languages,
    language_aliases: &language_aliases,
    formatters: &formatters,
    wasm_formatter: &wasm_formatter,
  };

  copy_dir_recursive(&input_dir, &temp_dir)?;

  let changes = format::format_files(
    &temp_dir,
    "**/*.clj",
    None,
    true,
    &opts,
    false,
    &format_context,
  )?;

  for change in changes.collect::<Vec<_>>() {
    change?;
  }

  let actual_files = collect_files(&temp_dir)?;
  let expected_files = collect_files(&output_dir)?;

  assert_eq!(actual_files, expected_files);

  let _ = fs::remove_dir_all(&temp_dir);
  Ok(())
}

#[test]
fn check_mode_returns_file_changes_for_dirty_files() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let language_aliases = common::language_aliases();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let input_dir = PathBuf::from("tests/fixtures/tests/format_files/input");
  let temp_dir = create_temp_dir("pruner-format-files-check")?;
  let opts = FormatOpts {
    printwidth: 80,
    language: "clojure",
    source_file: None,
  };
  let format_context = FormatContext {
    grammars: &grammars,
    languages: &languages,
    language_aliases: &language_aliases,
    formatters: &formatters,
    wasm_formatter: &wasm_formatter,
  };

  copy_dir_recursive(&input_dir, &temp_dir)?;

  let changes = format::format_files(
    &temp_dir,
    "**/*.clj",
    None,
    false,
    &opts,
    false,
    &format_context,
  )?;

  let changes = changes
    .collect::<Vec<_>>()
    .into_iter()
    .collect::<Result<Vec<_>>>()?;
  assert!(!changes.is_empty());
  for change in changes {
    let diff = diff::unified_diff(&change.path, &change.original, &change.formatted);
    assert!(diff.contains("--- a/"));
    assert!(diff.contains("+++ b/"));
    assert!(diff.contains("@@"));
  }

  let _ = fs::remove_dir_all(&temp_dir);
  Ok(())
}

#[test]
fn relative_include_globs_match_exact_paths() -> Result<()> {
  assert_glob_matches("a.clj", None, &["a.clj"])?;
  assert_glob_matches("nested/b.clj", None, &["nested/b.clj"])?;
  Ok(())
}

#[test]
fn relative_exclude_globs_skip_files() -> Result<()> {
  assert_glob_matches("**/*.clj", Some(vec!["a.clj".into()]), &["nested/b.clj"])?;
  assert_glob_matches("**/*.clj", Some(vec!["nested/b.clj".into()]), &["a.clj"])?;
  Ok(())
}

fn assert_glob_matches(
  include_glob: &str,
  exclude_globs: Option<Vec<String>>,
  expected_paths: &[&str],
) -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let language_aliases = common::language_aliases();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let input_dir = PathBuf::from("tests/fixtures/tests/format_files/input");
  let output_dir = PathBuf::from("tests/fixtures/tests/format_files/output");
  let temp_dir = create_temp_dir("pruner-format-files-glob")?;
  let opts = FormatOpts {
    printwidth: 80,
    language: "clojure",
    source_file: None,
  };
  let format_context = FormatContext {
    grammars: &grammars,
    languages: &languages,
    language_aliases: &language_aliases,
    formatters: &formatters,
    wasm_formatter: &wasm_formatter,
  };

  assert!(temp_dir.is_absolute());
  copy_dir_recursive(&input_dir, &temp_dir)?;

  let changes = format::format_files(
    &temp_dir,
    include_glob,
    exclude_globs,
    true,
    &opts,
    false,
    &format_context,
  )?
  .collect::<Vec<_>>()
  .into_iter()
  .collect::<Result<Vec<_>>>()?;

  let mut actual_paths = changes
    .iter()
    .map(|change| {
      Path::new(&change.path)
        .strip_prefix(&temp_dir)
        .map(Path::to_path_buf)
    })
    .collect::<std::result::Result<Vec<_>, _>>()?;
  actual_paths.sort();
  let mut expected_paths = expected_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
  expected_paths.sort();
  assert_eq!(actual_paths, expected_paths);

  let mut expected_files = collect_files(&input_dir)?;
  for path in expected_paths {
    expected_files.insert(path.clone(), fs::read_to_string(output_dir.join(path))?);
  }
  assert_eq!(collect_files(&temp_dir)?, expected_files);

  let _ = fs::remove_dir_all(&temp_dir);
  Ok(())
}

fn create_temp_dir(prefix: &str) -> Result<PathBuf> {
  let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
  let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
  fs::create_dir_all(&dir)?;
  Ok(dir)
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
  fs::create_dir_all(to)?;
  for entry in fs::read_dir(from)? {
    let entry = entry?;
    let path = entry.path();
    let target = to.join(entry.file_name());
    let file_type = entry.file_type()?;
    if file_type.is_dir() {
      copy_dir_recursive(&path, &target)?;
    } else if file_type.is_file() {
      fs::copy(&path, &target)?;
    }
  }
  Ok(())
}

fn collect_files(dir: &Path) -> Result<BTreeMap<PathBuf, String>> {
  let mut files = BTreeMap::new();
  collect_files_inner(dir, dir, &mut files)?;
  Ok(files)
}

fn collect_files_inner(
  dir: &Path,
  base: &Path,
  files: &mut BTreeMap<PathBuf, String>,
) -> Result<()> {
  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();
    let file_type = entry.file_type()?;
    if file_type.is_dir() {
      collect_files_inner(&path, base, files)?;
    } else if file_type.is_file() {
      let relative = path.strip_prefix(base)?.to_path_buf();
      let contents = fs::read_to_string(&path)?;
      files.insert(relative, contents);
    }
  }
  Ok(())
}
