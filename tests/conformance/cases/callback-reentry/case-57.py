saved = []
try:
    try:
        {[]}
    except TypeError as a:
        saved.append(a)
        try:
            {[]}
        except TypeError as b:
            print(a is b)
        raise
except TypeError as c:
    print(c is saved[-1])
try:
    try:
        int("x")
    except ValueError as a:
        saved.append(a)
        try:
            int("x")
        except ValueError as b:
            print(a is b)
        raise
except ValueError as c:
    print(c is saved[-1])
print("END")
