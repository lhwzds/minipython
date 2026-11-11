tail = ValueError("tail")
middle = ValueError("middle")
middle.__notes__ = tail
head = ValueError("head")
head.__notes__ = middle
del tail, head
print(middle.__notes__.args)
