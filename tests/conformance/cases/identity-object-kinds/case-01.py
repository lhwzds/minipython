s = slice(1, 2)
emit('slice', same=s is s, equal=s == slice(1, 2))
