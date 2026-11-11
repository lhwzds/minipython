value = 7
x = value
emit('identity', same=x is x, different=x is not x)
