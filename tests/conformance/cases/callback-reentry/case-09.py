from collections import deque
d = deque([1, 2, 3])
before = id(d)
d.rotate(len(d))
print(id(d) == before)
d.clear()
d.append(d)
print(id(d) == before, d[0] is d)
