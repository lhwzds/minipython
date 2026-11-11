value = 0
log = CallLogger(); a = [1, 2]; x = Mutating('x', log, a); a.append(x); y = value
emit('result', found=y in a, length=len(a)); emit_calls(log)
