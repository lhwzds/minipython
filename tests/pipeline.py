#!/usr/bin/env python3
"""Compare a fixed CPython oracle against MiniPython over a bounded corpus."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
import platform
import random
import re
import subprocess
import sys
import tempfile
import textwrap
import time
import tomllib
from collections import Counter
from collections.abc import Callable, Iterable
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any



LAYERS = ("syntax", "runtime", "stdlib", "security")


def _case(
    *,
    name: str,
    layer: str,
    root_cause: str,
    modules: list[str],
    source: str,
    priority: str,
    category: str = "runtime-semantic",
    expected: str | None = None,
    expected_cpython_stdout: str | None = None,
    expected_minipython_stdout: str | None = None,
) -> dict[str, Any]:
    scope = {
        "syntax": "syntax",
        "runtime": "core-runtime",
        "stdlib": "stdlib-sandbox",
        "security": "intentional-sandbox",
    }[layer]
    result: dict[str, Any] = {
        "name": name,
        "layer": layer,
        "scope": scope,
        "category": category,
        "root_cause": root_cause,
        "modules": modules,
        "priority": priority,
        "source": source,
        "origin": "generated",
    }
    if expected is not None:
        result["expected"] = expected
    if expected_cpython_stdout is not None:
        result["expected_cpython_stdout"] = expected_cpython_stdout
    if expected_minipython_stdout is not None:
        result["expected_minipython_stdout"] = expected_minipython_stdout
    return result


def _syntax_expression(rng: random.Random, index: int) -> dict[str, Any]:
    left = rng.randint(-1000, 1000)
    right = rng.randint(1, 97)
    offset = rng.randint(-20, 20)
    source = f"""
def calculate(value, offset={offset}):
    return ((value + offset) * {right}) // {rng.randint(1, 13)}
print(calculate({left}))
"""
    return _case(
        name=f"generated-syntax-expression-{index}",
        layer="syntax",
        root_cause="generated-syntax-expression",
        modules=["syntax"],
        source=source,
        priority="must_fix",
        category="syntax",
    )


def _syntax_comprehension(rng: random.Random, index: int) -> dict[str, Any]:
    stop = rng.randint(1, 16)
    modulus = rng.randint(2, 5)
    source = f"""
x = 91
values = [x * x for x in range({stop}) if x % {modulus} == 0]
mapping = {{x: x + 1 for x in values}}
print(x, values, sorted(mapping.items()))
"""
    return _case(
        name=f"generated-syntax-comprehension-{index}",
        layer="syntax",
        root_cause="generated-syntax-comprehension-scope",
        modules=["syntax", "core-runtime"],
        source=source,
        priority="must_fix",
        category="syntax",
    )


def _syntax_match(rng: random.Random, index: int) -> dict[str, Any]:
    label = rng.choice(["ok", "skip", "other"])
    value = rng.randint(-20, 20)
    source = f"""
value = ({label!r}, {value})
match value:
    case ("ok", number) if number >= 0:
        print("positive", number)
    case ("ok", number):
        print("negative", number)
    case (label, number):
        print(label, number)
"""
    return _case(
        name=f"generated-syntax-match-{index}",
        layer="syntax",
        root_cause="generated-syntax-pattern-matching",
        modules=["syntax"],
        source=source,
        priority="must_fix",
        category="syntax",
    )


def _syntax_unpacking(rng: random.Random, index: int) -> dict[str, Any]:
    values = [rng.randint(-9, 9) for _ in range(rng.randint(3, 7))]
    source = f"""
first, *middle, last = {values!r}
def collect(head, *items, tail=0, **named):
    return head, list(items), tail, sorted(named.items())
print(first, middle, last)
print(collect(first, *middle, tail=last, size=len(middle)))
"""
    return _case(
        name=f"generated-syntax-unpacking-{index}",
        layer="syntax",
        root_cause="generated-syntax-unpacking-calls",
        modules=["syntax"],
        source=source,
        priority="must_fix",
        category="syntax",
    )


def _runtime_containers(rng: random.Random, index: int) -> dict[str, Any]:
    values = [rng.randint(-20, 20) for _ in range(rng.randint(2, 8))]
    extra = rng.randint(-20, 20)
    source = f"""
values = {values!r}
values.append({extra})
values.reverse()
mapping = {{str(i): value for i, value in enumerate(values)}}
print(values[1:-1], sorted(mapping.items()), sum(values))
"""
    return _case(
        name=f"generated-runtime-containers-{index}",
        layer="runtime",
        root_cause="generated-runtime-container-operations",
        modules=["core-runtime"],
        source=source,
        priority="must_fix",
    )


def _runtime_class_protocol(rng: random.Random, index: int) -> dict[str, Any]:
    value = rng.randint(-100, 100)
    source = f"""
class Box:
    def __init__(self, value):
        self.value = value
    def __str__(self):
        return "Box(" + str(self.value) + ")"
    def __eq__(self, other):
        return isinstance(other, Box) and self.value == other.value
box = Box({value})
print(str(box), box == Box({value}), box == Box({value + 1}))
"""
    return _case(
        name=f"generated-runtime-class-protocol-{index}",
        layer="runtime",
        root_cause="generated-runtime-class-protocol",
        modules=["core-runtime"],
        source=source,
        priority="must_fix",
    )


def _runtime_exceptions(rng: random.Random, index: int) -> dict[str, Any]:
    divisor = rng.choice([0, 1, 2])
    source = f"""
events = []
try:
    events.append("try")
    value = 12 // {divisor}
except ZeroDivisionError as error:
    events.append(type(error).__name__)
else:
    events.append(value)
finally:
    events.append("finally")
print(events)
"""
    return _case(
        name=f"generated-runtime-exceptions-{index}",
        layer="runtime",
        root_cause="generated-runtime-exception-unwinding",
        modules=["exceptions", "core-runtime"],
        source=source,
        priority="must_fix",
        category="exception-shape",
    )


def _runtime_generator(rng: random.Random, index: int) -> dict[str, Any]:
    stop = rng.randint(0, 12)
    step = rng.randint(1, 4)
    source = f"""
def generate(stop):
    current = 0
    while current < stop:
        received = yield current
        current += {step} if received is None else received
iterator = generate({stop})
values = []
try:
    values.append(next(iterator))
    values.append(iterator.send(None))
except StopIteration:
    values.append("done")
