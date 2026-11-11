def boom(value):
    raise ValueError('boom')


try:
    a, b = map(boom, [1])
except ValueError as error:
    print('caught', type(error).__name__, error)
