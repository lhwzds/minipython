class Source:
    def __iter__(self): return self
    def __next__(self): raise LookupError('callback')
try:
    for a in Source():
        print('unexpected')
except LookupError as e:
    print(type(e).__name__, str(e))
print('continued')
