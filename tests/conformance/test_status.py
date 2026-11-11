import importlib.util
import json
from pathlib import Path
import re
import sys
import tempfile
import tomllib
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('status', ROOT / 'status.py')
status = importlib.util.module_from_spec(spec)
spec.loader.exec_module(status)

TREE = '''[[leaf]]
id = "leaf-a"
protocol = "alpha"

[[leaf]]
id = "leaf-b"
protocol = "beta"
'''


def report(*pairs):
    return {'summary': {}, 'results': [{'leaf': leaf, 'case': 'case-01.py', 'verdict': verdict}
                                       for leaf, verdict in pairs]}


class StatusTests(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.addCleanup(self.folder.cleanup)
        self.root = Path(self.folder.name)
        self.readme = self.root / 'README.md'
        self.tree = self.root / 'layer0.toml'
        self.report = self.root / 'report.json'
        self.tree.write_text(TREE)
        for name, value in (('README', self.readme), ('TREE', self.tree)):
            patch = mock.patch.object(status, name, value)
            patch.start()
            self.addCleanup(patch.stop)

    def write_report(self, *pairs):
        self.report.write_text(json.dumps(report(*pairs)))

    def write_readme(self, block=''):
        self.readme.write_text('# readme\n\n' + block + '\n## Install\n')

    def run_status(self, *args):
        with mock.patch.object(sys, 'argv', ['status.py', *args]):
            return status.main()

    def block(self):
        payload = json.loads(self.report.read_text())
        tree = tomllib.loads(self.tree.read_text())
        rows = status.counts(payload, tree)
        return status.render(rows, status.totals(rows))

    def test_totals_count_every_verdict_and_every_protocol(self):
        self.write_report(('leaf-a', 'match'), ('leaf-a', 'result-differs'), ('leaf-a', 'output-differs'),
                          ('leaf-b', 'match'), ('leaf-b', 'unsupported'), ('leaf-b', 'crash'),
                          ('leaf-b', 'timeout'))
        block = self.block()
        self.assertIn('Oracle: CPython 3.14.6.', block)
        self.assertIn('Totals: 2 leaves, 7 cases, 2 matching, 2 differing, 1 unsupported, '
                      '1 crash, 1 timeout.', block)
        self.assertIn('| alpha | 1 | 3 | 1 | 2 | 0 | 0 | 0 |', block)
        self.assertIn('| beta | 1 | 4 | 1 | 0 | 1 | 1 | 1 |', block)
        self.assertEqual(block.count('| --- |'), 1)

    def test_the_block_states_the_goal_and_holds_no_volatile_text(self):
        self.write_report(('leaf-a', 'match'), ('leaf-b', 'result-differs'))
        block = self.block()
        self.assertIn('A differing case is a known mismatch that still has to be fixed; the goal is that '
                      'every case matches.', block)
        self.assertIsNone(re.search(r'\d{4}-\d{2}-\d{2}', block))
        self.assertIsNone(re.search(r'/(Users|home)/', block))
        self.assertEqual(block, self.block())

    def test_the_block_names_where_the_numbers_were_measured(self):
        self.write_report(('leaf-a', 'match'))
        self.assertIn(status.MEASUREMENT, self.block())

    def test_the_measurement_line_names_no_other_platform(self):
        self.write_report(('leaf-a', 'match'))
        self.assertNotIn('Linux', self.block())

    def test_write_replaces_the_block_and_is_stable(self):
        self.write_report(('leaf-a', 'match'))
        self.write_readme(status.START + '\nold block\n' + status.END)
        self.assertEqual(self.run_status('--report', str(self.report)), 0)
        first = self.readme.read_bytes()
        self.assertIn(b'## Conformance status', first)
        self.assertNotIn(b'old block', first)
        self.assertIn(b'## Install', first)
        self.assertEqual(self.run_status('--report', str(self.report)), 0)
        self.assertEqual(self.readme.read_bytes(), first)

    def test_write_keeps_the_text_around_the_block(self):
        self.write_report(('leaf-a', 'match'))
        self.readme.write_text('# café\n\n' + status.START + '\nold block\n' + status.END + '\n## Install\n')
        self.assertEqual(self.run_status('--report', str(self.report)), 0)
        written = self.readme.read_bytes()
        self.assertTrue(written.startswith('# café\n\n'.encode()))
        self.assertTrue(written.endswith('\n## Install\n'.encode()))
        self.assertIn('## Conformance status'.encode(), written)

    def test_check_writes_nothing_and_reports_a_stale_block(self):
        self.write_report(('leaf-a', 'match'))
        self.write_readme(status.START + '\nstale block\n' + status.END)
        before = self.readme.read_bytes()
        self.assertEqual(self.run_status('--report', str(self.report), '--check'), 1)
        self.assertEqual(self.readme.read_bytes(), before)
        self.assertEqual(self.run_status('--report', str(self.report)), 0)
        self.assertEqual(self.run_status('--report', str(self.report), '--check'), 0)

    def test_missing_markers_are_an_error(self):
        self.write_report(('leaf-a', 'match'))
        self.readme.write_text('# readme\n')
        self.assertEqual(self.run_status('--report', str(self.report)), 2)
        self.assertEqual(self.run_status('--report', str(self.report), '--check'), 2)

    def test_a_missing_end_marker_is_an_error_and_leaves_the_readme_alone(self):
        self.write_report(('leaf-a', 'match'))
        self.readme.write_text('# readme\n\n' + status.START + '\nblock without an end\n')
        before = self.readme.read_bytes()
        self.assertEqual(self.run_status('--report', str(self.report)), 2)
        self.assertEqual(self.run_status('--report', str(self.report), '--check'), 2)
        self.assertEqual(self.readme.read_bytes(), before)

    def test_a_usage_error_is_an_error(self):
        with self.assertRaises(SystemExit) as caught:
            self.run_status()
        self.assertEqual(caught.exception.code, 2)


if __name__ == '__main__':
    unittest.main()
