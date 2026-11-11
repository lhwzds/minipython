value = 2**63 - 1
x = value
emit('identity', same=x is x, different=x is not x)
