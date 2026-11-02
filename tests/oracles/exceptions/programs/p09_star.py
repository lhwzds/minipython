E = ValueError
errs = (TypeError, KeyError)
try:
    raise ExceptionGroup("eg", [ValueError(1), TypeError(2), KeyError(3), OSError(4)])
except* E as g:
    print("star E", repr(g))
except* errs as g:
    print("star errs", repr(g))
except* OSError as g:
    print("star os", repr(g))
eg = ExceptionGroup("eg", [ValueError(1), TypeError(2)])
m, r = eg.split(E)
print(repr(m), repr(r))
print(repr(eg.subgroup(errs)))
class error(Exception): pass
import re
eg2 = ExceptionGroup("x", [error("u"), re.error("r")])
print(repr(eg2.split(re.error)))
try:
    raise ExceptionGroup("eg", [ValueError(1)])
except* 42:
    pass
