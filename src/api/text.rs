use std::collections::HashSet;

pub fn offset_lines(data: &mut Vec<u8>, offset: usize) {
  if offset == 0 {
    return;
  }

  let mut i = 0;
  while i < data.len() {
    if data[i] == b'\n' {
      let next = data.get(i + 1).copied();
      if matches!(next, Some(b'\n') | Some(b'\r') | None) {
        i += 1;
        continue;
      }
      let spaces = vec![b' '; offset];
      data.splice(i + 1..i + 1, spaces);
      i += offset + 1;
    } else {
      i += 1;
    }
  }
}

pub fn trim_trailing_whitespace(data: &mut Vec<u8>, preserve_newline: bool) {
  let mut removed_newline = false;
  while data.last() == Some(&b'\n') || data.last() == Some(&b'\r') {
    data.pop();
    removed_newline = true;
  }

  if preserve_newline && removed_newline {
    data.push(b'\n');
  }
}

pub fn column_for_byte(source: &[u8], byte_index: usize) -> usize {
  let target = byte_index.min(source.len());
  let line_start = source[..target]
    .iter()
    .rposition(|byte| *byte == b'\n')
    .map(|index| index + 1)
    .unwrap_or(0);

  target - line_start
}

pub fn min_leading_indent(text: &str) -> usize {
  let mut min_indent: Option<usize> = None;
  for line in text.lines() {
    if line.trim().is_empty() {
      continue;
    }
    let indent = line.chars().take_while(|ch| *ch == ' ').count();
    min_indent = Some(min_indent.map_or(indent, |current| current.min(indent)));
  }

  min_indent.unwrap_or(0)
}

pub fn strip_leading_indent(text: &str, indent: usize) -> String {
  if indent == 0 {
    return text.to_string();
  }

  let mut result = String::with_capacity(text.len());
  for segment in text.split_inclusive('\n') {
    let (line, newline) = if segment.ends_with('\n') {
      (&segment[..segment.len() - 1], "\n")
    } else {
      (segment, "")
    };
    let leading_spaces = line.chars().take_while(|ch| *ch == ' ').count();
    let trim_count = indent.min(leading_spaces);
    let trimmed = if trim_count > 0 {
      &line[trim_count..]
    } else {
      line
    };
    result.push_str(trimmed);
    result.push_str(newline);
  }

  result
}

pub fn sort_escape_chars(escape_chars: &HashSet<String>) -> Vec<String> {
  let mut chars: Vec<String> = escape_chars.iter().cloned().collect();
  chars.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
  chars
}

pub fn unescape_text(text: &str, escape_chars: &[String]) -> String {
  let mut result = text.to_string();
  for escape_char in escape_chars {
    let mut pattern = String::from("\\");
    pattern.push_str(escape_char);
    result = result.replace(&pattern, escape_char);
  }
  result
}

pub fn escape_text(text: &str, escape_chars: &[String]) -> String {
  let mut result = text.to_string();
  for escape_char in escape_chars {
    let mut replacement = String::from("\\");
    replacement.push_str(escape_char);
    result = result.replace(escape_char, &replacement);
  }
  result
}
