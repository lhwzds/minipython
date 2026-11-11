# Behaviour probes shared by the two interpreters.
class CallLogger:
    def __init__(self):
        self.calls = []

    def record(self, method, *operands):
        self.calls.append([method] + [getattr(x, 'tag', x) for x in operands])


class Normal:
    def __init__(self, tag, logger, value=0):
        self.tag, self.logger, self.value = tag, logger, value

    def __repr__(self):
        return '<probe ' + self.tag + '>'

    def __eq__(self, other):
        self.logger.record('__eq__', self, other)
        if not isinstance(other, Normal):
            return NotImplemented
        return self.value == other.value

    def __lt__(self, other):
        self.logger.record('__lt__', self, other)
        if not isinstance(other, Normal):
            return NotImplemented
        return self.value < other.value

    def __hash__(self):
        self.logger.record('__hash__', self)
        return self.value


class Raises(Normal):
    def __eq__(self, other):
        self.logger.record('__eq__', self, other)
        raise ValueError('comparison failed')

    __hash__ = Normal.__hash__


class NotImplementedObject(Normal):
    def __eq__(self, other):
        self.logger.record('__eq__', self, other)
        return NotImplemented

    def __lt__(self, other):
        self.logger.record('__lt__', self, other)
        return NotImplemented

    __hash__ = Normal.__hash__


class WrongType(Normal):
    def __eq__(self, other):
        self.logger.record('__eq__', self, other)
        return 'truthy'

    def __hash__(self):
        self.logger.record('__hash__', self)
        return 'invalid'


class Mutating(Normal):
    def __init__(self, tag, logger, container):
        Normal.__init__(self, tag, logger)
        self.container = container

    def __eq__(self, other):
        self.logger.record('__eq__', self, other)
        self.container.clear()
        return False

    __hash__ = Normal.__hash__


class ChangingHash(Normal):
    def __init__(self, tag, logger):
        Normal.__init__(self, tag, logger)
        self.current_hash = 1

    def __hash__(self):
        self.logger.record('__hash__', self)
        return self.current_hash

    __eq__ = Normal.__eq__
