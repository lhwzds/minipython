from collections import UserString
class Recorder:
    def __init__(self): self.operands = []
    def __eq__(self, other):
        self.operands.append(type(other).__name__)
        return True
recorder = Recorder()
emit('comparison', equal=UserString('a') == recorder)
emit('operands', names=recorder.operands)
