"""Internal-name overlap scan between MiniPython's MIT files and CPython.

usage: provenance_names.py --cpython-src DIR [--repo DIR] [--files FILE ...]
                           [--json OUT]

MiniPython's own files are MIT; files derived from CPython are
"PSF-2.0 AND MIT" and are expected to say so. This module is the other
half of that promise: a file that claims to be MIT-only must not carry
CPython's internal vocabulary. It compares the internal identifiers of
every MIT-only Rust file with the identifiers of the CPython source tree
and reports a file that shares four or more private names with a single
CPython source file.

The method (fixed, so that the result is reproducible):

- The identifiers of MiniPython's own files are the ones written in code
  style: they contain an underscore or mix upper and lower case. The
  comments and the string literals are removed first, so a name that only
  appears as text (in a comment or in a string) is not a name. The
  comments are read before the strings: the quote of a comment does not
  open a string, and the code behind the comment stays code. The CPython
  side keeps every identifier it has: the code-style filter is our rule
  for our own files, not CPython's, and the name CPython writes
  "privatealpha" is the name we write "private_alpha".
- A name is normalised by dropping the underscores and lowercasing it
  ("PyUnicode_Type" and "py_unicode_type" are the same name).
- Hex-like tokens are ignored: they are data, not names.
- Only names that occur in at most three CPython source files count; a
  name many files use is vocabulary, not a fingerprint.
- Public vocabulary is excluded: the names in ``code`` spans, `role`
  spans and directive signatures of Doc/**/*.rst (a signature counts
  with its parameters, not with the prose behind it), the names in
  Lib/token.py, Lib/keyword.py, Lib/encodings/aliases.py,
  Parser/Python.asdl, Grammar/python.gram and
  Include/internal/pycore_token.h (the strings of those files count as
  well: the keywords of Lib/keyword.py are string literals), and every
  name formed by concatenating a module name or a public name with a
  public name ("os" + "path" is "os.path", which is public). A module
  name is the name of a module or of a package of the Lib directory (the
  modules of a package count as well: "email.mime" is a module as much
  as "os") or of a native module (the C modules of the Modules
  directory: "gc" is a module name as much as "os"). The name of a
  native module is the name of its initialisation function
  ("PyInit_gc"), of its definition ("struct PyModuleDef gcmodule") or
  the name it gives itself (".m_name = \"gc\""), not the name of its
  file: "Modules/pythonrun.c" is a helper, not a module.
- The CPython source files are the ones under Lib, Modules, Objects,
  Python, Parser and Include (every source file type of those trees, not
  only the Python, C and header files), skipping Lib/test, idlelib,
  clinic, _testcapi*, tkinter and turtledemo.

A finding is printed as

    file:1: provenance: N internal names shared with REF: a, b, c

where REF is the CPython file (relative to the source tree) and a, b, c
are the shared names.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

# The CPython trees the index is built from.
CPYTHON_SOURCE_DIRS = ("Lib", "Modules", "Objects", "Python", "Parser", "Include")
# Directories that hold tests, generated code or demos rather than the
# interpreter's own vocabulary.
SKIPPED_DIRECTORIES = frozenset({"test", "idlelib", "clinic", "tkinter", "turtledemo"})
# Generated test modules (files and directories).
SKIPPED_FILE_PREFIXES = ("_testcapi",)
# The file types the index is built from: every source file type of the
# CPython tree, not only its Python, C and header files. A file of
# another type (data, images, archives) is not source and is skipped.
SOURCE_SUFFIXES = (
    ".py", ".pyi", ".pyx", ".pxd",
    ".c", ".h", ".hh", ".hpp", ".hxx", ".inc",
    ".cc", ".cpp", ".cxx", ".m", ".mm",
    ".rs", ".def", ".S", ".s", ".asm", ".in",
)

# The comment and string syntax of the languages the scanner knows. The
# comments are read before the strings: the quote of a comment ("// don't
# panic") does not open a string, and the code behind the comment stays
# code.
LANGUAGE_SYNTAX: dict[str, dict] = {
    "rust": {
        "line_comments": ("//",),
        "block_comments": (("/*", "*/"),),
        "nested_block_comments": True,
        "apostrophe_is_string": False,
        "backtick_is_string": False,
        "multiline_strings": True,
        "triple_quotes": False,
    },
    "c": {
        "line_comments": ("//",),
        "block_comments": (("/*", "*/"),),
        "nested_block_comments": False,
        "apostrophe_is_string": False,
        "backtick_is_string": False,
        "multiline_strings": False,
        "triple_quotes": False,
    },
    "javascript": {
        "line_comments": ("//",),
        "block_comments": (("/*", "*/"),),
        "nested_block_comments": False,
        "apostrophe_is_string": True,
        "backtick_is_string": True,
        "multiline_strings": True,
        "triple_quotes": False,
    },
    "python": {
        "line_comments": ("#",),
        "block_comments": (),
        "nested_block_comments": False,
        "apostrophe_is_string": True,
        "backtick_is_string": False,
        "multiline_strings": False,
        "triple_quotes": True,
    },
    "shell": {
        "line_comments": ("#",),
        "block_comments": (),
        "nested_block_comments": False,
        "apostrophe_is_string": True,
        "backtick_is_string": False,
        "multiline_strings": True,
        "triple_quotes": False,
    },
    "toml": {
        "line_comments": ("#",),
        "block_comments": (),
        "nested_block_comments": False,
        "apostrophe_is_string": True,
        "backtick_is_string": False,
        "multiline_strings": False,
        # A multi-line string ("""...""" or '''...''') is a string:
        # the "#" of its text is not a comment.
        "triple_quotes": True,
    },
    "text": {
        "line_comments": (),
        "block_comments": (),
        "nested_block_comments": False,
        "apostrophe_is_string": True,
        "backtick_is_string": False,
        "multiline_strings": False,
        "triple_quotes": False,
    },
}

IDENTIFIER_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
# Hexadecimal data: six or more hexadecimal digits, or CPython's "xNNNN"
# names of the Unicode database.
HEX_LIKE_RES = (
    re.compile(r"[0-9a-f]{6,}"),
    re.compile(r"x[0-9a-f]{4,}"),
    re.compile(r"[0-9a-f]{4,}x[0-9a-f]+"),
)

# Doc sources of public vocabulary.
DOC_CODE_SPAN_RE = re.compile(r"``(.+?)``", re.S)
DOC_ROLE_RE = re.compile(r":[A-Za-z][A-Za-z0-9:+-]*:`([^`]+)`")
DIRECTIVE_RE = re.compile(r"^\s*\.\.\s+[A-Za-z][A-Za-z0-9:+-]*::\s*(.+)$", re.M)
# The signature of a directive: the name of the thing it documents and
# its parameters ("parse(source, encoding=None)").
DIRECTIVE_SIGNATURE_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_.]*)\s*(\([^)]*\))?")
# The initialisation function of a native module: "PyInit_gc" is the
# module "gc".
PY_INIT_RE = re.compile(r"\bPyInit_([A-Za-z_][A-Za-z0-9_]*)\b")
# The definition of a native module: "static struct PyModuleDef gcmodule = {".
PY_MODULE_DEF_RE = re.compile(r"\bstruct\s+PyModuleDef\s+([A-Za-z_][A-Za-z0-9_]*)")
# The name a module gives itself: ".m_name = \"gc\"".
MODULE_NAME_FIELD_RE = re.compile(r"\.m_name\s*=\s*\"([A-Za-z_][A-Za-z0-9_.]*)\"")
# The name of a module that is written as a macro: "#define MODULE_NAME gc".
MODULE_NAME_MACRO_RE = re.compile(
    r"^[ \t]*#[ \t]*define[ \t]+MODULE_NAME[ \t]+([A-Za-z_][A-Za-z0-9_]*)", re.M)
# The suffixes the name of a definition carries: "gcmodule" and
# "gc_module" are the module "gc", "sslmodule_def" is the module "ssl".
MODULE_DEFINITION_SUFFIXES = ("_module", "module", "_def", "def")
# The names of a definition that name no module at all: CPython writes
# the definition of a module without a name of its own as "moduledef".
GENERIC_MODULE_NAMES = frozenset({"module", "moduledef", "def", "module_def"})
# The files that name the public vocabulary of the language itself.
PUBLIC_VOCABULARY_FILES = (
    "Lib/token.py",
    "Lib/keyword.py",
    "Lib/encodings/aliases.py",
    "Parser/Python.asdl",
    "Grammar/python.gram",
    "Include/internal/pycore_token.h",
)
# A name that occurs in more files than this is vocabulary, not a
# fingerprint of one file.
MAX_CPYTHON_FILES = 3
# A file that shares this many private names with one CPython file is
# derived from it.
MINIMUM_SHARED_NAMES = 4


def _syntax(language: str) -> dict:
    """The comment and string syntax of a language."""
    return LANGUAGE_SYNTAX.get(language, LANGUAGE_SYNTAX["text"])


_INTERESTING: dict[str, re.Pattern] = {}


def _interesting_re(language: str) -> re.Pattern:
    """The pattern of the places where a piece starts.

    A piece starts at a comment marker or at a quote; the code between
    two of them is one piece.
    """
    pattern = _INTERESTING.get(language)
    if pattern is None:
        syntax = _syntax(language)
        markers = set(syntax["line_comments"])
        markers.update(start for start, _ in syntax["block_comments"])
        markers.update(['"', "'"])
        if syntax["backtick_is_string"]:
            markers.add("`")
        pattern = re.compile("|".join(
            re.escape(marker) for marker in sorted(markers, key=len, reverse=True)))
        _INTERESTING[language] = pattern
    return pattern


def _string_end(text: str, start: int, quote: str, allow_newline: bool = False) -> tuple[int, int]:
    """The end of a string literal's content and the end of the literal.

    ``start`` is the index of the opening quote. The content ends before
    the closing quote; a language whose literals may not span lines ends
    the literal at the newline instead (Rust's literals may span lines).
    """
    index = start + 1
    length = len(text)
    while index < length:
        char = text[index]
        if char == "\\":
            index += 2
            continue
        if char == quote:
            return index, index + 1
        if char == "\n" and not allow_newline:
            return index, index
        index += 1
    return length, length


def _comment_end(text: str, index: int, syntax: dict) -> int | None:
    """The end of the comment that starts at index.

    ``None`` means that no comment starts at index. A line comment ends
    at the newline, a block comment at its end marker (Rust's block
    comments nest).
    """
    for marker in syntax["line_comments"]:
        if text.startswith(marker, index):
            end = text.find("\n", index)
            return len(text) if end == -1 else end
    for start, stop in syntax["block_comments"]:
        if not text.startswith(start, index):
            continue
        depth = 1
        cursor = index + len(start)
        while cursor < len(text):
            if syntax["nested_block_comments"] and text.startswith(start, cursor):
                depth += 1
                cursor += len(start)
                continue
            if text.startswith(stop, cursor):
                depth -= 1
                cursor += len(stop)
                if depth == 0:
                    return cursor
                continue
            cursor += 1
        return len(text)
    return None


def _raw_string(text: str, index: int) -> tuple[int, int] | None:
    """The end of the content and of the literal of a Rust raw string.

    ``index`` is the index of the opening quote. The raw string of
    ``r"..."`` ends at the next quote, the one of ``r#"..."#`` at the
    next quote followed by the hashes. ``None`` means that the quote does
    not open a raw string.

    Only the prefixes that make a raw string count: "r", "br" and "rb".
    A plain "b" is the prefix of a byte string, which is an ordinary
    string: the escape of a quote ends it, and the code behind it stays
    code.
    """
    cursor = index - 1
    hashes = 0
    while cursor >= 0 and text[cursor] == "#":
        hashes += 1
        cursor -= 1
    # The prefix of a raw string is "r", "br" or "rb".
    if cursor >= 0 and text[cursor] in ("r", "b"):
        prefix = text[cursor]
        cursor -= 1
    else:
        return None
    if cursor >= 0 and text[cursor] in ("r", "b") and text[cursor] != prefix:
        # "br" and "rb" are the raw byte strings.
        prefix = text[cursor] + prefix
        cursor -= 1
    if "r" not in prefix:
        # A byte string (b"...") is not a raw string.
        return None
    before = text[cursor] if cursor >= 0 else ""
    if before.isalnum() or before == "_":
        return None
    closing = '"' + "#" * hashes
    end = text.find(closing, index + 1)
    if end == -1:
        return len(text), len(text)
    return end, end + len(closing)


def _character_literal(text: str, index: int) -> tuple[int, int] | None:
    """The end of the content and of the literal of a character literal.

    ``'a'`` and ``'\\n'`` are character literals; a Rust lifetime
    (``'a``) is not. ``None`` means that the quote does not open a
    character literal.
    """
    cursor = index + 1
    if cursor < len(text) and text[cursor] == "\\":
        end = text.find("'", cursor + 1)
        if end != -1 and "\n" not in text[cursor:end]:
            return end, end + 1
        return None
    if cursor + 1 < len(text) and text[cursor + 1] == "'":
        return cursor + 1, cursor + 2
    if cursor + 2 < len(text) and text[cursor + 2] == "'":
        return cursor + 1, cursor + 3
    return None


def scan_source(text: str, language: str = "rust") -> list[tuple[str, str]]:
    """The pieces of a source text.

    Every piece is a pair: its kind ("code", "string" or "comment") and
    its text. Rust's raw strings (``r"..."``, ``r#"..."#``) and
    character literals, the character literals of C and the strings of
    Python are strings; the comments of the language are comments;
    everything else is code, a Rust lifetime ('a) included.

    The comments are read before the strings: the quote of a comment
    ("// don't panic") does not open a string, and the code behind the
    comment stays code.
    """
    syntax = _syntax(language)
    pattern = _interesting_re(language)
    pieces: list[tuple[str, str]] = []
    index = 0
    length = len(text)
    while index < length:
        match = pattern.search(text, index)
        if match is None:
            pieces.append(("code", text[index:]))
            break
        start = match.start()
        if start > index:
            pieces.append(("code", text[index:start]))
        comment_end = _comment_end(text, start, syntax)
        if comment_end is not None:
            pieces.append(("comment", text[start:comment_end]))
            index = comment_end
            continue
        quote = match.group(0)
        if quote == '"' and language == "rust":
            raw = _raw_string(text, start)
            if raw is not None:
                content_end, literal_end = raw
                pieces.append(("string", text[start + 1:content_end]))
                index = literal_end
                continue
        if quote == "'" and not syntax["apostrophe_is_string"]:
            literal = _character_literal(text, start)
            if literal is None:
                # A Rust lifetime ('a): the name behind it is code.
                pieces.append(("code", "'"))
                index = start + 1
                continue
            content_end, literal_end = literal
            pieces.append(("string", text[start + 1:content_end]))
            index = literal_end
            continue
        if syntax["triple_quotes"] and text[start:start + 3] in ('"""', "'''"):
            closing = text[start:start + 3]
            end = text.find(closing, start + 3)
            if end == -1:
                pieces.append(("string", text[start + 3:]))
                index = length
            else:
                pieces.append(("string", text[start + 3:end]))
                index = end + 3
            continue
        content_end, literal_end = _string_end(text, start, quote,
                                               allow_newline=syntax["multiline_strings"])
        pieces.append(("string", text[start + 1:content_end]))
        index = literal_end
    return pieces


def strip_string_literals(text: str, language: str = "rust") -> str:
    """Remove the string literals of Rust, C and Python source text.

    The text of a literal is data, not code: a name that only occurs
    inside a string is not an identifier of the file. Rust's raw strings
    (``r"..."``, ``r#"..."#``) and character literals, the character
    literals of C and the strings of Python are removed; everything else
    is kept as it is, a Rust lifetime ('a) included.
    """
    return "".join(" " if kind == "string" else chunk for kind, chunk in scan_source(text, language))


def _blank(chunk: str) -> str:
    """The chunk with everything but the newlines replaced by a space."""
    return "".join("\n" if char == "\n" else " " for char in chunk)


def strip_comments(text: str, language: str = "rust") -> str:
    """Remove the comments of Rust, C and Python source text.

    The text of a comment is a comment, not code: a name that only
    occurs in a comment is not an identifier of the file. The line
    structure of the text is kept.
    """
    return "".join(_blank(chunk) if kind == "comment" else chunk
                   for kind, chunk in scan_source(text, language))


def strip_code(text: str, language: str = "rust") -> str:
    """Remove the strings and the comments of a source text.

    What is left is the code: the identifiers of a file are the
    identifiers of its code, not of its text.
    """
    return "".join(" " if kind in ("string", "comment") else chunk
                   for kind, chunk in scan_source(text, language))


def comment_lines(text: str, language: str = "rust") -> list[str]:
    """The comment text of every line of a source text.

    A line without a comment has no comment text; a line in the middle
    of a block comment has the text of that comment. The list has one
    entry per line of the text.
    """
    lines = [""]
    for kind, chunk in scan_source(text, language):
        if kind != "comment":
            lines.extend([""] * chunk.count("\n"))
            continue
        parts = chunk.split("\n")
        lines[-1] += parts[0]
        lines.extend(parts[1:])
    if text.endswith("\n") and len(lines) > 1:
        # The newline at the end of the text does not start a line.
        lines.pop()
    return lines


def leading_comment_lines(text: str, language: str = "rust") -> list[str]:
    """The comment that starts every line of a source text.

    A line whose comment follows code has no leading comment: the
    comment of such a line does not start it. The delimiters of a
    string are content as well: a comment behind an empty string does
    not start its line. The text of a string is not a comment at all,
    so the marker of a multi-line string is the text of no line. The
    list has one entry per line of the text.
    """
    lines: list[str] = [""]
    # Whether the line has code (or a string) before its comment.
    code_before: list[bool] = [False]
    for kind, chunk in scan_source(text, language):
        if kind == "comment":
            parts = chunk.split("\n")
            if not code_before[-1]:
                lines[-1] += parts[0]
            lines.extend(parts[1:])
            code_before.extend([False] * (len(parts) - 1))
            continue
        parts = chunk.split("\n")
        if kind == "string":
            # The delimiters of a string are content: the comment of a
            # line that holds a string does not start the line, not
            # even when the string is empty.
            code_before[-1] = True
            lines.extend([""] * (len(parts) - 1))
            code_before.extend([True] * (len(parts) - 1))
            continue
        if parts[0].strip():
            code_before[-1] = True
        for part in parts[1:]:
            lines.append("")
            code_before.append(bool(part.strip()))
    if text.endswith("\n") and len(lines) > 1:
        # The newline at the end of the text does not start a line.
        lines.pop()
    return lines


def string_literal_identifiers(text: str, language: str = "rust") -> set[str]:
    """The normalised identifiers that occur in the string literals.

    Some of the public vocabulary of CPython is written as text: the
    keywords of Lib/keyword.py and the aliases of
    Lib/encodings/aliases.py are string literals.
    """
    names: set[str] = set()
    for kind, chunk in scan_source(text, language):
        if kind == "string":
            names |= {normalise(match.group(0)) for match in IDENTIFIER_RE.finditer(chunk)}
    return names


def is_code_style(name: str) -> bool:
    """True for a name written in code style: an underscore or mixed case."""
    if "_" in name:
        return True
    return any(char.isupper() for char in name) and any(char.islower() for char in name)


def normalise(name: str) -> str:
    """Drop the underscores and lowercase the name."""
    return name.replace("_", "").lower()


def is_hex_like(token: str) -> bool:
    """True for a token that is data (a hexadecimal number), not a name."""
    return any(pattern.fullmatch(token) for pattern in HEX_LIKE_RES)


def code_identifiers(text: str, language: str = "rust") -> set[str]:
    """The normalised internal names of a source text.

    Only the names written in code style count: they are the names of
    MiniPython's own files.
    """
    names: set[str] = set()
    for match in IDENTIFIER_RE.finditer(strip_code(text, language)):
        name = match.group(0)
        if not is_code_style(name):
            continue
        token = normalise(name)
        if is_hex_like(token):
            continue
        names.add(token)
    return names


def all_identifiers(text: str, language: str = "rust") -> set[str]:
    """Every identifier of a source text, normalised (no code-style filter)."""
    stripped = strip_code(text, language)
    return {normalise(match.group(0)) for match in IDENTIFIER_RE.finditer(stripped)}


def index_identifiers(text: str, language: str = "rust") -> set[str]:
    """Every identifier of a source text, normalised (hex-like data ignored).

    The index of the CPython tree holds every identifier CPython has:
    the code-style filter is our rule for our own files, and CPython
    writes "privatealpha" where we write "private_alpha".
    """
    names: set[str] = set()
    for match in IDENTIFIER_RE.finditer(strip_code(text, language)):
        token = normalise(match.group(0))
        if is_hex_like(token):
            continue
        names.add(token)
    return names


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def language_of(path: Path) -> str:
    """The comment and string-literal rules of a file type."""
    suffix = path.suffix
    if suffix in (".py", ".pyi", ".pyx", ".pxd"):
        return "python"
    if suffix in (".c", ".h", ".hh", ".hpp", ".hxx", ".inc", ".cc", ".cpp", ".cxx",
                  ".m", ".mm", ".def", ".S", ".s", ".asm", ".in"):
        return "c"
    if suffix == ".rs":
        return "rust"
    return "text"


def is_skipped(relative: Path) -> bool:
    """True for a CPython file the index skips."""
    for part in relative.parts[:-1]:
        if part in SKIPPED_DIRECTORIES or part.startswith(SKIPPED_FILE_PREFIXES):
            return True
    return relative.name.startswith(SKIPPED_FILE_PREFIXES)


def cpython_source_files(root: Path) -> list[Path]:
    """The CPython source files the index is built from."""
    files: list[Path] = []
    for name in CPYTHON_SOURCE_DIRS:
        base = root / name
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file() or path.suffix not in SOURCE_SUFFIXES:
                continue
            if is_skipped(path.relative_to(root)):
                continue
            files.append(path)
    return files


def build_index(root: Path) -> tuple[dict[str, set[str]], list[str]]:
    """Map every name to the CPython files that use it."""
    index: dict[str, set[str]] = defaultdict(set)
    files = cpython_source_files(root)
    for path in files:
        relative = str(path.relative_to(root))
        for token in index_identifiers(read_text(path), language_of(path)):
            index[token].add(relative)
    return dict(index), [str(path.relative_to(root)) for path in files]


def module_names(root: Path) -> set[str]:
    """The names of CPython's modules.

    The modules of the Lib directory are the ones of the standard
    library; the native modules (the C modules of the Modules directory)
    count as well: "gc" is a module name as much as "os".
    """
    names: set[str] = set()
    names |= library_module_names(root / "Lib")
    names |= native_module_names(root)
    return names


def library_module_names(library: Path) -> set[str]:
    """The names of the modules and the packages of the Lib directory.

    The modules of the standard library live in the top level of the
    Lib directory, the modules of a package in the directory of the
    package: "email.mime" is a module as much as "os" is.
    """
    names: set[str] = set()
    if not library.is_dir():
        return names
    for path in sorted(library.iterdir()):
        if path.is_dir():
            if path.name in SKIPPED_DIRECTORIES or path.name.startswith("_"):
                continue
            names.add(normalise(path.name))
            names |= library_module_names(path)
        elif path.suffix == ".py":
            names.add(normalise(path.stem))
    return names


def module_name_from_definition(identifier: str) -> str:
    """The name of the module a definition declares.

    CPython names the definition of a module after the module itself:
    "gcmodule" and "gc_module" are the module "gc", "sslmodule_def" is
    the module "ssl". A definition whose name carries no such suffix
    ("moduledef") names no module at all.
    """
    name = identifier
    stripped = False
    while True:
        for suffix in MODULE_DEFINITION_SUFFIXES:
            if name.endswith(suffix) and len(name) > len(suffix):
                name = name[: -len(suffix)]
                stripped = True
                break
        else:
            break
    if not stripped or name.lower() in GENERIC_MODULE_NAMES:
        return ""
    return name


def native_module_names(root: Path) -> set[str]:
    """The names of CPython's native modules.

    A native module is a module of the Modules directory: the C modules
    of the standard library. Its name is the name of its initialisation
    function ("PyInit_gc" is the module "gc"), the name of its
    definition ("struct PyModuleDef gcmodule" is the module "gc") or
    the name the module gives itself (".m_name = \"gc\""). The name of
    the file does not name the module: "Modules/pythonrun.c" is a
    helper of the interpreter and "Modules/getpath.c" is a helper of
    the start-up, not modules.
    """
    names: set[str] = set()
    modules = root / "Modules"
    if not modules.is_dir():
        return names
    for path in sorted(modules.rglob("*.c")):
        if is_skipped(path.relative_to(root)):
            continue
        text = read_text(path)
        for match in PY_INIT_RE.finditer(text):
            names.add(normalise(match.group(1)))
        for match in PY_MODULE_DEF_RE.finditer(text):
            name = module_name_from_definition(match.group(1))
            if name:
                names.add(normalise(name))
        for match in MODULE_NAME_FIELD_RE.finditer(text):
            names.add(normalise(match.group(1)))
        for match in MODULE_NAME_MACRO_RE.finditer(text):
            names.add(normalise(match.group(1)))
    return names


def span_identifiers(text: str) -> set[str]:
    """The identifiers of a piece of documentation.

    The text of a span is prose: the quotes in it are quotes of the
    prose, not the delimiters of a string literal, so the names are
    read from the text itself and nothing is dropped.
    """
    return {normalise(match.group(0)) for match in IDENTIFIER_RE.finditer(text)}


def doc_public_names(root: Path) -> set[str]:
    """The names the documentation uses as public vocabulary."""
    names: set[str] = set()
    documentation = root / "Doc"
    if not documentation.is_dir():
        return names
    for path in sorted(documentation.rglob("*.rst")):
        text = read_text(path)
        for match in DOC_CODE_SPAN_RE.finditer(text):
            names |= span_identifiers(match.group(1))
        for match in DOC_ROLE_RE.finditer(text):
            names |= span_identifiers(match.group(1))
        for match in DIRECTIVE_RE.finditer(text):
            names |= signature_identifiers(match.group(1))
    return names


def signature_identifiers(signature: str) -> set[str]:
    """The identifiers of a directive signature.

    The name of the directive and the parameters of its signature count:
    the parameters of a documented function are public vocabulary as
    well, not only its name. The prose behind the signature is prose
    and not a name.
    """
    match = DIRECTIVE_SIGNATURE_RE.match(signature.strip())
    if not match:
        return set()
    return {normalise(name) for name in IDENTIFIER_RE.findall(match.group(0))}


def public_vocabulary(root: Path) -> tuple[set[str], set[str]]:
    """The public names and the module names of the CPython tree."""
    names: set[str] = set()
    for relative in PUBLIC_VOCABULARY_FILES:
        path = root / relative
        if not path.is_file():
            continue
        text = read_text(path)
        language = language_of(path)
        names |= all_identifiers(text, language)
        # The vocabulary of the language is often written as text: the
        # keywords of Lib/keyword.py and the aliases of
        # Lib/encodings/aliases.py are string literals.
        names |= string_literal_identifiers(text, language)
    names |= doc_public_names(root)
    modules = module_names(root)
    names |= modules
    return names, modules


def is_public(token: str, public: set[str], modules: set[str]) -> bool:
    """True for a name that is public vocabulary rather than an internal name."""
    if token in public or token in modules:
        return True
    for split in range(1, len(token)):
        prefix, suffix = token[:split], token[split:]
        if suffix in public and (prefix in public or prefix in modules):
            return True
    return False


def shared_names(text: str, index: dict[str, set[str]], public: set[str], modules: set[str],
                 language: str = "rust") -> dict[str, set[str]]:
    """Map every CPython file to the private names it shares with the text."""
    shared: dict[str, set[str]] = defaultdict(set)
    for token in code_identifiers(text, language):
        files = index.get(token)
        if not files or len(files) > MAX_CPYTHON_FILES:
            continue
        if is_public(token, public, modules):
            continue
        for name in files:
            shared[name].add(token)
    return dict(shared)


def scan_text(text: str, index: dict[str, set[str]], public: set[str], modules: set[str],
              language: str = "rust") -> list[dict]:
    """The findings of one file: the CPython files it shares four names with."""
    findings = []
    for reference, names in sorted(shared_names(text, index, public, modules, language).items()):
        if len(names) >= MINIMUM_SHARED_NAMES:
            findings.append({"reference": reference, "names": sorted(names)})
    return findings


def scan_files(paths: list[Path], index: dict[str, set[str]], public: set[str], modules: set[str],
               root: Path | None = None) -> list[dict]:
    """The findings of every file, with the file they belong to.

    The file is named relative to ``root`` when one is given.
    """
    findings = []
    for path in paths:
        text = read_text(path)
        for finding in scan_text(text, index, public, modules, language_of(path)):
            findings.append({
                "file": relative_name(path, root),
                "reference": finding["reference"],
                "names": finding["names"],
                "count": len(finding["names"]),
            })
    return findings


def relative_name(path: Path, root: Path | None) -> str:
    """The name of a file, relative to the repository when possible."""
    if root is None:
        return str(path)
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def format_finding(finding: dict) -> str:
    """The report line of a finding."""
    names = ", ".join(finding["names"])
    return (f"{finding['file']}:1: provenance: {finding['count']} internal names "
            f"shared with {finding['reference']}: {names}")


def tracked_rust_files(repo: Path) -> list[str]:
    """The tracked Rust files of the repository."""
    try:
        completed = subprocess.run(["git", "ls-files", "-z", "*.rs"], cwd=repo, capture_output=True, text=True)
    except OSError:
        return []
    if completed.returncode != 0:
        return []
    return [name for name in completed.stdout.split("\0") if name]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--cpython-src", required=True, help="The CPython source tree to compare with.")
    parser.add_argument("--repo", default=".", help="The MiniPython checkout (for the default file list).")
    parser.add_argument("--files", nargs="*", default=None, help="The files to scan (default: every tracked .rs file).")
    parser.add_argument("--json", default=None, help="Write the findings as JSON to this file.")
    args = parser.parse_args(argv)

    root = Path(args.cpython_src).resolve()
    if not root.is_dir():
        print(f"provenance: {root} is not a directory", file=sys.stderr)
        return 2
    if args.files:
        paths = [Path(name) for name in args.files]
    else:
        repo = Path(args.repo).resolve()
        paths = [repo / name for name in tracked_rust_files(repo)]

    index, scanned = build_index(root)
    public, modules = public_vocabulary(root)
    findings = scan_files(paths, index, public, modules)
    payload = {
        "cpython_src": str(root),
        "cpython_files": len(scanned),
        "public_names": len(public),
        "files_scanned": len(paths),
        "findings": findings,
        "passed": not findings,
    }
    if args.json:
        Path(args.json).write_text(json.dumps(payload, indent=2) + "\n")
    print(f"provenance: {len(paths)} file(s) against {len(scanned)} CPython files: "
          f"{len(findings)} finding(s)")
    for finding in findings:
        print("  " + format_finding(finding))
    return 0 if not findings else 1


if __name__ == "__main__":
    raise SystemExit(main())
