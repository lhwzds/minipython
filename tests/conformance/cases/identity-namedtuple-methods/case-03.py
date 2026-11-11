value = -0.0
from collections import namedtuple
P = namedtuple('P', 'x')
p = P(value)
a = p._asdict
b = p._asdict
emit('identity', separate=a is b, separate_not=a is not b, self=a is a, self_not=a is not a)
emit('method_result', result=p._asdict()['x'])
