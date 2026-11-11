import itertools
class Source:
    def __iter__(self): return self
    def __next__(self):
        if self.n == 2: raise StopIteration
        self.n += 1
        try:
            t.close()
        except AttributeError:
            print('no close')
        return self.n
for wrap in [lambda s: map(lambda x: x, s), lambda s: filter(None, s), lambda s: zip(s), lambda s: enumerate(s), lambda s: itertools.chain(s), lambda s: itertools.accumulate(s)]:
    s = Source()
    s.n = 0
    t = wrap(s)
    print(list(t))
