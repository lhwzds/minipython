"""Generate randomized csv/StringIO differential programs (literal data only)."""
import random, sys, os
out = sys.argv[1]; count = int(sys.argv[2]); seed = int(sys.argv[3])
rng = random.Random(seed)
POOL = list('abcxyz019 .-') + [',', ';', '\t', '|', ':', '"', "'", '\\', '/', '\n', '\r', '\r\n', 'é', '中', '\x00', '  ', '""']
def field():
    k = rng.random()
    if k < 0.1: return ''
    if k < 0.15: return None
    if k < 0.22: return rng.choice([0, 1, -3, 2.5, 1e20, 0.1, True, 10**20])
    return ''.join(rng.choice(POOL) for _ in range(rng.randint(1, 6)))
def dialect():
    d = {}
    if rng.random() < .7: d['delimiter'] = rng.choice([',', ';', '\t', '|', ' ', ':', 'é'])
    if rng.random() < .5: d['quotechar'] = rng.choice(['"', "'", None, '|', '\n'])
    if rng.random() < .5: d['escapechar'] = rng.choice([None, '\\', '/', ','])
    if rng.random() < .4: d['doublequote'] = rng.random() < .5
    if rng.random() < .4: d['skipinitialspace'] = rng.random() < .5
    if rng.random() < .5: d['lineterminator'] = rng.choice(['\r\n', '\n', '\r', '|', ';;', ''])
    if rng.random() < .6: d['quoting'] = rng.randint(0, 5)
    if rng.random() < .3: d['strict'] = rng.random() < .5
    return d
HARNESS = r'''import csv, io
def rows_of(r):
    out = []
    try:
        for row in r:
            out.append((r.line_num, row))
    except csv.Error as e:
        out.append(('Error', r.line_num, str(e)))
    except Exception as e:
        out.append((type(e).__name__, str(e)))
    return out
def run(rows, kw, raw):
    try:
        s = io.StringIO()
        w = csv.writer(s, **kw)
    except Exception as e:
        print('dialect', type(e).__name__, e)
        return
    res = []
    for row in rows:
        try:
            res.append(w.writerow(row))
        except csv.Error as e:
            res.append(('Error', str(e)))
    text = s.getvalue()
    print('w', repr(text), res)
    print('r1', rows_of(csv.reader(io.StringIO(text, newline=''), **kw)))
    print('r2', rows_of(csv.reader(text.splitlines(True), **kw)))
    print('r3', rows_of(csv.reader(io.StringIO(raw, newline=''), **kw)))
    print('r4', rows_of(csv.reader(raw.split('\n'), **kw)))
    try:
        d = csv.DictReader(io.StringIO(text, newline=''), restkey='_', **kw)
        print('d', list(d), d.line_num)
    except Exception as e:
        print('d', type(e).__name__, e)
def sniff(text):
    sn = csv.Sniffer()
    for delims in (None, ',;\t'):
        try:
            d = sn.sniff(text, delims)
            print('s', repr(d.delimiter), repr(d.quotechar), d.doublequote, d.skipinitialspace)
        except csv.Error as e:
            print('s', 'Error', e)
    try:
        print('h', sn.has_header(text))
    except Exception as e:
        print('h', type(e).__name__, e)
    print('g', sn._guess_delimiter(text, None))
def sio(nl, init, ops):
    s = io.StringIO(init, nl)
    out = []
    for op, arg in ops:
        try:
            if op == 'write': out.append(s.write(arg))
            elif op == 'read': out.append(s.read(arg))
            elif op == 'readline': out.append(s.readline(arg))
            elif op == 'seek': out.append(s.seek(arg))
            elif op == 'truncate': out.append(s.truncate(arg))
            elif op == 'tell': out.append(s.tell())
            elif op == 'readlines': out.append(s.readlines(arg))
            elif op == 'getvalue': out.append(s.getvalue())
        except (ValueError, OSError, TypeError) as e:
            out.append((type(e).__name__, str(e)))
    print('io', repr(nl), out, s.newlines, s.tell(), repr(s.getvalue()))
'''
os.makedirs(out, exist_ok=True)
for n in range(count):
    lines = [HARNESS]
    kind = n % 3
    for _ in range(12):
        if kind == 0:
            rows = [[field() for _ in range(rng.randint(0, 5))] for _ in range(rng.randint(1, 4))]
            raw = ''.join(rng.choice(POOL) for _ in range(rng.randint(0, 30)))
            lines.append('run(%r, %r, %r)' % (rows, dialect(), raw))
        elif kind == 1:
            delim = rng.choice([',', ';', '\t', '|', ' ', ':'])
            q = rng.choice(['"', "'"])
            nrows = rng.randint(1, 25)
            ncols = rng.randint(1, 5)
            def cell(r):
                k = rng.random()
                if r == 0: return rng.choice(['name', 'age', 'city', 'id', 'x', 'value', 'a b'])
                if k < .4: return str(rng.randint(-100, 10000))
                if k < .5: return '%.2f' % rng.uniform(-10, 10)
                if k < .6: return q + rng.choice(['x', 'y z', delim + 'w', 'q' + q + q + 'r']) + q
                return ''.join(rng.choice('abcdefgh ') for _ in range(rng.randint(0, 8)))
            sp = rng.choice(['', ' '])
            text = '\n'.join((delim + sp).join(cell(r) for _ in range(ncols)) for r in range(nrows))
            if rng.random() < .3: text += '\n'
            lines.append('sniff(%r)' % text)
        else:
            ops = []
            for _ in range(rng.randint(1, 10)):
                op = rng.choice(['write', 'write', 'read', 'readline', 'seek', 'truncate', 'tell', 'readlines', 'getvalue'])
                if op == 'write': arg = ''.join(rng.choice(['a', 'b', 'é', '\n', '\r', '\r\n', '中', 'xyz']) for _ in range(rng.randint(0, 5)))
                elif op == 'seek': arg = rng.randint(0, 12)
                elif op in ('read', 'readline', 'readlines'): arg = rng.choice([None, -1, 0, 1, 2, 5])
                elif op == 'truncate': arg = rng.choice([None, 0, 2, 20])
                else: arg = None
                ops.append((op, arg))
            nl = rng.choice([None, '', '\n', '\r', '\r\n'])
            init = ''.join(rng.choice(['a', '\n', '\r', '\r\n', 'é']) for _ in range(rng.randint(0, 6)))
            lines.append('sio(%r, %r, %r)' % (nl, init, ops))
    with open(os.path.join(out, 'case_%04d.py' % n), 'w') as f:
        f.write('\n'.join(lines) + '\n')
