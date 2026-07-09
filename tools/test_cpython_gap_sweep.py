#!/usr/bin/env python3
"""Unit tests for the CPython gap sweep driver.

These tests intentionally avoid running MiniPython or downloading an oracle.
They cover the pure driver logic so the broader sweep can stay a discovery
tool instead of becoming the only way to catch tool regressions.
"""

from __future__ import annotations

import argparse
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


class ParseArgsTests(unittest.TestCase):
    def test_default_cpython_oracle_is_homebrew_python(self):
        with patch.object(sys, "argv", ["cpython_gap_sweep.py", "--require-version", "3.14.6"]):
            args = gap.parse_args()

        self.assertEqual(args.cpython, "/opt/homebrew/bin/python3")
        self.assertEqual(args.require_version, "3.14.6")


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
        self.assertEqual(cases[0]["priority"], "unspecified")
        self.assertTrue(cases[0]["_path"].endswith("cases.toml"))

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
        }
        result = gap.SweepResult(
            name="case-one",
            scope="syntax",
            category="syntax",
            priority="must_fix",
            status="OUTPUT_DIFF",
            expected=None,
            diff="stdout differs",
            cpython=run_result(stdout="1\n"),
            minipython=run_result(stdout="2\n"),
        )

        with tempfile.TemporaryDirectory() as tmp:
            prefix = Path(tmp) / "report"
            gap.write_reports(prefix, meta, [result])
            payload = json.loads(prefix.with_suffix(".json").read_text())
            markdown = prefix.with_suffix(".md").read_text()

        self.assertEqual(payload["summary"], {"OUTPUT_DIFF": 1})
        self.assertEqual(payload["categories"], {"syntax": 1})
        self.assertEqual(payload["meta"]["required_cpython_version"], "3.14.6")
        self.assertEqual(payload["results"][0]["name"], "case-one")
        self.assertEqual(payload["results"][0]["category"], "syntax")
        self.assertEqual(payload["results"][0]["diff"], "stdout differs")
        self.assertIn("- Required CPython: `3.14.6`", markdown)
        self.assertIn("- Driver Python: `3.14.6` at `/python`", markdown)
        self.assertIn("- Categories: `syntax`", markdown)
        self.assertIn("| `OUTPUT_DIFF` | 1 |", markdown)
        self.assertIn("| `syntax` | 1 |", markdown)
        self.assertIn("| `case-one` | `syntax` | `syntax` | `must_fix` | `OUTPUT_DIFF` |", markdown)
        self.assertIn("- Diff: `stdout differs`", markdown)
        self.assertIn("CPython stdout:", markdown)
        self.assertIn("MiniPython stdout:", markdown)


if __name__ == "__main__":
    unittest.main(verbosity=2)
