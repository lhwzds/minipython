// SPDX-License-Identifier: MIT

//! Worker-side process capabilities.
//!
//! A worker talks to its parent only through length-framed messages on
//! stdin and stdout, and reports protocol failures on stderr. Before it reads
//! its first request, [`harden_worker_process`] takes away everything else it
//! inherited from the parent.

use std::io;

/// Working directory of every worker. `/` exists on every Unix host, needs
/// no creation or cleanup (an empty temporary directory would need both, and
/// a crashed or killed worker would leak it), and is not writable by an
/// unprivileged worker. The worker never resolves a relative path after
/// start-up: the parent sends the module root as an absolute path.
pub(crate) const WORKER_CWD: &str = "/";

/// File-mode creation mask of every worker: nothing it might create is
/// readable or writable by other users.
pub(crate) const WORKER_UMASK: u32 = 0o077;

/// Applies the worker's start-up hygiene. Call it first in a worker, while
/// the process is still single-threaded and before it reads a request:
///
/// - closes every inherited descriptor except stdin, stdout, and stderr
///   (the protocol channel), so a descriptor the embedder leaked without
///   close-on-exec cannot be reached;
/// - clears the whole environment. The allowlist of kept variables is empty
///   because nothing in the worker reads one: the internal worker marker was
///   checked before this runs, the interpreter stack size is explicit
///   (`RUST_MIN_STACK` is irrelevant), and the local time zone is fixed to
///   UTC (`TZ` is never read);
/// - changes the working directory to [`WORKER_CWD`];
/// - sets the umask to [`WORKER_UMASK`];
/// - sets `RLIMIT_CORE` to zero, soft and hard, so a crash never writes a
///   core file holding the worker's memory and the limit cannot be raised;
/// - with GNU libc, keeps every thread's allocations in the one main heap
///   (see [`limit_malloc_arenas`]).
pub(crate) fn harden_worker_process() -> Result<(), String> {
    limit_malloc_arenas();
    close_inherited_descriptors()?;
    clear_environment();
    std::env::set_current_dir(WORKER_CWD)
        .map_err(|error| format!("cannot change worker directory to {WORKER_CWD}: {error}"))?;
    // SAFETY: umask only replaces the process's file-mode creation mask.
    unsafe {
        libc::umask(WORKER_UMASK as libc::mode_t);
    }
    disable_core_dumps()
}

/// GNU libc gives each new thread that allocates its own malloc arena and
/// reserves 64 MiB of address space for it. The process memory limit on
/// Linux is `RLIMIT_AS`, which counts those reservations, so the worker's
/// two helper threads alone would hold half of the default limit before the
/// program allocates anything. One arena keeps the limit for real memory;
/// musl, which the Linux release links, has no arenas.
fn limit_malloc_arenas() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: mallopt only changes a malloc tuning parameter; the process is
    // still single-threaded.
    unsafe {
        libc::mallopt(libc::M_ARENA_MAX, 1);
    }
}

fn clear_environment() {
    let names = std::env::vars_os()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    for name in names {
        // SAFETY: the worker is still single-threaded (see
        // `harden_worker_process`), so nothing reads the environment
        // concurrently.
        unsafe {
            std::env::remove_var(name);
        }
    }
}

fn disable_core_dumps() -> Result<(), String> {
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is a valid rlimit for the duration of the call.
    if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) } != 0 {
        return Err(format!(
            "cannot disable worker core dumps: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(())
}

/// Closes every descriptor above stderr.
fn close_inherited_descriptors() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: close_range only closes descriptors, and fails cleanly
        // (ENOSYS) on kernels older than 5.9.
        if unsafe { libc::syscall(libc::SYS_close_range, 3_u32, u32::MAX, 0_u32) } == 0 {
            return Ok(());
        }
    }
    for fd in open_descriptors_above_stderr()? {
        // SAFETY: closing a descriptor number. One that is no longer open
        // (the listing's own directory handle) fails harmlessly with EBADF.
        unsafe {
            libc::close(fd);
        }
    }
    Ok(())
}

