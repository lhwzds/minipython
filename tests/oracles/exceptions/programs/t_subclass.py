class MyKey(KeyError):
    pass
class Missing(AttributeError):
    pass
try:
    raise MyKey('x')
except LookupError as error:
    print('lookup', type(error).__name__)
K = MyKey
try:
    raise MyKey('y')
except (ValueError, K) as error:
    print('tuple alias', type(error).__name__)
try:
    try:
        raise KeyError('z')
    except K:
        print('wrong')
except KeyError as error:
    print('base not caught by subclass', repr(error))
print(isinstance(MyKey(), LookupError), issubclass(MyKey, (ValueError, LookupError)))
class Getter:
    def __getattr__(self, name):
        raise Missing(name)
print(getattr(Getter(), 'gone', 'default'), hasattr(Getter(), 'gone'))
class Exhausted(StopIteration):
    pass
class Countdown:
    def __init__(self, start):
        self.left = start
    def __iter__(self):
        return self
    def __next__(self):
        if not self.left:
            raise Exhausted('done')
        self.left -= 1
        return self.left
print(list(Countdown(4)), sum(Countdown(3)), [x for x in Countdown(2)])
