from collections import UserList
emit('attribute', name=type(UserList.__abstractmethods__).__name__,
     same=UserList.__abstractmethods__ is UserList.__abstractmethods__)
