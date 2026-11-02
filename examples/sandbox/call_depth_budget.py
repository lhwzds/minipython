# SPDX-License-Identifier: MIT

"""A finite recursion that a low MiniPython call-depth cap stops.

With `--max-depth 3` the recursion limit is 3 frames, so the fourth nested
call raises `RecursionError: maximum recursion depth exceeded`.
"""


def descend(depth):
    if depth == 0:
        return 0
    return 1 + descend(depth - 1)


print(descend(10))
