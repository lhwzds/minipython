def source():
    for action in [lambda: next(g), lambda: g.send(None), lambda: g.close(), lambda: g.throw(KeyError)]:
        try:
            action()
        except ValueError as e:
            print(type(e).__name__, str(e))
    yield 42
g = source()
print(next(g))
