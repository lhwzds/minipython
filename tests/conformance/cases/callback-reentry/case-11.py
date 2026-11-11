import itertools
def source():
    yield 1
    try:
        next(t)
    except RuntimeError as e:
        print(type(e).__name__, str(e))
    yield 2
t, peer = itertools.tee(source())
print(next(t), next(t))
print(list(peer))
class I:
    def __iter__(self): return self
    def __next__(self): return next(other)
x, other = itertools.tee(I())
try:
    next(x)
except RuntimeError as e:
    print(type(e).__name__, str(e))
