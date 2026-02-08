use anyhow::Result;
use std::collections::HashSet;
use tree_sitter::{Point, Range};

use pruner::api::injections::{self, InjectedRegion, InjectionOpts};

mod common;

#[test]
fn pruner_ignore_annotation_test() -> Result<()> {
  let grammars = common::grammars()?;

  let grammar = grammars
    .get("nix")
    .ok_or_else(|| anyhow::anyhow!("Missing grammar"))?;

  let source = r#"{}: let
  embeddedTs =
    # pruner-ignore
    # typescript
    ''console.log("hello")'';
"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions =
    injections::extract_language_injections(&mut parser, grammar, source_bytes)?;

  assert_eq!(injected_regions, vec![]);

  let source = r#"{}: let
  embeddedTs1 =
    # pruner-ignore
    # typescript
    ''console.log("hello")'';
  embeddedTs2 =
    # typescript
    ''console.log("hello")'';
"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions =
    injections::extract_language_injections(&mut parser, grammar, source_bytes)?;

  assert_eq!(
    injected_regions,
    vec![InjectedRegion {
      range: Range {
        start_byte: 130,
        end_byte: 150,
        start_point: Point { row: 7, column: 6 },
        end_point: Point { row: 7, column: 26 }
      },
      lang: "typescript".into(),
      opts: InjectionOpts {
        escape_chars: HashSet::new()
      }
    }]
  );

  Ok(())
}

// This test is checking the code-paths that use pruner/ignore.scm treesitter queries. The Markdown
// grammar is pretty weird in that comments are defined via nested html_block language injections.
//
// This test uses the ignore.scm query from fixtures/markdown/pruner/ignore.scm
#[test]
fn pruner_ignore_markdown() -> Result<()> {
  let grammars = common::grammars()?;

  let markdown = grammars
    .get("markdown")
    .ok_or_else(|| anyhow::anyhow!("Missing grammar"))?;

  let source = r#"abc

<!-- pruner-ignore -->
```typescript
console.log(1)
```
"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions =
    injections::extract_language_injections(&mut parser, markdown, source_bytes)?;

  assert_eq!(
    injected_regions,
    vec![InjectedRegion {
      range: Range {
        start_byte: 0,
        end_byte: 3,
        start_point: Point { row: 0, column: 0 },
        end_point: Point { row: 0, column: 3 }
      },
      lang: "markdown_inline".into(),
      opts: InjectionOpts {
        escape_chars: HashSet::new()
      }
    }]
  );

  Ok(())
}

#[test]
fn pruner_ignore_indirect() -> Result<()> {
  let grammars = common::grammars()?;

  let nix = grammars
    .get("nix")
    .ok_or_else(|| anyhow::anyhow!("Missing grammar"))?;

  let source = r#"{}: let
  # pruner-ignore
  embeddedTs =
    # typescript
    ''console.log("hello")'';
"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions = injections::extract_language_injections(&mut parser, nix, source_bytes)?;

  assert_eq!(injected_regions, vec![]);

  let clojure = grammars
    .get("clojure")
    .ok_or_else(|| anyhow::anyhow!("Missing grammar"))?;

  let source = r#";; pruner-ignore
(defn foo
  "This is markdown"
  []
  "SELECT * FROM user;")
"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions =
    injections::extract_language_injections(&mut parser, clojure, source_bytes)?;

  assert_eq!(injected_regions, vec![]);

  Ok(())
}
