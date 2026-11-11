class Custom(Exception):
    pass


error = Custom('key')
events = []


def key(value):
    events.append(value)
    raise error


try:
    sorted([3, 1, 2], key=key)
    events.append('after')
except Custom as caught:
    print(type(caught).__name__, str(caught), caught is error)
print(events)
