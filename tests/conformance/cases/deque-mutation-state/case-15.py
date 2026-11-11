from collections import deque

for maxlen in [None, 1, 2, 3, 4]:
    for length in [0, 1, 2, 3]:
        for count in [-1, 0, 1, 2, 3, 5]:
            d = deque(range(length), maxlen=maxlen)
            it = iter(d)
            d *= count
            try:
                value = next(it)
            except StopIteration:
                value = 'stop'
            except Exception as error:
                value = type(error).__name__
            emit('repeat', maxlen=maxlen, length=length, count=count, items=list(d), next=value)
