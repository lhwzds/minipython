// SPDX-License-Identifier: MIT

//! Host capabilities: the only code in the interpreter that touches host
//! state.
//!
//! Everything the sandbox reads from or does to the host beyond its own
//! memory goes through this module, so a reviewer can audit it in one place
//! and a VM bug cannot reach the host through code that is not here:
//!
//! - [`clock`]: the wall clock `datetime` reads (or a pinned instant), the
//!   always-UTC local time zone, and the worker's usage stopwatch;
//! - [`module_root`]: reading a sandbox module root into in-memory source
//!   before any Python runs, the only file access;
//! - [`worker`]: the worker process's start-up hygiene (environment, current
//!   directory, inherited descriptors, umask, core dumps) and its protocol
//!   channel on stdin/stdout/stderr.
//!
//! The VM itself has no host access: environment variables, processes,
//! network, entropy, and the current directory are not reachable from
//! Python at all.
//!
//! Split with the parent: supervising a worker (spawning it with a cleared
//! environment and memory limits, polling its footprint, enforcing the
//! wall-clock deadline, killing and reaping it) is parent-side code in
//! `product.rs`; the `mnpy` command line (arguments, the script file, exit
//! status) is in `sandbox.rs`; the Python binding's broker locates its own
//! executable in `binding.rs`. Those call sites are listed, with reasons, in
//! the host-capability lint (`tests/sandbox.rs`, `host_capability_lint`),
//! which fails on any other host-touching call outside this module.

pub(crate) mod clock;
pub(crate) mod module_root;
pub(crate) mod worker;
