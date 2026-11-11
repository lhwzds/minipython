value = 7
log = CallLogger(); x = Raises('x', log, value)
container = (x,)
emit('result', result=x in container)
emit_calls(log)
