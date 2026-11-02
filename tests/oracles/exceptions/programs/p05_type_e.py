try:
    try:
        raise KeyError("k")
    except Exception as e:
        first = e
        raise
except type(first) as e2:
    print("type(e)", e2 is first)
