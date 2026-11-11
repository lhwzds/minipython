from collections import UserString
emit('attribute', name=type(UserString.__abstractmethods__).__name__,
     same=UserString.__abstractmethods__ is UserString.__abstractmethods__)
