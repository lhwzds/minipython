value = -1
x = float('nan')
emit('nan', equal=x == x, unequal=x != x, list_equal=[x] == [x], member=x in [x], other_equal=x == value, other_unequal=x != value)
