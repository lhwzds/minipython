class E:
    def __eq__(self, other):
        a.clear()
        return False
    def __lt__(self, other):
        raise AssertionError('removed item compared')
a = [E(), 1]
print(a < [7, 1], a)
