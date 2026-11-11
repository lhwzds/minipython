from collections import Counter
class A:
    def __init__(self, v, tag): self.v, self.tag = v, tag
    def __hash__(self): return 1
    def __eq__(self, other):
        emit('calls', left=self.tag, right=other.tag)
        return self.v == other.v
c = Counter({A(1, 'c'): 1}); d = Counter({A(1, 'd'): 1})
emit('result', equal=c == d)
