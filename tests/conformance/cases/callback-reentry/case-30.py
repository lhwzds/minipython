import itertools
items = list(range(300))
a, b = itertools.tee(iter(items))
first = list(a)
second = list(b)
print(first == items, second == items)
c = itertools.cycle(items)
seen = [next(c) for _ in range(700)]
print(seen == items * 2 + items[:100])
