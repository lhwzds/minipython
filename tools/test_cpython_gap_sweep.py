#!/usr/bin/env python3
"""Unit tests for the CPython gap sweep driver.

These tests intentionally avoid running MiniPython or downloading an oracle.
They cover the pure driver logic so the broader sweep can stay a discovery
tool instead of becoming the only way to catch tool regressions.
"""

from __future__ import annotations

import argparse
import io
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[1]
SWEEP_PATH = REPO_ROOT / "tools" / "cpython_gap_sweep.py"


def load_sweep_module():
    spec = importlib.util.spec_from_file_location("cpython_gap_sweep", SWEEP_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"failed to load {SWEEP_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


gap = load_sweep_module()


REQUIRED_STDLIB_MODULES = {
    "array",
    "builtins",
    "collections",
    "collections.abc",
    "copy",
    "functools",
    "io.BytesIO",
    "itertools",
    "json",
    "math",
    "math.integer",
    "operator",
    "sys",
    "types",
}

REQUIRED_CATEGORIES = {
    "syntax",
    "runtime-semantic",
    "exception-shape",
    "stdlib-missing",
    "sandbox-excluded",
    "cpython-internal",
}

REQUIRED_EXPECTED_MARKERS = {
    "intentional_sandbox_block",
    "unsupported_out_of_scope",
    "stdlib_missing",
    "cpython_missing_compat",
    "cpython_internal",
}

REQUIRED_JSON_ROOT_CAUSES = {
    "json-loads-core",
    "json-loads-number-hooks",
    "json-loads-object-hooks",
    "json-loads-string-escapes",
    "json-loads-top-level-scalars",
    "json-dumps-format-options",
    "json-dumps-default-skipkeys",
    "json-dumps-nonfinite-and-circular",
}


def run_result(
    *,
    exit_code=0,
    stdout="",
    stderr="",
    timeout=False,
    exception_class=None,
    exception_message=None,
):
    return gap.RunResult(
        exit_code=exit_code,
        stdout=stdout,
        stderr=stderr,
        timeout=timeout,
        exception_class=exception_class,
        exception_message=exception_message,
    )


class ClassifyTests(unittest.TestCase):
    def test_match_requires_equal_exit_stdout_and_stderr(self):
        left = run_result(stdout="ok\n")
        right = run_result(stdout="ok\n")
        self.assertEqual(gap.classify(left, right, None), "MATCH")

    def test_output_diff_for_successful_stdout_mismatch(self):
        cpython = run_result(stdout="cpython\n")
        mini = run_result(stdout="mini\n")
        self.assertEqual(gap.classify(cpython, mini, None), "OUTPUT_DIFF")

    def test_rejection_direction_is_visible(self):
        cpython = run_result(stdout="ok\n")
        mini = run_result(exit_code=1, stderr="runtime error: NameError: x\n")
        self.assertEqual(gap.classify(cpython, mini, None), "MINIPYTHON_REJECTS")

        cpython = run_result(exit_code=1, stderr="SyntaxError: bad\n")
        mini = run_result(stdout="ok\n")
        self.assertEqual(gap.classify(cpython, mini, None), "CPYTHON_REJECTS")

    def test_intentional_sandbox_expected_overrides_nonmatching_results(self):
        cpython = run_result(exit_code=1, stderr="ModuleNotFoundError: socket\n")
        mini = run_result(stdout="blocked\n")
        self.assertEqual(
            gap.classify(cpython, mini, "intentional_sandbox_block"),
            "INTENTIONAL_SANDBOX_BLOCK",
        )

    def test_unsupported_out_of_scope_expected_overrides_nonmatching_results(self):
        cpython = run_result(stdout="imported\n")
        mini = run_result(exit_code=1, stderr="ModuleNotFoundError: subprocess\n")
        self.assertEqual(
            gap.classify(cpython, mini, "unsupported_out_of_scope"),
            "UNSUPPORTED_OUT_OF_SCOPE",
        )

    def test_stdlib_missing_expected_overrides_nonmatching_results(self):
        cpython = run_result(stdout="imported\n")
        mini = run_result(exit_code=1, stderr="ModuleNotFoundError: pdb\n")
        self.assertEqual(gap.classify(cpython, mini, "stdlib_missing"), "STDLIB_MISSING")

    def test_cpython_missing_compat_expected_overrides_nonmatching_results(self):
        cpython = run_result(
            exit_code=1,
            stderr="ModuleNotFoundError: No module named 'math.integer'\n",
        )
        mini = run_result(stdout="6\n")
        self.assertEqual(
            gap.classify(cpython, mini, "cpython_missing_compat"),
            "CPYTHON_MISSING_COMPAT",
        )

    def test_cpython_internal_expected_overrides_nonmatching_results(self):
        cpython = run_result(stdout="imported\n")
        mini = run_result(exit_code=1, stderr="ModuleNotFoundError: _testcapi\n")
        self.assertEqual(
            gap.classify(cpython, mini, "cpython_internal"),
            "CPYTHON_INTERNAL",
        )

    def test_exception_class_and_message_diffs_are_separate(self):
        cpython = run_result(
            exit_code=1,
            stderr="ValueError: bad\n",
            exception_class="ValueError",
        )
        mini = run_result(
            exit_code=1,
            stderr="TypeError: bad\n",
            exception_class="TypeError",
        )
        self.assertEqual(gap.classify(cpython, mini, None), "EXCEPTION_CLASS_DIFF")

        mini = run_result(
            exit_code=1,
            stderr="ValueError: different\n",
            exception_class="ValueError",
        )
        self.assertEqual(gap.classify(cpython, mini, None), "EXCEPTION_MESSAGE_DIFF")

    def test_timeout_and_process_crash_are_distinct(self):
        cpython = run_result()
        mini = run_result(exit_code=None, timeout=True)
        self.assertEqual(gap.classify(cpython, mini, None), "TIMEOUT")

        mini = run_result(exit_code=None, stderr="exec failed")
        self.assertEqual(gap.classify(cpython, mini, None), "CRASH")

    def test_exception_extraction_accepts_cpython_and_minipython_shapes(self):
        self.assertEqual(
            gap.extract_exception_class(
                "Traceback (most recent call last):\nValueError: bad\n"
            ),
            "ValueError",
        )
        self.assertEqual(
            gap.extract_exception_class("runtime error: TypeError: bad\n"),
            "TypeError",
        )

    def test_normalized_message_strips_minipython_runtime_prefix(self):
        mini = run_result(
            exit_code=1,
            stderr="runtime error: ValueError: bad\n",
            exception_class="ValueError",
        )
        self.assertEqual(gap.normalized_message(mini), "ValueError: bad")

    def test_run_result_records_exception_message_and_normalized_diff(self):
        cpython = run_result(
            exit_code=1,
            stderr="ValueError: bad\n",
            exception_class="ValueError",
            exception_message="ValueError: bad",
        )
        mini = run_result(
            exit_code=1,
            stderr="runtime error: TypeError: bad\n",
            exception_class="TypeError",
            exception_message="TypeError: bad",
        )

        diff = gap.normalized_diff(cpython, mini)

        self.assertIn("exception_class 'ValueError' != 'TypeError'", diff)
        self.assertIn("exception_message 'ValueError: bad' != 'TypeError: bad'", diff)


class TriageTests(unittest.TestCase):
    def test_triage_status_marks_passing_accepted_and_open_diffs(self):
        self.assertEqual(gap.triage_status("MATCH"), "passing")
        self.assertEqual(gap.triage_status("STDLIB_MISSING"), "accepted_gap")
        self.assertEqual(gap.triage_status("CPYTHON_INTERNAL"), "accepted_gap")
        self.assertEqual(gap.triage_status("OUTPUT_DIFF"), "needs_triage")
        self.assertEqual(gap.triage_status("TIMEOUT"), "needs_triage")


class ParseArgsTests(unittest.TestCase):
    def test_default_cpython_oracle_is_homebrew_python(self):
        with patch.object(sys, "argv", ["cpython_gap_sweep.py", "--require-version", "3.14.6"]):
            args = gap.parse_args()

        self.assertEqual(args.cpython, "/opt/homebrew/bin/python3")
        self.assertEqual(args.require_version, "3.14.6")
        self.assertEqual(args.root_cause, "*")
        self.assertFalse(args.fail_on_open)

    def test_fail_on_open_flag_is_available(self):
        with patch.object(
            sys,
            "argv",
            [
                "cpython_gap_sweep.py",
                "--require-version",
                "3.14.6",
                "--fail-on-open",
            ],
        ):
            args = gap.parse_args()

        self.assertTrue(args.fail_on_open)


class CorpusLoadingTests(unittest.TestCase):
    def test_load_cases_from_file_adds_defaults_and_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / "cases.toml"
            corpus.write_text(
                """
[[case]]
name = "basic"
source = "print(1)"
""".lstrip()
            )

            cases = gap.load_cases(corpus)

        self.assertEqual(len(cases), 1)
        self.assertEqual(cases[0]["name"], "basic")
        self.assertEqual(cases[0]["scope"], "unspecified")
        self.assertEqual(cases[0]["category"], "runtime-semantic")
        self.assertEqual(cases[0]["modules"], ["unspecified"])
        self.assertEqual(cases[0]["root_cause"], "runtime-semantic:unspecified")
        self.assertEqual(cases[0]["priority"], "unspecified")
        self.assertTrue(cases[0]["_path"].endswith("cases.toml"))

    def test_load_cases_accepts_module_string_and_modules_list(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / "cases.toml"
            corpus.write_text(
                """
[[case]]
name = "one"
module = "json"
root_cause = "json-loads-core"
source = "print(1)"

[[case]]
name = "two"
modules = ["sys", "builtins", "sys"]
source = "print(2)"
""".lstrip()
            )

            cases = gap.load_cases(corpus)

        self.assertEqual(cases[0]["modules"], ["json"])
        self.assertEqual(cases[0]["root_cause"], "json-loads-core")
        self.assertEqual(cases[1]["modules"], ["builtins", "sys"])

    def test_load_cases_rejects_missing_required_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / "cases.toml"
            corpus.write_text(
                """
[[case]]
name = "missing-source"
""".lstrip()
            )
            with self.assertRaisesRegex(ValueError, "every case needs name and source"):
                gap.load_cases(corpus)

    def test_load_cases_rejects_unknown_expected_marker(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / "cases.toml"
            corpus.write_text(
                """
[[case]]
name = "bad-expected"
source = "print(1)"
expected = "typo"
""".lstrip()
            )
            with self.assertRaisesRegex(ValueError, "unknown expected marker"):
                gap.load_cases(corpus)

    def test_load_cases_rejects_unknown_category(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / "cases.toml"
            corpus.write_text(
                """
[[case]]
name = "bad-category"
category = "typo"
source = "print(1)"
""".lstrip()
            )
            with self.assertRaisesRegex(ValueError, "unknown category"):
                gap.load_cases(corpus)

    def test_load_cases_rejects_invalid_modules(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / "cases.toml"
            corpus.write_text(
                """
[[case]]
name = "bad-modules"
modules = [1]
source = "print(1)"
""".lstrip()
            )
            with self.assertRaisesRegex(ValueError, "modules must be"):
                gap.load_cases(corpus)

    def test_load_cases_rejects_invalid_root_cause(self):
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp) / "cases.toml"
            corpus.write_text(
                """
[[case]]
name = "bad-root-cause"
root_cause = ""
source = "print(1)"
""".lstrip()
            )
            with self.assertRaisesRegex(ValueError, "root_cause must be"):
                gap.load_cases(corpus)


class CorpusContractTests(unittest.TestCase):
    def test_repo_corpus_covers_required_stdlib_modules_and_categories(self):
        cases = gap.load_cases(REPO_ROOT / "tests" / "gap_corpus")
        stdlib_modules = {
            module
            for case in cases
            if case["scope"] == "stdlib-sandbox"
            for module in case["modules"]
        }
        categories = {case["category"] for case in cases}

        self.assertEqual(REQUIRED_STDLIB_MODULES - stdlib_modules, set())
        self.assertEqual(REQUIRED_CATEGORIES - categories, set())

    def test_repo_corpus_keeps_expected_gap_markers_and_json_root_causes(self):
        cases = gap.load_cases(REPO_ROOT / "tests" / "gap_corpus")
        expected_markers = {
            case["expected"]
            for case in cases
            if "expected" in case
        }
        json_root_causes = {
            case["root_cause"]
            for case in cases
            if "json" in case["modules"]
        }
        root_cause_counts = {
            root_cause: sum(1 for case in cases if case["root_cause"] == root_cause)
            for root_cause in json_root_causes
        }

        self.assertEqual(REQUIRED_EXPECTED_MARKERS - expected_markers, set())
        self.assertEqual(REQUIRED_JSON_ROOT_CAUSES - json_root_causes, set())
        self.assertGreaterEqual(root_cause_counts["json-loads-core"], 2)


class VersionGuardTests(unittest.TestCase):
    def test_run_sweep_rejects_wrong_oracle_version(self):
        original_oracle_version = gap.oracle_version
        gap.oracle_version = lambda _cpython: "3.14.5"
        args = argparse.Namespace(
            cpython="/does/not/matter",
            require_version="3.14.6",
            minipython="/does/not/matter",
            corpus="tests/gap_corpus",
            scope="syntax",
            category="syntax",
            module="syntax",
            root_cause="syntax-frontend-functions",
            out="reports/cpython-gap-sweep",
            timeout=0.1,
            fail_on_diff=False,
        )
        try:
            with self.assertRaisesRegex(
                SystemExit,
                "CPython oracle version mismatch: expected 3.14.6, got 3.14.5",
            ):
                gap.run_sweep(args)
        finally:
            gap.oracle_version = original_oracle_version


class RootCauseSummaryTests(unittest.TestCase):
    def test_root_cause_summary_groups_cases_and_metadata(self):
        results = [
            gap.SweepResult(
                name="case-one",
                scope="stdlib-sandbox",
                category="runtime-semantic",
                root_cause="json-loads-core",
                modules=["json"],
                priority="should_fix",
                status="MATCH",
                triage_status="passing",
                expected=None,
                diff="",
                cpython=run_result(stdout="ok\n"),
                minipython=run_result(stdout="ok\n"),
            ),
            gap.SweepResult(
                name="case-two",
                scope="stdlib-sandbox",
                category="runtime-semantic",
                root_cause="json-loads-core",
                modules=["json", "builtins"],
                priority="must_fix",
                status="OUTPUT_DIFF",
                triage_status="needs_triage",
                expected=None,
                diff="stdout differs",
                cpython=run_result(stdout="1\n"),
                minipython=run_result(stdout="2\n"),
            ),
        ]

        summary = gap.summarize_root_causes(results)

        self.assertEqual(summary["json-loads-core"]["count"], 2)
        self.assertEqual(summary["json-loads-core"]["triage"], {"needs_triage": 1, "passing": 1})
        self.assertEqual(summary["json-loads-core"]["statuses"], {"MATCH": 1, "OUTPUT_DIFF": 1})
        self.assertEqual(summary["json-loads-core"]["modules"], ["builtins", "json"])
        self.assertEqual(summary["json-loads-core"]["categories"], ["runtime-semantic"])
        self.assertEqual(summary["json-loads-core"]["priorities"], {"must_fix": 1, "should_fix": 1})
        self.assertEqual(summary["json-loads-core"]["cases"], ["case-one", "case-two"])

    def test_open_root_causes_keeps_only_needs_triage_groups(self):
        results = [
            gap.SweepResult(
                name="passing-case",
                scope="stdlib-sandbox",
                category="runtime-semantic",
                root_cause="json-loads-core",
                modules=["json"],
                priority="should_fix",
                status="MATCH",
                triage_status="passing",
                expected=None,
                diff="",
                cpython=run_result(stdout="ok\n"),
                minipython=run_result(stdout="ok\n"),
            ),
            gap.SweepResult(
                name="accepted-case",
                scope="sandbox",
                category="sandbox-excluded",
                root_cause="sandbox-network-block",
                modules=["socket"],
                priority="wont_fix",
                status="INTENTIONAL_SANDBOX_BLOCK",
                triage_status="accepted_gap",
                expected="intentional_sandbox_block",
                diff="intentional sandbox block",
                cpython=run_result(stdout="ok\n"),
                minipython=run_result(exit_code=1, stderr="blocked\n"),
            ),
            gap.SweepResult(
                name="open-case",
                scope="stdlib-sandbox",
                category="runtime-semantic",
                root_cause="json-dumps-format-options",
                modules=["json"],
                priority="should_fix",
                status="OUTPUT_DIFF",
                triage_status="needs_triage",
                expected=None,
                diff="stdout differs",
                cpython=run_result(stdout="1\n"),
                minipython=run_result(stdout="2\n"),
            ),
        ]

        open_summary = gap.open_root_causes(results)

        self.assertEqual(list(open_summary), ["json-dumps-format-options"])
        self.assertEqual(open_summary["json-dumps-format-options"]["count"], 1)
        self.assertEqual(
            open_summary["json-dumps-format-options"]["triage"],
            {"needs_triage": 1},
        )
        self.assertEqual(
            open_summary["json-dumps-format-options"]["cases"],
            ["open-case"],
        )


class ReportTests(unittest.TestCase):
    def test_write_reports_emits_json_summary_and_markdown_details(self):
        meta = {
            "generated_at": "2026-07-01T00:00:00+00:00",
            "duration_seconds": 0.01,
            "cwd": str(REPO_ROOT),
            "required_cpython_version": "3.14.6",
            "cpython_executable": "/python",
            "cpython_version": "3.14.6",
            "driver_executable": "/python",
            "driver_python": "3.14.6",
            "minipython_executable": "/mnpy",
            "corpus": "tests/gap_corpus",
            "scope": ["syntax"],
            "category": ["syntax"],
            "module": ["json"],
            "root_cause": ["json-loads-core"],
        }
        result = gap.SweepResult(
            name="case-one",
            scope="syntax",
            category="syntax",
            root_cause="json-loads-core",
            modules=["json"],
            priority="must_fix",
            status="OUTPUT_DIFF",
            triage_status="needs_triage",
            expected=None,
            diff="stdout differs",
            cpython=run_result(
                exit_code=1,
                stdout="1\n",
                stderr="ValueError: bad\n",
                exception_class="ValueError",
                exception_message="ValueError: bad",
            ),
            minipython=run_result(
                exit_code=1,
                stdout="2\n",
                stderr="runtime error: TypeError: bad\n",
                exception_class="TypeError",
                exception_message="TypeError: bad",
            ),
        )

        with tempfile.TemporaryDirectory() as tmp:
            prefix = Path(tmp) / "report"
            gap.write_reports(prefix, meta, [result])
            payload = json.loads(prefix.with_suffix(".json").read_text())
            markdown = prefix.with_suffix(".md").read_text()

        self.assertEqual(payload["summary"], {"OUTPUT_DIFF": 1})
        self.assertEqual(payload["triage"], {"needs_triage": 1})
        self.assertEqual(payload["categories"], {"syntax": 1})
        self.assertEqual(payload["root_causes"], {"json-loads-core": 1})
        self.assertEqual(
            payload["root_cause_summary"]["json-loads-core"],
            {
                "count": 1,
                "triage": {"needs_triage": 1},
                "statuses": {"OUTPUT_DIFF": 1},
                "modules": ["json"],
                "categories": ["syntax"],
                "priorities": {"must_fix": 1},
                "cases": ["case-one"],
            },
        )
        self.assertEqual(
            payload["open_root_causes"]["json-loads-core"],
            payload["root_cause_summary"]["json-loads-core"],
        )
        self.assertEqual(payload["modules"], {"json": 1})
        self.assertEqual(payload["meta"]["required_cpython_version"], "3.14.6")
        self.assertEqual(payload["results"][0]["name"], "case-one")
        self.assertEqual(payload["results"][0]["category"], "syntax")
        self.assertEqual(payload["results"][0]["root_cause"], "json-loads-core")
        self.assertEqual(payload["results"][0]["modules"], ["json"])
        self.assertEqual(payload["results"][0]["triage_status"], "needs_triage")
        self.assertEqual(payload["results"][0]["diff"], "stdout differs")
        self.assertEqual(payload["results"][0]["cpython"]["exit_code"], 1)
        self.assertEqual(payload["results"][0]["cpython"]["stdout"], "1\n")
        self.assertEqual(payload["results"][0]["cpython"]["stderr"], "ValueError: bad\n")
        self.assertEqual(payload["results"][0]["cpython"]["timeout"], False)
        self.assertEqual(payload["results"][0]["cpython"]["exception_class"], "ValueError")
        self.assertEqual(
            payload["results"][0]["cpython"]["exception_message"],
            "ValueError: bad",
        )
        self.assertEqual(payload["results"][0]["minipython"]["exit_code"], 1)
        self.assertEqual(payload["results"][0]["minipython"]["stdout"], "2\n")
        self.assertEqual(
            payload["results"][0]["minipython"]["stderr"],
            "runtime error: TypeError: bad\n",
        )
        self.assertEqual(payload["results"][0]["minipython"]["timeout"], False)
        self.assertEqual(
            payload["results"][0]["minipython"]["exception_class"],
            "TypeError",
        )
        self.assertEqual(
            payload["results"][0]["minipython"]["exception_message"],
            "TypeError: bad",
        )
        self.assertIn("- Required CPython: `3.14.6`", markdown)
        self.assertIn("- Driver Python: `3.14.6` at `/python`", markdown)
        self.assertIn("- Categories: `syntax`", markdown)
        self.assertIn("- Modules: `json`", markdown)
        self.assertIn("- Root Causes: `json-loads-core`", markdown)
        self.assertIn("| `OUTPUT_DIFF` | 1 |", markdown)
        self.assertIn("| `needs_triage` | 1 |", markdown)
        self.assertIn("## Open Root Causes", markdown)
        self.assertIn(
            "| `json-loads-core` | 1 | `needs_triage=1` | `OUTPUT_DIFF=1` | `json` |",
            markdown,
        )
        self.assertIn("| `syntax` | 1 |", markdown)
        self.assertIn(
            "| `json-loads-core` | 1 | `needs_triage=1` | `OUTPUT_DIFF=1` | `json` |",
            markdown,
        )
        self.assertIn("| `json` | 1 |", markdown)
        self.assertIn(
            "| `case-one` | `syntax` | `syntax` | `json-loads-core` | `json` | `must_fix` | `OUTPUT_DIFF` | `needs_triage` |",
            markdown,
        )
        self.assertIn("- Root Cause: `json-loads-core`", markdown)
        self.assertIn("- Triage: `needs_triage`", markdown)
        self.assertIn("- Diff: `stdout differs`", markdown)
        self.assertIn("CPython stdout:", markdown)
        self.assertIn("MiniPython stdout:", markdown)


class MainTests(unittest.TestCase):
    def test_fail_on_open_reports_root_cause_queue(self):
        args = argparse.Namespace(
            out="reports/cpython-gap-sweep",
            fail_on_diff=False,
            fail_on_open=True,
        )
        result = gap.SweepResult(
            name="open-case",
            scope="stdlib-sandbox",
            category="runtime-semantic",
            root_cause="json-dumps-format-options",
            modules=["json"],
            priority="should_fix",
            status="OUTPUT_DIFF",
            triage_status="needs_triage",
            expected=None,
            diff="stdout differs",
            cpython=run_result(stdout="1\n"),
            minipython=run_result(stdout="2\n"),
        )

        stdout = io.StringIO()
        stderr = io.StringIO()
        with patch.object(gap, "parse_args", return_value=args), patch.object(
            gap, "run_sweep", return_value=({}, [result])
        ), patch.object(gap, "write_reports") as write_reports, patch(
            "sys.stdout", stdout
        ), patch(
            "sys.stderr", stderr
        ):
            exit_code = gap.main()

        self.assertEqual(exit_code, 1)
        write_reports.assert_called_once()
        self.assertIn("open root causes:", stderr.getvalue())
        self.assertIn("json-dumps-format-options(1)", stderr.getvalue())

    def test_fail_on_open_allows_passing_and_accepted_gaps(self):
        args = argparse.Namespace(
            out="reports/cpython-gap-sweep",
            fail_on_diff=False,
            fail_on_open=True,
        )
        results = [
            gap.SweepResult(
                name="passing-case",
                scope="stdlib-sandbox",
                category="runtime-semantic",
                root_cause="json-loads-core",
                modules=["json"],
                priority="should_fix",
                status="MATCH",
                triage_status="passing",
                expected=None,
                diff="",
                cpython=run_result(stdout="ok\n"),
                minipython=run_result(stdout="ok\n"),
            ),
            gap.SweepResult(
                name="accepted-case",
                scope="intentional-sandbox",
                category="sandbox-excluded",
                root_cause="sandbox-network-block",
                modules=["socket"],
                priority="wont_fix",
                status="INTENTIONAL_SANDBOX_BLOCK",
                triage_status="accepted_gap",
                expected="intentional_sandbox_block",
                diff="intentional sandbox block",
                cpython=run_result(stdout="ok\n"),
                minipython=run_result(exit_code=1, stderr="blocked\n"),
            ),
        ]

        stdout = io.StringIO()
        with patch.object(gap, "parse_args", return_value=args), patch.object(
            gap, "run_sweep", return_value=({}, results)
        ), patch.object(gap, "write_reports"), patch("sys.stdout", stdout):
            exit_code = gap.main()

        self.assertEqual(exit_code, 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
