value = -3
log = CallLogger(); a = Raises('a', log, value); b = Normal('b', log, value); d = {a: 1}
try: emit('result', result=d[b])
except Exception as exc: emit_exception(exc)
emit_calls(log)
