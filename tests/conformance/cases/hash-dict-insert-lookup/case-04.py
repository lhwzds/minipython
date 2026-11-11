value = 2**63 - 1
log = CallLogger(); x = Normal('x', log, value); d = {}; d[x] = 1; emit_calls(log)
log.calls = []; emit('lookup', value=d[x]); emit_calls(log)
