log = CallLogger(); a = NotImplementedObject('a', log)
emit('result', same=a == a); emit_calls(log)