print(values, list(iterator))
"""
    return _case(
        name=f"generated-runtime-generator-{index}",
        layer="runtime",
        root_cause="generated-runtime-generator-state",
        modules=["core-runtime"],
        source=source,
        priority="must_fix",
    )


def _runtime_expression_model(rng: random.Random, index: int) -> dict[str, Any]:
    atoms = (
        "None",
        "False",
        "True",
        "0",
        "1",
        "-1",
        "7",
        "255",
        "10**20",
        "0.0",
        "-0.0",
        "1.5",
        "-2.25",
        "float('inf')",
        "float('-inf')",
        "float('nan')",
        "''",
        "'abc'",
        "'é'",
        "b''",
        "b'abc'",
        "[]",
        "[1, 2, 3]",
        "()",
        "(1, 2)",
        "{}",
        "{'a': 1}",
        "range(0)",
        "range(5)",
        "range(-2, 5, 2)",
    )
    numeric_atoms = (
        "False",
        "True",
        "0",
        "1",
        "-1",
        "7",
        "255",
        "10**20",
        "0.0",
        "-0.0",
        "1.5",
        "-2.25",
        "float('inf')",
        "float('-inf')",
        "float('nan')",
    )
    integer_atoms = ("-5", "-1", "0", "1", "2", "255", "10**20")
    containers = ("''", "'abc'", "b''", "b'abc'", "[]", "[1, 2, 3]", "()", "(1, 2)", "{}", "{'a': 1}", "range(5)")
    operators = (
        "+",
        "-",
        "*",
        "/",
        "//",
        "%",
        "**",
        "==",
        "!=",
        "<",
        "<=",
        ">",
        ">=",
        "and",
        "or",
        "in",
        "not in",
        "|",
        "&",
        "^",
        "<<",
        ">>",
    )

    expressions: list[str] = []
    for _ in range(12):
        operator = rng.choice(operators)
        if operator in {"in", "not in"}:
            left = rng.choice(atoms)
            right = rng.choice(containers)
        elif operator in {"<<", ">>"}:
            left = rng.choice(integer_atoms)
            right = rng.choice(("-2", "-1", "0", "1", "2", "8", "65"))
        elif operator == "**":
            left = rng.choice(("-3", "-1", "0", "1", "2", "3", "1.5"))
            right = rng.choice(("-3", "-1", "0", "1", "2", "3", "4"))
        elif operator == "*":
            left = rng.choice(("-3", "-1", "0", "1", "2", "3", "'ab'", "b'ab'", "[1, 2]", "(1, 2)"))
            right = rng.choice(("-3", "-1", "0", "1", "2", "3"))
        elif operator in {"/", "//"}:
            left = rng.choice(numeric_atoms)
            right = rng.choice(numeric_atoms)
        else:
            left = rng.choice(atoms)
            right = rng.choice(atoms)
        expressions.append(f"({left}) {operator} ({right})")

    probes = "\n".join(
        f"probe({probe_index}, lambda: ({expression}))"
        for probe_index, expression in enumerate(expressions)
    )
    source = f"""
def probe(label, thunk):
    try:
        value = thunk()
        print(label, "OK", type(value).__name__, repr(value))
    except BaseException as error:
        print(label, "ERR", type(error).__name__)

{probes}
"""
    return _case(
        name=f"generated-runtime-expression-model-{index}",
        layer="runtime",
        root_cause="generated-runtime-expression-model",
        modules=["core-runtime", "exceptions"],
        source=source,
        priority="must_fix",
    )


def _runtime_value_shape_model(rng: random.Random, index: int) -> dict[str, Any]:
    sequences = (
        "''",
        "'abc'",
        "'é😀'",
        "b''",
        "b'abc'",
        "bytearray(b'abc')",
        "[]",
        "[1, 2, 3]",
        "()",
        "(1, 2)",
        "range(0)",
        "range(5)",
        "range(-3, 6, 2)",
        "{'a': 1, 0: 'zero'}",
    )
    indices = (
        "-10",
        "-2",
        "-1",
        "0",
        "1",
        "2",
        "5",
        "False",
        "True",
        "1.5",
        "'a'",
        "slice(None)",
        "slice(None, None, -1)",
        "slice(1, 4, 2)",
        "slice(-10, 10)",
    )
    unary_atoms = (
        "None",
        "False",
        "True",
        "0",
        "1",
        "-1",
        "10**20",
        "0.0",
        "-0.0",
        "1.5",
        "float('inf')",
        "float('nan')",
        "''",
        "'abc'",
        "b''",
        "b'abc'",
        "[]",
        "[1]",
        "()",
        "(1,)",
        "{}",
        "{'a': 1}",
        "range(0)",
        "range(2)",
    )
    unary_operators = ("+", "-", "~", "not ")
    builtin_expressions = (
        "len(None)",
        "len(0)",
        "len('é😀')",
        "len(bytearray(b'abc'))",
        "bool(float('nan'))",
        "abs(float('-inf'))",
        "int('010')",
        "int('-12')",
        "int(1.5)",
        "float('nan')",
        "float('  -2.5  ')",
        "complex('1+2j')",
        "list(range(-2, 5, 2))",
        "tuple('é😀')",
        "bytes([0, 127, 255])",
        "bytes([256])",
        "bytearray(range(4))",
        "sorted([3, -1, 2, 0])",
        "min([3, -1, 2, 0])",
        "max([3, -1, 2, 0])",
    )

    bindings: list[str] = []
    expressions: list[str] = []
    for probe_index in range(6):
        bindings.append(f"value_{probe_index} = {rng.choice(sequences)}")
        bindings.append(f"index_{probe_index} = {rng.choice(indices)}")
        expressions.append(f"value_{probe_index}[index_{probe_index}]")
    for _ in range(3):
        operator = rng.choice(unary_operators)
        atoms = unary_atoms
        if operator == "~":
            atoms = tuple(atom for atom in unary_atoms if atom not in {"False", "True"})
        expressions.append(f"{operator}({rng.choice(atoms)})")
    expressions.extend(rng.choice(builtin_expressions) for _ in range(3))
    setup = "\n".join(bindings)
    probes = "\n".join(
        f"probe({probe_index}, lambda: ({expression}))"
        for probe_index, expression in enumerate(expressions)
    )
    source = f"""
def probe(label, thunk):
    try:
        value = thunk()
        print(label, "OK", type(value).__name__, repr(value))
    except BaseException as error:
        print(label, "ERR", type(error).__name__)

{setup}
{probes}
"""
    return _case(
        name=f"generated-runtime-value-shape-model-{index}",
        layer="runtime",
        root_cause="generated-runtime-value-shape-model",
        modules=["builtins", "core-runtime", "exceptions"],
        source=source,
        priority="must_fix",
    )


def _stdlib_json(rng: random.Random, index: int) -> dict[str, Any]:
    values = [rng.randint(-100, 100) for _ in range(rng.randint(0, 7))]
    label = rng.choice(["alpha", "beta", "unicode-é", "line\nbreak"])
    source = f"""
import json
value = {{"label": {label!r}, "values": {values!r}, "enabled": {rng.choice([True, False])!r}}}
encoded = json.dumps(value, sort_keys=True, ensure_ascii=True, separators=(",", ":"))
print(encoded)
print(json.loads(encoded) == value)
"""
    return _case(
        name=f"generated-stdlib-json-{index}",
        layer="stdlib",
        root_cause="generated-stdlib-json-roundtrip",
        modules=["json"],
        source=source,
        priority="should_fix",
    )


def _stdlib_iterators(rng: random.Random, index: int) -> dict[str, Any]:
    start = rng.randint(-10, 10)
    count = rng.randint(1, 8)
    source = f"""
import functools, itertools, operator
values = list(itertools.islice(itertools.count({start}, 2), {count}))
print(values)
print(functools.reduce(operator.add, values, 0))
print(list(itertools.chain(values[:2], values[2:])))
"""
    return _case(
        name=f"generated-stdlib-iterators-{index}",
        layer="stdlib",
        root_cause="generated-stdlib-iterator-tools",
        modules=["functools", "itertools", "operator"],
        source=source,
        priority="should_fix",
    )


def _stdlib_buffers(rng: random.Random, index: int) -> dict[str, Any]:
    values = [rng.randint(0, 255) for _ in range(rng.randint(0, 12))]
    source = f"""
import array, copy, io
original = array.array("B", {values!r})
cloned = copy.copy(original)
buffer = io.BytesIO(cloned.tobytes())
print(original.tolist(), cloned == original)
print(buffer.read({rng.randint(0, len(values) + 2)}), buffer.tell(), buffer.getvalue())
"""
    return _case(
        name=f"generated-stdlib-buffers-{index}",
        layer="stdlib",
        root_cause="generated-stdlib-memory-buffers",
        modules=["array", "copy", "io.BytesIO"],
        source=source,
        priority="should_fix",
    )


def _stdlib_collections_math(rng: random.Random, index: int) -> dict[str, Any]:
    text = "".join(rng.choice("abc") for _ in range(rng.randint(0, 16)))
    left = rng.randint(1, 1000)
    right = rng.randint(1, 1000)
    source = f"""
