class C:
    @property
    def x(self):
        raise AttributeError("p")
    def __getattr__(self, name):
        raise AttributeError(name)
try:
    try:
        C().x
    except AttributeError as e:
        print("inner", e)
except AttributeError as e:
    print("outer", e)
class D:
    @property
    def x(self):
        raise AttributeError("p")
    def __getattr__(self, name):
        return 42
try:
    print(D().x)
except AttributeError as e:
    print("handler", e)
class F:
    @property
    def x(self):
        raise KeyError("p")
    def __getattr__(self, name):
        return 42
try:
    print(F().x)
except KeyError as e:
    print("key", e)
print("END")
