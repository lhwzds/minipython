value = 0
from collections import namedtuple
P = namedtuple('P', 'x')
a = P._make
b = P._make
emit('identity', separate=a is b, separate_not=a is not b, self=a is a, self_not=a is not a)
emit('method_result', result=P._make([value]).x)
