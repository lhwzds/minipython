value = 2**63 - 1
log = CallLogger(); a = {'probe': Normal('left', log, value)}; b = {'probe': Normal('right', log, value)}
emit('result', equal=a == b, unequal=a != b); emit_calls(log)
