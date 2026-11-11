from collections import deque

init = getattr(deque, '__init__')
emit('descriptor', text=repr(init), type=type(init).__name__, name=init.__name__,
     qualname=init.__qualname__, owner=init.__objclass__ is deque,
     signature=init.__text_signature__, doc=init.__doc__)
emit('visible', in_dir='__init__' in dir(deque), in_dict='__init__' in deque.__dict__,
     class_name=init.__class__.__name__, dict_text=repr(deque.__dict__['__init__']))
for args in [(), (None,), ([],), (1,)]:
    try: emit('call', result=init(*args))
    except Exception as error: emit_exception(error)

for args, kwargs in [((), {}), (([7, 8],), {}), (([7, 8, 9], 2), {}),
                     ((), {'iterable': [7, 8, 9], 'maxlen': 1}),
                     (([], -1), {}), ((1,), {})]:
    d = deque([1, 2], 3)
    it = iter(d)
    try: emit('call', result=deque.__init__(d, *args, **kwargs))
    except Exception as error: emit_exception(error)
    emit('state', items=list(d), maxlen=d.maxlen)
    try: emit('next', value=next(it))
    except Exception as error: emit_exception(error)
