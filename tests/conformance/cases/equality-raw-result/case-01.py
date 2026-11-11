import operator
class Result:
    def __bool__(self): raise ValueError('truth tested')
class Equal:
    def __eq__(self, other): return Result()
x = Equal(); y = Equal()
emit('equality', name=type(x == y).__name__)
emit('operator', name=type(operator.eq(x, y)).__name__)
try:
    x != y
except Exception as exc:
    emit_exception(exc)
