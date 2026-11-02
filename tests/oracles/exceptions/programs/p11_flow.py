E = KeyError
def f(n):
    for i in range(n):
        try:
            if i == 1:
                raise E(i)
            if i == 2:
                raise ValueError(i)
        except E:
            print("E at", i)
            continue
        except (ValueError,) as e:
            print("V at", i)
            return "ret"
        else:
            print("else", i)
        finally:
            print("finally", i)
print(f(4))
