class Custom(Exception):
    pass


error = Custom('binary')
events = []


class Hook:
    def __add__(self, other):
        events.append('add')
        raise error


try:
    Hook() + 1
    events.append('after')
except Custom as caught:
    print(type(caught).__name__, str(caught), caught is error)
print(events)
