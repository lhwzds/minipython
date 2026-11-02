E = ValueError
try:
    raise ValueError('x')
except E as error:
    print('alias', type(error).__name__, error)
errors = (ValueError, KeyError)
for exc in (ValueError('a'), KeyError('b'), IndexError('c')):
    try:
        try:
            raise exc
        except errors as error:
            print('tuple variable', type(error).__name__)
    except IndexError as error:
        print('outer', type(error).__name__)
class Case:
    failureException = AssertionError
    def run(self):
        try:
            assert False, 'boom'
        except self.failureException as error:
            print('attribute', error)
Case().run()
import types
mod = types.SimpleNamespace(Error=LookupError)
try:
    {}['k']
except mod.Error as error:
    print('namespace attribute', repr(error))
def matcher(label, cls):
    print('evaluating', label)
    return cls
try:
    1 / 0
except matcher('first', KeyError):
    print('wrong')
except matcher('second', ZeroDivisionError) as error:
    print('call result', error)
except matcher('third', Exception):
    print('wrong')
try:
    pass
except matcher('unused', ValueError):
    print('wrong')
print('no evaluation without an exception')
try:
    try:
        raise KeyError('k')
    except Exception as error:
        first = error
        raise
except type(first) as error:
    print('type(e)', error is first)
try:
    raise E
except E as error:
    print('raise alias class', repr(error))
try:
    raise E('a') from KeyError('b')
except E as error:
    print('raise from', repr(error.__cause__), error.__suppress_context__)
try:
    raise E('direct')
except ValueError as error:
    print(isinstance(error, E), issubclass(type(error), E), type(error) is E)
