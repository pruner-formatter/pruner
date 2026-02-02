use tree_sitter::QueryProperty;

pub fn is_indented(properties: &[QueryProperty]) -> bool {
  properties
    .iter()
    .any(|property| property.key.as_ref() == "pruner.injection.indented")
}

pub fn trim_bytes(source: &[u8], start_byte: usize, end_byte: usize) -> (usize, usize) {
  let mut start = start_byte;
  let mut end = end_byte;

  if start >= end || end > source.len() {
    return (start_byte, end_byte);
  }

  // If the first line is whitespace-only (usually just the newline after an opening delimiter),
  // drop it so embedded formatters don't see a phantom leading blank line.
  let slice = &source[start..end];
  if let Some(newline_index) = slice.iter().position(|b| *b == b'\n') {
    let prefix = &slice[..newline_index];
    let is_whitespace_only = prefix.iter().all(|b| matches!(*b, b' ' | b'\t' | b'\r'));

    if is_whitespace_only {
      start = (start + newline_index + 1).min(end);
    }
  }

  // Drop trailing indentation before a closing delimiter, but never remove newlines.
  while end > start {
    let last = source[end - 1];
    if matches!(last, b' ' | b'\t') {
      end -= 1;
      continue;
    }
    break;
  }

  (start, end)
}
