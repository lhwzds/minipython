import copy
x = list[int]
copied = copy.deepcopy(x)
emit('alias', same=copied is x, equal=copied == x, origin=copied.__origin__ is x.__origin__)
