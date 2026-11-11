try:
    {[]}
except TypeError as error:
    print('caught', type(error).__name__)
try:
    {[]: 1}
except TypeError as error:
    print('caught', type(error).__name__)
