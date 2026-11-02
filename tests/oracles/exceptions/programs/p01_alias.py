E = ValueError
try:
    raise ValueError("x")
except E:
    print("alias caught")
