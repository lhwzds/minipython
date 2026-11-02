"""Hand-written API scenarios (module surface, objects, errors)."""
import sys
from common import PRELUDE
calls = r'''
re.__all__
sorted(n for n in dir(re) if not n.startswith('_'))
re.__version__
(re.I, re.M, re.S, re.X, re.A, re.U, re.L, re.NOFLAG, re.DEBUG)
[int(f) for f in (re.I, re.M, re.S, re.X, re.A, re.U, re.L, re.NOFLAG, re.DEBUG)]
repr(re.I|re.M)
str(re.I|re.M)
repr(re.I|512)
repr(re.RegexFlag(0))
repr(~re.I)
repr(re.I & re.M)
repr(re.I ^ re.I)
re.I.name
(re.I|re.M).name
re.I.value
type(re.I).__name__
type(re.I|re.M).__name__
type(re.I + 1).__name__
re.I + 1
re.I | 1
1 | re.I
isinstance(re.I, int)
re.I == 2
hash(re.I) == hash(2)
{re.I: 'x'}[2]
'%d %s %r' % (re.I, re.I, re.I)
f'{re.I}'
format(re.I, 'd')
re.compile('a', re.I|re.M)
re.compile('a', 2|8)
re.compile(b'a', re.I)
re.compile('a', re.A)
re.compile('a', re.U)
re.compile('a'*300)
re.compile('(?i)x').flags
re.compile('x', re.I).flags
re.compile(b'x').flags
re.compile('x', 512)
re.compile('x', 1)
re.compile('(?x) a b ', 0).flags
re.compile('a', re.L)
re.compile(b'a', re.U)
re.compile(b'a', re.A|re.L)
re.compile('a', re.A|re.U)
re.compile('a', 'x')
re.compile('a', 1.0)
re.compile('a', 2**40)
re.compile('a', -1)
re.compile(b'a', -1)
re.compile(1)
re.compile(bytearray(b'a'))
re.compile(re.compile('a'), re.I)
re.compile(re.compile('a')) is re.compile(re.compile('a'))
re.search(re.compile('a'), 'ba')
re.search(re.compile('a'), 'ba', re.I)
re.compile('a') is re.compile('a')
re.compile('a') == re.compile('a')
re.compile('a') == re.compile('a', re.I)
re.compile('a') != re.compile('b')
re.compile('a') == re.compile(b'a')
re.compile('(?i)a') == re.compile('a', re.I)
hash(re.compile('ab')) == hash(re.compile('ab'))
re.compile('a') < re.compile('a')
re.compile('(a)(?P<n>b)?').groups
re.compile('(a)(?P<n>b)?').groupindex
re.compile('(a)').groupindex
re.compile('(?P<x>a)(?P<y>b)').pattern
re.Pattern
re.Match
re.Pattern[str]
re.Match[bytes]
re.Pattern[str, int]
re.Pattern.__name__
re.Pattern.__qualname__
re.Pattern.__module__
re.Match.__name__
re.Match.__module__
re.error
re.PatternError
re.error is re.PatternError
re.error.__name__
re.error.__module__
re.error.__qualname__
issubclass(re.error, Exception)
type(re.compile('a'))
type(re.match('a', 'a'))
type(re.finditer('a', 'a'))
isinstance(re.compile('a'), re.Pattern)
isinstance(re.match('a', 'a'), re.Match)
isinstance(re.match('a', 'a'), re.Pattern)
re.Pattern()
re.Match()
type('X', (re.Pattern,), {})
type('X', (re.Match,), {})
[x for x in dir(re.compile('a')) if not x.startswith('__')]
[x for x in dir(re.match('a', 'a')) if not x.startswith('__')]
re.compile('a').foo
re.match('a', 'a').foo
setattr(re.compile('a'), 'pattern', 'x')
setattr(re.match('a', 'a'), 'pos', 3)
setattr(re.compile('a'), 'x', 1)
delattr(re.compile('a'), 'flags')
__import__('copy').copy(re.compile('a')) is re.compile('a')
__import__('copy').deepcopy(re.compile('a')) is re.compile('a')
(lambda m: __import__('copy').copy(m) is m)(re.match('a', 'a'))
(lambda m: __import__('copy').deepcopy(m) is m)(re.match('a', 'a'))
(lambda m: m.__copy__() is m)(re.match('a', 'a'))
(lambda m: m.__deepcopy__({}) is m)(re.match('a', 'a'))
re.compile('a').__copy__()
re.compile('a').__deepcopy__(None)
re.match('a', 'a').__copy__(1)
re.match('a', 'a').__deepcopy__()
bool(re.match('', ''))
str(re.compile('a'))
repr(re.match('a'*100, 'a'*100))
repr(re.match(b'\xff', b'\xff'))
repr(re.match('(a)', 'a'))
repr(re.search('b', 'ab'))
len(re.match('a', 'a'))
list(re.match('a', 'a'))
re.match('a', 'a')[0]
re.match('(a)(b)', 'ab')[2]
re.match('(?P<n>a)', 'a')['n']
re.match('a', 'a')[5]
re.compile('a')[0]
re.match('(a)', 'a') == re.match('(a)', 'a')
(lambda m: m == m)(re.match('a', 'a'))
type(hash(re.match('a', 'a')))
re.compile('(a)(?P<n>b)?').search()
re.compile('(a)(?P<n>b)?').search('a', 0, 1, 2)
re.compile('(a)(?P<n>b)?').search(string='xab', pos=1)
re.compile('a').search('a', foo=1)
re.compile('a').search('a', 'x')
re.compile('a').search('a', 1.5)
re.compile('a').search('a', 2**100)
re.compile('a').search('a', -2**100)
re.compile('a').search('aaa', -5)
re.compile('a').search('ba', -5).pos
re.compile('a').search('ba', 0, 100).endpos
re.compile('a').match(b'a')
re.compile('a').match(1)
re.compile(b'a').match('a')
re.compile('a').findall()
re.compile('a').split('a', 1, 2)
re.compile('a').split('a', maxsplit='x')
re.compile('a').sub('x')
re.compile('a').sub('x', 'a', count='1')
re.compile('a').subn('x', 'a', 1, 2)
re.compile('a').search('a', string='b')
re.compile('a').sub('x', 'a', repl='y')
re.compile('').match('abc', 2, 1)
re.compile('').search('abc', 2, 1)
re.compile('').findall('abc', 2, 1)
re.compile('$').match('abc', 2, 1)
re.compile('a').fullmatch('ab', 0, 1)
re.compile('a$').search('ab', 0, 1)
re.compile(r'\b').search('ab', 1, 1)
re.compile('^b').search('ab', 1)
re.compile('(?m)^b').search('a\nb', 2)
re.compile(r'\Ab').search('ab', 1)
re.compile(r'(?<=a)b').search('ab', 1)
re.compile(r'b(?=c)').search('abc', 0, 2)
re.compile('(a)(?P<n>b)?').match('ab').group(3)
re.compile('(a)(?P<n>b)?').match('ab').group(-1)
re.compile('(a)(?P<n>b)?').match('ab').group('zz')
re.compile('(a)(?P<n>b)?').match('ab').group(1.0)
re.compile('(a)(?P<n>b)?').match('ab').group(None)
re.compile('(a)(?P<n>b)?').match('ab').group(True)
re.compile('(a)(?P<n>b)?').match('ab').group([1])
re.compile('(a)(?P<n>b)?').match('ab').group(0, 1, 'n')
re.compile('(a)(?P<n>b)?').match('ab').groups(1, 2)
re.compile('(a)(?P<n>b)?').match('a').groups(default=5)
re.compile('(a)(?P<n>b)?').match('a').groups('x')
re.compile('(a)(?P<n>b)?').match('a').groupdict('x')
re.compile('(a)(?P<n>b)?').match('ab').groupdict(x=1)
re.compile('(a)(?P<n>b)?').match('ab').groups(default=1, x=2)
re.compile('(a)(?P<n>b)?').match('ab').start(5)
re.compile('(a)(?P<n>b)?').match('ab').span('n')
re.compile('(a)(?P<n>b)?').match('a').span('n')
re.compile('(a)(?P<n>b)?').match('a').start(2)
re.compile('(a)(?P<n>b)?').match('ab').span(1, 2)
re.compile('(a)(?P<n>b)?').match('ab').span(group=1)
re.compile('(a)(?P<n>b)?').match('ab').start(1, 2)
re.compile('(a)(?P<n>b)?').match('ab').end(group=1)
re.compile('(a)(?P<n>b)?').match('ab').group(group=1)
re.compile('(a)(?P<n>b)?').match('ab').expand()
re.compile('(a)(?P<n>b)?').match('ab').expand('a', 'b')
re.compile('(a)(?P<n>b)?').match('ab').expand(template=r'\1')
re.compile('(a)(?P<n>b)?').match('ab').expand(r'<\g<n>\1\g<0>>')
re.compile('(a)(?P<n>b)?').match('ab').__getitem__()
re.compile('(a)(?P<n>b)?').match('ab').lastgroup
re.compile('(a)(?P<n>b)?').match('a').lastgroup
re.compile('(?P<x>a)(?P<n>b)?').match('a').lastgroup
re.compile('((a)b)').match('ab').lastindex
re.compile('(a)|(b)').match('b').lastindex
re.compile('(a)|(b)').match('b').regs
re.compile('a').match('a').re
re.compile('a').match('xa', 1).string
re.match(b'a', bytearray(b'ab')).group()
re.match(b'a', bytearray(b'ab')).string
re.match(b'a', memoryview(b'ab')).group()
re.match(b'a', __import__('array').array('b', b'ab')).group()
re.match(b'\x01', __import__('array').array('I', [1])).group()
type(re.match(b'a', bytearray(b'ab')).group())
re.escape('a.b*c\n \t#&~-é')
re.escape(b'a.b*\xff')
re.escape(bytearray(b'a.b'))
re.escape(memoryview(b'a.b'))
re.escape(5)
re.escape(''.join(chr(i) for i in range(128)))
re.purge()
re.purge(1)
re.search('a')
re.search('a', 'b', 0, 1)
re.sub('a', 'b')
re.sub('a', 'b', 'c', 1, 0, 9)
re.sub('a', 'b', 'c', 1, count=2)
re.sub('a', 'b', 'c', 1, 0, flags=2)
re.split('a', 'b', 1, 0, 9)
re.split('a', 'b', 1, maxsplit=2)
re.compile('a', flag=2)
re.split('a', 'b', maxsplits=2)
re.findall(pattern='a', string='aa', flags=0)
re.sub(pattern='a', repl='b', string='aaa', count=1)
re.sub('a', 'b', 'aaa', 1)
re.subn('a', 'b', 'aaa', 2)
re.split('a', 'bab', 1)
re.sub('a', 'b', 'aaa', count=-1)
re.split('a', 'bab', maxsplit=-1)
re.subn('a', 'b', 'aaa', count=-5)
re.sub('a', 5, 'aaa')
re.sub('a', b'x', 'aaa')
re.sub('a', b'\\n', 'aaa')
re.sub(b'a', 'x', b'aaa')
re.sub(b'a', '\\n', b'aaa')
re.sub(b'(a)', '\\1', b'aaa')
re.sub('(a)', b'\\1', 'aaa')
re.sub('a', lambda m: 5, 'aaa')
re.sub('a', lambda m: None, 'aaba')
re.sub(b'a', lambda m: bytearray(b'xy'), b'aaba')
re.sub(b'a', lambda m: memoryview(b'z'), b'aaba')
re.sub(b'a', lambda m: 'z', b'aaba')
re.sub('a', lambda m: 1/0, 'a')
re.sub('a', str.upper, 'banana') if False else re.sub('a', lambda m: m.group().upper(), 'banana')
re.sub('x', 'y', 'aaa')
re.sub('(a)', r'\g<1>\g<0>\1\\', 'a')
re.sub('(a)', r'\2', 'a')
re.sub('(a)', r'\g<x>', 'a')
re.sub('(a)', r'\g<1', 'a')
re.sub('(a)', r'\q', 'a')
re.sub('(a)', r'\&\é', 'a')
re.sub('(a)', r'\101\0\08\1a', 'a')
re.sub('(a)', r'\777', 'a')
re.sub('(a)|b', r'[\1]', 'ab')
re.match('(a)|b', 'b').expand(r'[\1]')
re.sub('(a)', r'\g<0>\g< 1>', 'a')
re.sub('(a)', r'\g<1a>', 'a')
re.sub('(a)', r'\g<99999999999>', 'a')
re.sub('(a)', r'\N{EM DASH}', 'a')
re.sub('(a)', r'\x41', 'a')
re.sub(b'(a)', rb'\1\n\x', b'a')
re.sub(b'(?P<a>a)', b'\\g<\xe9>', b'a')
re.match(b'(a)', b'a').expand(b'<\\1>')
re.match(b'(a)', b'a').expand('<\\1>')
re.match('(a)', 'a').expand(b'<\\1>')
re.match('(a)', 'a').expand(5)
re.sub('a', '\\', 'a')
re.sub('a', 'x\\', 'a')
re.sub('(?P<a>x)', '\\g<a>\\g<a>', 'xx')
re.sub('a', r'\t\n\v\r\f\a\b\\', 'a')
re.subn('', '-', 'abc')
re.sub('x*', '-', 'abxd')
re.sub('x*', '-', 'abxd', count=2)
re.split('x*', 'axbc')
re.split('(x*)', 'axbc')
re.split(r'\b', 'a b')
re.split('', 'ab')
re.findall('', 'ab')
re.findall('a|', 'ab')
re.findall(r'(a)|b', 'ab')
re.findall(r'(a)(b)?', 'aab')
[m.span() for m in re.finditer('x*', 'axxb')]
list(re.finditer('a', 'aaa', 1)) if False else [m.span() for m in re.compile('a').finditer('aaaa', 1, 3)]
[m.pos for m in re.compile('a').finditer('aaaa', 1, 3)]
next(iter(re.finditer('a', 'a'))).group()
re.compile('(a)').scanner('a').match().group()
re.compile('(a)').scanner('aa').search().span()
re.fullmatch('a|ab', 'ab')
re.fullmatch('a*?', 'aa')
re.fullmatch(r'(?:a+?)?', 'aa')
re.fullmatch('a', 'ab')
re.match('a*+a', 'aaa')
re.match('(?>a*)a', 'aaa')
re.match('(?:a|b)*+c', 'ababc')
re.match('a{2,3}+a', 'aaaa')
'''.strip().split('\n')
out = sys.argv[1]
lines = [PRELUDE]
for i, c in enumerate(calls):
    lines.append('run(%d, lambda: %s)' % (i, c))
open(out, 'w').write('\n'.join(lines) + '\n')
open(out.replace('.py', '.src'), 'w').write('\n'.join('%d %s' % (i, c) for i, c in enumerate(calls)) + '\n')
print(len(calls), 'scenarios')
