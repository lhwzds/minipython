class Reverse:
    def __init__(self, target):
        self.target = target

    def __bool__(self):
        self.target.clear()
        return True


items = [3, 1, 2]
print(sorted(items, reverse=Reverse(items)))
print(items)
