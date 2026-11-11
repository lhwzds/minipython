import itertools
class N:
    armed = True
    def __int__(self): return 0
    def __add__(self, other):
        if self.armed:
            self.armed = False
            try:
                next(t)
                print('inner yielded')
            except BaseException as e:
                print('inner', type(e).__name__)
        return self
n = N()
t = itertools.count(n)
print(next(t) is n)
