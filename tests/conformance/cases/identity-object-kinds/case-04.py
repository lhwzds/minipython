from collections import deque
d = deque()
before = id(d)
d.append(1)
after = id(d)
emit('deque', stable=before == after, length=len(d))
