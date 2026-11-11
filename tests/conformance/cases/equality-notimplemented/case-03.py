log = CallLogger(); a = NotImplementedObject('a', log)
b = NotImplementedObject('b', log); emit('result', unequal=a != b); emit_calls(log)
