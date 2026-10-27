#!/usr/bin/env bash

# Runs all code checks and tests. Stops at the first failure.

set -x
set -e

# Run from the repo root, wherever the script is called from.
cd "$(dirname "$0")/.."

export PATH=/opt/homebrew/opt/rustup/bin:$PATH

# (Re)install the JS packages if they're missing (e.g. after ./clean.sh) or
# changed since the last install (e.g. a pull that added a package).
if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  npm install
fi

# UI (TypeScript + Svelte): type check, unit tests, a production build, then
# the end-to-end UI tests (Playwright, WebKit; first time on a machine:
# npx playwright install webkit).
npm run check
npm test
npm run build
npm run test:e2e

# Backend (Rust): formatting, lint, then unit tests.
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

set +x
echo "All checks and tests passed."
