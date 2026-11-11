value = -3
log = CallLogger(); a = Normal('a', log, value); b = NotImplementedObject('b', log, value)
emit('result', result=a != b)
emit_calls(log)
