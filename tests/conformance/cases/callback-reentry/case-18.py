import itertools
class Source:
    def __iter__(self): return self
    def __next__(self):
        if self.n == 3: raise StopIteration
        n = self.n
        self.n += 1
        if n == 0: print('inner', next(t))
        return n
for wrap in [lambda s: map(lambda x: x + 10, s), lambda s: filter(lambda x: True, s), lambda s: zip(s), lambda s: itertools.starmap(lambda x: x, zip(s)), lambda s: itertools.filterfalse(lambda x: False, s), lambda s: itertools.compress(s, [True] * 4), lambda s: itertools.batched(s, 1)]:
    s = Source()
    s.n = 0
    t = wrap(s)
    print('outer', next(t))
    print('tail', list(t))
