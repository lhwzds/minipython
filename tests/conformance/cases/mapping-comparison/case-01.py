value = True
a = {1: value, 2: -0.0}; b = {2: 0.0, 1: value}
emit('result', equal=a == b, unequal=a != b)
