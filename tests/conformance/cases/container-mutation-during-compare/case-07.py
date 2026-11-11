value = 0
log = CallLogger(); a = {'k': 1}; x = Mutating('x', log, a); a['j'] = x; y = value
emit('result', found=('j', y) in a.items(), length=len(a)); emit_calls(log)