import collections, math
counter = collections.Counter({text!r})
print(sorted(counter.items()), counter.most_common())
print(math.gcd({left}, {right}), math.isqrt({left * right}))
"""
    return _case(
        name=f"generated-stdlib-collections-math-{index}",
        layer="stdlib",
        root_cause="generated-stdlib-collections-math",
        modules=["collections", "math"],
        source=source,
        priority="should_fix",
    )


def _security_import(rng: random.Random, index: int) -> dict[str, Any]:
    module = rng.choice(
        ["socket", "subprocess", "signal", "os", "pathlib", "threading", "_ctypes"]
    )
    source = f"""
try:
    __import__({module!r})
except Exception as error:
    print(type(error).__name__)
else:
    print("imported")
"""
    return _case(
        name=f"generated-security-import-{index}",
        layer="security",
        root_cause="generated-security-module-block",
        modules=[module],
        source=source,
        priority="wont_fix",
        category="sandbox-excluded",
        expected="intentional_sandbox_block",
        expected_cpython_stdout="imported\n",
        expected_minipython_stdout="ModuleNotFoundError\n",
    )


def _security_dynamic_import(rng: random.Random, index: int) -> dict[str, Any]:
    module = rng.choice(["socket", "subprocess", "os", "_socket"])
    mechanism = rng.choice(["eval", "exec", "compile"])
    if mechanism == "eval":
        action = f"eval(\"__import__({module!r})\")"
    elif mechanism == "exec":
        action = f"exec(\"__import__({module!r})\")"
    else:
        action = f"exec(compile(\"__import__({module!r})\", \"<generated>\", \"exec\"))"
    source = f"""
try:
    {action}
except Exception as error:
    print(type(error).__name__)
else:
    print("imported")
"""
    return _case(
        name=f"generated-security-dynamic-import-{index}",
        layer="security",
        root_cause="generated-security-dynamic-import-block",
        modules=[module],
        source=source,
        priority="wont_fix",
        category="sandbox-excluded",
        expected="intentional_sandbox_block",
        expected_cpython_stdout="imported\n",
        expected_minipython_stdout="ModuleNotFoundError\n",
    )


def _security_host_builtin(rng: random.Random, index: int) -> dict[str, Any]:
    name = rng.choice(["open", "input"])
    source = f"""
try:
    {name}
except Exception as error:
    print(type(error).__name__)
else:
    print("available")
"""
    return _case(
        name=f"generated-security-host-builtin-{index}",
        layer="security",
        root_cause="generated-security-host-builtin-block",
        modules=["builtins"],
        source=source,
        priority="wont_fix",
        category="sandbox-excluded",
        expected="intentional_sandbox_block",
        expected_cpython_stdout="available\n",
        expected_minipython_stdout="NameError\n",
    )


GENERATORS: dict[str, tuple[Callable[[random.Random, int], dict[str, Any]], ...]] = {
    "syntax": (
        _syntax_expression,
        _syntax_comprehension,
        _syntax_match,
        _syntax_unpacking,
    ),
    "runtime": (
        _runtime_containers,
        _runtime_class_protocol,
        _runtime_exceptions,
        _runtime_generator,
        _runtime_expression_model,
        _runtime_value_shape_model,
    ),
    "stdlib": (
        _stdlib_json,
        _stdlib_iterators,
        _stdlib_buffers,
        _stdlib_collections_math,
    ),
    "security": (
        _security_import,
        _security_dynamic_import,
        _security_host_builtin,
    ),
}


def generate_cases(seed: int, count: int, layers: Iterable[str]) -> list[dict[str, Any]]:
    """Generate exactly ``count`` deterministic cases, balanced across layers."""
    selected = tuple(dict.fromkeys(layers))
    unknown = sorted(set(selected) - set(LAYERS))
    if unknown:
        raise ValueError(f"unknown discovery layers: {', '.join(unknown)}")
    if count < 0:
        raise ValueError("generated case count must be non-negative")
    if count and not selected:
        raise ValueError("at least one discovery layer is required")

    rng = random.Random(seed)
    cases: list[dict[str, Any]] = []
    per_layer_index = {layer: 0 for layer in selected}
    for ordinal in range(count):
        layer = selected[ordinal % len(selected)]
        layer_index = per_layer_index[layer]
        per_layer_index[layer] += 1
        generators = GENERATORS[layer]
        generator = generators[layer_index % len(generators)]
        case = generator(rng, layer_index)
        case["seed"] = seed
        case["ordinal"] = ordinal
        cases.append(case)
    return cases


def top_level_reduction_candidates(source: str) -> list[str]:
    """Return deterministic candidates with one top-level statement removed."""
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return []
    lines = source.splitlines(keepends=True)
    candidates: list[str] = []
    for node in tree.body:
        start = max(node.lineno - 1, 0)
        end = getattr(node, "end_lineno", node.lineno)
        candidate = "".join(lines[:start] + lines[end:])
        if candidate.strip() and candidate != source:
            candidates.append(candidate)
    return candidates


def literal_reduction_candidates(source: str) -> list[str]:
    """Return conservative candidates that simplify one literal at a time."""
    candidates: list[str] = []
    patterns = (
        (r"(?<![A-Za-z0-9_])-?[1-9][0-9]*", "0"),
        (r"(?<![A-Za-z0-9_])(?:True|False)", "False"),
        (r"(['\"])(?:\\.|(?!\1).)*\1", "''"),
    )
    for pattern, replacement in patterns:
        for match in re.finditer(pattern, source):
            candidate = source[: match.start()] + replacement + source[match.end() :]
            if candidate != source:
                candidates.append(candidate)
    return candidates


def minimize_source(
    source: str,
    preserves_failure: Callable[[str], bool],
    max_attempts: int = 64,
) -> tuple[str, int]:
    """Greedily reduce a source while preserving the caller's failure signature."""
    current = source
    attempts = 0
    changed = True
    while changed and attempts < max_attempts:
        changed = False
        candidates = top_level_reduction_candidates(current)
        candidates.extend(literal_reduction_candidates(current))
        candidates.sort(key=lambda candidate: (len(candidate), candidate))
        for candidate in candidates:
            if attempts >= max_attempts:
                break
            attempts += 1
            if preserves_failure(candidate):
                current = candidate
                changed = True
                break
    return current, attempts


HOMEBREW_CPYTHON_ORACLE = "/opt/homebrew/bin/python3"


def default_cpython_oracle() -> str:
    """The CPython oracle: $MINIPYTHON_CPYTHON, else the Homebrew python3."""
    return os.environ.get("MINIPYTHON_CPYTHON") or HOMEBREW_CPYTHON_ORACLE


DEFAULT_CPYTHON_ORACLE = default_cpython_oracle()

STATUSES = [
    "MATCH",
    "OUTPUT_DIFF",
    "EXCEPTION_CLASS_DIFF",
    "EXCEPTION_MESSAGE_DIFF",
    "MINIPYTHON_REJECTS",
    "CPYTHON_REJECTS",
    "INTENTIONAL_SANDBOX_BLOCK",
    "UNSUPPORTED_OUT_OF_SCOPE",
    "STDLIB_MISSING",
    "CPYTHON_MISSING_COMPAT",
    "CPYTHON_INTERNAL",
    "EXPECTED_CONTRACT_DIFF",
    "TIMEOUT",
    "CRASH",
]

