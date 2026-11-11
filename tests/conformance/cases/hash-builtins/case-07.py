value = 10**100
x = value
d = {x: 'found'}
emit('lookup', result=d[x])
