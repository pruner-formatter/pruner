use anyhow::{Context, Result};
use rayon::prelude::*;
use std::{fs, path::Path};
use tree_sitter::Parser;

use crate::{
  api::{self, grammar::Grammars, text},
  config::{FormatterSpecs, LanguageFormatters},
  wasm::formatter::WasmFormatter,
};

mod runner;
pub use runner::FormatOpts;

pub struct FormatContext<'a> {
  pub grammars: &'a Grammars,
  pub languages: &'a LanguageFormatters,
  pub language_aliases: &'a std::collections::HashMap<String, String>,
  pub formatters: &'a FormatterSpecs,
  pub wasm_formatter: &'a WasmFormatter,
}

#[derive(Debug, Clone)]
pub struct FormatFileChange {
  pub path: String,
  pub original: Vec<u8>,
  pub formatted: Vec<u8>,
}

pub fn format(
  source: &[u8],
  opts: &FormatOpts,
  format_root: bool,
  is_root: bool,
  format_context: &FormatContext,
) -> Result<Vec<u8>> {
  let mut parser = Parser::new();
  let language = format_context
    .language_aliases
    .get(opts.language)
    .map(|s| s.as_str())
    .unwrap_or(opts.language);

  let mut formatted_result = Vec::from(source);

  if !is_root || format_root {
    for format_spec in format_context
      .languages
      .get(language)
      .unwrap_or(&Vec::new())
    {
      if (is_root && format_spec.run_in_root()) || (!is_root && format_spec.run_in_injections()) {
        let formatter_name = format_spec.formatter();

        formatted_result = if let Some(formatter) = format_context.formatters.get(formatter_name) {
          runner::format(
            formatter,
            &formatted_result,
            &FormatOpts { language, ..*opts },
          )
          .context(format!("Failed to run formatter: {formatter_name}"))?
        } else if format_context.wasm_formatter.has_formatter(formatter_name) {
          format_context
            .wasm_formatter
            .format(formatter_name, &formatted_result, opts)?
        } else {
          formatted_result
        }
      }
    }
  }

  let Some(grammar) = format_context.grammars.get(language) else {
    return Ok(formatted_result);
  };

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

      let mut indent = text::column_for_byte(&formatted_result, region.range.start_byte);
      let mut indent_from_content = false;
      let mut normalized_source = unescaped_source_str;
      if indent > 0 {
        normalized_source = text::strip_leading_indent(&normalized_source, indent);
      } else {
        let min_indent = text::min_leading_indent(&normalized_source);
        if min_indent > 0 {
          normalized_source = text::strip_leading_indent(&normalized_source, min_indent);
          indent = min_indent;
          indent_from_content = true;
        }
      }

      let unescaped_source = normalized_source.into_bytes();
      let trailing_newlines = if !indent_from_content && indent > 0 {
        text::trailing_newlines_with_indent(source_slice, indent)
      } else {
        text::trailing_newlines(source_slice)
      };
      let adjusted_printwidth = opts.printwidth.saturating_sub(indent as u32);
      let mut formatted_sub_result = format(
        &unescaped_source,
        &FormatOpts {
          printwidth: adjusted_printwidth.max(1),
          language: &region.lang,
          source_file: opts.source_file,
        },
        format_root,
        false,
        format_context,
      )?;
      if !escape_chars.is_empty() {
        let formatted_str = String::from_utf8(formatted_sub_result)?;
        formatted_sub_result = text::escape_text(&formatted_str, &escape_chars).into_bytes();
      }

      text::strip_trailing_newlines(&mut formatted_sub_result);
      formatted_sub_result.extend_from_slice(&trailing_newlines);
      if indent_from_content && indent > 0 {
        if formatted_sub_result.first() != Some(&b'\n')
          && formatted_sub_result.first() != Some(&b'\r')
        {
          let spaces = vec![b' '; indent];
          formatted_sub_result.splice(0..0, spaces);
        }
      }
      text::offset_lines(&mut formatted_sub_result, indent);

      if !indent_from_content && indent > 0 && formatted_sub_result.last() == Some(&b'\n') {
        let spaces = vec![b' '; indent];
        formatted_sub_result.extend_from_slice(&spaces);
      }

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
) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
  let content = fs::read(file).context("Failed to read temp file after formatting")?;

  let result = format(
    &content,
    &FormatOpts {
      printwidth: opts.printwidth,
      language: opts.language,
      source_file: Some(file),
    },
    !skip_root,
    true,
    format_context,
  )
  .context("Failed to format file contents")?;

  if result == content {
    return Ok(None);
  }

  if write {
    fs::write(file, &result).context("Failed to write formatted contents to file")?;
  }

  Ok(Some((content, result)))
}

pub fn format_files<'a>(
  dir: &Path,
  include_glob: &str,
  exclude_globs: Option<Vec<String>>,

  write: bool,

  opts: &'a FormatOpts,
  skip_root: bool,
  format_context: &'a FormatContext,
) -> Result<impl rayon::iter::ParallelIterator<Item = Result<FormatFileChange>> + 'a> {
  let include_matcher = globset::Glob::new(include_glob)?.compile_matcher();

  let mut exclude_glob_builder = globset::GlobSetBuilder::new();
  for glob in exclude_globs.unwrap_or_default() {
    exclude_glob_builder.add(globset::Glob::new(&glob)?);
  }

  let exclude_matcher = exclude_glob_builder.build()?;

  let walker = ignore::WalkBuilder::new(dir).current_dir(dir).build();
  Ok(
    walker
      .filter_map(|entry| entry.ok())
      .filter(|entry| !entry.path().is_dir())
      .filter(move |entry| {
        include_matcher.is_match(entry.path()) && !exclude_matcher.is_match(entry.path())
      })
      .par_bridge()
      .filter_map(move |entry| {
        let path = entry.path().to_path_buf();
        match format_file(&path, write, opts, skip_root, format_context) {
          Err(err) => {
            let path_str = path.to_string_lossy();
            Some(Err(err).context(format!("Failed to format file {path_str}")))
          }
          Ok(Some((original, formatted))) => {
            let path = path.to_string_lossy();
            Some(Ok(FormatFileChange {
              path: String::from(path),
              original,
              formatted,
            }))
          }
          Ok(None) => None,
        }
      }),
  )
}
