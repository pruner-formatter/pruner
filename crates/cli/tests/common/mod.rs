use anyhow::Result;
use fslock::LockFile;
use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};

use pruner::{
  api::grammar::{self, Grammars},
  config::{FormatterSpecs, LanguageFormatters},
};

#[allow(dead_code)]
pub fn formatters() -> FormatterSpecs {
  HashMap::from([
    (
      "prettier".to_string(),
      pruner::config::FormatterSpec {
        cmd: "prettier".into(),
        args: Vec::from([
          "--prose-wrap=always".into(),
          "--print-width=$textwidth".into(),
          "--parser=$language".into(),
        ]),
        stdin: None,
        fail_on_stderr: None,
      },
    ),
    (
      "cljfmt".to_string(),
      pruner::config::FormatterSpec {
        cmd: "cljfmt".into(),
        args: Vec::from([
          "fix".into(),
          "-".into(),
          "--remove-multiple-non-indenting-spaces".into(),
        ]),
        stdin: Some(true),
        fail_on_stderr: None,
      },
    ),
  ])
}

#[allow(dead_code)]
pub fn grammars() -> Result<Grammars> {
  grammars_with_queries(&["tests/fixtures/queries".into()])
}

#[allow(dead_code)]
pub fn grammars_with_queries(query_paths: &[PathBuf]) -> Result<Grammars> {
  let mut file = LockFile::open("tests/fixtures/.build.lock")?;
  file.lock()?;

  grammar::load_grammars(
    &["tests/fixtures/grammars".into()],
    query_paths,
    Some("tests/fixtures/.build".into()),
  )
}

#[allow(dead_code)]
pub fn languages() -> LanguageFormatters {
  HashMap::from([
    ("markdown".to_string(), vec!["prettier".into()]),
    ("clojure".to_string(), vec!["cljfmt".into()]),
    ("typescript".to_string(), vec!["prettier".into()]),
  ])
}

#[allow(dead_code)]
pub fn language_aliases() -> HashMap<String, String> {
  HashMap::new()
}

#[allow(dead_code)]
pub fn load_file(path: &str) -> String {
  let filepath = PathBuf::from("tests/fixtures/tests/").join(path);
  let mut file = File::open(filepath).expect("File should exist");
  let mut contents = String::new();
  file
    .read_to_string(&mut contents)
    .expect("Should be able to read source file");
  contents
}
