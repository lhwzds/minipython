ws = ''.join(chr(c) for c in range(0x110000) if not 0xd800 <= c <= 0xdfff and chr(c).isspace())
print(len(ws), [hex(ord(c)) for c in ws])
text = 'a\x1cb\x1dc\x1ed\x1fe　f\x85g h\xa0i'
print(text.split())
print(text.rsplit(None, 2))
print(text.split(maxsplit=3))
print(repr(('\x1c\x1f  x \x1e ').strip()), repr('\x1cx\x1d'.lstrip()), repr('\x1cx\x1d'.rstrip()))
from collections import UserString
print(repr(UserString('\x1cx\x1d').strip()), UserString('a\x1cb').split())
print(repr('\x1c'.isspace()), ' \x1c\x1f'.isspace(), ''.isspace())
for s in ['Hello World', 'HELLO', 'hello', 'ǅungla', 'ᾈ', 'ᾈa', 'aᾈ', 'Σίσυφος', 'ʕ', 'ⅰⅱ', 'Ⅰ', 'ⓐ', 'Ⓐb', '1A', 'A1b', '', '123', 'ǅ ǅ', 'Ab Cd', 'AB Cd']:
    print(repr(s), s.isupper(), s.islower(), s.istitle(), s.isalpha(), s.isalnum())
for s in ['²', '①', '½', '一', '٣', '𝟙', 'Ⅻ', '፩', '3.4', '']:
    print(repr(s), s.isdecimal(), s.isdigit(), s.isnumeric(), s.isalnum())
for s in ['_x', 'x1', '1x', 'ℕ', '·', 'a·', '℘', '℮', 'ⅰ', '゛', 'a゛', 'x‌', 'a-b', '']:
    print(repr(s), s.isidentifier())
for s in ['　42', ' 42\x85', '\xa042 ', '\x1c42']:
    try:
        print(repr(s), int(s), float(s))
    except ValueError as error:
        print('ValueError', error)
