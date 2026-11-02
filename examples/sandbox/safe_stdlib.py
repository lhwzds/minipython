# SPDX-License-Identifier: MIT

"""Exercise every standard-library module in the sandbox allowlist."""

from __future__ import annotations

import __future__
import argparse
import array
import builtins
import collections
import collections.abc
import copy
import csv
import datetime
import functools
import io
import itertools
import json
import math
import operator
import re
import sys
import types

print(builtins.len([1, 2]))
print(isinstance(sys.modules, dict))
print(types.SimpleNamespace(x=1).x)
print(collections.Counter("aa")["a"], collections.abc.Sequence.__name__)
print(math.sqrt(4))
try:
    import math.integer
except ModuleNotFoundError:
    print("math.integer unavailable")
else:
    print("math.integer", math.integer.gcd(12, 18))
print(array.array("B", [65]).tobytes())
print(copy.copy([1]) == [1])
buffer = io.BytesIO(b"ab")
print(buffer.read(1))
print(operator.add(2, 3), functools.reduce(lambda a, b: a + b, [1, 2, 3]))
print(next(itertools.count(4)))
print(json.loads('{"a": 1}')["a"])


def typed(value: Undefined) -> list[int]:
    return value


print(__future__.annotations.compiler_flag, typed.__annotations__)

parser = argparse.ArgumentParser(prog="safe")
parser.add_argument("--count", type=int, default=1)
print(parser.parse_args(["--count", "3"]), parser.format_usage(), end="")
print(re.sub(r"(\w+)@(\w+)", r"\2 at \1", "ann@example"), re.findall(r"\d+", "a1b22"))

due = datetime.date.fromisoformat("2024-02-26") + datetime.timedelta(days=30)
print(due, due.strftime("%a %d %b"), datetime.datetime(2024, 3, 1, 12, tzinfo=datetime.UTC).isoformat())

table = io.StringIO('name,qty\r\n"Widget, large",2\r\n')
print(list(csv.DictReader(table)), end=" ")
csv.writer(sys.stdout, lineterminator="\n").writerow(["x,y", 1.5, None])
