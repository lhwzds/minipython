from collections import UserString
class Always:
    def __eq__(self, other): return True
    def __ne__(self, other): return False
emit('text', equal=UserString('a') == UserString('a'), unequal=UserString('a') != UserString('b'))
emit('string', equal=UserString('a') == 'a', unequal=UserString('a') != 'b')
emit('reflected', equal=Always() == UserString('a'), unequal=Always() != UserString('a'))
