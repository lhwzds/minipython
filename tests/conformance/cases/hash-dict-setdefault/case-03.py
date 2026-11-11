value = -3
log = CallLogger(); x = Normal('x', log, value); d = {}; d.setdefault(x, 1); emit_calls(log)
log.calls = []; d.setdefault(x, 2); emit_calls(log)
