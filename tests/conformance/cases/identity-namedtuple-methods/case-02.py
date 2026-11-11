value = "非ASCII"
from collections import namedtuple
P = namedtuple('P', 'x')
p = P(value)
a = p._replace
b = p._replace
emit('identity', separate=a is b, separate_not=a is not b, self=a is a, self_not=a is not a)
emit('method_result', result=p._replace(x=value).x)
