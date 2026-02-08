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
