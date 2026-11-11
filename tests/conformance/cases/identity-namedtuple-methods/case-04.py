value = True
from collections import namedtuple
P = namedtuple('P', 'x')
p = P(value)
a = p.count
b = p.count
emit('identity', separate=a is b, separate_not=a is not b, self=a is a, self_not=a is not a)
emit('method_result', result=p.count(value))
