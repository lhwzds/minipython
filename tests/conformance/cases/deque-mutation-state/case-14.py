from collections import deque

class Huge:
    def __index__(self):
        return 2 ** 63

counts = [('limit', 2 ** 63 - 1), ('over', 2 ** 63), ('negative-limit', -2 ** 63),
          ('under', -2 ** 63 - 1), ('large', 2 ** 70), ('large-negative', -2 ** 70)]

for label, count in counts:
    for items in [[], [1], [1, 2, 3]]:
        d = deque(items)
        try:
            d.rotate(count)
            emit('rotated', count=label, items=list(d))
        except Exception as error:
            emit_exception(error)
            emit('unchanged', count=label, items=list(d))

d = deque([1, 2, 3])
try:
    d.rotate(Huge())
except Exception as error:
    emit_exception(error)
    emit('unchanged', items=list(d))

d = deque([1, 2, 3])
try:
    d.rotate(2 ** 63)
except BaseException as error:
    emit('caught', type=type(error).__name__, message=str(error), items=list(d))
else:
    emit('not-caught', items=list(d))
