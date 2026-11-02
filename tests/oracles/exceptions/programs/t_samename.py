import re
class error(Exception):
    pass
try:
    raise error('mine')
except re.error:
    print('wrong: re.error caught a user error')
except error:
    print('user error')
try:
    re.compile('(')
except error:
    print('wrong: user error caught re.error')
except re.error as caught:
    print('re.error', type(caught).__name__, type(caught) is re.PatternError)
print(isinstance(error(), re.error), isinstance(re.error('m'), error), issubclass(error, re.error))
def make():
    class Boom(Exception):
        pass
    return Boom
A = make()
B = make()
print(A.__name__ == B.__name__, A is B)
try:
    raise A('a')
except B:
    print('wrong: B caught A')
except A:
    print('A caught by A')
try:
    raise B('b')
except A:
    print('wrong: A caught B')
except B:
    print('B caught by B')
print(isinstance(A(), B), isinstance(B(), A), issubclass(A, B))
class ValueError(Exception):
    pass
import builtins
try:
    raise ValueError('shadow')
except builtins.ValueError:
    print('wrong: builtin caught the shadowing class')
except ValueError as caught:
    print('shadowing class', type(caught) is ValueError)
try:
    int('x')
except ValueError:
    print('wrong: shadowing class caught the builtin')
except builtins.ValueError as caught:
    print('builtin', type(caught) is builtins.ValueError)
print(isinstance(ValueError(), builtins.ValueError), issubclass(ValueError, builtins.ValueError))
class AttributeError(Exception):
    pass
class Getter:
    def __getattr__(self, name):
        raise AttributeError(name)
try:
    getattr(Getter(), 'missing', 'default')
except AttributeError as caught:
    print('getattr propagates the shadowing class', caught)
try:
    hasattr(Getter(), 'missing')
except AttributeError as caught:
    print('hasattr propagates the shadowing class', caught)
class StopIteration(Exception):
    pass
class Counter:
    def __iter__(self):
        return self
    def __next__(self):
        raise StopIteration
try:
    list(Counter())
except StopIteration:
    print('list propagates the shadowing class')
