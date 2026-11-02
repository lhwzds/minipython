"""math functions CPython takes from the platform C library, on deterministic
inputs: each result prints as `FUNCTION ARGUMENTS.hex() -> RESULT.hex()` (or
the exception class). Run by the release gate's smoke test of the shipped
binary (tests/release_gate/math_probe.py) against CPython.
"""
import math
state = 20261031
def draw(low, high):
    global state
    state = (state * 6364136223846793005 + 1442695040888963407) % 2 ** 64
    return low + (high - low) * (state >> 11) / 2 ** 53
def wide():
    global state
    state = (state * 6364136223846793005 + 1442695040888963407) % 2 ** 64
    return (state / 2 ** 64 - 0.5) * 10.0 ** ((state >> 9) % 60 - 30)
unary = {
    'exp': lambda: draw(-700, 700), 'expm1': lambda: draw(-40, 40), 'exp2': lambda: draw(-1000, 1000),
    'log': lambda: abs(wide()), 'log2': lambda: abs(wide()), 'log10': lambda: abs(wide()), 'log1p': lambda: draw(-0.999, 1e6),
    'sin': lambda: draw(-1e4, 1e4), 'cos': lambda: draw(-1e4, 1e4), 'tan': lambda: draw(-1e4, 1e4),
    'asin': lambda: draw(-1, 1), 'acos': lambda: draw(-1, 1), 'atan': wide,
    'sinh': lambda: draw(-700, 700), 'cosh': lambda: draw(-700, 700), 'tanh': lambda: draw(-20, 20),
    'asinh': wide, 'acosh': lambda: 1 + abs(wide()), 'atanh': lambda: draw(-1, 1),
    'erf': lambda: draw(-6, 6), 'erfc': lambda: draw(-6, 27), 'gamma': lambda: draw(-30, 170), 'lgamma': lambda: draw(-30, 1e5),
    'sqrt': lambda: abs(wide()), 'cbrt': wide,
}
binary = {
    'pow': lambda: (draw(0, 100), draw(-50, 50)), 'atan2': lambda: (wide(), wide()), 'hypot': lambda: (wide(), wide()),
    'fmod': lambda: (wide(), wide()), 'remainder': lambda: (wide(), wide()), 'copysign': lambda: (wide(), wide()),
}
for name, make in list(unary.items()) + list(binary.items()):
    function = getattr(math, name)
    for i in range(200):
        args = make()
        args = args if isinstance(args, tuple) else (args,)
        label = name + ' ' + ' '.join(a.hex() for a in args)
        try:
            print(label, '->', function(*args).hex())
        except (ValueError, OverflowError) as error:
            print(label, '!!', type(error).__name__)
