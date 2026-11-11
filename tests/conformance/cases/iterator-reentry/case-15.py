class Source:
    armed = True
    def __getitem__(self, index):
        if index > 2: raise IndexError
        if self.armed:
            self.armed = False
            try: print('inner', next(t))
            except BaseException as e: print('inner', type(e).__name__)
        return index
t = iter(Source())
print('outer', next(t))
