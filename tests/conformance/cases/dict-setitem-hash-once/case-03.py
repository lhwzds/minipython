VALUE = 3
HASH = 3
class K:
    n = 0
    def __hash__(self): K.n += 1; return HASH
    def __eq__(self, other): return True
d = {}; k = K(); d.__setitem__(k, VALUE)
emit('result', calls=K.n, size=len(d))
