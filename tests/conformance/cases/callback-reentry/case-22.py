import copy
class E:
    armed = True
    def __hash__(self): return 1
    def __deepcopy__(self, memo):
        change()
        return 'copy'
def change(): target.clear()
target = {E()}
print(copy.deepcopy(target), len(target))
target = [E()]
it = iter(target)
print(list(copy.deepcopy(it)), len(target))
class Holder: pass
h = Holder()
h.x = E()
def change(): h.x = None
print(copy.deepcopy(h).x, h.x is None)
