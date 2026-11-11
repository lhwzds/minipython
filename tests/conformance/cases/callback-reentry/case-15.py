class E:
    armed = True
    def __hash__(self):
        if self.armed:
            self.armed = False
            s.clear()
        return 1
    def __eq__(self, other): return False
s = {1}
x = E()
s.add(x)
print(len(s), x in s)
