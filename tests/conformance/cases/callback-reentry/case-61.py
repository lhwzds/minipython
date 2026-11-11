import collections

class C:
    def __lt__(self, other):
        raise TypeError('no order')
    def __repr__(self):
        return 'C'

def after():
    return 'clean'

print(repr(collections.Counter({'a': C(), 'b': C()})))
print(after())
