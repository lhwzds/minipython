log = CallLogger(); a = Raises('a', log)
try: emit('result', result=a != 0)
except Exception as exc: emit_exception(exc)
emit_calls(log)
