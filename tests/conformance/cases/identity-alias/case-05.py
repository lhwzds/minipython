value = b"bytes"
x = value
emit('identity', same=x is x, different=x is not x)
