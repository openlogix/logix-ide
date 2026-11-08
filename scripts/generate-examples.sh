#!/usr/bin/env bash
#
# Generates every catalog board's blinky and Bare Minimum from Logix
# Engine's board catalog (src-tauri/src/generate.rs), into
# src-tauri/target/generated-examples/<board>/<name>/.
#
#   scripts/generate-examples.sh                                  # the engine in LOGIX_DEV_ENGINE_BINARY
#   scripts/generate-examples.sh --engine-repo-dir <engine clone> # a locally built engine
#
# For now the results are only compared with examples/ (see CLAUDE.md,
# Examples); nothing in examples/ changes.

set -e

export PATH=/opt/homebrew/opt/rustup/bin:$PATH

engine_repo_dir=""
while [ $# -gt 0 ]; do
  case "$1" in
    --engine-repo-dir)
      [ -n "$2" ] || { echo "--engine-repo-dir needs a folder" >&2; exit 2; }
      engine_repo_dir="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1 (see the top of $0)" >&2
      exit 2
      ;;
  esac
done

cd "$(dirname "$0")/.."

if [ -n "$engine_repo_dir" ]; then
  [ -f "$engine_repo_dir/Cargo.toml" ] || { echo "Not a logix-engine clone: $engine_repo_dir" >&2; exit 2; }
  (cd "$engine_repo_dir" && cargo build)
  export LOGIX_DEV_ENGINE_BINARY="$(cd "$engine_repo_dir" && pwd)/target/debug/logix-engine"
fi
[ -x "${LOGIX_DEV_ENGINE_BINARY:-}" ] || { echo "Set LOGIX_DEV_ENGINE_BINARY to a logix-engine binary, or use --engine-repo-dir" >&2; exit 2; }

cd src-tauri
cargo test --lib generate::tests::generate_examples -- --ignored --nocapture
