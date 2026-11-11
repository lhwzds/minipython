from collections import deque


class Index:
    def __index__(self):
        source.clear()
        return 0


source = deque([1, 2, 3])
try:
    source[Index():Index()]
except TypeError as error:
    print('caught', type(error).__name__, error)
print(list(source))
target = deque([1, 2, 3])
try:
    target[Index():Index()] = [9]
except TypeError as error:
    print('caught', type(error).__name__, error)
print(list(target))
