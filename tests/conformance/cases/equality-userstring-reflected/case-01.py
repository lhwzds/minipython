from collections import UserString
class Always:
    def __eq__(self, other): return True
    def __ne__(self, other): return False
emit('comparison', equal=UserString('a') == Always(), unequal=UserString('a') != Always())
