value = 0
log = CallLogger(); a = {'k': 1}; x = Mutating('x', log, a); a['j'] = x; y = value
try:
    emit('result', found=y in a.values())
except RuntimeError as error:
    emit_exception(error)
emit('result', length=len(a)); emit_calls(log)
