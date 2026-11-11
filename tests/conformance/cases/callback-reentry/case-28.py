class Source:
    def __iter__(self): return self
    def __next__(self): return next(t)
for wrap in [lambda s: map(lambda x: x, s), lambda s: filter(None, s), lambda s: zip(s)]:
    t = wrap(Source())
    try:
        next(t)
    except RecursionError:
        print('caught RecursionError')
