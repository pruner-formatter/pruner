<div align="center">
  <h1>Pruner</h1>
  <p>
    A language-agnostic, TreeSitter-powered formatter for your code.
  </p>

</div>

## What

Pruner is a language and editor agnostic formatter which allows encapsulating all the formatting rules of your project
behind a shared, re-usable piece of config. It is designed in such a way as to allow leveraging all the existing,
language-specific formatter tools you are already using while also adding additional formatting capabilities.

Often times real-world source code contains multiple embedded languages, while formatters are typically very
language-specific and only operate on the root document - treating these embedded language regions as opaque strings.
Pruner uses Tree-Sitter to parse and understand source code files containing embedded languages, and can format those
embedded regions using their native formatting toolchain.

![extraction-diagram](./assets/pruner-diagram.webp)

This effectively allows you to utilize a languages' native ecosystem for formatting across language barriers. This would
be in contrast to, for example, trying to build a single formatter which knows how to format all languages - an
impractical goal.

The goal is not to re-implement individual formatters for each language, but rather to define how to compose, configure,
and execute them.

In addition to being able to call out to existing language formatters, Pruner can also be extended using WASM-compiled
plugins. This allows encapsulating project/organization specific formatting rules, or writing brand new language
formatters in any language that can compile to WASM.

## Installation

### Homebrew

```bash
brew install pruner-formatter/tap/pruner
```

### Raw Binaries

You can download the latest binary for your platform via the
**[Github releases page](https://github.com/pruner-formatter/pruner/releases)**

## Quick-Start

A configuration file is required for pruner to do anything when presented with source code. Without one pruner will just
return the presented text verbatim.

For this example lets configure Pruner to format the following Markdown document which contains some embedded
JavaScript:

````markdown
<!-- hello-world.md -->
Hello, 
world!

```javascript
console.log(  "Hello, world"  )
```
````

The only external formatter we require for this is `prettier` which conveniently understands how to format both of these
languages. We also need the Markdown treesitter grammar so that Pruner can parse out the embedded JS code region.

Add the following config to `$XDG_CONFIG_HOME/pruner/config.toml` (or `~/.config/pruner/config.toml`):

```toml
# ~/.config/pruner/config.toml

[grammars]
markdown = "https://github.com/tree-sitter-grammars/tree-sitter-markdown"

# Instruct pruner on how to invoke the `prettier` binary
[formatters]
prettier = { cmd = "prettier", args = [
  "--prose-wrap=always",
  "--print-width=$textwidth",
  "--parser=$language",
] }

# Instruct pruner to use the `prettier` formatter for both markdown and javascript
[languages]
markdown = ["prettier"]
javascript = ["prettier"]
```

Pruner reads from stdin and writes to stdout.

```bash
cat hello-world.md | pruner format --lang markdown > hello-world.md
cat hello-world.md
```

````markdown
<!-- hello-world.md -->

Hello, world!

```javascript
console.log("Hello, world");
```
````

And we can see `pruner` has successfully formatted both the outer Markdown document as well as the inner JavaScript code
region!

See the **[project documentation](https://pruner-formatter.github.io)** for more information regarding configuration,
plugins, and language injections.

## Acknowledgements

Thanks to

- [stevearc/conform.nvim](https://github.com/stevearc/conform.nvim/) For being the driving inspiration for Pruners
  config and approach to formatter composition.
