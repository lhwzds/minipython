from collections import deque

for length in [0, 1, 2]:
    for operation in ['rotate-zero', 'rotate-one', 'clear', 'imul-zero', 'imul-two']:
        for backward in [False, True]:
            d = deque(range(length))
            it = reversed(d) if backward else iter(d)
            if operation == 'rotate-zero': d.rotate(0)
            elif operation == 'rotate-one': d.rotate()
            elif operation == 'clear': d.clear()
            elif operation == 'imul-zero': d *= 0
            else: d *= 2
            emit('operation', length=length, name=operation, backward=backward)
            try: emit('next', value=next(it, 'end'))
            except Exception as error: emit_exception(error)

for operation in ['append', 'appendleft', 'extend', 'extendleft']:
    d = deque([], maxlen=0)
    it = iter(d)
    getattr(d, operation)([1] if operation.startswith('extend') else 1)
    emit('operation', name=operation, maxlen=0)
    try: emit('next', value=next(it, 'end'))
    except Exception as error: emit_exception(error)
