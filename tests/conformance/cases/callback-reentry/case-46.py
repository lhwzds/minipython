def fail(value):
    raise ValueError('boom')


def inner():
    first, second = map(fail, [1])


def outer():
    inner()


try:
    outer()
except ValueError as error:
    traceback = error.__traceback__
    depth = 0
    while traceback is not None:
        depth += 1
        traceback = traceback.tb_next
    print(type(error).__name__, str(error), depth)
    saved = error.__traceback__
    try:
        raise error
    except ValueError as raised:
        print(raised.__traceback__.tb_next is saved)
