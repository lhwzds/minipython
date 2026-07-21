#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mode="release"
seed="20260710"
generated_cases="1024"
driver_args=(--module "*")

while [[ $# -gt 0 ]]; do
  case "$1" in
    --focused)
      mode="focused"
      generated_cases="64"
      shift
      ;;
    --discovery)
      mode="discovery"
      shift
      ;;
    --seed)
      seed="${2:?--seed requires a value}"
      shift 2
      ;;
    --generated-cases)
      generated_cases="${2:?--generated-cases requires a value}"
      shift 2
      ;;
    --module|--root-cause|--layer|--category|--scope)
      mode="discovery"
      driver_args+=("$1" "${2:?$1 requires a value}")
      shift 2
      ;;
    --fail-on-open|--fail-on-diff)
      mode="discovery"
      driver_args+=("$1")
      shift
      ;;
    -h|--help)
      cat <<'EOF'
usage: tests/run.sh [--focused|--discovery] [--seed N] [--generated-cases N]
                    [--module NAME] [--root-cause ID] [--layer NAME]

  default       complete Rust suite plus 1024 deterministic differential cases
  --focused     sandbox-focused Rust suite plus 64 differential cases
  --discovery   differential unit tests and generated cases only
EOF
      exit 0
      ;;
    *)
      echo "unknown test pipeline argument: $1" >&2
      exit 2
      ;;
  esac
done

cpython="/opt/homebrew/bin/python3"
python_version="$(tr -d '[:space:]' < .python-version)"
if [[ ! -x "$cpython" ]]; then
  echo "CPython oracle not found or not executable: $cpython" >&2
  exit 1
fi

actual_version="$($cpython -c 'import platform; print(platform.python_version())')"
if [[ "$actual_version" != "$python_version" ]]; then
  echo "CPython oracle version mismatch: expected $python_version, got $actual_version" >&2
  exit 1
fi

: "${CARGO_TARGET_DIR:=/tmp/minipython-target}"
export CARGO_TARGET_DIR
export RUST_MIN_STACK="${RUST_MIN_STACK:-8388608}"

run_python_harness_tests() {
  "$cpython" tests/test_pipeline.py
}

run_differential_discovery() {
  cargo build --bin mnpy
  "$cpython" tests/pipeline.py \
    --cpython "$cpython" \
    --require-version "$python_version" \
    --minipython "${CARGO_TARGET_DIR}/debug/mnpy" \
    --corpus tests/cases.toml \
    --out reports/test-pipeline \
    --generated-cases "$generated_cases" \
    --seed "$seed" \
    --shrink \
    --fail-priority must_fix,should_fix \
    "${driver_args[@]}"
}

if [[ "$mode" == "release" ]]; then
  MINIPYTHON_CPYTHON="$cpython" cargo test
elif [[ "$mode" == "focused" ]]; then
  cargo test --test product
  cargo test --test sandbox
  cargo test --test runtime sandbox_policy
  cargo test --test runtime instruction_budget
  cargo test --test runtime call_depth_guard
  cargo test --test runtime output_budget
  cargo test --test runtime allocation_budget
  cargo test --test parity manifest::docs_keep_product_scope_and_single_test_command
fi

run_python_harness_tests
run_differential_discovery
cargo fmt --check
git diff --check
