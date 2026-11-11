value = -3
log = CallLogger(); a = Raises('a', log, value); b = Normal('b', log, value); d = {a: 1}
try: d[b] = 2
except Exception as exc: emit_exception(exc)
emit('state', length=len(d)); emit_calls(log)
