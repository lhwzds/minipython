value = 10**30
class C:
    def f(self): return value
    p = property(f)
    s = staticmethod(f)
    c = classmethod(f)
for key in ('p', 's', 'c'):
    x = C.__dict__[key]
    emit('identity', key=key, same=x is x, self_not=x is not x)
emit('descriptor_value', property_value=C().p, static_value=C.s(C()), class_value=C.c())
