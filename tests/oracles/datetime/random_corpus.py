"""Randomized differential corpus for datetime (CPython vs mnpy).

Writes PROGRAMS program files; each prints one line per scenario.
usage: gen.py OUTDIR SEED PROGRAMS SCENARIOS_PER_PROGRAM

The scenarios whose result the platform C library decides are marked in a
`<program>.platform-keys.json` file next to each program it writes; the
corpus runner skips exactly those on the platforms the manifest lists for
the corpus (tests/README.md, "Platform-dependent behavior").
"""
import datetime as _datetime
import json
import random
import re
import sys

# The seed is set by `main`; this keeps the module importable, which the
# regression tests of the platform exclusions rely on.
rng = random.Random(0)

PRELUDE = r'''from datetime import date, time, datetime, timedelta, timezone, tzinfo, MINYEAR, MAXYEAR, UTC
import copy
def t(label, f):
    try:
        r = f()
        print(label, '->', repr(r))
    except Exception as e:
        print(label, '!!', type(e).__name__, e)
class Fixed(tzinfo):
    def __init__(self, minutes=0, name='F', dst=0):
        self.off = timedelta(minutes=minutes)
        self.name = name
        self.dstoff = timedelta(minutes=dst)
    def utcoffset(self, dt):
        return self.off
    def tzname(self, dt):
        return self.name
    def dst(self, dt):
        return self.dstoff
    def __repr__(self):
        return 'Fixed(%d)' % (self.off // timedelta(minutes=1))
'''


def rint(a, b):
    return rng.randint(a, b)


def rand_year():
    return rng.choice([rint(1, 9999), rint(1, 9999), rint(1900, 2100),
                       rng.choice([1, 2, 9998, 9999, 1970, 2000, 1600, 2400])])


def rand_date():
    y = rand_year(); m = rint(1, 12)
    d = rint(1, 28) if rng.random() < 0.8 else rint(1, 31)
    return f"date({y}, {m}, {d})"


def rand_offset_expr():
    choice = rng.random()
    if choice < 0.2:
        return "timezone.utc"
    if choice < 0.3:
        return "UTC"
    secs = rng.choice([rint(-86399, 86399), rint(-14, 14) * 3600, rint(-14 * 4, 14 * 4) * 900])
    us = rng.choice([0, 0, 0, rint(0, 999999)])
    if rng.random() < 0.5:
        return f"timezone(timedelta(seconds={secs}, microseconds={us}))"
    return f"timezone(timedelta(seconds={secs}), {rng.choice(['EST', 'X', 'UTC', 'Zone+1'])!r})"


def rand_time_args():
    h, mi, s = rint(0, 23), rint(0, 59), rint(0, 59)
    us = rng.choice([0, rint(0, 999999), rint(0, 999) * 1000])
    return h, mi, s, us


def rand_datetime(aware=None):
    y = rand_year(); m = rint(1, 12); d = rint(1, 28)
    h, mi, s, us = rand_time_args()
    aware = rng.random() < 0.4 if aware is None else aware
    tz = f", tzinfo={rand_offset_expr()}" if aware else ""
    fold = ", fold=1" if rng.random() < 0.1 else ""
    return f"datetime({y}, {m}, {d}, {h}, {mi}, {s}, {us}{tz}{fold})"


def rand_time(aware=None):
    h, mi, s, us = rand_time_args()
    aware = rng.random() < 0.4 if aware is None else aware
    tz = f", tzinfo={rand_offset_expr()}" if aware else ""
    fold = ", fold=1" if rng.random() < 0.1 else ""
    return f"time({h}, {mi}, {s}, {us}{tz}{fold})"


def rand_number():
    c = rng.random()
    if c < 0.4:
        return str(rng.choice([rint(-1000, 1000), rint(-10**6, 10**6), rint(-10**10, 10**10)]))
    if c < 0.8:
        v = rng.choice([rng.uniform(-1000, 1000), rng.uniform(-1, 1), rng.uniform(-1e6, 1e6),
                        rint(-100, 100) + 0.5, rint(-100, 100) / 8])
        return repr(v)
    return rng.choice(['0', '1', '-1', '0.5', '-0.5', '1.5', '2.5', '1e-6', '0.0000005',
                       '0.0000015', '-0.0000005', 'True', '10**12', '1e15'])


