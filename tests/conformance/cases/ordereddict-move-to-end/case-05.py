KEY = 'b'
LAST = True
from collections import OrderedDict
od = OrderedDict(a=1, b=2, c=3); od.move_to_end(KEY, last=LAST)
emit('result', keys=list(od), unpacked=list({**od}))
