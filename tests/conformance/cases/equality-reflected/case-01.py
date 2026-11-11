value = 0
log = CallLogger(); a = NotImplementedObject('a', log, value); b = Normal('b', log, value)
emit('result', result=a == b)
emit_calls(log)
