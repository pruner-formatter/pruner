use anyhow::Result;
use std::{
  collections::BTreeMap,
  fs,
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use pruner::api::format::{self, FormatContext, FormatOpts};

mod common;

#[test]
fn format_files() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();

  let input_dir = PathBuf::from("tests/fixtures/tests/format_files/input");
  let output_dir = PathBuf::from("tests/fixtures/tests/format_files/output");
  let temp_dir = create_temp_dir("pruner-format-files")?;

  copy_dir_recursive(&input_dir, &temp_dir)?;

  format::format_files(
    &temp_dir,
    "**/*.clj",
    None,
    true,
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
    },
  )?;

  let actual_files = collect_files(&temp_dir)?;
  let expected_files = collect_files(&output_dir)?;

  assert_eq!(actual_files, expected_files);

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
