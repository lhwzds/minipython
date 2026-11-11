from collections import deque
from copy import copy, deepcopy

for operation in ['method-copy', 'copy', 'deepcopy', 'add', 'mul', 'rmul']:
    d = deque([1, 2, 3], 3)
    if operation == 'method-copy': other = d.copy()
    elif operation == 'copy': other = copy(d)
    elif operation == 'deepcopy': other = deepcopy(d)
    elif operation == 'add': other = d + deque()
    elif operation == 'mul': other = d * 1
    else: other = 1 * d
    it = iter(other)
    rev = reversed(other)
    d.append(4)
    d.pop()
    emit('independent', operation=operation, forward=list(it), backward=list(rev))
    it = iter(d)
    other.append(9)
    other.__init__([8], 1)
    emit('original', items=list(it), maxlen=d.maxlen, other_maxlen=other.maxlen)
