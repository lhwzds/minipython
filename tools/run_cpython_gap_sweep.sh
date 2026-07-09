#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python_version="$(tr -d '[:space:]' < .python-version)"
cpython="/opt/homebrew/bin/python3"

if [[ ! -x "$cpython" ]]; then
  echo "CPython oracle not found or not executable: $cpython" >&2
  exit 1
fi

: "${CARGO_TARGET_DIR:=/tmp/minipython-target}"
export CARGO_TARGET_DIR

cargo build --bin mnpy

minipython="${CARGO_TARGET_DIR}/debug/mnpy"
if [[ ! -x "$minipython" ]]; then
  echo "MiniPython executable not found: $minipython" >&2
  exit 1
fi

exec "$cpython" tools/cpython_gap_sweep.py \
  --cpython "$cpython" \
  --require-version "$python_version" \
  --minipython "$minipython" \
  --corpus tests/gap_corpus \
  --out reports/cpython-gap-sweep \
  --fail-on-diff \
  "$@"
