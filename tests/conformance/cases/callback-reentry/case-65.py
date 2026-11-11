e = ValueError('x')
for i in range(5):
    try:
        raise e
    except ValueError:
        pass
first = e.__traceback__
shared = first.tb_next
e.__traceback__ = None
first = None
count = 0
tb = shared
while tb is not None:
    count += 1
    tb = tb.tb_next
print(count)
print(shared is shared.tb_next)
