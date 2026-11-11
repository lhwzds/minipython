from collections import deque

class Mutator:
    def __eq__(self, other):
        if self.armed:
            self.armed = False
            change(d, operation)
        return False

def change(d, name):
    if name == 'setitem': d[0] = 8
    elif name == 'reverse': d.reverse()
    elif name == 'extend-empty': d.extend([])
    elif name == 'extendleft-empty': d.extendleft([])
    elif name == 'iadd-empty': d += []
    elif name == 'imul-one': d *= 1
    elif name == 'copy': d.copy()
    elif name == 'count': d.count(8)
    elif name == 'read': len(d), d[1], d.maxlen

for operation in ['setitem', 'reverse', 'extend-empty', 'extendleft-empty',
                  'iadd-empty', 'imul-one', 'copy', 'count', 'read']:
    x = Mutator()
    x.armed = True
    d = deque([x, 1, 2])
    emit('operation', name=operation)
    try: emit('result', found=99 in d, length=len(d))
    except Exception as error: emit_exception(error)
