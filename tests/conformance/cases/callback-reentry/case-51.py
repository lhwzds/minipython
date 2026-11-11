class Custom(Exception):
    pass


error = Custom('orig')


class Hook:
    def __iter__(self):
        yield 1

    def __len__(self):
        raise error


for operation in (bool, len):
    try:
        operation(Hook())
        print('ran after')
    except Custom as caught:
        print(type(caught).__name__, str(caught), caught is error)
