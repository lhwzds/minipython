from collections import UserList
class Result:
    def __bool__(self):
        raise AssertionError('ordering result coerced')
    def __repr__(self): return 'answer'
class E:
    def __eq__(self, other): return False
    def __gt__(self, other): return Result()
    def __lt__(self, other): return 'lt'
for wrap in [list, tuple, UserList]:
    a, b = wrap([E(), 1]), wrap([7, 1])
    print(wrap.__name__, a > b, a < b)
