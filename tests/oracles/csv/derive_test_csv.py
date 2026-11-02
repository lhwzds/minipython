"""Turn CPython's Lib/test/test_csv.py into a standalone program (outside the repo).

The test classes run against a tiny unittest shim; each test method prints
PASS/FAIL/SKIP so CPython and mnpy can be compared method by method."""
import re, sys
src = open(sys.argv[1]).read()
out = sys.argv[2]
header_end = src.index('class BadIterable')
body = src[header_end:]
body = body.replace('unittest.main()', 'pass')
SHIM = r'''
import sys, csv, io, copy
from io import StringIO
from collections import OrderedDict
import itertools
from itertools import permutations
class SkipTest(Exception): pass
class _Raises:
    def __init__(self, exc, regex=None): self.exc = exc; self.regex = regex; self.exception = None
    def __enter__(self): return self
    def __exit__(self, t, v, tb):
        if t is None: raise AssertionError('did not raise %r' % (self.exc,))
        if not issubclass(t, self.exc): return False
        if self.regex is not None:
            import re
            if not re.search(self.regex, str(v)): raise AssertionError('regex %r not in %r' % (self.regex, str(v)))
        self.exception = v
        return True
class _Sub:
    def __enter__(self): return self
    def __exit__(self, *a): return False
class TestCase:
    maxDiff = None
    def assertEqual(self, a, b, msg=None):
        if not a == b: raise AssertionError('%r != %r' % (a, b))
    assertEquals = assertEqual
    def assertNotEqual(self, a, b, msg=None):
        if a == b: raise AssertionError('%r == %r' % (a, b))
    def assertTrue(self, a, msg=None):
        if not a: raise AssertionError('not true: %r' % (a,))
    def assertFalse(self, a, msg=None):
        if a: raise AssertionError('not false: %r' % (a,))
    def assertIs(self, a, b, msg=None):
        if a is not b: raise AssertionError('%r is not %r' % (a, b))
    def assertIsNot(self, a, b, msg=None):
        if a is b: raise AssertionError('%r is %r' % (a, b))
    def assertIsNone(self, a, msg=None): self.assertIs(a, None)
    def assertHasAttr(self, o, n, msg=None):
        if not hasattr(o, n): raise AssertionError('no attr %r' % (n,))
    def assertNotHasAttr(self, o, n, msg=None):
        if hasattr(o, n): raise AssertionError('has attr %r' % (n,))
    def assertLess(self, a, b, msg=None):
        if not a < b: raise AssertionError('%r >= %r' % (a, b))
    def assertIsInstance(self, a, b, msg=None):
        if not isinstance(a, b): raise AssertionError('%r not instance of %r' % (a, b))
    def assertIn(self, a, b, msg=None):
        if a not in b: raise AssertionError('%r not in %r' % (a, b))
    def assertNotIn(self, a, b, msg=None):
        if a in b: raise AssertionError('%r in %r' % (a, b))
    def assertGreater(self, a, b, msg=None):
        if not a > b: raise AssertionError('%r <= %r' % (a, b))
    def assertRaises(self, exc, *args, **kwargs):
        ctx = _Raises(exc)
        if not args: return ctx
        with ctx: args[0](*args[1:], **kwargs)
    def assertRaisesRegex(self, exc, regex, *args, **kwargs):
        ctx = _Raises(exc, regex)
        if not args: return ctx
        with ctx: args[0](*args[1:], **kwargs)
    def subTest(self, *a, **k): return _Sub()
    def addCleanup(self, f, *a, **k):
        self.__dict__.setdefault('_cleanups', []).append((f, a, k))
    def skipTest(self, why): raise SkipTest(why)
    def setUp(self): pass
    def tearDown(self): pass
class _unittest:
    TestCase = TestCase
    SkipTest = SkipTest
    @staticmethod
    def skipUnless(cond, why):
        return (lambda f: f) if cond else (lambda f: None)
    @staticmethod
    def skipIf(cond, why):
        return (lambda f: None) if cond else (lambda f: f)
unittest = _unittest
def TemporaryFile(mode='w+', encoding=None, newline=None):
    return StringIO(newline=newline)
def cpython_only(f): return None
class _support:
    cpython_only = staticmethod(cpython_only)
    @staticmethod
    def gc_collect(): pass
    @staticmethod
    def run_with_locale(*a, **k): return lambda f: None
    @staticmethod
    def check__all__(*a, **k): raise SkipTest('check__all__')
    @staticmethod
    def requires_resource(*a): return lambda f: None
support = _support
def check_disallow_instantiation(test, tp, *args, **kwds):
    try:
        tp(*args, **kwds)
    except TypeError:
        return
    raise AssertionError('instantiation allowed')
class import_helper:
    @staticmethod
    def import_module(name): raise SkipTest(name)
def ensure_lazy_imports(*a): raise SkipTest('lazy imports')
def dedent(text):
    lines = text.split('\n')
    margin = None
    for line in lines:
        if line.strip():
            indent = len(line) - len(line.lstrip())
            margin = indent if margin is None else min(margin, indent)
    margin = margin or 0
    return '\n'.join(line[margin:] if line.strip() else line.lstrip(' \t') for line in lines)
'''
RUNNER = r'''
def _run():
    names = sorted(n for n, v in list(globals().items()) if isinstance(v, type) and issubclass(v, TestCase) and v is not TestCase)
    for name in names:
        cls = globals()[name]
        for attr in sorted(dir(cls)):
            if not attr.startswith('test'):
                continue
            method = getattr(cls, attr, None)
            if method is None or not callable(method):
                continue
            inst = cls()
            try:
                inst.setUp()
                try:
                    method(inst)
                finally:
                    for f, a, k in reversed(inst.__dict__.get('_cleanups', [])):
                        f(*a, **k)
                print(name + '.' + attr, 'PASS')
            except SkipTest as e:
                print(name + '.' + attr, 'SKIP')
            except BaseException as e:
                print(name + '.' + attr, 'FAIL', type(e).__name__, str(e)[:200])
_run()
'''
open(out, 'w').write(SHIM + body + RUNNER)
