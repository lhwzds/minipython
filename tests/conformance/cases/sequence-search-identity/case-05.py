value = lambda log: Mutating('mutates', log, [10**30])
log = CallLogger(); x = value(log); a = [x, x]
emit('search', count=a.count(x), index=a.index(x))
a.remove(x)
emit('remaining', length=len(a)); emit_calls(log)
