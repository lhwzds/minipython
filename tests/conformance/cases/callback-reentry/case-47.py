try:
    try:
        raise KeyError('first')
    except KeyError:
        {[]}
except TypeError as error:
    print(type(error).__name__, str(error))
    print(type(error.__context__).__name__, str(error.__context__))
