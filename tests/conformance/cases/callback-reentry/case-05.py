from collections import OrderedDict
d = OrderedDict([(1, 1), (2, 2)])
try:
    for a in d:
        d.clear()
except RuntimeError as e:
    print('caught', type(e).__name__)
print('after', len(d))
