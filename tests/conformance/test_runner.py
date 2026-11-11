import contextlib
import importlib.util
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('runner', ROOT / 'run.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def result(stdout='', stderr='', exit=0, timeout=False):
    return {'stdout': stdout.splitlines(keepends=True), 'stderr': stderr,
            'exit': exit, 'timeout': timeout}


class RunnerTests(unittest.TestCase):
    def test_all_differences_and_precedence(self):
        expected = result('{"kind":"calls","x":1}\n{"kind":"exception","x":1}\nplain\n', 'old\n', 1)
        actual = result('{"kind":"calls","x":2}\n{"kind":"exception","x":2}\nother\n', 'new\n', 2)
        verdict, differences = runner.compare(expected, actual)
        self.assertEqual(verdict, 'exception-differs')
        self.assertEqual([d['kind'] for d in differences], ['calls', 'exception', 'output', 'stderr', 'exit'])

    def test_newline_and_stderr_semantics(self):
        self.assertEqual(runner.compare(result('x\n'), result('x'))[0], 'output-differs')
        self.assertEqual(runner.compare(result(stderr='a\nlast\n', exit=1),
                                        result(stderr='b\nlast\n', exit=1))[0], 'output-differs')
        self.assertEqual(runner.compare(result(stderr='a\nlast\n'),
                                        result(stderr='b\nlast\n'))[0], 'output-differs')

    def test_verdicts(self):
        expected = result('x\n')
        for actual, verdict in [
            (result(timeout=True, exit=-9), 'timeout'),
            (result(stderr="thread 'main' (123) panicked at f.rs:1:2:\n", exit=3), 'crash'),
            (result(stderr='ModuleNotFoundError: x\n', exit=1), 'unsupported'),
            (result(stderr='TypeError: not supported\n', exit=1), 'output-differs'),
            (result('{"kind":"result"}\n'), 'output-differs'),
        ]:
            self.assertEqual(runner.compare(expected, actual)[0], verdict)

    def test_execute_environment_and_timeout(self):
        program = 'import os; print(sorted(os.environ)); print(os.getcwd())'
        out = runner.execute([sys.executable, '-c', program])
        self.assertEqual(out['exit'], 0)
        self.assertIn("'PYTHONHASHSEED'", out['stdout'][0])
        self.assertEqual(out['stdout'][1].strip(), str(runner.CASES))
        with mock.patch.object(runner, 'TIMEOUT', 0.1):
            out = runner.execute([sys.executable, '-c', 'import time; time.sleep(30)'])
        self.assertTrue(out['timeout'])
        self.assertLess(out['exit'], 0)

    def test_fake_interpreter_and_orphan(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            fake = root / 'fake.py'
            fake.write_text('print("fake")\n')
            out = runner.execute([sys.executable, str(fake)])
            self.assertEqual(out['stdout'], ['fake\n'])
            case_dir = root / 'cases' / 'leaf'
            case_dir.mkdir(parents=True)
            (case_dir / 'case-01.py').write_text('pass\n')
            with mock.patch.object(runner, 'CASES', root / 'cases'):
                with self.assertRaises(ValueError): runner.corpus()
                (case_dir / 'case-01.expected').write_text('{}')
                self.assertEqual(len(runner.corpus()), 1)
                (case_dir / 'orphan.expected').write_text('{}')
                with self.assertRaises(ValueError): runner.corpus()

    def test_relative_interpreter_path(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'python').symlink_to(sys.executable)
            previous = Path.cwd()
            try:
                os.chdir(root)
                out = runner.execute(['./python', '-c', 'print("relative")'])
            finally:
                os.chdir(previous)
            self.assertEqual(out['stdout'], ['relative\n'])
            self.assertEqual(out['exit'], 0)

    def test_normalize(self):
        self.assertEqual(runner.normalize('<object at 0xab> 0x12\r\n'), '<object at <addr>> 0x12\r\n')

    def test_exact_failure_acceptance(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            case_dir = root / 'cases' / 'leaf'
            case_dir.mkdir(parents=True)
            (root / 'behaviours.py').write_text('')
            (root / 'trace.py').write_text('')
            (case_dir / 'case-01.py').write_text('pass')
            expected = result('{"kind":"result","a":1}\nplain\n')
            expected.pop('timeout')
            (case_dir / 'case-01.expected').write_text(json.dumps(expected))
            fake = root / 'fake'
            def fake_output(text):
                fake.write_text('#!' + sys.executable + '\nprint(' + repr(text) + ', end="")\n')
            fake_output('{"kind":"result","a":2}\nother\n')
            fake.chmod(0o755)
            out = root / 'report.json'
            manifest = root / 'expected-failures.toml'
            def entry(differences, case='case-01.py', reason='test'):
                digest = hashlib.sha256(json.dumps(differences, sort_keys=True).encode()).hexdigest()
                manifest.write_text(f'[[failure]]\nleaf = "leaf"\ncase = "{case}"\nreason = "{reason}"\ndigest = "{digest}"\n')
            actual = result('{"kind":"result","a":2}\nother\n')
            differences = runner.compare(expected, actual)[1]
            with mock.patch.object(runner, 'ROOT', root), mock.patch.object(runner, 'CASES', root / 'cases'):
                entry(differences)
                self.assertEqual(runner.run(str(fake), out, None), 0)
                entry(runner.compare(expected, result('{"kind":"result","a":3}\nother\n'))[1])
                self.assertEqual(runner.run(str(fake), out, None), 1)
                entry(differences)
                fake_output('{"kind":"result","a":2}\nplain\n')
                self.assertEqual(runner.run(str(fake), out, None), 1)
                fake_output('{"kind":"result","a":2}\nother\n')
                entry(differences)
                valid_entry = manifest.read_text()
                entry(differences, case='missing.py')
                manifest.write_text(valid_entry + manifest.read_text())
                self.assertEqual(runner.run(str(fake), out, None), 1)
                entry(runner.compare(expected, result('{"kind":"result","a":3}\nother\n'))[1])
                manifest.write_text(manifest.read_text() + valid_entry)
                self.assertEqual(runner.run(str(fake), out, None), 1)
                fake_output('{"kind":"result","a":1}\nplain\n')
                entry(differences)
                self.assertEqual(runner.run(str(fake), out, None), 1)
                manifest.write_text('')
                self.assertEqual(runner.run(str(fake), out, None), 0)
                entry(differences, reason='')
                with self.assertRaises(ValueError):
                    runner.run(str(fake), out, None)

    def test_non_event_json_is_output(self):
        self.assertEqual(runner.compare(result(), result('42\n'))[1][0]['kind'], 'output')
        self.assertEqual(runner.compare(result(), result('{"kind":"result"}\n'))[1][0]['kind'], 'result')
        self.assertEqual(runner.compare(result('42\n'), result('{"kind":"result"}\n'))[1][0]['kind'], 'output')

    def test_equality_corpus_dispatch_paths(self):
        def case(leaf, number):
            path = ROOT / 'cases' / leaf / f'case-{number:02}.py'
            data = json.loads(path.with_suffix('.expected').read_text())
            return path.read_text(), [json.loads(line) for line in data['stdout']]

        source, events = case('dict-setitem-user-eq', 3)
        self.assertIn('|=', source)
        self.assertIn('Counter(source)', source)
        self.assertIn('merged', events[-2])
        self.assertIn('counter', events[-1])

        leaf = 'counter-eq-operand-order'
        self.assertIn('c == d', case(leaf, 1)[0])
        self.assertIn('d == c', case(leaf, 2)[0])
        self.assertEqual([x['left'] for x in case(leaf, 3)[1][:-1]], ['d', 'c'])
        self.assertFalse(case(leaf, 4)[1][-1]['equal'])
        self.assertIn('c != d', case(leaf, 5)[0])

        leaf = 'equality-exception'
        for number, expression in enumerate(('a == 0', '0 == a', 'a != 0',
                                             'b == a', '[a] == [b]'), 1):
            source, events = case(leaf, number)
            self.assertIn(expression, source)
            self.assertEqual(events[0]['type'], 'ValueError')
        leaf = 'equality-wrong-type'
        self.assertEqual(case(leaf, 1)[1][0]['result'], 'truthy')
        self.assertIn('0 == a', case(leaf, 2)[0])
        self.assertIs(case(leaf, 3)[1][0]['result'], False)
        self.assertIs(case(leaf, 4)[1][0]['result'], True)
        events = case(leaf, 5)[1]
        self.assertEqual(events[0]['result'], 'subclass result')
        self.assertEqual(events[1]['calls'], [['__eq__', 'b', 'a']])
        self.assertNotEqual(events[1]['calls'], case(leaf, 1)[1][1]['calls'])
        leaf = 'equality-notimplemented'
        self.assertIs(case(leaf, 1)[1][0]['same'], True)
        self.assertIs(case(leaf, 2)[1][0]['distinct'], False)
        self.assertIs(case(leaf, 3)[1][0]['unequal'], True)
        self.assertEqual(len(case(leaf, 4)[1][1]['calls']), 2)
        self.assertEqual(case(leaf, 5)[1][1]['calls'], [['__eq__', 'b', 'a']])


class CheckOracleTests(unittest.TestCase):
    """The oracle check of the runner: it compares and writes no oracle or report.

    The cases it executes are trusted, tracked code.
    """

    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.addCleanup(self.folder.cleanup)
        self.root = Path(self.folder.name)
        self.cases = self.root / 'cases'
        (self.cases / 'leaf').mkdir(parents=True)
        (self.root / 'behaviours.py').write_text('')
        (self.root / 'trace.py').write_text('')
        (self.cases / 'leaf' / 'case-01.py').write_text('pass\n')
        self.record('out\n')
        patch = mock.patch.multiple(runner, ROOT=self.root, CASES=self.cases)
        patch.start()
        self.addCleanup(patch.stop)

    def record(self, stdout):
        """The recorded observation of the case."""
        (self.cases / 'leaf' / 'case-01.expected').write_text(
            json.dumps({'exit': 0, 'stderr': '', 'stdout': [stdout]},
                       sort_keys=True, indent=2, ensure_ascii=True) + '\n')

    def interpreter(self, version='3.14.6', body="sys.stdout.write('out\\n')"):
        """A fake pinned CPython that answers the version probe and the case."""
        path = self.root / 'python'
        path.write_text('#!' + sys.executable + '\nimport sys\n'
                        'program = sys.argv[sys.argv.index("-c") + 1]\n'
                        'if "sys.version_info" in program:\n'
                        f'    sys.stdout.write("{version}\\n")\n'
                        'else:\n'
                        f'    {body}\n')
        path.chmod(0o755)
        return path

    def cli(self, interpreter, *arguments):
        """The exit code and the stderr of the runner on the command line."""
        stderr = io.StringIO()
        argv = ['run.py', *arguments, '--cpython', str(interpreter)]
        with mock.patch.object(sys, 'argv', argv), contextlib.redirect_stderr(stderr):
            code = runner.cli()
        return code, stderr.getvalue()

    def test_a_matching_corpus_exits_zero_and_writes_nothing(self):
        expected = self.cases / 'leaf' / 'case-01.expected'
        before = expected.read_bytes()
        code, stderr = self.cli(self.interpreter(), '--check-oracle')
        self.assertEqual(code, 0)
        self.assertEqual(stderr, '')
        self.assertEqual(expected.read_bytes(), before)

    def test_a_differing_case_is_named_on_stderr(self):
        self.record('other\n')
        code, stderr = self.cli(self.interpreter(), '--check-oracle')
        self.assertEqual(code, 1)
        self.assertIn('leaf/case-01.py', stderr)

    def test_a_wrong_version_is_a_runner_error(self):
        code, stderr = self.cli(self.interpreter(version='3.12.0'), '--check-oracle')
        self.assertEqual(code, 2)
        self.assertIn('runner error', stderr)

    def test_a_case_without_its_expected_file_is_a_runner_error(self):
        (self.cases / 'leaf' / 'case-02.py').write_text('pass\n')
        code, stderr = self.cli(self.interpreter(), '--check-oracle')
        self.assertEqual(code, 2)
        self.assertIn('runner error', stderr)

    def test_an_expected_file_without_its_case_is_a_runner_error(self):
        (self.cases / 'leaf' / 'case-99.expected').write_text('{}\n')
        code, stderr = self.cli(self.interpreter(), '--check-oracle')
        self.assertEqual(code, 2)
        self.assertIn('runner error', stderr)

    def test_a_timeout_is_a_runner_error(self):
        with mock.patch.object(runner, 'TIMEOUT', 2):
            code, stderr = self.cli(self.interpreter(body='import time; time.sleep(30)'), '--check-oracle')
        self.assertEqual(code, 2)
        self.assertIn('timed out', stderr)

    def test_the_update_still_rewrites_the_observations(self):
        self.record('other\n')
        code, _ = self.cli(self.interpreter(), '--update-oracle')
        self.assertEqual(code, 0)
        recorded = json.loads((self.cases / 'leaf' / 'case-01.expected').read_text())
        self.assertEqual(recorded['stdout'], ['out\n'])


if __name__ == '__main__':
    unittest.main()
