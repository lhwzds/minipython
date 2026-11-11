value = b"bytes"
x = value
d = {x: 'found'}
emit('lookup', result=d[x])
