use std::{collections::HashMap, ops::Deref};
use tree_sitter::{Point, QueryPredicate, QueryPredicateArg, Range};

#[derive(Debug, Clone, Copy)]
pub struct RangeOffset {
  pub start_row: isize,
  pub start_col: isize,
  pub end_row: isize,
  pub end_col: isize,
}

pub fn collect(predicates: &[QueryPredicate]) -> HashMap<u32, RangeOffset> {
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

pub fn apply_offset_to_range(source: &str, range: &Range, offset: &RangeOffset) -> Option<Range> {
  let new_start_point = Point {
    row: apply_signed(range.start_point.row, offset.start_row)?,
    column: apply_signed(range.start_point.column, offset.start_col)?,
  };
  let new_end_point = Point {
    row: apply_signed(range.end_point.row, offset.end_row)?,
    column: apply_signed(range.end_point.column, offset.end_col)?,
  };

  let new_start_byte = point_to_byte(source, new_start_point)?;
  let new_end_byte = point_to_byte(source, new_end_point)?;

  Some(Range {
    start_byte: new_start_byte,
    end_byte: new_end_byte,
    start_point: new_start_point,
    end_point: new_end_point,
  })
}

fn parse_offset_predicate(pred: &QueryPredicate) -> anyhow::Result<(u32, RangeOffset)> {
  if pred.args.len() != 5 {
    anyhow::bail!("Offset predicate requires 5 arguments");
  }

  let [
    QueryPredicateArg::Capture(capture),
    QueryPredicateArg::String(start_row),
    QueryPredicateArg::String(start_col),
    QueryPredicateArg::String(end_row),
    QueryPredicateArg::String(end_col),
  ] = pred.args.deref()
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

fn apply_signed(value: usize, offset: isize) -> Option<usize> {
  let value: isize = value.try_into().ok()?;
  let adjusted = value.checked_add(offset)?;
  if adjusted < 0 {
    return None;
  }
  adjusted.try_into().ok()
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
