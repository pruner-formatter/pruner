use anyhow::Result;
use std::{fs, path::PathBuf};

use pruner::api::grammar;

#[test]
fn load_grammars_skips_missing_base_query_files() -> Result<()> {
  let _ = env_logger::builder().is_test(true).try_init();

  let grammar_root = PathBuf::from("tests/fixtures/grammars");
  let grammar_dir = grammar_root.join("nix");
  let tree_sitter_json = grammar_dir.join("tree-sitter.json");
  let original_tree_sitter_json = fs::read_to_string(&tree_sitter_json)?;

  let _restore = RestoreFile {
    path: tree_sitter_json.clone(),
    contents: original_tree_sitter_json,
  };

  fs::write(
    &tree_sitter_json,
    r#"{
  "$schema": "https://tree-sitter.github.io/tree-sitter/assets/schemas/config.schema.json",
  "grammars": [
    {
      "name": "nix",
      "camelcase": "Nix",
      "title": "Nix",
      "scope": "source.nix",
      "file-types": ["nix"],
      "injection-regex": "^nix$",
      "injections": ["queries/missing.scm", "queries/injections.scm"],
      "tags": "queries/tags.scm",
      "class-name": "TreeSitterNix"
    }
  ],
  "metadata": {
    "version": "0.3.0",
    "license": "MIT",
    "description": "Nix grammar for tree-sitter"
  },
  "bindings": {
    "c": true,
    "go": true,
    "node": true,
    "python": true,
    "rust": true,
    "swift": true,
    "zig": false
  }
}
"#,
  )?;

  let grammars = grammar::load_grammars(
    &[grammar_root],
    &[],
    Some(PathBuf::from("tests/fixtures/.build")),
  )?;

  assert!(grammars.contains_key("nix"));

  Ok(())
}

struct RestoreFile {
  path: PathBuf,
  contents: String,
}

impl Drop for RestoreFile {
  fn drop(&mut self) {
    let _ = fs::write(&self.path, &self.contents);
  }
}
