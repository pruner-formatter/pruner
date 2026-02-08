use tree_sitter::{Node, Range};

fn is_comment_node(node: Node) -> bool {
  node.kind().contains("comment")
}

pub(crate) fn collect_ignore_ranges(root: Node, source: &[u8]) -> Vec<Range> {
  fn visit(node: Node, source: &[u8], ignore_ranges: &mut Vec<Range>) {
    if is_comment_node(node)
      && let Ok(text) = node.utf8_text(source)
      && text.contains("pruner-ignore")
    {
      ignore_ranges.push(node.range());

      let mut target = node.next_named_sibling();
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

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
      visit(child, source, ignore_ranges);
    }
  }

  let mut ignore_ranges = Vec::new();
  visit(root, source, &mut ignore_ranges);
  ignore_ranges
}

pub(crate) fn is_ignored(range: &Range, ignore_ranges: &[Range]) -> bool {
  ignore_ranges
    .iter()
    .any(|ignore| range.start_byte >= ignore.start_byte && range.end_byte <= ignore.end_byte)
}
