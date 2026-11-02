"""Regression tests for the platform-scoped exclusions of the oracle corpora.

The `datetime` corpus marks the scenarios whose result the platform C
library decides (tests/README.md, "Platform-dependent behavior"). The runner
skips exactly those on the platforms the manifest lists for the corpus and
counts every one of them as a documented exclusion, never as a pass; a
scenario mnpy did not print is a mismatch, never an exclusion. The generator
marks only the scenarios that really call an affected format, so a scenario
that computes its result itself (`ctime()`, `isocalendar()`, ...) stays
compared on every platform.

Run them with the pinned CPython:

    python3 tests/oracles/test_platform_exclusions.py
"""

from __future__ import annotations

import importlib.util
import random
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import run_oracles  # noqa: E402

REASON = "platform strftime (Linux)"
TIMETUPLE_REASON = "platform datetime timetuple type (Linux)"


def load_generator():
    """The datetime corpus generator, imported without running it."""
    path = HERE / "datetime" / "random_corpus.py"
    spec = importlib.util.spec_from_file_location("datetime_random_corpus", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"failed to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ScriptedRandom(random.Random):
    """A `random.Random` that picks one kind of scenario and one value.

    `kind` is what `randrange` returns (the scenario kind), `wanted` the
    entry `choice` picks when the sequence holds it, and `number` what
    `randint` returns (a year below 1000 when it is 1).
    """

    def __init__(self, kind: int = 0, wanted: str | None = None, number: int = 1):
        super().__init__(0)
        self.kind = kind
        self.wanted = wanted
        self.number = number

    def randrange(self, stop):
        return self.kind

    def randint(self, low, high):
        return self.number

    def random(self):
        return 0.0

    def choice(self, seq):
        if self.wanted is not None:
            for item in seq:
                if self.wanted in str(item):
                    return item
        return seq[0]


def completed(stdout: str, returncode: int = 0) -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess(["python"], returncode, stdout.encode(), b"")


class RunnerAccountingTests(unittest.TestCase):
    """How the runner counts a marked scenario."""

    def compare(self, cpython_out: str, mnpy_out: str, marked=()):
        with tempfile.TemporaryDirectory() as directory:
            corpus = {"_dir": Path(directory), "compare": "keyed", "key_pattern": r"s\d+"}
            program = Path(directory) / "prog000.py"
            program.write_text("")
            return run_oracles.compare_program(
                corpus,
                program,
                completed(cpython_out),
                completed(mnpy_out),
                {},
                set(marked),
                REASON,
            )

    def test_a_marked_scenario_is_an_exclusion_and_never_a_pass(self):
        result = self.compare("s1 -> 'a'\n", "s1 -> 'a'\n", marked=["s1"])
        self.assertEqual(result.scenarios, 1)
        self.assertEqual(result.excluded, 1)
        self.assertEqual(result.matched, 0)
        self.assertEqual(result.mismatches, [])
        self.assertEqual(result.platform_exclusions, [("prog000.py:s1", REASON)])

    def test_a_marked_scenario_that_differs_is_an_exclusion(self):
        result = self.compare("s1 -> 'a'\n", "s1 -> 'b'\n", marked=["s1"])
        self.assertEqual(result.excluded, 1)
        self.assertEqual(result.matched, 0)
        self.assertEqual(result.mismatches, [])
        self.assertEqual(result.platform_exclusions, [("prog000.py:s1", REASON)])

    def test_a_marked_scenario_without_a_minipython_record_is_a_mismatch(self):
        result = self.compare("s1 -> 'a'\n", "", marked=["s1"])
        self.assertEqual(result.excluded, 0)
        self.assertEqual(result.matched, 0)
        self.assertEqual(len(result.mismatches), 1)
        self.assertEqual(result.mismatches[0]["key"], "s1")
        self.assertEqual(result.platform_exclusions, [])

    def test_an_unmarked_scenario_that_differs_is_a_mismatch(self):
        result = self.compare("s1 -> 'a'\n", "s1 -> 'b'\n")
        self.assertEqual(result.excluded, 0)
        self.assertEqual(result.matched, 0)
        self.assertEqual(len(result.mismatches), 1)

    def test_an_unmarked_scenario_that_matches_is_a_pass(self):
        result = self.compare("s1 -> 'a'\n", "s1 -> 'a'\n")
        self.assertEqual(result.matched, 1)
        self.assertEqual(result.excluded, 0)


class PlatformScopeTests(unittest.TestCase):
    """Which platform the exclusion applies to."""

    def corpus(self):
        return {"platform_exclusions": [{"platform": "Linux", "reason": REASON}]}

    def test_the_exclusion_applies_on_linux(self):
        with mock.patch.object(run_oracles.platform, "system", return_value="Linux"):
            self.assertEqual(len(run_oracles.platform_exclusions(self.corpus())), 1)

    def test_the_exclusion_does_not_apply_on_macos(self):
        with mock.patch.object(run_oracles.platform, "system", return_value="Darwin"):
            self.assertEqual(run_oracles.platform_exclusions(self.corpus()), [])

    def test_the_datetime_corpus_lists_the_linux_exclusion(self):
        manifest = tomllib.loads((HERE / "corpora.toml").read_text())
        corpus = next(
            entry
            for entry in manifest["corpus"]
            if entry["module"] == "datetime" and entry["name"] == "random"
        )
        self.assertEqual(
            corpus["platform_exclusions"],
            [
                {"platform": "Linux", "reason": REASON},
                {"platform": "Linux", "reason": TIMETUPLE_REASON},
            ],
        )


class GeneratorMarkTests(unittest.TestCase):
    """Which scenarios the datetime corpus generator marks."""

    def setUp(self):
        self.generator = load_generator()

    def scenario(self, kind: int, wanted: str | None = None, number: int = 1):
        self.generator.rng = ScriptedRandom(kind, wanted, number)
        self.generator.PLATFORM_KEYS.clear()
        source = self.generator.scenario(0)
        return source, list(self.generator.PLATFORM_KEYS)

    def test_a_low_year_strftime_scenario_is_marked(self):
        source, marks = self.scenario(8, wanted="Y", number=1)
        self.assertIn("strftime(", source)
        self.assertEqual(marks, ["s0"])

    def test_a_ctime_scenario_is_not_marked(self):
        source, marks = self.scenario(13, wanted="ctime()", number=1)
        self.assertIn("ctime()", source)
        self.assertEqual(marks, [])

    def test_a_low_year_format_scenario_is_marked(self):
        source, marks = self.scenario(13, wanted='format({}, "%Y|%H")', number=1)
        self.assertIn("format(", source)
        self.assertEqual(marks, ["s0"])

    def test_the_pure_results_are_not_marked(self):
        source, marks = self.scenario(21, number=1)
        self.assertIn("isocalendar()", source)
        self.assertIn("ctime()", source)
        self.assertEqual(marks, [])

    def test_the_affected_formats_are_the_documented_ones(self):
        dependent = self.generator.platform_dependent
        for fmt in ("%Y", "%G", "%C", "%F", "%c", "%x"):
            self.assertTrue(dependent(fmt, "date(999, 1, 1)"), fmt)
            self.assertFalse(dependent(fmt, "date(2020, 1, 1)"), fmt)
        self.assertTrue(dependent("%q", "date(2020, 1, 1)"))
        self.assertTrue(dependent("%-Y", "date(2020, 1, 1)"))
        self.assertFalse(dependent("%-d", "date(2020, 1, 1)"))
        self.assertFalse(dependent("%H:%M:%S.%f %z %Z", "time(1, 2, 3)"))

    def test_only_scenarios_that_call_strftime_are_marked(self):
        self.generator.rng = random.Random(20260710)
        self.generator.PLATFORM_KEYS.clear()
        body = [self.generator.PRELUDE]
        for index in range(400):
            body.append(self.generator.scenario(index))
        program = "\n".join(body) + "\n"
        marked = set(self.generator.PLATFORM_KEYS)
        sources = self.generator.scenario_sources(program)
        for label in marked:
            self.assertIn(label, sources)
            self.assertTrue(
                self.generator.calls_strftime(sources[label]),
                f"{label} is marked but does not call strftime: {sources[label]}",
            )
        for label, source in sources.items():
            if "ctime()" in source or "isocalendar()" in source:
                self.assertNotIn(label, marked, f"{label} must not be marked: {source}")


if __name__ == "__main__":
    unittest.main()
