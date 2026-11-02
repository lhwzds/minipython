E = ValueError
try:
    raise E("x")
except ValueError as e:
    print(isinstance(e, E), issubclass(type(e), E))
try:
    raise E
except E as e:
    print("raise class alias", repr(e))
try:
    raise E("a") from KeyError("b")
except E as e:
    print("from", repr(e.__cause__))
class CM:
    def __enter__(self): return self
    def __exit__(self, t, v, tb):
        print("exit", t is E, issubclass(t, LookupError))
        return t is not None and issubclass(t, E)
with CM():
    raise ValueError
def gen():
    try:
        yield 1
    except E:
        print("gen caught")
        yield 2
g = gen(); next(g)
print(g.throw(ValueError("t")))
def gen2():
    try:
        yield 1
    except GeneratorExit:
        print("gen exit")
        raise
g = gen2(); next(g); g.close()
