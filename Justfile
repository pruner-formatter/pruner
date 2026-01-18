default:
    @just --list

prepare:
    cd crates/cli && just prepare

build:
    cargo build -p pruner --release

install: build
    cp target/release/pruner ~/.local/bin/pruner

test test = "":
    cargo test {{ test }} -- --nocapture
