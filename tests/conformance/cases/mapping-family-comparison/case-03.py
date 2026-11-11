from collections import Counter, OrderedDict, ChainMap, UserDict
for cls in (Counter, OrderedDict, ChainMap, UserDict):
    item = cls({'x': 1, 'y': 2})
    emit('result', name=cls.__name__, different_key=item == {'x': 1, 'z': 2}, reverse={'x': 1, 'z': 2} == item)
