def get():
    print("evaluating")
    return ZeroDivisionError
try:
    1/0
except get() as e:
    print("call caught", e)
try:
    pass
except get():
    print("never")
print("no eval when no exception")
