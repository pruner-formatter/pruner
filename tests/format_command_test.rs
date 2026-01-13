use anyhow::Result;

use pruner::{
  api::format::{self, FormatContext, FormatOpts},
  wasm::formatter::WasmFormatter,
};

mod common;

#[test]
fn format_command() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let source = common::load_file("format_command/input.clj");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )
  .unwrap();

  let expected = common::load_file("format_command/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn fail_on_empty_stdout() -> Result<()> {
  let grammars = common::grammars()?;
  let mut formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  formatters.insert(
    "prettier".into(),
    pruner::config::FormatterSpec {
      cmd: "echo".into(),
      args: vec!["-n".into()],
      stdin: None,
      fail_on_stderr: None,
    },
  );

  let source = common::load_file("format_command/input.clj");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  );

  match result {
    Ok(_) => panic!("the formatter should cause a failure"),
    Err(err) => assert_eq!(
      "Unexpected empty result received from formatter: echo",
      err.to_string()
    ),
  };

  Ok(())
}

#[test]
fn format_escaped() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let source = common::load_file("format_escaped/input.clj");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )
  .unwrap();

  let expected = common::load_file("format_escaped/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn markdown_with_escape_characters() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let source = common::load_file("markdown_with_escape_characters/input.md");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "markdown",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )
  .unwrap();

  let expected = common::load_file("markdown_with_escape_characters/output.md");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn format_double_escaped() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let source = common::load_file("double_escaped/input.clj");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )
  .unwrap();

  let expected = common::load_file("double_escaped/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn format_injections_only() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let source = common::load_file("format_injections_only/input.clj");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    true,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )
  .unwrap();

  let expected = common::load_file("format_injections_only/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn offset_dependent_printwidth() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let source = common::load_file("offset_dependent_printwidth/input.clj");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    true,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )
  .unwrap();

  let expected = common::load_file("offset_dependent_printwidth/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn format_fixes_indent() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let source = common::load_file("format_fixes_indent/input.clj");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )
  .unwrap();

  let expected = common::load_file("format_fixes_indent/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn markdown_with_html() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();
  let wasm_formatter = WasmFormatter::new("cache".into())?;

  let source = common::load_file("markdown_with_html/input.md");

  let result = format::format(
    source.as_bytes(),
    &FormatOpts {
      printwidth: 80,
      language: "markdown",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
      wasm_formatter: &wasm_formatter,
    },
  )
  .unwrap();

  let expected = common::load_file("markdown_with_html/output.md");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}
