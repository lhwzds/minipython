from collections import deque
class I:
    def __index__(self):
        print('length', len(d))
        d.clear()
        d.__init__([8], maxlen=3)
        return 1
d = deque([1, 2], maxlen=2)
d.insert(I(), 42)
print(d)
