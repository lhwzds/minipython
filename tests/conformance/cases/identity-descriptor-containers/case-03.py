def f(): pass
a = classmethod(f); b = classmethod(f)
emit('containers', list_equal=[a] == [b], member=b in [a], tuple_equal=(a,) == (b,))
emit('search', count=[a].count(b))
try:
    [a].index(b)
except ValueError as exc:
    emit_exception(exc)
