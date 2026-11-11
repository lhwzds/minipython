value = frozenset([1])
x = value
d = {x: 'found'}
emit('lookup', result=d[x])
