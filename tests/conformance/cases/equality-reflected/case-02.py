value = 7
log = CallLogger(); a = NotImplementedObject('a', log, value); b = Normal('b', log, value + 1)
emit('result', result=a == b)
emit_calls(log)
