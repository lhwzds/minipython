value = [1]
x = value
y = x
z = type(x)(x)
emit('identity', alias=x is y, alias_not=x is not y, fresh=x is z, fresh_not=x is not z)
