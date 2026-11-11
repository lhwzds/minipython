value = 2**63 - 1
log = CallLogger(); x = Normal('x', log, value); d = {x: 1}; log.calls = []
emit('operation', name='copy'); a = d.copy(); emit_calls(log)
log.calls = []; emit('operation', name='dict'); b = dict(d); emit_calls(log)
log.calls = []; emit('operation', name='update'); c = {}; c.update(d); emit_calls(log)
emit('lengths', lengths=[len(a), len(b), len(c)])
