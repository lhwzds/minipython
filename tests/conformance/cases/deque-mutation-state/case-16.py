from collections import deque

d = deque([1, 2, 3])
for name, iterator in [('iter', iter(d)), ('reversed', reversed(d)), ('dunder', d.__reversed__())]:
    for kind, text in [('repr', repr(iterator)), ('str', str(iterator))]:
        head, separator, tail = text.partition(' at ')
        emit(kind, name=name, type=type(iterator).__name__, text=head, has_address=bool(separator))