TRIAGE_STATUSES = [
    "passing",
    "accepted_gap",
    "needs_triage",
]

EXPECTED_STATUS_BY_MARKER = {
    "intentional_sandbox_block": "INTENTIONAL_SANDBOX_BLOCK",
    "unsupported_out_of_scope": "UNSUPPORTED_OUT_OF_SCOPE",
    "stdlib_missing": "STDLIB_MISSING",
    "cpython_missing_compat": "CPYTHON_MISSING_COMPAT",
    "cpython_internal": "CPYTHON_INTERNAL",
}

VALID_CATEGORIES = {
    "syntax",
    "runtime-semantic",
    "exception-shape",
    "stdlib-missing",
    "sandbox-excluded",
    "cpython-internal",
}

DEFAULT_CATEGORY_BY_SCOPE = {
    "syntax": "syntax",
    "core-runtime": "runtime-semantic",
    "stdlib-sandbox": "runtime-semantic",
    "intentional-sandbox": "sandbox-excluded",
}

DEFAULT_LAYER_BY_SCOPE = {
    "syntax": "syntax",
    "core-runtime": "runtime",
    "stdlib-sandbox": "stdlib",
    "intentional-sandbox": "security",
}

# Statuses that fail any gated run regardless of the case's priority: a
# crashed worker or a broken security/expectation contract is never an
# accepted gap, even for wont_fix discovery cases.
ALWAYS_FAILING_STATUSES = {"CRASH", "EXPECTED_CONTRACT_DIFF"}

NON_FAILING_STATUSES = {
    "MATCH",
    "INTENTIONAL_SANDBOX_BLOCK",
    "UNSUPPORTED_OUT_OF_SCOPE",
    "STDLIB_MISSING",
    "CPYTHON_MISSING_COMPAT",
    "CPYTHON_INTERNAL",
}


@dataclass
class RunResult:
    exit_code: int | None
    stdout: str
    stderr: str
    timeout: bool
    exception_class: str | None
    exception_message: str | None
    # The status mnpy wrote to its status file: "Success", "Error",
    # "TimeLimit", "MemoryLimit" or "WorkerCrash". None when there was no
    # status file, when it held no string status, or for a CPython run.
    status: str | None = None
    # The terminal condition mnpy wrote to its status file: the sandbox
    # condition that ended the run, such as "InstructionLimit" or
    # "NativeStackExhausted". None when the sandbox did not end the run,
    # when there was no status file, when it held no string terminal, or
    # for a CPython run.
    terminal: str | None = None


@dataclass
class SweepResult:
    name: str
    scope: str
    category: str
    root_cause: str
    modules: list[str]
    priority: str
    status: str
    triage_status: str
    expected: str | None
    diff: str
    cpython: RunResult
    minipython: RunResult
    layer: str = "runtime"
    origin: str = "corpus"
    seed: int | None = None
    source: str = ""
    minimized_source: str | None = None
    shrink_attempts: int = 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run bounded CPython-vs-MiniPython gap discovery cases.",
    )
    parser.add_argument(
        "--cpython",
        default=default_cpython_oracle(),
        help=(
            "CPython oracle executable. Defaults to $MINIPYTHON_CPYTHON, or "
            f"{HOMEBREW_CPYTHON_ORACLE} when it is unset."
        ),
    )
    parser.add_argument(
        "--require-version",
        required=True,
        help="Exact CPython version required for the oracle, for example 3.14.6.",
    )
    parser.add_argument(
        "--minipython",
        default="target/debug/mnpy",
        help="MiniPython executable to compare.",
    )
    parser.add_argument(
        "--corpus",
        default="tests/cases.toml",
        help="TOML corpus file or directory containing TOML corpus files.",
    )
    parser.add_argument(
        "--scope",
        default="syntax,core-runtime,stdlib-sandbox,intentional-sandbox",
        help="Comma-separated scopes to run.",
    )
    parser.add_argument(
        "--category",
        default=",".join(sorted(VALID_CATEGORIES)),
        help="Comma-separated root-cause categories to run.",
    )
    parser.add_argument(
        "--module",
        default="*",
        help="Comma-separated module names to run, or * for all modules.",
    )
    parser.add_argument(
        "--root-cause",
        default="*",
        help="Comma-separated root-cause ids to run, or * for all root causes.",
    )
    parser.add_argument(
        "--out",
        default="reports/cpython-gap-sweep",
        help="Output path prefix. Writes .json and .md files.",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=5.0,
        help="Per-case timeout in seconds for each interpreter.",
    )
    parser.add_argument(
        "--generated-cases",
        type=int,
        default=0,
        help="Add this many deterministic generated cases to the checked-in corpus.",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=20260710,
        help="Fixed seed used by generated discovery cases.",
    )
    parser.add_argument(
        "--layer",
        default=",".join(LAYERS),
        help="Comma-separated discovery layers: syntax,runtime,stdlib,security.",
    )
    parser.add_argument(
        "--shrink",
        action="store_true",
        help="Minimize generated cases that produce unaccepted differences.",
    )
    parser.add_argument(
        "--shrink-max-attempts",
        type=int,
        default=64,
        help="Maximum candidate executions while minimizing one failure.",
    )
    parser.add_argument(
        "--repro-dir",
        default="reports/differential-repros",
        help="Directory for minimized generated failure programs.",
    )
    parser.add_argument(
        "--fail-priority",
        default="",
        help="Comma-separated priorities that must have no open root causes.",
    )
    parser.add_argument(
        "--fail-on-diff",
        action="store_true",
        help="Exit nonzero when non-intentional differences are found.",
    )
    parser.add_argument(
        "--fail-on-open",
        action="store_true",
        help="Exit nonzero when any root cause still has needs_triage cases.",
    )
    return parser.parse_args()


class StatusFiles:
    """Fresh status file paths inside one temporary directory.

    Every mnpy run gets its own path, so a run never reads the status a
    previous run wrote.
    """

    def __init__(self, directory: Path) -> None:
        self._directory = directory
        self._count = 0

    def next(self) -> Path:
        """Return a path no run has used yet."""
        self._count += 1
        return self._directory / f"status-{self._count:04d}.json"


def read_status_file(path: Path | None) -> tuple[str | None, str | None]:
    """Return the status and terminal condition mnpy wrote to its status file.

    Both are None when there is no status file, when the file does not hold
    valid JSON, or when the JSON holds no string for that field.
    """
    if path is None:
        return None, None
    try:
        payload = json.loads(path.read_text())
    except (OSError, ValueError):
        return None, None
    if not isinstance(payload, dict):
        return None, None
    status = payload.get("status")
    terminal = payload.get("terminal")
    return (
        status if isinstance(status, str) else None,
        terminal if isinstance(terminal, str) else None,
    )


