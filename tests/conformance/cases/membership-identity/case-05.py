value = 10**30
log = CallLogger(); x = Raises('x', log, value)
container = {x: 1}
emit('result', result=x in container)
emit_calls(log)
