"""Licensing check for the MiniPython release gate.

usage: licensing.py [--repo DIR] [--cpython-src DIR] [--json OUT]

MiniPython is dual licensed: the files we wrote ourselves are MIT, the
files derived from CPython are "PSF-2.0 AND MIT". This check makes that
claim verifiable instead of a convention. It fails when

1. a tracked source file has no SPDX header (within its first five
   lines, in a comment) and no REUSE.toml rule covers it, or when it
   declares an expression outside "MIT", "PSF-2.0 AND MIT" and
   "MIT AND PSF-2.0";
2. a file with a PSF-2.0 header has no "// Derived from CPython" (or
   "# Derived from CPython") line and no "Changes:" line in its first
   ten lines, or is not listed in NOTICE; or when NOTICE lists a path
   that does not exist or has no PSF-2.0 header of its own (every line
   of NOTICE names a file). The markers have to be comments: the text
   of a string is not a comment, so a marker inside a multi-line string
   does not count;
3. LICENSES/MIT.txt or LICENSES/PSF-2.0.txt is missing or empty;
4. a Cargo dependency declares a licence the project may not use: the
   expression has to offer a licence of the allowlist (MIT,
   Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0,
   Unicode-DFS-2016, Zlib, 0BSD, CC0-1.0, PSF-2.0 and
   "Apache-2.0 WITH LLVM-exception"; OR, AND and parentheses are
   allowed). The Unlicense is a public-domain dedication and is not a
   licence of its own here: it is accepted as one branch of a choice
   (memchr is published under "Unlicense OR MIT" and the project uses
   it under MIT), never as the licence of a dependency on its own;
5. a MIT-only Rust file shares four or more internal names with one
   CPython source file, which would mean it is derived from CPython
   after all (see provenance_names.py for the method). Every tracked
   Rust file is scanned, not only the ones in the directories that need
   a licence header;
6. a file derived from CPython names a CPython file that does not
   exist in the CPython source tree, or a NOTICE (the one at the root
   of the repository and the copy the npm package carries) names
   another origin for a file than its own header does. The check needs
   the CPython source tree, like the provenance scan needs it.

The check reads the repository, the REUSE.toml at its root, the NOTICE
at its root and the CPython source tree named by MINIPYTHON_CPYTHON_SRC.
Exit status: 0 clean, 1 findings, 2 usage error.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from collections.abc import Iterator
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import provenance_names  # noqa: E402

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 and older
    tomllib = None

# The licence expressions this project uses.
ALLOWED_EXPRESSIONS = ("MIT", "PSF-2.0 AND MIT", "MIT AND PSF-2.0")
# The directories whose tracked files carry a licence.
SOURCE_DIRECTORIES = ("src", "tests", "ci", "npm", "python", "examples", "homebrew-tap-template")
# The file types that carry a licence.
SOURCE_SUFFIXES = (".rs", ".py", ".sh", ".js", ".toml")
# The licences a dependency may use. The Unlicense is a public-domain
# dedication and is not one of them: it is only accepted as one branch
# of a choice, so that memchr, a transitive dependency of serde_json,
# passes (it is dual licensed, "Unlicense OR MIT", and the project
# uses it under MIT) while a dependency published under the Unlicense
# alone does not.
ALLOWED_DEPENDENCY_LICENSES = frozenset({
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "Zlib",
    "0BSD",
    "CC0-1.0",
    "PSF-2.0",
    "Apache-2.0 WITH LLVM-exception",
})
# The header a file derived from CPython carries: the line has to start
# with "// Derived from CPython" or "# Derived from CPython", not with
# any other comment marker.
DERIVED_RE = re.compile(r"^(?://|#)\s*Derived from CPython\b")
CHANGES_PREFIX = "Changes:"
# The comment markers a header line may carry.
COMMENT_RE = re.compile(r"^(?://+|#+|/\*+|\*+)\s*")
# The files of the LICENSES directory.
REQUIRED_LICENCE_FILES = ("MIT.txt", "PSF-2.0.txt")
# The NOTICE files of the repository: the one at the root of the
# repository and the copy the npm package carries.
NOTICE_FILES = ("NOTICE", "npm/NOTICE")
# The languages of the files that carry a licence: the comment syntax
# of a language decides what a header line is.
LANGUAGES = {
    ".rs": "rust",
    ".py": "python",
    ".sh": "shell",
    ".js": "javascript",
    ".toml": "toml",
}

SPDX_RE = re.compile(r"SPDX-License-Identifier:\s*(.+?)\s*$")
LICENSE_TOKEN_RE = re.compile(r"\(|\)|[A-Za-z0-9][A-Za-z0-9.+-]*")
# The line of a derived file that names the CPython files it comes from.
ORIGIN_RE = re.compile(r"Derived from CPython\s+(?P<origins>.*)$")
# The comment of the NOTICE that names the CPython origin of a file.
NOTICE_ORIGIN_RE = re.compile(r"^#\s*CPython origin:\s*(?P<origins>.*)$")


def normalise_expression(expression: str) -> str:
    """The expression with its whitespace normalised."""
    return re.sub(r"\s+", " ", expression.strip())


def comment_text(line: str) -> str:
    """The text of a line without its comment marker."""
    text = line.lstrip()
    while True:
        match = COMMENT_RE.match(text)
        if not match:
            return text
        text = text[match.end():].lstrip()


def derived_line(line: str) -> bool:
    """True for a comment that starts with the "Derived from CPython" marker.

    The marker has to start the comment, behind a "//" or a "#" comment
    marker: a comment without a marker, or one that uses another marker
    ("/* ... */"), does not say that the file comes from CPython.
    """
    return bool(DERIVED_RE.match(line.lstrip()))


def changes_line(line: str) -> bool:
    """True for a comment that starts with the "Changes:" marker."""
    return comment_text(line).startswith(CHANGES_PREFIX)


def language_of(name: str) -> str:
    """The language of a file, by its suffix."""
    return LANGUAGES.get(Path(name).suffix, "text")


def spdx_expression(text: str, limit: int = 5, language: str = "rust") -> str | None:
    """The SPDX expression of a file's header, or None when it has none.

    The tag has to be in a comment: a "SPDX-License-Identifier: MIT"
    inside a string literal (a multi-line string, a doc example) is
    text, not a licence header.
    """
    for line in provenance_names.comment_lines(text, language)[:limit]:
        match = SPDX_RE.search(line)
        if not match:
            continue
        expression = match.group(1).strip()
        for suffix in ("*/", "-->"):
            if expression.endswith(suffix):
                expression = expression[: -len(suffix)].strip()
        return normalise_expression(expression)
    return None


def glob_to_regex(pattern: str) -> re.Pattern:
    """Compile a REUSE path pattern (gitignore style) into a regex."""
    pattern = pattern.lstrip("/")
    out = ["^"]
    index = 0
    length = len(pattern)
    while index < length:
        if pattern.startswith("**/", index):
            out.append("(?:.*/)?")
            index += 3
        elif pattern.startswith("/**", index) and index + 3 == length:
            out.append("/.*")
            index += 3
        elif pattern[index] == "*":
            out.append("[^/]*")
            index += 1
        elif pattern[index] == "?":
            out.append("[^/]")
            index += 1
        elif pattern[index] == "[":
            end = pattern.find("]", index)
            if end == -1:
                out.append(re.escape("["))
                index += 1
            else:
                out.append(pattern[index:end + 1])
                index = end + 1
        else:
            out.append(re.escape(pattern[index]))
            index += 1
    out.append("$")
    return re.compile("".join(out))


def load_toml(text: str) -> dict:
    """Read a TOML file with tomllib."""
    if tomllib is not None:
        return tomllib.loads(text)
    return _fallback_toml(text)


def _fallback_toml(text: str) -> dict:
    """A reader for the small TOML subset REUSE.toml uses.

    Only used on Python 3.10 and older, where tomllib does not exist;
    the release gate runs on CPython 3.14.6, which has tomllib.
    """
    document: dict = {}
    table = document
    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("[["):
            if not stripped.endswith("]]"):
                raise ValueError(f"line {number}: unterminated table header")
            name = stripped[2:-2].strip()
            tables = document.setdefault(name, [])
            if not isinstance(tables, list):
                raise ValueError(f"line {number}: {name} is not a table array")
            table = {}
            tables.append(table)
            continue
        if stripped.startswith("["):
            if not stripped.endswith("]"):
                raise ValueError(f"line {number}: unterminated table header")
            name = stripped[1:-1].strip()
            table = document.setdefault(name, {})
            continue
        if "=" not in stripped:
            raise ValueError(f"line {number}: {stripped!r} is not a key/value pair")
        key, _, value = stripped.partition("=")
        key = key.strip().strip('"')
        value = value.strip()
        if value.startswith('"') and value.endswith('"') and len(value) >= 2:
            table[key] = value[1:-1]
        elif value in ("true", "false"):
            table[key] = value == "true"
        elif value.startswith("[") and value.endswith("]"):
            table[key] = _fallback_toml_array(value, number)
        else:
            try:
                table[key] = int(value)
            except ValueError:
                raise ValueError(f"line {number}: cannot read {value!r}") from None
    return document


def _fallback_toml_array(value: str, number: int) -> list:
    """The items of a TOML array.

    The arrays of REUSE.toml hold the paths of an annotation: a list of
    strings.
    """
    items: list = []
    for item in value[1:-1].split(","):
        item = item.strip()
        if not item:
            continue
        if item.startswith('"') and item.endswith('"') and len(item) >= 2:
            items.append(item[1:-1])
        elif item in ("true", "false"):
            items.append(item == "true")
        else:
            try:
                items.append(int(item))
            except ValueError:
                raise ValueError(f"line {number}: cannot read {item!r}") from None
    return items


def load_reuse_rules(repo: Path) -> tuple[list[dict], list[str]]:
    """The path rules of REUSE.toml and the problems of the file."""
    path = repo / "REUSE.toml"
    if not path.is_file():
        return [], []
    try:
        document = load_toml(path.read_text(encoding="utf-8"))
    except Exception as error:
        return [], [f"REUSE.toml cannot be read: {error}"]
    rules: list[dict] = []
    failures: list[str] = []
    annotations = document.get("annotations", [])
    if not isinstance(annotations, list):
        return [], ["REUSE.toml: 'annotations' is not a list of tables"]
    for annotation in annotations:
        if not isinstance(annotation, dict):
            failures.append(f"REUSE.toml: the annotation {annotation!r} is not a table")
            continue
        pattern = annotation.get("path")
        expression = annotation.get("SPDX-License-Identifier")
        if not pattern or not expression:
            failures.append(f"REUSE.toml: the annotation {annotation!r} lacks 'path' or 'SPDX-License-Identifier'")
            continue
        expression = normalise_expression(str(expression))
        if expression not in ALLOWED_EXPRESSIONS:
            failures.append(
                f"REUSE.toml: {pattern} declares {expression!r}, which is not one of "
                f"{', '.join(ALLOWED_EXPRESSIONS)}"
            )
            continue
        # The path of an annotation is a path or a list of paths: every
        # path of the list is a rule of its own.
        for item in annotation_paths(pattern):
            if not isinstance(item, str) or not item:
                failures.append(
                    f"REUSE.toml: the annotation {annotation!r} has an empty or non-string path"
                )
                continue
            rules.append({"pattern": item, "expression": expression, "regex": glob_to_regex(item)})
    return rules, failures


def annotation_paths(pattern) -> list:
    """The paths of a REUSE annotation.

    The path of an annotation is a path or a list of paths.
    """
    if isinstance(pattern, (list, tuple)):
        return list(pattern)
    return [pattern]


def matching_rule(rules: list[dict], path: str) -> dict | None:
    """The REUSE rule that covers a path, if any."""
    for rule in rules:
        if rule["regex"].match(path):
            return rule
    return None


def tracked_files(repo: Path) -> tuple[list[str], str]:
    """The tracked files of the repository and the error, if any."""
    try:
        completed = subprocess.run(["git", "ls-files", "-z"], cwd=repo, capture_output=True, text=True)
    except OSError as error:
        return [], f"git cannot be run: {error}"
    if completed.returncode != 0:
        message = (completed.stderr or completed.stdout).strip()
        return [], message or f"git ls-files exited {completed.returncode}"
    return [name for name in completed.stdout.split("\0") if name], ""


def is_source_file(path: str) -> bool:
    """True for a tracked file that carries a licence."""
    parts = path.split("/")
    if len(parts) < 2 or parts[0] not in SOURCE_DIRECTORIES:
        return False
    return path.endswith(SOURCE_SUFFIXES)


def source_files(repo: Path) -> list[str]:
    """The tracked source files that carry a licence."""
    files, _ = tracked_files(repo)
    return sorted(name for name in files if is_source_file(name))


def is_repository_path(entry: str) -> bool:
    """True for a path of the repository.

    A NOTICE line names a file of this repository: an absolute path and
    a path that leaves the repository are not. A path with a space in it
    is a path as well ("src/ported file.rs" is a file of the
    repository); a line of prose names no file, so the check that the
    file exists rejects it.
    """
    if not entry or "\0" in entry:
        return False
    if entry.startswith("/") or entry.startswith("~"):
        return False
    return ".." not in Path(entry).parts


def notice_entries(repo: Path, name: str = "NOTICE") -> tuple[list[str], bool]:
    """The paths NOTICE lists and whether the file exists.

    Every line that is not empty and not a comment names a file: a
    NOTICE holds nothing but the paths of the files derived from
    CPython, one per line. The NOTICE of the npm package is a copy of
    the one at the root of the repository.
    """
    path = repo / name
    if not path.is_file():
        return [], False
    entries = []
    for line in path.read_text(encoding="utf-8").splitlines():
        text = line.strip()
        if not text or text.startswith("#"):
            continue
        entries.append(text)
    return entries, True


def parse_license_expression(expression: str):
    """The tree of an SPDX expression.

    The nodes are ("or", branches), ("and", factors) and
    ("license", name): OR, AND, parentheses and "WITH" exceptions are
    understood; the old "MIT/Apache-2.0" spelling of OR is read as OR.
    A malformed expression raises ValueError.
    """
    tokens = LICENSE_TOKEN_RE.findall(expression.replace("/", " OR "))
    position = 0

    def parse_factor():
        nonlocal position
        if position >= len(tokens):
            raise ValueError("the expression ends too early")
        token = tokens[position]
        if token == ")":
            raise ValueError("a parenthesis is closed but never opened")
        if token == "(":
            position += 1
            node = parse_expression()
            if position >= len(tokens) or tokens[position] != ")":
                raise ValueError("a parenthesis is never closed")
            position += 1
            return node
        position += 1
        name = token
        if position < len(tokens) and tokens[position].upper() == "WITH":
            position += 1
            if position >= len(tokens):
                raise ValueError("WITH has no exception")
            name = f"{name} WITH {tokens[position]}"
            position += 1
        return ("license", name)

    def parse_term():
        nonlocal position
        factors = [parse_factor()]
        while position < len(tokens) and tokens[position].upper() == "AND":
            position += 1
            factors.append(parse_factor())
        if len(factors) == 1:
            return factors[0]
        return ("and", factors)

    def parse_expression():
        nonlocal position
        branches = [parse_term()]
        while position < len(tokens) and tokens[position].upper() == "OR":
            position += 1
            branches.append(parse_term())
        if len(branches) == 1:
            return branches[0]
        return ("or", branches)

    node = parse_expression()
    if position != len(tokens):
        raise ValueError(f"unexpected {tokens[position]!r}")
    return node


def license_identifiers(expression: str) -> list[str]:
    """The licence identifiers of an SPDX expression.

    OR, AND, parentheses and "WITH" exceptions are understood; the old
    "MIT/Apache-2.0" spelling of OR is read as OR. A malformed
    expression raises ValueError.
    """
    return list(walk_licenses(parse_license_expression(expression)))


def walk_licenses(node) -> Iterator[str]:
    """The licence identifiers of a parsed expression, in order."""
    if node[0] == "license":
        yield node[1]
        return
    for child in node[1]:
        yield from walk_licenses(child)


def expression_is_allowed(node) -> bool:
    """True when the expression offers a licence the project accepts.

    A choice ("OR") is allowed when one of its branches is; a
    conjunction ("AND") only when every one of its factors is. The
    Unlicense is a public-domain dedication and is not a licence of
    its own here: memchr is published under "Unlicense OR MIT" and the
    project uses it under MIT, but a dependency published under the
    Unlicense alone is not accepted.
    """
    kind = node[0]
    if kind == "or":
        return any(expression_is_allowed(branch) for branch in node[1])
    if kind == "and":
        return all(expression_is_allowed(factor) for factor in node[1])
    return node[1] in ALLOWED_DEPENDENCY_LICENSES


def dependency_offenders(payload: dict) -> list[dict]:
    """The dependencies of a cargo metadata document outside the allowlist."""
    workspace = set(payload.get("workspace_members", []))
    offenders = []
    for package in payload.get("packages", []):
        if package.get("id") in workspace:
            continue
        name = f"{package.get('name')} {package.get('version')}"
        expression = package.get("license")
        if not expression:
            offenders.append({"package": name, "license": None, "reason": "no SPDX licence expression"})
            continue
        try:
            node = parse_license_expression(expression)
        except ValueError as error:
            offenders.append({"package": name, "license": expression, "reason": f"malformed expression: {error}"})
            continue
        if expression_is_allowed(node):
            # The expression offers a licence of the allowlist: the
            # dependency may be used under it.
            continue
        outside = list(dict.fromkeys(
            item for item in walk_licenses(node) if item not in ALLOWED_DEPENDENCY_LICENSES
        ))
        if outside:
            reason = f"outside the allowlist: {', '.join(outside)}"
        else:
            reason = "the expression offers no licence of the allowlist"
        offenders.append({
            "package": name,
            "license": expression,
            "reason": reason,
        })
    return offenders


def dependency_licenses(repo: Path) -> tuple[list[dict], list[str]]:
    """The dependencies whose licence is outside the allowlist."""
    try:
        completed = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--locked"],
            cwd=repo, capture_output=True, text=True,
        )
    except OSError as error:
        return [], [f"cargo cannot be run: {error}"]
    if completed.returncode != 0:
        tail = "\n".join((completed.stderr or completed.stdout).splitlines()[-10:])
        return [], [f"cargo metadata exited {completed.returncode}:\n{tail}"]
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        return [], [f"cargo metadata printed no usable JSON: {error}"]
    return dependency_offenders(payload), []


def licence_files(repo: Path) -> list[str]:
    """The problems of the LICENSES directory."""
    problems = []
    for name in REQUIRED_LICENCE_FILES:
        path = repo / "LICENSES" / name
        if not path.is_file():
            problems.append(f"LICENSES/{name} is missing")
        elif not path.read_text(encoding="utf-8").strip():
            problems.append(f"LICENSES/{name} is empty")
    return problems


def check_files(repo: Path, files: list[str], rules: list[dict]) -> tuple[list[str], dict]:
    """The licence problems of the tracked source files.

    Returns the problems and the files that carry a PSF-2.0 header.
    """
    failures: list[str] = []
    psf_files: list[str] = []
    uncovered: list[str] = []
    for name in files:
        text = (repo / name).read_text(encoding="utf-8", errors="replace")
        expression = spdx_expression(text, language=language_of(name))
        origin = "header"
        from_header = expression is not None
        if expression is None:
            rule = matching_rule(rules, name)
            if rule is None:
                uncovered.append(name)
                failures.append(
                    f"{name}: no SPDX header in the first 5 lines and no REUSE.toml rule covers it"
                )
                continue
            expression = rule["expression"]
            origin = f"REUSE.toml ({rule['pattern']})"
        if expression not in ALLOWED_EXPRESSIONS:
            failures.append(
                f"{name}: the {origin} declares {expression!r}, which is not one of "
                f"{', '.join(ALLOWED_EXPRESSIONS)}"
            )
            continue
        if "PSF-2.0" in expression:
            if not from_header:
                # A REUSE.toml rule covers the file; only a header names
                # PSF-2.0 in the file itself.
                continue
            psf_files.append(name)
            # The markers have to be comments: the text of a string is
            # not a comment, so a marker inside a multi-line string does
            # not count.
            comments = provenance_names.leading_comment_lines(
                text, language_of(name))[:10]
            if not any(derived_line(line) for line in comments):
                failures.append(
                    f"{name}: a PSF-2.0 file needs a '// Derived from CPython' line in its first 10 lines"
                )
            if not any(changes_line(line) for line in comments):
                failures.append(f"{name}: a PSF-2.0 file needs a 'Changes:' line in its first 10 lines")
    return failures, {"psf_files": psf_files, "files_without_licence": uncovered}


def mit_only_rust_files(repo: Path, rules: list[dict]) -> list[str]:
    """The tracked Rust files that carry the MIT licence only.

    Every tracked Rust file counts, not only the ones in the directories
    that need a licence header: a file that declares no PSF-2.0 licence
    is MiniPython's own, and it must not carry CPython's internal names.
    """
    files, _ = tracked_files(repo)
    names: list[str] = []
    for name in sorted(files):
        if not name.endswith(".rs"):
            continue
        path = repo / name
        expression = None
        if path.is_file():
            expression = spdx_expression(path.read_text(encoding="utf-8", errors="replace"),
                                         language=language_of(name))
        if expression is None:
            rule = matching_rule(rules, name)
            expression = rule["expression"] if rule else None
        if expression is not None and "PSF-2.0" in expression:
            continue
        names.append(name)
    return names


def check_notice(repo: Path, psf_files: list[str]) -> tuple[list[str], list[str]]:
    """The problems of NOTICE and the paths it lists."""
    failures: list[str] = []
    entries, notice_exists = notice_entries(repo)
    if not notice_exists and psf_files:
        failures.append("NOTICE is missing, but PSF-2.0 files must be listed in it")
    for entry in entries:
        if not is_repository_path(entry):
            failures.append(f"NOTICE lists {entry!r}, which is not a path of the repository")
            continue
        path = repo / entry
        if not path.is_file():
            if any(char.isspace() for char in entry):
                # A path with a space in it that names no file is prose,
                # not a path of the repository.
                failures.append(
                    f"NOTICE lists {entry!r}, which is not a path of the repository: it does not exist"
                )
            else:
                failures.append(f"NOTICE lists {entry}, which does not exist")
            continue
        expression = spdx_expression(path.read_text(encoding="utf-8", errors="replace"),
                                     language=language_of(entry))
        if expression is None or "PSF-2.0" not in expression:
            failures.append(f"NOTICE lists {entry}, which has no PSF-2.0 header")
    for name in psf_files:
        if name not in entries:
            failures.append(f"{name} has a PSF-2.0 header but is not listed in NOTICE")
    return failures, entries


def origin_paths(line: str) -> list[str]:
    """The CPython files a "Derived from CPython" line names.

    The line reads "Derived from CPython 3.14.6 Lib/re/__init__.py
    (Copyright (c) 2001 Python Software Foundation; All Rights
    Reserved).": the version and the copyright notice are dropped, and
    the paths that are left are returned.
    """
    match = ORIGIN_RE.search(line)
    if not match:
        return []
    rest = re.sub(r"\(Copyright.*$", "", match.group("origins"))
    rest = re.sub(r"^[0-9][0-9.]*\s+", "", rest)
    return [item.strip() for item in rest.split(",") if item.strip()]


def notice_origin_paths(line: str) -> list[str]:
    """The CPython files the origin comment of the NOTICE names."""
    match = NOTICE_ORIGIN_RE.match(line.strip())
    if not match:
        return []
    return [item.strip() for item in match.group("origins").split(",") if item.strip()]


def notice_origins(repo: Path, name: str = "NOTICE") -> dict[str, list[str]]:
    """The CPython origin the NOTICE names for every file it lists.

    The NOTICE names the origin of a file in a comment below the path
    of the file: "# CPython origin: Lib/re/__init__.py".
    """
    origins: dict[str, list[str]] = {}
    path = repo / name
    if not path.is_file():
        return origins
    current = ""
    for line in path.read_text(encoding="utf-8").splitlines():
        text = line.strip()
        if not text:
            continue
        if text.startswith("#"):
            found = notice_origin_paths(text)
            if found and current:
                origins[current] = found
            continue
        current = text
    return origins


def check_origins(repo: Path, psf_files: list[str], cpython_src: Path | None) -> tuple[list[str], dict]:
    """The problems of the CPython origins the derived files name.

    Every CPython file a derived file names has to exist in the
    CPython source tree: a path that does not exist there names
    nothing, so the provenance of the file cannot be checked. The
    NOTICE has to name the same origin as the header of the file does.
    """
    if cpython_src is None or not cpython_src.is_dir():
        # Without the CPython source tree there is nothing to check the
        # origins against; the provenance scan reports the missing tree.
        return [], {}
    failures: list[str] = []
    found: dict[str, list[str]] = {}
    headers: dict[str, list[str]] = {}
    for name in psf_files:
        text = (repo / name).read_text(encoding="utf-8", errors="replace")
        header: list[str] = []
        for line in provenance_names.leading_comment_lines(text, language_of(name))[:10]:
            header += origin_paths(line)
        headers[name] = header
        found[name] = header
        if not header:
            failures.append(f"{name}: the header names no CPython file")
            continue
        for origin in header:
            if not (cpython_src / origin).is_file():
                failures.append(
                    f"{name}: the CPython file {origin} does not exist in the CPython source tree"
                )
    for notice in NOTICE_FILES:
        entries, exists = notice_entries(repo, notice)
        if not exists:
            continue
        origins = notice_origins(repo, notice)
        for entry in entries:
            listed = origins.get(entry)
            header = headers.get(entry, [])
            if listed is None:
                failures.append(f"{notice} names no CPython origin for {entry}")
            elif sorted(listed) != sorted(header):
                failures.append(
                    f"{notice} names the CPython origin "
                    f"{', '.join(listed) or 'none'} for {entry}, the header names "
                    f"{', '.join(header) or 'none'}"
                )
    return failures, found


def check_provenance(repo: Path, mit_only_rust: list[str], cpython_src: Path | None) -> tuple[list[str], dict]:
    """The provenance findings of the MIT-only Rust files."""
    if cpython_src is None:
        return (["MINIPYTHON_CPYTHON_SRC is not set: the provenance scan needs the CPython source tree"],
                {"scanned": 0, "cpython_files": 0, "findings": []})
    if not cpython_src.is_dir():
        return ([f"MINIPYTHON_CPYTHON_SRC={cpython_src} is not a directory"],
                {"scanned": 0, "cpython_files": 0, "findings": []})
    if not mit_only_rust:
        # Nothing to compare: the index of the CPython tree is not built.
        return ([], {"scanned": 0, "cpython_files": len(provenance_names.cpython_source_files(cpython_src)),
                     "findings": []})
    index, scanned = provenance_names.build_index(cpython_src)
    public, modules = provenance_names.public_vocabulary(cpython_src)
    findings = provenance_names.scan_files(
        [repo / name for name in mit_only_rust], index, public, modules, root=repo
    )
    return ([provenance_names.format_finding(finding) for finding in findings],
            {"scanned": len(mit_only_rust), "cpython_files": len(scanned), "findings": findings})


def check(repo: Path, cpython_src: Path | None) -> dict:
    """Run every licensing check and return the report."""
    failures: list[str] = []
    details: dict = {}

    files, error = tracked_files(repo)
    if error:
        failures.append(f"the tracked files cannot be listed: {error}")
    files = sorted(name for name in files if is_source_file(name))
    details["tracked_source_files"] = len(files)
    rules, reuse_failures = load_reuse_rules(repo)
    failures += reuse_failures
    details["reuse_rules"] = [rule["pattern"] for rule in rules]

    file_failures, file_details = check_files(repo, files, rules)
    failures += file_failures
    details.update(file_details)

    notice_failures, entries = check_notice(repo, file_details["psf_files"])
    failures += notice_failures
    details["notice_entries"] = entries

    origin_failures, origins = check_origins(repo, file_details["psf_files"], cpython_src)
    failures += origin_failures
    details["cpython_origins"] = origins

    failures += licence_files(repo)

    offenders, dependency_failures = dependency_licenses(repo)
    failures += dependency_failures
    details["dependency_offenders"] = offenders
    for offender in offenders:
        failures.append(
            f"the dependency {offender['package']} declares the licence "
            f"{offender['license']!r}: {offender['reason']}"
        )

    mit_only_rust = mit_only_rust_files(repo, rules)
    details["mit_only_rust_files"] = mit_only_rust

    provenance_failures, provenance = check_provenance(repo, mit_only_rust, cpython_src)
    failures += provenance_failures
    details["provenance"] = provenance

    return {
        "repo": str(repo),
        "details": details,
        "failures": failures,
        "passed": not failures,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", default=".", help="The MiniPython checkout to check.")
    parser.add_argument("--cpython-src", default=os.environ.get("MINIPYTHON_CPYTHON_SRC"),
                        help="The CPython source tree (default: $MINIPYTHON_CPYTHON_SRC).")
    parser.add_argument("--json", default=None, help="Write the report as JSON to this file.")
    args = parser.parse_args(argv)

    repo = Path(args.repo).resolve()
    if not repo.is_dir():
        print(f"licensing: {repo} is not a directory", file=sys.stderr)
        return 2
    cpython_src = Path(args.cpython_src).resolve() if args.cpython_src else None
    report = check(repo, cpython_src)
    if args.json:
        Path(args.json).write_text(json.dumps(report, indent=2, default=str) + "\n")
    details = report["details"]
    print(
        f"licensing: {details['tracked_source_files']} tracked source files, "
        f"{len(details['psf_files'])} PSF-2.0, {len(details['mit_only_rust_files'])} MIT-only Rust files, "
        f"{len(details['dependency_offenders'])} dependency licence offender(s), "
        f"{len(details['provenance'].get('findings', []))} provenance finding(s): "
        f"{len(report['failures'])} problem(s)"
    )
    for failure in report["failures"]:
        print(f"  {failure}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