def run_command(
    command: list[str],
    source: str,
    timeout: float,
    status_path: Path | None = None,
) -> RunResult:
    """Run a command on a program read from stdin.

    With a status path, the command is mnpy and is asked to write the
    run's status to that path; the status it wrote is read back into the
    result. CPython runs pass no status path and always report None.
    """
    if status_path is not None:
        # The option goes before the program's own arguments: mnpy reads
        # its options first and hands the rest to the program.
        command = [command[0], "--status-file", str(status_path), *command[1:]]
    try:
        completed = subprocess.run(
            command,
            input=source,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        return RunResult(
            exit_code=None,
            stdout=error.stdout or "",
            stderr=error.stderr or "",
            timeout=True,
            exception_class=None,
            exception_message=None,
            status=None,
            terminal=None,
        )
    except OSError as error:
        return RunResult(
            exit_code=None,
            stdout="",
            stderr=str(error),
            timeout=False,
            exception_class=error.__class__.__name__,
            exception_message=str(error),
            status=None,
            terminal=None,
        )
    exception_class = extract_exception_class(completed.stderr)
    status, terminal = read_status_file(status_path)
    return RunResult(
        exit_code=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
        timeout=False,
        exception_class=exception_class,
        exception_message=extract_exception_message(completed.stderr, exception_class),
        status=status,
        terminal=terminal,
    )


# The message mnpy prints when it cannot write its `--status-file`.
STATUS_WRITE_FAILURE = "mnpy: cannot write status file"


def status_write_failure(result: RunResult) -> str | None:
    """Return the error when a run ended because its status file was lost.

    None for every other run. A status file that cannot be written is a
    harness failure, not a crash: mnpy cannot report how the run ended,
    so the sweep must stop instead of classifying the run.

    A run that left a readable status file did not lose it: mnpy writes
    the status file before it prints anything, so a program that prints
    the failure message itself cannot forge a write failure.
    """
    if result.exit_code != 2:
        return None
    if result.status is not None:
        return None
    for line in result.stderr.splitlines():
        if STATUS_WRITE_FAILURE in line:
            return line.strip()
    return None


def check_status_write(result: RunResult, minipython: str) -> None:
    """Stop the sweep when a run ended because its status file was lost."""
    failure = status_write_failure(result)
    if failure is None:
        return
    raise SystemExit(f"mnpy at {minipython} could not write its status file: {failure}")


def run_minipython(
    command: list[str], source: str, timeout: float, status_path: Path
) -> RunResult:
    """Run mnpy with a status file, and stop the sweep when that file is lost.

    A status file that cannot be written is a harness failure, not a
    crash: mnpy cannot report how the run ended, so the sweep must stop
    instead of classifying the run.
    """
    result = run_command(command, source, timeout, status_path)
    check_status_write(result, command[0])
    return result


def check_status_file_support(minipython: str, status_path: Path, timeout: float) -> None:
    """Fail unless mnpy writes the status file the sweep depends on.

    The sweep reads every mnpy run's status from its status file, so a
    binary that does not support `--status-file` cannot be swept at all:
    every case would look like a crash.

    The preflight runs the binary the way the sweep runs a case: the
    program comes in on stdin, and the arguments are the ones the sweep
    uses, so the preflight exercises the path the sweep depends on.
    """
    result = run_command([minipython], "pass\n", timeout, status_path)
    check_status_write(result, minipython)
    if result.status is not None:
        return
    # No status file: the binary cannot be swept, and every case would
    # look like a crash. The sweep stops instead.
    raise SystemExit(f"mnpy at {minipython} does not write --status-file; rebuild it")


# Both interpreters print uncaught exceptions in CPython's traceback shape and
# end with the `traceback.format_exception_only` line (module-qualified type
# name, then `: str(exc)` when it is not empty). mnpy omits the frame lines
# because it does not track line numbers yet, so exceptions are compared by
# that final line rather than by the whole stderr.
EXCEPTION_CLASS_PATTERN = re.compile(
    r"((?:[A-Za-z_][A-Za-z0-9_]*\.)*[A-Za-z_][A-Za-z0-9_]*"
    r"(?:Error|Exception|Warning|Exit|Interrupt))\b"
)


def extract_exception_class(stderr: str) -> str | None:
    lines = [line.strip() for line in stderr.splitlines() if line.strip()]
    if not lines:
        return None
    for line in reversed(lines):
        match = EXCEPTION_CLASS_PATTERN.search(line)
        if match:
            return match.group(1)
    return None


def final_stderr_line(stderr: str) -> str:
    lines = [line.strip() for line in stderr.splitlines() if line.strip()]
    return lines[-1] if lines else ""


def normalized_message(result: RunResult) -> str:
    if result.exception_message is not None:
        return result.exception_message
    if result.exception_class is None:
        return final_stderr_line(result.stderr)
    return extract_exception_message(result.stderr, result.exception_class) or result.stderr.strip()


def extract_exception_message(stderr: str, exception_class: str | None) -> str | None:
    if exception_class is None:
        return None
    lines = [line.strip() for line in stderr.splitlines() if line.strip()]
    for line in reversed(lines):
        if exception_class in line:
            return line
    return None


def normalized_diff(cpython: RunResult, minipython: RunResult) -> str:
    parts: list[str] = []
    if cpython.exit_code != minipython.exit_code:
        parts.append(f"exit_code {cpython.exit_code!r} != {minipython.exit_code!r}")
    if cpython.timeout != minipython.timeout:
        parts.append(f"timeout {cpython.timeout!r} != {minipython.timeout!r}")
    if cpython.exception_class != minipython.exception_class:
        parts.append(
            f"exception_class {cpython.exception_class!r} != {minipython.exception_class!r}"
        )
    if normalized_message(cpython) != normalized_message(minipython):
        parts.append(
            f"exception_message {normalized_message(cpython)!r} != {normalized_message(minipython)!r}"
        )
    if cpython.stdout != minipython.stdout:
        parts.append("stdout differs")
    elif cpython.stderr != minipython.stderr:
        parts.append("stderr differs")
    return "; ".join(parts)


def worker_crashed(result: RunResult) -> bool:
    """Report whether a run ended in a worker or sandbox crash.

    The rule reads mnpy's status file, not its stderr: mnpy prints the
    program's own output to the same stderr it uses for its report, so a
    program can write "sandbox error: worker crashed" to fake a crash, or
    write a limit message to hide one. Only mnpy's own status file says
    what the run was.

    - a negative exit code means mnpy died from a signal: a crash;
    - the terminal condition `NativeStackExhausted` means the native stack
      guard tripped: a crash, whatever status the run reported;
    - no status at all means mnpy wrote no status file, for example
      because the parent panicked before it could report: a crash;
    - the status `WorkerCrash` means the worker itself failed: a crash;
    - every other status is a run that ended on its own terms, so it is
      not a crash, whatever its stderr holds.
    """
    if result.exit_code is not None and result.exit_code < 0:
        return True
    if result.terminal == "NativeStackExhausted":
        return True
    if result.status is None:
        return True
    return result.status == "WorkerCrash"


def classify(cpython: RunResult, minipython: RunResult, expected: str | None) -> str:
    if cpython.timeout or minipython.timeout:
        return "TIMEOUT"
    if cpython.exit_code is None or minipython.exit_code is None:
        return "CRASH"
    same_exit = cpython.exit_code == minipython.exit_code
    same_stdout = cpython.stdout == minipython.stdout
    same_stderr = cpython.stderr == minipython.stderr
    if same_exit and same_stdout and same_stderr:
        # A run that matches CPython exactly is a match, whatever status
        # mnpy reported: the program and mnpy agree, so there is nothing
        # to triage.
        return "MATCH"
    if worker_crashed(minipython):
        return "CRASH"
    if expected in EXPECTED_STATUS_BY_MARKER:
        return EXPECTED_STATUS_BY_MARKER[expected]
    cpython_ok = cpython.exit_code == 0
    minipython_ok = minipython.exit_code == 0
    if cpython_ok and not minipython_ok:
        return "MINIPYTHON_REJECTS"
    if minipython_ok and not cpython_ok:
        return "CPYTHON_REJECTS"
    if cpython_ok and minipython_ok:
        return "OUTPUT_DIFF"
    if cpython.exception_class != minipython.exception_class:
        return "EXCEPTION_CLASS_DIFF"
    if normalized_message(cpython) != normalized_message(minipython):
        return "EXCEPTION_MESSAGE_DIFF"
    return "OUTPUT_DIFF"


def expected_contract_diff(
    cpython: RunResult, minipython: RunResult, case: dict[str, Any]
) -> str:
    mismatches: list[str] = []
    for side, result in (("cpython", cpython), ("minipython", minipython)):
        for field in ("exit_code", "stdout", "stderr"):
            key = f"expected_{side}_{field}"
            if key in case and case[key] != getattr(result, field):
                mismatches.append(
                    f"{key} {case[key]!r} != {getattr(result, field)!r}"
                )
    return "; ".join(mismatches)


def classify_case(
    cpython: RunResult, minipython: RunResult, case: dict[str, Any]
) -> tuple[str, str]:
    contract_diff = expected_contract_diff(cpython, minipython, case)
    if contract_diff:
        return "EXPECTED_CONTRACT_DIFF", contract_diff
    status = classify(cpython, minipython, case.get("expected"))
    expected = case.get("expected")
    if status == "MATCH" and expected in EXPECTED_STATUS_BY_MARKER:
        # An expected difference that disappeared is a broken contract: for
        # intentional_sandbox_block it means the sandbox let the host
        # capability through; for the other markers the entry is stale and
        # must be removed (strict expectation registry).
        return (
            "EXPECTED_CONTRACT_DIFF",
            f"expected {expected} but MiniPython now matches CPython",
        )
    return status, "" if status == "MATCH" else normalized_diff(cpython, minipython)


def triage_status(status: str) -> str:
    if status == "MATCH":
        return "passing"
    if status in NON_FAILING_STATUSES:
        return "accepted_gap"
    return "needs_triage"


def corpus_paths(corpus: Path) -> list[Path]:
    if corpus.is_file():
        return [corpus]
    return sorted(corpus.glob("*.toml"))


def load_cases(corpus: Path) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for path in corpus_paths(corpus):
        data = tomllib.loads(path.read_text())
        for case in data.get("case", []):
            case = dict(case)
            case.setdefault("scope", "unspecified")
            case.setdefault(
                "category",
                DEFAULT_CATEGORY_BY_SCOPE.get(case["scope"], "runtime-semantic"),
            )
            case.setdefault("priority", "unspecified")
            case.setdefault("layer", DEFAULT_LAYER_BY_SCOPE.get(case["scope"], "runtime"))
            case.setdefault("origin", "corpus")
            case["_path"] = str(path)
            if "name" not in case or "source" not in case:
                raise ValueError(f"{path}: every case needs name and source")
            case["modules"] = normalize_case_modules(path, case)
            case["root_cause"] = normalize_case_root_cause(path, case)
            if case["category"] not in VALID_CATEGORIES:
                raise ValueError(
                    f"{path}: unknown category `{case['category']}`; "
                    f"expected one of {sorted(VALID_CATEGORIES)}"
                )
            if case["layer"] not in LAYERS:
                raise ValueError(
                    f"{path}: unknown layer `{case['layer']}`; expected one of {list(LAYERS)}"
                )
            if (
                "expected" in case
                and case["expected"] not in EXPECTED_STATUS_BY_MARKER
            ):
                raise ValueError(
                    f"{path}: unknown expected marker `{case['expected']}`"
                )
            cases.append(case)
    return cases


def normalize_case_modules(path: Path, case: dict[str, Any]) -> list[str]:
    modules = case.get("modules", case.get("module", case["scope"]))
    if isinstance(modules, str):
        normalized = [modules]
    elif isinstance(modules, list) and all(isinstance(module, str) for module in modules):
        normalized = modules
    else:
        raise ValueError(f"{path}: modules must be a string or list of strings")

    normalized = sorted({module.strip() for module in normalized if module.strip()})
    if not normalized:
        raise ValueError(f"{path}: modules must contain at least one module name")
    return normalized


def normalize_case_root_cause(path: Path, case: dict[str, Any]) -> str:
    root_cause = case.get(
        "root_cause",
        f"{case['category']}:{','.join(case['modules'])}",
    )
    if not isinstance(root_cause, str) or not root_cause.strip():
        raise ValueError(f"{path}: root_cause must be a non-empty string")
    return root_cause.strip()


def oracle_version(cpython: str) -> str:
    completed = subprocess.run(
        [cpython, "-c", "import platform; print(platform.python_version())"],
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr.strip() or "failed to run CPython oracle")
    return completed.stdout.strip()


def run_sweep(args: argparse.Namespace) -> tuple[dict[str, Any], list[SweepResult]]:
    version = oracle_version(args.cpython)
    if version != args.require_version:
        raise SystemExit(
            f"CPython oracle version mismatch: expected {args.require_version}, got {version}"
        )

    minipython = str(Path(args.minipython))
    if not Path(minipython).exists():
        raise SystemExit(f"MiniPython executable not found: {minipython}")

    scopes = {scope.strip() for scope in args.scope.split(",") if scope.strip()}
    categories = {
        category.strip() for category in args.category.split(",") if category.strip()
    }
    unknown_categories = sorted(categories - VALID_CATEGORIES)
    if unknown_categories:
        raise SystemExit(f"Unknown gap sweep categories: {', '.join(unknown_categories)}")
    modules = {module.strip() for module in args.module.split(",") if module.strip()}
    run_all_modules = not modules or "*" in modules
    root_causes = {
        root_cause.strip()
        for root_cause in args.root_cause.split(",")
        if root_cause.strip()
    }
    run_all_root_causes = not root_causes or "*" in root_causes
    layers = {layer.strip() for layer in args.layer.split(",") if layer.strip()}
    unknown_layers = sorted(layers - set(LAYERS))
    if unknown_layers:
        raise SystemExit(f"Unknown discovery layers: {', '.join(unknown_layers)}")
    if args.generated_cases < 0:
        raise SystemExit("--generated-cases must be non-negative")
    if args.shrink_max_attempts < 0:
        raise SystemExit("--shrink-max-attempts must be non-negative")

    loaded_cases = load_cases(Path(args.corpus))
    generated = generate_cases(
        args.seed,
        args.generated_cases,
        [layer for layer in LAYERS if layer in layers],
    )
    all_cases = loaded_cases + generated
    names = [case["name"] for case in all_cases]
    duplicate_names = sorted(name for name, count in Counter(names).items() if count > 1)
    if duplicate_names:
        raise SystemExit(f"Duplicate differential case names: {', '.join(duplicate_names)}")
    cases = [
        case
        for case in all_cases
        if case["scope"] in scopes
        and case["category"] in categories
        and case["layer"] in layers
        and (run_all_modules or modules.intersection(case["modules"]))
        and (run_all_root_causes or case["root_cause"] in root_causes)
    ]
    started = time.time()
    results: list[SweepResult] = []
    shrunk_root_causes: set[str] = set()
    cpython_command = [args.cpython, "-I", "-B", "-"]
    minipython_command = [minipython]
    # The status files live in one temporary directory that lasts for the
    # whole sweep and is removed with it, so the sweep leaves nothing
    # behind in the temporary directory.
    with tempfile.TemporaryDirectory(prefix="minipython-gap-sweep-") as status_dir:
        status_files = StatusFiles(Path(status_dir))
        # The sweep reads every run's status from its status file, so a
        # binary that cannot write one cannot be swept at all.
        check_status_file_support(minipython, status_files.next(), args.timeout)
        for case in cases:
            source = textwrap.dedent(case["source"]).lstrip("\n")
            cpython = run_command(cpython_command, source, args.timeout)
            mini = run_minipython(
                minipython_command,
                source,
                args.timeout,
                status_files.next(),
            )
            status, diff = classify_case(cpython, mini, case)
            result = SweepResult(
                name=case["name"],
                scope=case["scope"],
                category=case["category"],
                root_cause=case["root_cause"],
                modules=case["modules"],
                priority=case["priority"],
                status=status,
                triage_status=triage_status(status),
                expected=case.get("expected"),
                diff=diff,
                cpython=cpython,
                minipython=mini,
                layer=case["layer"],
                origin=case.get("origin", "corpus"),
                seed=case.get("seed"),
                source=source,
            )
            if (
                args.shrink
                and result.origin == "generated"
                and result.triage_status == "needs_triage"
                and result.root_cause not in shrunk_root_causes
            ):
                shrunk_root_causes.add(result.root_cause)
                signature = failure_signature(status, cpython, mini)

                def preserves_failure(candidate: str) -> bool:
                    candidate_cpython = run_command(cpython_command, candidate, args.timeout)
                    candidate_mini = run_minipython(
                        minipython_command,
                        candidate,
                        args.timeout,
                        status_files.next(),
                    )
                    candidate_status, _ = classify_case(
                        candidate_cpython, candidate_mini, case
                    )
                    return failure_signature(
                        candidate_status, candidate_cpython, candidate_mini
                    ) == signature

                minimized, attempts = minimize_source(
                    source,
                    preserves_failure,
                    max_attempts=args.shrink_max_attempts,
                )
                result.minimized_source = minimized
                result.shrink_attempts = attempts
                write_repro(Path(args.repro_dir), result)
            results.append(result)

    meta = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "duration_seconds": round(time.time() - started, 3),
        "cwd": os.getcwd(),
        "required_cpython_version": args.require_version,
        "cpython_executable": str(Path(args.cpython).resolve()),
        "cpython_version": version,
        "driver_executable": str(Path(sys.executable).resolve()),
        "driver_python": platform.python_version(),
        "minipython_executable": str(Path(minipython).resolve()),
        "minipython_sha256": file_sha256(Path(minipython)),
        "minipython_git_revision": git_revision(),
        "corpus": str(Path(args.corpus)),
        "scope": sorted(scopes),
        "category": sorted(categories),
        "module": ["*"] if run_all_modules else sorted(modules),
        "root_cause": ["*"] if run_all_root_causes else sorted(root_causes),
        "layer": sorted(layers),
        "seed": args.seed,
        "generated_cases_requested": args.generated_cases,
        "generated_cases_selected": sum(
            result.origin == "generated" for result in results
        ),
        "corpus_cases_selected": sum(result.origin == "corpus" for result in results),
        "shrink": args.shrink,
    }
    return meta, results


