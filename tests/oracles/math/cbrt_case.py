"""math.cbrt over fixed edge values and 40 deterministic inputs.

Each value prints as `cbrt LABEL INPUT.hex() RESULT.hex()`; argument errors
print their class. Read by `cpython_math_cbrt_diff_subset` (tests/parity.rs)
and by the release gate's smoke test of the shipped binary
(tests/release_gate/math_probe.py); cbrt-exclusions.txt lists where the
oracle's C library differs.
"""
import math
def show(label, x, r):
    print('cbrt', label, float(x).hex(), r.hex())
print(type(math.cbrt(0)).__name__)
for x in [0, 1, 8, 27, -1, -27, 0.0, -0.0, 1.2, -2.6, 64.0, 125.0, 1000.0, 3.375, 2.0, 10.0, 1e-300, -1e-300, 5e-324, 1e308, 2.0 ** -1074, 2.0 ** 1023]:
    show(repr(x), x, math.cbrt(x))
print(math.cbrt(math.inf), math.cbrt(-math.inf), math.isnan(math.cbrt(math.nan)))
class Carrier:
    # holds one number and hands it back through a single conversion hook
    def __init__(self, payload):
        self.payload = payload
class FloatLike(Carrier):
    def __float__(self):
        return self.payload
class IndexLike(Carrier):
    def __index__(self):
        return self.payload
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')
show('FloatLike(8.0)', 8.0, math.cbrt(FloatLike(8.0)))
show('IndexLike(27)', 27, math.cbrt(IndexLike(27)))
state = 12345
for i in range(40):
    state = (state * 6364136223846793005 + 1442695040888963407) % 2 ** 64
    x = (state / 2 ** 64 - 0.5) * 10 ** ((state >> 7) % 40 - 20)
    show('sweep', x, math.cbrt(x))
for expr in [
    lambda: math.cbrt(),
    lambda: math.cbrt(1, 2),
    lambda: math.cbrt('1.0'),
    lambda: math.cbrt(1+2j),
    lambda: math.cbrt(IndexLike(10**10000)),
    lambda: math.cbrt(BadFloat()),
    lambda: math.cbrt(RaisesFloat()),
    lambda: math.cbrt(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)
