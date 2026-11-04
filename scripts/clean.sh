#!/usr/bin/env bash

# Removes build artifacts, installed dependencies and the IDE's data
# (~/.logix: the downloaded Logix Engine and its tools). Run `npm install` afterwards to restore the JS dependencies.

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

# The IDE's data: Logix Engine, its packages, settings and logs.
rm -rf "$HOME/.logix"
