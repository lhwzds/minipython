# Syntax that a newer CPython development grammar accepts but CPython 3.14.6
# rejects: soft-keyword lazy imports and unpacking in comprehensions. mnpy
# must reject them exactly as 3.14.6 does, with the same message and
# location, and accept the starred displays 3.14.6 accepts.
for source in [
    "lazy import json",
    "lazy from json import loads",
    "lazy = 1\nprint(lazy)",
    "[*x for x in [[1], [2]]]",
    "{*x for x in [[1]]}",
    "(*x for x in [[1]])",
    "f(*x for x in y)",
    "{**x for x in [{}]}",
    "[*x, *y]",
    "*x, *y = z",
]:
    try:
        compile(source, "<case>", "exec")
        print(repr(source), "accepted")
    except SyntaxError as error:
        print(repr(source), type(error).__name__, error.msg, error.args[0] == error.msg,
              error.lineno, error.offset, error.end_lineno, error.end_offset, repr(error.text))
