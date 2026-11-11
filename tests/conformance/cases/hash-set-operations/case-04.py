value = frozenset({1, 2})
log = CallLogger(); x = ChangingHash('x', log); s = {x, value}
x.current_hash = 2
emit('changed', present=x in s, frozen_present=value in s)
emit_calls(log)
