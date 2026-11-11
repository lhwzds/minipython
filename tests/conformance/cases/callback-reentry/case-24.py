import itertools
def source():
    yield 1
    print('buffered peer', next(peer))
    yield 2
t, peer = itertools.tee(source())
print(next(t))
print(next(t))
print(next(peer))
