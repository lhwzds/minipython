e = ValueError('x')
for i in range(5):
    try:
        raise e
    except ValueError:
        pass
tb = e.__traceback__
print(tb is e.__traceback__)
count = 0
while tb is not None:
    count += 1
    tb = tb.tb_next
print(count)
print(e.with_traceback(e.__traceback__) is e)
