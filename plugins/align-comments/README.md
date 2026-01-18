# Plugin: align-comments

A Pruner Plugin which can align Clojure comments.

## Building

This project compiles to a WASM component targeting `wasm32-wasip2`. Because it uses tree-sitter (which has C code), you
need a C compiler and WASI sysroot that can target WebAssembly.
