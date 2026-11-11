value, other = 1, 2
emit('result', different_members={value} == frozenset([other]), equal={value} == frozenset([value]))
