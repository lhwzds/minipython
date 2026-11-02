# SPDX-License-Identifier: MIT

"""Recursion follows CPython 3.14.6 under the sandbox defaults.

The recursion limit starts at 1000, recursion within it works, reaching it
raises a catchable RecursionError, a program may raise the limit, and an
uncaught RecursionError ends the run with CPython's final report line.
"""

import sys


def depth(n):
    return 0 if n == 0 else 1 + depth(n - 1)


def forever(n):
    return forever(n + 1)


print(sys.getrecursionlimit())
print(depth(990))
try:
    forever(0)
except RecursionError as error:
    print("caught:", error)
sys.setrecursionlimit(2000)
print(depth(1500))
sys.setrecursionlimit(1000)
forever(0)
