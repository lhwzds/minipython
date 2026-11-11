class K:
    def __init__(self, v): self.v = v
    def __hash__(self): return 1
    def __eq__(self, other):
        return self.v == other.v
d = {}; dict.__setitem__(d, K(1), 'first')
try:
    dict.__setitem__(d, K(2), 'second')
    emit('result', size=len(d), values=sorted(d.values()))
except RuntimeError as e: emit_exception(e)
