value = 7
log = CallLogger(); x = ChangingHash('x', log); x.current_hash = value; d = {x: 1}; x.current_hash = value + 1
emit_calls(log); log.calls.clear(); x in d; emit_calls(log)
