value = 0
log = CallLogger(); a = [1, 2]; x = Mutating('x', log, a); a.append(x); y = value
try:
    emit('result', index=a.index(y))
except ValueError as error:
    emit_exception(error)
emit('result', length=len(a)); emit_calls(log)
