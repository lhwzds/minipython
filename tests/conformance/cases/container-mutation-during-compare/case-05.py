from collections import deque
value = 0
log = CallLogger(); a = deque([1, 2]); x = Mutating('x', log, a); a.append(x); y = value
try:
    emit('result', found=y in a)
except RuntimeError as error:
    emit_exception(error)
emit('result', length=len(a)); emit_calls(log)
