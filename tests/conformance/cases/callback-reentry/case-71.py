tail = ValueError('iterator tail')
middle = ValueError('iterator middle')
middle.__notes__ = iter((tail,))
head = ValueError('iterator head')
head.__notes__ = iter((middle,))
del tail, head
print(next(middle.__notes__).args)

tail = ValueError('template tail')
middle = ValueError('template middle')
middle.__notes__ = t'{tail}'
head = ValueError('template head')
head.__notes__ = t'{middle}'
del tail, head
print(middle.__notes__.interpolations[0].value.args)
