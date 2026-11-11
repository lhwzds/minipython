from collections import deque

class Replacer:
    def __eq__(self, other):
        d[1] = 9
        return False

for method in ['contains', 'count', 'index', 'remove']:
    d = deque([Replacer(), 1, 9])
    result = 9 in d if method == 'contains' else getattr(d, method)(9)
    emit('result', method=method, value=result, length=len(d))

for backward in [False, True]:
    d = deque([1])
    it = reversed(d) if backward else iter(d)
    emit('exhaust', backward=backward, first=next(it), end=next(it, 'end'))
    d.append(2)
    for attempt in range(2):
        try: emit('next', value=next(it, 'end'))
        except Exception as error: emit_exception(error)
