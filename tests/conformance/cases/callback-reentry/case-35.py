def boom(value):
    raise ValueError('boom')


try:
    try:
        a, b = map(boom, [1])
    finally:
        print('finally')
except ValueError as error:
    print('caught', type(error).__name__, error)
