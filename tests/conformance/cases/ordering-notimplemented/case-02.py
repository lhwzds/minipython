value = "非ASCII"
log = CallLogger(); a = NotImplementedObject('a', log); b = NotImplementedObject('b', log)
try:
    emit('result', result=a < b)
except Exception as exc:
    emit_exception(exc)
try: emit('result', builtin_result=a < value)
except Exception as exc: emit_exception(exc)
emit_calls(log)
