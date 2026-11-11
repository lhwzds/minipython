class Index:
    def __index__(self):
        a.append(42)
        return -1
a = [0]
try:
    print(a.index(0, Index()))
except ValueError:
    print('missing', a)
