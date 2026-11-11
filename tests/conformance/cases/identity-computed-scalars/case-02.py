x = 'ab'
y = 'a' + str('b')
emit('identity', same=x is y, equal=x == y)
