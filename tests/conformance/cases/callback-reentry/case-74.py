a = t"{1}"
b = t"{1}"
print(id(a) == id(b), id(a) == id(a), a is a, a is b)
print(a.interpolations[0] is a.interpolations[0])
