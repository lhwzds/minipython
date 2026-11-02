# minipython npm package

This package installs `mnpy`, the MiniPython sandbox for untrusted Python
code, through npm. It is a convenience wrapper around the same native binary
published in GitHub Releases.

The published package is this directory; the repository root carries the npm
workspace manifest. The `LICENSE`, `LICENSE-PSF`, and `NOTICE` files of this
directory are copies of the files at the repository root, so that the
published tarball carries the licences of the project.

## Installation

```bash
npm install -g minipython
```

## Usage

```bash
mnpy script.py          # run a file in the sandbox
mnpy -c "print(1+2)"    # execute a string
mnpy -e "1 + 2 * 3"     # evaluate an expression
mnpy --version
```

See the [README](https://github.com/lhwzds/minipython#readme) for the sandbox
contract, its limits, and what it does not protect against.

## Install note

The npm package downloads the matching prebuilt binary from GitHub Releases and
verifies it against the published `checksums.txt` file.

Supported release targets are:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl`

Windows is not supported: the sandbox's worker isolation uses Unix process
limits.

## Troubleshooting

If installation fails with an unsupported platform error, build from source:

```bash
git clone https://github.com/lhwzds/minipython
cd minipython
cargo build --release
```

If installation fails because a GitHub Release asset is unavailable, check that
the npm package version matches an existing MiniPython release tag.

If checksum verification fails, do not bypass it. Remove the partially installed
package and reinstall after confirming the release assets are correct:

```bash
npm uninstall -g minipython
npm install -g minipython
```
