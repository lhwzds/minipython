value = lambda log: Raises('raises', log, -3)
log = CallLogger()
x = value(log)
a = x.__eq__
b = x.__eq__
emit('identity', same=a is a, self_not=a is not a, separate=a is b, separate_not=a is not b, equal=a == b)
try:
    result = a(Normal('other', log, 7))
    emit('result', declined=result is NotImplemented, result=None if result is NotImplemented else result)
except Exception as exc: emit_exception(exc)
emit_calls(log)
