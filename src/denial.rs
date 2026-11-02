// SPDX-License-Identifier: MIT

//! The denial log: what the sandbox refused during one execution.
//!
//! Blocking a capability is silent from the program's side: it sees exactly
//! the exception CPython raises when the name does not exist
//! (`ModuleNotFoundError`, `NameError`, `AttributeError`). The embedder sees
//! the refusal here, as structured [`Denial`]s in `ExecutionResult::denials`,
//! so an agent's attempt to reach the host is visible even when the program
//! catches the exception.
//!
//! The log is per interpreter thread, reset when a worker request starts,
//! and bounded: at most [`MAX_DENIALS`] distinct entries, each target at most
//! [`MAX_TARGET_BYTES`] bytes. A repeated denial increments its entry's
//! `count`; distinct denials beyond the bound are dropped.

use std::cell::RefCell;

use serde::{Deserialize, Serialize};

/// Most distinct denials one execution records.
pub const MAX_DENIALS: usize = 32;

/// Longest target recorded, in bytes; longer targets are cut at a character
/// boundary.
pub const MAX_TARGET_BYTES: usize = 128;

/// What kind of capability the sandbox refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DenialKind {
    /// An `import` (statement, `__import__`, or `from` import) of a module
    /// outside the sandbox allowlist and the module root, such as `os`,
    /// `socket`, or `subprocess`.
    Import,
    /// A host builtin the sandbox does not provide, such as `open` or
    /// `input`, looked up as a name.
    Builtin,
    /// A host attribute of an allowed module, such as `io.open` or
    /// `builtins.open`.
    Attribute,
}

/// One refused capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Denial {
    pub kind: DenialKind,
    /// The module name, builtin name, or `module.attribute`.
    pub target: String,
    /// How many times the program asked for it, saturating.
    pub count: u32,
}

/// CPython builtins that reach the host and are absent from the sandbox.
const HOST_BUILTINS: &[&str] = &[
    "open",
    "input",
    "help",
    "exit",
    "quit",
    "copyright",
    "credits",
    "license",
];

/// Host-reaching attributes of allowed modules that the sandbox omits.
const HOST_ATTRIBUTES: &[(&str, &str)] = &[("io", "open"), ("io", "open_code"), ("io", "FileIO")];

thread_local! {
    static LOG: RefCell<Vec<Denial>> = const { RefCell::new(Vec::new()) };
}

fn record(kind: DenialKind, target: &str) {
    LOG.with(|log| {
        let mut log = log.borrow_mut();
        let target = bounded(target);
        if let Some(entry) = log
            .iter_mut()
            .find(|entry| entry.kind == kind && entry.target == target)
        {
            entry.count = entry.count.saturating_add(1);
        } else if log.len() < MAX_DENIALS {
            log.push(Denial {
                kind,
                target: target.to_string(),
                count: 1,
            });
        }
    });
}

fn bounded(target: &str) -> &str {
    if target.len() <= MAX_TARGET_BYTES {
        return target;
    }
    let mut end = MAX_TARGET_BYTES;
    while !target.is_char_boundary(end) {
        end -= 1;
    }
    &target[..end]
}

/// Records an import the sandbox policy blocked.
pub(crate) fn record_blocked_import(module: &str) {
    record(DenialKind::Import, module);
}

/// Records a failed name lookup when the name is a host builtin.
pub(crate) fn record_missing_name(name: &str) {
    if HOST_BUILTINS.contains(&name) {
        record(DenialKind::Builtin, name);
    }
}

/// Records a failed module attribute lookup when the attribute is a host
/// capability the sandbox omits.
pub(crate) fn record_missing_module_attribute(module: &str, name: &str) {
    let host = if module == "builtins" {
        HOST_BUILTINS.contains(&name)
    } else {
        HOST_ATTRIBUTES.contains(&(module, name))
    };
    if host {
        record(DenialKind::Attribute, &format!("{module}.{name}"));
    }
}

/// Clears this thread's log at the start of an execution.
pub(crate) fn reset() {
    LOG.with(|log| log.borrow_mut().clear());
}

/// Takes this thread's log.
pub(crate) fn take() -> Vec<Denial> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denials_are_deduplicated_counted_and_bounded() {
        std::thread::spawn(|| {
            record_blocked_import("os");
            record_blocked_import("os");
            record_missing_name("open");
            record_missing_name("not_a_host_builtin");
            record_missing_module_attribute("io", "open");
            record_missing_module_attribute("io", "StringIO");
            record_missing_module_attribute("builtins", "input");
            assert_eq!(
                take(),
                vec![
                    Denial {
                        kind: DenialKind::Import,
                        target: "os".into(),
                        count: 2
                    },
                    Denial {
                        kind: DenialKind::Builtin,
                        target: "open".into(),
                        count: 1
                    },
                    Denial {
                        kind: DenialKind::Attribute,
                        target: "io.open".into(),
                        count: 1
                    },
                    Denial {
                        kind: DenialKind::Attribute,
                        target: "builtins.input".into(),
                        count: 1
                    },
                ]
            );
            for index in 0..100 {
                record_blocked_import(&format!("module{index}"));
            }
            record_blocked_import(&"x".repeat(1000));
            let log = take();
            assert_eq!(log.len(), MAX_DENIALS);
            assert_eq!(log.last().map(|d| d.target.as_str()), Some("module31"));
            record_blocked_import(&"é".repeat(100));
            assert!(take()[0].target.len() <= MAX_TARGET_BYTES);
            record_blocked_import("os");
            reset();
            assert!(take().is_empty());
        })
        .join()
        .expect("denial thread");
    }
}
