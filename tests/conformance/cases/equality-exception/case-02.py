log = CallLogger(); a = Raises('a', log)
try: emit('result', result=0 == a)
except Exception as exc: emit_exception(exc)
emit_calls(log)
