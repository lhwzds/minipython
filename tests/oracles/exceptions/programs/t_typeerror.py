def check(matcher):
    try:
        try:
            raise ValueError('orig')
        except matcher:
            print('caught')
    except TypeError as error:
        print(type(matcher).__name__, error, repr(error.__context__))
for matcher in (42, 'ValueError', int, object, [ValueError], (ValueError, 3), (TypeError, (ValueError,))):
    check(matcher)
check((KeyError, ValueError))
try:
    try:
        raise ValueError('empty')
    except ():
        print('wrong')
except ValueError as error:
    print('an empty tuple catches nothing', error)
try:
    pass
except 42:
    pass
print('an invalid clause that is never reached is not checked')
try:
    raise ValueError
except ValueError:
    print('an earlier clause matched')
except 42:
    pass
try:
    try:
        raise ValueError('orig')
    except undefined_name:
        pass
except NameError as error:
    print('NameError', error, repr(error.__context__))
def finally_runs():
    try:
        try:
            raise KeyError('k')
        except 42:
            pass
        finally:
            print('finally runs')
    except TypeError as error:
        print('then', error)
finally_runs()
import collections
Pair = collections.namedtuple('Pair', 'a b')
try:
    raise KeyError('nt')
except Pair(ValueError, KeyError) as error:
    print('namedtuple of classes', repr(error))
