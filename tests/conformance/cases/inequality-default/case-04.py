value = 2**63 - 1
log = CallLogger(); a = Normal('a', log, value); b = WrongType('b', log, value)
emit('result', result=a != b)
emit_calls(log)
