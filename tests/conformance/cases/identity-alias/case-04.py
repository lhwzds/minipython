value = "非ASCII"
x = value
emit('identity', same=x is x, different=x is not x)
