from collections import UserString
s = UserString('{}')
class T:
    def __format__(self, spec):
        s.data = 'changed'
        return 'value'
print(s.format(T()), s.data)
s.data = '{x}'
class Mapping:
    def __getitem__(self, key):
        s.data = 'again'
        return 'mapped'
print(s.format_map(Mapping()), s.data)
