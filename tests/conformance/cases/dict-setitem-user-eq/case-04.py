calls = []
class Base:
    def __hash__(self): return 1
    def __eq__(self, other):
        calls.append('base')
        return True
class Child(Base):
    __hash__ = Base.__hash__
    def __eq__(self, other):
        calls.append('child')
        return False
d = {}; dict.__setitem__(d, Base(), 'first')
dict.__setitem__(d, Child(), 'second')
emit('calls', calls=calls)
emit('result', size=len(d), values=sorted(d.values()))
