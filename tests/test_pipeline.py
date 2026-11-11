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
import os
import sys
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[1]
SWEEP_PATH = REPO_ROOT / "tests" / "pipeline.py"


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
    "__future__",
    "argparse",
    "array",
    "builtins",
    "collections",
    "collections.abc",
    "copy",
    "csv",
    "datetime",
    "functools",
    "io.BytesIO",
    "io.StringIO",
    "itertools",
    "json",
    "math",
    "math.integer",
    "operator",
    "re",
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
    "json-loads-error-boundaries",
    "json-dumps-format-options",
    "json-dumps-default-skipkeys",
    "json-dumps-nonfinite-and-circular",
    "json-dumps-error-boundaries",
    "json-file-api-out-of-scope",
    "json-codec-classes-out-of-scope",
}

REQUIRED_SANDBOX_ROOT_CAUSES = {
    "sandbox-cabi-block",
    "sandbox-host-io-block",
    "sandbox-network-block",
    "sandbox-process-block",
    "sandbox-signal-block",
}

REQUIRED_STDLIB_ROOT_CAUSES = {
    "builtins-sys-breakpoint-hook",
    "collections-copy-io-array-core",
    "operator-functools-itertools-core",
    "stdlib-allowlist-runtime-smoke",
    "stdlib-module-metadata-smoke",
}


# Marks a status the test did not choose, so a test can ask for no status
# at all without the helper picking one.
UNSET = object()


