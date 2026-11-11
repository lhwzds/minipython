from collections import deque
d = deque([1, 2])
try:
    for a in d:
        d.clear()
except RuntimeError as e:
    print('caught', str(e))
print('after', len(d))
