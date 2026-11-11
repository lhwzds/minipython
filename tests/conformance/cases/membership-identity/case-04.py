value = 2**63 - 1
log = CallLogger(); x = Raises('x', log, value)
container = frozenset([x])
emit('result', result=x in container)
emit_calls(log)
