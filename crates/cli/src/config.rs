use anyhow::{Context, Result};
use std::{
  collections::HashMap,
  hash::Hash,
  path::{Path, PathBuf},
};
use url::Url;

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum GrammarSpec {
  Url(Url),
  Table { url: Url, rev: Option<String> },
}

impl GrammarSpec {
  pub fn url(&self) -> &Url {
    match self {
      GrammarSpec::Url(url) => url,
      GrammarSpec::Table { url, .. } => url,
    }
  }

  pub fn rev(&self) -> Option<&str> {
    match self {
      GrammarSpec::Url(_) => None,
      GrammarSpec::Table { rev, .. } => match rev {
        Some(rev) => Some(rev),
        None => None,
      },
    }
  }
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FormatterSpec {
  pub cmd: String,
  pub args: Vec<String>,
  pub stdin: Option<bool>,
  pub fail_on_stderr: Option<bool>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum PluginSpec {
  Url(Url),
  Table { url: Url },
}

impl PluginSpec {
  pub fn url(&self) -> &Url {
    match self {
      Self::Url(url) => url,
      Self::Table { url, .. } => url,
    }
  }
}

pub type FormatterSpecs = HashMap<String, FormatterSpec>;
pub type PluginSpecs = HashMap<String, PluginSpec>;
pub type GrammarSpecs = HashMap<String, GrammarSpec>;

fn default_resource() -> bool {
  true
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum LanguageFormatSpec {
  String(String),
  Table {
    formatter: String,

    #[serde(default = "default_resource")]
    run_in_root: bool,
    #[serde(default = "default_resource")]
    run_in_injections: bool,
  },
}
impl LanguageFormatSpec {
  pub fn formatter(&self) -> &str {
    match self {
      Self::String(formatter) => formatter,
      Self::Table { formatter, .. } => formatter,
    }
  }
  pub fn run_in_root(&self) -> bool {
    match self {
      Self::String(_) => true,
      Self::Table { run_in_root, .. } => *run_in_root,
    }
  }
  pub fn run_in_injections(&self) -> bool {
    match self {
      Self::String(_) => true,
      Self::Table {
        run_in_injections, ..
      } => *run_in_injections,
    }
  }
}

impl From<String> for LanguageFormatSpec {
  fn from(value: String) -> Self {
    LanguageFormatSpec::String(value)
  }
}

impl From<&str> for LanguageFormatSpec {
  fn from(value: &str) -> Self {
    LanguageFormatSpec::String(value.into())
  }
}

pub type LanguageFormatSpecs = Vec<LanguageFormatSpec>;
pub type LanguageFormatters = HashMap<String, LanguageFormatSpecs>;
pub type LanguageAliasSpecs = HashMap<String, Vec<String>>;

/// Profile-specific configuration overrides.
/// Has the same fields as ConfigFile (except profiles) to allow full override capability.
#[derive(serde::Deserialize, Debug, Default, Clone)]
pub struct ProfileConfig {
  pub query_paths: Option<Vec<PathBuf>>,
  pub grammar_paths: Option<Vec<PathBuf>>,

  pub grammar_download_dir: Option<PathBuf>,
  pub grammar_build_dir: Option<PathBuf>,

  pub grammars: Option<GrammarSpecs>,
  pub languages: Option<LanguageFormatters>,
  pub language_aliases: Option<LanguageAliasSpecs>,
  pub formatters: Option<FormatterSpecs>,
  pub plugins: Option<PluginSpecs>,
}

impl ProfileConfig {
  fn absolutize_paths(mut self, base_dir: &Path) -> Self {
    self.query_paths = self
      .query_paths
      .map(|paths| absolutize_vec(paths, base_dir));
    self.grammar_paths = self
      .grammar_paths
      .map(|paths| absolutize_vec(paths, base_dir));
    self.grammar_download_dir = self
      .grammar_download_dir
      .map(|path| absolutize_path(path, base_dir));
    self.grammar_build_dir = self
      .grammar_build_dir
      .map(|path| absolutize_path(path, base_dir));

    self
  }
}

/// Represents the on-disk configuration format. All fields are optional
/// to allow partial configs that get merged together.
#[derive(serde::Deserialize, Debug, Default, Clone)]
pub struct ConfigFile {
  pub query_paths: Option<Vec<PathBuf>>,
  pub grammar_paths: Option<Vec<PathBuf>>,

  pub grammar_download_dir: Option<PathBuf>,
  pub grammar_build_dir: Option<PathBuf>,

  pub grammars: Option<GrammarSpecs>,
  pub languages: Option<LanguageFormatters>,
  pub language_aliases: Option<LanguageAliasSpecs>,
  pub formatters: Option<FormatterSpecs>,
  pub plugins: Option<PluginSpecs>,

  pub profiles: Option<HashMap<String, ProfileConfig>>,
}

/// The fully resolved configuration with all defaults applied.
/// Used by the rest of the application.
#[derive(Debug, Clone)]
pub struct Config {
  pub query_paths: Vec<PathBuf>,
  pub grammar_paths: Vec<PathBuf>,

  pub grammar_download_dir: PathBuf,
  pub grammar_build_dir: PathBuf,
  pub cache_dir: PathBuf,

  pub grammars: GrammarSpecs,
  pub languages: LanguageFormatters,
  pub language_aliases: HashMap<String, String>,
  pub formatters: FormatterSpecs,
  pub plugins: PluginSpecs,
}

fn absolutize_vec(paths: Vec<PathBuf>, base_dir: &Path) -> Vec<PathBuf> {
  paths
    .into_iter()
    .map(|path| absolutize_path(path, base_dir))
    .collect()
}

fn absolutize_path(path: PathBuf, base_dir: &Path) -> PathBuf {
  if path.is_absolute() {
    path
  } else {
    base_dir.join(path)
  }
}

fn merge_vecs<T: Clone>(base: &Option<Vec<T>>, overlay: &Option<Vec<T>>) -> Option<Vec<T>> {
  match (base, overlay) {
    (None, None) => None,
    (Some(values), None) | (None, Some(values)) => Some(values.clone()),
    (Some(base_values), Some(overlay_values)) => {
      let mut merged = base_values.clone();
      merged.extend(overlay_values.clone());
      Some(merged)
    }
  }
}

fn merge_maps<K: Eq + Hash + Clone, V: Clone>(
  base: &Option<HashMap<K, V>>,
  overlay: &Option<HashMap<K, V>>,
) -> Option<HashMap<K, V>> {
  match (base, overlay) {
    (None, None) => None,
    (Some(values), None) | (None, Some(values)) => Some(values.clone()),
    (Some(base_values), Some(overlay_values)) => {
      let mut merged = base_values.clone();
      merged.extend(overlay_values.clone());
      Some(merged)
    }
  }
}

impl ConfigFile {
  pub fn from_file(path: &Path) -> Result<Self> {
    let content = std::fs::read_to_string(path)?;
    let config: ConfigFile = toml::from_str(&content)?;
    Ok(config.absolutize_paths(path.parent()))
  }

  pub fn merge(base: &ConfigFile, overlay: &ConfigFile) -> ConfigFile {
    ConfigFile {
      query_paths: merge_vecs(&base.query_paths, &overlay.query_paths),
      grammar_paths: merge_vecs(&base.grammar_paths, &overlay.grammar_paths),
      grammar_download_dir: overlay
        .grammar_download_dir
        .clone()
        .or_else(|| base.grammar_download_dir.clone()),
      grammar_build_dir: overlay
        .grammar_build_dir
        .clone()
        .or_else(|| base.grammar_build_dir.clone()),
      grammars: merge_maps(&base.grammars, &overlay.grammars),
      languages: merge_maps(&base.languages, &overlay.languages),
      language_aliases: merge_maps(&base.language_aliases, &overlay.language_aliases),
      formatters: merge_maps(&base.formatters, &overlay.formatters),
      plugins: merge_maps(&base.plugins, &overlay.plugins),
      profiles: merge_maps(&base.profiles, &overlay.profiles),
    }
  }

  pub fn apply_profile(self, profile: &ProfileConfig) -> ConfigFile {
    ConfigFile {
      query_paths: merge_vecs(&self.query_paths, &profile.query_paths),
      grammar_paths: merge_vecs(&self.grammar_paths, &profile.grammar_paths),
      grammar_download_dir: profile
        .grammar_download_dir
        .clone()
        .or(self.grammar_download_dir),
      grammar_build_dir: profile.grammar_build_dir.clone().or(self.grammar_build_dir),
      grammars: merge_maps(&self.grammars, &profile.grammars),
      languages: merge_maps(&self.languages, &profile.languages),
      language_aliases: merge_maps(&self.language_aliases, &profile.language_aliases),
      formatters: merge_maps(&self.formatters, &profile.formatters),
      plugins: merge_maps(&self.plugins, &profile.plugins),
      profiles: self.profiles,
    }
  }

  fn absolutize_paths(mut self, base_dir: Option<&Path>) -> Self {
    let Some(base_dir) = base_dir else {
      return self;
    };

    self.query_paths = self
      .query_paths
      .map(|paths| absolutize_vec(paths, base_dir));
    self.grammar_paths = self
      .grammar_paths
      .map(|paths| absolutize_vec(paths, base_dir));
    self.grammar_download_dir = self
      .grammar_download_dir
      .map(|path| absolutize_path(path, base_dir));
    self.grammar_build_dir = self
      .grammar_build_dir
      .map(|path| absolutize_path(path, base_dir));
    self.profiles = self.profiles.map(|profiles| {
      profiles
        .into_iter()
        .map(|(name, profile)| (name, profile.absolutize_paths(base_dir)))
        .collect()
    });

    self
  }
}

fn find_local_config(start_dir: &Path) -> Option<PathBuf> {
  for ancestor in start_dir.ancestors() {
    let candidate = ancestor.join("pruner.toml");
    if candidate.is_file() {
      return Some(candidate);
    }
  }
  None
}

fn load_config_file(config_path: Option<PathBuf>) -> Result<ConfigFile> {
  let cwd = std::env::current_dir()?;

  if let Some(path) = config_path {
    return ConfigFile::from_file(&cwd.join(path));
  }

  let xdg_dirs = xdg::BaseDirectories::with_prefix("pruner");
  let config_path = xdg_dirs.find_config_file("config.toml");
  let global_config = match config_path.as_deref() {
    Some(config_path) => ConfigFile::from_file(config_path)
      .with_context(|| format!("Failed to load config {:?}", config_path))?,
    None => ConfigFile::default(),
  };

  let local_config_path = find_local_config(&cwd);
  let local_config = match local_config_path.as_deref() {
    Some(local_config_path) => ConfigFile::from_file(local_config_path)
      .with_context(|| format!("Failed to load config {:?}", local_config_path))?,
    None => ConfigFile::default(),
  };

  Ok(ConfigFile::merge(&global_config, &local_config))
}

pub struct LoadOpts {
  pub config_path: Option<PathBuf>,
  pub profiles: Vec<String>,
}

pub fn load(opts: LoadOpts) -> Result<Config> {
  let xdg_dirs = xdg::BaseDirectories::with_prefix("pruner");
  let mut config_file = load_config_file(opts.config_path)?;

  for profile_name in &opts.profiles {
    let profile = config_file
      .profiles
      .as_ref()
      .and_then(|p| p.get(profile_name))
      .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile_name))?
      .clone();
    config_file = config_file.apply_profile(&profile);
  }

  let mut alias_to_canonical: HashMap<String, String> = HashMap::new();
  for (canonical, aliases) in config_file.language_aliases.clone().unwrap_or_default() {
    for alias in aliases {
      if let Some(existing) = alias_to_canonical.get(&alias)
        && existing != &canonical
      {
        anyhow::bail!(
          "Language alias '{}' conflicts: maps to '{}' and '{}'",
          alias,
          existing,
          canonical
        );
      }
      alias_to_canonical.insert(alias, canonical.clone());
    }
  }

  Ok(Config {
    query_paths: config_file.query_paths.unwrap_or_default(),
    grammar_paths: config_file.grammar_paths.unwrap_or_default(),
    grammar_download_dir: config_file
      .grammar_download_dir
      .unwrap_or(xdg_dirs.place_data_file("grammars")?),
    grammar_build_dir: config_file
      .grammar_build_dir
      .unwrap_or(xdg_dirs.place_data_file("build")?),
    cache_dir: xdg_dirs.place_data_file("cache")?,
    grammars: config_file.grammars.unwrap_or_default(),
    languages: config_file.languages.unwrap_or_default(),
    language_aliases: alias_to_canonical,
    formatters: config_file.formatters.unwrap_or_default(),
    plugins: config_file.plugins.unwrap_or_default(),
  })
}
