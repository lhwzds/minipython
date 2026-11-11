l = []
u = next(iter(list[l]))
before = id(u)
l.append(1)
print(before == id(u))
print(u is u)
