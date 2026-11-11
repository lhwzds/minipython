value = frozenset({3, 4})
log = CallLogger(); x = Normal('x', log, 3); s = {x, value}
log.calls = []; t = s.copy(); emit('copy', frozenset_present=value in t)
emit_calls(log)
log.calls = []; u = set(); u.update(s); emit('update', size=len(u)); emit_calls(log)
