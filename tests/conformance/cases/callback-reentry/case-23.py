import itertools
class I:
    def __iter__(self):
        try: t.close()
        except AttributeError: print('no close')
        return iter([1, 2])
t = itertools.chain.from_iterable([I()])
print(list(t))
