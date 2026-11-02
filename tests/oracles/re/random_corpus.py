"""Generate randomized re differential scenarios.

usage: random_corpus.py SEED COUNT OUT.py [mode]
modes: ascii (default), unicode, bytes, errors
Each scenario is one call; the .src file records the call source.
"""
import random, sys
from common import PRELUDE

seed, count, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
mode = sys.argv[4] if len(sys.argv) > 4 else 'ascii'
rng = random.Random(seed)

ALPHA = {'ascii': 'abcab \n1_.-', 'unicode': 'aAbBsSſkKKİıiIσςΣéÉßẞµΜ 1٣²_\n', 'bytes': 'abcAB \n1_\xe9\xff.-', 'errors': 'abc'}[mode]
LIT_ALPHA = {'ascii': 'abc', 'unicode': 'aAsSſkKKİıiσςΣéßµ', 'bytes': 'abAB\xe9', 'errors': 'ab'}[mode]

class Gen:
    def __init__(self):
        self.groups = 0
        self.names = []
        self.depth = 0

    def atom(self, fixed=False):
        r = rng.random()
        if self.depth > 3:
            r = r * 0.45
        if r < 0.30:
            c = rng.choice(LIT_ALPHA)
            return c
        if r < 0.36:
            return '.'
        if r < 0.46:
            choices = ['[abc]', '[^a]', '[a-c]', '[^\\n]', '[\\w]', '[\\d\\s]', '[^\\W]', '[a-z]', '[A-Z]', '[.^$]', '[-a]', '[]a]', '[\\]]', '[k-m]', '[^b-c]']
            if mode == 'unicode':
                choices += ['[sσ]', '[^ſ]', '[ä-ü]', '[Σ-ω]', '[\\u0100-\\u017f]', '[K]', '[ı-ſ]']
            if mode == 'bytes':
                choices += ['[\\xe0-\\xff]', '[^\\x80-\\xff]']
            return rng.choice(choices)
        if r < 0.54:
            return rng.choice(['\\d', '\\w', '\\s', '\\D', '\\W', '\\S'])
        if r < 0.58 and not fixed:
            return rng.choice(['\\b', '\\B', '\\A', '\\Z', '\\z', '^', '$'])
        if r < 0.70:
            self.depth += 1
            kind = rng.random()
            if kind < 0.45:
                self.groups += 1
                body = self.alt(fixed)
                s = '(' + body + ')'
            elif kind < 0.6:
                self.groups += 1
                name = 'n%d' % self.groups
                self.names.append(name)
                s = '(?P<%s>%s)' % (name, self.alt(fixed))
            elif kind < 0.75:
                s = '(?:' + self.alt(fixed) + ')'
            elif kind < 0.82:
                s = '(?>' + self.alt(fixed) + ')'
            elif kind < 0.9:
                s = '(?%s:%s)' % (rng.choice(['i', 's', 'm', '-i', 'i-s', 'x']), self.alt(fixed))
            else:
                s = '(?' + rng.choice(['=', '!']) + self.alt(False) + ')'
            self.depth -= 1
            return s
        if r < 0.76 and not fixed:
            self.depth += 1
            s = '(?' + rng.choice(['<=', '<!']) + self.seq_fixed() + ')'
            self.depth -= 1
            return s
        if r < 0.82 and self.groups and not fixed:
            g = rng.randint(1, self.groups)
            if self.names and rng.random() < 0.3:
                return '(?P=%s)' % rng.choice(self.names)
            return '\\%d' % g
        if r < 0.86 and self.groups and not fixed:
            g = rng.randint(1, self.groups)
            self.depth += 1
            s = '(?(%d)%s|%s)' % (g, self.seq(), self.seq()) if rng.random() < 0.7 else '(?(%d)%s)' % (g, self.seq())
            self.depth -= 1
            return s
        return rng.choice(LIT_ALPHA)

    def quant(self, fixed=False):
        if fixed:
            return rng.choice(['', '', '', '{2}', '{1}'])
        r = rng.random()
        if r < 0.55:
            return ''
        q = rng.choice(['*', '+', '?', '{2}', '{1,2}', '{0,}', '{,2}', '{2,3}', '{0}'])
        mod = rng.random()
        if mod < 0.2:
            q += '?'
        elif mod < 0.3:
            q += '+'
        return q

    def seq(self, fixed=False):
        n = rng.choice([1, 1, 2, 2, 3, 4])
        out = ''
        for _ in range(n):
            a = self.atom(fixed)
            if a in ('\\b', '\\B', '\\A', '\\Z', '\\z', '^', '$') or a.startswith('(?=') or a.startswith('(?!') or a.startswith('(?<'):
                out += a
            else:
                out += a + self.quant(fixed)
        return out

    def seq_fixed(self):
        saved = self.groups
        s = self.seq(fixed=True)
        return s

    def alt(self, fixed=False):
        if fixed:
            return self.seq(True)
        n = rng.choice([1, 1, 1, 2, 3])
        return '|'.join(self.seq() for _ in range(n))

