value = "非ASCII"
x = value
d = {x: 'found'}
emit('lookup', result=d[x])
