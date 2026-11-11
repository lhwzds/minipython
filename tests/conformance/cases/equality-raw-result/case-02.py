class Result:
    def __bool__(self): raise ValueError('truth tested')
class Equal:
    def __eq__(self, other): return Result()
x = Equal(); y = Equal()
try:
    emit('list', equal=[x] == [y])
except Exception as exc:
    emit_exception(exc)
