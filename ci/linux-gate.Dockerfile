# SPDX-License-Identifier: MIT
# Linux release gate: runs `tests/run.sh --release-gate` for a candidate
# commit on Linux (RELEASING.md, "Linux gate").
#
#   git bundle create /tmp/minipython-candidate.bundle HEAD
#   docker build -f ci/linux-gate.Dockerfile -t minipython-linux-gate ci
#   mkdir -p /tmp/minipython-linux-gate
#   docker run --rm \
#     -v /tmp/minipython-candidate.bundle:/in/candidate.bundle:ro \
#     -v /tmp/minipython-linux-gate:/out \
#     minipython-linux-gate
#
# The report lands in /tmp/minipython-linux-gate/report.md (and .json, logs/).
# The image holds only tools; the candidate arrives as a git bundle so the
# gate sees a clean checkout with its full history (the leak scan reads every
# commit since the last release tag).

# The official CPython 3.14.6 image is the oracle (/usr/local/bin/python3).
FROM python:3.14.6-bookworm

ARG RUST_VERSION=1.95.0
ARG CPYTHON_SRC_SHA256=143b1dddefaec3bd2e21e3b839b34a2b7fb9842272883c576420d605e9f30c63

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl file git musl-tools nodejs npm xz-utils \
 && rm -rf /var/lib/apt/lists/*

# The CPython v3.14.6 source tree, for Grammar/python.gram (not installed)
# and for the Lib/test files the gate reads as oracle input: the official
# image strips Lib/test, so MINIPYTHON_CPYTHON_LIB points at the tarball's
# Lib/ (the gate's preflight checks it matches the installed stdlib).
RUN curl -fsSL -o /tmp/Python-3.14.6.tar.xz https://www.python.org/ftp/python/3.14.6/Python-3.14.6.tar.xz \
 && echo "${CPYTHON_SRC_SHA256}  /tmp/Python-3.14.6.tar.xz" | sha256sum -c - \
 && mkdir -p /opt/cpython-3.14.6-src \
 && tar -xJf /tmp/Python-3.14.6.tar.xz -C /opt/cpython-3.14.6-src --strip-components 1 \
 && rm /tmp/Python-3.14.6.tar.xz \
 && cp -r /opt/cpython-3.14.6-src/Lib/test "$(python3 -c 'import sysconfig; print(sysconfig.get_path("stdlib"))')/test"

# Some oracle programs import CPython's own test package (for example
# test.typinganndata), so the oracle needs it installed, not just readable.

# An unprivileged user: some sandbox tests check that the worker cannot reach
# files, which root could.
RUN useradd --create-home --shell /bin/bash gate \
 && mkdir -p /work /out /opt/rust \
 && chown gate:gate /work /out /opt/rust
USER gate
ENV RUSTUP_HOME=/opt/rust/rustup \
    CARGO_HOME=/opt/rust/cargo \
    PATH=/opt/rust/cargo/bin:$PATH

RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "${RUST_VERSION}" --component rustfmt \
 && rustup target add "$(uname -m)-unknown-linux-musl"

ENV MINIPYTHON_CPYTHON=/usr/local/bin/python3 \
    MINIPYTHON_CPYTHON_SRC=/opt/cpython-3.14.6-src \
    MINIPYTHON_CPYTHON_LIB=/opt/cpython-3.14.6-src/Lib \
    CARGO_TARGET_DIR=/work/target \
    RUST_MIN_STACK=8388608 \
    CARGO_TERM_COLOR=never

COPY --chown=gate:gate linux-gate-entrypoint.sh /usr/local/bin/linux-gate-entrypoint.sh
WORKDIR /work
ENTRYPOINT ["/usr/local/bin/linux-gate-entrypoint.sh"]
