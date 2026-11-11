from collections import defaultdict

def factory():
    d.default_factory = None
    d.clear()
    return 42
for get in [lambda: d['x'], lambda: d.__missing__('x'), lambda: d.__getitem__('x')]:
    d = defaultdict(factory, {'old': 1})
    print(get(), d.default_factory is None, list(d.items()))
