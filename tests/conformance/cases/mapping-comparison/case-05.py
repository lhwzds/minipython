value = 10**30
log = CallLogger(); a = {'probe': NotImplementedObject('left', log, value)}; b = {'probe': Normal('right', log, value + 1)}
emit('result', equal=a == b, unequal=a != b); emit_calls(log)
