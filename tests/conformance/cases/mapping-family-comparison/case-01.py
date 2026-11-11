from collections import Counter, OrderedDict
v = 1
for cls in (Counter, OrderedDict):
    item = cls({'x': v}); same = {'x': v}; different = {'x': v + 1}
    emit('result', name=cls.__name__, forward=item == same, reverse=same == item)
    emit('result', name=cls.__name__, forward=item == different, reverse=different == item)
