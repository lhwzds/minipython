log = CallLogger(); a = NotImplementedObject('a', log)
b = Normal('b', log); emit('result', equal=a == b); emit_calls(log)
