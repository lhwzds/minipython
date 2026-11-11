value = (1, 'x')
log = CallLogger(); x = NotImplementedObject('x', log, 2); y = NotImplementedObject('y', log, 2)
s = {x, value}; emit('membership', result=y in s, tuple_present=value in s)
emit_calls(log)
