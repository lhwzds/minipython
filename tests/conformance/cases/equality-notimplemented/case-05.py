log = CallLogger(); a = NotImplementedObject('a', log)
class Answer(NotImplementedObject):
    def __eq__(self, other):
        self.logger.record('__eq__', self, other)
        return True
b = Answer('b', log); emit('result', equal=a == b); emit_calls(log)
