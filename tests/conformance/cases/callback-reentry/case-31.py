from collections import ChainMap, Counter, deque

# A builtin method called with the wrong arguments or the wrong receiver must
# raise a catchable exception, not end the program.
calls = [
    lambda: {}.get(),
    lambda: {}.pop(),
    lambda: {}.update(1, 2),
    lambda: [].sort(1),
    lambda: list.append(),
    lambda: list.pop(1, 2),
    lambda: set.union(),
    lambda: set.union(1),
    lambda: set.difference(),
    lambda: deque.append(object()),
    lambda: deque.append(),
    lambda: deque([1]).pop(1),
    lambda: ChainMap({}).pop(),
    lambda: ChainMap({}).get(),
    lambda: ChainMap.pop(),
    lambda: Counter.most_common(),
    lambda: Counter().most_common(1, 2),
    lambda: Counter().elements(1),
    lambda: int.to_bytes(),
    lambda: int.from_bytes(),
    lambda: int.bit_length(),
]
for call in calls:
    try:
        call()
    except BaseException as exc:
        print(type(exc).__name__)
