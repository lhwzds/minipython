from collections import ChainMap
c = ChainMap()
emit('chainmap', same=c is c, equal=c == ChainMap())
