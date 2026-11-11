# `d = deque([1, 2], maxlen=2**62); d *= 2**61` is checked outside the corpus,
# because CPython does not finish it in minutes and so cannot serve as the
# oracle; mnpy must raise a catchable MemoryError.
from collections import deque
d = deque(maxlen=1)
class S:
    def __iter__(self): d.__init__([], 0); return iter([7])
d.extend(S()); print(d.maxlen, list(d))
d = deque(maxlen=1); d.extendleft(S()); print(d.maxlen, list(d))
d = deque(maxlen=1); d += S(); print(d.maxlen, list(d))
