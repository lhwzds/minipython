"""Every GitHub Actions runner label in .github/workflows is pinned.

A floating label such as ubuntu-latest moves to a new image when GitHub
changes it, and a new image can change C math library results that the
release gate's records were measured on. Labels must name an image version.
"""

import re
import unittest
from pathlib import Path

WORKFLOWS = Path(__file__).resolve().parents[2] / '.github' / 'workflows'
LABEL = re.compile(r'\b(?:runs-on|os):\s*([A-Za-z0-9._-]+)')


def runner_labels(text):
    return LABEL.findall(text)


class WorkflowRunnerTests(unittest.TestCase):
    def test_workflows_exist(self):
        self.assertTrue(sorted(WORKFLOWS.glob('*.yml')))

    def test_no_floating_runner_label(self):
        for path in sorted(WORKFLOWS.glob('*.yml')):
            text = path.read_text()
            with self.subTest(workflow=path.name):
                self.assertNotIn('-latest', text)
                for label in runner_labels(text):
                    if label.startswith('$'):
                        continue
                    self.assertRegex(label, r'^(?:ubuntu|macos|windows)-[0-9][0-9.]*(?:-[a-z0-9]+)*$')

    def test_the_check_refuses_a_floating_label(self):
        self.assertEqual(runner_labels('    runs-on: ubuntu-latest\n'), ['ubuntu-latest'])
        self.assertNotRegex('ubuntu-latest', r'^(?:ubuntu|macos|windows)-[0-9][0-9.]*(?:-[a-z0-9]+)*$')
        self.assertRegex('ubuntu-24.04-arm', r'^(?:ubuntu|macos|windows)-[0-9][0-9.]*(?:-[a-z0-9]+)*$')


if __name__ == '__main__':
    unittest.main()
