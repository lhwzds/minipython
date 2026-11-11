from collections import deque
d = deque()
d.append(d)
emit('deque', length=len(d), text=repr(d))