/// Descriptors above stderr that are open now, from the kernel's listing
/// where there is one, otherwise by probing every number below the
/// descriptor limit.
fn open_descriptors_above_stderr() -> Result<Vec<libc::c_int>, String> {
    let listing = if cfg!(target_os = "linux") {
        "/proc/self/fd"
    } else {
        "/dev/fd"
    };
    if let Ok(entries) = std::fs::read_dir(listing) {
        // The listing's own descriptor appears in it; it is closed when
        // `entries` is dropped, before the caller closes the others.
        return Ok(entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str()?.parse::<libc::c_int>().ok())
            .filter(|fd| *fd > 2)
            .collect());
    }
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is valid writable memory for the call.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
        return Err(format!(
            "cannot read the worker descriptor limit: {}",
            io::Error::last_os_error()
        ));
    }
    let highest = libc::c_int::try_from(limit.rlim_cur.min(1 << 20)).unwrap_or(1 << 20);
    Ok((3..highest)
        // SAFETY: F_GETFD only reads descriptor flags.
        .filter(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) } != -1)
        .collect())
}

/// The parent's request stream.
pub(crate) fn protocol_input() -> io::Stdin {
    io::stdin()
}

/// The stream the worker answers on.
pub(crate) fn protocol_output() -> io::Stdout {
    io::stdout()
}

/// Reports a protocol failure to the parent's diagnostics pipe.
pub(crate) fn report_protocol_error(message: &str) {
    eprintln!("worker protocol error: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    const PROBE_ENV: &str = "MINIPYTHON_WORKER_SETUP_PROBE";
    const PROBE_FD_ENV: &str = "MINIPYTHON_WORKER_SETUP_FD";
    const SECRET_ENV: &str = "MINIPYTHON_TEST_SECRET_API_TOKEN";
    const THIS_TEST: &str =
        "host::worker::tests::worker_setup_clears_environment_directory_descriptors_and_core_dumps";

    fn descriptor_is_open(fd: libc::c_int) -> bool {
        // SAFETY: F_GETFD only reads descriptor flags.
        unsafe { libc::fcntl(fd, libc::F_GETFD) != -1 }
    }

    /// Runs in a re-executed copy of the test binary, so changing the
    /// environment, directory, and descriptors cannot disturb other tests.
    fn probe() {
        let inherited = std::env::var(PROBE_FD_ENV)
            .expect("probe receives the inherited descriptor number")
            .parse::<libc::c_int>()
            .expect("descriptor number");
        assert!(
            descriptor_is_open(inherited),
            "the probe inherits fd {inherited}"
        );
        assert!(
            std::env::var_os(SECRET_ENV).is_some(),
            "the probe sees the secret"
        );

        harden_worker_process().expect("worker setup succeeds");

        assert_eq!(std::env::vars_os().count(), 0, "environment is empty");
        assert_eq!(
            std::env::current_dir().expect("cwd"),
            std::path::Path::new(WORKER_CWD)
        );
        let still_open = (3..4096)
            .filter(|fd| descriptor_is_open(*fd))
            .collect::<Vec<_>>();
        assert!(
            still_open.is_empty(),
            "descriptors left open: {still_open:?}"
        );
        for fd in 0..=2 {
            assert!(descriptor_is_open(fd), "fd {fd} stays open");
        }
        // SAFETY: umask only replaces the mask; the old value is restored.
        let mask = unsafe {
            let mask = libc::umask(0o022);
            libc::umask(mask);
            mask
        };
        assert_eq!(u32::from(mask), WORKER_UMASK);
        let mut core = libc::rlimit {
            rlim_cur: 1,
            rlim_max: 1,
        };
        // SAFETY: `core` is valid writable memory for the call.
        assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut core) }, 0);
        assert_eq!((core.rlim_cur, core.rlim_max), (0, 0));
        println!("worker setup probe passed");
    }

    #[test]
    fn worker_setup_clears_environment_directory_descriptors_and_core_dumps() {
        if std::env::var_os(PROBE_ENV).is_some() {
            probe();
            return;
        }
        let executable = std::env::current_exe().expect("test binary");
        let leaked = std::fs::File::open(&executable).expect("open a file to leak");
        let fd = std::os::fd::AsRawFd::as_raw_fd(&leaked);
        // SAFETY: clears close-on-exec on a descriptor this test owns, so the
        // child inherits it like a descriptor an embedder leaked.
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFD);
            assert_ne!(flags, -1);
            assert_eq!(libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC), 0);
        }
        let output = Command::new(&executable)
            .args([THIS_TEST, "--exact", "--nocapture", "--test-threads=1"])
            .env(PROBE_ENV, "1")
            .env(PROBE_FD_ENV, fd.to_string())
            .env(SECRET_ENV, "sk-live-must-never-reach-a-worker")
            .output()
            .expect("run the probe");
        drop(leaked);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("worker setup probe passed"),
            "probe failed\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
