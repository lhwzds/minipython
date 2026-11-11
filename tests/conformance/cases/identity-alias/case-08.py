value = float("inf")
x = value
emit('identity', same=x is x, different=x is not x)
