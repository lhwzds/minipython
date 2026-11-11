class E(Exception):
    pass

class S:
    def __str__(self):
        return 's'

class D(dict):
    __setitem__ = print

err = E('x')
try:
    exec('y = S()\nraise err', {'S': S, 'err': err}, D())
except E as caught:
    print('caught', caught is err)
print('end')
