use tree_sitter::{Node, Query, QueryCursor, Range, StreamingIterator};

fn is_comment_node(node: Node) -> bool {
  node.kind().contains("comment")
}

pub(crate) fn collect_ignore_ranges(
  root: Node,
  source: &[u8],
  ignore_query: Option<&Query>,
) -> Vec<Range> {
  fn add_marker(ignore_ranges: &mut Vec<Range>, marker: Node) {
    ignore_ranges.push(marker.range());

    let mut target = marker.next_named_sibling();
    while let Some(candidate) = target {
      if is_comment_node(candidate) {
        target = candidate.next_named_sibling();
      } else {
        break;
      }
    }

    if let Some(target) = target {
      ignore_ranges.push(target.range());
    }
  }

  fn visit(node: Node, source: &[u8], ignore_ranges: &mut Vec<Range>) {
    if is_comment_node(node)
      && let Ok(text) = node.utf8_text(source)
      && text.contains("pruner-ignore")
    {
      add_marker(ignore_ranges, node);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
      visit(child, source, ignore_ranges);
    }
  }

  let mut ignore_ranges = Vec::new();
  visit(root, source, &mut ignore_ranges);

  if let Some(ignore_query) = ignore_query {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(ignore_query, root, source);
    let ignore_target_capture = ignore_query.capture_index_for_name("pruner.ignore");
    let ignore_marker_capture =
      ignore_query.capture_index_for_name("pruner.ignore.marker");

    if ignore_target_capture.is_none() && ignore_marker_capture.is_none() {
      return ignore_ranges;
    }

    while let Some(query_match) = matches.next() {
      for capture in query_match.captures {
        if Some(capture.index) == ignore_target_capture {
          ignore_ranges.push(capture.node.range());
        }

        if Some(capture.index) == ignore_marker_capture {
          add_marker(&mut ignore_ranges, capture.node);
        }
      }
    }
  }

  ignore_ranges
}

pub(crate) fn is_ignored(range: &Range, ignore_ranges: &[Range]) -> bool {
  ignore_ranges
    .iter()
    .any(|ignore| range.start_byte >= ignore.start_byte && range.end_byte <= ignore.end_byte)
}
