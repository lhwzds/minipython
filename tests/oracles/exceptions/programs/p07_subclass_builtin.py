class MyKey(KeyError):
    pass
try:
    raise MyKey("x")
except LookupError as e:
    print("lookup", type(e).__name__)
K = MyKey
try:
    raise MyKey("y")
except (ValueError, K) as e:
    print("alias subclass", type(e).__name__)
try:
    raise KeyError("z")
except K:
    print("WRONG")
except KeyError:
    print("base not caught by subclass")
