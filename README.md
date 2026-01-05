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

## How to use it

Pruner reads from stdin and writes to stdout.

```bash
pruner format --lang clojure < input.clj > output.clj
```

Options:

- `--lang`: the root language name (must match the grammar name).
- `--print-width`: print width passed to formatters (default: 80).
- `--injected-regions-only`: only format injected regions; do not format root.
- `--config`: path to `config.toml` (defaults to XDG config if present).

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
