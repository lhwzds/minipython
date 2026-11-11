from itertools import permutations


for pool, count in (([], 1), ([], 2), ([1], 2), ([1], 3)):
    print(list(permutations(pool, count)))
print(list(permutations([], 0)))
