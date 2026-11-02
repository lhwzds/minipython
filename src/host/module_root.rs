// SPDX-License-Identifier: MIT

//! Module-root reading: the only host file access.
//!
//! A sandbox module root (`mnpy file.py`'s directory, `--root`, or
//! `Sandbox::with_root`) is read once, before any Python runs, into
//! in-memory [`VirtualModule`]s. The VM never touches the file system: an
//! `import` finds source text in that table. Every path is canonicalized and
//! must stay below the canonical root, so a symlink cannot reach outside it.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{VirtualModule, is_python_module_component, join_sandbox_module_name};

/// Reads the importable modules below `root`, unsorted.
pub(crate) fn read_modules(root: &Path) -> Result<Vec<VirtualModule>, String> {
    let root = canonicalize_sandbox_root(root)?;
    let mut modules = Vec::new();
    collect_sandbox_modules(&root, &root, None, &mut modules)?;
    Ok(modules)
}

fn canonicalize_sandbox_root(root: &Path) -> Result<PathBuf, String> {
    let root = fs::canonicalize(root)
        .map_err(|error| sandbox_io_error("cannot open module root", root, error))?;
    let metadata = fs::metadata(&root)
        .map_err(|error| sandbox_io_error("cannot stat module root", &root, error))?;
    if !metadata.is_dir() {
        return Err(format!(
            "sandbox error: module root is not a directory: {}",
            root.display()
        ));
    }
    Ok(root)
}

fn collect_sandbox_modules(
    root: &Path,
    dir: &Path,
    package_name: Option<String>,
    modules: &mut Vec<VirtualModule>,
) -> Result<(), String> {
    for entry in sandbox_dir_entries(dir)? {
        let name = sandbox_entry_name(&entry)?;
        if should_skip_sandbox_entry(&name) {
            continue;
        }

        let path = entry.path();
        let canonical_path = canonicalize_sandbox_child(root, &path)?;
        let metadata = fs::metadata(&canonical_path)
            .map_err(|error| sandbox_io_error("cannot stat module path", &canonical_path, error))?;

        if metadata.is_dir() {
            collect_sandbox_package_dir(
                root,
                &canonical_path,
                &name,
                package_name.as_deref(),
                modules,
            )?;
        } else if metadata.is_file() {
            collect_sandbox_module_file(
                root,
                &canonical_path,
                &name,
                package_name.as_deref(),
                modules,
            )?;
        }
    }

    Ok(())
}

fn collect_sandbox_package_dir(
    root: &Path,
    dir: &Path,
    dir_name: &str,
    parent_package: Option<&str>,
    modules: &mut Vec<VirtualModule>,
) -> Result<(), String> {
    if !is_python_module_component(dir_name) {
        // Like CPython's path finder, a directory that is not a valid module
        // name is simply not importable; it must not abort the whole run.
        return Ok(());
    }
    let Some(init_path) = sandbox_package_init_path(root, dir)? else {
        return Ok(());
    };
    let module_name = join_sandbox_module_name(parent_package, dir_name);
    let source = read_sandbox_source_file(root, &init_path)?;
    modules.push(VirtualModule::package(module_name.clone(), source));
    collect_sandbox_modules(root, dir, Some(module_name), modules)
}

fn collect_sandbox_module_file(
    root: &Path,
    path: &Path,
    file_name: &str,
    package_name: Option<&str>,
    modules: &mut Vec<VirtualModule>,
) -> Result<(), String> {
    if file_name == "__init__.py" {
        return Ok(());
    }
    if path.extension().and_then(|extension| extension.to_str()) != Some("py") {
        return Ok(());
    }
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return Err(format!(
            "sandbox error: non-UTF-8 module filename: {}",
            path.display()
        ));
    };
    if !is_python_module_component(stem) {
        // Files such as `self-test.py` cannot be imported by name, but they
        // may be the entry script itself; skip them instead of failing.
        return Ok(());
    }
    let module_name = join_sandbox_module_name(package_name, stem);
    let source = read_sandbox_source_file(root, path)?;
    modules.push(VirtualModule::module(module_name, source));
    Ok(())
}

fn sandbox_dir_entries(dir: &Path) -> Result<Vec<fs::DirEntry>, String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|error| sandbox_io_error("cannot read module directory", dir, error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| sandbox_io_error("cannot read module directory entry", dir, error))?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

fn sandbox_entry_name(entry: &fs::DirEntry) -> Result<String, String> {
    entry.file_name().into_string().map_err(|_| {
        format!(
            "sandbox error: non-UTF-8 module path component: {}",
            entry.path().display()
        )
    })
}

fn should_skip_sandbox_entry(name: &str) -> bool {
    name.starts_with('.') || name == "__pycache__"
}

fn sandbox_package_init_path(root: &Path, dir: &Path) -> Result<Option<PathBuf>, String> {
    let init_path = dir.join("__init__.py");
    match fs::symlink_metadata(&init_path) {
        Ok(metadata) if metadata.file_type().is_file() || metadata.file_type().is_symlink() => {
            Ok(Some(canonicalize_sandbox_child(root, &init_path)?))
        }
        Ok(_) => Err(format!(
            "sandbox error: package initializer is not a file: {}",
            init_path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(sandbox_io_error(
            "cannot stat package initializer",
            &init_path,
            error,
        )),
    }
}

fn read_sandbox_source_file(root: &Path, path: &Path) -> Result<String, String> {
    let path = canonicalize_sandbox_child(root, path)?;
    fs::read_to_string(&path)
        .map_err(|error| sandbox_io_error("cannot read module source", &path, error))
}

fn canonicalize_sandbox_child(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| sandbox_io_error("cannot open module path", path, error))?;
    if !canonical.starts_with(root) {
        return Err(format!(
            "sandbox error: module path escapes sandbox root: {}",
            path.display()
        ));
    }
    Ok(canonical)
}

fn sandbox_io_error(action: &str, path: &Path, error: std::io::Error) -> String {
    format!("sandbox error: {action}: {}: {error}", path.display())
}
