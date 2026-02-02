use anyhow::Result;
use regex::Regex;
use std::{
  borrow::Cow,
  collections::{HashMap, HashSet},
  ops::Deref,
};
use tree_sitter::{
  Parser, Point, QueryCursor, QueryPredicate, QueryPredicateArg, QueryProperty, Range,
  StreamingIterator,
};

use super::grammar::Grammar;

pub fn get_lang_name(properties: &[QueryProperty]) -> Option<String> {
  for property in properties {
    if property.key.deref() == "injection.language" {
      return property.value.clone().map(String::from);
    }
  }
  None
}

#[derive(Debug)]
struct RangeOffset {
  start_row: isize,
  start_col: isize,
  end_row: isize,
  end_col: isize,
}

fn parse_offset_predicate(pred: &QueryPredicate) -> Result<(u32, RangeOffset)> {
  if pred.args.len() != 5 {
    anyhow::bail!("Offset predicate requires 5 arguments");
  }

  let [QueryPredicateArg::Capture(capture), QueryPredicateArg::String(start_row), QueryPredicateArg::String(start_col), QueryPredicateArg::String(end_row), QueryPredicateArg::String(end_col)] =
    pred.args.deref()
  else {
    anyhow::bail!("Offset predicate contained unexpected arguments");
  };

  let range = RangeOffset {
    start_row: start_row.parse()?,
    start_col: start_col.parse()?,
    end_row: end_row.parse()?,
    end_col: end_col.parse()?,
  };

  Ok((*capture, range))
}

fn get_offset_modifiers(predicates: &[QueryPredicate]) -> HashMap<u32, RangeOffset> {
  let mut map = HashMap::new();
  for pred in predicates {
    if pred.operator.deref() != "offset!" {
      continue;
    }

    let Ok((capture, range)) = parse_offset_predicate(pred) else {
      continue;
    };

    map.insert(capture, range);
  }

  map
}

fn parse_escape_predicate(pred: &QueryPredicate) -> Result<(u32, HashSet<String>)> {
  if pred.args.len() < 2 {
    anyhow::bail!("Escape predicate requires at least 2 arguments");
  }

  let QueryPredicateArg::Capture(capture) = pred.args[0] else {
    anyhow::bail!("Escape predicate requires capture as first argument");
  };

  let mut escape_chars = HashSet::new();
  for arg in pred.args.iter().skip(1) {
    let QueryPredicateArg::String(value) = arg else {
      anyhow::bail!("Escape predicate only supports string arguments");
    };
    escape_chars.insert(value.to_string());
  }

  Ok((capture, escape_chars))
}

fn get_escape_modifiers(predicates: &[QueryPredicate]) -> HashMap<u32, HashSet<String>> {
  let mut map: HashMap<u32, HashSet<String>> = HashMap::new();
  for pred in predicates {
    if pred.operator.deref() != "escape!" {
      continue;
    }

    let Ok((capture, escape_chars)) = parse_escape_predicate(pred) else {
      continue;
    };

    map.entry(capture).or_default().extend(escape_chars);
  }

  map
}

fn parse_gsub_predicate(pred: &QueryPredicate) -> Result<(u32, String, String)> {
  if pred.args.len() != 3 {
    anyhow::bail!("Gsub predicate requires 3 arguments");
  }

  let [QueryPredicateArg::Capture(capture), QueryPredicateArg::String(pattern), QueryPredicateArg::String(replacement)] =
    pred.args.deref()
  else {
    anyhow::bail!("Gsub predicate contained unexpected arguments");
  };

  Ok((*capture, pattern.to_string(), replacement.to_string()))
}

fn get_gsub_modifiers(predicates: &[QueryPredicate]) -> HashMap<u32, Vec<(String, String)>> {
  let mut map: HashMap<u32, Vec<(String, String)>> = HashMap::new();
  for pred in predicates {
    if pred.operator.deref() != "gsub!" {
      continue;
    }

    let Ok((capture, pattern, replacement)) = parse_gsub_predicate(pred) else {
      continue;
    };

    map.entry(capture).or_default().push((pattern, replacement));
  }

  map
}

