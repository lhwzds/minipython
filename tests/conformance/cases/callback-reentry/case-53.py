class Custom(Exception):
    pass


error = Custom('repr')


class Hook:
    def __repr__(self):
        raise error


for value in (Hook(), [Hook()], {'x': Hook()}):
    try:
        repr(value)
        print('ran after')
    except Custom as caught:
        print(type(caught).__name__, str(caught), caught is error)