def run_result(
    *,
    exit_code=0,
    stdout="",
    stderr="",
    timeout=False,
    exception_class=None,
    exception_message=None,
    status=UNSET,
    terminal=None,
):
    if status is UNSET:
        status = "Success" if exit_code == 0 else "Error"
    return gap.RunResult(
        exit_code=exit_code,
        stdout=stdout,
        stderr=stderr,
        timeout=timeout,
        exception_class=exception_class,
        exception_message=exception_message,
        status=status,
        terminal=terminal,
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
        mini = run_result(
            exit_code=1, stderr="Traceback (most recent call last):\nNameError: x\n"
        )
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

    def test_a_worker_crash_status_is_a_crash_even_with_a_marker(self):
        cpython = run_result()
        crashed = run_result(exit_code=1, status="WorkerCrash")
        for marker in (None, "unsupported_out_of_scope"):
            with self.subTest(marker=marker):
                self.assertEqual(gap.classify(cpython, crashed, marker), "CRASH")

    def test_native_stack_exhaustion_is_a_crash(self):
        cpython = run_result()
        exhausted = run_result(
            exit_code=1, status="Error", terminal="NativeStackExhausted"
        )
        self.assertTrue(gap.worker_crashed(exhausted))
        for marker in (None, "unsupported_out_of_scope"):
            with self.subTest(marker=marker):
                self.assertEqual(gap.classify(cpython, exhausted, marker), "CRASH")

        # The terminal condition decides, whatever status the run reported.
        reported_success = run_result(
            exit_code=1, status="Success", terminal="NativeStackExhausted"
        )
        for marker in (None, "unsupported_out_of_scope"):
            with self.subTest(marker=marker):
                self.assertEqual(
                    gap.classify(cpython, reported_success, marker), "CRASH"
                )

        limited = run_result(
            exit_code=1, status="Error", terminal="InstructionLimit"
        )
        self.assertFalse(gap.worker_crashed(limited))
        for marker in (None, "unsupported_out_of_scope"):
            with self.subTest(marker=marker):
                self.assertNotEqual(gap.classify(cpython, limited, marker), "CRASH")

    def test_a_run_without_a_status_is_a_crash(self):
        cpython = run_result()
        crashed = run_result(exit_code=101, status=None)
        for marker in (None, "unsupported_out_of_scope"):
            with self.subTest(marker=marker):
                self.assertEqual(gap.classify(cpython, crashed, marker), "CRASH")

    def test_a_run_that_died_from_a_signal_is_a_crash(self):
        cpython = run_result()
        crashed = run_result(exit_code=-11, status=None)
        for marker in (None, "unsupported_out_of_scope"):
            with self.subTest(marker=marker):
                self.assertEqual(gap.classify(cpython, crashed, marker), "CRASH")

    def test_program_output_cannot_forge_a_crash(self):
        cpython = run_result()
        finished = run_result(
            exit_code=1,
            stderr="sandbox error: worker crashed (signal 11)\n",
            status="Error",
        )
        for marker in (None, "unsupported_out_of_scope"):
            with self.subTest(marker=marker):
                self.assertNotEqual(gap.classify(cpython, finished, marker), "CRASH")

    def test_a_sandbox_limit_is_not_a_crash(self):
        cpython = run_result()
        limited = run_result(
            exit_code=1,
            stderr="sandbox error: instruction limit exceeded\n",
            status="Error",
        )
        for marker in (None, "unsupported_out_of_scope"):
            with self.subTest(marker=marker):
                self.assertNotEqual(gap.classify(cpython, limited, marker), "CRASH")

    def test_the_memory_and_time_limits_are_not_crashes(self):
        cpython = run_result()
        for status in ("MemoryLimit", "TimeLimit"):
            limited = run_result(exit_code=1, status=status)
            for marker in (None, "unsupported_out_of_scope"):
                with self.subTest(status=status, marker=marker):
                    self.assertNotEqual(
                        gap.classify(cpython, limited, marker), "CRASH"
                    )

    def test_a_result_identical_to_cpython_matches_without_a_status(self):
        cpython = run_result(stdout="ok\n")
        mini = run_result(stdout="ok\n", status=None)
        self.assertEqual(gap.classify(cpython, mini, None), "MATCH")

    def test_matching_result_breaks_an_expected_difference_contract(self):
        cpython = run_result(stdout="imported\n")
        bypass = run_result(stdout="imported\n")
        for marker in gap.EXPECTED_STATUS_BY_MARKER:
            status, diff = gap.classify_case(cpython, bypass, {"expected": marker})
            self.assertEqual(status, "EXPECTED_CONTRACT_DIFF")
            self.assertIn(marker, diff)
        self.assertEqual(gap.classify_case(cpython, bypass, {}), ("MATCH", ""))

    def test_exception_extraction_reads_the_final_traceback_line(self):
        cpython = (
            "Traceback (most recent call last):\n"
            '  File "<stdin>", line 2, in <module>\n'
            "    json.loads('{')\n"
            "json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)\n"
        )
        mini = (
            "Traceback (most recent call last):\n"
            "json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)\n"
        )
        self.assertEqual(
            gap.extract_exception_class(cpython), "json.decoder.JSONDecodeError"
        )
        self.assertEqual(
            gap.extract_exception_class(mini), "json.decoder.JSONDecodeError"
        )
        self.assertEqual(
            gap.extract_exception_message(mini, "json.decoder.JSONDecodeError"),
            "json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)",
        )

    def test_normalized_message_ignores_traceback_frames(self):
        cpython = run_result(
            exit_code=1,
            stderr=(
                "Traceback (most recent call last):\n"
                '  File "<stdin>", line 3, in <module>\n'
                "    raise MyErr()\n"
                "MyErr\n"
            ),
        )
        mini = run_result(
            exit_code=1, stderr="Traceback (most recent call last):\nMyErr\n"
        )
        self.assertEqual(gap.normalized_message(cpython), "MyErr")
        self.assertEqual(gap.normalized_message(mini), "MyErr")
        self.assertEqual(gap.classify(cpython, mini, None), "OUTPUT_DIFF")
        self.assertNotIn("exception_message", gap.normalized_diff(cpython, mini))

    def test_expected_security_contract_rejects_a_matching_bypass(self):
        case = {
            "expected": "intentional_sandbox_block",
            "expected_cpython_stdout": "imported\n",
            "expected_minipython_stdout": "ModuleNotFoundError\n",
        }
        cpython = run_result(stdout="imported\n")
        blocked = run_result(stdout="ModuleNotFoundError\n")
        bypass = run_result(stdout="imported\n")

        self.assertEqual(
            gap.classify_case(cpython, blocked, case),
            ("INTENTIONAL_SANDBOX_BLOCK", "stdout differs"),
        )
        status, diff = gap.classify_case(cpython, bypass, case)
        self.assertEqual(status, "EXPECTED_CONTRACT_DIFF")
        self.assertIn("expected_minipython_stdout", diff)

    def test_run_result_records_exception_message_and_normalized_diff(self):
        cpython = run_result(
            exit_code=1,
            stderr="ValueError: bad\n",
            exception_class="ValueError",
            exception_message="ValueError: bad",
        )
        mini = run_result(
            exit_code=1,
            stderr="Traceback (most recent call last):\nTypeError: bad\n",
            exception_class="TypeError",
            exception_message="TypeError: bad",
        )

        diff = gap.normalized_diff(cpython, mini)

        self.assertIn("exception_class 'ValueError' != 'TypeError'", diff)
        self.assertIn("exception_message 'ValueError: bad' != 'TypeError: bad'", diff)


class StatusFileTests(unittest.TestCase):
    def test_read_status_reads_the_status_mnpy_wrote(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "status.json"
            path.write_text('{"status":"Error","terminal":"InstructionLimit"}\n')

            self.assertEqual(gap.read_status_file(path)[0], "Error")

    def test_read_status_reads_the_terminal_condition_mnpy_wrote(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "status.json"
            path.write_text(
                '{"status":"Error","terminal":"NativeStackExhausted"}\n'
            )

            self.assertEqual(
                gap.read_status_file(path),
                ("Error", "NativeStackExhausted"),
            )
            self.assertEqual(gap.read_status_file(path)[0], "Error")
            self.assertEqual(gap.read_status_file(path)[1], "NativeStackExhausted")

    def test_read_status_is_none_without_a_string_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            missing = Path(tmp) / "missing.json"
            broken = Path(tmp) / "broken.json"
            broken.write_text("not json\n")
            without_status = Path(tmp) / "without.json"
            without_status.write_text('{"terminal":null}\n')
            wrong_type = Path(tmp) / "wrong.json"
            wrong_type.write_text('{"status":7}\n')

            self.assertIsNone(gap.read_status_file(missing)[0])
            self.assertIsNone(gap.read_status_file(broken)[0])
            self.assertIsNone(gap.read_status_file(without_status)[0])
            self.assertIsNone(gap.read_status_file(wrong_type)[0])
            self.assertIsNone(gap.read_status_file(None)[0])

    def test_read_terminal_is_none_without_a_string_terminal(self):
        with tempfile.TemporaryDirectory() as tmp:
            missing = Path(tmp) / "missing.json"
            broken = Path(tmp) / "broken.json"
            broken.write_text("not json\n")
            without_terminal = Path(tmp) / "without.json"
            without_terminal.write_text('{"status":"Error"}\n')
            wrong_type = Path(tmp) / "wrong.json"
            wrong_type.write_text('{"terminal":7}\n')

            self.assertIsNone(gap.read_status_file(missing)[1])
            self.assertIsNone(gap.read_status_file(broken)[1])
            self.assertIsNone(gap.read_status_file(without_terminal)[1])
            self.assertIsNone(gap.read_status_file(wrong_type)[1])
            self.assertIsNone(gap.read_status_file(None)[1])
            self.assertEqual(gap.read_status_file(None), (None, None))

    def test_run_command_asks_for_a_status_file_and_reads_it_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            script = Path(tmp) / "fake_mnpy.sh"
            script.write_text(
                "#!/bin/sh\n"
                "path=\"\"\n"
                "while [ $# -gt 0 ]; do\n"
                "  if [ \"$1\" = \"--status-file\" ]; then path=\"$2\"; fi\n"
                "  shift\n"
                "done\n"
                "printf '{\"status\": \"WorkerCrash\", \"terminal\": null}\\n' > \"$path\"\n"
            )
            script.chmod(0o755)
            status = Path(tmp) / "status.json"

            result = gap.run_command(
                [str(script)], "print('hi')\n", 30.0, status
            )

            self.assertEqual(result.exit_code, 0)
            self.assertEqual(result.status, "WorkerCrash")
            self.assertIsNone(result.terminal)

    def test_run_command_reads_the_terminal_condition_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            script = Path(tmp) / "fake_mnpy.sh"
            script.write_text(
                "#!/bin/sh\n"
                "path=\"\"\n"
                "while [ $# -gt 0 ]; do\n"
                "  if [ \"$1\" = \"--status-file\" ]; then path=\"$2\"; fi\n"
                "  shift\n"
                "done\n"
                "printf '{\"status\": \"Error\", \"terminal\":"
                " \"NativeStackExhausted\"}\n' > \"$path\"\n"
            )
            script.chmod(0o755)
            status = Path(tmp) / "status.json"

            result = gap.run_command(
                [str(script)], "print('hi')\n", 30.0, status
            )

            self.assertEqual(result.status, "Error")
            self.assertEqual(result.terminal, "NativeStackExhausted")
            self.assertTrue(gap.worker_crashed(result))


class StatusFileSupportTests(unittest.TestCase):
    """The sweep needs a binary that writes the status file it asks for.

    A binary that ignores `--status-file` cannot be swept at all: every
    case would look like a crash. A status file that cannot be written
    is a harness failure, not a crash, so the sweep stops instead of
    classifying the run.
    """

    def write_fake_binary(self, directory: Path, writes_status: bool) -> Path:
        """Write a fake mnpy that ignores or honours `--status-file`."""
        script = directory / "fake_mnpy.sh"
        if writes_status:
            script.write_text(
                "#!/bin/sh\n"
                "path=\"\"\n"
                "while [ $# -gt 0 ]; do\n"
                "  if [ \"$1\" = \"--status-file\" ]; then path=\"$2\"; fi\n"
                "  shift\n"
                "done\n"
                "printf '{\"status\": \"Success\", \"terminal\": null}\\n' > \"$path\"\n"
            )
        else:
            script.write_text("#!/bin/sh\nexit 0\n")
        script.chmod(0o755)
        return script

    def test_a_binary_that_ignores_the_status_file_fails_the_preflight(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            script = self.write_fake_binary(directory, writes_status=False)
            status = directory / "status.json"

            with self.assertRaises(SystemExit) as caught:
                gap.check_status_file_support(str(script), status, 30.0)

            self.assertEqual(
                str(caught.exception),
                f"mnpy at {script} does not write --status-file; rebuild it",
            )
            self.assertFalse(status.exists())

    def test_a_binary_that_cannot_be_run_fails_the_preflight(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            script = directory / "not_executable.sh"
            script.write_text("#!/bin/sh\nexit 0\n")
            script.chmod(0o644)
            status = directory / "status.json"

            with self.assertRaises(SystemExit) as caught:
                gap.check_status_file_support(str(script), status, 30.0)

            self.assertEqual(
                str(caught.exception),
                f"mnpy at {script} does not write --status-file; rebuild it",
            )
            self.assertFalse(status.exists())

    def test_a_binary_that_writes_the_status_file_passes_the_preflight(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            script = self.write_fake_binary(directory, writes_status=True)
            status = directory / "status.json"

            gap.check_status_file_support(str(script), status, 30.0)

            self.assertEqual(gap.read_status_file(status), ("Success", None))

    def test_the_preflight_asks_the_binary_for_a_status_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            script = self.write_fake_binary(directory, writes_status=True)
            status = directory / "status.json"
            commands = []

            def fake_run(command, source, timeout, status_path=None):
                del timeout
                commands.append((command, source, status_path))
                # A binary that writes no status file reports no status.
                return run_result(status=None)

            with patch.object(gap, "run_command", side_effect=fake_run):
                with self.assertRaises(SystemExit):
                    gap.check_status_file_support(str(script), status, 30.0)

            command, source, status_path = commands[0]
            self.assertEqual(command, [str(script)])
            # The preflight runs the binary the way the sweep runs a
            # case: the program comes in on stdin, not as a -c argument.
            self.assertEqual(source, "pass\n")
            # run_command turns the status path into the --status-file
            # option the binary is asked to honour.
            self.assertEqual(status_path, status)

    def test_a_run_that_cannot_write_its_status_file_stops_the_sweep(self):
        # A run that lost its status file has no readable status: that
        # is what tells a lost status file from a program that printed
        # the failure message itself.
        result = run_result(
            exit_code=2,
            stderr=(
                "mnpy: cannot write status file /tmp/status-0001.json: "
                "No such file or directory (os error 2)\n"
            ),
            status=None,
        )

        self.assertIsNotNone(gap.status_write_failure(result))
        with self.assertRaises(SystemExit) as caught:
            gap.check_status_write(result, "/mnpy")
        self.assertIn("could not write its status file", str(caught.exception))

        with patch.object(gap, "run_command", return_value=result):
            with self.assertRaises(SystemExit):
                gap.run_minipython(["/mnpy"], "", 30.0, Path("/tmp/status.json"))

    def test_a_program_that_forges_a_status_write_failure_is_not_one(self):
        # A program that prints the failure message and exits 2 has
        # still written its status file, so the sweep keeps going.
        result = run_result(
            exit_code=2,
            stderr=(
                "mnpy: cannot write status file /tmp/status-0001.json: "
                "No such file or directory (os error 2)\n"
            ),
            status="Error",
        )

        self.assertIsNone(gap.status_write_failure(result))
        gap.check_status_write(result, "/mnpy")

        with patch.object(gap, "run_command", return_value=result):
            self.assertIs(
                gap.run_minipython(["/mnpy"], "", 30.0, Path("/tmp/status.json")),
                result,
            )

    def test_a_run_that_exits_two_for_another_reason_is_not_a_status_failure(self):
        result = run_result(exit_code=2, stderr="usage: mnpy [options]\n")

        self.assertIsNone(gap.status_write_failure(result))
        gap.check_status_write(result, "/mnpy")

    def test_a_run_that_writes_its_status_file_is_not_a_status_failure(self):
        result = run_result(exit_code=0, stderr="")

        self.assertIsNone(gap.status_write_failure(result))
        gap.check_status_write(result, "/mnpy")


class TriageTests(unittest.TestCase):
    def test_triage_status_marks_passing_accepted_and_open_diffs(self):
        self.assertEqual(gap.triage_status("MATCH"), "passing")
        self.assertEqual(gap.triage_status("STDLIB_MISSING"), "accepted_gap")
        self.assertEqual(gap.triage_status("CPYTHON_INTERNAL"), "accepted_gap")
        self.assertEqual(gap.triage_status("OUTPUT_DIFF"), "needs_triage")
        self.assertEqual(gap.triage_status("TIMEOUT"), "needs_triage")


class ParseArgsTests(unittest.TestCase):
    def test_cpython_oracle_comes_from_minipython_cpython(self):
        with (
            patch.object(sys, "argv", ["cpython_gap_sweep.py", "--require-version", "3.14.6"]),
            patch.dict(os.environ, {"MINIPYTHON_CPYTHON": "/usr/local/bin/python3.14"}),
        ):
            args = gap.parse_args()

        self.assertEqual(args.cpython, "/usr/local/bin/python3.14")

    def test_default_cpython_oracle_is_homebrew_python(self):
        with (
            patch.object(sys, "argv", ["cpython_gap_sweep.py", "--require-version", "3.14.6"]),
            patch.dict(os.environ, {}, clear=False),
        ):
            os.environ.pop("MINIPYTHON_CPYTHON", None)
            args = gap.parse_args()

        self.assertEqual(args.cpython, "/opt/homebrew/bin/python3")
        self.assertEqual(args.require_version, "3.14.6")
        self.assertEqual(args.root_cause, "*")
        self.assertEqual(args.generated_cases, 0)
        self.assertEqual(args.seed, 20260710)
        self.assertEqual(args.layer, "syntax,runtime,stdlib,security")
        self.assertFalse(args.shrink)
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
        cases = gap.load_cases(REPO_ROOT / "tests" / "cases.toml")
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
        cases = gap.load_cases(REPO_ROOT / "tests" / "cases.toml")
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
        sandbox_root_causes = {
            case["root_cause"]
            for case in cases
            if case["category"] == "sandbox-excluded"
        }
        stdlib_root_causes = {
            case["root_cause"]
            for case in cases
            if case["scope"] == "stdlib-sandbox"
        }
        root_cause_counts = {
            root_cause: sum(1 for case in cases if case["root_cause"] == root_cause)
            for root_cause in json_root_causes
        }

        self.assertEqual(REQUIRED_EXPECTED_MARKERS - expected_markers, set())
        self.assertEqual(REQUIRED_JSON_ROOT_CAUSES - json_root_causes, set())
        self.assertEqual(REQUIRED_SANDBOX_ROOT_CAUSES - sandbox_root_causes, set())
        self.assertEqual(REQUIRED_STDLIB_ROOT_CAUSES - stdlib_root_causes, set())
        self.assertGreaterEqual(root_cause_counts["json-loads-core"], 2)


class VersionGuardTests(unittest.TestCase):
    def test_run_sweep_rejects_wrong_oracle_version(self):
        original_oracle_version = gap.oracle_version
        gap.oracle_version = lambda _cpython: "3.14.5"
        args = argparse.Namespace(
            cpython="/does/not/matter",
            require_version="3.14.6",
            minipython="/does/not/matter",
            corpus="tests/cases.toml",
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
        self.assertEqual(
            gap.open_root_cause_commands(open_summary),
            {
                "json-dumps-format-options": [
                    "tests/run.sh",
                    "--root-cause",
                    "json-dumps-format-options",
                ],
            },
        )
        self.assertEqual(
            gap.format_command(
                [
                    "tests/run.sh",
                    "--root-cause",
                    "json-dumps-format-options",
                ]
            ),
            "tests/run.sh --root-cause json-dumps-format-options",
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
            "corpus": "tests/cases.toml",
            "scope": ["syntax"],
            "category": ["syntax"],
            "module": ["json"],
            "root_cause": ["json-loads-core"],
            "layer": ["syntax"],
            "seed": 20260710,
            "generated_cases_selected": 0,
            "corpus_cases_selected": 1,
            "shrink": False,
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
                stderr="Traceback (most recent call last):\nTypeError: bad\n",
                exception_class="TypeError",
                exception_message="TypeError: bad",
            ),
            layer="syntax",
        )

        with tempfile.TemporaryDirectory() as tmp:
            prefix = Path(tmp) / "report"
            gap.write_reports(prefix, meta, [result])
            payload = json.loads(prefix.with_suffix(".json").read_text())
            markdown = prefix.with_suffix(".md").read_text()

        self.assertEqual(payload["summary"], {"OUTPUT_DIFF": 1})
        self.assertEqual(payload["triage"], {"needs_triage": 1})
        self.assertEqual(payload["categories"], {"syntax": 1})
        self.assertEqual(payload["layers"], {"syntax": 1})
        self.assertEqual(payload["origins"], {"corpus": 1})
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
        self.assertEqual(
            payload["open_root_cause_commands"]["json-loads-core"],
            ["tests/run.sh", "--root-cause", "json-loads-core"],
        )
        self.assertEqual(payload["modules"], {"json": 1})
        self.assertEqual(payload["meta"]["required_cpython_version"], "3.14.6")
        self.assertEqual(payload["results"][0]["name"], "case-one")
        self.assertEqual(payload["results"][0]["category"], "syntax")
        self.assertEqual(payload["results"][0]["root_cause"], "json-loads-core")
        self.assertEqual(payload["results"][0]["modules"], ["json"])
        self.assertEqual(payload["results"][0]["triage_status"], "needs_triage")
        self.assertEqual(payload["results"][0]["layer"], "syntax")
        self.assertEqual(payload["results"][0]["origin"], "corpus")
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
            "Traceback (most recent call last):\nTypeError: bad\n",
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
            "| `json-loads-core` | 1 | `needs_triage=1` | `OUTPUT_DIFF=1` | `json` | `tests/run.sh --root-cause json-loads-core` |",
            markdown,
        )
        self.assertIn("| `syntax` | 1 |", markdown)
        self.assertIn(
            "| `json-loads-core` | 1 | `needs_triage=1` | `OUTPUT_DIFF=1` | `json` |",
            markdown,
        )
        self.assertIn("| `json` | 1 |", markdown)
        self.assertIn(
            "| `case-one` | `corpus` | `syntax` | `syntax` | `syntax` | `json-loads-core` | `json` | `must_fix` | `OUTPUT_DIFF` | `needs_triage` |",
            markdown,
        )
        self.assertIn("- Root Cause: `json-loads-core`", markdown)
        self.assertIn("- Triage: `needs_triage`", markdown)
        self.assertIn("- Diff: `stdout differs`", markdown)
        self.assertIn("CPython stdout:", markdown)
        self.assertIn("MiniPython stdout:", markdown)


class DiscoveryIntegrationTests(unittest.TestCase):
    def test_generated_differences_shrink_once_per_root_cause(self):
        generated_case = {
            "name": "generated-integration-case",
            "scope": "core-runtime",
            "category": "runtime-semantic",
            "root_cause": "generated-integration-root",
            "modules": ["core-runtime"],
            "priority": "must_fix",
            "source": "unused = 99\nvalue = 42\nprint(value)\n",
            "layer": "runtime",
            "origin": "generated",
            "seed": 7,
        }
        second_generated_case = {
            **generated_case,
            "name": "generated-integration-case-two",
            "source": "unused = 100\nvalue = 43\nprint(value)\n",
        }

        commands = []

        def fake_run(command, source, timeout, status_path=None):
            del timeout
            commands.append((command, status_path))
            prefix = "cpython" if command[0] == "/cpython" else "minipython"
            return run_result(stdout=f"{prefix}:{len(source)}\n")

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            minipython = root / "mnpy"
            minipython.touch()
            corpus = root / "corpus"
            corpus.mkdir()
            repro_dir = root / "repros"
            args = argparse.Namespace(
                cpython="/cpython",
                require_version="3.14.6",
                minipython=str(minipython),
                corpus=str(corpus),
                scope="syntax,core-runtime,stdlib-sandbox,intentional-sandbox",
                category=",".join(sorted(gap.VALID_CATEGORIES)),
                module="*",
                root_cause="*",
                layer="syntax,runtime,stdlib,security",
                generated_cases=2,
                seed=7,
                shrink=True,
                shrink_max_attempts=32,
                repro_dir=str(repro_dir),
                timeout=1.0,
            )
            with patch.object(
                gap, "oracle_version", return_value="3.14.6"
            ), patch.object(
                gap,
                "generate_cases",
                return_value=[generated_case, second_generated_case],
            ), patch.object(
                gap, "run_command", side_effect=fake_run
            ):
                meta, results = gap.run_sweep(args)

            result = results[0]
            repro = repro_dir / "generated-integration-case.py"
            self.assertEqual(result.status, "OUTPUT_DIFF")
            self.assertEqual(result.triage_status, "needs_triage")
            self.assertLess(len(result.minimized_source), len(result.source))
            self.assertGreater(result.shrink_attempts, 0)
            self.assertTrue(repro.exists())
            self.assertIn(result.minimized_source, repro.read_text())
            self.assertEqual(results[1].status, "OUTPUT_DIFF")
            self.assertIsNone(results[1].minimized_source)
            self.assertEqual(results[1].shrink_attempts, 0)
            self.assertFalse((repro_dir / "generated-integration-case-two.py").exists())
            self.assertEqual(meta["generated_cases_selected"], 2)
            self.assertEqual(meta["corpus_cases_selected"], 0)
            self.assertEqual(meta["seed"], 7)

            # mnpy is asked for a status file; CPython is not.
            mini_runs = [run for run in commands if run[0][0] != "/cpython"]
            self.assertTrue(mini_runs)
            for command, status_path in mini_runs:
                self.assertIsNotNone(status_path)
            for command, status_path in [run for run in commands if run[0][0] == "/cpython"]:
                self.assertIsNone(status_path)

    def test_priority_filter_keeps_only_requested_open_roots(self):
        must_fix = gap.SweepResult(
            name="must-fix",
            scope="core-runtime",
            category="runtime-semantic",
            root_cause="must-root",
            modules=["core-runtime"],
            priority="must_fix",
            status="OUTPUT_DIFF",
            triage_status="needs_triage",
            expected=None,
            diff="stdout differs",
            cpython=run_result(stdout="1\n"),
            minipython=run_result(stdout="2\n"),
        )
        nice_to_have = gap.SweepResult(
            name="nice",
            scope="core-runtime",
            category="exception-shape",
            root_cause="nice-root",
            modules=["core-runtime"],
            priority="nice_to_have",
            status="EXCEPTION_MESSAGE_DIFF",
            triage_status="needs_triage",
            expected=None,
            diff="message differs",
            cpython=run_result(exit_code=1, exception_class="ValueError"),
            minipython=run_result(exit_code=1, exception_class="ValueError"),
        )

        self.assertEqual(
            set(gap.open_root_causes([must_fix, nice_to_have], {"must_fix"})),
            {"must-root"},
        )


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

    def test_priority_gate_fails_wont_fix_crashes_and_broken_contracts(self):
        args = argparse.Namespace(
            out="reports/cpython-gap-sweep",
            fail_on_diff=False,
            fail_on_open=False,
            fail_priority="must_fix,should_fix",
        )
        result = gap.SweepResult(
            name="generated-security-bypass",
            scope="intentional-sandbox",
            category="sandbox-excluded",
            root_cause="sandbox-network-block",
            modules=["socket"],
            priority="wont_fix",
            status="EXPECTED_CONTRACT_DIFF",
            triage_status="needs_triage",
            expected="intentional_sandbox_block",
            diff="expected intentional_sandbox_block but MiniPython now matches CPython",
            cpython=run_result(stdout="imported\n"),
            minipython=run_result(stdout="imported\n"),
        )

        stderr = io.StringIO()
        with patch.object(gap, "parse_args", return_value=args), patch.object(
            gap, "run_sweep", return_value=({}, [result])
        ), patch.object(gap, "write_reports"), patch(
            "sys.stdout", io.StringIO()
        ), patch("sys.stderr", stderr):
            exit_code = gap.main()

        self.assertEqual(exit_code, 1)
        self.assertIn("generated-security-bypass=EXPECTED_CONTRACT_DIFF", stderr.getvalue())

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

LAYERS = gap.LAYERS
generate_cases = gap.generate_cases
literal_reduction_candidates = gap.literal_reduction_candidates
minimize_source = gap.minimize_source
top_level_reduction_candidates = gap.top_level_reduction_candidates

class GenerationTests(unittest.TestCase):
    def test_fixed_seed_is_deterministic_and_names_are_unique(self):
        first = generate_cases(1234, 80, LAYERS)
        second = generate_cases(1234, 80, LAYERS)

        self.assertEqual(first, second)
        self.assertEqual(len(first), 80)
        self.assertEqual(len({case["name"] for case in first}), 80)

    def test_large_shape_is_balanced_across_all_four_layers(self):
        cases = generate_cases(20260710, 1024, LAYERS)

        self.assertEqual(
            Counter(case["layer"] for case in cases),
            {"syntax": 256, "runtime": 256, "stdlib": 256, "security": 256},
        )
        self.assertTrue(all(case["origin"] == "generated" for case in cases))
        self.assertTrue(all(case["seed"] == 20260710 for case in cases))

    def test_runtime_expression_model_combines_multiple_catchable_probes(self):
        case = generate_cases(20260715, 5, ["runtime"])[4]

        self.assertEqual(case["root_cause"], "generated-runtime-expression-model")
        self.assertEqual(case["priority"], "must_fix")
        self.assertEqual(case["modules"], ["core-runtime", "exceptions"])
        self.assertEqual(case["source"].count("lambda:"), 12)
        self.assertIn("except BaseException as error:", case["source"])
        self.assertNotIn(" is ", case["source"])
        self.assertNotIn(" is not ", case["source"])

    def test_runtime_value_shape_model_covers_subscripts_unary_and_builtins(self):
        case = generate_cases(20260715, 6, ["runtime"])[5]

        self.assertEqual(case["root_cause"], "generated-runtime-value-shape-model")
        self.assertEqual(case["priority"], "must_fix")
        self.assertEqual(case["modules"], ["builtins", "core-runtime", "exceptions"])
        self.assertEqual(case["source"].count("lambda:"), 12)
        self.assertEqual(case["source"].count("value_"), 12)
        self.assertEqual(case["source"].count("index_"), 12)
        self.assertNotIn("def getitem", case["source"])
        self.assertNotIn("~(False)", case["source"])
        self.assertNotIn("~(True)", case["source"])
        self.assertIn("except BaseException as error:", case["source"])

    def test_security_generation_is_explicitly_classified(self):
        cases = generate_cases(9, 12, ["security"])

        self.assertTrue(
            all(case["expected"] == "intentional_sandbox_block" for case in cases)
        )
        self.assertTrue(all(case["priority"] == "wont_fix" for case in cases))
        self.assertTrue(all(case["category"] == "sandbox-excluded" for case in cases))
        self.assertTrue(all("expected_cpython_stdout" in case for case in cases))
        self.assertTrue(all("expected_minipython_stdout" in case for case in cases))

    def test_invalid_generation_requests_fail_clearly(self):
        with self.assertRaisesRegex(ValueError, "non-negative"):
            generate_cases(1, -1, LAYERS)
        with self.assertRaisesRegex(ValueError, "unknown discovery layers"):
            generate_cases(1, 1, ["unknown"])
        with self.assertRaisesRegex(ValueError, "at least one"):
            generate_cases(1, 1, [])


class ReductionTests(unittest.TestCase):
    def test_top_level_candidates_preserve_complete_statement_blocks(self):
        source = "value = 1\nif value:\n    print(value)\nprint('done')\n"
        candidates = top_level_reduction_candidates(source)

        self.assertIn("if value:\n    print(value)\nprint('done')\n", candidates)
        self.assertIn("value = 1\nprint('done')\n", candidates)
        self.assertIn("value = 1\nif value:\n    print(value)\n", candidates)

    def test_literal_candidates_simplify_numbers_booleans_and_strings(self):
        candidates = literal_reduction_candidates("print(42, True, 'value')\n")

        self.assertIn("print(0, True, 'value')\n", candidates)
        self.assertIn("print(42, False, 'value')\n", candidates)
        self.assertIn("print(42, True, '')\n", candidates)

    def test_minimizer_keeps_only_candidates_that_preserve_failure(self):
        source = "unused = 99\ntrigger = 42\nprint(trigger)\n"

        minimized, attempts = minimize_source(
            source,
            lambda candidate: "trigger" in candidate and "print" in candidate,
            max_attempts=30,
        )

        self.assertNotIn("unused", minimized)
        self.assertIn("trigger", minimized)
        self.assertIn("print", minimized)
        self.assertGreater(attempts, 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
