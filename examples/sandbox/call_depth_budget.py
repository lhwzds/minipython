"""A finite recursion that a low MiniPython call-depth budget stops."""


def descend(depth):
    if depth == 0:
        return 0
    return 1 + descend(depth - 1)


print(descend(10))
