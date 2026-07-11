#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

export RUST_MIN_STACK="${RUST_MIN_STACK:-8388608}"

cpython="/opt/homebrew/bin/python3"
if [[ ! -x "$cpython" ]]; then
  echo "CPython oracle not found or not executable: $cpython" >&2
  exit 1
fi

mode="${1:-}"
if [[ -n "$mode" && "$mode" != "--focused" ]]; then
  echo "usage: tools/run_sandbox_mvp_checks.sh [--focused]" >&2
  exit 2
fi

if [[ "$mode" == "--focused" ]]; then
  cargo test --test sandbox_process
  cargo test --test sandbox_boundary
  cargo test --test sandbox_examples
  cargo test --test language sandbox_policy
  cargo test --test language instruction_budget
  cargo test --test language call_depth_guard
  cargo test --test language output_budget
  cargo test --test language allocation_budget
  cargo test --test cpython_manifest sandbox_mvp_checklist_keeps_completion_requirements_explicit
else
  MINIPYTHON_CPYTHON="$cpython" cargo test
fi

"$cpython" tools/test_cpython_gap_sweep.py
tools/run_cpython_gap_sweep.sh
cargo fmt --check
git diff --check
