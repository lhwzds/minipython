value = lambda log: WrongType('wrong', log, True)
log = CallLogger(); x = value(log)
emit('result', lists=[x] == [x], tuples=(x,) == (x,))
emit_calls(log)