def rand_delta():
    kws = ['days', 'seconds', 'microseconds', 'milliseconds', 'minutes', 'hours', 'weeks']
    parts = []
    for k in rng.sample(kws, rint(0, 4)):
        parts.append(f"{k}={rand_number()}")
    if rng.random() < 0.2:
        return f"timedelta({rint(-999999999, 999999999)}, {rint(0, 86399)}, {rint(0, 999999)})"
    return f"timedelta({', '.join(parts)})"


STRFTIME_SAFE = list('aAbBcCdDeFgGhHIjklmMnprRStTuUVwWxXyYzZf%') + [
    ':z', '-d', '-m', '-H', '-I', '-M', '-S', '-j', '_d', '0e', 'Ey', 'Od', 'OH', 'EC']


def rand_strftime_format():
    parts = []
    for _ in range(rint(1, 6)):
        if rng.random() < 0.7:
            parts.append('%' + rng.choice(STRFTIME_SAFE))
        else:
            parts.append(rng.choice(['-', ':', ' ', 'T', '/', 'x', '..', ', ', '%%', 'é']))
    return ''.join(parts)


STRPTIME_DIRECTIVES = ['%Y', '%m', '%d', '%H', '%M', '%S', '%f', '%y', '%b', '%B', '%a', '%A',
                       '%p', '%I', '%j', '%z', '%Z', '%e', '%k', '%l', '%T', '%R', '%c', '%x',
                       '%X', '%U', '%W', '%w', '%u', '%G', '%V', '%%']
GROUPS = {'%T': 'HMS', '%R': 'HM', '%c': 'abdHMSY', '%x': 'mdy', '%X': 'HMS', '%Y': 'Y',
          '%y': 'y', '%m': 'm', '%d': 'd', '%e': 'd', '%H': 'H', '%k': 'H', '%M': 'M',
          '%S': 'S', '%f': 'f', '%b': 'b', '%B': 'B', '%a': 'a', '%A': 'A', '%p': 'p',
          '%I': 'I', '%l': 'I', '%j': 'j', '%z': 'z', '%Z': 'Z', '%U': 'U', '%W': 'W',
          '%w': 'w', '%u': 'u', '%G': 'G', '%V': 'V', '%%': ''}


# The directives whose rendering for a year below 1000 the platform decides:
# glibc does not zero-pad the year the way macOS does.
YEAR_DIRECTIVES = set("YGCFcx")

# The directives CPython replaces itself; the C library never sees them.
CPYTHON_DIRECTIVES = set("fzZ")

# The directives MiniPython renders the way macOS's C library does.
SUPPORTED_DIRECTIVES = set("aAbBcCdDeFgGhHIjklmMnprRStTuUVwWxXyYzZ%") | CPYTHON_DIRECTIVES

# The E and O modifiers, and the directives each of them allows.
MODIFIER_DIRECTIVES = {"E": set("cCxXyY"), "O": set("deHImMSuUVwWy")}

# The directives whose padding flags the two C libraries render differently.
FLAG_DIRECTIVES = set("yCgGY")

VALUE_YEAR = re.compile(r"^date(?:time)?\((\d+),\s*(\d+),\s*(\d+)")

# The scenario keys of the current program the platform C library decides.
PLATFORM_KEYS = []

# The label of every scenario of a program, and its source line.
SCENARIO_LABEL = re.compile(r"t\('([^']+)'")


def format_directives(fmt):
    """The (padding flag, modifier, directive) triples of a strftime format."""
    out = []
    index = 0
    while index < len(fmt):
        if fmt[index] != '%':
            index += 1
            continue
        index += 1
        if fmt.startswith(':z', index):
            out.append(('', '', 'z'))
            index += 2
            continue
        if index >= len(fmt):
            break
        flag = ''
        if fmt[index] in '-_0':
            flag = fmt[index]
            index += 1
        modifier = ''
        if index < len(fmt) and fmt[index] in 'EO':
            modifier = fmt[index]
            index += 1
        if index >= len(fmt):
            break
        out.append((flag, modifier, fmt[index]))
        index += 1
    return out


def value_years(expr):
    """The calendar year and the ISO week-based year a value renders."""
    match = VALUE_YEAR.match(expr)
    if not match:
        return None, None  # a time: the C library uses 1900-01-01
    year, month, day = (int(part) for part in match.groups())
    try:
        iso_year = _datetime.date(year, month, day).isocalendar()[0]
    except ValueError:
        iso_year = None
    return year, iso_year


