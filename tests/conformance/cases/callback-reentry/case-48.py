class Custom(Exception):
    pass


raised = []


class AttributeSource:
    @property
    def keys(self):
        error = Custom('orig')
        raised.append(error)
        raise error


class CallSource:
    def keys(self):
        error = Custom('orig')
        raised.append(error)
        raise error


class ItemSource:
    def keys(self):
        return ['a']

    def __getitem__(self, key):
        error = Custom('orig')
        raised.append(error)
        raise error


for source in (AttributeSource(), CallSource(), ItemSource()):
    try:
        {**source}
    except Custom as error:
        print('caught', type(error).__name__, error, error is raised[-1])
