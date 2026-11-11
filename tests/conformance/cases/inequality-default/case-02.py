value = 7
log = CallLogger(); a = Normal('a', log, value); b = Normal('b', log, value + 1)
emit('result', result=a != b)
emit_calls(log)
