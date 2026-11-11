value = 0
log = CallLogger(); a = [1, 2]; x = Mutating('x', log, a); a.append(x); other = [1, 2, 0]
emit('result', equal=a == other, length=len(a)); emit_calls(log)
