value = (1, 2)
x = value
d = {x: 'found'}
emit('lookup', result=d[x])
