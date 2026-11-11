VALUE = 3
from collections import UserDict
class K:
    fail = False
    def __hash__(self):
        if self.fail: raise RuntimeError('rehashed')
        return 1
k = K(); d = UserDict(); d[k] = VALUE; d['last'] = 2
item = d.popitem(); k.fail = True; d.data['new'] = 3
emit('result', popped=repr(item), equal=d.data == d.data.copy())
