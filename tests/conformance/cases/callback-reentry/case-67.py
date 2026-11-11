e = ValueError("x")
e.__cause__ = e
held = t"{e}"
before = id(held)
try:
    raise e
except ValueError:
    pass
after_raise = id(held)
e.add_note("note")
after_note = id(held)
print(before == after_raise, before == after_note)
