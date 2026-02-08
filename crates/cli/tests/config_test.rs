use pruner::config::{ConfigFile, ProfileConfig};
use std::{
  collections::HashMap,
  fs::{self, File},
  io::Write,
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir() -> PathBuf {
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("time should be available")
    .as_nanos();
  let temp_dir = std::env::temp_dir().join(format!("pruner-test-{nanos}"));
  fs::create_dir_all(&temp_dir).expect("should create temp dir");
  temp_dir
}

#[test]
fn loads_config_and_absolutizes_paths() {
  let temp_dir = unique_temp_dir();
  let config_path = temp_dir.join("config.toml");

  let mut file = File::create(&config_path).expect("should create config file");
  writeln!(
    file,
    r#"
query_paths = ["queries"]
grammar_paths = ["grammars"]
grammar_download_dir = "downloads"
grammar_build_dir = "build"
"#
  )
  .expect("should write config file");

  let config = ConfigFile::from_file(&config_path).expect("should load config");

  let query_paths = config.query_paths.expect("query_paths should be set");
  let grammar_paths = config.grammar_paths.expect("grammar_paths should be set");

  assert_eq!(query_paths.len(), 1);
  assert_eq!(grammar_paths.len(), 1);
  assert_eq!(query_paths[0], temp_dir.join("queries"));
  assert_eq!(grammar_paths[0], temp_dir.join("grammars"));

  assert_eq!(
    config
      .grammar_download_dir
      .expect("grammar_download_dir should be set"),
    temp_dir.join("downloads")
  );
  assert_eq!(
    config
      .grammar_build_dir
      .expect("grammar_build_dir should be set"),
    temp_dir.join("build")
  );
}

#[test]
fn merges_configs_with_overlay_priority() {
  let base = ConfigFile {
    query_paths: Some(vec![PathBuf::from("base_query")]),
    grammar_paths: Some(vec![PathBuf::from("base_grammar")]),
    grammar_download_dir: Some(PathBuf::from("base_downloads")),
    grammar_build_dir: Some(PathBuf::from("base_build")),
    languages: Some(HashMap::from([
      ("markdown".to_string(), vec!["base_fmt".into()]),
      ("clojure".to_string(), vec!["base_clj".into()]),
    ])),
    formatters: Some(HashMap::from([
      (
        "a".to_string(),
        pruner::config::FormatterSpec {
          cmd: "a".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
      (
        "fmt".to_string(),
        pruner::config::FormatterSpec {
          cmd: "base".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
    ])),
    ..Default::default()
  };

  let overlay = ConfigFile {
    query_paths: Some(vec![PathBuf::from("overlay_query")]),
    grammar_paths: Some(vec![PathBuf::from("overlay_grammar")]),
    grammar_download_dir: Some(PathBuf::from("overlay_downloads")),
    languages: Some(HashMap::from([
      ("markdown".to_string(), vec!["overlay_fmt".into()]),
      ("rust".to_string(), vec!["rust_fmt".into()]),
    ])),
    formatters: Some(HashMap::from([
      (
        "fmt".to_string(),
        pruner::config::FormatterSpec {
          cmd: "overlay".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
      (
        "b".to_string(),
        pruner::config::FormatterSpec {
          cmd: "b".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
    ])),
    ..Default::default()
  };

  let merged = ConfigFile::merge(&base, &overlay);

  assert_eq!(
    merged.query_paths.unwrap(),
    vec![PathBuf::from("base_query"), PathBuf::from("overlay_query")]
  );
  assert_eq!(
    merged.grammar_paths.unwrap(),
    vec![
      PathBuf::from("base_grammar"),
      PathBuf::from("overlay_grammar")
    ]
  );
  assert_eq!(
    merged.grammar_download_dir.unwrap(),
    PathBuf::from("overlay_downloads")
  );
  assert_eq!(
    merged.grammar_build_dir.unwrap(),
    PathBuf::from("base_build")
  );

  let formatters = merged.formatters.unwrap();
  assert_eq!(
    HashMap::from([
      (
        "a".to_string(),
        pruner::config::FormatterSpec {
          cmd: "a".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
      (
        "fmt".to_string(),
        pruner::config::FormatterSpec {
          cmd: "overlay".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
      (
        "b".to_string(),
        pruner::config::FormatterSpec {
          cmd: "b".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
    ]),
    formatters
  );

  let languages = merged.languages.unwrap();
  assert_eq!(
    HashMap::from([
      ("clojure".to_string(), vec!["base_clj".into()]),
      ("markdown".to_string(), vec!["overlay_fmt".into()]),
      ("rust".to_string(), vec!["rust_fmt".into()]),
    ]),
    languages
  );
}

#[test]
fn applies_profile_overrides() {
  let base = ConfigFile {
    query_paths: Some(vec![PathBuf::from("base_query")]),
    grammar_paths: Some(vec![PathBuf::from("base_grammar")]),
    grammar_download_dir: Some(PathBuf::from("base_downloads")),
    grammar_build_dir: Some(PathBuf::from("base_build")),
    languages: Some(HashMap::from([(
      "markdown".to_string(),
      vec!["base_fmt".into()],
    )])),
    formatters: Some(HashMap::from([(
      "fmt".to_string(),
      pruner::config::FormatterSpec {
        cmd: "base_cmd".to_string(),
        args: Vec::new(),
        stdin: None,
        fail_on_stderr: None,
      },
    )])),
    ..Default::default()
  };

  let profile = ProfileConfig {
    query_paths: Some(vec![PathBuf::from("profile_query")]),
    grammar_download_dir: Some(PathBuf::from("profile_downloads")),
    languages: Some(HashMap::from([
      ("markdown".to_string(), vec!["profile_fmt".into()]),
      ("rust".to_string(), vec!["rust_fmt".into()]),
    ])),
    ..Default::default()
  };

  let result = base.apply_profile(&profile);

  assert_eq!(
    result.query_paths.unwrap(),
    vec![PathBuf::from("base_query"), PathBuf::from("profile_query")]
  );
  assert_eq!(
    result.grammar_paths.unwrap(),
    vec![PathBuf::from("base_grammar")]
  );
  assert_eq!(
    result.grammar_download_dir.unwrap(),
    PathBuf::from("profile_downloads")
  );
  assert_eq!(
    result.grammar_build_dir.unwrap(),
    PathBuf::from("base_build")
  );

  let languages = result.languages.unwrap();
  assert_eq!(
    HashMap::from([
      ("markdown".to_string(), vec!["profile_fmt".into()]),
      ("rust".to_string(), vec!["rust_fmt".into()]),
    ]),
    languages
  );

  let formatters = result.formatters.unwrap();
  assert_eq!(
    HashMap::from([(
      "fmt".to_string(),
      pruner::config::FormatterSpec {
        cmd: "base_cmd".to_string(),
        args: Vec::new(),
        stdin: None,
        fail_on_stderr: None,
      },
    )]),
    formatters
  );
}

#[test]
fn loads_config_with_profiles_from_toml() {
  let temp_dir = unique_temp_dir();
  let config_path = temp_dir.join("config.toml");

  let mut file = File::create(&config_path).expect("should create config file");
  writeln!(
    file,
    r#"
query_paths = ["queries"]
grammar_download_dir = "downloads"

[languages]
markdown = ["prettier"]

[profiles.ci]
grammar_download_dir = "ci_downloads"

[profiles.ci.languages]
markdown = ["ci_prettier"]
rust = ["rustfmt"]
"#
  )
  .expect("should write config file");

  let config = ConfigFile::from_file(&config_path).expect("should load config");

  assert!(config.profiles.is_some());
  let profiles = config.profiles.unwrap();
  assert!(profiles.contains_key("ci"));

  let ci_profile = profiles.get("ci").unwrap();
  assert_eq!(
    ci_profile.grammar_download_dir,
    Some(temp_dir.join("ci_downloads"))
  );
  assert_eq!(
    ci_profile.languages,
    Some(HashMap::from([
      ("markdown".to_string(), vec!["ci_prettier".into()]),
      ("rust".to_string(), vec!["rustfmt".into()]),
    ]))
  );
}

#[test]
fn loads_and_normalizes_language_aliases() {
  let temp_dir = unique_temp_dir();
  let config_path = temp_dir.join("config.toml");

  let mut file = File::create(&config_path).expect("should create config file");
  writeln!(
    file,
    r#"
[language_aliases]
typescript = ["ts", "tsx"]
"#
  )
  .expect("should write config file");

  let config = pruner::config::load(pruner::config::LoadOpts {
    config_path: Some(config_path),
    profiles: Vec::new(),
  })
  .expect("should load config");

  assert_eq!(
    config.language_aliases,
    HashMap::from([
      ("ts".to_string(), "typescript".to_string()),
      ("tsx".to_string(), "typescript".to_string()),
    ])
  );
}

#[test]
fn language_alias_conflict_is_an_error() {
  let temp_dir = unique_temp_dir();
  let config_path = temp_dir.join("config.toml");

  let mut file = File::create(&config_path).expect("should create config file");
  writeln!(
    file,
    r#"
[language_aliases]
typescript = ["ts"]
rust = ["ts"]
"#
  )
  .expect("should write config file");

  let err = pruner::config::load(pruner::config::LoadOpts {
    config_path: Some(config_path),
    profiles: Vec::new(),
  })
  .unwrap_err();

  assert!(
    err.to_string().contains("Language alias 'ts' conflicts"),
    "Unexpected error: {err}"
  );
}
