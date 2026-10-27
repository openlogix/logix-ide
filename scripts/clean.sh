#!/usr/bin/env bash

# Removes build artifacts, installed dependencies and the IDE's downloaded
# Apio files. Run `npm install` afterwards to restore the JS dependencies.

set -x
set -e

# Run from the repo root, wherever the script is called from.
cd "$(dirname "$0")/.."

# Build artifacts.
rm -rf ui/dist ui/test-results
rm -rf src-tauri/target
rm -rf src-tauri/gen/schemas

# Installed JS dependencies.
rm -rf node_modules

# Apio bundle and packages downloaded by the IDE. Doesn't touch ~/.apio.
rm -rf "$HOME/.logix"
