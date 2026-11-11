value = lambda log: Mutating('mutates', log, [10**30])
log = CallLogger(); x = value(log)
emit('result', lists=[x] == [x], tuples=(x,) == (x,))
emit_calls(log)
