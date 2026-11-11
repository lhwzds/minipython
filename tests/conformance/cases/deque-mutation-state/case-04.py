from collections import deque

class Mutator:
    def __eq__(self, other):
        if self.armed:
            self.armed = False
            target.rotate(0)
        return True

def compare(a, b, op):
    if op == 'eq': return a == b
    if op == 'ne': return a != b
    if op == 'lt': return a < b
    if op == 'le': return a <= b
    if op == 'gt': return a > b
    return a >= b

for side in ['left', 'right']:
    for op in ['eq', 'ne', 'lt', 'le', 'gt', 'ge']:
        x = Mutator()
        x.armed = True
        a = deque([x, 1])
        b = deque([0, 1])
        target = a if side == 'left' else b
        emit('operation', side=side, op=op)
        try: emit('result', value=compare(a, b, op))
        except Exception as error: emit_exception(error)

class Replacer:
    def __eq__(self, other):
        a[0] = 8
        return True

for op in ['eq', 'ne', 'lt', 'le', 'gt', 'ge']:
    a = deque([Replacer(), 1])
    b = deque([0, 1])
    emit('result', op=op, value=compare(a, b, op))
