import re
class error(Exception):
    pass
try:
    raise error("mine")
except re.error:
    print("WRONG: re.error caught user error")
except error:
    print("user error ok")
try:
    re.compile("(")
except error:
    print("WRONG: user error caught re.error")
except re.error as e:
    print("re.error ok", type(e).__name__)
def make():
    class Boom(Exception):
        pass
    return Boom
A = make(); B = make()
try:
    raise A()
except B:
    print("WRONG: B caught A")
except A:
    print("A ok")