def platform_dependent(fmt, value):
    """Whether the platform C library decides how `strftime(fmt)` renders.

    Two groups of inputs decide it (tests/README.md, "Platform-dependent
    behavior"): the year-bearing directives of a year below 1000, which glibc
    does not zero-pad the way macOS does, and the unknown or platform-specific
    directives and the padding flags on them, which the two C libraries echo
    differently.
    """
    directives = format_directives(fmt)
    for flag, modifier, directive in directives:
        if directive not in SUPPORTED_DIRECTIVES:
            return True
        if modifier and directive not in MODIFIER_DIRECTIVES[modifier]:
            return True
        if flag and directive in FLAG_DIRECTIVES:
            return True
    if not any(directive in YEAR_DIRECTIVES for _, _, directive in directives):
        return False
    year, iso_year = value_years(value)
    return (year is not None and year < 1000) or (iso_year is not None and iso_year < 1000)


def mark_platform_key(label, fmt, value):
    """Record a scenario the platform C library decides.

    `fmt` is the format the scenario hands to `strftime` (or to `format`,
    which calls it) and `value` the source of the value it formats. Only a
    scenario that really calls an affected format may be marked: a scenario
    that computes its result itself (`ctime()`, `isocalendar()`,
    `timetuple()`, ...) is decided by MiniPython and CPython alike and must
    be compared on every platform.
    """
    if platform_dependent(fmt, value):
        PLATFORM_KEYS.append(label)


def scenario_sources(program):
    """The source line of every scenario of a program, by its label."""
    sources = {}
    for line in program.splitlines():
        match = SCENARIO_LABEL.search(line)
        if match:
            sources[match.group(1)] = line
    return sources


def calls_strftime(source):
    """Whether a scenario's source calls strftime, directly or via format."""
    return "strftime(" in source or bool(re.search(r"(?<!iso)format\(", source))


def verify_marks(program, marks):
    """Check that every marked scenario really calls an affected format.

    A mark says that the platform C library decides the scenario's result,
    so it must name a scenario that calls `strftime` (directly or through
    `format`). A scenario that does not call it -- `ctime()` is the one the
    datetime corpus generates, and CPython formats it itself, with a
    zero-padded four-digit year, on every platform -- is decided by
    MiniPython and CPython alike and must be compared on every platform.
    """
    marked = set(marks)
    for label, source in scenario_sources(program).items():
        if label in marked and not calls_strftime(source):
            raise SystemExit(f"marked scenario {label} does not call strftime: {source}")


def rand_strptime_format():
    picked = []
    used = set()
    for _ in range(rint(1, 5)):
        dirv = rng.choice(STRPTIME_DIRECTIVES)
        g = set(GROUPS[dirv])
        if g & used:
            continue
        used |= g
        picked.append(dirv)
    sep = rng.choice([' ', '-', '/', ':', 'T', ' ', ''])
    return sep.join(picked)


def iso_mutations(s):
    out = [s]
    for _ in range(3):
        chars = list(s)
        op = rng.random()
        pos = rint(0, len(chars))
        if op < 0.3 and chars:
            del chars[min(pos, len(chars) - 1)]
        elif op < 0.6:
            chars.insert(pos, rng.choice('0123456789-:T.,Z+ Wx'))
        elif chars:
            chars[min(pos, len(chars) - 1)] = rng.choice('0123456789-:T.,Z+ W')
        out.append(''.join(chars))
    return out


def rand_iso_string():
    y = rand_year(); m = rint(1, 13); d = rint(1, 31)
    date_forms = [f"{y:04d}-{m:02d}-{d:02d}", f"{y:04d}{m:02d}{d:02d}",
                  f"{y:04d}-W{rint(1,53):02d}", f"{y:04d}W{rint(1,53):02d}",
                  f"{y:04d}-W{rint(1,53):02d}-{rint(1,7)}", f"{y:04d}W{rint(1,53):02d}{rint(1,7)}"]
    h, mi, s, us = rand_time_args()
    if rng.random() < 0.05:
        h = 24; mi = s = us = 0
    frac = rng.choice(['', '', f".{us:06d}", f".{us // 1000:03d}", f",{us:06d}",
                       f".{rint(0, 9)}", f".{rint(0, 10**9):09d}"])
    time_forms = [f"{h:02d}", f"{h:02d}:{mi:02d}", f"{h:02d}:{mi:02d}:{s:02d}{frac}",
                  f"{h:02d}{mi:02d}", f"{h:02d}{mi:02d}{s:02d}{frac}"]
    oh, om, os_ = rint(0, 23), rint(0, 59), rint(0, 59)
    sign = rng.choice('+-')
    tz_forms = ['', '', 'Z', f"{sign}{oh:02d}", f"{sign}{oh:02d}:{om:02d}",
                f"{sign}{oh:02d}{om:02d}", f"{sign}{oh:02d}:{om:02d}:{os_:02d}",
                f"{sign}{oh:02d}:{om:02d}:{os_:02d}.{rint(0, 999999):06d}",
                f"{sign}{oh:02d}{om:02d}{os_:02d}"]
    sep = rng.choice(['T', 'T', ' ', 't', 'x', 'é'])
    kind = rng.random()
    if kind < 0.3:
        return 'date', rng.choice(date_forms)
    if kind < 0.55:
        return 'time', rng.choice(['', 'T']) + rng.choice(time_forms) + rng.choice(tz_forms)
    return 'datetime', rng.choice(date_forms) + rng.choice(
        ['', sep + rng.choice(time_forms) + rng.choice(tz_forms)])


