from collections import deque

for args, kwargs in [(([], -1), {}), (([], 1.5), {}), (([], 1, 2), {}),
                     (([],), {'iterable': []}), ((), {'unknown': 1}),
                     ((), {'a': 1, 'b': 2, 'c': 3}), ((1,), {}), ((None,), {})]:
    d = deque([1, 2], 3)
    it = iter(d)
    try: d.__init__(*args, **kwargs)
    except Exception as error: emit_exception(error)
    emit('state', items=list(d), maxlen=d.maxlen)
    try: emit('next', value=next(it, 'end'))
    except Exception as error: emit_exception(error)

def broken():
    yield 7
    yield 8
    raise ValueError('broken iterable')

d = deque([1, 2], 3)
try: d.__init__(broken(), 1)
except Exception as error: emit_exception(error)
emit('state', items=list(d), maxlen=d.maxlen)
