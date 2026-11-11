from collections import deque

for value in [-1, -(10 ** 100), 2 ** 63, 2 ** 64, 10 ** 100, 1.5, 'a', True, None]:
    d = deque([1, 2], 3)
    it = iter(d)
    try:
        d.__init__([7], value)
        emit('result', items=list(d), maxlen=d.maxlen)
    except Exception as error:
        emit_exception(error)
        emit('unchanged', items=list(d), maxlen=d.maxlen, first=next(it))
