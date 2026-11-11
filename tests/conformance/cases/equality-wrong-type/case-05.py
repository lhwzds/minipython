class Child(WrongType):
    def __eq__(self, other):
        self.logger.record('__eq__', self, other)
        return 'subclass result'

log = CallLogger(); a = WrongType('a', log); b = Child('b', log)
emit('result', result=a == b); emit_calls(log)
