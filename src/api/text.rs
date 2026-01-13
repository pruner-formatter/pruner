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

pub fn strip_trailing_newlines(data: &mut Vec<u8>) {
  while data.last() == Some(&b'\n') || data.last() == Some(&b'\r') {
    data.pop();
  }
}

pub fn trailing_newlines(data: &[u8]) -> Vec<u8> {
  let mut index = data.len();
  while index > 0 {
    match data[index - 1] {
      b'\n' | b'\r' => index -= 1,
      _ => break,
    }
  }

  data[index..].to_vec()
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

// Unescape injected text before passing it to a nested formatter.
//
// We scan left-to-right and treat `\\` as a literal backslash so double-escaped sequences survive.
//
// Only when a backslash directly prefixes one of the configured escape characters do we drop the
// backslash and emit the raw character.
pub fn unescape_text(text: &str, escape_chars: &[String]) -> String {
  let mut result = String::with_capacity(text.len());
  let escape_bytes: Vec<&[u8]> = escape_chars.iter().map(|s| s.as_bytes()).collect();

  let mut index = 0;
  while index < text.len() {
    let remaining = &text[index..];
    if remaining.as_bytes().first() == Some(&b'\\') {
      if remaining.as_bytes().get(1) == Some(&b'\\') {
        result.push('\\');
        index += 2;
        continue;
      }
      let mut matched = false;
      for escape in &escape_bytes {
        if remaining
          .as_bytes()
          .get(1..)
          .is_some_and(|rest| rest.starts_with(escape))
        {
          result.push_str(std::str::from_utf8(escape).unwrap());
          index += 1 + escape.len();
          matched = true;
          break;
        }
      }
      if matched {
        continue;
      }
    }

    let ch = remaining.chars().next().unwrap();
    result.push(ch);
    index += ch.len_utf8();
  }

  result
}

// Re-escape injected text before reinserting it into the outer document.
//
// We scan left-to-right and always escape literal backslashes, then prefix any configured escape
// character with a backslash.
pub fn escape_text(text: &str, escape_chars: &[String]) -> String {
  let mut result = String::with_capacity(text.len());
  let escape_bytes: Vec<&[u8]> = escape_chars.iter().map(|s| s.as_bytes()).collect();

  let mut index = 0;
  while index < text.len() {
    let remaining = &text[index..];
    if remaining.as_bytes().first() == Some(&b'\\') {
      result.push_str("\\\\");
      index += 1;
      continue;
    }
    let mut matched = false;
    for escape in &escape_bytes {
      if remaining.as_bytes().starts_with(escape) {
        result.push('\\');
        result.push_str(std::str::from_utf8(escape).unwrap());
        index += escape.len();
        matched = true;
        break;
      }
    }
    if matched {
      continue;
    }

    let ch = remaining.chars().next().unwrap();
    result.push(ch);
    index += ch.len_utf8();
  }

  result
}
