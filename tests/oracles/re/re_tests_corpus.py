"""Scenarios from CPython's Lib/test/re_tests.py table (external oracle input).

usage: re_tests_corpus.py STDLIB OUT.py

STDLIB is the pinned CPython's standard library directory. The table is read
from STDLIB/test/re_tests.py at gate time; nothing extracted is committed.
"""
import os, sys
from common import PRELUDE
sys.path.insert(0, os.path.join(sys.argv[1], 'test'))
import re_tests
out = sys.argv[2]
lines = [PRELUDE]
srcs = []
i = 0
def add(call):
    global i
    lines.append('run(%d, lambda: %s)' % (i, call))
    srcs.append('%d %s' % (i, call.replace('\n', '\\n')))
    i += 1
for t in re_tests.tests:
    pattern, s = t[0], t[1]
    add('re.search(%r, %r)' % (pattern, s))
    add('re.findall(%r, %r)' % (pattern, s))
    add('re.search(%r, %r, re.I)' % (pattern, s.upper()))
    add('re.fullmatch(%r, %r)' % (pattern, s))
    add('re.sub(%r, "<\\\\g<0>>", %r)' % (pattern, s))
    add('re.split(%r, %r)' % (pattern, s))
    try:
        bp, bs = pattern.encode('latin-1'), s.encode('latin-1')
        add('re.search(%r, %r)' % (bp, bs))
        add('re.search(%r, %r, re.I)' % (bp, bs.upper()))
    except UnicodeEncodeError:
        pass
    add('re.compile(%r).search(%r, 1)' % (pattern, s))
    add('re.compile(%r).search(%r, 0, max(0, len(%r) - 1))' % (pattern, s, s))
open(out, 'w').write('\n'.join(lines) + '\n')
open(out.replace('.py', '.src'), 'w').write('\n'.join(srcs) + '\n')
print(i, 'scenarios')
