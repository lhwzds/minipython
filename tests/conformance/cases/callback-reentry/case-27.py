from collections import deque
from array import array
for a in [[], deque(), bytearray(), array('b')]:
    try:
        a.insert(2**100, 1)
    except OverflowError as e:
        print(type(e).__name__, str(e), len(a))
