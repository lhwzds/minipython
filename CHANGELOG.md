# Changelog

All notable changes to MiniPython are recorded here. Versions follow
[Semantic Versioning](https://semver.org/); before 1.0, a minor version may
change behavior or the embedding API.

## [0.1.0] - Unreleased

First public release: the `mnpy` command, the Rust `Sandbox` API, and the
pure-Python `minipython_sandbox` client, published as prebuilt binaries on
GitHub Releases, the `minipython` npm package, and the `lhwzds/tap` Homebrew
tap.

### What it is

- A **pure-compute sandbox** for untrusted Python code, such as code written
  by AI agents. Programs get the language, core runtime semantics, and a safe
  in-memory standard library subset: `builtins`, `sys`, `types`,
  `collections`, `collections.abc`, `math`, `math.integer`, `array`, `copy`,
  `io.BytesIO`, `io.StringIO`, `operator`, `functools`, `itertools`, `json`,
  `__future__`, `argparse`, `re`, `datetime`, and `csv`.
- **No files and no network.** `open()`, `input()`, `os`, `socket`,
  `subprocess`, `ctypes`, and every other host capability are absent; the
  only way out is a host function the embedding application registers by
  name. Blocked imports and builtins raise CPython's own exceptions and are
  recorded in `ExecutionResult::denials`.
- **The clock is readable.** `datetime` reads the host's real wall clock
  (the local time zone is always UTC); embedders can pin it with
  `Sandbox::with_clock`. Programs can therefore observe time, including the
  time their own code takes.
- Every run happens in a separate worker process with an empty environment,
  working directory `/`, no inherited descriptors, umask `077`, no core
  dumps, and instruction, allocation, output, source-size, wall-clock, and
  memory limits. Limit hits are uncatchable terminal conditions. A session
  owns one worker for its whole life; no worker is ever reused or shared.

### Dual licence

MiniPython is dual licensed. The code we wrote ourselves is MIT; the files
derived from CPython 3.14.6 (the `re`, `argparse`, `csv`, and `datetime`
modules, the exception reports, the annotation unparser, and the parts
listed in [NOTICE](NOTICE)) keep the PSF License 2.0 and are therefore
"PSF-2.0 AND MIT". The test cases are adapted from CPython's test suite and
are "MIT AND PSF-2.0". See [LICENSE](LICENSE), [LICENSE-PSF](LICENSE-PSF),
and [NOTICE](NOTICE).

### CPython 3.14.6 alignment

CPython 3.14.6 is the behavior oracle. The release gate checks it three
ways: about 4,300 tests in `cargo test`, most of them CPython differential
and subset tests; the 1024-case generated discovery sweep (794 exact
matches, 266 accepted sandbox or scope gaps, no open must-fix or should-fix
root cause); and oracle corpora of about 270,000 generated and extracted
scenarios for `re`, `datetime`, `csv`, `io.StringIO`, exception reports,
syntax, and `str` predicates, which must match CPython exactly apart from
documented exclusions. Alignment is not complete; see Known gaps.

### Security status

- Sandbox hardening part 1 is done: every host-touching call lives in
  `src/host/` and a lint fails on strays; worker start-up hygiene; one worker
  per execution and per session; the denial log; the capability contract
  tests; and typed, uncatchable terminal conditions.
- **The worker does not drop privileges yet** (no seccomp, Landlock, or
  macOS sandbox profile). It runs with the operating-system rights of the
  user who started it, so a bug in the interpreter could reach the host.
  Until privilege dropping lands, run untrusted or multi-tenant code inside
  a container or VM that you would be comfortable letting that code control.
- Report vulnerabilities privately; see [SECURITY.md](SECURITY.md).

### Known gaps

- A `SyntaxError` from `compile()`, `exec()`, or `eval()` has CPython's
  type, message, and attributes for the common errors; for less common ones
  `offset` and `end_offset` can differ from CPython's. A syntax error in the
  main program itself is still reported with MiniPython's internal message.
- Tracebacks have no `File "...", line N` frame lines: MiniPython does not
  track source lines yet.
- `zoneinfo`, `string`, `pickle`, and the rest of the standard library
  outside the list above are not provided.
- Deep recursion through large function bodies can stop short of CPython's
  1000-frame default under the default memory limits, and raised recursion
  limits reach far less deep than in CPython.
- `id()` values are derived from heap addresses.
- `datetime.strftime` renders every format the way macOS's C library does, so
  on Linux its results for years below 1000 and for unknown or
  platform-specific directives differ from Linux CPython (a documented,
  Linux-scoped exclusion; see tests/README.md, "Platform-dependent
  behavior").
- Unix only: macOS and Linux (x86-64 and arm64). There is no Windows build.
