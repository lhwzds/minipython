from collections import deque

class Mutator:
    def __eq__(self, other):
        if self.armed:
            self.armed = False
            d.append(4)
            d.pop()
        return matched

for method in ['contains', 'count', 'index', 'remove']:
    for matched in [False, True]:
        x = Mutator()
        x.armed = True
        d = deque([x, 1, 2])
        emit('operation', method=method, matched=matched)
        try:
            result = 99 in d if method == 'contains' else getattr(d, method)(99)
            emit('result', value=result)
        except Exception as error: emit_exception(error)

for d in [deque(), deque([1, 2])]:
    for method in ['index', 'remove']:
        try: getattr(d, method)(99)
        except Exception as error: emit_exception(error)
