use anyhow::Result;
use std::collections::HashSet;
use tree_sitter::{Point, Range};

use pruner::api::injections::{self, InjectedRegion, InjectionOpts};

mod common;

#[test]
fn injected_regions_markdown() -> Result<()> {
  let grammars = common::grammars()?;

  let grammar = grammars
    .get("clojure")
    .ok_or_else(|| anyhow::anyhow!("Missing clojure grammar"))?;

  let source = r#"(defn nested-clojure-example
  "Title

   ```clojure
   (println 1 )
   (println   \"awesome stuff\" )
   ```"
  []
  1)"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions =
    injections::extract_language_injections(&mut parser, grammar, source_bytes)?;

  assert_eq!(
    injected_regions,
    vec![InjectedRegion {
      range: Range {
        start_byte: 32,
        end_byte: 109,
        start_point: Point { row: 1, column: 3 },
        end_point: Point { row: 6, column: 6 }
      },
      lang: "markdown".into(),
      opts: InjectionOpts {
        escape_chars: HashSet::from(["\"".to_string()]),
      }
    }]
  );

  Ok(())
}

/// Some grammars (like markdown) require the file to end with a newline. When used in a clojure
/// docstring, however, we might not end on a newline (see the example below).
///
/// Pruner internally appends a newline to the injected region before re-parsing it to get around
/// this behaviour. This is testing that scenario.
#[test]
fn injected_regions_newline() -> Result<()> {
  let grammars = common::grammars()?;

  let grammar = grammars
    .get("markdown")
    .ok_or_else(|| anyhow::anyhow!("Missing markdown grammar"))?;

  let source = r#"Title

   ```clojure
   (println 1 )
   (println 2)
   ```"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions =
    injections::extract_language_injections(&mut parser, grammar, source_bytes)?;

  assert_eq!(
    injected_regions,
    vec![
      InjectedRegion {
        range: Range {
          start_byte: 0,
          end_byte: 5,
          start_point: Point { row: 0, column: 0 },
          end_point: Point { row: 0, column: 5 }
        },
        lang: "markdown_inline".into(),
        opts: InjectionOpts {
          escape_chars: HashSet::default(),
        }
      },
      InjectedRegion {
        range: Range {
          start_byte: 21,
          end_byte: 52,
          start_point: Point { row: 3, column: 0 },
          end_point: Point { row: 5, column: 0 }
        },
        lang: "clojure".into(),
        opts: InjectionOpts {
          escape_chars: HashSet::default(),
        }
      }
    ],
    "The clojure injected region should not contain the trailing ``` characters"
  );

  Ok(())
}
