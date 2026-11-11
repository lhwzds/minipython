import itertools
class Source:
    def __init__(self, nested=False):
        self.n = 0
        self.nested = nested
    def __iter__(self): return self
    def __len__(self): return 3
    def __getitem__(self, index):
        return self.__next__()
    def __next__(self):
        if self.n == 3: raise StopIteration
        n = self.n
        self.n += 1
        if n == 0:
            try:
                print('inner', describe(next(t)))
            except BaseException as e:
                print('inner', type(e).__name__)
        return [n] if self.nested else n
def describe(value):
    if isinstance(value, tuple):
        return tuple('iterator' if hasattr(x, '__next__') else x for x in value)
    return value
print('reversed')
t = reversed(Source())
try:
    print('outer', describe(next(t)))
except BaseException as e:
    print('outer', type(e).__name__)
