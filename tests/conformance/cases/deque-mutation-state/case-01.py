from collections import deque

class Mutator:
    def __eq__(self, other):
        if self.armed:
            self.armed = False
            change(d, operation)
        return False

def change(d, name):
    if name == 'append': d.append(4)
    elif name == 'appendleft': d.appendleft(4)
    elif name == 'pop': d.pop()
    elif name == 'popleft': d.popleft()
    elif name == 'extend': d.extend([4])
    elif name == 'extendleft': d.extendleft([4])
    elif name == 'rotate-zero': d.rotate(0)
    elif name == 'rotate-length': d.rotate(len(d))
    elif name == 'rotate-negative': d.rotate(-1)
    elif name == 'clear': d.clear()
    elif name == 'insert': d.insert(1, 4)
    elif name == 'remove': d.remove(2)
    elif name == 'delete': del d[1]
    elif name == 'iadd': d += [4]
    elif name == 'imul-two': d *= 2
    elif name == 'imul-zero': d *= 0
    elif name == 'init': d.__init__([1, 2])
    elif name == 'append-pop':
        d.append(4)
        d.pop()

for operation in ['append', 'appendleft', 'pop', 'popleft', 'extend', 'extendleft',
                  'rotate-zero', 'rotate-length', 'rotate-negative', 'clear', 'insert',
                  'remove', 'delete', 'iadd', 'imul-two', 'imul-zero', 'init', 'append-pop']:
    x = Mutator()
    x.armed = True
    d = deque([x, 1, 2])
    emit('operation', name=operation)
    try: emit('result', found=99 in d)
    except Exception as error: emit_exception(error)

for operation in ['append', 'appendleft', 'extend', 'extendleft', 'rotate-zero']:
    x = Mutator()
    x.armed = True
    d = deque([x, 1, 2], maxlen=3)
    emit('operation', name=operation, bounded=True)
    try: emit('result', found=99 in d)
    except Exception as error: emit_exception(error)
