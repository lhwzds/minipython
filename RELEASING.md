# Releasing MiniPython

Every release must pass the complete release gate first: the automated gate
on macOS and on Linux, and the manual checklist it writes. A release that
skipped any part of it is not a MiniPython release.

## Release gate

```bash
export MINIPYTHON_CPYTHON=/opt/homebrew/bin/python3      # CPython 3.14.6 (this is the default)
export MINIPYTHON_CPYTHON_SRC=/path/to/cpython-3.14.6    # source tree, for Grammar/python.gram
tests/run.sh --release-gate
```

The source tree is the CPython 3.14.6 tarball from python.org
(`Python-3.14.6.tar.xz`, SHA-256
`143b1dddefaec3bd2e21e3b839b34a2b7fb9842272883c576420d605e9f30c63`) or a
shallow clone of the `v3.14.6` tag. The gate needs `cargo`, `node`, `npm`,
`git`, and, on macOS, Homebrew.

The gate stops at the first failing step and writes
`reports/release-gate/report.md` and `report.json`, with every command's log
under `reports/release-gate/logs/`. `--keep-going` runs every step and reports
every failure; the run still fails. `--allow-dirty` and `--only` exist for
working on the gate itself and never produce a passing run of a release
candidate. The steps, the oracle corpora, and their documented exclusions are
described in `tests/README.md` ("Release Gate"):

| Step | Checks |
| --- | --- |
| preflight | committed tree; CPython 3.14.6 oracle and source tree; one version across `Cargo.toml`, `npm/package.json`, `python/pyproject.toml`, and `sys.implementation`; a `CHANGELOG.md` entry |
| a | `cargo fmt --check`; `cargo test --release --all-targets --locked --no-fail-fast` with `MINIPYTHON_REQUIRE_CPYTHON_SRC=1`: no failure, no `SKIPPED` evidence, no undocumented ignored test; this includes the capability-contract, `host_capability_lint`, worker-isolation, and budget suites |
| b | pipeline unit tests; the curated corpus plus 1024 generated cases (seed 20260710) with exactly the counts in `tests/release_gate/discovery-baseline.json` |
| c | the oracle corpora of `tests/oracles/corpora.toml`, identical to CPython apart from listed exclusions |
| conformance | the layer-zero conformance corpus of `tests/conformance/`: the comparison of the release `mnpy` with the recorded CPython 3.14.6 observations, the conformance status block of `README.md` (a stale block fails the step), the oracle check of `tests/conformance/cases` (the recorded observations are compared with the pinned CPython, which writes no oracle or report; the cases it executes are trusted, tracked code, and any difference fails the step), and the unit tests of the corpus |
| d | leak scan of tracked files and of commit messages and identities since the last `v*` tag |
| licensing | the dual licence: an `SPDX-License-Identifier:` header in a comment (or a `REUSE.toml` path rule) for every tracked source file, only `MIT`, `PSF-2.0 AND MIT`, and `MIT AND PSF-2.0`; a line that starts with `// Derived from CPython` (or `# Derived from CPython`), a `Changes:` line, and a `NOTICE` entry for every PSF-2.0 file; a NOTICE that holds nothing but paths, one per line, and no stale entry (every path NOTICE lists exists and carries a PSF-2.0 header of its own); `LICENSES/MIT.txt` and `LICENSES/PSF-2.0.txt`; no Cargo dependency outside the licence allowlist; and no tracked Rust file that declares no PSF-2.0 licence and shares four or more internal names with a CPython source file |
| e | release archive, `npm pack` and install with a local archive, `mnpy --version` and example runs against CPython, the shipped binary's math probe (`tests/release_gate/math_probe.py`), Homebrew formula install, `brew test`, and `brew audit --strict` from a throwaway local tap (macOS) |
| f | the manual checklist, written into the report |

A changed discovery count fails the gate. When the change is intended (a fixed
gap, a new generator template), run
`tests/run.sh --release-gate --update-discovery-baseline`, which rewrites
`tests/release_gate/discovery-baseline.json` from the run and still fails, and
commit that file on its own with the reason for every changed count. The same
holds for oracle exclusions and leak-allowlist entries: each needs its reason
in the file, and an entry that no longer applies fails the gate until it is
removed.

### Linux gate

Run the same gate on a Linux Docker host for the same commit:

