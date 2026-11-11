from collections import UserString
u = UserString('a')
emit('userstring', same=u is u, equal=u == UserString('a'))
