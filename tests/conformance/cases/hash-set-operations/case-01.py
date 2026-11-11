value = 7
log = CallLogger()
class HashRaises(Normal):
    def __hash__(self):
        self.logger.record('__hash__', self)
        raise ValueError('hash failed')
x = HashRaises('x', log, value)
s = {value}
emit('integer_present', result=value in s, size=len(s))
try:
    {x}
except ValueError as exc:
    emit_exception(exc)
emit_calls(log)