```bash
git bundle create /tmp/minipython-candidate.bundle HEAD
docker build -f ci/linux-gate.Dockerfile -t minipython-linux-gate ci
mkdir -p /tmp/minipython-linux-gate
docker run --rm \
  -v /tmp/minipython-candidate.bundle:/in/candidate.bundle:ro \
  -v /tmp/minipython-linux-gate:/out \
  minipython-linux-gate
```

The image is the official `python:3.14.6-bookworm` image (the oracle) plus
Rust, the host's musl target and `musl-tools`, Node.js, and the CPython 3.14.6
source tree, whose `Lib/` also supplies the `Lib/test` files the official
image strips (`MINIPYTHON_CPYTHON_LIB`). The container clones the candidate from the bundle, runs
`tests/run.sh --release-gate --keep-going` as an unprivileged user, and copies
the report to `/tmp/minipython-linux-gate/report.md`. Set `-e CANDIDATE=<sha>`
to gate another commit of the bundle. On Linux, step (e) builds and smoke-tests
the musl binary the release ships; the Homebrew install is covered by the
macOS gate.

### Manual checklist (step f)

The report ends with these items and blanks to fill in. All three must be
complete before tagging:

1. **Real-program corpus rerun** on the candidate binary: no regression against
   the previous release's run. For the first release, record it as the baseline.
2. **Dual review** of `git diff <previous tag>..<candidate>` (for the first
   release, the whole tree) by two independent reviewers (bugs, sandbox escapes,
   leaks, provenance). Verify every finding; fix every confirmed
   blocker, then rerun the gate.
3. **Linux gate** result for the same commit (above).

## Release steps

1. Pick the version (pre-1.0: a minor bump for behavior or API changes, a patch
   bump for fixes) and set it in `Cargo.toml` (then `cargo update -p
   minipython`), `npm/package.json`, `python/pyproject.toml`, and the
   `MINIPYTHON_VERSION_*` constants in `src/stdlib.rs`.
2. Move the `Unreleased` entry in `CHANGELOG.md` to the version and date.
3. Commit, and run the release gate on macOS and Linux for that commit; fill in
   the manual checklist. Fix anything that fails and start again.
4. Merge to `main` and wait for the `Tests` workflow on `main` to pass for that
   exact commit (it runs the release gate on Linux and macOS).
5. Tag the commit and push the tag (`git tag -a v0.1.0 -m "MiniPython 0.1.0"`,
   `git push origin v0.1.0`). Pushing a tag needs the owner's approval.
6. The `Release` workflow then:
   - checks that the tag is the latest `origin/main`, that `Tests` passed for
     it, and that the tag matches the three package versions and has a
     `CHANGELOG.md` entry, and dry-runs the npm publish;
   - builds `aarch64-apple-darwin`, `x86_64-apple-darwin`,
     `x86_64-unknown-linux-musl`, and `aarch64-unknown-linux-musl`,
     smoke-tests the native ones, and packages each with
     `ci/package-release.sh`;
   - creates the GitHub Release with the four archives, `checksums.txt`, and
     the `CHANGELOG.md` section as notes;
   - publishes the `minipython` npm package with provenance through npm
     trusted publishing.
7. Homebrew: the tap's scheduled `Update Formula` workflow picks the release
   up within six hours (`homebrew-tap-template/README.md`); run it by hand to
   update at once.
8. Check the published artifacts: `npm install -g minipython`,
   `brew install lhwzds/tap/minipython`, and `mnpy --version` on each.

### One-time setup

- npm: trusted publishing is configured per package on npmjs.com, and the
  package must exist first. For 0.1.0 the `Publish npm` job therefore cannot
  authenticate and fails after the GitHub Release is created. Publish 0.1.0
  once by hand from the tagged commit (`cd npm && npm publish --access
  public`), then add the trusted publisher (repository `lhwzds/minipython`,
  workflow `release.yml`) so later releases need no token.
- GitHub: enable private vulnerability reporting (Settings, Security) so the
  route in `SECURITY.md` works.
- Homebrew: add `Formula/minipython.rb` and the `update-minipython` job to
  `lhwzds/homebrew-tap` (`homebrew-tap-template/README.md`).

The workflows use only the automatic `GITHUB_TOKEN` and npm's OIDC trusted
publishing; no other secret is stored.
