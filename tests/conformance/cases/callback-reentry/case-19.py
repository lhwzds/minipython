from array import array
for make in [lambda: bytearray(b'ab'), lambda: array('b', [1, 2])]:
    a = make()
    class Index:
        def __index__(self):
            a.append(3)
            return 9
    a.insert(Index(), 42)
    print(list(a))
