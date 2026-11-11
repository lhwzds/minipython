value = 0
log = CallLogger(); a = [1, 2]; x = Mutating('x', log, a); a.append(x); y = value
emit('result', count=a.count(y), length=len(a)); emit_calls(log)
