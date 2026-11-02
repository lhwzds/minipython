"""Shared prelude for generated re scenario programs (run in CPython and mnpy)."""
PRELUDE = r'''
import re
def show(x):
    if isinstance(x, re.Match):
        return 'M' + repr((x.span(), x.groups(), x.groupdict(), x.lastindex, x.lastgroup, x.regs, x.pos, x.endpos))
    if isinstance(x, re.Pattern):
        return 'P' + repr((x.pattern, x.flags, x.groups, dict(x.groupindex)))
    if isinstance(x, list):
        return '[' + ', '.join([show(i) for i in x]) + ']'
    if isinstance(x, tuple):
        return '(' + ', '.join([show(i) for i in x]) + ')'
    return strip_addr(repr(x))
def strip_addr(s):
    while ' at 0x' in s:
        i = s.index(' at 0x')
        j = s.find('>', i)
        if j < 0:
            break
        s = s[:i] + s[j:]
    return s
def run(i, f):
    try:
        r = f()
        s = show(r)
    except Exception as e:
        s = 'EXC ' + type(e).__name__ + ' ' + strip_addr(str(e))
    print(i, s)
'''
