value = 10**30
log = CallLogger(); a = Normal('a', log, value); b = Normal('b', log, value)
hash((a, b)); emit_calls(log)
