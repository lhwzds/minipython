"""Unit tests for the release gate's conformance step.

The step is driven with stubbed commands: every command it would run is
replaced by a stub that reports a chosen exit status, so the verdicts of the
step are checked without a release build, a CPython run, or a git
repository. Nothing outside the temporary directory is written.
"""

from __future__ import annotations

import argparse
import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location('gate', ROOT.parent / 'release_gate' / 'gate.py')
gate = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gate)


class ConformanceStepTests(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.addCleanup(self.folder.cleanup)
        self.root = Path(self.folder.name)
        for name, value in (('REPO', self.root), ('REPORT_DIR', self.root)):
            patch = mock.patch.object(gate, name, value)
            patch.start()
            self.addCleanup(patch.stop)
        args = argparse.Namespace(cpython='/nonexistent/python3', require_version='3.14.6',
                                  allow_dirty=True, keep_going=False)
        self.gate = gate.Gate(args)
        self.gate.target_dir = self.root
        self.gate._built = True

    def drive(self, codes):
        """Run the step with the given exit codes for its three commands."""
        calls = []

        def run(command, log, **kwargs):
            calls.append((command, log, kwargs))
            return subprocess.CompletedProcess(command, codes.pop(0), stdout='')

        def check(command, log, **kwargs):
            calls.append((command, log, kwargs))
            return ''

        self.gate.run = run
        self.gate.check = check
        record = {'key': 'conformance', 'title': 'conformance', 'status': 'running',
                  'details': {}, 'failures': []}
        self.gate.conformance(record)
        return record, calls

    def test_the_step_runs_the_corpus_the_status_check_the_oracle_and_the_tests(self):
        record, calls = self.drive([0, 0, 0])
        self.assertEqual(record['failures'], [])
        self.assertEqual([call[0][1:3] for call in calls[:3]], [
            ['tests/conformance/run.py', '--mnpy'],
            ['tests/conformance/status.py', '--report'],
            ['tests/conformance/run.py', '--check-oracle'],
        ])
        self.assertEqual(calls[2][0][3:], ['--cpython', '/nonexistent/python3'])
        self.assertEqual(calls[3][0][1:5], ['-I', '-m', 'unittest', 'discover'])
        self.assertEqual(record['details']['oracle_drift'], False)

    def test_the_step_never_regenerates_the_observations(self):
        _, calls = self.drive([0, 0, 0])
        commands = [call[0] for call in calls]
        self.assertFalse([command for command in commands if '--update-oracle' in command])
        self.assertFalse([command for command in commands if command[:2] == ['git', 'diff']])

    def test_a_failing_comparison_fails_the_step(self):
        record, _ = self.drive([1, 0, 0])
        self.assertEqual(len(record['failures']), 1)
        self.assertIn('the conformance comparison failed', record['failures'][0])

    def test_a_stale_status_block_fails_the_step(self):
        record, _ = self.drive([0, 1, 0])
        self.assertEqual(len(record['failures']), 1)
        self.assertIn('stale', record['failures'][0])

    def test_a_failing_status_check_is_not_a_stale_block(self):
        record, _ = self.drive([0, 2, 0])
        self.assertEqual(len(record['failures']), 1)
        self.assertIn('the conformance status check failed', record['failures'][0])
        self.assertNotIn('stale', record['failures'][0])

    def test_a_changed_observation_is_reported_as_oracle_drift(self):
        record, _ = self.drive([0, 0, 1])
        self.assertEqual(len(record['failures']), 1)
        self.assertIn('oracle drift:', record['failures'][0])
        self.assertEqual(record['details']['oracle_drift'], True)

    def test_the_oracle_drift_message_names_both_remedies(self):
        record, _ = self.drive([0, 0, 1])
        self.assertEqual(len(record['failures']), 1)
        message = record['failures'][0]
        self.assertIn('regenerate them and commit them', message)
        self.assertIn('or fix the case that differs', message)

    def test_a_failing_oracle_check_is_not_oracle_drift(self):
        record, _ = self.drive([0, 0, 2])
        self.assertEqual(len(record['failures']), 1)
        self.assertIn('the oracle check failed', record['failures'][0])
        self.assertNotIn('oracle drift:', record['failures'][0])
        self.assertEqual(record['details']['oracle_drift'], False)


if __name__ == '__main__':
    unittest.main()
