log = CallLogger(); a = NotImplementedObject('a', log)
b = NotImplementedObject('b', log); emit('result', distinct=a == b); emit_calls(log)
