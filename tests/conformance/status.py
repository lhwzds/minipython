#!/usr/bin/env python3
"""Write the conformance status block of README.md from a runner report.

usage: status.py --report REPORT [--check]

--report rewrites the block between the `<!-- conformance-status:start -->`
and the `<!-- conformance-status:end -->` markers of the repository README.md
from a report of `run.py` and the leaves of `tree/layer0.toml`. --check
writes nothing and exits 1 when the block differs from the one the report
implies. A usage error or a missing marker exits 2.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parent
README = ROOT.parent.parent / 'README.md'
TREE = ROOT / 'tree' / 'layer0.toml'
START = '<!-- conformance-status:start -->'
END = '<!-- conformance-status:end -->'
COLUMNS = ('matching', 'differing', 'unsupported', 'crash', 'timeout')
# Where the numbers come from. The line is fixed, not read from the running
# host, so that `--check` sees the same block on every host.
MEASUREMENT = 'Snapshot from one macOS release-build run.'

_spec = importlib.util.spec_from_file_location('runner', ROOT / 'run.py')
_runner = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_runner)
ORACLE_VERSION = '.'.join(str(part) for part in _runner.ORACLE_VERSION)


def bucket(verdict):
    """The status column a verdict belongs to."""
    if verdict == 'match':
        return 'matching'
    if verdict.endswith('-differs'):
        return 'differing'
    return verdict


def counts(report, tree):
    """The per-protocol counts of a runner report."""
    protocol_of = {leaf['id']: leaf['protocol'] for leaf in tree['leaf']}
    rows = {protocol: {'leaves': 0, 'cases': 0, **{name: 0 for name in COLUMNS}}
            for protocol in sorted(set(protocol_of.values()))}
    for leaf in tree['leaf']:
        rows[leaf['protocol']]['leaves'] += 1
    for result in report['results']:
        row = rows[protocol_of[result['leaf']]]
        row['cases'] += 1
        row[bucket(result['verdict'])] += 1
    return rows


def totals(rows):
    """The totals of the per-protocol counts."""
    total = {'leaves': sum(row['leaves'] for row in rows.values()),
             'cases': sum(row['cases'] for row in rows.values())}
    for name in COLUMNS:
        total[name] = sum(row[name] for row in rows.values())
    return total


def render(rows, total):
    """The status block, from the start marker to the end marker."""
    columns = ('leaves', 'cases') + COLUMNS
    lines = [START, '', '## Conformance status', '',
             f'Oracle: CPython {ORACLE_VERSION}.', '',
             MEASUREMENT, '',
             'Totals: ' + ', '.join(f'{total[name]} {name}' for name in columns) + '.', '',
             '| Protocol | Leaves | Cases | Matching | Differing | Unsupported | Crash | Timeout |',
             '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |']
    for protocol, row in rows.items():
        lines.append('| ' + protocol + ' | ' + ' | '.join(str(row[name]) for name in columns) + ' |')
    lines += ['', 'A differing case is a known mismatch that still has to be fixed; the goal is that '
              'every case matches.', '', END]
    return '\n'.join(lines)


def markers(text):
    """The bounds of the status block in a README text."""
    first = text.find(START)
    if first < 0:
        raise ValueError('README.md has no ' + START + ' marker')
    last = text.find(END, first)
    if last < 0:
        raise ValueError('README.md has no ' + END + ' marker')
    return first, last


def block_of(text):
    """The status block of a README text."""
    first, last = markers(text)
    return text[first:last + len(END)]


def splice(text, block):
    """The README text with the status block replaced."""
    first, last = markers(text)
    return text[:first] + block + text[last + len(END):]


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--report', required=True, help='the report of tests/conformance/run.py')
    parser.add_argument('--check', action='store_true', help='check the block instead of writing it')
    args = parser.parse_args()
    try:
        report = json.loads(Path(args.report).read_text(encoding='utf-8'))
        tree = tomllib.loads(TREE.read_text(encoding='utf-8'))
        rows = counts(report, tree)
        block = render(rows, totals(rows))
        text = README.read_text(encoding='utf-8')
        if args.check:
            if block_of(text) != block:
                print('the conformance status block of README.md is stale; regenerate it with '
                      'status.py --report ' + args.report, file=sys.stderr)
                return 1
            return 0
        README.write_text(splice(text, block), encoding='utf-8')
        return 0
    except (OSError, ValueError, KeyError) as exc:
        print('status error: ' + str(exc), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
