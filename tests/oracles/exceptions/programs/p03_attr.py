class T:
    failureException = AssertionError
    def run(self):
        try:
            assert False, "boom"
        except self.failureException as e:
            print("attr caught", e)
T().run()
import types
mod = types.SimpleNamespace(Error=LookupError)
try:
    {}["k"]
except mod.Error as e:
    print("module attr", repr(e))
