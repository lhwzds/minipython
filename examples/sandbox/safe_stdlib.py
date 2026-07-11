"""Exercise every standard-library module in the sandbox allowlist."""

import array
import builtins
import collections
import collections.abc
import copy
import functools
import io
import itertools
import json
import math
import operator
import sys
import types

print(builtins.len([1, 2]))
print(isinstance(sys.modules, dict))
print(types.SimpleNamespace(x=1).x)
print(collections.Counter("aa")["a"], collections.abc.Sequence.__name__)
print(math.sqrt(4))
try:
    import math.integer
except ModuleNotFoundError:
    print("math.integer unavailable")
else:
    print("math.integer", math.integer.gcd(12, 18))
print(array.array("B", [65]).tobytes())
print(copy.copy([1]) == [1])
buffer = io.BytesIO(b"ab")
print(buffer.read(1))
print(operator.add(2, 3), functools.reduce(lambda a, b: a + b, [1, 2, 3]))
print(next(itertools.count(4)))
print(json.loads('{"a": 1}')["a"])
