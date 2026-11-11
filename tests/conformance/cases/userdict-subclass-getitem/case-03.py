VALUE = 3
from collections import UserDict, Counter
class U(UserDict):
    def __getitem__(self, k): return VALUE
a = U(a=1, b=2); d = {}; d |= a
emit('result', merged=d)
emit('result', counter=dict(Counter(a)))
