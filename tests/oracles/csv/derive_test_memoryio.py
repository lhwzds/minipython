"""Turn CPython's Lib/test/test_memoryio.py StringIO tests into a standalone program."""
import os, sys
code = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'derive_test_csv.py')).read()
SHIM = code.split("SHIM = r'''")[1].split("'''")[0]
RUNNER = code.split("RUNNER = r'''")[1].split("'''")[0]
src = open(sys.argv[1]).read()
body = src[src.index('class IntLike'):]
body = body.replace('unittest.main()', 'pass')
import re as _re
# Lone surrogates are outside MiniPython's lexer (Rust strings): skip them.
body = _re.sub(r"    def test_lone_surrogates\(self\):\n(?:        .*\n|\n)*", "    def test_lone_surrogates(self):\n        raise SkipTest('lone surrogates')\n\n", body)
RUNNER = RUNNER.replace("if isinstance(v, type) and issubclass(v, TestCase) and v is not TestCase)",
                        "if isinstance(v, type) and issubclass(v, TestCase) and v is not TestCase and 'StringIO' in n and 'Pickle' not in n)")
EXTRA = r'''
import io
pyio = io
class _weakref:
    @staticmethod
    def ref(o): raise SkipTest('weakref')
weakref = _weakref
class _gc:
    @staticmethod
    def collect(): pass
gc = _gc
def _check_sizeof(*a): raise SkipTest('sizeof')
support.check_sizeof = staticmethod(_check_sizeof)
support.calcobjsize = staticmethod(lambda *a: 0)
class _pickle:
    HIGHEST_PROTOCOL = 0
    @staticmethod
    def dumps(*a, **k): raise SkipTest('pickle')
pickle = _pickle
'''
open(sys.argv[2], 'w').write(SHIM + EXTRA + body + RUNNER)
