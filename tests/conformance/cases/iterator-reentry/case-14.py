import itertools
class Source:
    n = 0
    def __iter__(self): return self
    def __next__(self):
        self.n += 1
        if self.n == 2:
            try:
                print('inner', next(group))
            except BaseException as e:
                print('inner', type(e).__name__)
        if self.n > 3: raise StopIteration
        return 1
t = itertools.groupby(Source())
key, group = next(t)
print(next(group))
print(next(group))
