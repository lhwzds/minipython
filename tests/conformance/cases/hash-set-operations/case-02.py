value = '非ASCII'
log = CallLogger(); x = Raises('x', log, 1); y = Normal('y', log, 1)
s = {x, value}
try:
    s.add(y)
except ValueError as exc:
    emit_exception(exc)
emit('string_present', result=value in s)
emit_calls(log)
