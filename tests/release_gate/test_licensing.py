"""Unit tests for the release gate's licensing check (run by the gate step).

The tests build small temporary trees: a MiniPython checkout with a few
source files, and a CPython source tree with a few files, and check the
licensing rules against them. Nothing outside the temporary directory is
read or written.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import licensing  # noqa: E402
import provenance_names  # noqa: E402

MIT_HEADER = "// SPDX-License-Identifier: MIT\n"
PSF_HEADER = (
    "// SPDX-License-Identifier: PSF-2.0 AND MIT\n"
    "// Derived from CPython Lib/thing.py\n"
    "// Changes: rewritten in Rust\n"
)
# Four internal names that no public vocabulary contains.
PRIVATE_NAMES = ("mnp_private_alpha", "mnp_private_bravo", "mnp_private_charlie", "mnp_private_delta")


class Tree:
    """A temporary directory that stands in for a checkout."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def write(self, name: str, text: str) -> Path:
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

    def read(self, name: str) -> str:
        return (self.root / name).read_text(encoding="utf-8")


@unittest.skipUnless(shutil.which("git"), "git is needed to build the fixture")
class LicensingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="minipython-licensing-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.repo = Tree(self.tmp / "repo")
        self.repo.root.mkdir()
        try:
            subprocess.run(["git", "init", "-q"], cwd=self.repo.root, check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            self.git_works = True
        except (OSError, subprocess.CalledProcessError):
            self.git_works = False
        self.cpython = Tree(self.tmp / "cpython")
        self.cpython.root.mkdir()

    def track(self) -> None:
        if not self.git_works:
            self.skipTest("git cannot create a repository here")
        subprocess.run(["git", "add", "-A"], cwd=self.repo.root, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def cpython_tree(self, files: dict[str, str]) -> Tree:
        for name, text in files.items():
            self.cpython.write(name, text)
        return self.cpython

    # -- the file list ---------------------------------------------------

    def test_only_the_licensed_directories_and_types_count(self):
        self.repo.write("src/main.rs", MIT_HEADER)
        self.repo.write("tests/test_x.py", MIT_HEADER)
        self.repo.write("ci/run.sh", MIT_HEADER)
        self.repo.write("npm/scripts/install.js", MIT_HEADER)
        self.repo.write("python/pyproject.toml", MIT_HEADER)
        self.repo.write("examples/sandbox/demo.py", MIT_HEADER)
        self.repo.write("homebrew-tap-template/Formula/minipython.rb", MIT_HEADER)
        self.repo.write("docs/readme.md", MIT_HEADER)
        self.repo.write("top.py", MIT_HEADER)
        self.track()
        self.assertEqual(
            licensing.source_files(self.repo.root),
            ["ci/run.sh", "examples/sandbox/demo.py", "npm/scripts/install.js",
             "python/pyproject.toml", "src/main.rs", "tests/test_x.py"],
        )

    # -- the SPDX header -------------------------------------------------

    def test_a_header_in_the_first_five_lines_is_accepted(self):
        self.repo.write("src/main.rs", "// a comment\n// another\n\n" + MIT_HEADER + "fn main() {}\n")
        problems, details = licensing.check_files(self.repo.root, ["src/main.rs"], [])
        self.assertEqual(problems, [])
        self.assertEqual(details["psf_files"], [])

    def test_a_header_after_the_fifth_line_is_not_a_header(self):
        self.repo.write("src/main.rs", "\n\n\n\n\n" + MIT_HEADER + "fn main() {}\n")
        problems, _ = licensing.check_files(self.repo.root, ["src/main.rs"], [])
        self.assertEqual(len(problems), 1)
        self.assertIn("no SPDX header", problems[0])

    def test_a_file_without_a_header_is_reported(self):
        self.repo.write("src/main.rs", "fn main() {}\n")
        problems, details = licensing.check_files(self.repo.root, ["src/main.rs"], [])
        self.assertEqual(len(problems), 1)
        self.assertIn("no SPDX header", problems[0])
        self.assertEqual(details["files_without_licence"], ["src/main.rs"])

    def test_a_header_in_a_block_comment_is_accepted(self):
        self.repo.write("src/main.rs", "/* SPDX-License-Identifier: MIT */\nfn main() {}\n")
        problems, _ = licensing.check_files(self.repo.root, ["src/main.rs"], [])
        self.assertEqual(problems, [])

    def test_an_spdx_tag_in_a_string_is_not_a_header(self):
        # The tag of a string is text, not a licence header: a file
        # without a header of its own has none.
        self.repo.write("src/main.rs",
                        'const TEXT: &str = "\n// SPDX-License-Identifier: MIT\n";\n')
        problems, details = licensing.check_files(self.repo.root, ["src/main.rs"], [])
        self.assertEqual(len(problems), 1)
        self.assertIn("no SPDX header", problems[0])
        self.assertEqual(details["psf_files"], [])

    def test_an_spdx_tag_in_a_python_string_is_not_a_header(self):
        self.repo.write("src/main.py", 'TEXT = """\n# SPDX-License-Identifier: MIT\n"""\n')
        problems, _ = licensing.check_files(self.repo.root, ["src/main.py"], [])
        self.assertEqual(len(problems), 1)
        self.assertIn("no SPDX header", problems[0])

    def test_an_spdx_tag_that_is_not_in_a_comment_is_not_a_header(self):
        self.repo.write("src/main.rs", 'let tag = "SPDX-License-Identifier: MIT";\n')
        problems, _ = licensing.check_files(self.repo.root, ["src/main.rs"], [])
        self.assertEqual(len(problems), 1)
        self.assertIn("no SPDX header", problems[0])

    def test_the_header_of_every_language_is_read(self):
        # The comment syntax of the language decides what a header is.
        for name, header in (("src/main.rs", "// SPDX-License-Identifier: MIT\n"),
                             ("src/main.py", "# SPDX-License-Identifier: MIT\n"),
                             ("ci/run.sh", "# SPDX-License-Identifier: MIT\n"),
                             ("npm/scripts/install.js", "// SPDX-License-Identifier: MIT\n"),
                             ("python/pyproject.toml", "# SPDX-License-Identifier: MIT\n")):
            with self.subTest(name=name):
                self.repo.write(name, header)
                problems, _ = licensing.check_files(self.repo.root, [name], [])
                self.assertEqual(problems, [])

    def test_an_spdx_tag_in_a_toml_multiline_string_is_not_a_header(self):
        # The text of a multi-line string is text: the "#" of it is not
        # a comment, so the tag inside it is not a licence header.
        self.repo.write("python/pyproject.toml",
                        '[project]\nname = "minipython"\n'
                        'description = """\n# SPDX-License-Identifier: MIT\n"""\n')
        problems, details = licensing.check_files(self.repo.root, ["python/pyproject.toml"], [])
        self.assertEqual(len(problems), 1)
        self.assertIn("no SPDX header", problems[0])
        self.assertEqual(details["psf_files"], [])

    def test_an_expression_outside_the_allowed_set_is_reported(self):
        self.repo.write("src/main.rs", "// SPDX-License-Identifier: GPL-3.0\nfn main() {}\n")
        problems, _ = licensing.check_files(self.repo.root, ["src/main.rs"], [])
        self.assertEqual(len(problems), 1)
        self.assertIn("GPL-3.0", problems[0])

    def test_the_allowed_expressions_are_accepted(self):
        for expression in licensing.ALLOWED_EXPRESSIONS:
            with self.subTest(expression=expression):
                header = (f"// SPDX-License-Identifier: {expression}\n"
                          + ("// Derived from CPython Lib/thing.py\n// Changes: rewritten in Rust\n"
                             if "PSF-2.0" in expression else ""))
                self.repo.write("src/main.rs", header)
                problems, _ = licensing.check_files(self.repo.root, ["src/main.rs"], [])
                self.assertEqual(problems, [])

    # -- REUSE.toml ------------------------------------------------------

    def test_a_reuse_rule_covers_a_file(self):
        self.repo.write("src/main.rs", "fn main() {}\n")
        self.repo.write("REUSE.toml", 'version = 1\n\n[[annotations]]\npath = "src/**"\n'
                                    'SPDX-License-Identifier = "MIT"\n')
        rules, failures = licensing.load_reuse_rules(self.repo.root)
        self.assertEqual(failures, [])
        self.assertEqual(len(rules), 1)
        problems, details = licensing.check_files(self.repo.root, ["src/main.rs"], rules)
        self.assertEqual(problems, [])
        self.assertEqual(details["psf_files"], [])
        self.track()
        self.assertEqual(licensing.mit_only_rust_files(self.repo.root, rules), ["src/main.rs"])

    def test_a_reuse_rule_with_a_list_of_paths_covers_every_path(self):
        # The path of an annotation is a path or a list of paths: every
        # path of the list is a rule of its own.
        self.repo.write("src/main.rs", "fn main() {}\n")
        self.repo.write("src/other.rs", "fn other() {}\n")
        self.repo.write("REUSE.toml", '[[annotations]]\npath = ["src/main.rs", "src/other.rs"]\n'
                                    'SPDX-License-Identifier = "MIT"\n')
        rules, failures = licensing.load_reuse_rules(self.repo.root)
        self.assertEqual(failures, [])
        self.assertEqual([rule["pattern"] for rule in rules], ["src/main.rs", "src/other.rs"])
        problems, _ = licensing.check_files(self.repo.root, ["src/main.rs", "src/other.rs"], rules)
        self.assertEqual(problems, [])

    def test_a_reuse_rule_with_an_unknown_expression_is_reported(self):
        self.repo.write("REUSE.toml", '[[annotations]]\npath = "src/**"\n'
                                    'SPDX-License-Identifier = "GPL-3.0"\n')
        rules, failures = licensing.load_reuse_rules(self.repo.root)
        self.assertEqual(rules, [])
        self.assertEqual(len(failures), 1)
        self.assertIn("GPL-3.0", failures[0])

    def test_a_reuse_rule_needs_a_path_and_an_expression(self):
        self.repo.write("REUSE.toml", '[[annotations]]\npath = "src/**"\n')
        rules, failures = licensing.load_reuse_rules(self.repo.root)
        self.assertEqual(rules, [])
        self.assertEqual(len(failures), 1)

    def test_the_path_patterns_of_reuse_are_read_as_globs(self):
        rule = licensing.glob_to_regex("tests/**")
        self.assertTrue(rule.match("tests/release_gate/gate.py"))
        self.assertFalse(rule.match("src/main.rs"))
        single = licensing.glob_to_regex("src/*.rs")
        self.assertTrue(single.match("src/main.rs"))
        self.assertFalse(single.match("src/host/clock.rs"))

    # -- PSF-2.0 files and NOTICE ----------------------------------------

    def test_a_psf_file_needs_the_derived_line_the_changes_line_and_a_notice_entry(self):
        self.repo.write("src/ported.rs", "// SPDX-License-Identifier: PSF-2.0 AND MIT\nfn f() {}\n")
        problems, details = licensing.check_files(self.repo.root, ["src/ported.rs"], [])
        self.assertEqual(details["psf_files"], ["src/ported.rs"])
        self.assertTrue(any("Derived from CPython" in problem for problem in problems))
        self.assertTrue(any("Changes:" in problem for problem in problems))
        notice_problems, _ = licensing.check_notice(self.repo.root, details["psf_files"])
        self.assertTrue(any("NOTICE is missing" in problem for problem in notice_problems))

    def test_a_complete_psf_file_is_accepted(self):
        self.repo.write("src/ported.rs", PSF_HEADER + "fn f() {}\n")
        self.repo.write("NOTICE", "src/ported.rs\n")
        problems, details = licensing.check_files(self.repo.root, ["src/ported.rs"], [])
        self.assertEqual(problems, [])
        notice_problems, entries = licensing.check_notice(self.repo.root, details["psf_files"])
        self.assertEqual(notice_problems, [])
        self.assertEqual(entries, ["src/ported.rs"])

    def test_notice_must_not_list_a_path_that_does_not_exist(self):
        self.repo.write("src/ported.rs", PSF_HEADER)
        self.repo.write("NOTICE", "src/ported.rs\nsrc/gone.rs\n")
        problems, entries = licensing.check_notice(self.repo.root, ["src/ported.rs"])
        self.assertEqual(entries, ["src/ported.rs", "src/gone.rs"])
        self.assertTrue(any("src/gone.rs" in problem and "does not exist" in problem for problem in problems))

    def test_notice_must_not_list_a_path_without_a_psf_header(self):
        self.repo.write("src/other.rs", MIT_HEADER)
        self.repo.write("NOTICE", "src/other.rs\n")
        problems, _ = licensing.check_notice(self.repo.root, [])
        self.assertTrue(any("no PSF-2.0 header" in problem for problem in problems))

    def test_a_psf_file_must_be_listed_in_notice(self):
        self.repo.write("src/ported.rs", PSF_HEADER)
        self.repo.write("NOTICE", "src/other.rs\n")
        self.repo.write("src/other.rs", PSF_HEADER)
        problems, _ = licensing.check_notice(self.repo.root, ["src/ported.rs"])
        self.assertTrue(any("not listed in NOTICE" in problem for problem in problems))

    def test_a_reuse_rule_with_a_psf_expression_needs_no_derived_line(self):
        self.repo.write("src/ported.rs", "fn f() {}\n")
        self.repo.write("REUSE.toml", '[[annotations]]\npath = "src/**"\n'
                                    'SPDX-License-Identifier = "MIT AND PSF-2.0"\n')
        rules, failures = licensing.load_reuse_rules(self.repo.root)
        self.assertEqual(failures, [])
        problems, details = licensing.check_files(self.repo.root, ["src/ported.rs"], rules)
        self.assertEqual(problems, [])
        self.assertEqual(details["psf_files"], [])
        self.track()
        self.assertEqual(licensing.mit_only_rust_files(self.repo.root, rules), [])

    def test_a_notice_entry_needs_its_own_psf_header(self):
        # A REUSE.toml rule does not make the entry a PSF-2.0 file: the
        # file itself has to say so.
        self.repo.write("src/ported.rs", "fn f() {}\n")
        self.repo.write("REUSE.toml", '[[annotations]]\npath = "src/**"\n'
                                    'SPDX-License-Identifier = "MIT AND PSF-2.0"\n')
        rules, failures = licensing.load_reuse_rules(self.repo.root)
        self.assertEqual(failures, [])
        self.assertEqual(len(rules), 1)
        self.repo.write("NOTICE", "src/ported.rs\n")
        problems, entries = licensing.check_notice(self.repo.root, [])
        self.assertEqual(entries, ["src/ported.rs"])
        self.assertTrue(any("no PSF-2.0 header" in problem for problem in problems))

    def test_the_derived_line_needs_a_line_start_prefix(self):
        # Only "// Derived from CPython" and "# Derived from CPython"
        # count: a line without a comment marker, or one that uses
        # another marker, does not say that the file comes from CPython.
        for marker in ("/* Derived from CPython Lib/thing.py */",
                       "Derived from CPython Lib/thing.py",
                       " * Derived from CPython Lib/thing.py"):
            with self.subTest(marker=marker):
                self.repo.write("src/ported.rs", "// SPDX-License-Identifier: PSF-2.0 AND MIT\n"
                                                f"{marker}\n"
                                                "// Changes: rewritten in Rust\n")
                problems, _ = licensing.check_files(self.repo.root, ["src/ported.rs"], [])
                self.assertTrue(any("Derived from CPython" in problem for problem in problems))

    def test_the_derived_line_may_start_with_a_hash(self):
        self.repo.write("src/ported.py", "# SPDX-License-Identifier: PSF-2.0 AND MIT\n"
                                        "# Derived from CPython Lib/thing.py\n"
                                        "# Changes: rewritten in Python\n")
        problems, details = licensing.check_files(self.repo.root, ["src/ported.py"], [])
        self.assertEqual(problems, [])
        self.assertEqual(details["psf_files"], ["src/ported.py"])

    def test_a_derived_marker_in_a_string_is_not_a_derived_line(self):
        # The text of a string is not a comment: the marker of a
        # multi-line string does not say that the file comes from
        # CPython, and it is not a "Changes:" line either.
        self.repo.write("src/ported.rs",
                        "// SPDX-License-Identifier: PSF-2.0 AND MIT\n"
                        'let text = "\n// Derived from CPython Lib/thing.py\n'
                        '// Changes: nothing at all\n";\n')
        problems, details = licensing.check_files(self.repo.root, ["src/ported.rs"], [])
        self.assertEqual(details["psf_files"], ["src/ported.rs"])
        self.assertTrue(any("Derived from CPython" in problem for problem in problems))
        self.assertTrue(any("Changes:" in problem for problem in problems))

    def test_a_derived_marker_in_a_python_docstring_is_not_a_derived_line(self):
        self.repo.write("src/ported.py",
                        "# SPDX-License-Identifier: PSF-2.0 AND MIT\n"
                        '"""\n# Derived from CPython Lib/thing.py\n# Changes: nothing\n"""\n')
        problems, details = licensing.check_files(self.repo.root, ["src/ported.py"], [])
        self.assertEqual(details["psf_files"], ["src/ported.py"])
        self.assertTrue(any("Derived from CPython" in problem for problem in problems))
        self.assertTrue(any("Changes:" in problem for problem in problems))

    def test_a_derived_marker_behind_an_empty_string_is_not_a_derived_line(self):
        # The delimiters of a string are content: a comment behind an
        # empty string does not start its line, so the marker of such a
        # line does not say that the file comes from CPython.
        self.repo.write("src/ported.py",
                        "# SPDX-License-Identifier: PSF-2.0 AND MIT\n"
                        '"" # Derived from CPython Lib/thing.py\n'
                        "# Changes: rewritten in Python\n")
        problems, details = licensing.check_files(self.repo.root, ["src/ported.py"], [])
        self.assertEqual(details["psf_files"], ["src/ported.py"])
        self.assertTrue(any("Derived from CPython" in problem for problem in problems))

    def test_a_derived_marker_behind_an_empty_string_is_not_a_derived_line_in_rust(self):
        self.repo.write("src/ported.rs",
                        "// SPDX-License-Identifier: PSF-2.0 AND MIT\n"
                        '"" // Derived from CPython Lib/thing.py\n'
                        "// Changes: rewritten in Rust\n")
        problems, details = licensing.check_files(self.repo.root, ["src/ported.rs"], [])
        self.assertEqual(details["psf_files"], ["src/ported.rs"])
        self.assertTrue(any("Derived from CPython" in problem for problem in problems))

    def test_a_changes_marker_behind_an_empty_string_is_not_a_changes_line(self):
        # The same holds for the "Changes:" marker: it has to start the
        # comment of the line as well.
        self.repo.write("src/ported.py",
                        "# SPDX-License-Identifier: PSF-2.0 AND MIT\n"
                        "# Derived from CPython Lib/thing.py\n"
                        '"" # Changes: rewritten in Python\n')
        problems, _ = licensing.check_files(self.repo.root, ["src/ported.py"], [])
        self.assertTrue(any("Changes:" in problem for problem in problems))

    def test_a_marker_in_a_comment_of_the_eleventh_line_does_not_count(self):
        # Only the first ten lines count.
        self.repo.write("src/ported.rs",
                        "// SPDX-License-Identifier: PSF-2.0 AND MIT\n"
                        + "// filler\n" * 9
                        + "// Derived from CPython Lib/thing.py\n"
                        + "// Changes: rewritten in Rust\n")
        problems, _ = licensing.check_files(self.repo.root, ["src/ported.rs"], [])
        self.assertTrue(any("Derived from CPython" in problem for problem in problems))
        self.assertTrue(any("Changes:" in problem for problem in problems))

    def test_every_tracked_rust_file_is_mit_only(self):
        # A Rust file outside the directories that need a licence header
        # is MiniPython's own too, so it is scanned for provenance.
        self.repo.write("build.rs", "fn main() {}\n")
        self.repo.write("src/ported.rs", PSF_HEADER)
        self.repo.write("src/main.rs", MIT_HEADER)
        self.repo.write("src/notes.txt", "not a source file\n")
        self.track()
        rules, _ = licensing.load_reuse_rules(self.repo.root)
        self.assertEqual(licensing.mit_only_rust_files(self.repo.root, rules),
                         ["build.rs", "src/main.rs"])

    def test_a_root_rust_file_is_scanned_for_provenance(self):
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        self.cpython_tree(cpython)
        self.repo.write("build.rs", "\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES))
        self.repo.write("src/main.rs", MIT_HEADER)
        self.track()
        rules, _ = licensing.load_reuse_rules(self.repo.root)
        mit_only = licensing.mit_only_rust_files(self.repo.root, rules)
        failures, details = licensing.check_provenance(self.repo.root, mit_only, self.cpython.root)
        self.assertEqual(details["scanned"], 2)
        self.assertEqual(len(failures), 1)
        self.assertIn("build.rs", failures[0])
        self.assertIn("provenance:", failures[0])

    def test_every_notice_line_names_a_path(self):
        # NOTICE holds the paths of the files derived from CPython, one
        # per line: every line that is not empty and not a comment
        # names a file, and a line that names no file fails the step.
        self.repo.write("NOTICE", "MiniPython is MIT licensed.\n\n# a comment\nsrc/ported.rs\n")
        entries, exists = licensing.notice_entries(self.repo.root)
        self.assertTrue(exists)
        self.assertEqual(entries, ["MiniPython is MIT licensed.", "src/ported.rs"])

    def test_a_notice_line_that_is_not_a_path_is_reported(self):
        self.repo.write("NOTICE", "MiniPython is MIT licensed.\n")
        problems, entries = licensing.check_notice(self.repo.root, [])
        self.assertEqual(entries, ["MiniPython is MIT licensed."])
        self.assertTrue(any("not a path" in problem for problem in problems))

    def test_notice_must_not_list_a_path_that_has_no_slash(self):
        # A path of NOTICE that does not exist fails the step, however
        # it is spelled: "MISSING" is not a file of the repository.
        self.repo.write("NOTICE", "MISSING\n")
        problems, entries = licensing.check_notice(self.repo.root, [])
        self.assertEqual(entries, ["MISSING"])
        self.assertTrue(any("MISSING" in problem and "does not exist" in problem for problem in problems))

    def test_a_notice_path_with_a_space_is_a_path(self):
        # A path of the repository may hold a space: "src/ported file.rs"
        # is a file of the repository.
        self.repo.write("src/ported file.rs", PSF_HEADER)
        self.repo.write("NOTICE", "src/ported file.rs\n")
        problems, entries = licensing.check_notice(self.repo.root, ["src/ported file.rs"])
        self.assertEqual(entries, ["src/ported file.rs"])
        self.assertEqual(problems, [])

    def test_a_notice_path_with_a_space_that_does_not_exist_fails(self):
        self.repo.write("NOTICE", "src/gone file.rs\n")
        problems, _ = licensing.check_notice(self.repo.root, [])
        self.assertTrue(any("gone file.rs" in problem for problem in problems))

    def test_notice_must_not_list_a_path_outside_the_repository(self):
        self.repo.write("NOTICE", "../elsewhere/ported.rs\n")
        problems, _ = licensing.check_notice(self.repo.root, [])
        self.assertTrue(any("not a path" in problem for problem in problems))

    def test_notice_must_not_list_an_absolute_path(self):
        # A path that leaves the repository is not a path of it, however
        # it is spelled.
        for entry in ("/etc/passwd", "~/ported.rs"):
            with self.subTest(entry=entry):
                self.repo.write("NOTICE", entry + "\n")
                problems, _ = licensing.check_notice(self.repo.root, [])
                self.assertTrue(any("not a path" in problem for problem in problems))

    # -- the CPython origins ---------------------------------------------

    def test_the_origin_of_a_derived_file_is_read(self):
        # The header of a derived file names the CPython files it comes
        # from; the version of CPython and the copyright notice are not
        # paths.
        line = ("// Derived from CPython 3.14.6 Lib/re/__init__.py, "
                "Objects/complexobject.c (Copyright (c) 2001 Python Software "
                "Foundation; All Rights Reserved).")
        self.assertEqual(licensing.origin_paths(line),
                         ["Lib/re/__init__.py", "Objects/complexobject.c"])
        self.assertEqual(licensing.origin_paths("// a comment"), [])

    def test_a_cpython_file_that_does_not_exist_is_reported(self):
        # The CPython file a derived file names has to exist in the
        # CPython source tree: a path that names nothing cannot be the
        # origin of the file.
        self.cpython_tree({"Lib/thing.py": "x = 1\n"})
        self.repo.write("src/ported.rs", PSF_HEADER)
        self.repo.write("NOTICE", "src/ported.rs\n# CPython origin: Lib/thing.py\n")
        problems, origins = licensing.check_origins(
            self.repo.root, ["src/ported.rs"], self.cpython.root)
        self.assertEqual(problems, [])
        self.assertEqual(origins, {"src/ported.rs": ["Lib/thing.py"]})

        self.cpython.root.joinpath("Lib/thing.py").unlink()
        problems, _ = licensing.check_origins(
            self.repo.root, ["src/ported.rs"], self.cpython.root)
        self.assertTrue(any("Lib/thing.py" in problem and "does not exist" in problem
                            for problem in problems))

    def test_the_notice_and_the_header_must_name_the_same_origin(self):
        # The NOTICE names the origin of a file below its path; it has
        # to be the origin the header of the file names.
        self.cpython_tree({"Lib/thing.py": "x = 1\n", "Lib/other.py": "y = 2\n"})
        self.repo.write("src/ported.rs", PSF_HEADER)
        self.repo.write("NOTICE", "src/ported.rs\n# CPython origin: Lib/other.py\n")
        problems, _ = licensing.check_origins(
            self.repo.root, ["src/ported.rs"], self.cpython.root)
        self.assertTrue(any("NOTICE" in problem for problem in problems))

    def test_a_file_without_an_origin_in_the_notice_is_reported(self):
        self.cpython_tree({"Lib/thing.py": "x = 1\n"})
        self.repo.write("src/ported.rs", PSF_HEADER)
        self.repo.write("NOTICE", "src/ported.rs\n")
        problems, _ = licensing.check_origins(
            self.repo.root, ["src/ported.rs"], self.cpython.root)
        self.assertTrue(any("NOTICE" in problem for problem in problems))

    def test_the_notice_of_the_npm_package_names_the_same_origin(self):
        # The npm package carries a copy of the NOTICE: the copy has to
        # name the origin the header of the file names.
        self.cpython_tree({"Lib/thing.py": "x = 1\n", "Lib/other.py": "y = 2\n"})
        self.repo.write("src/ported.rs", PSF_HEADER)
        self.repo.write("NOTICE", "src/ported.rs\n# CPython origin: Lib/thing.py\n")
        self.repo.write("npm/NOTICE", "src/ported.rs\n# CPython origin: Lib/other.py\n")
        problems, _ = licensing.check_origins(
            self.repo.root, ["src/ported.rs"], self.cpython.root)
        self.assertTrue(any("npm/NOTICE" in problem for problem in problems))

    def test_the_origins_are_not_checked_without_the_cpython_source_tree(self):
        # Without the CPython source tree there is nothing to check the
        # origins against; the provenance scan reports the missing tree.
        self.repo.write("src/ported.rs", PSF_HEADER)
        problems, origins = licensing.check_origins(self.repo.root, ["src/ported.rs"], None)
        self.assertEqual(problems, [])
        self.assertEqual(origins, {})

    # -- the LICENSES directory ------------------------------------------

    def test_the_licence_files_must_exist_and_not_be_empty(self):
        self.assertEqual(
            licensing.licence_files(self.repo.root),
            ["LICENSES/MIT.txt is missing", "LICENSES/PSF-2.0.txt is missing"],
        )
        self.repo.write("LICENSES/MIT.txt", "MIT License\n")
        self.repo.write("LICENSES/PSF-2.0.txt", "  \n")
        self.assertEqual(licensing.licence_files(self.repo.root), ["LICENSES/PSF-2.0.txt is empty"])
        self.repo.write("LICENSES/PSF-2.0.txt", "PYTHON SOFTWARE FOUNDATION LICENSE\n")
        self.assertEqual(licensing.licence_files(self.repo.root), [])

    # -- the dependency licences -----------------------------------------

    def test_the_licence_expressions_are_read(self):
        self.assertEqual(licensing.license_identifiers("MIT OR Apache-2.0"), ["MIT", "Apache-2.0"])
        self.assertEqual(
            licensing.license_identifiers("(MIT OR Apache-2.0) AND Unicode-3.0"),
            ["MIT", "Apache-2.0", "Unicode-3.0"],
        )
        self.assertEqual(
            licensing.license_identifiers("Apache-2.0 WITH LLVM-exception"),
            ["Apache-2.0 WITH LLVM-exception"],
        )
        self.assertEqual(licensing.license_identifiers("MIT/Apache-2.0"), ["MIT", "Apache-2.0"])
        with self.assertRaises(ValueError):
            licensing.license_identifiers("(MIT OR Apache-2.0")

    def test_the_dependencies_outside_the_allowlist_are_reported(self):
        payload = {
            "workspace_members": ["pkg minipython"],
            "packages": [
                {"id": "pkg minipython", "name": "minipython", "version": "0.1.0", "license": "MIT"},
                {"id": "pkg serde", "name": "serde", "version": "1.0.0", "license": "MIT OR Apache-2.0"},
                {"id": "pkg wasi", "name": "wasi", "version": "0.11.3",
                 "license": "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT"},
                {"id": "pkg copyleft", "name": "copyleft", "version": "1.0.0", "license": "GPL-3.0-only"},
                {"id": "pkg odd", "name": "odd", "version": "1.0.0", "license": None},
            ],
        }
        offenders = licensing.dependency_offenders(payload)
        self.assertEqual([offender["package"] for offender in offenders], ["copyleft 1.0.0", "odd 1.0.0"])
        self.assertIn("GPL-3.0-only", offenders[0]["reason"])
        self.assertIn("no SPDX licence expression", offenders[1]["reason"])

    def test_a_dependency_under_the_unlicense_is_allowed(self):
        # memchr, a transitive dependency of serde_json, is dual licensed
        # (Unlicense OR MIT); the project uses it under MIT.
        payload = {
            "workspace_members": ["pkg minipython"],
            "packages": [
                {"id": "pkg minipython", "name": "minipython", "version": "0.1.0", "license": "MIT"},
                {"id": "pkg memchr", "name": "memchr", "version": "2.8.3", "license": "Unlicense OR MIT"},
            ],
        }
        self.assertEqual(licensing.dependency_offenders(payload), [])

    def test_a_dependency_under_the_unlicense_alone_is_reported(self):
        # The Unlicense is a public-domain dedication and is no licence
        # of its own here: it may be one branch of a choice, but a
        # dependency published under the Unlicense alone is not
        # accepted.
        payload = {
            "workspace_members": ["pkg minipython"],
            "packages": [
                {"id": "pkg minipython", "name": "minipython", "version": "0.1.0", "license": "MIT"},
                {"id": "pkg public_domain", "name": "public_domain", "version": "1.0.0",
                 "license": "Unlicense"},
            ],
        }
        offenders = licensing.dependency_offenders(payload)
        self.assertEqual([offender["package"] for offender in offenders], ["public_domain 1.0.0"])
        self.assertIn("Unlicense", offenders[0]["reason"])

    def test_a_dependency_under_a_choice_of_allowed_licences_is_allowed(self):
        # A choice is allowed when one of its branches is: the
        # dependency may be used under that branch.
        payload = {
            "workspace_members": ["pkg minipython"],
            "packages": [
                {"id": "pkg minipython", "name": "minipython", "version": "0.1.0", "license": "MIT"},
                {"id": "pkg either", "name": "either", "version": "1.0.0",
                 "license": "MIT OR GPL-3.0-only"},
                {"id": "pkg both", "name": "both", "version": "1.0.0",
                 "license": "MIT AND Apache-2.0"},
            ],
        }
        self.assertEqual(licensing.dependency_offenders(payload), [])

    def test_a_dependency_under_a_conjunction_with_a_forbidden_licence_is_reported(self):
        # A conjunction is a conjunction: every one of its factors
        # applies, so a copyleft factor cannot be dropped.
        payload = {
            "workspace_members": ["pkg minipython"],
            "packages": [
                {"id": "pkg minipython", "name": "minipython", "version": "0.1.0", "license": "MIT"},
                {"id": "pkg copyleft", "name": "copyleft", "version": "1.0.0",
                 "license": "MIT AND GPL-3.0-only"},
                {"id": "pkg dedication", "name": "dedication", "version": "1.0.0",
                 "license": "Unlicense AND MIT"},
            ],
        }
        offenders = licensing.dependency_offenders(payload)
        self.assertEqual([offender["package"] for offender in offenders],
                         ["copyleft 1.0.0", "dedication 1.0.0"])

    def test_the_licence_of_a_dependency_is_read_as_an_expression(self):
        # The allowlist holds the licences a dependency may use; the
        # Unlicense is no licence of its own here, so it is not in it.
        self.assertNotIn("Unlicense", licensing.ALLOWED_DEPENDENCY_LICENSES)
        self.assertTrue(licensing.expression_is_allowed(
            licensing.parse_license_expression("Unlicense OR MIT")))
        self.assertFalse(licensing.expression_is_allowed(
            licensing.parse_license_expression("Unlicense")))


class RepositoryLicensingTests(unittest.TestCase):
    """The licences of the MiniPython checkout itself."""

    REPO = Path(__file__).resolve().parents[2]

    def setUp(self) -> None:
        if not (self.REPO / "NOTICE").is_file():
            self.skipTest("the NOTICE of the repository is not here")

    def header_origins(self, name: str) -> list[str]:
        """The CPython files the header of a file names."""
        text = (self.REPO / name).read_text(encoding="utf-8")
        origins: list[str] = []
        for line in provenance_names.leading_comment_lines(text, licensing.language_of(name))[:10]:
            origins += licensing.origin_paths(line)
        return origins

    def test_the_notice_and_the_headers_name_the_same_origin(self):
        # The NOTICE names the origin of every file it lists, and it
        # names the origin the header of the file names.
        origins = licensing.notice_origins(self.REPO)
        self.assertTrue(origins, "the NOTICE names the origin of no file")
        for name, listed in origins.items():
            with self.subTest(name=name):
                self.assertEqual(sorted(listed), sorted(self.header_origins(name)))

    def test_the_cpython_files_the_repository_names_exist(self):
        # Every CPython file a derived file names has to exist in the
        # CPython source tree: "Python/complexobject.c" names nothing,
        # the file is "Objects/complexobject.c".
        source = os.environ.get("MINIPYTHON_CPYTHON_SRC")
        if not source or not Path(source).is_dir():
            self.skipTest("MINIPYTHON_CPYTHON_SRC is not set")
        cpython = Path(source)
        for name, listed in licensing.notice_origins(self.REPO).items():
            for origin in listed:
                with self.subTest(name=name, origin=origin):
                    self.assertTrue((cpython / origin).is_file(),
                                    f"{name} names {origin}, which is no file of the CPython tree")


class ProvenanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="minipython-provenance-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.repo = Tree(self.tmp / "repo")
        self.repo.root.mkdir()
        self.cpython = Tree(self.tmp / "cpython")
        self.cpython.root.mkdir()

    def cpython_tree(self, files: dict[str, str]) -> Tree:
        for name, text in files.items():
            self.cpython.write(name, text)
        return self.cpython

    def scan(self, rust_text: str, cpython_files: dict[str, str]) -> list[dict]:
        tree = self.cpython_tree(cpython_files)
        self.repo.write("src/ported.rs", rust_text)
        index, _ = provenance_names.build_index(tree.root)
        public, modules = provenance_names.public_vocabulary(tree.root)
        return provenance_names.scan_files([self.repo.root / "src/ported.rs"], index, public, modules)

    def test_four_shared_internal_names_are_reported(self):
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["reference"], "Lib/thing.py")
        self.assertEqual(findings[0]["count"], 4)
        self.assertEqual(len(findings[0]["names"]), 4)

    def test_three_shared_internal_names_are_not_reported(self):
        names = PRIVATE_NAMES[:3]
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in names)}
        self.assertEqual(self.scan("\n".join(f"fn {name}() {{}}" for name in names), cpython), [])

    def test_the_report_line_names_the_file_and_the_reference(self):
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        line = provenance_names.format_finding(findings[0])
        self.assertIn(":1: provenance: 4 internal names shared with Lib/thing.py: ", line)

    def test_the_public_vocabulary_is_not_a_fingerprint(self):
        cpython = {
            "Lib/token.py": "\n".join(PRIVATE_NAMES),
            "Lib/thing.py": "\n".join(PRIVATE_NAMES),
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_the_documentation_names_are_public(self):
        cpython = {
            "Doc/library.rst": ".. function:: " + "()\n\n``" + PRIVATE_NAMES[0] + "``\n",
            "Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_a_quoted_name_in_a_doc_span_is_public(self):
        # The quotes of a documentation span are quotes of the prose:
        # the name behind them is public vocabulary, not an internal
        # name.
        cpython = {
            "Doc/library.rst": '``"' + PRIVATE_NAMES[0] + '"``\n',
            "Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_a_quoted_name_in_a_doc_role_is_public(self):
        # The quotes of a role are quotes of the prose as well: the name
        # behind them is public vocabulary.
        cpython = {
            "Doc/library.rst": ':func:`"' + PRIVATE_NAMES[0] + '"`\n',
            "Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_an_unmatched_quote_in_a_comment_does_not_hide_the_code(self):
        # The quote of a comment is not the start of a string: the code
        # behind the comment stays code and is scanned.
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        text = '// the " sign of a comment\n' + "\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES)
        findings = self.scan(text, cpython)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["reference"], "Lib/thing.py")
        self.assertEqual(findings[0]["count"], 4)

    def test_the_names_in_a_comment_are_not_names(self):
        # A name that only occurs in a comment is text, not an internal
        # name of the file.
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        text = "\n".join(f"// fn {name}() {{}}" for name in PRIVATE_NAMES)
        self.assertEqual(self.scan(text, cpython), [])

    def test_the_names_in_a_block_comment_are_not_names(self):
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        text = "/*\n" + "\n".join(f" * fn {name}() {{}}" for name in PRIVATE_NAMES) + "\n */\n"
        self.assertEqual(self.scan(text, cpython), [])

    def test_the_index_covers_every_source_file_type(self):
        # A name that a source file of an uncommon type uses is a name
        # of CPython as well (Objects/typeslots.inc is such a file).
        cpython = {"Objects/typeslots.inc": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["reference"], "Objects/typeslots.inc")
        self.assertEqual(findings[0]["count"], 4)

    def test_a_name_used_by_many_source_files_is_vocabulary(self):
        # The file count includes the source files of every type: a name
        # of four files is vocabulary, not a fingerprint.
        cpython = {
            "Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
            "Objects/thing.c": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
            "Python/thing.h": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
            "Objects/typeslots.inc": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
        }
        self.assertEqual(self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython), [])

    def test_the_parameters_of_a_directive_signature_are_public(self):
        # A signature counts with its parameters: they are public
        # vocabulary, not only the name of the signature.
        cpython = {
            "Doc/library.rst": ".. function:: parse(" + ", ".join(PRIVATE_NAMES) + ")\n",
            "Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_the_prose_behind_a_directive_signature_is_not_a_name(self):
        # Only the name and the parameters of a signature are names: the
        # prose of a directive (".. note:: this is a note") is prose.
        cpython = {
            "Doc/library.rst": ".. note:: the mnp_private_alpha of a note\n",
            "Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["reference"], "Lib/thing.py")

    def test_a_name_used_by_many_files_is_vocabulary(self):
        cpython = {f"Lib/mod{number}.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)
                   for number in range(4)}
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_the_tests_of_cpython_are_not_indexed(self):
        cpython = {"Lib/test/test_thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_a_name_in_a_string_literal_is_not_a_name(self):
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        text = 'let text = "' + " ".join(PRIVATE_NAMES) + '";\n'
        self.assertEqual(self.scan(text, cpython), [])

    def test_a_raw_string_literal_is_removed_as_well(self):
        self.assertEqual(provenance_names.code_identifiers('let a = r#"mnp_private_alpha"#;'), set())

    def test_only_code_style_names_count(self):
        text = "fn foo_bar() { let Baz = 1; let qux = 2; let lower = 3; }"
        self.assertEqual(provenance_names.code_identifiers(text), {"foobar", "baz"})

    def test_the_names_are_normalised(self):
        self.assertEqual(provenance_names.normalise("PyUnicode_Type"), "pyunicodetype")
        self.assertEqual(provenance_names.normalise("py_unicode_type"), "pyunicodetype")

    def test_hex_like_names_are_ignored(self):
        text = "let dead_beef = 1; let x06_f0 = 2; let real_name = 3;"
        self.assertEqual(provenance_names.code_identifiers(text), {"realname"})

    def test_a_rust_file_is_scanned_with_the_rust_rules(self):
        # Rust's lifetimes ('a) are not strings: with the rules of another
        # language the names behind them would be lost.
        self.assertEqual(provenance_names.language_of(Path("src/main.rs")), "rust")
        self.assertEqual(provenance_names.language_of(Path("Lib/thing.py")), "python")
        self.assertEqual(provenance_names.language_of(Path("Objects/thing.c")), "c")

    def test_rust_lifetimes_are_not_string_literals(self):
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        text = ("fn parse<'ast, " + ", ".join(PRIVATE_NAMES) + ">(node: &'ast Node) {}\n")
        findings = self.scan(text, cpython)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["reference"], "Lib/thing.py")
        self.assertEqual(findings[0]["count"], 4)

    def test_the_cpython_index_holds_every_identifier(self):
        # CPython writes "privatealpha" where we write "private_alpha":
        # the code-style filter is our rule, not CPython's.
        names = ("private_alpha", "private_bravo", "private_charlie", "private_delta")
        cpython = {"Lib/thing.py": "\n".join(f"def {name.replace('_', '')}():\n    pass" for name in names)}
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in names), cpython)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["count"], 4)

    def test_the_keywords_of_keyword_py_are_public(self):
        # The keywords of Lib/keyword.py are string literals.
        cpython = {
            "Lib/keyword.py": "kwlist = ['" + "', '".join(PRIVATE_NAMES) + "']",
            "Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_the_encoding_aliases_are_public(self):
        # The aliases of Lib/encodings/aliases.py are string literals.
        cpython = {
            "Lib/encodings/aliases.py": "aliases = {'" + "': 'x', '".join(PRIVATE_NAMES) + "': 'x'}",
            "Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES),
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES), cpython)
        self.assertEqual(findings, [])

    def test_a_rust_string_may_span_lines(self):
        # The names in a string that spans lines are text, not code.
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        text = 'let text = "the first line\n' + " ".join(PRIVATE_NAMES) + '\nthe last line";\n'
        self.assertEqual(self.scan(text, cpython), [])
        self.assertNotIn("mnpprivatealpha",
                         provenance_names.code_identifiers('let text = "a\nmnp_private_alpha\n";'))

    def test_the_comment_text_of_every_line_is_known(self):
        text = "// one\nfn main() {}\n/* two\n   three */\n"
        lines = provenance_names.comment_lines(text)
        self.assertEqual(lines, ["// one", "", "/* two", "   three */"])

    def test_the_leading_comment_of_every_line_is_known(self):
        # A comment that follows code does not start its line.
        text = "// one\nfn main() {} // two\n// three\n"
        lines = provenance_names.leading_comment_lines(text)
        self.assertEqual(lines, ["// one", "", "// three"])

    def test_an_empty_string_hides_the_comment_behind_it(self):
        # The delimiters of a string are content: the comment behind an
        # empty string does not start its line.
        self.assertEqual(provenance_names.leading_comment_lines('"" # a comment\n', "python"), [""])
        self.assertEqual(provenance_names.leading_comment_lines('"" // a comment\n'), [""])
        self.assertEqual(provenance_names.leading_comment_lines('# a comment\n', "python"),
                         ["# a comment"])
        self.assertEqual(provenance_names.leading_comment_lines('// a comment\n'), ["// a comment"])

    def test_a_byte_string_is_not_a_raw_string(self):
        # A byte string (b"...") is an ordinary string: the escape of a
        # quote ends it, and the code behind it stays code.
        cpython = {"Lib/thing.py": "\n".join(f"def {name}():\n    pass" for name in PRIVATE_NAMES)}
        text = ('let bytes = b"a \\" b";\n'
                + "\n".join(f"fn {name}() {{}}" for name in PRIVATE_NAMES))
        findings = self.scan(text, cpython)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["reference"], "Lib/thing.py")
        self.assertEqual(findings[0]["count"], 4)

    def test_the_raw_string_prefixes_are_raw_strings(self):
        for literal in ('r"mnp_private_alpha"', 'r#"mnp_private_alpha"#',
                        'br"mnp_private_alpha"', 'rb"mnp_private_alpha"'):
            with self.subTest(literal=literal):
                self.assertEqual(provenance_names.code_identifiers(f"let text = {literal};"), set())

    def test_the_native_modules_of_cpython_are_module_names(self):
        # The name of a native module is the name of its initialisation
        # function, of its definition or the name it gives itself: the
        # name of its file names nothing.
        tree = self.cpython_tree({
            "Modules/gcmodule.c": "PyInit_gc(void) { return 0; }\n",
            "Modules/posixmodule.c": "static struct PyModuleDef posixmodule = {\n"
                                     "    PyModuleDef_HEAD_INIT,\n"
                                     "    .m_name = \"posix\",\n};\n"
                                     "PyInit_posix(void) { return 0; }\n",
            "Modules/_ctypes/_ctypes.c": "PyInit__ctypes(void) { return 0; }\n",
            "Modules/_io/_iomodule.c": "static struct PyModuleDef _iomodule = {\n"
                                        "    PyModuleDef_HEAD_INIT,\n"
                                        "    .m_name = \"_io\",\n};\n"
                                        "PyInit__io(void) { return 0; }\n",
        })
        names = provenance_names.module_names(tree.root)
        self.assertIn("gc", names)
        self.assertIn("posix", names)
        # The names are normalised: the module "_ctypes" is "ctypes".
        self.assertIn("ctypes", names)
        self.assertIn("io", names)

    def test_a_module_definition_names_the_module(self):
        # The definition of a module names it: "gcmodule" is the module
        # "gc" and "sslmodule_def" is the module "ssl".
        tree = self.cpython_tree({
            "Modules/gcmodule.c": "static struct PyModuleDef gcmodule = {\n"
                                   "    PyModuleDef_HEAD_INIT,\n};\n",
            "Modules/_ssl.c": "static struct PyModuleDef _sslmodule_def = {\n"
                              "    PyModuleDef_HEAD_INIT,\n};\n",
        })
        names = provenance_names.module_names(tree.root)
        self.assertIn("gc", names)
        self.assertIn("ssl", names)

    def test_a_definition_without_a_name_names_no_module(self):
        # CPython writes the definition of a module without a name of
        # its own as "moduledef": such a definition names no module.
        tree = self.cpython_tree({
            "Modules/_interpretersmodule.c": "static struct PyModuleDef moduledef = {\n"
                                             "    PyModuleDef_HEAD_INIT,\n};\n",
        })
        names = provenance_names.module_names(tree.root)
        self.assertNotIn("module", names)
        self.assertNotIn("def", names)

    def test_the_name_of_a_helper_file_is_not_a_module_name(self):
        # "Modules/pythonrun.c" is a helper of the interpreter and
        # "Modules/getpath.c" a helper of the start-up: neither of them
        # is a module, so their names name no module.
        tree = self.cpython_tree({
            "Modules/pythonrun.c": "int pythonrun_config = 0;\n",
            "Modules/getpath.c": "int getpath_config = 0;\n",
        })
        names = provenance_names.module_names(tree.root)
        self.assertNotIn("pythonrun", names)
        self.assertNotIn("getpath", names)

    def test_a_helper_file_name_does_not_hide_a_private_name(self):
        # The name of a helper file is not a module name: the names it
        # holds are internal names and not public vocabulary.
        names = ("pythonrun_config", "pythonrun_option", "pythonrun_value", "pythonrun_entry")
        cpython = {
            "Modules/pythonrun.c": "\n".join(f"static int {name};" for name in names),
            "Doc/library.rst": "``config``\n``option``\n``value``\n``entry``\n",
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in names), cpython)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["reference"], "Modules/pythonrun.c")
        self.assertEqual(findings[0]["count"], 4)

    def test_a_public_name_made_of_a_native_module_and_a_name_is_public(self):
        # "posix" is a native module of CPython: "posix" + "path" is
        # public vocabulary, not an internal name.
        names = ("posix_path", "posix_join", "posix_open", "posix_close")
        cpython = {
            "Modules/posixmodule.c": "static struct PyModuleDef posixmodule = {\n"
                                     "    PyModuleDef_HEAD_INIT,\n"
                                     "    .m_name = \"posix\",\n};\n"
                                     + "\n".join(f"static int {name};" for name in names),
            "Doc/library.rst": "``path.join``\n``open``\n``close``\n",
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in names), cpython)
        self.assertEqual(findings, [])

    def test_the_nested_modules_of_cpython_are_module_names(self):
        # The modules of a package are modules as much as the packages
        # are: "email.mime" is a module of the standard library.
        tree = self.cpython_tree({
            "Lib/email/__init__.py": "",
            "Lib/email/mime/__init__.py": "",
            "Lib/email/mime/text.py": "",
        })
        names = provenance_names.module_names(tree.root)
        self.assertIn("email", names)
        self.assertIn("mime", names)
        self.assertIn("text", names)

    def test_a_public_name_made_of_a_nested_module_and_a_name_is_public(self):
        # "mime" is the module of a package of the standard library:
        # "mime" + "text" is public vocabulary, not an internal name.
        names = ("mime_text", "mime_encode", "mime_decode", "mime_header")
        cpython = {
            "Lib/email/mime/text.py": "\n".join(f"static int {name};" for name in names),
            "Doc/library.rst": "``text``\n``encode``\n``decode``\n``header``\n",
        }
        findings = self.scan("\n".join(f"fn {name}() {{}}" for name in names), cpython)
        self.assertEqual(findings, [])

    def test_a_public_name_made_of_a_module_and_a_name_is_public(self):
        public = {"path", "join"}
        modules = {"os"}
        self.assertTrue(provenance_names.is_public("ospath", public, modules))
        self.assertTrue(provenance_names.is_public("osjoin", public, modules))
        self.assertTrue(provenance_names.is_public("pathjoin", public, modules))
        self.assertFalse(provenance_names.is_public("osprivate", public, modules))
        self.assertFalse(provenance_names.is_public("mnpprivatealpha", public, modules))


if __name__ == "__main__":
    unittest.main()
