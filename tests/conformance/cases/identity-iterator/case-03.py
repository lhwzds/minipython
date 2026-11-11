value = 2**63 - 1
source = {value: 1}
x = iter(source)
emit('identity', self=x is x, self_not=x is not x, iterator=iter(x) is x, iterator_not=iter(x) is not x, distinct=x is iter(source))
emit('item', result=next(x))
