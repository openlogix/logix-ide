#!/usr/bin/env bash

set -e

export PATH=/opt/homebrew/opt/rustup/bin:$PATH

set -x

# Run from the repo root, wherever the script is called from.
cd "$(dirname "$0")/.."

# (Re)install the JS packages if they're missing (e.g. after ./clean.sh) or
# changed since the last install (e.g. a pull that added a package).
if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  npm install
fi

npm run tauri dev

