value = 0
def f(): return value
class C: pass
x = [f, C, C()]
emit('identity', same=[v is v for v in x], self_not=[v is not v for v in x], different=f is C, returned=f())
