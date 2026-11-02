# Security Policy

MiniPython runs untrusted Python code, so a way out of the sandbox is the
most serious bug it can have. Please report it privately first.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting: open the repository's
**Security** tab and choose **Report a vulnerability**
(<https://github.com/lhwzds/minipython/security/advisories/new>). Do not open a
public issue, pull request, or discussion for a vulnerability.

Please include:

- the MiniPython version (`mnpy --version`) and how it was installed,
- the host operating system and architecture,
- the program or embedding code that shows the problem, and the command line,
- what you expected and what happened.

You will get an acknowledgement within seven days. Fixes are released as a new
patch version, and the advisory is published once a fixed release is
available. Reporters are credited unless they ask not to be.

## What counts

In scope, for example:

- Python code run by `mnpy`, `Sandbox`, `SandboxSession`, or the Python
  client that reads or writes host files, opens network connections, starts
  processes, reads the worker's or the embedder's environment, or otherwise
  reaches a host capability the embedder did not register.
- Code that survives or escapes a limit that should end the run (instruction,
  allocation, output, source size, wall-clock, memory), or that crashes the
  embedding process rather than its worker.
- State leaking between executions or sessions.
- Values other than the documented inert data crossing the worker protocol.

Out of scope:

- Anything a registered host function does: host functions are trusted code
  of the embedding application.
- Denial of service within the configured limits, such as a program that uses
  its whole budget.
- The documented limits of the current release, notably that the worker does
  not drop operating-system privileges yet (see `CHANGELOG.md`), and that
  programs can read the clock.

## Supported versions

MiniPython is pre-1.0. Only the latest release receives security fixes.

| Version | Supported |
| --- | --- |
| 0.1.x | Yes |
| Earlier, unreleased builds | No |
