from collections import deque
class T:
    def __repr__(self):
        d.clear()
        d.append(42)
        return 't'
d = deque([T(), 1], maxlen=5)
print(repr(d))
print(d)
d.clear()
d.append(d)
print(repr(d))
