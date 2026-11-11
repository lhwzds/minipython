calls = []
class Left:
    def __hash__(self): return 1
    def __eq__(self, other):
        calls.append('left')
        return NotImplemented
class Right:
    def __hash__(self): return 1
    def __eq__(self, other):
        calls.append('right')
        return True
d = {}; dict.__setitem__(d, Left(), 'first')
dict.__setitem__(d, Right(), 'second')
emit('calls', calls=calls)
emit('result', size=len(d), values=sorted(d.values()))
from collections import UserDict, Counter
class U(UserDict):
    def __getitem__(self, key): return 3
source = U(a=1, b=2)
merged = {}; merged |= source
emit('result', merged=merged)
emit('result', counter=dict(Counter(source)))
