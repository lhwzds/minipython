class Custom(Exception):
    pass


error = Custom('throw')
events = []


def source():
    try:
        yield 'ready'
        events.append('after yield')
    except Custom as caught:
        print(type(caught).__name__, str(caught), caught is error)
        yield 'handled'
    finally:
        events.append('finally')


gen = source()
print(next(gen))
print(gen.throw(error))
print(next(gen, 'done'))
print(events)
