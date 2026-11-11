class Boom(Exception):
    pass


class Iterable:
    def __iter__(self):
        raise Boom('boom')


for join in ("'abc'.join", "b'abc'.join", "bytearray(b'abc').join"):
    try:
        eval(join)(Iterable())
    except Boom as error:
        print('caught', type(error).__name__, error)

try:
    bytearray(b'abc').extend(Iterable())
except Boom as error:
    print('caught', type(error).__name__, error)
