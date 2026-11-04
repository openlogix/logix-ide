#!/usr/bin/env bash
#
# Runs Logix in development mode (live reload).
#
#   scripts/run.sh                                  # with the pinned engine release
#   scripts/run.sh --engine-repo-dir <engine clone> # with a locally built engine
#
# --engine-repo-dir builds the engine in that local logix-engine clone
# (cargo build, debug) and runs Logix with it, through LOGIX_DEV_ENGINE_BINARY.
# Its packages go to ~/.logix/engine-dev, apart from the released engine's.
# Restart the script after changing the engine.

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

set -x

# Run from the repo root, wherever the script is called from.
cd "$(dirname "$0")/.."

if [ -n "$engine_repo_dir" ]; then
  [ -f "$engine_repo_dir/Cargo.toml" ] || { echo "Not a logix-engine clone: $engine_repo_dir" >&2; exit 2; }
  (cd "$engine_repo_dir" && cargo build)
  export LOGIX_DEV_ENGINE_BINARY="$(cd "$engine_repo_dir" && pwd)/target/debug/logix-engine"
fi

# (Re)install the JS packages if they're missing (e.g. after ./clean.sh) or
# changed since the last install (e.g. a pull that added a package).
if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  npm install
fi

npm run tauri dev
