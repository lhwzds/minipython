from collections import deque

for backward in [False, True]:
    for length in [0, 1, 3]:
        for operation in ['append', 'extend', 'clear', 'imul-zero']:
            d = deque(range(length))
            it = reversed(d) if backward else iter(d)
            values = []
            for index in range(length): values.append(next(it))
            if operation == 'append': d.append(9)
            elif operation == 'extend': d.extend([9])
            elif operation == 'clear': d.clear()
            else: d *= 0
            emit('exhausted', backward=backward, length=length, operation=operation, values=values)
            for attempt in range(2):
                try: emit('next', value=next(it))
                except Exception as error: emit_exception(error)

for length in [0, 1, 3]:
    d = deque(range(length))
    it = d.__reversed__()
    for index in range(length): next(it)
    d.append(9)
    try: emit('dunder', value=next(it))
    except Exception as error: emit_exception(error)
