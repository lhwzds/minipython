value = str
x = tuple[value]
y = tuple[value]
emit('identity', same=x is y, equal=x == y)
emit('identity', self=x is x, self_not=x is not x, unequal=list[int] == list[str])
