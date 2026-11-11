import copy
x = list[int]
y = copy.deepcopy([x, x])
emit('alias', same=y[0] is y[1], copied=y[0] is not x, equal=y[0] == x)
