value = lambda x: frozenset([x])
class C:
    __hash__ = None
try: value(C())
except Exception as exc: emit_exception(exc)
