# Layer-zero conformance

Compare the checked-in cases with MiniPython without CPython:

```sh
python3 -I tests/conformance/run.py --mnpy path/to/mnpy --out path/to/report.json
python3 -I tests/conformance/run.py --mnpy path/to/mnpy --out path/to/report.json --leaf identity-alias
```

Each case has a neighboring, human-readable `.expected` file. Check the
recorded observations against the pinned CPython 3.14.6 interpreter. The check
writes no oracle or report; the cases it executes are trusted, tracked code:

```sh
python3 -I tests/conformance/run.py --check-oracle --cpython path/to/python3
```

The check exits 0 when every observation matches, exits 1 with the differing
cases on stderr when one does not, and exits 2 on a runner error: a different
version, a case or an expected file without its partner, or a timeout. The
release gate runs it, so an observation that differs from the oracle on the
host fails the gate as oracle drift.

To rewrite the observations, run the explicit maintenance command:

```sh
python3 -I tests/conformance/run.py --update-oracle --cpython path/to/python3
```

The update refuses a different version or a timeout, and writes changed files
atomically. CPython is run with `-s`. Cases are black-box observations; do not
copy CPython source or its tests.

`expected-failures.toml` records the current MiniPython differences and their
reasons and SHA-256 digests of their canonical JSON differences. A full comparison
fails if a listed case now matches or changes its differences, or if an unlisted
difference appears. A leaf run checks entries for that leaf. Exit status is 0
for matches and accepted failures, 1 for unexpected or stale differences, and 2
for runner errors. The report lists every difference.
To re-accept a reviewed change, copy the `digest` from the report into the
matching entry of `expected-failures.toml`.

The conformance status block of the root `README.md` is generated from such a
report:

```sh
python3 -I tests/conformance/status.py --report path/to/report.json
```

The block holds the oracle version, the totals, and one row per protocol. Only
the numbers of the report appear in it, so the same report always writes the
same bytes. `status.py --report path/to/report.json --check` writes nothing
and exits 1 when the block of `README.md` differs from the one the report
implies; the release gate runs it, so a stale block fails the gate.

Stdout, stderr, and exit status are compared in full. JSON cannot tell a tuple
from a list, so cases where that matters emit `repr`. Incidental object addresses are normalized. Each process
has a bounded timeout and runs in the cases directory with a minimal environment.

Add at least three distinct cases for a new leaf in `tree/layer0.toml`, then
update expected files and record any observed failures with specific reasons.
Run the runner tests with:

```sh
python3 -I -m unittest discover -s tests/conformance -p 'test_*.py'
```

The same command runs `test_gate_step.py`, the unit tests of the release
gate's conformance step: they drive the step with stubbed commands and check
its verdicts, so a broken step fails without a release build.

The `sort_keys` setting in trace.py hides dict insertion order, so a case that cares emits a list.
The provenance name scan covers Rust only; this corpus is covered by review.
