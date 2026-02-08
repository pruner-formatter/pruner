use std::collections::HashMap;

use anyhow::Result;

use pruner::{
  api::format::{self, FormatContext, FormatOpts},
  config::LanguageFormatSpec,
  wasm::formatter::WasmFormatter,
};

mod common;

#[test]
fn injections_only_pipeline_condition_test() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let language_aliases = common::language_aliases();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let languages = HashMap::from([(
    "clojure".to_string(),
    vec![LanguageFormatSpec::Table {
      formatter: "cljfmt".into(),
      run_in_root: false,
      run_in_injections: true,
    }],
  )]);

  let source = r"(println 1  )";

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
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
  )
  .unwrap();

  assert_eq!(String::from_utf8(result).unwrap(), source);

  let source = r"```clojure
(println 1  )
```
";

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "markdown",
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
  )
  .unwrap();

  assert_eq!(
    String::from_utf8(result).unwrap(),
    r"```clojure
(println 1)
```
"
  );

  Ok(())
}

#[test]
fn root_only_pipeline_condition_test() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let language_aliases = common::language_aliases();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let languages = HashMap::from([(
    "clojure".to_string(),
    vec![LanguageFormatSpec::Table {
      formatter: "cljfmt".into(),
      run_in_root: true,
      run_in_injections: false,
    }],
  )]);

  let source = r"(println 1  )";

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
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
  )
  .unwrap();

  assert_eq!(String::from_utf8(result).unwrap(), r"(println 1)");

  let source = r"```clojure
(println 1  )
```
";

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "markdown",
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
  )
  .unwrap();

  assert_eq!(String::from_utf8(result).unwrap(), source);

  Ok(())
}
