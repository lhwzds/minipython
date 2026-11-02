blocks = [(0x0, 0x100), (0x295, 0x296), (0x345, 0x346), (0x363, 0x370), (0x88f, 0x890),
          (0xc5c, 0xc5d), (0x1369, 0x137d), (0x19da, 0x19db), (0x1f80, 0x1fb0), (0x2070, 0x2090),
          (0x2150, 0x2190), (0x2460, 0x2500), (0x2776, 0x2794), (0x3000, 0x3010), (0x3405, 0x3406),
          (0x4e00, 0x4e10), (0xa7c0, 0xa7f5), (0xff10, 0xff1a), (0x10940, 0x10950),
          (0x10d40, 0x10d4a), (0x11de0, 0x11dea), (0x16ea0, 0x16ec5), (0x1f100, 0x1f10c),
          (0x20000, 0x20002), (0xe0000, 0xe0002)]
methods = ['isspace', 'isdigit', 'isdecimal', 'isnumeric', 'isalpha', 'isalnum',
           'isidentifier', 'isprintable', 'isupper', 'islower', 'istitle']
for name in methods:
    hits = [cp for start, stop in blocks for cp in range(start, stop) if getattr(chr(cp), name)()]
    print(name, len(hits), ' '.join('%x' % cp for cp in hits))
text = 'a\x1cb\x1dc\x1ed\x1fe　f\x85g h\xa0i'
print(text.split(), text.rsplit(None, 2), text.split(maxsplit=3))
print(repr('\x1c\x1f  x \x1e '.strip()), repr('\x1cx\x1d'.lstrip()), repr('\x1cx\x1d'.rstrip()))
from collections import UserString
print(repr(UserString('\x1cx\x1d').strip()), UserString('a\x1cb').split())
for s in ['Hello World', 'ǅungla', 'ᾈa', 'aᾈ', 'Σίσυφος', 'ʕ', 'ⅰⅱ', 'Ⓐb', 'A1b', 'ǅ ǅ', 'AB Cd']:
    print(repr(s), s.isupper(), s.islower(), s.istitle(), s.isalpha(), s.isalnum())
for s in ['²', '①', '½', '一', '𝟙', 'Ⅻ', '፩', '3.4']:
    print(repr(s), s.isdecimal(), s.isdigit(), s.isnumeric(), s.isalnum())
for s in ['_x', 'x1', '1x', 'ℕ', '·', 'a·', '℘', '゛', 'a゛', 'a-b']:
    print(repr(s), s.isidentifier())
for s in ['　42', ' 42\x85', '\xa042 ', '\x1c42']:
    try:
        print(repr(s), int(s), float(s))
    except ValueError as error:
        print('ValueError', error)
