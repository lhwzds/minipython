from collections import deque


class Index:
    def __index__(self):
        source.clear()
        return 0


source = deque([1, 2, 3])
print(list(enumerate(source, Index())))
print(list(source))
