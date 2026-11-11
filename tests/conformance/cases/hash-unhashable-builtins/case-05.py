value = [1]
try: hash(value)
except Exception as exc: emit_exception(exc)
