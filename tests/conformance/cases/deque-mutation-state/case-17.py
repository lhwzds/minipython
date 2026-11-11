from collections import deque

# A count that does not fit in a size is an error, and a repeated sequence
# that does not fit in a size is a memory error.
for maxlen, length, counts in [(2, 1, [2 ** 40, 2 ** 62, 2 ** 63, 2 ** 70]),
                               (2, 2, [2 ** 40, 2 ** 62, 2 ** 63, 2 ** 70]),
                               (3, 3, [2 ** 40, 2 ** 62, 2 ** 63, 2 ** 70]),
                               (None, 1, [2 ** 63, 2 ** 70]),
                               (None, 2, [2 ** 62, 2 ** 63, 2 ** 70])]:
    for count in counts:
        d = deque(range(length), maxlen=maxlen)
        iterator = iter(d)
        try:
            d *= count
            items = list(d)
        except Exception as error:
            items = 'error ' + type(error).__name__
        try:
            value = next(iterator)
        except StopIteration:
            value = 'stop'
        except Exception as error:
            value = type(error).__name__
        emit('repeat', maxlen=maxlen, length=length, count=count, items=items, next=value)

# The product of a full deque holds the same items in a new deque.
d = deque([0, 1, 2], maxlen=3)
product = d * 2
emit('product', items=list(product), maxlen=product.maxlen)
