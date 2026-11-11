value = lambda x: hash((x,))
class C:
    def __eq__(self, other): return True
try: value(C())
except Exception as exc: emit_exception(exc)
