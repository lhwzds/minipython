value = lambda log: Normal('normal', log, 0)
log = CallLogger(); x = value(log)
emit('result', lists=[x] == [x], tuples=(x,) == (x,))
emit_calls(log)
