import itertools
def a():
    raise ValueError("a")
    yield
def b():
    raise KeyError("b")
    yield
try:
    try:
        itertools.product(a(), b())
    except ValueError as e:
        print("inner", e)
except KeyError as e:
    print("outer", e)
events = []
def source():
    yield 1
    raise ValueError("boom")
def key(x):
    events.append(x)
    return x
try:
    sorted(source(), key=key)
except ValueError:
    print(events)
print("END")
