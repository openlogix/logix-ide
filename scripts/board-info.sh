#!/usr/bin/env bash
#
# Prints what Logix Engine's catalog says about boards, as JSON: for each
# board ID, its get_catalog entry (resources, requires and ai_hints
# included), its FPGA and its FPGA family. It's the board input of the AI
# generator of the standard examples (spec/examples/README.md). Open Logix's
# tools read the catalog only through the engine, never from Logix
# Catalog's files.
#
#   scripts/board-info.sh <board id>...                                 # the engine in LOGIX_DEV_ENGINE_BINARY
#   scripts/board-info.sh --engine-repo-dir <engine clone> <board id>... # a locally built engine
#   scripts/board-info.sh --list                                        # every board ID, one per line

set -e

export PATH=/opt/homebrew/opt/rustup/bin:$PATH

engine_repo_dir=""
args=()
while [ $# -gt 0 ]; do
  case "$1" in
    --engine-repo-dir)
      [ -n "$2" ] || { echo "--engine-repo-dir needs a folder" >&2; exit 2; }
      engine_repo_dir="$2"
      shift 2
      ;;
    *)
      args+=("$1")
      shift
      ;;
  esac
done
[ ${#args[@]} -gt 0 ] || { echo "Name one or more board IDs, or --list (see the top of $0)" >&2; exit 2; }

if [ -n "$engine_repo_dir" ]; then
  [ -f "$engine_repo_dir/Cargo.toml" ] || { echo "Not a logix-engine clone: $engine_repo_dir" >&2; exit 2; }
  (cd "$engine_repo_dir" && cargo build -q)
  LOGIX_DEV_ENGINE_BINARY="$(cd "$engine_repo_dir" && pwd)/target/debug/logix-engine"
fi
[ -x "${LOGIX_DEV_ENGINE_BINARY:-}" ] || { echo "Set LOGIX_DEV_ENGINE_BINARY to a logix-engine binary, or use --engine-repo-dir" >&2; exit 2; }

# The engine needs its own folder, whose name contains "engine".
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/logix-engine"

printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"board-info","version":"1"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_engine_info","arguments":{}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_catalog","arguments":{"boards":{"fetch":true},"fpgas":{"fetch":true},"families":{"fetch":true}}}}' |
  "$LOGIX_DEV_ENGINE_BINARY" --engine-folder "$tmp/logix-engine" --log-level warn > "$tmp/out.jsonl" 2> "$tmp/err.txt" ||
  { cat "$tmp/err.txt" >&2; exit 1; }

python3 -I - "$tmp/out.jsonl" "${args[@]}" <<'EOF'
import json, sys

results = {}
for line in open(sys.argv[1]):
    m = json.loads(line)
    if m.get("id") in (2, 3):
        r = m["result"]
        results[m["id"]] = r.get("structuredContent") or json.loads(r["content"][0]["text"])
info, catalog = results[2], results[3]
ids = sys.argv[2:]
if ids == ["--list"]:
    print("\n".join(sorted(catalog["boards"])))
    sys.exit(0)
out = {"catalog": info.get("catalog"), "boards": {}}
for bid in ids:
    b = catalog["boards"].get(bid)
    if b is None:
        sys.exit(f"{bid!r} isn't a board in the engine's catalog (--list lists them)")
    fpga = catalog["fpgas"][b["fpga_id"]]
    out["boards"][bid] = {"board": b, "fpga": fpga, "family": catalog["families"][fpga["family"]]}
print(json.dumps(out, indent=2, ensure_ascii=False))
EOF
