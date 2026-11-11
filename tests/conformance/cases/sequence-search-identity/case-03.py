value = lambda log: NotImplementedObject('declines', log, 2**63 - 1)
log = CallLogger(); x = value(log); a = [x, x]
emit('search', count=a.count(x), index=a.index(x))
a.remove(x)
emit('remaining', length=len(a)); emit_calls(log)
