class I:
    def __index__(self):
        a.clear()
        return 5
a = [1, 2]
a.insert(I(), 42)
print(a)
try:
    a.insert(object(), 2)
except TypeError as e:
    print(type(e).__name__, str(e))
try:
    {}.pop()
except TypeError as e:
    print(type(e).__name__, str(e))
