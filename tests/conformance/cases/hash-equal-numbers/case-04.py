value = 2**63
a = value; b = float(a)
emit('result', equal=a == b, same_hash=hash(a) == hash(b), bool_hash=hash(True) == hash(1))
