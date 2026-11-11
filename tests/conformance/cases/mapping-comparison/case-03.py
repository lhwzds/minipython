value = float("nan")
a = {'number': value}; b = {'number': float('inf')}
emit('result', equal=a == b, unequal=a != b)
