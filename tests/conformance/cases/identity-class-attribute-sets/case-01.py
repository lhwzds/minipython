from collections import UserDict
emit('attribute', name=type(UserDict.__abstractmethods__).__name__,
     same=UserDict.__abstractmethods__ is UserDict.__abstractmethods__)
