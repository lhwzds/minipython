import operator

class C:
    def __len__(self):
        raise TypeError('bad len')
    def __length_hint__(self):
        return 2

def after():
    return 'clean'

print(operator.length_hint(C()))
print(after())

import functools

class BrokenIter:
    def __iter__(self):
        raise TypeError('from iter')

class BadValue:
    def __iter__(self):
        raise ValueError('other')

for operation in (bytes, bytearray, lambda obj: functools.reduce(max, obj)):
    for obj in (BrokenIter(), BadValue()):
        try:
            operation(obj)
        except Exception as error:
            print(type(error).__name__, str(error))
        print(after())

class Sep(str):
    calls = 0
    def __str__(self):
        Sep.calls += 1
        return '-'

print(1, 2, 3, sep=Sep('x'))
print(Sep.calls)
print(after())
