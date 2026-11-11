d = {1: 1, 2: 2}
try:
    for a in d:
        d.clear()
except RuntimeError as e:
    print('caught', str(e))
print('after', len(d))
