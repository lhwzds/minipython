import array
import itertools as it
import re

class Sequence:
    def __getitem__(self, index):
        if 0 <= index < 3:
            return index
        raise IndexError
    def __len__(self):
        return 3

def positive(x):
    return x > 0

def next_value():
    return 1

def make_group():
    return next(it.groupby([1, 1]))[1]

factories = [
    lambda: iter([1, 2]),
    lambda: iter((1, 2)),
    lambda: iter(t"a{1}b"),
    lambda: iter(array.array('i', [1, 2])),
    lambda: iter({1, 2}),
    lambda: iter({1: 2, 3: 4}),
    lambda: reversed([1, 2]),
    lambda: reversed({1: 2, 3: 4}),
    lambda: enumerate([1, 2]),
    lambda: zip([1, 2], [3, 4]),
    lambda: map(abs, [1, 2]),
    lambda: filter(positive, [1, 2]),
    lambda: iter(next_value, 0),
    lambda: iter(Sequence()),
    lambda: reversed(Sequence()),
    lambda: it.count(),
    lambda: it.repeat(1, 2),
    lambda: it.cycle([1, 2]),
    lambda: it.chain([1, 2]),
    lambda: it.chain.from_iterable([[1, 2]]),
    lambda: it.accumulate([1, 2]),
    lambda: it.compress([1, 2], [1, 1]),
    lambda: it.filterfalse(positive, [0, 0]),
    lambda: it.takewhile(positive, [1, 2]),
    lambda: it.dropwhile(positive, [0, 0]),
    lambda: it.starmap(pow, [(2, 2), (3, 2)]),
    lambda: it.zip_longest([1, 2], [3, 4]),
    lambda: it.islice([1, 2], 2),
    lambda: it.pairwise([1, 2, 3]),
    lambda: it.product([1, 2]),
    lambda: it.combinations([1, 2, 3], 2),
    lambda: it.combinations_with_replacement([1, 2], 2),
    lambda: it.permutations([1, 2], 2),
    lambda: it.tee([1, 2])[0],
    lambda: it.batched([1, 2], 1),
    make_group,
    lambda: re.finditer('a', 'aa'),
]
for make in factories:
    x = make()
    y = make()
    next(x)
    print(x is x, x is y, id(x) == id(x), iter(x) is x)
