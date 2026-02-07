use std::{collections::HashMap, ops::Deref};
use tree_sitter::{QueryPredicate, QueryPredicateArg};

#[derive(Debug, Clone, Copy)]
pub struct TrimSpec {
  pub start_linewise: bool,
  pub start_charwise: bool,
  pub end_linewise: bool,
  pub end_charwise: bool,
}

impl TrimSpec {
  fn default_end_linewise_only() -> Self {
    Self {
      start_linewise: false,
      start_charwise: false,
      end_linewise: true,
      end_charwise: false,
    }
  }
}

pub fn collect(predicates: &[QueryPredicate]) -> HashMap<u32, TrimSpec> {
  let mut map = HashMap::new();

  for pred in predicates {
    if pred.operator.deref() != "trim!" {
      continue;
    }

    let Ok((capture, spec)) = parse_trim_predicate(pred) else {
      continue;
    };

    map.insert(capture, spec);
  }

  map
}

pub fn apply_trim(
  source: &[u8],
  start_byte: usize,
  end_byte: usize,
  spec: TrimSpec,
) -> (usize, usize) {
  let mut start = start_byte;
  let mut end = end_byte;
  if start >= end || end > source.len() {
    return (start_byte, end_byte);
  }

  if spec.start_linewise {
    start = trim_start_linewise(source, start, end);
  }
  if spec.start_charwise {
    start = trim_start_charwise(source, start, end);
  }

  if spec.end_linewise {
    end = trim_end_linewise(source, start, end);
  }
  if spec.end_charwise {
    end = trim_end_charwise(source, start, end);
  }

  (start, end)
}

fn parse_trim_predicate(pred: &QueryPredicate) -> anyhow::Result<(u32, TrimSpec)> {
  match pred.args.len() {
    1 => {
      let [QueryPredicateArg::Capture(capture)] = pred.args.as_ref() else {
        anyhow::bail!("Trim predicate contained unexpected arguments");
      };
      Ok((*capture, TrimSpec::default_end_linewise_only()))
    }
    5 => {
      let [QueryPredicateArg::Capture(capture), QueryPredicateArg::String(start_linewise), QueryPredicateArg::String(start_charwise), QueryPredicateArg::String(end_linewise), QueryPredicateArg::String(end_charwise)] =
        pred.args.as_ref()
      else {
        anyhow::bail!("Trim predicate contained unexpected arguments");
      };

      let spec = TrimSpec {
        start_linewise: parse_bool_int(start_linewise)?,
        start_charwise: parse_bool_int(start_charwise)?,
        end_linewise: parse_bool_int(end_linewise)?,
        end_charwise: parse_bool_int(end_charwise)?,
      };
      Ok((*capture, spec))
    }
    _ => anyhow::bail!("Trim predicate requires 1 or 5 arguments"),
  }
}

fn parse_bool_int(value: &str) -> anyhow::Result<bool> {
  match value {
    "0" => Ok(false),
    "1" => Ok(true),
    _ => anyhow::bail!("Expected 0 or 1"),
  }
}

fn is_line_whitespace_only(bytes: &[u8]) -> bool {
  bytes.iter().all(|b| matches!(*b, b' ' | b'\t' | b'\r'))
}

fn trim_start_linewise(source: &[u8], mut start: usize, end: usize) -> usize {
  while start < end {
    let slice = &source[start..end];
    let Some(nl_rel) = slice.iter().position(|b| *b == b'\n') else {
      if is_line_whitespace_only(slice) {
        return end;
      }
      return start;
    };

    if is_line_whitespace_only(&slice[..nl_rel]) {
      start = (start + nl_rel + 1).min(end);
      continue;
    }

    break;
  }

  start
}

fn trim_end_linewise(source: &[u8], start: usize, mut end: usize) -> usize {
  while end > start {
    let slice = &source[start..end];
    let line_end = if slice.last() == Some(&b'\n') {
      end - 1
    } else {
      end
    };

    let before_line = &source[start..line_end];
    let prev_nl = before_line.iter().rposition(|b| *b == b'\n');
    let line_start = prev_nl.map(|i| start + i + 1).unwrap_or(start);

    if is_line_whitespace_only(&source[line_start..line_end]) {
      end = line_start;
      continue;
    }

    break;
  }

  end
}

fn is_charwise_whitespace(byte: u8) -> bool {
  matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

fn trim_start_charwise(source: &[u8], mut start: usize, end: usize) -> usize {
  while start < end && is_charwise_whitespace(source[start]) {
    start += 1;
  }
  start
}

fn trim_end_charwise(source: &[u8], start: usize, mut end: usize) -> usize {
  while end > start && is_charwise_whitespace(source[end - 1]) {
    end -= 1;
  }
  end
}
