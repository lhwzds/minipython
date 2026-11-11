value = 7
log = CallLogger(); x = Raises('x', log, value); d = {x: 42}
emit('lookup', result=d[x]); emit_calls(log)
