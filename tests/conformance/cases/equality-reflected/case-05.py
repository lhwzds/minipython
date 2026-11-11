value = 10**30
log = CallLogger(); a = NotImplementedObject('a', log, value); b = value
emit('result', result=a == b)
emit_calls(log)
