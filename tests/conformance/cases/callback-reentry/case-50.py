class Custom(BaseException):
    pass


error = Custom('orig')
events = []


class Hook:
    def __bool__(self):
        events.append('bool')
        raise error


try:
    if Hook():
        events.append('branch')
    events.append('after')
except BaseException as caught:
    print(type(caught).__name__, str(caught), caught is error)
print(events)
