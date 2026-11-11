import copy
class Holder:
    def __init__(self): self.items = [1]
holder = Holder()
x = list[holder]
y = copy.deepcopy([x, x])
argument = y[0].__args__[0]
emit('argument', copied=argument is not holder, deep=argument.items is not holder.items)
emit('alias', same=y[0] is y[1])
