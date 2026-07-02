#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python_version="$(tr -d '[:space:]' < .python-version)"

: "${CARGO_TARGET_DIR:=/tmp/minipython-target}"
export CARGO_TARGET_DIR

cargo build --bin mnpy

minipython="${CARGO_TARGET_DIR}/debug/mnpy"
if [[ ! -x "$minipython" ]]; then
  echo "MiniPython executable not found: $minipython" >&2
  exit 1
fi

exec uv run --python "$python_version" python tools/cpython_gap_sweep.py \
  --require-version "$python_version" \
  --minipython "$minipython" \
  --corpus tests/gap_corpus \
  --out reports/cpython-gap-sweep \
  --fail-on-diff \
  "$@"
