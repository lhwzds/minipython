from collections import ChainMap, UserDict
v = 2
for cls in (ChainMap, UserDict):
    item = cls({'x': v}); same = {'x': v}; different = {'x': v + 1}
    emit('result', name=cls.__name__, forward=item == same, reverse=same == item)
    emit('result', name=cls.__name__, forward=item == different, reverse=different == item)
