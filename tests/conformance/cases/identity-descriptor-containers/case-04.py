n = float('nan')
emit('same', list_equal=[n] == [n], member=n in [n], count=[n].count(n))
m = float('nan')
emit('other', list_equal=[m] == [float('nan')], member=float('nan') in [m], count=[m].count(float('nan')))
