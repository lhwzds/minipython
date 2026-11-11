value = 10**30
log = CallLogger(); a = Normal('a', log, value); b = value
emit('result', result=a != b)
emit_calls(log)
