log = CallLogger(); a = WrongType('a', log)
b = Normal('b', log); emit('result', result=[a] == [b]); emit_calls(log)
