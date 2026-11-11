type A = int
type B = int
print(A is A, A is B, id(A) == id(A), id(A) == id(B))
TypeAliasType = type(A)
x = TypeAliasType("X", int)
y = TypeAliasType("X", int)
print(x is x, x is y, id(x) == id(x), id(x) == id(y))
