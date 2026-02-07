use anyhow::Result;
use std::{
  borrow::Cow,
  collections::{HashMap, HashSet},
};
use tree_sitter::{Node, Parser, Point, QueryCursor, QueryProperty, Range, StreamingIterator};

use super::{
  directives::{escape, gsub, indented, offset, trim},
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

fn is_combined(properties: &[QueryProperty]) -> bool {
  properties
    .iter()
    .any(|property| property.key.as_ref() == "injection.combined")
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

fn container_range_for_content(content_node: Node) -> Range {
  content_node
    .parent()
    .map(|node| node.range())
    .unwrap_or_else(|| content_node.range())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CombinedKey {
  pattern_index: usize,
  lang: String,
  container_start: usize,
  container_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum GroupKey {
  Combined(CombinedKey),
  Single(u64),
}

#[derive(Debug, Clone)]
struct InjectedRegionFragment {
  pattern_index: usize,
  lang: String,
  start_byte: usize,
  end_byte: usize,
  escape_chars: HashSet<String>,
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

  let mut fragments: HashMap<GroupKey, InjectedRegionFragment> = HashMap::new();
  let mut fragment_key_order: Vec<GroupKey> = Vec::new();
  let mut single_key_counter: u64 = 0;

  let query = &grammar.injections;

  let mut cursor = QueryCursor::new();
  let mut matches = cursor.matches(query, tree.root_node(), source_with_newline.as_ref());

  let lang_capture_index = query.capture_index_for_name("injection.language");
  let Some(content_capture_index) = query.capture_index_for_name("injection.content") else {
    return Ok(Vec::new());
  };

  let mut directives_cache: HashMap<
    usize,
    (
      HashMap<u32, offset::RangeOffset>,
      HashMap<u32, HashSet<String>>,
      HashMap<u32, Vec<gsub::GsubRule>>,
      HashMap<u32, trim::TrimSpec>,
    ),
  > = HashMap::new();

  while let Some(query_match) = matches.next() {
    let pattern_properties = query.property_settings(query_match.pattern_index);
    let harcoded_lang_name = get_lang_name(pattern_properties);
    let is_hardcoded_lang = harcoded_lang_name.is_some();
    let is_combined = is_combined(pattern_properties);

    let mut lang_capture = None;
    let mut content_captures = Vec::new();
    for capture in query_match.captures {
      if let Some(lang_capture_index) = lang_capture_index
        && capture.index == lang_capture_index
      {
        lang_capture = Some(capture);
      }
      if capture.index == content_capture_index {
        content_captures.push(capture);
      }
    }

    if content_captures.is_empty() {
      continue;
    };

    let (offset_modifiers, escape_modifiers, gsub_modifiers, trim_modifiers) = directives_cache
      .entry(query_match.pattern_index)
      .or_insert_with(|| {
        let predicates = query.general_predicates(query_match.pattern_index);
        (
          offset::collect(predicates),
          escape::collect(predicates),
          gsub::collect(predicates),
          trim::collect(predicates),
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

    if !is_hardcoded_lang && let Some(lang_capture_index) = lang_capture_index {
      lang_name = gsub::apply_gsub(gsub_modifiers, lang_capture_index, &lang_name);
    }

    for content_capture in content_captures {
      let base_range = content_capture.node.range();
      let mut range = if let Some(offset) = offset_modifiers.get(&content_capture.index) {
        offset::apply_offset_to_range(&source_str, &base_range, offset).unwrap_or(base_range)
      } else {
        base_range
      };

      if let Some(trim_spec) = trim_modifiers.get(&content_capture.index) {
        let (start_byte, end_byte) = trim::apply_trim(
          source_with_newline.as_ref(),
          range.start_byte,
          range.end_byte,
          *trim_spec,
        );
        range.start_byte = start_byte;
        range.end_byte = end_byte;
      }

      let escape_chars = escape::escape_chars(escape_modifiers, content_capture.index);

      let key = if is_combined {
        let container_range = container_range_for_content(content_capture.node);
        GroupKey::Combined(CombinedKey {
          pattern_index: query_match.pattern_index,
          lang: lang_name.clone(),
          container_start: container_range.start_byte,
          container_end: container_range.end_byte,
        })
      } else {
        let key = GroupKey::Single(single_key_counter);
        single_key_counter += 1;
        key
      };

      match fragments.entry(key.clone()) {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
          let fragment = entry.get_mut();
          fragment.start_byte = fragment.start_byte.min(range.start_byte);
          fragment.end_byte = fragment.end_byte.max(range.end_byte);
          fragment.escape_chars.extend(escape_chars.iter().cloned());
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
          fragment_key_order.push(key);
          entry.insert(InjectedRegionFragment {
            pattern_index: query_match.pattern_index,
            lang: lang_name.clone(),
            start_byte: range.start_byte,
            end_byte: range.end_byte,
            escape_chars,
          });
        }
      }
    }
  }

  let mut injected_regions: Vec<InjectedRegion> = Vec::with_capacity(fragments.len());
  for key in fragment_key_order {
    let Some(fragment) = fragments.remove(&key) else {
      continue;
    };
    let mut range = Range {
      start_byte: fragment.start_byte,
      end_byte: fragment.end_byte,
      start_point: point_for_byte(source_with_newline.as_ref(), fragment.start_byte),
      end_point: point_for_byte(source_with_newline.as_ref(), fragment.end_byte),
    };

    let props = query.property_settings(fragment.pattern_index);
    if indented::is_indented(props) {
      range = trim_indented_range(source_with_newline.as_ref(), range);
    }

    injected_regions.push(InjectedRegion {
      lang: fragment.lang,
      range: remap_range_for_appended_newline(range, &original_endpoint),
      opts: InjectionOpts {
        escape_chars: fragment.escape_chars,
      },
    });
  }

  Ok(injected_regions)
}
