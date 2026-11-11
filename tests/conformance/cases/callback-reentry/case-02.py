class E:
    def __eq__(self, other):
        a[0] = 3
        b[0] = 4
        return False
    def __lt__(self, other):
        raise AssertionError('stale item compared')
a = [E(), 1]
b = [7, 1]
print(a < b, a, b)
