value = lambda log: NotImplementedObject('declines', log, 2**63 - 1)
log = CallLogger(); x = value(log)
emit('result', lists=[x] == [x], tuples=(x,) == (x,))
emit_calls(log)
