E = KeyError
def f(n):
    for i in range(n):
        try:
            if i == 1:
                raise E(i)
            if i == 2:
                raise ValueError(i)
            if i == 3:
                break
        except E:
            print('E at', i)
            continue
        except (ValueError,) as error:
            print('V at', i, error)
            return 'returned'
        else:
            print('else', i)
        finally:
            print('finally', i)
    return 'loop done'
print(f(2))
print(f(4))
def g():
    for i in range(3):
        try:
            if i == 0:
                continue
            if i == 1:
                break
        except ValueError:
            print('stale handler')
    try:
        raise ValueError('after loop')
    except KeyError:
        print('wrong')
try:
    g()
except ValueError as error:
    print('propagated', error)
def h():
    try:
        for i in range(2):
            break
        print('after loop')
    finally:
        print('outer finally once')
    try:
        for i in range(2):
            try:
                continue
            finally:
                print('inner finally', i)
        return 'value'
    finally:
        print('outer finally')
print(h())
def handler_raises():
    try:
        try:
            raise ValueError('a')
        except ValueError:
            raise KeyError('b')
        finally:
            print('finally after handler raise')
    except KeyError as error:
        print('outer', repr(error), repr(error.__context__))
handler_raises()
def finally_nested_handler():
    try:
        try:
            raise ValueError('v')
        finally:
            try:
                raise KeyError('k')
            except KeyError:
                pass
    except ValueError as error:
        print('finally re-raises', error)
finally_nested_handler()
def gen():
    try:
        yield 1
    except E as error:
        print('generator caught', repr(error))
        yield 2
g = gen()
next(g)
print(g.throw(KeyError('thrown')))
class Manager:
    def __enter__(self):
        return self
    def __exit__(self, kind, value, tb):
        print('exit', kind is E, isinstance(value, E))
        return kind is not None and issubclass(kind, E)
with Manager():
    raise KeyError('suppressed')
print('after with')
try:
    try:
        raise ValueError(1)
    except ValueError as error:
        raise error
except ValueError as error:
    print('raise handled exception', error.__context__)
