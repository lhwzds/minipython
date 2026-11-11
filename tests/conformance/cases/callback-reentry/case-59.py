class S:
    def __iter__(self): return self
    def __next__(self): return 1
def g():
    yield from S()
G = g()
next(G)
try:
    print("send", G.send(1))
except AttributeError as e:
    print("AttributeError", e)
G = g()
next(G)
print(G.send(None))
print("END")
