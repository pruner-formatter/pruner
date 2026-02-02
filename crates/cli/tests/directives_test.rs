use anyhow::Result;
use std::collections::HashSet;
use tree_sitter::{Point, Range};

use pruner::api::injections::{self, InjectedRegion, InjectionOpts};

mod common;

#[test]
fn gsub_directive_test() -> Result<()> {
  let grammars = common::grammars()?;

  let grammar = grammars
    .get("nix")
    .ok_or_else(|| anyhow::anyhow!("Missing grammar"))?;

  let source = r#"{}: let
  embeddedTs =
    # javascript
    ''console.log(1)'';
"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions =
    injections::extract_language_injections(&mut parser, grammar, source_bytes)?;

  assert_eq!(
    injected_regions,
    vec![InjectedRegion {
      range: Range {
        start_byte: 46,
        end_byte: 60,
        start_point: Point { row: 3, column: 6 },
        end_point: Point { row: 3, column: 20 }
      },
      lang: "javascript".into(),
      opts: InjectionOpts {
        escape_chars: HashSet::new()
      }
    }]
  );

  Ok(())
}

#[test]
fn pruner_indented_property_test() -> Result<()> {
  let grammars = common::grammars()?;

  let grammar = grammars
    .get("nix")
    .ok_or_else(|| anyhow::anyhow!("Missing grammar"))?;

  let source = r#"{}: let
  embeddedTs =
    # javascript
    ''
      console.log(1)
    '';
"#;
  let source_bytes = source.as_bytes();

  let mut parser = tree_sitter::Parser::new();
  let injected_regions =
    injections::extract_language_injections(&mut parser, grammar, source_bytes)?;

  assert_eq!(
    injected_regions,
    vec![InjectedRegion {
      range: Range {
        start_byte: 47,
        end_byte: 68,
        start_point: Point { row: 4, column: 0 },
        end_point: Point { row: 5, column: 0 }
      },
      lang: "javascript".into(),
      opts: InjectionOpts {
        escape_chars: HashSet::new()
      }
    }]
  );

  Ok(())
}
