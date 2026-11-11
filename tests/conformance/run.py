#!/usr/bin/env python3
"""Compare MiniPython with checked-in, black-box CPython observations.

`--mnpy P --out R [--leaf ID]` compares the release binary with the recorded
observations, `--check-oracle --cpython P` compares the recorded observations
with the oracle and writes no oracle or report, and `--update-oracle --cpython P`
rewrites the recorded observations. The check exits 0 when every observation
matches, 1 when one differs, and 2 on a runner error. The cases it executes are
trusted, tracked code.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent
CASES = ROOT / 'cases'
ORACLE_VERSION = (3, 14, 6)
TIMEOUT = 10
ADDRESS = re.compile(r'(<(?:[^<>\n]|<[^<>\n]*>)*\bat )0x[0-9a-fA-F]+(?=>)')
PANIC = re.compile(r"^thread '[^'\n]+'(?: \(\d+\))? panicked at .+:\d+:\d+:", re.MULTILINE)


def normalize(text):
    return ADDRESS.sub(r'\1<addr>', text)


def execute(command):
    if os.sep in command[0]:
        command = [str(Path(command[0]).resolve()), *command[1:]]
    env = {'PATH': os.environ.get('PATH', os.defpath), 'PYTHONHASHSEED': '0',
           'PYTHONIOENCODING': 'utf-8'}
    p = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE, cwd=CASES, env=env)
    try:
        out, err = p.communicate(timeout=TIMEOUT)
        timeout = False
    except subprocess.TimeoutExpired:
        p.kill()
        out, err = p.communicate(timeout=TIMEOUT)
        timeout = True
    return {'stdout': normalize(out.decode('utf-8', errors='replace')).splitlines(keepends=True),
            'stderr': normalize(err.decode('utf-8', errors='replace')),
            'exit': p.returncode, 'timeout': timeout}


def last_line(text):
    return (text.splitlines() or [''])[-1]


def event(line):
    try:
        value = json.loads(line)
        return value.get('kind') if isinstance(value, dict) else None
    except (ValueError, TypeError):
        return None


def compare(expected, actual):
    differences = []
    left, right = expected['stdout'], actual['stdout']
    for i in range(max(len(left), len(right))):
        a = left[i] if i < len(left) else None
        b = right[i] if i < len(right) else None
        if a != b:
            ak, bk = event(a), event(b)
            kind = ('output' if (a is not None and ak is None) or (b is not None and bk is None) else
                    'calls' if ak == bk == 'calls' else
                    'exception' if 'exception' in (ak, bk) else
                    'result' if ak is not None or bk is not None else 'output')
            differences.append({'kind': kind, 'line': i + 1, 'expected': a, 'actual': b})
    a, b = expected['stderr'], actual['stderr']
    if a != b:
        differences.append({'kind': 'stderr', 'expected': a, 'actual': b})
    if expected['exit'] != actual['exit']:
        differences.append({'kind': 'exit', 'expected': expected['exit'], 'actual': actual['exit']})
    if actual['timeout']:
        verdict = 'timeout'
    elif not differences:
        verdict = 'match'
    elif actual['exit'] < 0 or actual['exit'] == 101 or PANIC.search(actual['stderr']):
        verdict = 'crash'
    elif actual['exit'] != 0 and last_line(actual['stderr']).startswith(
            ('ImportError', 'ModuleNotFoundError', 'NotImplementedError')):
        verdict = 'unsupported'
    else:
        kinds = {d['kind'] for d in differences}
        verdict = next((name + '-differs' for name in ('exception', 'result', 'output', 'calls')
                        if name in kinds), 'output-differs')
    return verdict, differences


def corpus(leaf=None):
    cases = sorted(CASES.glob('*/*.py'))
    expected = set(CASES.glob('*/*.expected'))
    if leaf:
        cases = [p for p in cases if p.parent.name == leaf]
        expected = {p for p in expected if p.parent.name == leaf}
    if {p.with_suffix('.expected') for p in cases} != expected:
        raise ValueError('case and expected files do not correspond')
    if not cases:
        raise ValueError('no cases found')
    return cases


def pending_changes(cpython):
    """The recorded observations that differ from the oracle's, as (file, text)."""
    version = execute([cpython, '-s', '-c', 'import sys; print(*sys.version_info[:3], sep=".")'])
    if version['exit'] != 0 or ''.join(version['stdout']).strip() != '.'.join(map(str, ORACLE_VERSION)):
        raise ValueError('oracle interpreter version must be 3.14.6')
    prelude = (ROOT / 'behaviours.py').read_text() + '\n' + (ROOT / 'trace.py').read_text()
    pending = []
    for path in sorted(CASES.glob('*/*.py')):
        result = execute([cpython, '-s', '-c', prelude + '\n' + path.read_text()])
        if result.pop('timeout'):
            raise ValueError('oracle timed out: ' + str(path))
        content = json.dumps(result, sort_keys=True, indent=2, ensure_ascii=True) + '\n'
        target = path.with_suffix('.expected')
        if not target.exists() or target.read_text() != content:
            pending.append((target, content))
    return pending


