"""Randomized scenarios stressing captures under repetition and backtracking."""
import random, sys
from common import PRELUDE
seed, count, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
rng = random.Random(seed)

def gen(depth, groups):
    if depth <= 0 or rng.random() < 0.25:
        return rng.choice(['a', 'b', 'ab', 'a?', 'b*', '', '.', '[ab]', 'a|b', 'aa'])
    k = rng.random()
    inner = lambda: gen(depth - 1, groups)
    if k < 0.3:
        groups[0] += 1
        body = '(' + inner() + ')'
    elif k < 0.45:
        body = '(?:' + inner() + '|' + inner() + ')'
    elif k < 0.55:
        groups[0] += 1
        body = '(' + inner() + '|' + inner() + ')'
    elif k < 0.62:
        body = '(?>' + inner() + ')'
    elif k < 0.68:
        body = '(?=' + inner() + ')'
    elif k < 0.72:
        body = '(?!' + inner() + ')'
    elif k < 0.78 and groups[0]:
        body = '(?(%d)%s|%s)' % (rng.randint(1, groups[0]), inner(), inner())
    elif k < 0.84 and groups[0]:
        body = '\\%d' % rng.randint(1, groups[0])
    else:
        body = inner() + inner()
    q = rng.choice(['', '', '*', '+', '?', '*?', '+?', '??', '{1,2}', '{2}', '{0,2}?', '*+', '++', '?+', '{1,3}+'])
    if body.startswith('\\') or body.startswith('(?=') or body.startswith('(?!') or body.startswith('(?('):
        q = ''
    return body + q

import re, signal
class Slow(Exception):
    pass
def _alarm(*_):
    raise Slow()
signal.signal(signal.SIGALRM, _alarm)
def quick(call):
    signal.setitimer(signal.ITIMER_REAL, 0.2)
    try:
        eval(call)
    except Slow:
        return False
    except Exception:
        pass
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    return True
lines = [PRELUDE]
srcs = []
i = 0
while i < count:
    groups = [0]
    p = ''.join(gen(rng.randint(1, 4), groups) for _ in range(rng.randint(1, 3)))
    if rng.random() < 0.15:
        p = '(?x) ' + ' '.join(p) + '  # comment'
    s = ''.join(rng.choice('aab') for _ in range(rng.randint(0, 9)))
    op = rng.random()
    if op < 0.35:
        call = 're.match(%r, %r)' % (p, s)
    elif op < 0.55:
        call = 're.search(%r, %r)' % (p, s)
    elif op < 0.7:
        call = 're.fullmatch(%r, %r)' % (p, s)
    elif op < 0.85:
        call = 're.findall(%r, %r)' % (p, s)
    else:
        call = '[m.span() + m.groups() for m in re.finditer(%r, %r)]' % (p, s)
    if not quick(call):
        continue
    lines.append('run(%d, lambda: %s)' % (i, call))
    srcs.append('%d %s' % (i, call))
    i += 1
open(out, 'w').write('\n'.join(lines) + '\n')
open(out.replace('.py', '.src'), 'w').write('\n'.join(srcs) + '\n')
