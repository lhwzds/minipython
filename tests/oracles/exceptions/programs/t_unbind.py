def probe(label, getter):
    try:
        print(label, repr(getter()))
    except NameError as error:
        print(label, type(error).__name__, error)
try:
    raise ValueError('v')
except ValueError as error:
    pass
probe('after handler', lambda: error)
error = 'kept'
try:
    pass
except ValueError as error:
    pass
probe('no exception keeps', lambda: error)
try:
    raise KeyError('k')
except KeyError as error:
    del error
print('del inside handler is fine')
def from_return():
    global seen
    try:
        raise ValueError('r')
    except ValueError as seen:
        return 'returned'
print(from_return())
probe('after return', lambda: seen)
def from_raise():
    global raised
    try:
        try:
            raise ValueError('a')
        except ValueError as raised:
            raise KeyError('b')
    except KeyError as other:
        return repr(other.__context__)
print(from_raise())
probe('after raise', lambda: raised)
def nonlocal_bind():
    target = 'before'
    def inner():
        nonlocal target
        try:
            raise KeyError(1)
        except KeyError as target:
            pass
    inner()
    try:
        return target
    except NameError as error:
        return type(error).__name__
print(nonlocal_bind())
for attempt in range(3):
    try:
        raise ValueError(attempt)
    except ValueError as loop_error:
        if attempt == 1:
            continue
        if attempt == 2:
            break
probe('after loop', lambda: loop_error)
try:
    raise ExceptionGroup('eg', [ValueError(1)])
except* ValueError as group:
    print('star', repr(group))
probe('after except*', lambda: group)
class Holder:
    try:
        raise ValueError('class body')
    except ValueError as caught:
        message = str(caught)
print(Holder.message, hasattr(Holder, 'caught'))
