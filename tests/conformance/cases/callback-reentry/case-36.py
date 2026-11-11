from collections import UserString


class Index:
    def __index__(self):
        return 1


text = UserString('abcabc')
print(text.find('b', Index()))
print(text.rfind('b', Index()))
print(text.count('b', Index()))
print(text.index('b', Index()))
print(text.rindex('b', Index()))
print(text.startswith('a', Index()))
print(text.endswith('c', Index()))
