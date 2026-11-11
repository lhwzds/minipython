import copy
import itertools

first, second = itertools.tee([1, 2, 3])
branch = copy.copy(second)
print(branch is second)
print(list(branch))
print(list(second))
