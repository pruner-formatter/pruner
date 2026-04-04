use anyhow::Result;
use std::collections::HashMap;

use pruner::{
  api::format::{self, FormatContext, FormatOpts},
  wasm::formatter::WasmFormatter,
};

mod common;

#[test]
fn normalizes_injected_language_via_aliases() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let language_aliases = HashMap::from([("ts".to_string(), "typescript".to_string())]);

  let source = "```ts\nconsole.log(  1  )\n```\n";
  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "markdown",
      source_file: None,
    },
    false,
    true,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      language_aliases: &language_aliases,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )?;

  assert_eq!(
    String::from_utf8(result).unwrap(),
    "```ts\nconsole.log(1);\n```\n"
  );

  Ok(())
}

#[test]
fn normalizes_root_language_via_aliases() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let language_aliases = HashMap::from([("clj".to_string(), "clojure".to_string())]);

  let source = "(def x [  1   2   3  ])\n";
  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clj",
      source_file: None,
    },
    true,
    true,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      language_aliases: &language_aliases,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )?;

  assert_eq!(String::from_utf8(result).unwrap(), "(def x [1 2 3])\n");

  Ok(())
}