def subject():
    n = rng.choice([0, 1, 2, 3, 4, 5, 6, 8, 10, 12])
    return ''.join(rng.choice(ALPHA) for _ in range(n))

def lit(s, is_bytes):
    if is_bytes:
        return repr(s.encode('latin-1'))
    return repr(s)

def pattern_text():
    g = Gen()
    p = g.alt()
    if mode == 'errors':
        # mutate to produce invalid patterns
        ops = ['(', ')', '[', ']', '{', '}', '?', '*', '+', '\\', '|', '(?', '(?P', '(?<', '(?P<', '\\g', '\\x', '\\u12', '{2,1}', '(?i', '(?-', '(?ai)', '(?L)', '(?#', '[z-a]', '\\8', '\\N{', '(?P=x)', '(?(', '(?(0)', 'x{99999999999}', '(?<=a*)']
        for _ in range(rng.randint(1, 2)):
            i = rng.randint(0, len(p))
            p = p[:i] + rng.choice(ops) + p[i:]
    return p, g

FLAGS = ['0', 're.I', 're.M', 're.S', 're.I|re.M', 're.X', 're.A', 're.I|re.A', 're.S|re.M']
lines = [PRELUDE]
srcs = []
for i in range(count):
    p, g = pattern_text()
    is_bytes = mode == 'bytes'
    if is_bytes:
        p = p.replace('\\z', '\\Z')
    flags = rng.choice(FLAGS)
    if is_bytes and 'A' in flags and rng.random() < 0.5:
        flags = rng.choice(['0', 're.I', 're.L', 're.L|re.I'])
    s = subject()
    P = lit(p, is_bytes)
    S = lit(s, is_bytes)
    r = rng.random()
    if r < 0.2:
        call = 're.search(%s, %s, %s)' % (P, S, flags)
    elif r < 0.32:
        call = 're.match(%s, %s, %s)' % (P, S, flags)
    elif r < 0.42:
        call = 're.fullmatch(%s, %s, %s)' % (P, S, flags)
    elif r < 0.55:
        call = 're.findall(%s, %s, %s)' % (P, S, flags)
    elif r < 0.62:
        call = 'list(re.finditer(%s, %s, %s))' % (P, S, flags)
    elif r < 0.72:
        tpl = rng.choice(['<\\g<0>>', '-', '', '[\\1]' if g.groups else '[\\g<0>]', '\\g<%d>' % g.groups if g.groups else 'x'])
        call = 're.sub(%s, %s, %s, flags=%s)' % (P, lit(tpl, is_bytes), S, flags)
    elif r < 0.77:
        call = 're.subn(%s, %s, %s, count=%d, flags=%s)' % (P, lit('#', is_bytes), S, rng.randint(0, 3), flags)
    elif r < 0.85:
        call = 're.split(%s, %s, maxsplit=%d, flags=%s)' % (P, S, rng.choice([0, 0, 1, 2]), flags)
    elif r < 0.93:
        pos, end = rng.randint(0, 4), rng.randint(0, 12)
        meth = rng.choice(['search', 'match', 'fullmatch', 'findall'])
        call = 're.compile(%s, %s).%s(%s, %d, %d)' % (P, flags, meth, S, pos, end)
    else:
        call = 're.sub(%s, lambda m: str(m.span()) if not isinstance(m.string, bytes) else bytes(str(m.span()), "ascii"), %s, flags=%s)' % (P, S, flags)
    lines.append('run(%d, lambda: %s)' % (i, call))
    srcs.append('%d %s' % (i, call.replace('\n', '\\n')))
open(out, 'w').write('\n'.join(lines) + '\n')
open(out.replace('.py', '.src'), 'w').write('\n'.join(srcs) + '\n')
