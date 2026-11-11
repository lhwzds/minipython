value = lambda log: Raises('raises', log, -3)
log = CallLogger(); x = value(log)
emit('result', lists=[x] == [x], tuples=(x,) == (x,))
emit_calls(log)
