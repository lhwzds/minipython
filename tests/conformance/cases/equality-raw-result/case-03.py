class Result:
    def __bool__(self): raise ValueError('truth tested')
class Equal:
    def __eq__(self, other): return Result()
x = Equal(); y = Equal()
try:
    emit('membership', member=x in [y])
except Exception as exc:
    emit_exception(exc)
try:
    emit('index', index=[y].index(x))
except Exception as exc:
    emit_exception(exc)
try:
    emit('count', count=[y].count(x))
except Exception as exc:
    emit_exception(exc)
