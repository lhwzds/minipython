from collections import deque

for operation in ['empty', 'values', 'bounded', 'unbounded', 'keyword', 'self']:
    d = deque([1, 2, 3], maxlen=3)
    alias = d
    append = d.append
    it = iter(d)
    rev = reversed(d)
    if operation == 'empty': result = d.__init__()
    elif operation == 'values': result = d.__init__([7, 8])
    elif operation == 'bounded': result = d.__init__([7, 8, 9], 2)
    elif operation == 'unbounded': result = d.__init__([7, 8], None)
    elif operation == 'keyword': result = d.__init__(iterable=[7, 8, 9], maxlen=1)
    else: result = d.__init__(d)
    emit('result', operation=operation, value=result, items=list(alias), maxlen=alias.maxlen)
    append(10)
    emit('alias', items=list(d), maxlen=d.maxlen)
    for iterator in [it, rev]:
        try: emit('next', value=next(iterator, 'end'))
        except Exception as error: emit_exception(error)
