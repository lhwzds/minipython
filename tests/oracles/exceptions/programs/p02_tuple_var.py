errors = (ValueError, KeyError)
for exc in (ValueError("a"), KeyError("b")):
    try:
        raise exc
    except errors as e:
        print("caught", type(e).__name__)
nested = (TypeError, (ValueError, (KeyError,)))
try:
    raise KeyError(1)
except nested as e:
    print("nested", repr(e))
