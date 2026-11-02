import sys
def f(matcher):
    try:
        raise ValueError("orig")
    except matcher:
        print("caught?")
for m in (42, "ValueError", int, (ValueError, 3), [ValueError], object):
    try:
        f(m)
    except TypeError as e:
        print(type(m).__name__, "->", e, "| ctx:", repr(e.__context__))
try:
    pass
except 42:
    pass
print("no exception, no error")
try:
    raise ValueError
except ValueError:
    print("first matches")
except 42:
    pass
try:
    raise ValueError
except undefined_name:
    pass