def file_sha256(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def git_revision() -> str | None:
    """Return HEAD plus a -dirty suffix, so reports name the tested source."""
    root = Path(__file__).resolve().parent.parent
    try:
        head = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
        dirty = subprocess.run(
            ["git", "-C", str(root), "status", "--porcelain", "--untracked-files=no"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None
    return f"{head}-dirty" if dirty else head


def failure_signature(
    status: str, cpython: RunResult, minipython: RunResult
) -> tuple[Any, ...]:
    """Identify a root symptom tightly enough for deterministic reduction."""
    return (
        status,
        cpython.exit_code == 0,
        minipython.exit_code == 0,
        cpython.exception_class,
        minipython.exception_class,
        cpython.timeout,
        minipython.timeout,
    )


def write_repro(directory: Path, result: SweepResult) -> None:
    if result.minimized_source is None:
        return
    directory.mkdir(parents=True, exist_ok=True)
    safe_name = re.sub(r"[^A-Za-z0-9_.-]+", "-", result.name).strip("-")
    header = (
        f"# generated differential repro\n"
        f"# case: {result.name}\n"
        f"# layer: {result.layer}\n"
        f"# root_cause: {result.root_cause}\n"
        f"# seed: {result.seed}\n"
        f"# status: {result.status}\n"
    )
    (directory / f"{safe_name}.py").write_text(header + result.minimized_source)


def write_reports(prefix: Path, meta: dict[str, Any], results: list[SweepResult]) -> None:
    prefix.parent.mkdir(parents=True, exist_ok=True)
    open_summaries = open_root_causes(results)
    json_payload = {
        "meta": meta,
        "summary": dict(Counter(result.status for result in results)),
        "triage": dict(Counter(result.triage_status for result in results)),
        "categories": dict(Counter(result.category for result in results)),
        "layers": dict(Counter(result.layer for result in results)),
        "origins": dict(Counter(result.origin for result in results)),
        "root_causes": dict(Counter(result.root_cause for result in results)),
        "root_cause_summary": summarize_root_causes(results),
        "open_root_causes": open_summaries,
        "open_root_cause_commands": open_root_cause_commands(open_summaries),
        "modules": dict(Counter(module for result in results for module in result.modules)),
        "results": [asdict(result) for result in results],
    }
    (prefix.with_suffix(".json")).write_text(json.dumps(json_payload, indent=2) + "\n")
    (prefix.with_suffix(".md")).write_text(render_markdown(meta, results))


def summarize_root_causes(results: list[SweepResult]) -> dict[str, dict[str, Any]]:
    summaries: dict[str, dict[str, Any]] = {}
    for result in results:
        summary = summaries.setdefault(
            result.root_cause,
            {
                "count": 0,
                "triage": Counter(),
                "statuses": Counter(),
                "modules": set(),
                "categories": set(),
                "priorities": Counter(),
                "cases": [],
            },
        )
        summary["count"] += 1
        summary["triage"][result.triage_status] += 1
        summary["statuses"][result.status] += 1
        summary["modules"].update(result.modules)
        summary["categories"].add(result.category)
        summary["priorities"][result.priority] += 1
        summary["cases"].append(result.name)

    normalized: dict[str, dict[str, Any]] = {}
    for root_cause in sorted(summaries):
        summary = summaries[root_cause]
        normalized[root_cause] = {
            "count": summary["count"],
            "triage": sorted_counter(summary["triage"]),
            "statuses": sorted_counter(summary["statuses"]),
            "modules": sorted(summary["modules"]),
            "categories": sorted(summary["categories"]),
            "priorities": sorted_counter(summary["priorities"]),
            "cases": sorted(summary["cases"]),
        }
    return normalized


def open_root_causes(
    results: list[SweepResult], priorities: set[str] | None = None
) -> dict[str, dict[str, Any]]:
    return {
        root_cause: summary
        for root_cause, summary in summarize_root_causes(results).items()
        if summary["triage"].get("needs_triage", 0)
        and (
            priorities is None
            or any(priority in priorities for priority in summary["priorities"])
        )
    }


def open_root_cause_commands(
    open_summaries: dict[str, dict[str, Any]],
) -> dict[str, list[str]]:
    return {
        root_cause: focused_root_cause_command(root_cause)
        for root_cause in open_summaries
    }


def focused_root_cause_command(root_cause: str) -> list[str]:
    return ["tests/run.sh", "--root-cause", root_cause]


def format_command(command: list[str]) -> str:
    return " ".join(command)


def format_open_root_causes(open_summaries: dict[str, dict[str, Any]]) -> str:
    return ", ".join(
        f"{root_cause}({summary['triage']['needs_triage']})"
        for root_cause, summary in open_summaries.items()
    )


def sorted_counter(counter: Counter[str]) -> dict[str, int]:
    return {key: counter[key] for key in sorted(counter)}


def format_counts(counts: dict[str, int]) -> str:
    if not counts:
        return "none"
    return ", ".join(f"{key}={count}" for key, count in counts.items())


def render_markdown(meta: dict[str, Any], results: list[SweepResult]) -> str:
    summary = Counter(result.status for result in results)
    triage_summary = Counter(result.triage_status for result in results)
    root_cause_summary = summarize_root_causes(results)
    open_summaries = open_root_causes(results)
    lines = [
        "# CPython Gap Sweep",
        "",
        f"- Generated: `{meta['generated_at']}`",
        f"- Required CPython: `{meta['required_cpython_version']}`",
        f"- CPython: `{meta['cpython_version']}` at `{meta['cpython_executable']}`",
        f"- Driver Python: `{meta['driver_python']}` at `{meta['driver_executable']}`",
        f"- MiniPython: `{meta['minipython_executable']}`",
        f"- Corpus: `{meta['corpus']}`",
        f"- Scopes: `{', '.join(meta['scope'])}`",
        f"- Categories: `{', '.join(meta['category'])}`",
        f"- Modules: `{', '.join(meta['module'])}`",
        f"- Root Causes: `{', '.join(meta['root_cause'])}`",
        f"- Layers: `{', '.join(meta.get('layer', []))}`",
        f"- Fixed Seed: `{meta.get('seed', 'none')}`",
        f"- Generated Cases: `{meta.get('generated_cases_selected', 0)}`",
        f"- Corpus Cases: `{meta.get('corpus_cases_selected', len(results))}`",
        f"- Shrinking: `{meta.get('shrink', False)}`",
        f"- Duration: `{meta['duration_seconds']}s`",
        "",
        "## Summary",
        "",
        "| Status | Count |",
        "| --- | ---: |",
    ]
    for status in STATUSES:
        if summary[status]:
            lines.append(f"| `{status}` | {summary[status]} |")
    lines.extend(
        [
            "",
            "## Triage",
            "",
            "| Triage Status | Count |",
            "| --- | ---: |",
        ]
    )
    for status in TRIAGE_STATUSES:
        if triage_summary[status]:
            lines.append(f"| `{status}` | {triage_summary[status]} |")
    lines.extend(
        [
            "",
            "## Open Root Causes",
            "",
        ]
    )
    if open_summaries:
        lines.extend(
            [
                "| Root Cause | Open Cases | Triage | Statuses | Modules | Focused Command |",
                "| --- | ---: | --- | --- | --- | --- |",
            ]
        )
        for root_cause, data in open_summaries.items():
            command = format_command(focused_root_cause_command(root_cause))
            lines.append(
                f"| `{root_cause}` | {data['triage']['needs_triage']} | `{format_counts(data['triage'])}` | `{format_counts(data['statuses'])}` | `{', '.join(data['modules'])}` | `{command}` |"
            )
    else:
        lines.append("No open root causes.")
    lines.extend(
        [
            "",
            "## Categories",
            "",
            "| Category | Count |",
            "| --- | ---: |",
        ]
    )
    for category, count in sorted(Counter(result.category for result in results).items()):
        lines.append(f"| `{category}` | {count} |")
    lines.extend(
        [
            "",
            "## Layers",
            "",
            "| Layer | Count |",
            "| --- | ---: |",
        ]
    )
    for layer, count in sorted(Counter(result.layer for result in results).items()):
        lines.append(f"| `{layer}` | {count} |")
    lines.extend(
        [
            "",
            "## Root Causes",
            "",
            "| Root Cause | Cases | Triage | Statuses | Modules |",
            "| --- | ---: | --- | --- | --- |",
        ]
    )
    for root_cause, data in root_cause_summary.items():
        lines.append(
            f"| `{root_cause}` | {data['count']} | `{format_counts(data['triage'])}` | `{format_counts(data['statuses'])}` | `{', '.join(data['modules'])}` |"
        )
    lines.extend(
        [
            "",
            "## Modules",
            "",
            "| Module | Count |",
            "| --- | ---: |",
        ]
    )
    for module, count in sorted(
        Counter(module for result in results for module in result.modules).items()
    ):
        lines.append(f"| `{module}` | {count} |")
    lines.extend(
        [
            "",
            "## Cases",
            "",
            "| Case | Origin | Layer | Scope | Category | Root Cause | Modules | Priority | Status | Triage |",
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for result in results:
        lines.append(
            f"| `{result.name}` | `{result.origin}` | `{result.layer}` | `{result.scope}` | `{result.category}` | `{result.root_cause}` | `{', '.join(result.modules)}` | `{result.priority}` | `{result.status}` | `{result.triage_status}` |"
        )
    differing = [result for result in results if result.status != "MATCH"]
    if differing:
        lines.extend(["", "## Differences", ""])
        for result in differing:
            lines.extend(
                [
                    f"### `{result.name}`",
                    "",
                    f"- Status: `{result.status}`",
                    f"- Origin: `{result.origin}`",
                    f"- Layer: `{result.layer}`",
                    f"- Scope: `{result.scope}`",
                    f"- Category: `{result.category}`",
                    f"- Root Cause: `{result.root_cause}`",
                    f"- Modules: `{', '.join(result.modules)}`",
                    f"- Triage: `{result.triage_status}`",
                    f"- Priority: `{result.priority}`",
                    f"- Diff: `{result.diff}`",
                    "",
                    "```text",
                    "CPython stdout:",
                    result.cpython.stdout.rstrip(),
                    "CPython stderr:",
                    result.cpython.stderr.rstrip(),
                    "MiniPython stdout:",
                    result.minipython.stdout.rstrip(),
                    "MiniPython stderr:",
                    result.minipython.stderr.rstrip(),
                    "```",
                    "",
                ]
            )
            if result.minimized_source is not None:
                lines.extend(
                    [
                        f"- Shrink Attempts: `{result.shrink_attempts}`",
                        "",
                        "```python",
                        result.minimized_source.rstrip(),
                        "```",
                        "",
                    ]
                )
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    args = parse_args()
    meta, results = run_sweep(args)
    write_reports(Path(args.out), meta, results)
    summary = Counter(result.status for result in results)
    print(
        "gap sweep:",
        ", ".join(f"{status}={summary[status]}" for status in STATUSES if summary[status]),
    )
    open_summaries = open_root_causes(results)
    fail_priorities = {
        priority.strip()
        for priority in getattr(args, "fail_priority", "").split(",")
        if priority.strip()
    }
    gated = bool(args.fail_on_open or args.fail_on_diff or fail_priorities)
    hard_failures = [
        result for result in results if result.status in ALWAYS_FAILING_STATUSES
    ]
    if gated and hard_failures:
        print(
            "crashes or broken contracts (fail regardless of priority):",
            ", ".join(f"{result.name}={result.status}" for result in hard_failures),
            file=sys.stderr,
        )
        return 1
    if args.fail_on_open and open_summaries:
        print(
            "open root causes:",
            format_open_root_causes(open_summaries),
            file=sys.stderr,
        )
        return 1
    if fail_priorities:
        priority_open = open_root_causes(results, fail_priorities)
        if priority_open:
            print(
                f"open {','.join(sorted(fail_priorities))} root causes:",
                format_open_root_causes(priority_open),
                file=sys.stderr,
            )
            return 1
    if args.fail_on_diff:
        bad = [result for result in results if result.status not in NON_FAILING_STATUSES]
        if bad:
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
