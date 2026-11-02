import sys
methods = ["isspace", "isdigit", "isdecimal", "isnumeric", "isalpha", "isalnum", "isidentifier", "isprintable", "isupper", "islower", "istitle"]
lo = int(sys.argv[1]) if len(sys.argv) > 1 else 0
hi = int(sys.argv[2]) if len(sys.argv) > 2 else 0x110000
for name in methods:
    ranges = []
    start = None
    for cp in range(lo, hi):
        if 0xD800 <= cp <= 0xDFFF:
            v = False
        else:
            v = getattr(chr(cp), name)()
        if v and start is None:
            start = cp
        elif not v and start is not None:
            ranges.append((start, cp - 1)); start = None
    if start is not None:
        ranges.append((start, hi - 1))
    print(name, len(ranges), " ".join("%x-%x" % r for r in ranges))
