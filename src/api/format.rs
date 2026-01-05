use anyhow::{Context, Result};
use rayon::prelude::*;
use std::{fs, path::Path};
use tree_sitter::Parser;

use crate::{
  api::{self, grammar::Grammars, text},
  config::{FormatterSpecs, LanguageFormatters},
};

mod runner;
pub use runner::FormatOpts;

pub struct FormatContext<'a> {
  pub grammars: &'a Grammars,
  pub languages: &'a LanguageFormatters,
  pub formatters: &'a FormatterSpecs,
}

pub fn format(
  source: &[u8],
  opts: &FormatOpts,
  skip_root: bool,
  format_context: &FormatContext,
) -> Result<Vec<u8>> {
  let mut parser = Parser::new();

  let Some(grammar) = format_context.grammars.get(opts.language) else {
    return Ok(Vec::from(source));
  };

  let mut formatted_result = Vec::from(source);

  if !skip_root {
    if let Some(language_formatter_specs) = format_context.languages.get(opts.language) {
      if let Some(formatter_name) = language_formatter_specs.first() {
        if let Some(formatter) = format_context.formatters.get(formatter_name) {
          formatted_result = runner::format(formatter, &formatted_result, opts)?;
        }
      }
    }
  }

  let mut injected_regions =
    api::injections::extract_language_injections(&mut parser, grammar, &formatted_result)?;
  // Sort in reverse order. File modifications can therefore be applied from end to start
  injected_regions.sort_by(|a, b| b.range.start_byte.cmp(&a.range.start_byte));

  let formatted_regions = injected_regions
    .par_iter()
    .map(|region| {
      let source_slice = &formatted_result[region.range.start_byte..region.range.end_byte];
      let escape_chars = text::sort_escape_chars(&region.opts.escape_chars);
      let source_str = String::from_utf8(Vec::from(source_slice))?;
      let unescaped_source_str = if escape_chars.is_empty() {
        source_str
      } else {
        text::unescape_text(&source_str, &escape_chars)
      };

      let mut indent = text::column_for_byte(source, region.range.start_byte);
      let mut normalized_source = unescaped_source_str;
      if indent > 0 {
        normalized_source = text::strip_leading_indent(&normalized_source, indent);
      } else {
        let min_indent = text::min_leading_indent(&normalized_source);
        if min_indent > 0 {
          normalized_source = text::strip_leading_indent(&normalized_source, min_indent);
          indent = min_indent;
        }
      }

      let unescaped_source = normalized_source.into_bytes();
      let adjusted_printwidth = opts.printwidth.saturating_sub(indent as u32);
      let mut formatted_sub_result = format(
        &unescaped_source,
        &FormatOpts {
          printwidth: adjusted_printwidth.max(1),
          language: &region.lang,
        },
        false,
        format_context,
      )?;
      if !escape_chars.is_empty() {
        let formatted_str = String::from_utf8(formatted_sub_result)?;
        formatted_sub_result = text::escape_text(&formatted_str, &escape_chars).into_bytes();
      }
      let has_trailing_newline = source_slice.ends_with(b"\n");
      text::trim_trailing_whitespace(&mut formatted_sub_result, has_trailing_newline);
      text::offset_lines(&mut formatted_sub_result, indent);
      Ok((region.clone(), formatted_sub_result))
    })
    .collect::<Vec<Result<(api::injections::InjectedRegion, Vec<u8>)>>>();

  let mut region_results = Vec::with_capacity(formatted_regions.len());
  for result in formatted_regions {
    region_results.push(result?);
  }

  region_results.sort_by(|(a, _), (b, _)| b.range.start_byte.cmp(&a.range.start_byte));

  for (region, formatted_sub_result) in region_results {
    formatted_result.splice(
      region.range.start_byte..region.range.end_byte,
      formatted_sub_result,
    );
  }

  Ok(formatted_result)
}

pub fn format_file(
  file: &Path,
  write: bool,
  opts: &FormatOpts,
  skip_root: bool,
  format_context: &FormatContext,
) -> Result<bool> {
  let content = fs::read(file).context("Failed to read temp file after formatting")?;

  let result =
    format(&content, opts, skip_root, format_context).context("Failed to format file contents")?;

  if result == content {
    return Ok(false);
  }

  if write {
    fs::write(file, &result).context("Failed to write formatted contents to file")?;
  }

  Ok(true)
}

pub fn format_files(
  dir: &Path,
  include_glob: &str,
  exclude_globs: Option<Vec<String>>,

  write: bool,

  opts: &FormatOpts,
  skip_root: bool,
  format_context: &FormatContext,
) -> Result<Vec<String>> {
  let include_matcher = globset::Glob::new(include_glob)?.compile_matcher();

  let mut exclude_glob_builder = globset::GlobSetBuilder::new();
  for glob in exclude_globs.unwrap_or_default() {
    exclude_glob_builder.add(globset::Glob::new(&glob)?);
  }

  let exclude_matcher = exclude_glob_builder.build()?;

  let walker = ignore::WalkBuilder::new(dir).current_dir(dir).build();
  walker
    .filter_map(|entry| entry.ok())
    .filter(|entry| !entry.path().is_dir())
    .filter(|entry| {
      include_matcher.is_match(entry.path()) && !exclude_matcher.is_match(entry.path())
    })
    .par_bridge()
    .filter_map(
      |entry| match format_file(entry.path(), write, opts, skip_root, format_context) {
        Err(err) => {
          log::error!(
            "Failed to format file {}: {err}",
            entry.path().to_string_lossy()
          );
          Some(Err(err))
        }
        Ok(true) => {
          let path = entry.path().to_string_lossy();
          log::info!("{path}");
          Some(Ok(String::from(path)))
        }
        Ok(false) => None,
      },
    )
    .collect::<Result<Vec<String>>>()
}
