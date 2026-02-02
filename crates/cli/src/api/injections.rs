use anyhow::Result;
use std::{
  borrow::Cow,
  collections::{HashMap, HashSet},
};
use tree_sitter::{Parser, Point, QueryCursor, QueryProperty, Range, StreamingIterator};

use super::{
  directives::{escape, gsub, indented, offset},
  grammar::Grammar,
};

pub fn get_lang_name(properties: &[QueryProperty]) -> Option<String> {
  for property in properties {
    if property.key.as_ref() == "injection.language" {
      return property.value.clone().map(String::from);
    }
  }
  None
}

fn point_for_byte(source: &[u8], byte_index: usize) -> Point {
  let target = byte_index.min(source.len());
  let mut row = 0;
  let mut column = 0;

  for byte in source.iter().take(target) {
    if *byte == b'\n' {
      row += 1;
      column = 0;
    } else {
      column += 1;
    }
  }

  Point { row, column }
}

type EndPoint = (usize, Point);

fn with_newline<'a>(source: &'a [u8]) -> (Cow<'a, [u8]>, Option<EndPoint>) {
  let original_len = source.len();
  let should_append_newline = !source.ends_with(b"\n");
  let source_with_newline: Cow<[u8]> = if should_append_newline {
    let mut owned = Vec::with_capacity(original_len + 1);
    owned.extend_from_slice(source);
    owned.push(b'\n');
    Cow::Owned(owned)
  } else {
    Cow::Borrowed(source)
  };
  let original_endpoint =
    should_append_newline.then(|| (original_len, point_for_byte(source, original_len)));

  (source_with_newline, original_endpoint)
}

fn remap_range_for_appended_newline(range: Range, original_endpoint: &Option<EndPoint>) -> Range {
  let Some((end_byte, end_point)) = original_endpoint else {
    return range;
  };

  if range.end_byte < *end_byte {
    return range;
  }

  Range {
    start_byte: range.start_byte,
    start_point: range.start_point,
    end_byte: *end_byte,
    end_point: *end_point,
  }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InjectionOpts {
  pub escape_chars: HashSet<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InjectedRegion {
  pub range: Range,
  pub lang: String,
  pub opts: InjectionOpts,
}

fn trim_indented_range(source: &[u8], mut range: Range) -> Range {
  let (start_byte, end_byte) = indented::trim_bytes(source, range.start_byte, range.end_byte);

  range.start_byte = start_byte;
  range.end_byte = end_byte;
  range.start_point = point_for_byte(source, start_byte);
  range.end_point = point_for_byte(source, end_byte);
  range
}

pub fn extract_language_injections(
  parser: &mut Parser,
  grammar: &Grammar,
  source: &[u8],
) -> Result<Vec<InjectedRegion>> {
  let (source_with_newline, original_endpoint) = with_newline(source);
  let source_str = String::from_utf8(Vec::from(source_with_newline.as_ref()))?;

  parser.set_language(&grammar.lang)?;
  let tree = parser
    .parse(source_with_newline.as_ref(), None)
    .ok_or_else(|| anyhow::anyhow!("Parse returned None"))?;

  let mut injected_regions = Vec::new();

  let query = &grammar.injections;

  let mut cursor = QueryCursor::new();
  let mut matches = cursor.matches(query, tree.root_node(), source_with_newline.as_ref());

  let lang_capture_index = query.capture_index_for_name("injection.language");
  let Some(content_capture_index) = query.capture_index_for_name("injection.content") else {
    return Ok(injected_regions);
  };

  let mut directives_cache: HashMap<
    usize,
    (
      HashMap<u32, offset::RangeOffset>,
      HashMap<u32, HashSet<String>>,
      HashMap<u32, Vec<gsub::GsubRule>>,
    ),
  > = HashMap::new();

  while let Some(query_match) = matches.next() {
    let pattern_properties = query.property_settings(query_match.pattern_index);
    let harcoded_lang_name = get_lang_name(pattern_properties);
    let is_hardcoded_lang = harcoded_lang_name.is_some();
    let is_indented = indented::is_indented(pattern_properties);

    let mut lang_capture = None;
    let mut content_capture = None;
    for capture in query_match.captures {
      if let Some(lang_capture_index) = lang_capture_index
        && capture.index == lang_capture_index
      {
        lang_capture = Some(capture);
      }
      if capture.index == content_capture_index {
        content_capture = Some(capture);
      }
    }

    let Some(content_capture) = content_capture else {
      continue;
    };

    let (offset_modifiers, escape_modifiers, gsub_modifiers) = directives_cache
      .entry(query_match.pattern_index)
      .or_insert_with(|| {
        let predicates = query.general_predicates(query_match.pattern_index);
        (
          offset::collect(predicates),
          escape::collect(predicates),
          gsub::collect(predicates),
        )
      });

    let lang_capture_index = lang_capture.as_ref().map(|c| c.index);
    let Some(mut lang_name) = harcoded_lang_name.or_else(|| {
      lang_capture.and_then(|capture| {
        capture
          .node
          .utf8_text(source_with_newline.as_ref())
          .ok()
          .map(String::from)
      })
    }) else {
      continue;
    };

    if !is_hardcoded_lang {
      if let Some(lang_capture_index) = lang_capture_index {
        lang_name = gsub::apply_gsub(gsub_modifiers, lang_capture_index, &lang_name);
      }
    }

    let base_range = content_capture.node.range();
    let mut range = if let Some(offset) = offset_modifiers.get(&content_capture.index) {
      offset::apply_offset_to_range(&source_str, &base_range, offset).unwrap_or(base_range)
    } else {
      base_range
    };

    if is_indented {
      range = trim_indented_range(source_with_newline.as_ref(), range);
    }

    let escape_chars = escape::escape_chars(escape_modifiers, content_capture.index);

    injected_regions.push(InjectedRegion {
      lang: lang_name.clone(),
      range: remap_range_for_appended_newline(range, &original_endpoint),
      opts: InjectionOpts { escape_chars },
    });
  }

  Ok(injected_regions)
}
