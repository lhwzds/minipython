tail = ValueError("tail")
middle = ValueError("middle")
middle.__cause__ = tail
head = ValueError("head")
head.__cause__ = middle
del tail, head
print(middle.__cause__.args)
