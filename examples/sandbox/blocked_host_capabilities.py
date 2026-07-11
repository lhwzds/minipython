"""Show the host capabilities that MiniPython intentionally does not expose."""

for name in ["open", "input"]:
    try:
        eval(name)
    except NameError:
        print("builtin", name, "blocked")
    else:
        print("builtin", name, "available")

for name in ["os", "socket", "subprocess", "_ctypes"]:
    try:
        __import__(name)
    except ModuleNotFoundError:
        print("module", name, "blocked")
    else:
        print("module", name, "available")
