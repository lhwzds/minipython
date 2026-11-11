from collections import ChainMap, Counter, OrderedDict, defaultdict, UserDict, UserList


class Probe:
    def __repr__(self):
        return '<probe>'


probe = Probe()
print(repr([probe]))
print(repr({'key': probe}))
print(repr(OrderedDict([('key', probe)])))
print(repr(OrderedDict([('key', probe)]).items()))
print(repr(ChainMap({'key': probe})))
print(repr(Counter({'key': probe})))
print(repr(defaultdict(list, {'key': probe})))
print(repr(UserDict({'key': probe})))
print(repr(UserList([probe])))
