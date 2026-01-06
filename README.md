<div align="center">
  <h1>pruner</h1>

  <p>
    A TreeSitter-powered formatter orchestrator
  </p>
</div>

---

## What is this?

Pruner is a formatter orchestrator that understands tree-sitter language injections. It lets you format a host language
(like Clojure) and also format embedded regions (like Markdown docstrings or SQL strings) with the appropriate
formatter, then re-embeds the results back into the original file.

The core idea is:

- Parse the source with tree-sitter.
- Find injected regions using injection queries.
- Format both the root language and injected regions.
- Re-apply the injected formatting back into the root result.

## Installation

### Homebrew

```bash
brew install julienvincent/tap/pruner
```

### Binaries

The binaries are also available on every github release. Check the releases page to find the latest binary.

## How to use it

Pruner reads from stdin and writes to stdout.

```bash
pruner format --lang clojure < input.clj > output.clj
```

```bash
❯ pruner format --help
Format one or more files

Usage: pruner format [OPTIONS] --lang <LANG> [INCLUDE_GLOB]

Arguments:
  [INCLUDE_GLOB]
          A file pattern, in glob format, describing files on disk to be formatted.

          If this is specified then pruner will recursively format all files in the cwd (or --dir if set) that match this pattern.

          If this is _not_ set then pruner will expect source code to be provided via stdin and the formatted result will be outputted over stdout.

Options:
      --lang <LANG>
          The language name of the root document. Regions containing injected languages will be dynamically discovered from queries

      --log-level <LOG_LEVEL>


  -w, --print-width <PRINT_WIDTH>
          The desired print-width of the document after which text should wrap. This value specifies the starting point and will be dynamically adjusted for injected language regions

          [default: 80]

  -R, --skip-root [<SKIP_ROOT>]
          Specifying this will skip formatting the document root. This means only regions within the document containing language injections will be formatted. If you only want to use pruner to format injected regions, then this is the option to use.

          This can be especially useful in an editor context where you might want to use your LSP to format your document root, and then run pruner on the result to format injected regions.

          [default: false]
          [possible values: true, false]

  -d, --dir <DIR>
          The current working directory. Only used when formatting files

  -e, --exclude <EXCLUDE>
          Specify a file exclusion pattern as a glob. Any files matching this pattern will not be formatted. Can be specified multiple times

  -c, --check [<CHECK>]
          Setting this to true will result in no files being modified on disk. If any files are considered 'dirty' meaning, meaning they are not correctly formatted, then pruner will exit with a non-0 exit code

          [default: false]
          [possible values: true, false]

  -h, --help
          Print help (see a summary with '-h')
```

## Configuration

Config is read from the path provided via `--config`. If you do not pass a config, Pruner will look for `config.toml`
under the XDG config directory for the `pruner` app. If no config is found, Pruner uses defaults.

Example `config.toml`:

```toml
query_paths = ["queries"]

[grammars]
clojure = { url = "https://github.com/sogaiu/tree-sitter-clojure" }
markdown = { url = "https://github.com/tree-sitter-grammars/tree-sitter-markdown" }
sql = { url = "https://github.com/derekstride/tree-sitter-sql", rev = "gh-pages" }

[formatters]
prettier = { cmd = "prettier", args = ["--prose-wrap=always", "--print-width=$textwidth",
  "--parser=$language"] }
cljfmt = { cmd = "cljfmt", args = ["fix", "-", "--remove-multiple-non-indenting-spaces"],
  stdin = true }

[languages]
markdown = ["prettier"]
clojure = ["cljfmt"]
```

Notes:

- `query_paths` and `grammar_paths` are searched for tree-sitter files.
- `grammar_download_dir` and `grammar_build_dir` are relative to the current dir.
- `grammars` is a map of language name to a git URL (optionally pinned with `rev`).
- `formatters` define formatter commands; `$textwidth` and `$language` are replaced.
- `languages` maps a language name to the formatter(s) to use (first is chosen).

## Queries

Pruner uses tree-sitter injection queries (`injections.scm`) to find embedded regions. These live either inside a
grammar repo or alongside your custom queries.

Example: inject Markdown from Clojure docstrings:

```query
((list_lit
  ((sym_lit) @def-type
   (sym_lit) @def-name
   (str_lit) @docstring @injection.content)
   (map_lit)?

   (_)+)

  (#match? @def-type "^(def|defprotocol)$")
  (#offset! @injection.content 0 1 0 -1)
  (#escape! @injection.content "\"")
  (#set! injection.language "markdown"))
```

Example: inject SQL from string literals that start with SQL keywords:

```query
((str_lit) @injection.content
  (#match? @injection.content "^\"(SELECT|CREATE|ALTER|UPDATE|DROP|INSERT)")
  (#offset! @injection.content 0 1 0 -1)
  (#escape! @injection.content "\"")
  (#set! injection.language "sql"))
```

Query helpers used above:

- `#offset!` adjusts the captured range (e.g., trim quotes).
- `#escape!` tells Pruner which characters need unescaping before formatting.
- `injection.language` sets the formatter language for the injected region.
