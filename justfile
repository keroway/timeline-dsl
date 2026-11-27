# keroway 標準 justfile（Rust workspace / apps/webui への薄い委譲のみ）

default:
    @just --list

build:
    cargo build --workspace

test:
    cargo test --workspace --all-targets

# CI (.github/workflows/ci.yml の Clippy ステップ) と同一のコマンドにすること。
lint:
    cargo clippy --workspace --all-targets -- -D warnings

format:
    cargo fmt --all

# fmt check / clippy / test をまとめて実行（コミット前の全通し確認）
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets

# mutation testing（手動・週次のみ。CI / hook には入れない。#1062）。
# 対象は tdsl-parser に限定。要 `cargo install cargo-mutants --locked`。結果は mutants.out/ に出る。
mutants:
    cargo mutants -p tdsl-parser --no-shuffle

# --- WebUI (apps/webui, npm) ---

webui-lint:
    npm --prefix apps/webui run lint

webui-test:
    npm --prefix apps/webui test

webui-build:
    npm --prefix apps/webui run build