fn lua_replacement_to_regex(repl: &str) -> String {
  // Lua `string.gsub` uses `%1`..`%9` (and `%0`) for capture references and `%%` for a literal `%`.
  // Rust `regex` uses `$1`..`$9` (and `$0`) for capture references and `$$` for a literal `$`.
  let mut out = String::with_capacity(repl.len());
  let mut chars = repl.chars().peekable();

  while let Some(c) = chars.next() {
    match c {
      '$' => out.push_str("$$"),
      '%' => {
        let Some(next) = chars.next() else {
          out.push('%');
          continue;
        };

        match next {
          '%' => out.push('%'),
          d if d.is_ascii_digit() => {
            out.push('$');
            out.push(d);
          }
          other => {
            // Treat `%x` as escaping `x`.
            if other == '$' {
              out.push_str("$$")
            } else {
              out.push(other)
            }
          }
        }
      }
      other => out.push(other),
    }
  }

  out
}

fn apply_gsub_modifiers(text: &str, modifiers: &[(String, String)]) -> String {
  let mut out = text.to_owned();

  for (lua_pattern, lua_replacement) in modifiers {
    let Ok(ast) = lua_pattern::parse(lua_pattern) else {
      continue;
    };
    let Ok(re_src) = lua_pattern::try_to_regex(&ast, false, false) else {
      continue;
    };
    let Ok(re) = Regex::new(&re_src) else {
      continue;
    };

    let repl = lua_replacement_to_regex(lua_replacement);
    out = re.replace_all(&out, repl.as_str()).into_owned();
  }

  out
}

fn point_to_byte(source: &str, point: Point) -> Option<usize> {
  let mut byte_index = 0;

  for (current_row, line) in source.split_inclusive('\n').enumerate() {
    if current_row == point.row {
      let col_byte = point.column.min(line.len());
      return Some(byte_index + col_byte);
    }

    byte_index += line.len();
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

fn calculate_point_offset(value: usize, offset: isize) -> usize {
  ((value as isize) + offset) as usize
}

fn apply_offset_to_range(source: &str, range: &Range, offset: &RangeOffset) -> Range {
  let new_start_point = Point {
    row: calculate_point_offset(range.start_point.row, offset.start_row),
    column: calculate_point_offset(range.start_point.column, offset.start_col),
  };
  let new_end_point = Point {
    row: calculate_point_offset(range.end_point.row, offset.end_row),
    column: calculate_point_offset(range.end_point.column, offset.end_col),
  };

  let new_start_byte = point_to_byte(source, new_start_point).unwrap();
  let new_end_byte = point_to_byte(source, new_end_point).unwrap();

  Range {
    start_byte: new_start_byte,
    end_byte: new_end_byte,
    start_point: new_start_point,
    end_point: new_end_point,
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

  while let Some(query_match) = matches.next() {
    let harcoded_lang_name = get_lang_name(query.property_settings(query_match.pattern_index));
    let is_hardcoded_lang = harcoded_lang_name.is_some();

    let mut lang_capture = None;
    let mut content_capture = None;
    for capture in query_match.captures {
      if let Some(lang_capture_index) = lang_capture_index {
        if capture.index == lang_capture_index {
          lang_capture = Some(capture);
        }
      }
      if capture.index == content_capture_index {
        content_capture = Some(capture);
      }
    }

    let Some(content_capture) = content_capture else {
      continue;
    };

    let predicates = query.general_predicates(query_match.pattern_index);
    let offset_modifiers = get_offset_modifiers(predicates);
    let escape_modifiers = get_escape_modifiers(predicates);
    let gsub_modifiers = get_gsub_modifiers(predicates);

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
        if let Some(modifiers) = gsub_modifiers.get(&lang_capture_index) {
          lang_name = apply_gsub_modifiers(&lang_name, modifiers);
        }
      }
    }

    let range = if let Some(offset) = offset_modifiers.get(&content_capture.index) {
      apply_offset_to_range(&source_str, &content_capture.node.range(), offset)
    } else {
      content_capture.node.range()
    };

    let escape_chars = escape_modifiers
      .get(&content_capture.index)
      .cloned()
      .unwrap_or_default();

    injected_regions.push(InjectedRegion {
      lang: lang_name.clone(),
      range: remap_range_for_appended_newline(range, &original_endpoint),
      opts: InjectionOpts { escape_chars },
    });
  }

  Ok(injected_regions)
}
