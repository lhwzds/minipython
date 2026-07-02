#!/usr/bin/env python3
"""Unit tests for the CPython gap sweep driver.

These tests intentionally avoid running MiniPython or downloading an oracle.
They cover the pure driver logic so the broader sweep can stay a discovery
tool instead of becoming the only way to catch tool regressions.
"""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


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
):
    return gap.RunResult(
        exit_code=exit_code,
        stdout=stdout,
        stderr=stderr,
        timeout=timeout,
        exception_class=exception_class,
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


class ReportTests(unittest.TestCase):
    def test_write_reports_emits_json_summary_and_markdown_details(self):
        meta = {
            "generated_at": "2026-07-01T00:00:00+00:00",
            "duration_seconds": 0.01,
            "cwd": str(REPO_ROOT),
            "cpython_executable": "/python",
            "cpython_version": "3.14.6",
            "driver_python": "3.14.6",
            "minipython_executable": "/mnpy",
            "corpus": "tests/gap_corpus",
            "scope": ["syntax"],
        }
        result = gap.SweepResult(
            name="case-one",
            scope="syntax",
            priority="must_fix",
            status="OUTPUT_DIFF",
            expected=None,
            cpython=run_result(stdout="1\n"),
            minipython=run_result(stdout="2\n"),
        )

        with tempfile.TemporaryDirectory() as tmp:
            prefix = Path(tmp) / "report"
            gap.write_reports(prefix, meta, [result])
            payload = json.loads(prefix.with_suffix(".json").read_text())
            markdown = prefix.with_suffix(".md").read_text()

        self.assertEqual(payload["summary"], {"OUTPUT_DIFF": 1})
        self.assertEqual(payload["results"][0]["name"], "case-one")
        self.assertIn("| `OUTPUT_DIFF` | 1 |", markdown)
        self.assertIn("### `case-one`", markdown)
        self.assertIn("CPython stdout:", markdown)
        self.assertIn("MiniPython stdout:", markdown)


if __name__ == "__main__":
    unittest.main(verbosity=2)
