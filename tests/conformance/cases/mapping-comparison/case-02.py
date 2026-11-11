value = "非ASCII"
a = {'text': value}; b = {'text': ''}
emit('result', equal=a == b, unequal=a != b)
