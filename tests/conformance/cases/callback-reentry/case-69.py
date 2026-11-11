tail = ValueError("tail")
middle = ValueError("middle")
middle.__context__ = tail
head = ValueError("head")
head.__context__ = middle
del tail, head
print(middle.__context__.args)
