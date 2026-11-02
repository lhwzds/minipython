E = ValueError
errs = (TypeError, KeyError)
try:
    raise ExceptionGroup('eg', [ValueError(1), TypeError(2), KeyError(3), OSError(4)])
except* E as group:
    print('alias', repr(group))
except* errs as group:
    print('tuple', repr(group))
except* OSError as group:
    print('os', repr(group))
def matcher(label, cls):
    print('evaluating', label)
    return cls
try:
    raise ExceptionGroup('eg', [ValueError(1)])
except* matcher('first', ValueError):
    print('first matched')
except* matcher('second', TypeError):
    print('wrong')
def run(exc, matcher):
    try:
        try:
            raise exc
        except* matcher as group:
            print('match', repr(group))
    except BaseException as rest:
        print('rest', repr(rest))
nested = ExceptionGroup('t', [ValueError(1), ExceptionGroup('in', [TypeError(2), ValueError(3)])])
run(nested, ValueError)
run(nested, (TypeError, ValueError))
run(ValueError('naked'), ValueError)
run(ValueError('naked'), TypeError)
run(KeyboardInterrupt('ki'), KeyboardInterrupt)
run(BaseExceptionGroup('beg', [KeyboardInterrupt(1), ValueError(2)]), ValueError)
class error(Exception):
    pass
import re
run(ExceptionGroup('same', [error('user'), re.error('re')]), re.error)
run(ExceptionGroup('same', [error('user'), re.error('re')]), error)
# Re-raising and raising inside except* clauses is covered by
# t_except_star_groups.py.
def bad(matcher):
    try:
        try:
            raise ExceptionGroup('eg', [ValueError(1)])
        except* ValueError:
            print('handled')
        except* matcher:
            print('wrong')
    except TypeError as caught:
        print('TypeError', caught, repr(caught.__context__))
bad(42)
bad(ExceptionGroup)
bad((ValueError, BaseExceptionGroup))
bad((ValueError, (TypeError,)))
def with_finally(exc):
    try:
        try:
            if exc:
                raise exc
        except* ValueError as group:
            print('handled', repr(group))
        else:
            print('else')
        finally:
            print('finally')
    except BaseException as rest:
        print('rest', repr(rest))
with_finally(None)
with_finally(ExceptionGroup('g', [ValueError(1), KeyError(2)]))
with_finally(KeyError(4))
for attempt in range(2):
    try:
        raise ExceptionGroup('loop', [ValueError(attempt)])
    except* ValueError as group:
        print('loop', repr(group))
eg = ExceptionGroup('eg', [ValueError(1), TypeError(2), ExceptionGroup('in', [ValueError(3)])])
print(repr(eg.split(E)))
print(repr(eg.split(errs)))
print(repr(eg.subgroup(TypeError)), repr(eg.subgroup(KeyError)))
print(eg.subgroup(Exception) is eg)
print(repr(eg.split(lambda exc: isinstance(exc, ValueError) and exc.args == (3,))))
print(repr(eg.derive([KeyError(5)])))
for matcher in (42, (ValueError, (TypeError,)), ValueError(1)):
    try:
        eg.split(matcher)
    except TypeError as caught:
        print('split TypeError', caught)
print(repr(BaseExceptionGroup('x', [ValueError(1)])), type(BaseExceptionGroup('x', [KeyboardInterrupt()])).__name__)
try:
    ExceptionGroup('x', [KeyboardInterrupt()])
except TypeError as caught:
    print(caught)
