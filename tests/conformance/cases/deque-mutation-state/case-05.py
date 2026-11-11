from collections import deque

for backward in [False, True]:
    for operation in ['append', 'appendleft', 'pop', 'popleft', 'rotate-zero', 'clear',
                      'delete', 'iadd', 'imul', 'append-pop', 'init']:
        d = deque([1, 2, 3])
        it = reversed(d) if backward else iter(d)
        emit('first', backward=backward, operation=operation, value=next(it))
        if operation == 'append': d.append(4)
        elif operation == 'appendleft': d.appendleft(4)
        elif operation == 'pop': d.pop()
        elif operation == 'popleft': d.popleft()
        elif operation == 'rotate-zero': d.rotate(0)
        elif operation == 'clear': d.clear()
        elif operation == 'delete': del d[1]
        elif operation == 'iadd': d += [4]
        elif operation == 'imul': d *= 2
        elif operation == 'append-pop':
            d.append(4)
            d.pop()
        elif operation == 'init': d.__init__([1, 2, 3])
        for attempt in range(2):
            try: emit('next', value=next(it))
            except Exception as error: emit_exception(error)

for backward in [False, True]:
    d = deque([1, 2, 3, 4])
    it = reversed(d) if backward else iter(d)
    emit('first', backward=backward, value=next(it))
    d[1] = 9
    emit('next', value=next(it))
    d.reverse()
    d.extend([])
    d.extendleft([])
    d += []
    d *= 1
    emit('rest', values=list(it))
