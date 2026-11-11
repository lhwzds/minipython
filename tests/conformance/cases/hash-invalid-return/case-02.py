value = 0.0
class InvalidHash(WrongType):
    def __hash__(self):
        self.logger.record('__hash__', self)
        return value
log = CallLogger(); x = InvalidHash('x', log)
try: hash(x)
except Exception as exc: emit_exception(exc)
emit_calls(log)
