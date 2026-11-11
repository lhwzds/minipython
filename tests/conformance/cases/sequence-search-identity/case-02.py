value = lambda log: Raises('raises', log, -3)
log = CallLogger(); x = value(log); a = [x, x]
emit('search', count=a.count(x), index=a.index(x))
a.remove(x)
emit('remaining', length=len(a)); emit_calls(log)