def update(cpython):
    """Rewrite the recorded observations that differ from the oracle's."""
    for target, content in pending_changes(cpython):
        with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=target.parent, delete=False) as f:
            f.write(content)
        os.replace(f.name, target)


def check(cpython):
    """Report the recorded observations that differ from the oracle's; write no oracle or report.

    The cases it executes are trusted, tracked code.
    """
    corpus()
    pending = pending_changes(cpython)
    for target, _ in pending:
        case = target.with_suffix('.py').relative_to(CASES)
        print(f'{case}: the recorded observation differs from the oracle', file=sys.stderr)
    return 1 if pending else 0


def run(mnpy, out, leaf):
    cases = corpus(leaf)
    failures = tomllib.loads((ROOT / 'expected-failures.toml').read_text()).get('failure', [])
    if any(not f.get('reason') for f in failures):
        raise ValueError('failure reason is required')
    prelude = (ROOT / 'behaviours.py').read_text() + '\n' + (ROOT / 'trace.py').read_text()
    results = []
    for path in cases:
        expected = json.loads(path.with_suffix('.expected').read_text())
        actual = execute([mnpy, '-c', prelude + '\n' + path.read_text()])
        verdict, differences = compare(expected, actual)
        key = (path.parent.name, path.name)
        entries = [f for f in failures if (f['leaf'], f['case']) == key]
        digest = hashlib.sha256(json.dumps(differences, sort_keys=True).encode()).hexdigest()
        result = {'leaf': key[0], 'case': key[1], 'verdict': verdict,
                  'accepted': verdict != 'match' and bool(entries) and all(f['digest'] == digest for f in entries),
                  'differences': differences}
        if verdict != 'match':
            result['digest'] = digest
        results.append(result)
    summary = {}
    for result in results:
        verdict = result['verdict']
        summary[verdict] = summary.get(verdict, 0) + 1
    Path(out).write_text(json.dumps({'summary': summary, 'results': results}, sort_keys=True,
                                    indent=2, ensure_ascii=True) + '\n')
    bad = any(r['verdict'] != 'match' and not r['accepted'] for r in results)
    observed = {(r['leaf'], r['case']) for r in results if r['accepted']}
    bad |= any((f['leaf'], f['case']) not in observed for f in failures if not leaf or f['leaf'] == leaf)
    return int(bad)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--mnpy')
    parser.add_argument('--out')
    parser.add_argument('--leaf')
    parser.add_argument('--update-oracle', action='store_true')
    parser.add_argument('--check-oracle', action='store_true')
    parser.add_argument('--cpython')
    args = parser.parse_args()
    if args.update_oracle and args.check_oracle:
        parser.error('--update-oracle and --check-oracle are exclusive')
    oracle = args.update_oracle or args.check_oracle
    if oracle and args.cpython and not (args.mnpy or args.out or args.leaf):
        if args.update_oracle:
            update(args.cpython)
            return 0
        return check(args.cpython)
    if not oracle and args.mnpy and args.out and not args.cpython:
        return run(args.mnpy, args.out, args.leaf)
    parser.error('use --mnpy P --out R [--leaf ID], --update-oracle --cpython P, '
                 'or --check-oracle --cpython P')


def cli():
    """The exit code of the command line: 0 success, 1 a difference, 2 a runner error."""
    try:
        return main()
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as exc:
        print('runner error: ' + str(exc), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(cli())