def scenario(i):
    kind = rng.randrange(25)
    if kind == 22:
        a, b = rand_datetime(), rand_datetime()
        return (f"t('s{i}', lambda: (lambda x, y: ({{x: 1}}.get(y), y in {{x}}, (x, 1) in {{(y, 1)}}, "
                f"len({{x, y}}), x == y))({a}, {b}))")
    if kind == 23:
        m = rint(-1439, 1439)
        d = rint(0, 60)
        y = rand_year(); mo = rint(1, 12); dd = rint(1, 28)
        h, mi, se, us = rand_time_args()
        expr = f"dt0 = datetime({y}, {mo}, {dd}, {h}, {mi}, {se}, {us}, tzinfo=Fixed({m}, 'Z{m}', {d}))"
        f = rng.choice(['dt0.isoformat()', 'dt0.utcoffset()', 'dt0.dst()', 'dt0.tzname()',
                        'dt0.strftime("%z %:z %Z %Y")', 'dt0.astimezone(timezone.utc)',
                        'dt0.timestamp()', 'dt0.utctimetuple()', 'dt0.timetuple()',
                        'dt0 - datetime(2000, 1, 1, tzinfo=timezone.utc)',
                        'dt0 == dt0.astimezone(timezone.utc)', 'str(dt0.astimezone(Fixed(30)))',
                        'hash(dt0) == hash(dt0.astimezone(timezone.utc))'])
        if 'strftime' in f:
            mark_platform_key(f"s{i}", "%z %:z %Z %Y", f"datetime({y}, {mo}, {dd})")
        return f"t('s{i}', lambda: (lambda dt0: {f})({expr.split(' = ', 1)[1]}))"
    if kind == 24:
        parts = ', '.join(f"{k}={rand_number()}" for k in rng.sample(['days', 'seconds', 'microseconds', 'milliseconds', 'minutes', 'hours', 'weeks'], rint(1, 7)))
        return f"t('s{i}', lambda: (lambda d: (d, str(d), d.total_seconds()))(timedelta({parts})))"
    L = f"s{i}"
    if kind == 0:
        a, b = rand_date(), rand_delta()
        op = rng.choice(['+', '-'])
        return f"t('{L}', lambda: {a} {op} {b})"
    if kind == 1:
        a, b = rand_datetime(), rand_delta()
        op = rng.choice(['+', '-'])
        return f"t('{L}', lambda: {a} {op} {b})"
    if kind == 2:
        aw = rng.random() < 0.5
        a, b = rand_datetime(aw), rand_datetime(aw if rng.random() < 0.9 else not aw)
        op = rng.choice(['-', '<', '<=', '==', '!=', '>', '>='])
        return f"t('{L}', lambda: {a} {op} {b})"
    if kind == 3:
        a, b = rand_date(), rand_date()
        op = rng.choice(['-', '<', '==', '>='])
        return f"t('{L}', lambda: {a} {op} {b})"
    if kind == 4:
        a = rand_delta()
        op = rng.choice(['*', '/', '//', '%', 'divmod'])
        rhs = rng.choice([rand_number(), rand_delta(), str(rint(-5, 5))])
        if op == 'divmod':
            return f"t('{L}', lambda: divmod({a}, {rhs}))"
        if rng.random() < 0.2 and op == '*':
            return f"t('{L}', lambda: {rhs} * {a})"
        return f"t('{L}', lambda: {a} {op} {rhs})"
    if kind == 5:
        a = rand_delta()
        f = rng.choice(['repr({})', 'str({})', '({}).total_seconds()', '-{}', 'abs({})', '+{}',
                        'bool({})', '({}).days', '({}) == ({})', '({}) < ' + rand_delta()])
        return f"t('{L}', lambda: {f.format(a, a)})"
    if kind == 6:
        a = rng.choice([rand_date(), rand_datetime(), rand_time()])
        spec = rng.choice(['', 'auto', 'hours', 'minutes', 'seconds', 'milliseconds', 'microseconds'])
        if a.startswith('date('):
            return f"t('{L}', lambda: ({a}).isoformat())"
        if a.startswith('time(') or rng.random() < 0.5:
            if spec:
                return f"t('{L}', lambda: ({a}).isoformat({spec!r}))"
            return f"t('{L}', lambda: ({a}).isoformat())"
        sep = rng.choice(['T', ' ', 'x', 'é'])
        return f"t('{L}', lambda: ({a}).isoformat({sep!r}, {spec or 'auto'!r}))"
    if kind == 7:
        a = rng.choice([rand_date(), rand_datetime(), rand_time()])
        cls = a.split('(')[0]
        return (f"t('{L}', lambda: (lambda v: ({cls}.fromisoformat(v.isoformat()) == v, "
                f"str(v), repr(v)))({a}))")
    if kind == 8:
        a = rng.choice([rand_date(), rand_datetime(), rand_datetime(), rand_time()])
        fmt = rand_strftime_format()
        mark_platform_key(L, fmt, a)
        return f"t('{L}', lambda: ({a}).strftime({fmt!r}))"
    if kind == 9:
        cls, s = rand_iso_string()
        lines = []
        for j, m in enumerate(iso_mutations(s)):
            target = rng.choice([cls, cls, 'datetime', 'date', 'time'])
            lines.append(f"t('{L}.{j}', lambda: {target}.fromisoformat({m!r}))")
        return '\n'.join(lines)
    if kind == 10:
        fmt = rand_strptime_format()
        a = rand_datetime()
        cls = rng.choice(['datetime', 'datetime', 'date', 'time'])
        mark_platform_key(f"{L}.0", fmt, a)
        lines = [f"t('{L}.0', lambda: {cls}.strptime(({a}).strftime({fmt!r}), {fmt!r}))"]
        data = rng.choice(['2020-01-02', '12:30', 'Mon Jan  1 00:00:00 2020', '2020 10 5',
                           '1 pm', '+0530', 'UTC', '99', '366', '2020W01'])
        lines.append(f"t('{L}.1', lambda: {cls}.strptime({data!r}, {fmt!r}))")
        return '\n'.join(lines)
    if kind == 11:
        return f"t('{L}', lambda: {rand_delta()})"
    if kind == 12:
        ts = rng.choice([rand_number(), str(rint(-62135596800, 253402300799)),
                         repr(rng.uniform(-62135596800, 253402300799)), repr(rng.uniform(-1e9, 2e9))])
        f = rng.choice(['datetime.fromtimestamp({})', 'datetime.fromtimestamp({}, timezone.utc)',
                        'datetime.fromtimestamp({}, ' + rand_offset_expr() + ')',
                        'date.fromtimestamp({})', 'datetime.utcfromtimestamp({})'])
        return f"t('{L}', lambda: {f.format(ts)})"
    if kind == 13:
        a = rand_datetime()
        f = rng.choice(['({}).timestamp()', '({}).timetuple()', '({}).utctimetuple()',
                        '({}).ctime()', '({}).toordinal()', '({}).weekday()', '({}).isocalendar()',
                        '({}).astimezone(timezone.utc)', '({}).astimezone(' + rand_offset_expr() + ')',
                        '({}).astimezone()', '({}).utcoffset()', '({}).tzname()', '({}).dst()',
                        '({}).date()', '({}).timetz()', 'hash({}) == hash(copy.copy({}))',
                        '({}).__reduce_ex__(4)', '({}).__reduce__()', 'str({})', 'repr({})',
                        'format({}, "%Y|%H")', 'f"{{({})}}"'])
        # Only the format call is a strftime call: `format(value, "%Y|%H")`
        # hands %Y to the C library, whose rendering of a year below 1000 the
        # platform decides. `ctime()` is not one: CPython formats it itself,
        # with a zero-padded four-digit year, on every platform.
        if f == 'format({}, "%Y|%H")':
            mark_platform_key(L, '%Y|%H', a)
        return f"t('{L}', lambda: {f.format(a, a) if f.count('{}') == 2 else f.format(a)})"
    if kind == 14:
        y = rand_year()
        w = rint(0, 54); d = rint(0, 8)
        return f"t('{L}', lambda: (date.fromisocalendar({y}, {w}, {d}), datetime.fromisocalendar({y}, {w}, {d})))"
    if kind == 15:
        o = rng.choice([rint(-10, 3700000), rint(1, 3652059)])
        return f"t('{L}', lambda: (date.fromordinal({o}), datetime.fromordinal({o})))"
    if kind == 16:
        a = rng.choice([rand_date(), rand_datetime(), rand_time()])
        fields = {'date': ['year', 'month', 'day'],
                  'time': ['hour', 'minute', 'second', 'microsecond', 'tzinfo', 'fold']}
        cls = a.split('(')[0]
        names = fields.get(cls, fields['date'] + fields['time'])
        kws = []
        for n in rng.sample(names, rint(1, min(3, len(names)))):
            if n == 'tzinfo':
                kws.append(f"tzinfo={rng.choice(['None', rand_offset_expr()])}")
            elif n == 'fold':
                kws.append(f"fold={rint(0, 2)}")
            else:
                kws.append(f"{n}={rint(-1, 70) if n != 'year' else rand_year()}")
        how = rng.choice(['({}).replace({})', 'copy.replace({}, {})'])
        return f"t('{L}', lambda: {how.format(a, ', '.join(kws))})"
    if kind == 17:
        tz = rand_offset_expr()
        return (f"t('{L}', lambda: (repr({tz}), str({tz}), ({tz}).tzname(None), "
                f"({tz}).utcoffset(None), ({tz}) == timezone.utc))")
    if kind == 18:
        a, b = rand_time(), rand_time()
        op = rng.choice(['<', '==', '!=', '>='])
        return f"t('{L}', lambda: ({a} {op} {b}, hash({a}) == hash({a})))"
    if kind == 19:
        a = rand_time()
        f = rng.choice(['({}).utcoffset()', '({}).tzname()', 'str({})', 'repr({})',
                        '({}).strftime("%H:%M:%S.%f %z %Z")', '({}).__reduce__()', 'format({}, "")'])
        if f == '({}).strftime("%H:%M:%S.%f %z %Z")':
            mark_platform_key(L, "%H:%M:%S.%f %z %Z", a)
        return f"t('{L}', lambda: {f.format(a)})"
    if kind == 20:
        h, mi, s, us = rand_time_args()
        y, m, d = rand_year(), rint(1, 12), rint(1, 31)
        args = rng.choice([f"{y}, {m}, {d}", f"{y}, {m + rint(-1, 1)}, {d}, {h}, {mi}, {s}",
                           f"{y}, {m}, {d}, {h + rint(-1, 2)}, {mi}, {s}, {us}",
                           f"{y + rng.choice([0, -10000, 10000])}, {m}, {d}"])
        cls = rng.choice(['datetime', 'date'])
        if cls == 'date':
            args = ', '.join(args.split(', ')[:3])
        return f"t('{L}', lambda: {cls}({args}))"
    a = rng.choice([rand_date(), rand_datetime()])
    # `isocalendar`, `isoweekday` and `timetuple` are computed by MiniPython
    # and CPython alike, and so is `ctime()`: CPython formats it itself,
    # with a zero-padded four-digit year, on every platform. They are split
    # into scenarios of their own so that each is compared on its own.
    return (f"t('{L}', lambda: [({a}).isocalendar(), ({a}).isoweekday(), ({a}).timetuple()])\n"
            f"t('{L}.0', lambda: ({a}).ctime())")


def main(argv):
    global rng
    out, seed, programs, per = argv[1], int(argv[2]), int(argv[3]), int(argv[4])
    rng = random.Random(seed)
    for p in range(programs):
        body = [PRELUDE]
        PLATFORM_KEYS.clear()
        for i in range(per):
            body.append(scenario(p * per + i))
        program = '\n'.join(body) + '\n'
        verify_marks(program, PLATFORM_KEYS)
        with open(f"{out}/prog{p:03d}.py", 'w') as fh:
            fh.write(program)
        # The scenarios the platform C library decides. The corpus runner skips
        # them on the platforms the manifest lists for the corpus.
        with open(f"{out}/prog{p:03d}.platform-keys.json", 'w') as fh:
            json.dump(sorted(PLATFORM_KEYS), fh)


if __name__ == '__main__':
    main(sys.argv)
