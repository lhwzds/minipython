value = -3
class Base:
    def __eq__(self, other):
        emit('call', receiver='base', value=value); return NotImplemented
class Child(Base):
    def __eq__(self, other):
        emit('call', receiver='child', value=value); return self.answer
    answer = value == 0
emit('result', result=Base() == Child())
