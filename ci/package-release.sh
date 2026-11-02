#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Package one release archive: minipython-<target>.tar.gz holding `mnpy`,
# LICENSE, LICENSE-PSF and NOTICE at the archive root. Used by
# .github/workflows/release.yml and by step (e) of tests/run.sh
# --release-gate, so the gate tests the exact packaging the release
# publishes.
#
# usage: ci/package-release.sh TARGET BINARY OUT_DIR
set -euo pipefail

target="${1:?target triple}"
binary="${2:?path to the built mnpy binary}"
out_dir="${3:?output directory}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT

cp "$binary" "$staging/mnpy"
chmod 755 "$staging/mnpy"
cp "$repo_root/LICENSE" "$staging/LICENSE"
cp "$repo_root/LICENSE-PSF" "$staging/LICENSE-PSF"
cp "$repo_root/NOTICE" "$staging/NOTICE"
mkdir -p "$out_dir"
archive="$out_dir/minipython-$target.tar.gz"
tar -C "$staging" -czf "$archive" mnpy LICENSE LICENSE-PSF NOTICE
echo "$archive"
