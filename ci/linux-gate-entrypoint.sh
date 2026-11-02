#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Entry point of ci/linux-gate.Dockerfile: clone the candidate from the
# mounted bundle, run the release gate, and copy its report to /out.
#
# Environment: CANDIDATE (a revision in the bundle, default HEAD) and
# GATE_ARGS (extra gate flags, default --keep-going so every failure is
# reported).
set -euo pipefail

bundle=/in/candidate.bundle
if [[ ! -f "$bundle" ]]; then
  echo "mount the candidate bundle at $bundle (see ci/linux-gate.Dockerfile)" >&2
  exit 2
fi

git clone --quiet "$bundle" /work/minipython
cd /work/minipython
git checkout --quiet --detach "${CANDIDATE:-HEAD}"
echo "candidate: $(git rev-parse HEAD)"

status=0
# shellcheck disable=SC2086
tests/run.sh --release-gate ${GATE_ARGS:---keep-going} || status=$?

if [[ -d reports/release-gate ]]; then
  cp -R reports/release-gate/. /out/
fi
echo "release gate exit status: $status (report: /out/report.md)"
exit "$status"
