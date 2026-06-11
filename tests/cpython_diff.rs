use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use minipython::{run_source, run_source_bytes};

struct DiffCase {
    origin: &'static str,
    name: &'static str,
    source: &'static str,
}

struct ErrorMessageCase {
    origin: &'static str,
    name: &'static str,
    source: &'static str,
    expected_message: &'static str,
}

struct BytesDiffCase {
    origin: &'static str,
    name: &'static str,
    source: &'static [u8],
}

#[derive(Clone, Copy)]
struct BytesSource<'a> {
    origin: &'static str,
    name: &'static str,
    source: &'a [u8],
}

const MINIPYTHON_DIFF_STACK_SIZE: usize = 32 * 1024 * 1024;

impl<'a> From<&'a BytesDiffCase> for BytesSource<'a> {
    fn from(case: &'a BytesDiffCase) -> Self {
        Self {
            origin: case.origin,
            name: case.name,
            source: case.source,
        }
    }
}

fn assert_cpython_output_parity(case: &DiffCase) {
    assert_cpython_output_parity_source(case.origin, case.name, case.source);
}

fn assert_cpython_output_parity_source(origin: &str, name: &str, source: &str) {
    let cpython_output = run_cpython(source).unwrap_or_else(|message| {
        panic!(
            "failed to run CPython for {}::{}\nsource:\n{}\n\n{}",
            origin, name, source, message
        )
    });
    assert!(
        cpython_output.status.success(),
        "expected CPython to accept {}::{}\nsource:\n{}\n\nstderr:\n{}",
        origin,
        name,
        source,
        String::from_utf8_lossy(&cpython_output.stderr)
    );

    let cpython_stdout = String::from_utf8(cpython_output.stdout)
        .unwrap_or_else(|error| panic!("CPython emitted non-UTF-8 output: {error}"));
    let cpython_lines: Vec<String> = cpython_stdout.lines().map(str::to_string).collect();

    assert_eq!(
        run_minipython_source(source),
        Ok(cpython_lines),
        "MiniPython output differs from CPython for {}::{}\nsource:\n{}",
        origin,
        name,
        source
    );
}

fn cpython_formatfloat_testfile_source() -> Option<String> {
    let path = "/Volumes/samsung/GitHub/cpython/Lib/test/mathdata/formatfloat_testcases.txt";
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping CPython float format dataset diff: missing {path}");
            return None;
        }
        Err(error) => panic!("failed to read {path}: {error}"),
    };
    Some(format!(
        r#"data = {data:?}
cases = []
for line in data.splitlines():
    line = line.strip()
    if not line or line.startswith('--'):
        continue
    lhs, expected = [part.strip() for part in line.split('->')]
    fmt, arg = lhs.split()
    cases.append((fmt, arg, expected))

checks = 0
failures = 0
for fmt, arg, expected in cases:
    value = float(arg)
    for label, got, wanted in [
        ('percent', fmt % value, expected),
        ('percent-neg', fmt % -value, '-' + expected),
    ]:
        checks += 1
        if got != wanted:
            print('mismatch', label, fmt, arg, repr(got), repr(wanted))
            failures += 1
    if fmt != '%r':
        spec = fmt[1:]
        for label, got, wanted in [
            ('format', format(value, spec), expected),
            ('format-neg', format(-value, spec), '-' + expected),
        ]:
            checks += 1
            if got != wanted:
                print('mismatch', label, fmt, arg, repr(got), repr(wanted))
                failures += 1
print('checked', len(cases), checks, 'failures', failures)"#
    ))
}

fn cpython_floating_points_repr_source() -> Option<String> {
    let path = "/Volumes/samsung/GitHub/cpython/Lib/test/mathdata/floating_points.txt";
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping CPython float repr dataset diff: missing {path}");
            return None;
        }
        Err(error) => panic!("failed to read {path}: {error}"),
    };
    Some(format!(
        r#"import math
data = {data:?}
checked = 0
failures = 0
for line in data.splitlines():
    text = line.strip()
    if not text or text.startswith('#'):
        continue
    value = eval(text)
    rendered = repr(value)
    roundtrip = eval(rendered)
    if value != roundtrip:
        print('mismatch', text, rendered, roundtrip)
        failures += 1
    if value == 0.0 and math.copysign(1.0, value) != math.copysign(1.0, roundtrip):
        print('zero-sign-mismatch', text, rendered)
        failures += 1
    checked += 1
print('checked', checked, 'failures', failures)"#
    ))
}

fn assert_cpython_rejection_parity(case: &DiffCase) {
    let cpython_output = run_cpython(case.source).unwrap_or_else(|message| {
        panic!(
            "failed to run CPython for {}::{}\nsource:\n{}\n\n{}",
            case.origin, case.name, case.source, message
        )
    });
    assert!(
        !cpython_output.status.success(),
        "expected CPython to reject {}::{}\nsource:\n{}\n\nstdout:\n{}",
        case.origin,
        case.name,
        case.source,
        String::from_utf8_lossy(&cpython_output.stdout)
    );

    assert!(
        run_minipython_source(case.source).is_err(),
        "expected MiniPython to reject {}::{}\nsource:\n{}",
        case.origin,
        case.name,
        case.source
    );
}

fn assert_cpython_error_message_parity(case: &ErrorMessageCase) {
    let cpython_output = run_cpython(case.source).unwrap_or_else(|message| {
        panic!(
            "failed to run CPython for {}::{}\nsource:\n{}\n\n{}",
            case.origin, case.name, case.source, message
        )
    });
    assert!(
        !cpython_output.status.success(),
        "expected CPython to reject {}::{}\nsource:\n{}\n\nstdout:\n{}",
        case.origin,
        case.name,
        case.source,
        String::from_utf8_lossy(&cpython_output.stdout)
    );

    let cpython_stderr = String::from_utf8_lossy(&cpython_output.stderr);
    assert!(
        cpython_error_message_matches(&cpython_stderr, case.expected_message),
        "CPython error for {}::{} did not contain {:?}\nsource:\n{}\n\nstderr:\n{}",
        case.origin,
        case.name,
        case.expected_message,
        case.source,
        cpython_stderr
    );

    let minipython_error = run_minipython_source(case.source)
        .expect_err("expected MiniPython to reject the CPython rejection case");
    assert!(
        minipython_error.contains(case.expected_message),
        "MiniPython error for {}::{} did not contain {:?}\nsource:\n{}\n\nerror:\n{}",
        case.origin,
        case.name,
        case.expected_message,
        case.source,
        minipython_error
    );
}

fn cpython_error_message_matches(stderr: &str, expected: &str) -> bool {
    stderr.contains(expected) || cpython_legacy_error_message_matches(stderr, expected)
}

fn cpython_legacy_error_message_matches(stderr: &str, expected: &str) -> bool {
    match expected {
        // macOS system Python 3.9 reports the pre-3.10 wording for the local
        // CPython test_grammar.py EOF samples; MiniPython still targets the
        // current wording.
        "was never closed" => stderr.contains("unexpected EOF while parsing"),
        // CPython 3.9 has older invalid-del-target wording than current
        // test_syntax.py, which matches MiniPython's target wording.
        "cannot use starred expression" => stderr.contains("can't use starred expression here"),
        "cannot delete expression" => stderr.contains("cannot delete operator"),
        // Older Python releases reported mixed except/except* forms as a
        // generic syntax error before CPython gained the dedicated diagnostic.
        "cannot have both 'except' and 'except*' on the same 'try'" => {
            stderr.contains("invalid syntax")
        }
        // Older Python releases used the generic syntax wording for the
        // barry_as_flufl regression shape before the dedicated parser message.
        "expected ':'" => stderr.contains("invalid syntax"),
        // Older Python releases report the nested delimiter/string examples
        // from current test_syntax.py with pre-PEG or pre-specialized wording.
        "does not match opening parenthesis" => stderr.contains("invalid syntax"),
        "unterminated string literal" => stderr.contains("EOL while scanning string literal"),
        "perhaps you escaped the end quote" => stderr.contains("EOL while scanning string literal"),
        "unterminated triple-quoted string literal" => {
            stderr.contains("EOF while scanning triple-quoted string literal")
        }
        "invalid non-printable character" => stderr.contains("invalid syntax"),
        "Perhaps you forgot a comma" => stderr.contains("invalid syntax"),
        "cannot use except statement with attribute"
        | "cannot use attribute as pattern target"
        | "expected expression before 'if', but statement is given" => {
            stderr.contains("invalid syntax")
        }
        _ => false,
    }
}

fn run_minipython_source(source: &str) -> Result<Vec<String>, String> {
    let source = source.to_string();
    let handle = std::thread::Builder::new()
        .name("minipython-cpython-diff".to_string())
        .stack_size(MINIPYTHON_DIFF_STACK_SIZE)
        .spawn(move || run_source(&source))
        .expect("failed to spawn MiniPython differential test thread");
    match handle.join() {
        Ok(result) => result.map(minipython_print_records_to_stdout_lines),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn run_minipython_source_bytes(source: &[u8]) -> Result<Vec<String>, String> {
    let source = source.to_vec();
    let handle = std::thread::Builder::new()
        .name("minipython-cpython-diff-bytes".to_string())
        .stack_size(MINIPYTHON_DIFF_STACK_SIZE)
        .spawn(move || run_source_bytes(&source))
        .expect("failed to spawn MiniPython bytes differential test thread");
    match handle.join() {
        Ok(result) => result.map(minipython_print_records_to_stdout_lines),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn minipython_print_records_to_stdout_lines(records: Vec<String>) -> Vec<String> {
    let mut stdout = String::new();
    for record in records {
        stdout.push_str(&record);
        stdout.push('\n');
    }
    stdout.lines().map(str::to_string).collect()
}

fn assert_cpython_bytes_output_parity(case: &BytesDiffCase) {
    assert_cpython_bytes_source_output_parity(BytesSource::from(case));
}

fn assert_cpython_owned_bytes_exec_output_parity(
    origin: &'static str,
    name: &'static str,
    source: Vec<u8>,
) {
    assert_cpython_bytes_exec_source_output_parity(BytesSource {
        origin,
        name,
        source: &source,
    });
}

fn assert_cpython_bytes_source_output_parity(case: BytesSource<'_>) {
    let cpython_output = run_cpython_bytes(case).unwrap_or_else(|message| {
        panic!(
            "failed to run CPython for {}::{}\n\n{}",
            case.origin, case.name, message
        )
    });
    assert!(
        cpython_output.status.success(),
        "expected CPython to accept {}::{}\n\nstderr:\n{}",
        case.origin,
        case.name,
        String::from_utf8_lossy(&cpython_output.stderr)
    );

    let cpython_stdout = String::from_utf8(cpython_output.stdout)
        .unwrap_or_else(|error| panic!("CPython emitted non-UTF-8 output: {error}"));
    let cpython_lines: Vec<String> = cpython_stdout.lines().map(str::to_string).collect();

    assert_eq!(
        run_minipython_source_bytes(case.source),
        Ok(cpython_lines),
        "MiniPython bytes-source output differs from CPython for {}::{}",
        case.origin,
        case.name
    );
}

fn assert_cpython_bytes_exec_source_output_parity(case: BytesSource<'_>) {
    let cpython_output = run_cpython_exec_bytes(case).unwrap_or_else(|message| {
        panic!(
            "failed to run CPython bytes exec for {}::{}\n\n{}",
            case.origin, case.name, message
        )
    });
    assert!(
        cpython_output.status.success(),
        "expected CPython bytes exec to accept {}::{}\n\nstderr:\n{}",
        case.origin,
        case.name,
        String::from_utf8_lossy(&cpython_output.stderr)
    );

    let cpython_stdout = String::from_utf8(cpython_output.stdout)
        .unwrap_or_else(|error| panic!("CPython emitted non-UTF-8 output: {error}"));
    let cpython_lines: Vec<String> = cpython_stdout.lines().map(str::to_string).collect();

    assert_eq!(
        run_minipython_source_bytes(case.source),
        Ok(cpython_lines),
        "MiniPython bytes-source output differs from CPython bytes exec for {}::{}",
        case.origin,
        case.name
    );
}

fn assert_cpython_bytes_rejection_parity(case: &BytesDiffCase) {
    let cpython_output = run_cpython_bytes(BytesSource::from(case)).unwrap_or_else(|message| {
        panic!(
            "failed to run CPython for {}::{}\n\n{}",
            case.origin, case.name, message
        )
    });
    assert!(
        !cpython_output.status.success(),
        "expected CPython to reject {}::{}\n\nstdout:\n{}",
        case.origin,
        case.name,
        String::from_utf8_lossy(&cpython_output.stdout)
    );

    assert!(
        run_minipython_source_bytes(case.source).is_err(),
        "expected MiniPython to reject bytes source for {}::{}",
        case.origin,
        case.name
    );
}

fn run_cpython(source: &str) -> Result<Output, String> {
    let executable = env::var("MINIPYTHON_CPYTHON").unwrap_or_else(|_| "python3".to_string());
    Command::new(&executable)
        .arg("-I")
        .arg("-c")
        .arg(source)
        .output()
        .map_err(|error| format!("could not execute {executable:?}: {error}"))
}

fn run_cpython_bytes(case: BytesSource<'_>) -> Result<Output, String> {
    let executable = env::var("MINIPYTHON_CPYTHON").unwrap_or_else(|_| "python3".to_string());
    let path = cpython_bytes_case_path(case.name);
    fs::write(&path, case.source)
        .map_err(|error| format!("could not write CPython temp source {path:?}: {error}"))?;

    let output = Command::new(&executable)
        .arg("-I")
        .arg(&path)
        .output()
        .map_err(|error| format!("could not execute {executable:?}: {error}"));

    let _ = fs::remove_file(&path);
    output
}

fn run_cpython_exec_bytes(case: BytesSource<'_>) -> Result<Output, String> {
    let executable = env::var("MINIPYTHON_CPYTHON").unwrap_or_else(|_| "python3".to_string());
    let path = cpython_bytes_case_path(case.name);
    fs::write(&path, case.source)
        .map_err(|error| format!("could not write CPython temp source {path:?}: {error}"))?;

    let output = Command::new(&executable)
        .arg("-I")
        .arg("-c")
        .arg("import sys; exec(open(sys.argv[1], 'rb').read())")
        .arg(&path)
        .output()
        .map_err(|error| format!("could not execute {executable:?}: {error}"));

    let _ = fs::remove_file(&path);
    output
}

fn cpython_bytes_case_path(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let safe_name: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    path.push(format!(
        "minipython-cpython-diff-{}-{safe_name}.py",
        std::process::id()
    ));
    path
}

#[test]
fn cpython_json_loads_dumps_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads/dumps core data model subset",
        name: "json-loads-dumps-core-values",
        source: r#"import json
import math
from collections import namedtuple
from enum import IntEnum
source = '{"a": 1, "b": [true, false, null], "c": "x\\ny"}'
value = json.loads(source)
print(value)
print(value['a'], value['b'], repr(value['c']))
print(json.loads(b'[1, 2.5, -3, 4e2]'))
print(json.loads(bytearray(b'{"ba": true}')))
def u16le(text, bom=False):
    data = bytearray()
    if bom:
        data.extend(b'\xff\xfe')
    for ch in text:
        code = ord(ch)
        data.append(code & 255)
        data.append((code >> 8) & 255)
    return bytes(data)
def u16be(text):
    data = bytearray()
    for ch in text:
        code = ord(ch)
        data.append((code >> 8) & 255)
        data.append(code & 255)
    return bytes(data)
def u32le(text, bom=False):
    data = bytearray()
    if bom:
        data.extend(b'\xff\xfe\x00\x00')
    for ch in text:
        code = ord(ch)
        data.append(code & 255)
        data.append((code >> 8) & 255)
        data.append((code >> 16) & 255)
        data.append((code >> 24) & 255)
    return bytes(data)
def u32be(text, bom=False):
    data = bytearray()
    if bom:
        data.extend(b'\x00\x00\xfe\xff')
    for ch in text:
        code = ord(ch)
        data.append((code >> 24) & 255)
        data.append((code >> 16) & 255)
        data.append((code >> 8) & 255)
        data.append(code & 255)
    return bytes(data)
for label, data in [
    ('utf8-bom', b'\xef\xbb\xbf{"enc": "utf8-bom"}'),
    ('utf16-le', u16le('{"enc": "utf16-le"}')),
    ('utf16-be', u16be('{"enc": "utf16-be"}')),
    ('utf16', u16le('{"enc": "utf16"}', True)),
    ('utf32-le', u32le('{"enc": "utf32-le"}')),
    ('utf32-be', u32be('{"enc": "utf32-be"}')),
    ('utf32-bom-le', u32le('{"enc": "utf32-bom-le"}', True)),
    ('utf32-bom-be', u32be('{"enc": "utf32-bom-be"}', True)),
]:
    print(label, json.loads(data))
class JsonStr(str):
    pass
class JsonBytes(bytes):
    pass
class JsonByteArray(bytearray):
    pass
print('bytearray-sub-utf16', json.loads(JsonByteArray(u16le('{"subenc": "bytearray-utf16"}'))))
print('bytes-sub-utf32', json.loads(JsonBytes(u32le('{"subenc": "bytes-utf32"}', True))))
class JsonInt(int):
    pass
class JsonFloat(float):
    pass
class JsonList(list):
    pass
class JsonTuple(tuple):
    pass
class JsonDict(dict):
    pass
for source in [JsonStr('{"sub": "str"}'), JsonBytes(b'{"sub": "bytes"}'), JsonByteArray(b'{"sub": "bytearray"}')]:
    print(type(source).__name__, json.loads(source))
print(json.dumps(JsonStr('dump-str-subclass')))
print(json.dumps({JsonStr('key'): JsonStr('value')}))
print(json.dumps([JsonInt(7), JsonFloat(2.5)]))
print(json.dumps({JsonInt(2): JsonInt(3), JsonFloat(1.5): JsonFloat(4.5)}))
class JsonCode(IntEnum):
    ok = 200
print(json.dumps(JsonCode.ok))
print(json.dumps({JsonCode.ok: JsonCode.ok}))
print(json.dumps(JsonList([1, 2])))
print(json.dumps(JsonTuple((3, 4))))
print(json.dumps(JsonDict({'nested': JsonList([JsonInt(5)])})))
JsonPoint = namedtuple('JsonPoint', 'x y')
print(json.dumps(JsonPoint(8, JsonInt(9))))
for item in [None, True, False, 0, -3, 12, 1.5, 'plain', 'a\nb', [1, True, None], {'a': 1, 'b': [False, None]}]:
    encoded = json.dumps(item)
    print(encoded)
    print(json.loads(encoded) == item)
print(json.dumps({'quote': '"', 'slash': '\\', 'nonascii': 'é'}))
print(json.dumps({2: 'two', 4.5: 'float', False: 'no', None: 'nil'}))
print(json.dumps(json.loads('"\\ud834\\udd20"')))
print(json.dumps(float('nan')), json.dumps(float('inf')), json.dumps(float('-inf')))
for text in ['NaN', 'Infinity', '-Infinity', '1e9999', '-1e9999']:
    value = json.loads(text)
    print(text, math.isnan(value), math.isinf(value), value < 0)"#,
    });
}

#[test]
fn cpython_json_keyword_argument_binding_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads/dumps keyword binding subset",
        name: "json-loads-dumps-keyword-binding",
        source: r#"import json

def show(label, callback):
    try:
        print(label, callback())
    except Exception as error:
        print(label, type(error).__name__, isinstance(error, TypeError))

print(json.loads(s='{"a": 1}')['a'])
print(json.loads(s=b'[1, 2]', strict=True))
print(json.dumps(obj={'b': [2]}, sort_keys=True))
print(json.dumps(obj='é', ensure_ascii=False))
show('loads-duplicate-s', lambda: json.loads('{}', s='[]'))
show('dumps-duplicate-obj', lambda: json.dumps({}, obj=[]))
show('loads-missing-s', lambda: json.loads(strict=False))
show('dumps-missing-obj', lambda: json.dumps(sort_keys=True))"#,
    });
}

#[test]
fn cpython_json_loads_escape_and_duplicate_key_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads escape and duplicate key subset",
        name: "json-loads-escapes-duplicate-keys",
        source: r#"import json
print(json.loads('{"a": 1, "a": 2}'))
print(repr(json.loads('"\\/\\b\\f\\r\\t"')))
print(json.dumps(json.loads('"\\/\\b\\f\\r\\t"')))"#,
    });
}

#[test]
fn cpython_json_loads_unicode_escape_roundtrip_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public unicode escape round-trip subset",
        name: "json-loads-unicode-escape-roundtrip",
        source: r#"import json
sources = [
    '"\\u0041"',
    '"\\u00e9"',
    '"\\u20ac"',
    '"\\ud834\\udd20"',
    '{"\\u0061": "\\u00e9", "music": "\\ud834\\udd20"}',
]
for source in sources:
    value = json.loads(source)
    print(repr(value))
    print(json.dumps(value, ensure_ascii=True))
    print(json.dumps(value, ensure_ascii=False))"#,
    });
}

#[test]
fn cpython_json_loads_strict_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads strict subset",
        name: "json-loads-strict",
        source: r#"import json
sources = ['"a' + chr(10) + 'b"', '"a' + chr(9) + 'b"', '"a' + chr(0) + 'b"', '{"x": "a' + chr(10) + 'b"}']
for strict in [True, 1, False, 0, []]:
    for source in sources:
        try:
            print(repr(strict), repr(json.loads(source, strict=strict)))
        except Exception as error:
            print(repr(strict), isinstance(error, ValueError))
try:
    json.loads('{}', strict=False, unknown=1)
except Exception as error:
    print('unknown', type(error).__name__, isinstance(error, TypeError))"#,
    });
}

#[test]
fn cpython_json_dumps_string_escape_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps string escape subset",
        name: "json-dumps-string-escapes",
        source: r#"import json
for value in ['\x00\x1f', '\b\f\n\r\t', '"\\', 'é', '𝄠']:
    print(json.dumps(value))"#,
    });
}

#[test]
fn cpython_json_dumps_key_coercion_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps dict key coercion subset",
        name: "json-dumps-key-coercion",
        source: r#"import json
from enum import IntEnum
class S(str):
    pass
class I(int):
    pass
class F(float):
    pass
class Code(IntEnum):
    ok = 200
cases = [
    {'s': 1, 2: 'two', 4.5: 'float', False: 'no', None: 'nil'},
    {S('sub'): S('value'), I(7): I(8), F(1.5): F(2.5)},
    {Code.ok: Code.ok},
]
for value in cases:
    print(json.dumps(value))
    print(json.dumps(value, ensure_ascii=False, separators=(',', ':')))"#,
    });
}

#[test]
fn cpython_json_dumps_ensure_ascii_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps ensure_ascii subset",
        name: "json-dumps-ensure-ascii",
        source: r#"import json
for ensure_ascii in [False, 0, True, 1]:
    print(json.dumps('é𝄠', ensure_ascii=ensure_ascii))
    print(json.dumps({'é': ['𝄠']}, ensure_ascii=ensure_ascii))"#,
    });
}

#[test]
fn cpython_json_dumps_sort_keys_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps sort_keys subset",
        name: "json-dumps-sort-keys",
        source: r#"import json
for sort_keys in [False, 0, True, 1]:
    print(json.dumps({'b': 1, 'a': 2}, sort_keys=sort_keys))
    print(json.dumps({'outer': {'b': 1, 'a': 2}}, sort_keys=sort_keys))
    print(json.dumps({2: 'two', 1: 'one'}, sort_keys=sort_keys))
for value in [True, 1]:
    try:
        json.dumps({'2': 's', 1: 'i'}, sort_keys=value)
    except Exception as error:
        print(type(error).__name__, isinstance(error, TypeError))"#,
    });
}

#[test]
fn cpython_json_dumps_separators_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps separators subset",
        name: "json-dumps-separators",
        source: r#"import json
class Sep(str):
    pass
class SepList(list):
    pass
class SepTuple(tuple):
    pass
value = {'b': [1, 2], 'a': {'é': '𝄠'}}
for separators in [None, (',', ':'), [',', ': '], (Sep(' | '), Sep(' => ')), SepList([',', ':']), SepTuple((Sep(' / '), Sep(' -> ')))]:
    print(json.dumps(value, separators=separators))
print(json.dumps({'é': ['𝄠', {'b': 1, 'a': 2}]}, ensure_ascii=False, sort_keys=True, separators=(',', ':')))
for separators in [(',',), (',', ':', 'x'), 'bad', (1, ':')]:
    try:
        json.dumps(value, separators=separators)
    except Exception as error:
        print(type(error).__name__, isinstance(error, (TypeError, ValueError)))"#,
    });
}

#[test]
fn cpython_json_dumps_skipkeys_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps skipkeys subset",
        name: "json-dumps-skipkeys",
        source: r#"import json
class K:
    pass
class S(str):
    pass
class I(int):
    pass
cases = [
    ({(1, 2): 'tuple', 'a': 1, None: 2, 3: 'three'}, False, False),
    ({(1, 2): 'tuple', 'a': 1, None: 2, 3: 'three'}, True, False),
    ({K(): 'custom', 'a': [1, 2]}, True, False),
    ({(1, 2): 'tuple', S('s'): I(4)}, True, False),
    ({(1, 2): 'tuple', 'b': 1, 'a': 2}, True, True),
]
for value, skipkeys, sort_keys in cases:
    try:
        print(json.dumps(value, skipkeys=skipkeys, sort_keys=sort_keys))
    except Exception as error:
        print(type(error).__name__, isinstance(error, TypeError))
print(json.dumps({(1, 2): 'tuple', 'é': '𝄠'}, skipkeys=True, ensure_ascii=False, separators=(',', ':')))
for skipkeys in [[], {}, K()]:
    try:
        json.dumps({(1, 2): 'tuple', 'a': 1}, skipkeys=skipkeys)
    except Exception as error:
        print(type(error).__name__, isinstance(error, TypeError))
    else:
        print('ok')"#,
    });
}

#[test]
fn cpython_json_dumps_allow_nan_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps allow_nan subset",
        name: "json-dumps-allow-nan",
        source: r#"import json
class F(float):
    pass
values = [float('nan'), float('inf'), float('-inf'), F(float('nan')), F(float('inf')), 1.5]
for allow_nan in [True, 1, False, 0]:
    for value in values:
        try:
            print(allow_nan, json.dumps(value, allow_nan=allow_nan))
        except Exception as error:
            print(allow_nan, type(error).__name__, isinstance(error, ValueError))
for allow_nan in [True, False]:
    try:
        print('key', allow_nan, json.dumps({float('nan'): 'nan', float('inf'): 'inf', 1.0: 'one'}, allow_nan=allow_nan))
    except Exception as error:
        print('key', allow_nan, type(error).__name__, isinstance(error, ValueError))
try:
    json.dumps([float('nan')], allow_nan=[])
except Exception as error:
    print('list', type(error).__name__, isinstance(error, ValueError))
else:
    print('list ok')"#,
    });
}

#[test]
fn cpython_json_dumps_check_circular_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps check_circular subset",
        name: "json-dumps-check-circular",
        source: r#"import json
from collections import namedtuple
cases = []
cycle_list = []
cycle_list.append(cycle_list)
cases.append(('list', cycle_list))
cycle_dict = {}
cycle_dict['self'] = cycle_dict
cases.append(('dict', cycle_dict))
inner = []
cycle_tuple = (inner,)
inner.append(cycle_tuple)
cases.append(('tuple', cycle_tuple))
Point = namedtuple('Point', 'items')
items = []
cycle_namedtuple = Point(items)
items.append(cycle_namedtuple)
cases.append(('namedtuple', cycle_namedtuple))
for check_circular in [True, 1, False, 0, []]:
    for label, value in cases:
        try:
            json.dumps(value, check_circular=check_circular)
        except Exception as error:
            print(repr(check_circular), label, type(error).__name__, isinstance(error, (ValueError, RecursionError)))
        else:
            print(repr(check_circular), label, 'OK')
for value in [[1, 2], {'a': [1]}, (1, 2)]:
    print(json.dumps(value, check_circular=False))
try:
    json.dumps([1], check_circular=object())
except Exception as error:
    print('object', type(error).__name__)
else:
    print('object ok')"#,
    });
}

#[test]
fn cpython_json_dumps_indent_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps indent subset",
        name: "json-dumps-indent",
        source: r#"import json
value = {'b': [1, {'x': 2}], 'a': {'é': '𝄠'}, 'empty': []}
for indent in [None, 0, 2, '', '--']:
    print('CASE', repr(indent))
    print(repr(json.dumps(value, indent=indent, sort_keys=True, ensure_ascii=False)))
for args in [dict(indent=2, separators=(',', ':')), dict(indent=2, separators=(', ', ': ')), dict(indent=0, separators=(',', ':'))]:
    print('SEP', args['indent'], repr(args['separators']))
    print(repr(json.dumps({'b': [1, 2], 'a': 3}, **args)))
for indent in [True, False, 1.5, [], object()]:
    try:
        print('BAD', repr(json.dumps([1, 2], indent=indent)))
    except Exception as error:
        print('BAD', type(error).__name__, isinstance(error, TypeError))"#,
    });
}

#[test]
fn cpython_json_dumps_float_spelling_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public dumps float spelling subset",
        name: "json-dumps-float-spelling",
        source: r#"import json
for value in [-0.0, 0.0, 1.0, -1.0, 1.2345, 1e-06, 1e+20]:
    print(repr(value), json.dumps(value))"#,
    });
}

#[test]
fn cpython_json_loads_number_and_whitespace_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads number grammar and whitespace subset",
        name: "json-loads-numbers-whitespace",
        source: r#"import json
print(json.loads(' \t\r\n[1, 2, 3]\n '))
value = json.loads('{"negzero": -0, "negfloat": -0.0, "exp": 6.02e+23, "small": 1E-2}')
print(value['negzero'], type(value['negzero']).__name__)
print(value['negfloat'], type(value['negfloat']).__name__)
print(value['exp'])
print(value['small'])"#,
    });
}

#[test]
fn cpython_json_loads_top_level_scalar_and_empty_container_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads top-level scalar and empty container subset",
        name: "json-loads-scalars-empty-containers",
        source: r#"import json
for source in ['null', 'true', 'false', '""', '[]', '{}', '[[], {}]', '{"empty_list": [], "empty_dict": {}}']:
    value = json.loads(source)
    print(source, repr(value), type(value).__name__)"#,
    });
}

#[test]
fn cpython_json_loads_nonfinite_constants_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads non-finite constant subset",
        name: "json-loads-nonfinite-constants",
        source: r#"import json, math
for source in ['NaN', 'Infinity', '-Infinity', '[NaN, Infinity, -Infinity]', '{"x": NaN, "y": Infinity}']:
    value = json.loads(source)
    print(source, type(value).__name__, repr(value))
    encoded = json.dumps(value)
    print(encoded)
    reparsed = json.loads(encoded)
    if isinstance(reparsed, list):
        print(math.isnan(reparsed[0]), math.isinf(reparsed[1]), math.isinf(reparsed[2]), reparsed[2] < 0)
    elif isinstance(reparsed, dict):
        print(math.isnan(reparsed['x']), math.isinf(reparsed['y']))
    else:
        print(math.isnan(reparsed) if source == 'NaN' else math.isinf(reparsed), reparsed < 0)"#,
    });
}

#[test]
fn cpython_json_loads_dumps_error_boundary_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads/dumps error boundary subset",
        name: "json-loads-dumps-error-boundaries",
        source: r#"import json
from collections import namedtuple

def show(label, callback):
    try:
        callback()
    except Exception as error:
        print(label, isinstance(error, TypeError), isinstance(error, ValueError))
    else:
        print(label, 'OK')

show('loads-no-args', lambda: json.loads())
show('loads-extra-arg', lambda: json.loads('{}', 1))
show('loads-unknown-keyword', lambda: json.loads('{}', unknown=1))
show('loads-memoryview', lambda: json.loads(memoryview(b'{}')))
show('loads-invalid-utf8', lambda: json.loads(b'\xff'))
show('loads-trailing-data', lambda: json.loads('{} []'))
show('loads-array-trailing-comma', lambda: json.loads('[1,]'))
show('loads-object-trailing-comma', lambda: json.loads('{"a": 1,}'))
show('loads-missing-colon', lambda: json.loads('{"a" 1}'))
show('loads-uppercase-true', lambda: json.loads('True'))
show('loads-uppercase-null', lambda: json.loads('NULL'))
show('loads-leading-zero', lambda: json.loads('01'))
show('loads-invalid-escape', lambda: json.loads('"\\x"'))
show('loads-unclosed-array', lambda: json.loads('[1'))
show('dumps-no-args', lambda: json.dumps())
show('dumps-extra-arg', lambda: json.dumps({}, 1))
show('dumps-unknown-keyword', lambda: json.dumps({}, unknown=1))
show('dumps-object', lambda: json.dumps(object()))
show('dumps-bytes', lambda: json.dumps(b'abc'))
show('dumps-bytearray', lambda: json.dumps(bytearray(b'abc')))
show('dumps-memoryview', lambda: json.dumps(memoryview(b'abc')))
show('dumps-bad-key', lambda: json.dumps({(1, 2): 3}))
cycle_list = []
cycle_list.append(cycle_list)
show('dumps-list-cycle', lambda: json.dumps(cycle_list))
cycle_dict = {}
cycle_dict['self'] = cycle_dict
show('dumps-dict-cycle', lambda: json.dumps(cycle_dict))
inner = []
cycle_tuple = (inner,)
inner.append(cycle_tuple)
show('dumps-tuple-cycle', lambda: json.dumps(cycle_tuple))
class JsonList(list):
    pass
class JsonDict(dict):
    pass
sub_list = JsonList()
sub_list.append(sub_list)
show('dumps-list-subclass-cycle', lambda: json.dumps(sub_list))
sub_dict = JsonDict()
sub_dict['self'] = sub_dict
show('dumps-dict-subclass-cycle', lambda: json.dumps(sub_dict))
items = []
Cycle = namedtuple('Cycle', 'items')
cycle_namedtuple = Cycle(items)
items.append(cycle_namedtuple)
show('dumps-namedtuple-cycle', lambda: json.dumps(cycle_namedtuple))"#,
    });
}

#[test]
fn cpython_json_loads_string_error_boundary_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/json public loads string error boundary subset",
        name: "json-loads-string-error-boundaries",
        source: r#"import json

def show(label, source):
    try:
        json.loads(source)
    except Exception as error:
        print(label, isinstance(error, ValueError))
    else:
        print(label, 'OK')

show('bad-escape', '"\\q"')
show('short-unicode-escape', '"\\u12"')
show('nonhex-unicode-escape', '"\\u12xz"')
show('raw-newline', '"line\nbreak"')
show('raw-tab', '"a\tb"')"#,
    });
}

#[test]
fn cpython_math_core_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py public pure-memory core subset",
        name: "math-core-values",
        source: r#"import math
print(round(math.pi, 3), round(math.e, 3), round(math.tau, 3))
print(math.isfinite(1.0), math.isfinite(math.inf), math.isinf(-math.inf), math.isnan(math.nan))
print(math.sqrt(9), math.gcd(12, 18), math.lcm(4, 6), math.factorial(5), math.isqrt(17))
print(math.comb(5, 2), math.perm(5, 2), math.prod([2, 3, 4]), math.isclose(1.0, 1.0 + 1e-10))
print(math.fabs(-3.5), math.trunc(3.9), math.floor(-1.2), math.ceil(-1.2))
for expr in [lambda: math.sqrt(-1), lambda: math.factorial(-1), lambda: math.gcd(1.2), lambda: math.isclose(1.0, 1.1, rel_tol=-1.0)]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_constants_and_classification_diff_subset() {
    let probe =
        run_cpython("import math; print(hasattr(math, 'isnormal'), hasattr(math, 'issubnormal'))")
            .expect("failed to probe CPython math classification support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True True\n" {
        eprintln!(
            "skipping math constants/classification diff: CPython oracle lacks math.isnormal/issubnormal"
        );
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests constants/classification public stable subset",
        name: "math-constants-classification",
        source: r#"import math
tiny = float('2.2250738585072014e-308') / 2
print(math.pi)
print(math.e)
print(math.tau == 2 * math.pi)
print(math.isfinite(0.0), math.isfinite(-0.0), math.isfinite(1.0), math.isfinite(-1.0))
print(math.isfinite(float('nan')), math.isfinite(float('inf')), math.isfinite(float('-inf')))
print(math.isnormal(1.25), math.isnormal(-1.0))
print(math.isnormal(0.0), math.isnormal(-0.0), math.isnormal(float('inf')), math.isnormal(float('-inf')), math.isnormal(float('nan')))
print(math.isnormal(tiny), math.isnormal(-tiny))
print(math.issubnormal(1.25), math.issubnormal(-1.0), math.issubnormal(0.0), math.issubnormal(-0.0))
print(math.issubnormal(float('inf')), math.issubnormal(float('-inf')), math.issubnormal(float('nan')))
print(math.issubnormal(tiny), math.issubnormal(-tiny))
print(math.isnan(float('nan')), math.isnan(float('-nan')), math.isnan(float('inf') * 0.0))
print(math.isnan(float('inf')), math.isnan(0.0), math.isnan(1.0))
print(math.isinf(float('inf')), math.isinf(float('-inf')), math.isinf(1e400), math.isinf(-1e400))
print(math.isinf(float('nan')), math.isinf(0.0), math.isinf(1.0))
print(math.isnan(math.nan), math.copysign(1.0, math.nan))
print(math.isinf(math.inf), math.inf > 0.0, math.inf == float('inf'), -math.inf == float('-inf'))
for expr in [
    lambda: math.isfinite(),
    lambda: math.isnan(),
    lambda: math.isinf(),
    lambda: math.isnormal(),
    lambda: math.issubnormal(),
    lambda: math.isfinite(1, 2),
    lambda: math.isnan('x'),
    lambda: math.isfinite(1+2j),
    lambda: math.isnormal(1, 2),
    lambda: math.issubnormal('x'),
    lambda: math.isnormal(1+2j),
    lambda: math.isinf(10**10000),
    lambda: math.issubnormal(10**10000),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_isclose_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::IsCloseTests public stable subset",
        name: "math-isclose",
        source: r#"import math

print(math.isclose(2.0, 2.0, rel_tol=0.0, abs_tol=0.0), math.isclose(12345, 12345.0, rel_tol=0.0, abs_tol=0.0), math.isclose(0.0, -0.0, rel_tol=0.0, abs_tol=0.0))
print(math.isclose(1.0000000001, 1.0), math.isclose(1.00000001, 1.0), math.isclose(1e8, 1e8 + 1, rel_tol=1e-8), math.isclose(1e8, 1e8 + 1, rel_tol=1e-9))
print(math.isclose(-1e-8, -1.000000009e-8, rel_tol=1e-8), math.isclose(1.12345678, 1.12345679, rel_tol=1e-8), math.isclose(1.12345678, 1.12345679, rel_tol=1e-9))
print(math.isclose(1e-9, 0.0, rel_tol=0.9), math.isclose(-1e-9, 0.0, rel_tol=0.9), math.isclose(1e-9, 0.0, abs_tol=1e-8), math.isclose(-1e-150, 0.0, abs_tol=1e-8))
print(math.isclose(math.inf, math.inf), math.isclose(-math.inf, -math.inf), math.isclose(math.nan, math.nan), math.isclose(math.inf, -math.inf), math.isclose(math.inf, 1.0), math.isclose(1e308, math.inf))
print(math.isclose(9, 10, rel_tol=0.1), math.isclose(10, 9, rel_tol=0.1), math.isclose(100000001, 100000000, rel_tol=1e-8), math.isclose(100000001, 100000000, rel_tol=1e-9))
print(math.isclose(a=1, b=1), math.isclose(1, b=1), math.isclose(1, 1, rel_tol=0.0, abs_tol=0.0), math.isclose(1, 2, rel_tol=1.0, abs_tol=0.0))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.isclose(FloatLike(1.0), FloatLike(1.0)), math.isclose(IndexLike(100000001), IndexLike(100000000), rel_tol=1e-8), math.isclose(True, 1), math.isclose(False, 0))
for expr in [
    lambda: math.isclose(),
    lambda: math.isclose(1),
    lambda: math.isclose(1, 1, 1e-9),
    lambda: math.isclose(1, 1, spam=1),
    lambda: math.isclose(b=1),
    lambda: math.isclose(1, a=1),
    lambda: math.isclose(1, 1, rel_tol=-1e-100),
    lambda: math.isclose(1, 1, abs_tol=-1e10),
    lambda: math.isclose('x', 1),
    lambda: math.isclose(1+2j, 1),
    lambda: math.isclose(RaisesFloat(), 1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_hypot_dist_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testHypot/testDist public stable subset",
        name: "math-hypot-dist",
        source: r#"import math
print(math.hypot(), math.hypot(3, 4), math.hypot(3, 4, 12))
print(math.hypot(0.75, -1), math.hypot(-1, 0.75), math.hypot(-10.5))
print(math.hypot(True, False, True, True, True))
print(math.copysign(1.0, math.hypot(-0.0)))
print(math.hypot(math.inf), math.hypot(math.nan, math.inf), math.hypot(-math.inf, -math.inf))
print(math.isnan(math.hypot(math.nan)), math.isnan(math.hypot(10, math.nan)))
print(math.hypot(1e308, 1e308) > 1e308, math.isinf(math.hypot(1e308, 1e308)))
scale = 2.2250738585072014e-308 / 2
print(math.hypot(4 * scale, 3 * scale) == 5 * scale)
print(math.dist((1.0, 2.0, 3.0), (4.0, 2.0, -1.0)))
print(math.dist([1, 2, 3], [4, 2, -1]), math.dist(iter([1, 2, 3]), iter([4, 2, -1])))
print(math.dist((), ()), math.dist((True, True, False, False, True, True), (True, False, True, False, False, False)))
print(math.copysign(1.0, math.dist((-0.0,), (0.0,))), math.copysign(1.0, math.dist((0.0,), (-0.0,))))
print(math.dist((1e308, 1e308), (0.0, 0.0)) > 1e308, math.isinf(math.dist((1e308, 1e308), (0.0, 0.0))))
print(math.dist((math.inf,), (-math.inf,)), math.isnan(math.dist((math.nan,), (math.inf,))), math.isnan(math.dist((10,), (math.nan,))))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.hypot(FloatLike(-1.0), 0.75), math.hypot(IndexLike(3), 4))
print(math.dist((FloatLike(14.0), 1), (2, -4)), math.dist((11, 1), (FloatLike(-1.0), -4)))
for expr in [
    lambda: math.hypot(x=1),
    lambda: math.hypot(1.1, 'string', 2.2),
    lambda: math.hypot(1, 10**10000),
    lambda: math.hypot(BadFloat()),
    lambda: math.hypot(RaisesFloat()),
    lambda: math.dist(),
    lambda: math.dist((1, 2)),
    lambda: math.dist((1,), (2,), (3,)),
    lambda: math.dist(p=(1,), q=(2,)),
    lambda: math.dist((1,), (1, 2)),
    lambda: math.dist('a', 'b'),
    lambda: math.dist((1, 'x'), (2, 3)),
    lambda: math.dist((1,), (10**10000,)),
    lambda: math.dist((BadFloat(),), (0,)),
    lambda: math.dist((RaisesFloat(),), (0,)),
    lambda: math.dist((1,), 2),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_gcd_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testGcd public stable subset",
        name: "math-gcd",
        source: r#"import math
gcd = math.gcd
print(gcd(0, 0), gcd(1, 0), gcd(-1, 0), gcd(0, 1), gcd(0, -1))
print(gcd(7, 1), gcd(7, -1), gcd(-23, 15), gcd(120, 84), gcd(84, -120))
print(gcd(1216342683557601535506311712, 436522681849110124616458784))
print(gcd(), gcd(120), gcd(-120), gcd(120, 84, 102), gcd(120, 1, 84))
class MyIndexable:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadIndex:
    def __index__(self):
        return 1.5
class Boom:
    def __index__(self):
        raise RuntimeError('boom')
print(gcd(MyIndexable(120), MyIndexable(84)))
print(gcd(True, False), gcd(False, False))
for expr in [
    lambda: gcd(120.0),
    lambda: gcd(120.0, 84),
    lambda: gcd(120, 84.0),
    lambda: gcd(120, 1, 84.0),
    lambda: gcd(1, 1.5),
    lambda: gcd(BadIndex()),
    lambda: gcd(Boom()),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_lcm_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::test_lcm public stable subset",
        name: "math-lcm",
        source: r#"import math
lcm = math.lcm
print(lcm(0, 0), lcm(1, 0), lcm(-1, 0), lcm(0, 1), lcm(0, -1))
print(lcm(7, 1), lcm(7, -1), lcm(-23, 15), lcm(120, 84), lcm(84, -120))
print(lcm(1216342683557601535506311712, 436522681849110124616458784))
print(lcm(), lcm(120), lcm(-120), lcm(120, 84, 102), lcm(120, 0, 84))
class MyIndexable:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadIndex:
    def __index__(self):
        return 1.5
class Boom:
    def __index__(self):
        raise RuntimeError('boom')
print(lcm(MyIndexable(120), MyIndexable(84)))
print(lcm(True, False), lcm(True, True), lcm(False, False))
for expr in [
    lambda: lcm(120.0),
    lambda: lcm(120.0, 84),
    lambda: lcm(120, 84.0),
    lambda: lcm(120, 0, 84.0),
    lambda: lcm(1, 1.5),
    lambda: lcm(0, 0.5),
    lambda: lcm(BadIndex()),
    lambda: lcm(0, Boom()),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_prod_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::test_prod public stable subset",
        name: "math-prod",
        source: r#"import math
prod = math.prod
print(prod([]), prod([], start=5), prod(list(range(2, 8))))
print(prod(iter(list(range(2, 8)))), prod(range(1, 10), start=10))
print(prod([1, 2, 3, 4, 5]), prod([1.0, 2.0, 3.0, 4.0, 5.0]))
print(prod([1, 2, 3, 4.0, 5.0]), prod([1.0, 2.0, 3.0, 4, 5]))
print(prod([1, 1, 2**32, 1, 1]), prod([1.0, 1.0, 2**32, 1, 1]))
print(prod([2, 3], start='ab'))
print(prod([2, 3], start=[1, 2]))
print(prod([], start={2: 3}))
print(prod([0, 1, 2, 3]), prod([1, 0, 2, 3]), prod([1, 2, 3, 0]))
print(math.isnan(prod([1, 2, 3, float('nan'), 2, 3])))
print(math.isinf(prod([1, 2, 3, float('inf'), -3, 4])), prod([1, 2, 3, float('inf'), -3, 4]) < 0)
print(type(prod([1, 2, 3, 4, 5, 6])).__name__, type(prod([1, 2.0, 3, 4, 5, 6])).__name__)
values = [bytearray(b'a'), bytearray(b'b')]
for expr in [
    lambda: prod(),
    lambda: prod(42),
    lambda: prod(['a', 'b', 'c']),
    lambda: prod(['a', 'b', 'c'], start=''),
    lambda: prod([b'a', b'c'], start=b''),
    lambda: prod(values, start=bytearray(b'')),
    lambda: prod([[1], [2], [3]]),
    lambda: prod([{2: 3}]),
    lambda: prod([10, 20], 1),
    lambda: prod([1], missing=2),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_integer_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math_integer.py public stable subset",
        name: "math-integer",
        source: r#"import math
print(math.factorial(0), math.factorial(1), math.factorial(5), math.factorial(20))

total = 1
ok = True
for i in range(1, 20):
    total *= i
    ok = ok and math.factorial(i) == total
print(ok)

class IntSubclass(int):
    pass
class MyIndexable:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value

print(math.factorial(False), math.factorial(True), math.factorial(IntSubclass(5)), math.factorial(MyIndexable(6)), type(math.factorial(IntSubclass(5))).__name__)
values = [0, 1, 2, 3, 4, 15, 16, 17, 10**20, 3**40]
ok = True
for value in values:
    root = math.isqrt(value)
    ok = ok and root * root <= value and value < (root + 1) * (root + 1) and type(root).__name__ == 'int'
print(ok)
print(math.isqrt(True), math.isqrt(False), math.isqrt(MyIndexable(1729)), type(math.isqrt(MyIndexable(1729))).__name__)

ok = True
for n in range(10):
    for k in range(n + 1):
        ok = ok and math.comb(n, k) == math.factorial(n) // (math.factorial(k) * math.factorial(n - k))
        ok = ok and math.perm(n, k) == math.factorial(n) // math.factorial(n - k)
print(ok)
print(math.comb(5, 0), math.comb(5, 1), math.comb(5, 2), math.comb(5, 4), math.comb(5, 5), math.comb(1, 2))
print(math.perm(5, 0), math.perm(5, 1), math.perm(5, 2), math.perm(5, 5), math.perm(5), math.perm(5, None), math.perm(1, 2))
n = 2**40
print(math.comb(n, 0), math.comb(n, 1), math.comb(n, 2))
print(math.perm(n, 0), math.perm(n, 1), math.perm(n, 2))
print(math.comb(True, False), math.comb(IntSubclass(5), IntSubclass(2)), math.comb(MyIndexable(5), MyIndexable(2)), type(math.comb(MyIndexable(5), MyIndexable(2))).__name__)
print(math.perm(True, False), math.perm(IntSubclass(5), IntSubclass(2)), math.perm(MyIndexable(5), MyIndexable(2)), type(math.perm(MyIndexable(5), MyIndexable(2))).__name__)

for expr in [
    lambda: math.factorial(),
    lambda: math.factorial(1, 2),
    lambda: math.factorial(-1),
    lambda: math.factorial(n=5),
    lambda: math.isqrt(),
    lambda: math.isqrt(1, 2),
    lambda: math.isqrt(3.5),
    lambda: math.isqrt(3+0j),
    lambda: math.isqrt(-1),
    lambda: math.comb(),
    lambda: math.comb(1),
    lambda: math.comb(1, 2, 3),
    lambda: math.comb(10.0, 1),
    lambda: math.comb(10, 1.0),
    lambda: math.comb(-1, 1),
    lambda: math.comb(1, -1),
    lambda: math.comb(n=1, k=1),
    lambda: math.perm(),
    lambda: math.perm(1, 2, 3),
    lambda: math.perm(10.0, 1),
    lambda: math.perm(10, 1.0),
    lambda: math.perm(-1, 1),
    lambda: math.perm(1, -1),
    lambda: math.perm(n=1, k=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_sqrt_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testSqrt public stable subset",
        name: "math-sqrt",
        source: r#"import math
print(math.sqrt(0), math.sqrt(0.0))
print(math.sqrt(2.5))
print(math.sqrt(0.25), math.sqrt(25.25))
print(math.sqrt(1), math.sqrt(4))
print(math.sqrt(math.inf) == math.inf)
print(math.isnan(math.sqrt(math.nan)))
print(type(math.sqrt(4)).__name__)
for expr in [
    lambda: math.sqrt(),
    lambda: math.sqrt(1, 2),
    lambda: math.sqrt('x'),
    lambda: math.sqrt(1+2j),
    lambda: math.sqrt(-1),
    lambda: math.sqrt(float('-inf')),
    lambda: math.sqrt(10**10000),
    lambda: math.sqrt(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_fabs_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testFabs public stable subset",
        name: "math-fabs",
        source: r#"import math
print(math.fabs(-1), math.fabs(0), math.fabs(1))
print(math.fabs(-3.5), math.fabs(3.5), math.fabs(True), math.fabs(False))
print(math.copysign(1.0, math.fabs(-0.0)))
print(math.isinf(math.fabs(float('-inf'))), math.fabs(float('-inf')) > 0)
print(math.isnan(math.fabs(float('nan'))))
print(type(math.fabs(1)).__name__, type(math.fabs(1.0)).__name__)
for expr in [
    lambda: math.fabs(),
    lambda: math.fabs(1, 2),
    lambda: math.fabs('x'),
    lambda: math.fabs(1+2j),
    lambda: math.fabs(10**10000),
    lambda: math.fabs(value=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_copysign_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testCopysign public stable subset",
        name: "math-copysign",
        source: r#"import math
INF = float('inf')
NINF = float('-inf')
NAN = float('nan')
print(math.copysign(1, 42), math.copysign(0.0, 42))
print(math.copysign(1.0, -42), math.copysign(3, 0.0), math.copysign(4.0, -0.0))
print(math.copysign(1.0, 0.0), math.copysign(1.0, -0.0))
print(math.copysign(INF, 0.0), math.copysign(INF, -0.0))
print(math.copysign(NINF, 0.0), math.copysign(NINF, -0.0))
print(math.copysign(1.0, INF), math.copysign(1.0, NINF))
print(math.copysign(INF, INF), math.copysign(INF, NINF))
print(math.copysign(NINF, INF), math.copysign(NINF, NINF))
print(math.isnan(math.copysign(NAN, 1.0)))
print(math.isnan(math.copysign(NAN, INF)), math.isnan(math.copysign(NAN, NINF)), math.isnan(math.copysign(NAN, NAN)))
print(math.isinf(math.copysign(INF, NAN)), math.fabs(math.copysign(2.0, NAN)))
for expr in [
    lambda: math.copysign(),
    lambda: math.copysign(1),
    lambda: math.copysign(1, 2, 3),
    lambda: math.copysign('x', 1),
    lambda: math.copysign(1, 'x'),
    lambda: math.copysign(10**10000, 1),
    lambda: math.copysign(1, 10**10000),
    lambda: math.copysign(x=1, y=2),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_signbit_diff_subset() {
    let probe = run_cpython("import math; print(hasattr(math, 'signbit'))")
        .expect("failed to probe CPython math.signbit support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True\n" {
        eprintln!("skipping math.signbit diff: CPython oracle lacks math.signbit");
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::test_signbit public stable subset",
        name: "math-signbit",
        source: r#"import math
INF = float('inf')
NAN = float('nan')
print(type(math.signbit(1.0)).__name__)
for arg in [0.0, 1.0, INF, NAN]:
    print(math.signbit(arg), math.signbit(-arg))
print(math.signbit(False), math.signbit(True))
print(math.signbit(-0), math.signbit(-1))
for expr in [
    lambda: math.signbit(),
    lambda: math.signbit(1, 2),
    lambda: math.signbit('1.0'),
    lambda: math.signbit(1+2j),
    lambda: math.signbit(10**10000),
    lambda: math.signbit(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_trunc_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::test_trunc public stable subset",
        name: "math-trunc",
        source: r#"import math
print(math.trunc(1), math.trunc(-1))
print(type(math.trunc(1)).__name__, type(math.trunc(1.5)).__name__)
print(math.trunc(1.5), math.trunc(-1.5))
print(math.trunc(1.999999), math.trunc(-1.999999))
print(math.trunc(-0.999999), math.trunc(-100.999))
print(math.trunc(False), math.trunc(True), math.trunc(10**30))
print(math.trunc(1e20) == 100000000000000000000)

class TestTrunc:
    def __trunc__(self):
        return 23
class TestRaises:
    def __trunc__(self):
        raise ValueError('bad trunc')
class FloatTruncResult:
    def __trunc__(self):
        return 23.5
class TestNoTrunc:
    pass

print(math.trunc(TestTrunc()))
print(type(math.trunc(FloatTruncResult())).__name__)
for expr in [
    lambda: math.trunc(),
    lambda: math.trunc(1, 2),
    lambda: math.trunc('1.0'),
    lambda: math.trunc(1+2j),
    lambda: math.trunc(float('nan')),
    lambda: math.trunc(float('inf')),
    lambda: math.trunc(TestNoTrunc()),
    lambda: math.trunc(TestRaises()),
    lambda: math.trunc(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_ceil_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testCeil public stable subset",
        name: "math-ceil",
        source: r#"import math
print(type(math.ceil(0.5)).__name__)
print(math.ceil(0.5), math.ceil(1.0), math.ceil(1.5))
print(math.ceil(-0.5), math.ceil(-1.0), math.ceil(-1.5))
print(math.ceil(0.0), math.ceil(-0.0))
print(math.ceil(False), math.ceil(True), math.ceil(10**30) == 10**30)
print(math.ceil(1e20) == 100000000000000000000)

class TestCeil:
    def __ceil__(self):
        return 42
class FloatCeilResult:
    def __ceil__(self):
        return 42.5
class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class TestNoCeil:
    pass

print(math.ceil(TestCeil()), type(math.ceil(FloatCeilResult())).__name__)
print(math.ceil(FloatLike(42.5)), math.ceil(FloatLike(+1.0)), math.ceil(FloatLike(-1.0)))
print(math.ceil(IndexLike(7)))
for expr in [
    lambda: math.ceil(),
    lambda: math.ceil(1, 2),
    lambda: math.ceil('1.0'),
    lambda: math.ceil(1+2j),
    lambda: math.ceil(float('nan')),
    lambda: math.ceil(float('inf')),
    lambda: math.ceil(IndexLike(10**10000)),
    lambda: math.ceil(TestNoCeil()),
    lambda: math.ceil(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_floor_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testFloor public stable subset",
        name: "math-floor",
        source: r#"import math
print(type(math.floor(0.5)).__name__)
print(math.floor(0.5), math.floor(1.0), math.floor(1.5))
print(math.floor(-0.5), math.floor(-1.0), math.floor(-1.5))
print(math.floor(0.0), math.floor(-0.0))
print(math.floor(False), math.floor(True), math.floor(-10**30) == -10**30)
print(math.floor(-1e20) == -100000000000000000000)

class TestFloor:
    def __floor__(self):
        return 42
class FloatFloorResult:
    def __floor__(self):
        return 41.5
class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class TestNoFloor:
    pass

print(math.floor(TestFloor()), type(math.floor(FloatFloorResult())).__name__)
print(math.floor(FloatLike(41.9)), math.floor(FloatLike(+1.0)), math.floor(FloatLike(-1.0)))
print(math.floor(IndexLike(7)))
for expr in [
    lambda: math.floor(),
    lambda: math.floor(1, 2),
    lambda: math.floor('1.0'),
    lambda: math.floor(1+2j),
    lambda: math.floor(float('nan')),
    lambda: math.floor(float('-inf')),
    lambda: math.floor(IndexLike(10**10000)),
    lambda: math.floor(TestNoFloor()),
    lambda: math.floor(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_degrees_radians_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testDegrees and testRadians public stable subset",
        name: "math-degrees-radians",
        source: r#"import math
print(type(math.degrees(0)).__name__, type(math.radians(0)).__name__)
print(math.degrees(math.pi), math.degrees(math.pi / 2), math.degrees(-math.pi / 4), math.degrees(0))
print(math.radians(180) == math.pi, math.radians(90) == math.pi / 2, math.radians(-45) == -math.pi / 4, math.radians(0))
print(math.isinf(math.degrees(math.inf)), math.degrees(-math.inf) < 0, math.isnan(math.degrees(math.nan)))
print(math.isinf(math.radians(math.inf)), math.radians(-math.inf) < 0, math.isnan(math.radians(math.nan)))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.degrees(FloatLike(math.pi)), math.radians(FloatLike(180.0)) == math.pi)
print(round(math.degrees(IndexLike(1)), 6), math.radians(IndexLike(180)) == math.pi)
for expr in [
    lambda: math.degrees(),
    lambda: math.radians(),
    lambda: math.degrees(1, 2),
    lambda: math.radians(1, 2),
    lambda: math.degrees('1.0'),
    lambda: math.radians(1+2j),
    lambda: math.degrees(IndexLike(10**10000)),
    lambda: math.radians(BadFloat()),
    lambda: math.degrees(RaisesFloat()),
    lambda: math.radians(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_cbrt_diff_subset() {
    let probe = run_cpython("import math; print(hasattr(math, 'cbrt'))")
        .expect("failed to probe CPython math.cbrt support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True\n" {
        eprintln!("skipping math.cbrt diff: CPython oracle lacks math.cbrt");
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testCbrt public stable subset",
        name: "math-cbrt",
        source: r#"import math
print(type(math.cbrt(0)).__name__)
print(math.cbrt(0), math.cbrt(1), math.cbrt(8))
print(math.copysign(1.0, math.cbrt(0.0)), math.copysign(1.0, math.cbrt(-0.0)))
print(round(math.cbrt(1.2), 12), round(math.cbrt(-2.6), 12))
print(math.cbrt(27), math.cbrt(-1), math.cbrt(-27))
print(math.cbrt(math.inf), math.cbrt(-math.inf), math.isnan(math.cbrt(math.nan)))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.cbrt(FloatLike(8.0)), math.cbrt(IndexLike(27)))
for expr in [
    lambda: math.cbrt(),
    lambda: math.cbrt(1, 2),
    lambda: math.cbrt('1.0'),
    lambda: math.cbrt(1+2j),
    lambda: math.cbrt(IndexLike(10**10000)),
    lambda: math.cbrt(BadFloat()),
    lambda: math.cbrt(RaisesFloat()),
    lambda: math.cbrt(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_fma_diff_subset() {
    let probe = run_cpython("import math; print(hasattr(math, 'fma'))")
        .expect("failed to probe CPython math.fma support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True\n" {
        eprintln!("skipping math.fma diff: CPython oracle lacks math.fma");
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::FMATests public stable subset",
        name: "math-fma",
        source: r#"import math
print(math.fma(2.0, 3.0, 4.0), math.fma(-2.0, 3.0, 4.0), math.fma(2, 3, 4))
a = 2.0 ** -50
print(math.fma(a - 1.0, a + 1.0, 1.0) == a * a, math.fma(2.0 ** 512, 2.0 ** 512, -(2.0 ** 1023)) == 2.0 ** 1023)
print(math.copysign(1.0, math.fma(2.0, 2.0, -4.0)), math.copysign(1.0, math.fma(0.0, -2.3, -0.0)), math.copysign(1.0, math.fma(1e-300, -1e-300, 0.0)))
print(math.isnan(math.fma(math.nan, 2.0, 3.0)), math.isnan(math.fma(2.0, math.nan, 3.0)), math.isnan(math.fma(2.0, 3.0, math.nan)), math.isnan(math.fma(0.0, math.inf, math.nan)))
print(math.fma(math.inf, 2.0, 3.0), math.fma(-math.inf, 2.0, 3.0), math.fma(2.0, math.inf, math.inf), math.fma(2.0, -math.inf, -math.inf))
print(math.fma(2.0, 3.0, math.inf), math.fma(2.0, 3.0, -math.inf))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.fma(FloatLike(2.0), FloatLike(3.0), FloatLike(4.0)), math.fma(IndexLike(2), IndexLike(3), IndexLike(4)), math.fma(True, False, True))
for expr in [
    lambda: math.fma(),
    lambda: math.fma(1),
    lambda: math.fma(1, 2),
    lambda: math.fma(1, 2, 3, 4),
    lambda: math.fma(x=1, y=2, z=3),
    lambda: math.fma('x', 1, 1),
    lambda: math.fma(1+2j, 1, 1),
    lambda: math.fma(10**10000, 1, 1),
    lambda: math.fma(math.inf, 0.0, 1.0),
    lambda: math.fma(0.0, -math.inf, 0.0),
    lambda: math.fma(math.inf, 2.0, -math.inf),
    lambda: math.fma(1e308, 1e308, 0.0),
    lambda: math.fma(BadFloat(), 1, 1),
    lambda: math.fma(RaisesFloat(), 1, 1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_fmax_fmin_diff_subset() {
    let probe = run_cpython("import math; print(hasattr(math, 'fmax'), hasattr(math, 'fmin'))")
        .expect("failed to probe CPython math.fmax/fmin support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True True\n" {
        eprintln!("skipping math.fmax/fmin diff: CPython oracle lacks math.fmax/fmin");
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::test_fmax/test_fmin public stable subset",
        name: "math-fmax-fmin",
        source: r#"import math
print(math.fmax(0., 0.), math.fmax(1., 2.), math.fmax(2., 1.))
print(math.fmax(+1., +0.) == 1., math.fmax(+0., +1.) == 1., math.fmax(+1., -0.) == 1., math.fmax(-0., +1.) == 1.)
print(math.fmax(-1., +0.) == 0., math.fmax(+0., -1.) == 0., math.fmax(-1., -0.) == 0., math.fmax(-0., -1.) == 0.)
print(math.fmax(math.inf, -1.), math.fmax(-1., math.inf), math.fmax(-math.inf, -1.), math.fmax(-1., -math.inf))
print(math.isnan(math.fmax(math.nan, 1.)), math.fmax(math.nan, 1.), math.isnan(math.fmax(1., math.nan)), math.fmax(1., math.nan), math.isnan(math.fmax(math.nan, math.nan)))
print(math.fmin(0., 0.), math.fmin(1., 2.), math.fmin(2., 1.))
print(math.fmin(+1., +0.) == 0., math.fmin(+0., +1.) == 0., math.fmin(+1., -0.) == 0., math.fmin(-0., +1.) == 0.)
print(math.fmin(-1., +0.) == -1., math.fmin(+0., -1.) == -1., math.fmin(-1., -0.) == -1., math.fmin(-0., -1.) == -1.)
print(math.fmin(math.inf, -1.), math.fmin(-1., math.inf), math.fmin(-math.inf, -1.), math.fmin(-1., -math.inf))
print(math.isnan(math.fmin(math.nan, 1.)), math.fmin(math.nan, 1.), math.isnan(math.fmin(1., math.nan)), math.fmin(1., math.nan), math.isnan(math.fmin(math.nan, math.nan)))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.fmax(FloatLike(1.25), FloatLike(2.5)), math.fmin(IndexLike(-1), IndexLike(3)), math.fmax(True, False), math.fmin(True, False))
for expr in [
    lambda: math.fmax(),
    lambda: math.fmin(1),
    lambda: math.fmax(1, 2, 3),
    lambda: math.fmax(x=1, y=2),
    lambda: math.fmin('x', 1),
    lambda: math.fmax(1+2j, 1),
    lambda: math.fmin(IndexLike(10**10000), 1),
    lambda: math.fmax(BadFloat(), 1),
    lambda: math.fmin(RaisesFloat(), 1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_exp_exp2_diff_subset() {
    let probe = run_cpython("import math; print(hasattr(math, 'exp2'))")
        .expect("failed to probe CPython math.exp2 support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True\n" {
        eprintln!("skipping math.exp/exp2 diff: CPython oracle lacks math.exp2");
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testExp and testExp2 public stable subset",
        name: "math-exp-exp2",
        source: r#"import math
print(type(math.exp(0)).__name__, type(math.exp2(0)).__name__)
print(round(math.exp(-1), 12), math.exp(0), math.exp(1) == math.e)
print(math.exp(math.inf), math.exp(-math.inf), math.isnan(math.exp(math.nan)))
print(math.exp2(-1), math.exp2(0), math.exp2(1), round(math.exp2(2.3), 12))
print(math.exp2(math.inf), math.exp2(-math.inf), math.isnan(math.exp2(math.nan)))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.exp(FloatLike(1.0)) == math.e, math.exp2(FloatLike(2.0)))
print(math.exp(IndexLike(1)) == math.e, math.exp2(IndexLike(2)))
for expr in [
    lambda: math.exp(),
    lambda: math.exp2(),
    lambda: math.exp(1, 2),
    lambda: math.exp2(1, 2),
    lambda: math.exp('1.0'),
    lambda: math.exp2(1+2j),
    lambda: math.exp(1000000),
    lambda: math.exp2(1000000),
    lambda: math.exp(IndexLike(10**10000)),
    lambda: math.exp2(BadFloat()),
    lambda: math.exp(RaisesFloat()),
    lambda: math.exp2(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_log_family_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testLog/log1p/log2/log10 public stable subset",
        name: "math-log-family",
        source: r#"import math
print(type(math.log(1)).__name__, type(math.log1p(0)).__name__, type(math.log2(1)).__name__, type(math.log10(1)).__name__)
print(round(math.log(1 / math.e), 12), math.log(1), math.log(math.e))
print(math.log(32, 2), round(math.log(10**40, 10), 12), round(math.log(10**2000, 10**1000), 12))
print(round(math.log(10**1000), 12), math.log2(2**2000), math.log10(10**1000))
print(round(math.log1p(2), 12), round(math.log1p(2**90), 12) == round(math.log1p(float(2**90)), 12), round(math.log1p(2**300), 12) == round(math.log1p(float(2**300)), 12))
print(math.log(math.inf), math.log2(math.inf), math.log10(math.inf), math.log1p(math.inf))
print(math.isnan(math.log(math.nan)), math.isnan(math.log2(math.nan)), math.isnan(math.log10(math.nan)), math.isnan(math.log1p(math.nan)))

class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1

print(math.log(IndexLike(32), IndexLike(2)), math.log2(IndexLike(4)), math.log10(IndexLike(10)))
print(math.log(FloatLike(math.e)), math.log2(FloatLike(8.0)), math.log10(FloatLike(100.0)))
for expr in [
    lambda: math.log(),
    lambda: math.log(1, 2, 3),
    lambda: math.log(0),
    lambda: math.log(-1),
    lambda: math.log(10, -10),
    lambda: math.log(-math.inf),
    lambda: math.log(10, 1),
    lambda: math.log('1'),
    lambda: math.log(BadFloat()),
    lambda: math.log1p(),
    lambda: math.log1p(-1),
    lambda: math.log1p(-math.inf),
    lambda: math.log2(0),
    lambda: math.log2(-1),
    lambda: math.log10(0),
    lambda: math.log10(-1),
    lambda: math.log2(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_trig_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testAcos/Asin/Atan/Atan2/Cos/Sin/Tan public stable subset",
        name: "math-trig",
        source: r#"import math
print(round(math.acos(-1), 12) == round(math.pi, 12), round(math.acos(0), 12) == round(math.pi / 2, 12), math.acos(1))
print(round(math.asin(-1), 12) == round(-math.pi / 2, 12), math.asin(0), round(math.asin(1), 12) == round(math.pi / 2, 12))
print(round(math.atan(-1), 12) == round(-math.pi / 4, 12), math.atan(0), round(math.atan(1), 12) == round(math.pi / 4, 12))
print(round(math.atan(math.inf), 12) == round(math.pi / 2, 12), round(math.atan(-math.inf), 12) == round(-math.pi / 2, 12))
print(round(math.atan2(-1, 0), 12) == round(-math.pi / 2, 12), round(math.atan2(1, -1), 12) == round(3 * math.pi / 4, 12))
print(math.atan2(0.0, -0.0) == math.pi, math.copysign(1.0, math.atan2(-0.0, 0.0)))
print(round(math.cos(0), 12), round(math.cos(math.pi), 12), round(math.sin(math.pi / 2), 12), round(math.sin(-math.pi / 2), 12))
print(round(math.tan(math.pi / 4), 12), round(math.tan(-math.pi / 4), 12), math.tan(0))
print(math.isnan(math.acos(math.nan)), math.isnan(math.asin(math.nan)), math.isnan(math.atan(math.nan)), math.isnan(math.atan2(math.nan, 1)))
print(math.isnan(math.cos(math.nan)), math.isnan(math.sin(math.nan)), math.isnan(math.tan(math.nan)))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.acos(FloatLike(1.0)), round(math.sin(IndexLike(1)), 12), round(math.atan2(IndexLike(1), IndexLike(1)), 12) == round(math.pi / 4, 12))
for expr in [
    lambda: math.acos(),
    lambda: math.asin(1, 2),
    lambda: math.atan2(1),
    lambda: math.atan2(1, 2, 3),
    lambda: math.acos(1.1),
    lambda: math.asin(-1.1),
    lambda: math.cos(math.inf),
    lambda: math.sin(-math.inf),
    lambda: math.tan(math.inf),
    lambda: math.acos('x'),
    lambda: math.sin(1+2j),
    lambda: math.tan(IndexLike(10**10000)),
    lambda: math.cos(BadFloat()),
    lambda: math.atan(RaisesFloat()),
    lambda: math.atan2(x=1, y=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_hyperbolic_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testAcosh/Asinh/Atanh/Cosh/Sinh/Tanh public stable subset",
        name: "math-hyperbolic",
        source: r#"import math
print(math.acosh(1), round(math.acosh(2), 12), math.acosh(math.inf))
print(math.asinh(0), round(math.asinh(1), 12), round(math.asinh(-1), 12), math.asinh(math.inf), math.asinh(-math.inf))
print(math.atanh(0), round(math.atanh(0.5), 12), round(math.atanh(-0.5), 12))
print(math.cosh(0), round(math.cosh(2) - 2 * math.cosh(1) ** 2, 12), math.cosh(math.inf), math.cosh(-math.inf))
print(math.sinh(0), round(math.sinh(1) ** 2 - math.cosh(1) ** 2, 12), round(math.sinh(1) + math.sinh(-1), 12), math.sinh(math.inf), math.sinh(-math.inf))
print(math.tanh(0), round(math.tanh(1) + math.tanh(-1), 12), math.tanh(math.inf), math.tanh(-math.inf))
print(math.copysign(1.0, math.tanh(-0.0)), math.isnan(math.acosh(math.nan)), math.isnan(math.asinh(math.nan)), math.isnan(math.atanh(math.nan)), math.isnan(math.cosh(math.nan)), math.isnan(math.sinh(math.nan)), math.isnan(math.tanh(math.nan)))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.acosh(FloatLike(1.0)), round(math.sinh(IndexLike(1)), 12), math.tanh(IndexLike(0)))
for expr in [
    lambda: math.acosh(),
    lambda: math.asinh(1, 2),
    lambda: math.acosh(0),
    lambda: math.acosh(-math.inf),
    lambda: math.atanh(1),
    lambda: math.atanh(-1),
    lambda: math.atanh(math.inf),
    lambda: math.cosh(1000000),
    lambda: math.sinh(1000000),
    lambda: math.cosh('x'),
    lambda: math.sinh(1+2j),
    lambda: math.tanh(IndexLike(10**10000)),
    lambda: math.cosh(BadFloat()),
    lambda: math.asinh(RaisesFloat()),
    lambda: math.tanh(x=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_fmod_remainder_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testFmod/testRemainder public stable subset",
        name: "math-fmod-remainder",
        source: r#"import math
print(math.fmod(10, 1), math.fmod(10, 0.5), math.fmod(10, 1.5))
print(math.fmod(-10, 1), math.copysign(1.0, math.fmod(-10, 1)), math.fmod(-10, 1.5))
print(math.fmod(10, -1.5), math.fmod(-10, -1.5))
print(math.fmod(3.0, math.inf), math.fmod(-3.0, math.inf), math.fmod(3.0, -math.inf), math.fmod(0.0, -math.inf))
print(math.isnan(math.fmod(math.nan, 1.0)), math.isnan(math.fmod(1.0, math.nan)), math.isnan(math.fmod(math.nan, math.nan)))
print(math.remainder(10, 1), math.remainder(10, 0.5), math.remainder(10, 1.5))
print(math.remainder(10, 3), math.remainder(-10, 3), math.remainder(7, 2))
print(math.remainder(6, 4), math.copysign(1.0, math.remainder(6, 4)), math.remainder(6, -4), math.copysign(1.0, math.remainder(6, -4)))
print(math.remainder(-4.0, 1.0), math.copysign(1.0, math.remainder(-4.0, 1.0)))
print(math.remainder(2.3, math.inf), math.remainder(-2.3, -math.inf), math.remainder(0.0, math.inf))
print(math.isnan(math.remainder(math.nan, 1.0)), math.isnan(math.remainder(1.0, math.nan)))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.fmod(FloatLike(10.0), FloatLike(1.5)), math.remainder(IndexLike(10), IndexLike(3)))
for expr in [
    lambda: math.fmod(),
    lambda: math.remainder(1),
    lambda: math.fmod(1.0, 0.0),
    lambda: math.fmod(math.inf, 1.0),
    lambda: math.fmod(math.inf, math.inf),
    lambda: math.remainder(1.0, 0.0),
    lambda: math.remainder(math.inf, 1.0),
    lambda: math.remainder(-math.inf, -0.0),
    lambda: math.fmod('x', 1),
    lambda: math.remainder(1+2j, 1),
    lambda: math.fmod(IndexLike(10**10000), 1),
    lambda: math.fmod(BadFloat(), 1),
    lambda: math.remainder(RaisesFloat(), 1),
    lambda: math.fmod(x=1, y=2),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_frexp_ldexp_modf_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testFrexp/testLdexp/testLdexp_denormal/testModf public stable subset",
        name: "math-frexp-ldexp-modf",
        source: r#"import math
print(math.frexp(-1), math.frexp(0), math.frexp(1), math.frexp(2))
m, e = math.frexp(-0.0)
print(m, e, math.copysign(1.0, m))
print(math.frexp(math.inf)[0], math.frexp(-math.inf)[0], math.isnan(math.frexp(math.nan)[0]), math.frexp(math.inf)[1], math.frexp(math.nan)[1])
print(math.ldexp(0, 1), math.ldexp(1, 1), math.ldexp(1, -1), math.ldexp(-1, 1))
print(math.ldexp(1.0, -1000000), math.ldexp(-1.0, -1000000), math.copysign(1.0, math.ldexp(-1.0, -1000000)))
print(math.ldexp(math.inf, 30), math.ldexp(-math.inf, -213), math.isnan(math.ldexp(math.nan, 0)))
print(math.ldexp(6993274598585239, -1126))
print(math.ldexp(1.5, True), math.ldexp(1.5, False))
print(math.modf(1.5), math.modf(-1.5))
part, whole = math.modf(-1.0)
print(part, whole, math.copysign(1.0, part))
part, whole = math.modf(-0.0)
print(part, whole, math.copysign(1.0, part), math.copysign(1.0, whole))
print(math.modf(math.inf), math.modf(-math.inf), math.copysign(1.0, math.modf(-math.inf)[0]))
print(math.isnan(math.modf(math.nan)[0]), math.isnan(math.modf(math.nan)[1]))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')
class RaisesIndex:
    def __index__(self):
        raise ValueError('bad index')

print(math.frexp(FloatLike(8.0)), math.modf(IndexLike(3)))
print(math.ldexp(FloatLike(1.5), 2), math.ldexp(IndexLike(3), 2))
for expr in [
    lambda: math.frexp(),
    lambda: math.frexp(1, 2),
    lambda: math.frexp('x'),
    lambda: math.frexp(1+2j),
    lambda: math.frexp(IndexLike(10**10000)),
    lambda: math.frexp(BadFloat()),
    lambda: math.frexp(RaisesFloat()),
    lambda: math.frexp(x=1),
    lambda: math.modf(),
    lambda: math.modf(1, 2),
    lambda: math.modf('x'),
    lambda: math.modf(1+2j),
    lambda: math.modf(IndexLike(10**10000)),
    lambda: math.modf(BadFloat()),
    lambda: math.modf(RaisesFloat()),
    lambda: math.modf(x=1),
    lambda: math.ldexp(),
    lambda: math.ldexp(1),
    lambda: math.ldexp(1, 2, 3),
    lambda: math.ldexp(2.0, 1.1),
    lambda: math.ldexp(1.0, IndexLike(2)),
    lambda: math.ldexp(1.0, RaisesIndex()),
    lambda: math.ldexp(1.0, 1000000),
    lambda: math.ldexp(-1.0, 1000000),
    lambda: math.ldexp(IndexLike(10**10000), 1),
    lambda: math.ldexp(1.0, 10**10000),
    lambda: math.ldexp(x=1.0, i=1),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_fsum_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testFsum public stable subset",
        name: "math-fsum",
        source: r#"import math
print(math.fsum([]), math.fsum([0.0]), math.fsum([1, 2, 3]))
print(math.fsum([0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1]))
print(math.fsum([1e100, 1.0, -1e100]))
print(math.fsum([1e100, 1.0, -1e100, 1e-100, 1e50, -1.0, -1e50]))
print(math.fsum([2.0**53, -0.5, -2.0**-54]))
print(math.fsum([2.0**53, 1.0, 2.0**-100]))
print(math.fsum([2.0**53 + 10.0, 1.0, 2.0**-100]))
print(math.fsum([2.0**53 - 4.0, 0.5, 2.0**-54]))
print(math.fsum([1e16, 1.0, 1e-16]))
print(math.fsum([1e16 - 2.0, 1.0 - 2.0**-53, -(1e16 - 2.0), -(1.0 - 2.0**-53)]))
print(math.copysign(1.0, math.fsum([-0.0])), math.copysign(1.0, math.fsum([0.0, -0.0])))
print(math.fsum([1.0, math.inf]), math.fsum([1.0, -math.inf]), math.isnan(math.fsum([math.nan, 1.0])), math.isnan(math.fsum([math.inf, math.nan])))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')
def bad_iter():
    yield 1.0
    1 / 0

print(math.fsum([FloatLike(1.25), FloatLike(2.75)]), math.fsum([IndexLike(2), IndexLike(3)]), math.fsum([True, False, True]))
for expr in [
    lambda: math.fsum(),
    lambda: math.fsum([], []),
    lambda: math.fsum(iterable=[]),
    lambda: math.fsum(1),
    lambda: math.fsum(['spam']),
    lambda: math.fsum([1+2j]),
    lambda: math.fsum([IndexLike(10**10000)]),
    lambda: math.fsum([BadFloat()]),
    lambda: math.fsum([RaisesFloat()]),
    lambda: math.fsum([1e308, 1e308]),
    lambda: math.fsum([math.inf, -math.inf]),
    lambda: math.fsum(bad_iter()),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_sumprod_diff_subset() {
    let probe = run_cpython("import math; print(hasattr(math, 'sumprod'))")
        .expect("failed to probe CPython math.sumprod support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True\n" {
        eprintln!("skipping math.sumprod diff: CPython oracle lacks math.sumprod");
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testSumProd public stable subset",
        name: "math-sumprod",
        source: r#"import math
sumprod = math.sumprod
print(sumprod(iter([10, 20, 30]), (1, 2, 3)), type(sumprod(iter([10, 20, 30]), (1, 2, 3))).__name__)
print(sumprod([1.5, 2.5], [3.5, 4.5]), type(sumprod([1.5, 2.5], [3.5, 4.5])).__name__)
print(sumprod([], []), type(sumprod([], [])).__name__)
print(sumprod([-1], [1.]), sumprod([1.], [-1]), type(sumprod([-1], [1.])).__name__, type(sumprod([1.], [-1])).__name__)
print(sumprod([10**20], [1]), type(sumprod([10**20], [1])).__name__)
print(sumprod([1], [10**20]), type(sumprod([1], [10**20])).__name__)
print(sumprod([10**10], [10**10]), type(sumprod([10**10], [10**10])).__name__)
print(sumprod([0.1] * 10, [1] * 10))
print(sumprod([0.1] * 20, [True, False] * 10), sumprod([True, False] * 10, [0.1] * 20))
print(sumprod([1.0, 10E100, 1.0, -10E100], [1.0] * 4))
print(sumprod([10.1, math.inf], [20.2, 30.3]), sumprod([10.1, -math.inf], [20.2, 30.3]))
print(math.isnan(sumprod([10.1, math.inf], [-math.inf, math.inf])))
print(math.isnan(sumprod([10.1, math.nan], [20.2, 30.3])), math.isnan(sumprod([10.1, math.inf], [math.nan, 30.3])), math.isnan(sumprod([10.1, math.inf], [20.3, math.nan])))

for expr in [
    lambda: sumprod(),
    lambda: sumprod([]),
    lambda: sumprod([], [], []),
    lambda: sumprod(None, []),
    lambda: sumprod([], None),
    lambda: sumprod(['x'], [1.0]),
    lambda: sumprod([1], [1, 2]),
    lambda: sumprod([1, 2], [1]),
    lambda: sumprod([10**1000], [1.0]),
    lambda: sumprod([1.0], [10**1000]),
    lambda: sumprod(p=[], q=[]),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_nextafter_ulp_diff_subset() {
    let probe =
        run_cpython("import math\ntry:\n    math.nextafter(1.0, 2.0, steps=0)\n    print(True)\nexcept TypeError:\n    print(False)")
            .expect("failed to probe CPython math.nextafter steps support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True\n" {
        eprintln!(
            "skipping math.nextafter/ulp diff: CPython oracle lacks math.nextafter steps support"
        );
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::test_nextafter/test_ulp public stable subset",
        name: "math-nextafter-ulp",
        source: r#"import math
print(math.nextafter(4503599627370496.0, -math.inf), math.nextafter(4503599627370496.0, math.inf))
print(math.nextafter(9223372036854775808.0, 0.0), math.nextafter(-9223372036854775808.0, 0.0))
print(math.nextafter(1.0, -math.inf), math.nextafter(1.0, math.inf))
print(math.nextafter(1.0, -math.inf, steps=3), math.nextafter(1.0, math.inf, steps=3))
print(math.nextafter(2.0, 2.0), math.copysign(1.0, math.nextafter(-0.0, +0.0)), math.copysign(1.0, math.nextafter(+0.0, -0.0)))
print(math.nextafter(+0.0, math.inf), math.nextafter(-0.0, math.inf), math.nextafter(+0.0, -math.inf), math.nextafter(-0.0, -math.inf))
smallest = float('2.2250738585072014e-308') * 2.220446049250313e-16
print(math.copysign(1.0, math.nextafter(smallest, +0.0)), math.copysign(1.0, math.nextafter(-smallest, +0.0)), math.copysign(1.0, math.nextafter(smallest, -0.0)), math.copysign(1.0, math.nextafter(-smallest, -0.0)))
print(math.nextafter(math.inf, 0.0), math.nextafter(-math.inf, 0.0))
largest = float('1.7976931348623157e+308')
print(math.nextafter(largest, math.inf), math.nextafter(-largest, -math.inf))
print(math.isnan(math.nextafter(math.nan, 1.0)), math.isnan(math.nextafter(1.0, math.nan)), math.isnan(math.nextafter(math.nan, math.nan)), math.nextafter(1.0, math.inf, steps=0))
print(math.ulp(1.0), math.ulp(2 ** 52), math.ulp(2 ** 53), math.ulp(2 ** 64))
print(math.ulp(0.0), math.ulp(largest))
print(math.ulp(math.inf), math.isnan(math.ulp(math.nan)))
print(math.ulp(-0.0), math.ulp(-1.0), math.ulp(-(2 ** 64)), math.ulp(-math.inf))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')
class BadIndex:
    def __index__(self):
        return 1.0
class RaisesIndex:
    def __index__(self):
        raise ValueError('bad index')

print(math.nextafter(FloatLike(1.0), FloatLike(2.0)), math.nextafter(1.0, math.inf, steps=IndexLike(2)), math.nextafter(1.0, math.inf, steps=True), math.nextafter(1.0, math.inf, steps=False), math.nextafter(1.0, math.inf, steps=None), math.ulp(FloatLike(1.0)), math.ulp(IndexLike(2**64)))
for expr in [
    lambda: math.nextafter(),
    lambda: math.nextafter(1),
    lambda: math.nextafter(1, 2, 3),
    lambda: math.nextafter(1, 2, 3, 4),
    lambda: math.nextafter(x=1, y=2),
    lambda: math.nextafter(1, 2, steps=-1),
    lambda: math.nextafter(1, 2, steps=1.0),
    lambda: math.nextafter(1, 2, steps=BadIndex()),
    lambda: math.nextafter(1, 2, steps=RaisesIndex()),
    lambda: math.nextafter('x', 1),
    lambda: math.nextafter(1+2j, 1),
    lambda: math.nextafter(IndexLike(10**10000), 1),
    lambda: math.nextafter(BadFloat(), 1),
    lambda: math.nextafter(RaisesFloat(), 1),
    lambda: math.ulp(),
    lambda: math.ulp(1, 2),
    lambda: math.ulp(x=1),
    lambda: math.ulp('x'),
    lambda: math.ulp(1+2j),
    lambda: math.ulp(IndexLike(10**10000)),
    lambda: math.ulp(BadFloat()),
    lambda: math.ulp(RaisesFloat()),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_math_pow_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_math.py::MathTests::testPow public stable subset",
        name: "math-pow",
        source: r#"import math
print(math.pow(0, 1), math.pow(1, 0), math.pow(2, 1), math.pow(2, -1))
print(math.pow(math.inf, 1), math.pow(-math.inf, 1), math.pow(1, math.inf), math.pow(1, -math.inf))
print(math.isnan(math.pow(math.nan, 1)), math.isnan(math.pow(2, math.nan)), math.isnan(math.pow(0, math.nan)), math.pow(1, math.nan))
print(math.pow(0.0, math.inf), math.pow(0.0, 3.0), math.pow(0.0, 2.3), math.pow(0.0, 0.0))
print(math.pow(math.inf, math.inf), math.pow(math.inf, -2.0), math.pow(math.inf, -math.inf), math.isnan(math.pow(math.inf, math.nan)))
print(math.pow(-0.0, math.inf), math.pow(-0.0, 2.3), math.pow(-0.0, 2.0), math.isnan(math.pow(-0.0, math.nan)))
print(math.pow(-0.0, 3.0), math.copysign(1.0, math.pow(-0.0, 3.0)))
print(math.pow(-math.inf, math.inf), math.pow(-math.inf, 3.0), math.pow(-math.inf, 2.3), math.pow(-math.inf, 2.0))
print(math.pow(-math.inf, -3.0), math.copysign(1.0, math.pow(-math.inf, -3.0)), math.pow(-math.inf, -math.inf), math.isnan(math.pow(-math.inf, math.nan)))
print(math.pow(-1.0, math.inf), math.pow(-1.0, 3.0), math.pow(-1.0, 2.0), math.pow(-1.0, -3.0), math.pow(-1.0, -math.inf), math.isnan(math.pow(-1.0, math.nan)))
print(math.pow(1.9, -math.inf), math.pow(0.9, -math.inf), math.pow(-0.9, -math.inf), math.pow(-1.9, -math.inf))
print(math.pow(1.9, math.inf), math.pow(0.9, math.inf), math.pow(-0.9, math.inf), math.pow(-1.9, math.inf))
print(math.pow(-2.0, 3.0), math.pow(-2.0, 2.0), math.pow(-2.0, -1.0), math.pow(-2.0, -2.0), math.pow(-2.0, -3.0))

class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class IndexLike:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class BadFloat:
    def __float__(self):
        return 1
class RaisesFloat:
    def __float__(self):
        raise ValueError('bad float')

print(math.pow(FloatLike(2.0), FloatLike(3.0)), math.pow(IndexLike(2), IndexLike(-1)))
for expr in [
    lambda: math.pow(),
    lambda: math.pow(1),
    lambda: math.pow(1, 2, 3),
    lambda: math.pow(x=1, y=2),
    lambda: math.pow('x', 2),
    lambda: math.pow(1+2j, 2),
    lambda: math.pow(10**10000, 2),
    lambda: math.pow(BadFloat(), 2),
    lambda: math.pow(RaisesFloat(), 2),
    lambda: math.pow(1e100, 1e100),
    lambda: math.pow(0.0, -2.0),
    lambda: math.pow(-0.0, -3.0),
    lambda: math.pow(-1.0, 2.3),
    lambda: math.pow(-15.0, -3.1),
    lambda: math.pow(-2.0, 0.5),
    lambda: math.pow(-2.0, -0.5),
]:
    try:
        expr()
    except Exception as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_pure_memory_stdlib_core_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Pure-memory stdlib public smoke subset",
        name: "pure-memory-stdlib-core",
        source: r#"import collections, copy, functools, io, operator

Point = collections.namedtuple('Point', 'x y')
point = Point(1, 2)
print(point, point.x, sorted(point._asdict().items()))
counter = collections.Counter('ababa')
delta = collections.Counter({'b': -2, 'c': 3})
print(counter['a'], counter['z'], sorted(counter.items()), sorted((counter + delta).items()))

data = [1, [2]]
shallow = copy.copy(data)
deep = copy.deepcopy(data)
data[1].append(3)
print(shallow[1], deep[1], shallow is data, deep is data)

bio = io.BytesIO(b'ab')
print(bio.read(1), bio.write(b'Z'), bio.getvalue())

print(functools.reduce(lambda left, right: left + right, [1, 2, 3]))
pow2 = functools.partial(pow, 2)
print(pow2(5), pow2.func is pow, pow2.args)
def wrapped():
    return 'value'
@functools.wraps(wrapped)
def wrapper():
    return wrapped()
print(wrapper.__name__, wrapper())

class Box:
    pass
box = Box()
box.x = 7
print(operator.add(2, 3), operator.mul('x', 3), operator.itemgetter(1)(['a', 'b']))
print(operator.attrgetter('x')(box), operator.methodcaller('replace', 'a', 'b')('aardvark'))
print(operator.truth([]), operator.is_(None, None), operator.contains([1, 2], 2))"#,
    });
}

#[test]
fn cpython_collections_counter_public_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py public Counter subset",
        name: "collections-counter-public",
        source: r#"from collections import Counter
c = Counter('abracadabra')
print(c['a'], c['z'], sorted(c.items()))
print(sum(c.values()), sorted((c + Counter({'z': 2, 'a': -5})).items()))
print(sorted((c - Counter('aaa')).items()))
print(sorted((+Counter({'a': 2, 'b': 0, 'c': -1})).items()))
print(sorted((-Counter({'a': 2, 'b': 0, 'c': -1})).items()))
print(list(Counter({'a': 2, 'b': 0, 'c': -1}).elements()))
c.update('zz')
c.subtract({'a': 1, 'z': 3})
print(sorted(c.items()))
print(Counter(a=2, b=1) == Counter({'a': 2, 'b': 1}))"#,
    });
}

#[test]
fn cpython_collections_chainmap_public_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py public ChainMap subset",
        name: "collections-chainmap-public",
        source: r#"from collections import ChainMap
base = {'a': 1, 'b': 2}
override = {'b': 20, 'c': 30}
cm = ChainMap(override, base)
print(ChainMap().maps)
print(ChainMap({'x': 1}).maps)
print(bool(ChainMap()), bool(ChainMap({}, {})), bool(ChainMap({'x': 1}, {})), bool(ChainMap({}, {'x': 1})))
print(list(cm.items()), list(cm), len(cm), dict(cm))
print('a' in cm, 'b' in cm, 'c' in cm, 'z' in cm)
print(cm['a'], cm['b'], cm['c'], cm.get('z', 100))
child_source = {'d': 40}
child = cm.new_child(child_source)
print(child.maps, child.maps[0] is child_source, child.parents.maps == cm.maps)
child['e'] = 50
print(child.maps[0], 'e' in child, 'e' in cm)
del child['d']
print(child.maps[0], child.get('d', 'missing'))"#,
    });
}

#[test]
fn cpython_collections_namedtuple_public_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py public namedtuple subset",
        name: "collections-namedtuple-public",
        source: r#"from collections import namedtuple
Point = namedtuple('Point', 'x y')
p = Point(11, 22)
print(Point.__name__, Point.__slots__, Point._fields)
print(p, p.x, p.y, p[0], p[-1])
print(tuple(p), list(p), p == (11, 22), hash(p) == hash((11, 22)))
print(Point._make([11, 22]) == p)
print(p._replace(x=1))
print(p._asdict())
print(Point(x=11, y=22) == p, Point(y=22, x=11) == p)
Zero = namedtuple('Zero', '')
print(Zero(), Zero._fields, Zero()._asdict())
Dot = namedtuple('Dot', 'd')
print(Dot(1), Dot._make([1]), Dot(1)._replace(d=2), Dot(1)._asdict())
bad = 0
for typename, fields in [('class', 'x y'), ('Point', 'x x'), ('Point', '_x y'), ('9Point', 'x y')]:
    try:
        namedtuple(typename, fields)
    except ValueError:
        bad += 1
print('bad', bad)"#,
    });
}

#[test]
fn cpython_collections_userdict_userlist_public_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py public UserDict/UserList subset",
        name: "collections-userdict-userlist-public",
        source: r#"from collections import UserDict, UserList
from copy import copy
ud = UserDict()
ud[123] = 'abc'
print(ud[123], list(ud), len(ud), 123 in ud, ud.get(999))
inner = ud.copy()
print(inner.data is ud.data, inner.data == ud.data, type(inner).__name__)
ud.test = [1234]
outer = copy(ud)
print(outer.data is ud.data, outer.data == ud.data, outer.test is ud.test)
del ud[123]
print(list(ud), len(ud))
ul = UserList()
print(ul.data, type(ul).__name__)
ul.append(123)
print(ul.data, list(ul), len(ul), 123 in ul)
ul_copy = ul.copy()
print(ul_copy.data is ul.data, ul_copy.data == ul.data, type(ul_copy).__name__)
ul.test = [1234]
ul_outer = copy(ul)
print(ul_outer.data is ul.data, ul_outer.data == ul.data, ul_outer.test is ul.test)
constructed = UserList([1, 2])
from_userlist = UserList(constructed)
print(constructed.data, from_userlist.data, from_userlist.data is constructed.data)
from_userlist[0] = 9
del from_userlist[1]
print(from_userlist.data, constructed.data)"#,
    });
}

#[test]
fn cpython_collections_userdict_public_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py TestUserObjects UserDict public methods subset",
        name: "collections-userdict-public-methods",
        source: r#"from collections import UserDict
from copy import copy
print(set(dir(UserDict)) >= set(dir(dict)))
obj = UserDict()
obj[123] = 'abc'
print(obj[123], list(obj), len(obj), 123 in obj, obj.get(999))
internal = obj.copy()
print(internal.data is obj.data, internal.data == obj.data, type(internal).__name__)
obj.test = [1234]
external = copy(obj)
print(external.data is obj.data, external.data == obj.data, external.test is obj.test)
del obj[123]
print(list(obj), len(obj))"#,
    });
}

#[test]
fn cpython_collections_userlist_public_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py TestUserObjects UserList public methods subset",
        name: "collections-userlist-public-methods",
        source: r#"from collections import UserList
from copy import copy
print(set(dir(UserList)) >= set(dir(list)))
obj = UserList()
print(obj.data, type(obj).__name__)
obj.append(123)
print(obj.data, list(obj), len(obj), 123 in obj)
internal = obj.copy()
print(internal.data is obj.data, internal.data == obj.data, type(internal).__name__)
obj.test = [1234]
external = copy(obj)
print(external.data is obj.data, external.data == obj.data, external.test is obj.test)
constructed = UserList([1, 2])
from_userlist = UserList(constructed)
print(constructed.data, from_userlist.data, from_userlist.data is constructed.data)
from_userlist[0] = 9
del from_userlist[1]
print(from_userlist.data, constructed.data)"#,
    });
}

#[test]
fn cpython_collections_userstring_protocol_and_userdict_missing_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py TestUserObjects UserString protocol and UserDict missing subset",
        name: "collections-userstring-protocol-userdict-missing",
        source: r#"from collections import UserDict, UserString
print(set(dir(UserString)) >= set(dir(str)))
class A(UserDict):
    def __missing__(self, key):
        return 456
print(A()[123])
print(A().get(123) is None)
obj = A({1: 2})
print(obj[1], obj.get(999, 'fallback'), obj.data)
print(obj.__getitem__(123))"#,
    });
}

#[test]
fn cpython_collections_chainmap_missing_and_first_map_mutation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py TestChainMap missing and first-map mutation subset",
        name: "collections-chainmap-missing-first-map-mutation",
        source: r#"from collections import ChainMap
class DefaultChainMap(ChainMap):
    def __missing__(self, key):
        return 999
d = DefaultChainMap(dict(a=1, b=2), dict(b=20, c=30))
print(type(d).__name__, d.maps)
print([d[k] for k in ['a', 'b', 'c', 'd']])
print([d.get(k, 77) for k in ['a', 'b', 'c', 'd']])
print([k in d for k in ['a', 'b', 'c', 'd']])
print(d.pop('a', 1001), d.maps)
print(d.pop('a', 1002), d.maps)
print(d.popitem(), d.maps)
try:
    d.popitem()
except KeyError as error:
    print(error.__class__.__name__)
d = DefaultChainMap(dict(a=1, b=2), dict(c=3))
d.clear()
print(d.maps, list(d.items()), d.get('c'), 'a' in d, 'c' in d)
d['x'] = 5
print(d.maps, d['x'])
del d['x']
print(d.maps, d['missing'])"#,
    });
}

#[test]
fn cpython_collections_chainmap_iter_does_not_call_getitem_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py TestChainMap iteration avoids map __getitem__ subset",
        name: "collections-chainmap-iter-no-getitem",
        source: r#"from collections import ChainMap, UserDict
class DictWithGetItem(UserDict):
    def __init__(self, *args, **kwds):
        self.called = False
        UserDict.__init__(self, *args, **kwds)
    def __getitem__(self, item):
        self.called = True
        UserDict.__getitem__(self, item)
d = DictWithGetItem(a=1)
c = ChainMap(d)
d.called = False
print(set(c), d.called)"#,
    });
}

#[test]
fn cpython_collections_chainmap_new_child_custom_mapping_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py TestChainMap new_child custom mapping subset",
        name: "collections-chainmap-new-child-custom-mapping",
        source: r#"from collections import ChainMap
class lowerdict(dict):
    def __getitem__(self, key):
        if isinstance(key, str):
            key = key.lower()
        return dict.__getitem__(self, key)
    def __contains__(self, key):
        if isinstance(key, str):
            key = key.lower()
        return dict.__contains__(self, key)
c = ChainMap()
c['a'] = 1
c['b'] = 2
m = lowerdict(b=20, c=30)
d = c.new_child(m)
print(d.maps[0] is m)
print('a' in d, 'b' in d, 'c' in d, 'B' in d, 'C' in d)
print(d.get('a', 100), d.get('B', 100), d.get('C', 100), d.get('z', 100))
print(d['B'], d['C'])"#,
    });
}

#[test]
fn cpython_collections_chainmap_order_preservation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py TestChainMap order preservation subset",
        name: "collections-chainmap-order-preservation",
        source: r#"from collections import ChainMap, OrderedDict
d = ChainMap(
    OrderedDict(j=0, h=88888),
    OrderedDict(),
    OrderedDict(i=9999, d=4444, c=3333),
    OrderedDict(f=666, b=222, g=777, c=333, h=888),
    OrderedDict(),
    OrderedDict(e=55, b=22),
    OrderedDict(a=1, b=2, c=3, d=4, e=5),
    OrderedDict(),
)
print(''.join(d))
print(list(d.items()))"#,
    });
}

#[test]
fn cpython_collections_chainmap_union_operators_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_collections.py TestChainMap union operators subset",
        name: "collections-chainmap-union-operators",
        source: r#"from collections import ChainMap
cm1 = ChainMap(dict(a=1, b=2), dict(c=3, d=4))
cm2 = ChainMap(dict(a=10, e=5), dict(b=20, d=4))
cm3 = cm1.copy()
d = dict(a=10, c=30)
pairs = [('c', 3), ('p', 0)]
tmp = cm1 | cm2
print(tmp.maps)
cm1 |= cm2
print(cm1.maps, cm1 == tmp)
tmp = cm2 | d
print(tmp.maps)
print((d | cm2).maps)
cm2 |= d
print(cm2.maps, cm2 == tmp)
try:
    cm3 | pairs
except TypeError as error:
    print(error.__class__.__name__)
tmp = cm3.copy()
cm3 |= pairs
print(cm3.maps, tmp.maps)
class Subclass(ChainMap):
    pass
class SubclassRor(ChainMap):
    def __ror__(self, other):
        return super().__ror__(other)
left = Subclass(dict(a=1)) | ChainMap(dict(b=2))
right = dict(z=0) | Subclass(dict(a=1), dict(b=2))
mixed = ChainMap(dict(a=1)) | Subclass(dict(b=2))
print(type(left).__name__, left.maps)
print(type(right).__name__, right.maps)
print(type(mixed).__name__, mixed.maps)
for value in [
    ChainMap() | ChainMap(),
    ChainMap() | Subclass(),
    Subclass() | ChainMap(),
    ChainMap() | SubclassRor(),
]:
    print(type(value).__name__, type(value.maps[0]).__name__)"#,
    });
}

#[test]
fn cpython_operator_public_helpers_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_operator.py public helper subset",
        name: "operator-public-helpers",
        source: r#"import operator
class Box:
    pass
box = Box()
box.x = 7
box.child = Box()
box.child.y = 9
print(operator.lt(1, 2), operator.eq('a', 'a'), operator.ne('a', 'b'))
print(operator.truth([0]), operator.not_([]), operator.is_(None, None), operator.is_not(None, 0))
print(operator.add(2, 3), operator.sub(5, 2), operator.mul('x', 3), operator.floordiv(7, 2), operator.mod(7, 2), operator.pow(2, 5))
print(operator.and_(6, 3), operator.or_(4, 1), operator.xor(6, 3), operator.lshift(3, 2), operator.rshift(8, 1))
print(operator.concat('py', 'thon'), operator.contains([1, 2, 3], 2), operator.countOf([1, 2, 1], 1), operator.indexOf(['a', 'b'], 'b'))
items = [10, 20, 30]
print(operator.getitem(items, 1))
print(operator.setitem(items, 0, 99), items)
print(operator.delitem(items, 1), items)
print(operator.attrgetter('x')(box), operator.attrgetter('child.y')(box))
print(operator.itemgetter(1)(['a', 'b', 'c']), operator.itemgetter(0, 2)(['a', 'b', 'c']))
print(operator.methodcaller('replace', 'a', 'o')('banana'))"#,
    });
}

#[test]
fn cpython_operator_length_hint_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_operator.py::OperatorTestCase::test_length_hint and Lib/test/test_enumerate.py::TestReversed::test_len public subset",
        name: "operator-length-hint",
        source: r#"import operator
class X:
    def __init__(self, value):
        self.value = value
    def __length_hint__(self):
        if type(self.value) is type:
            raise self.value
        return self.value
class Y:
    pass
print(operator.length_hint([], 2), operator.length_hint(iter([1, 2, 3])))
print(operator.length_hint(X(2)), operator.length_hint(X(NotImplemented), 4), operator.length_hint(X(TypeError), 12), operator.length_hint(Y(), 10))
for value in [X('abc'), X(-2), X(LookupError)]:
    try:
        operator.length_hint(value)
    except (TypeError, ValueError, LookupError) as error:
        print(type(error).__name__)
try:
    operator.length_hint(X(2), 'abc')
except TypeError as error:
    print(type(error).__name__)
lengths = []
for seq in ('hello', tuple('hello'), list('hello'), range(5)):
    rev = reversed(seq)
    lengths.append((operator.length_hint(rev), len(seq)))
    list(rev)
    lengths.append(operator.length_hint(rev))
print(lengths)
class SeqWithWeirdLen:
    called = False
    def __len__(self):
        if not self.called:
            self.called = True
            return 10
        raise ZeroDivisionError
    def __getitem__(self, index):
        return index
try:
    operator.length_hint(reversed(SeqWithWeirdLen()))
except ZeroDivisionError as error:
    print(type(error).__name__)"#,
    });
}

#[test]
fn cpython_copy_public_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/copy.py public pure-memory subset",
        name: "copy-public",
        source: r#"import copy
nested = [1, [2], {'a': [3]}]
shallow = copy.copy(nested)
deep = copy.deepcopy(nested)
nested[1].append(4)
nested[2]['a'].append(5)
print(shallow is nested, deep is nested)
print(shallow[1] is nested[1], deep[1] is nested[1], shallow[1], deep[1])
print(shallow[2] is nested[2], deep[2] is nested[2], shallow[2], deep[2])
for value in [None, True, 42, 'abc', b'abc', (1, 2)]:
    print(type(value).__name__, copy.copy(value) == value, copy.deepcopy(value) == value)
ba = bytearray(b'ab')
ba_shallow = copy.copy(ba)
ba_deep = copy.deepcopy(ba)
ba.append(ord('c'))
print(type(ba_shallow).__name__, ba_shallow == bytearray(b'ab'), ba_shallow is ba)
print(type(ba_deep).__name__, ba_deep == bytearray(b'ab'), ba_deep is ba)
d = {'x': [1]}
ds = copy.copy(d)
dd = copy.deepcopy(d)
d['x'].append(2)
print(ds is d, dd is d, ds['x'], dd['x'])
for expr in [lambda: copy.copy(), lambda: copy.copy(1, 2), lambda: copy.deepcopy()]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_io_bytesio_public_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryio.py public BytesIO pure-memory subset",
        name: "io-bytesio-public",
        source: r#"import io
bio = io.BytesIO(b'abc')
print(type(bio).__name__, bio.read(1), bio.read(), bio.read())
print(bio.getvalue())
bio = io.BytesIO()
print(bio.write(b'ab'), bio.write(bytearray(b'cd')), bio.getvalue())
print(bio.read())
bio = io.BytesIO(b'XYZW')
target = bytearray(b'abc')
print(bio.readinto(target), target)
print(bio.readinto(target), target)
for source in [None, b'ab', bytearray(b'ab'), memoryview(b'ab')]:
    obj = io.BytesIO() if source is None else io.BytesIO(source)
    out = bytearray(4)
    print(type(obj).__name__, obj.readinto(out), out, obj.getvalue())
for label, expr in [('bad-source', lambda: io.BytesIO(123)), ('too-many', lambda: io.BytesIO(b'a', b'b')), ('write-str', lambda: io.BytesIO().write('x')), ('read-too-many', lambda: io.BytesIO().read(1, 2)), ('getvalue-arg', lambda: io.BytesIO().getvalue(1))]:
    try:
        expr()
    except TypeError as error:
        print(label, error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_functools_public_helpers_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py public helper subset",
        name: "functools-public-helpers",
        source: r#"import functools
print(functools.reduce(lambda a, b: a + b, [1, 2, 3]))
print(functools.reduce(lambda a, b: a * b, [2, 3, 4], 1))
pow2 = functools.partial(pow, 2)
print(pow2(5), pow2.func is pow, pow2.args, pow2.keywords)
mod10 = functools.partial(pow, mod=10)
print(mod10(2, 6), mod10(exp=6, base=2))
def wrapped(a=1):
    'doc'
    return a + 1
@functools.wraps(wrapped)
def wrapper(*args, **kwargs):
    return wrapped(*args, **kwargs)
print(wrapper.__name__, wrapper.__doc__, wrapper(4), wrapper.__wrapped__ is wrapped)
def cmp(left, right):
    return (left > right) - (left < right)
key = functools.cmp_to_key(cmp)
values = [3, 1, 2]
print(sorted(values, key=key))
print(key(1) < key(2), key(2) == key(2), key(3) > key(1))
for expr in [lambda: functools.reduce(lambda a,b:a+b, []), lambda: functools.partial(), lambda: functools.cmp_to_key(), lambda: functools.wraps()]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_functools_partial_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py::TestPartial public subset",
        name: "functools-partial",
        source: r#"from functools import partial
def capture(*args, **kwargs):
    return args, kwargs

p = partial(capture, 1, 2, a=10, b=20)
print(callable(p), type(p).__name__)
print(p(3, 4, b=30, c=40))
print(p.func is capture, p.args, p.keywords == {'a': 10, 'b': 20})
for expr in [lambda: partial(), lambda: partial(2), lambda: partial(2)()]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)

d = {'a': 3}
def func(a=10, b=20):
    return a
p = partial(func, a=5)
print(p(**d), d)
p(b=7)
print(d)

d = {'a': 3}
p = partial(capture, **d)
print(p())
d['a'] = 5
print(p())

p = partial(capture)
print(p.keywords, p(), p(1, 2))
p = partial(capture, 1, 2)
print(p(), p(3, 4))
p = partial(capture, a=1)
print(p.keywords, p(), p(b=2), p(a=3, b=2))
for args in [(), (0,), (0, 1), (0, 1, 2), (0, 1, 2, 3)]:
    got, empty = partial(capture, *args)('x')
    print(got == args + ('x',), empty == {})
for a in ['a', 0, None, 3.5]:
    empty, got = partial(capture, a=a)(x=None)
    print(empty == (), got == {'a': a, 'x': None})

p = partial(capture, 0, a=1)
args1, kw1 = p(1, b=2)
args2, kw2 = p()
print(args1, kw1, args2, kw2)

def div(x, y):
    x / y
for expr in [lambda: partial(div, 1, 0)(), lambda: partial(div, 1)(0), lambda: partial(div)(1, 0), lambda: partial(div, y=0)(1)]:
    try:
        expr()
    except ZeroDivisionError as error:
        print(error.__class__.__name__)

p = partial(capture, 'first')
p2 = partial(p, 'second')
p2.new_attr = 'spam'
print(p2(), p2.new_attr, p2.__dict__['new_attr'])
del p2.new_attr
try:
    p2.new_attr
except AttributeError as error:
    print(error.__class__.__name__)
for attr in ['func', 'args', 'keywords']:
    try:
        setattr(p2, attr, 42)
    except AttributeError as error:
        print(attr, error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_functools_reduce_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py::TestReduce public stable subset",
        name: "functools-reduce",
        source: r#"from functools import reduce
from operator import add

class Squares:
    def __init__(self, max):
        self.max = max
        self.sofar = []
    def __len__(self):
        return len(self.sofar)
    def __getitem__(self, i):
        if not 0 <= i < self.max:
            raise IndexError
        n = len(self.sofar)
        while n <= i:
            self.sofar.append(n * n)
            n += 1
        return self.sofar[i]

print(reduce(add, ['a', 'b', 'c'], ''))
print(reduce(add, [['a', 'c'], [], ['d', 'w']], []))
print(reduce(lambda x, y: x * y, range(2, 8), 1))
print(reduce(lambda x, y: x * y, range(2, 21), 1))
print(reduce(add, Squares(10)), reduce(add, Squares(10), 0), reduce(add, Squares(0), 0))
print(reduce(42, '1'), reduce(42, '', '1'))
print(reduce(add, [], None), reduce(add, [], 42))

class SequenceClass:
    def __init__(self, n):
        self.n = n
    def __getitem__(self, i):
        if 0 <= i < self.n:
            return i
        raise IndexError

print(reduce(add, SequenceClass(5)), reduce(add, SequenceClass(5), 42))
print(reduce(add, SequenceClass(0), 42), reduce(add, SequenceClass(1)), reduce(add, SequenceClass(1), 42))
d = {'one': 1, 'two': 2, 'three': 3}
print(reduce(add, d), ''.join(d.keys()))

class TestFailingIter:
    def __iter__(self):
        raise RuntimeError
class BadSeq:
    def __getitem__(self, index):
        raise ValueError

checks = [
    lambda: reduce(),
    lambda: reduce(add),
    lambda: reduce(42, 42),
    lambda: reduce(42, 42, 42),
    lambda: reduce(add, []),
    lambda: reduce(add, ''),
    lambda: reduce(add, ()),
    lambda: reduce(42, (42, 42)),
    lambda: reduce(add, object()),
    lambda: reduce(add, SequenceClass(0)),
    lambda: reduce(add, [1], 2, 3),
]
for check in checks:
    try:
        check()
    except TypeError as error:
        print(error.__class__.__name__)
for check in [lambda: reduce(add, TestFailingIter()), lambda: reduce(42, BadSeq())]:
    try:
        check()
    except (RuntimeError, ValueError) as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_functools_cmp_to_key_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py::TestCmpToKey public subset",
        name: "functools-cmp-to-key",
        source: r#"from functools import cmp_to_key
def mycmp(x, y):
    return (x > y) - (x < y)
K = cmp_to_key(mycmp)
a = K(1)
b = K(2)
same = K(1)
print(callable(K), callable(a), a.obj, b.obj)
again = a(2)
print(again.obj, again > same)
print(a < b, a <= b, a == b, a != b, a > b, a >= b)
print(a == same, a <= same, a >= same)
print(sorted([5, 2, 4, 1, 3], key=K))
a.obj = 3
print(a.obj, a > b)
del a.obj
print(a.obj is None)
print(cmp_to_key(mycmp=mycmp)(obj='x').obj)

def len_cmp(x, y):
    return (len(x) > len(y)) - (len(x) < len(y))
print(sorted(['aaa', 'b', 'cc'], key=cmp_to_key(len_cmp)))
K_reverse = cmp_to_key(lambda x, y: (y > x) - (y < x))
print(sorted([1, 2, 3], key=K_reverse))

def bad(x, y):
    raise ValueError('bad')
for expr in [
    lambda: K(1) < 1,
    lambda: K(1) == 1,
    lambda: K(1) != 1,
    lambda: hash(K),
    lambda: hash(K(1)),
    lambda: K(),
    lambda: K(1, 2),
    lambda: K(other=1),
    lambda: cmp_to_key(),
    lambda: cmp_to_key(mycmp=mycmp, other=1),
    lambda: cmp_to_key(3)(1) < cmp_to_key(3)(2),
    lambda: cmp_to_key(bad)(1) < cmp_to_key(bad)(2),
]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_functools_update_wrapper_wraps_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py TestUpdateWrapper/TestWraps public stable subset",
        name: "functools-update-wrapper-wraps",
        source: r#"from functools import update_wrapper, wraps

def f(a: 'new'):
    'doc'
    return a + 1
f.attr = 'value'
f.extra = {'a': 1}
f.__wrapped__ = 'lie'

def wrapper(b: 'old'):
    return b
result = update_wrapper(wrapper, f)
print(result is wrapper, wrapper.__wrapped__ is f, wrapper.__module__ == f.__module__)
print(wrapper.__name__, wrapper.__qualname__ == f.__qualname__, wrapper.__doc__, wrapper.attr)
print(wrapper.__annotations__ == {'a': 'new'}, 'b' in wrapper.__annotations__)
print(wrapper.__dict__['__wrapped__'] is f, wrapper.__dict__['attr'])

def g():
    'doc'
    pass
g.attr = 'x'
def w():
    pass
update_wrapper(w, g, (), ())
print(w.__name__, hasattr(w, 'attr'), w.__wrapped__ is g)

def source():
    pass
source.attr = 'assigned'
source.dict_attr = {'a': 1, 'b': 2}
def dest():
    pass
dest.dict_attr = {}
update_wrapper(dest, source, ('attr',), ('dict_attr',))
print(dest.attr, sorted(dest.dict_attr.items()), dest.__wrapped__ is source)

def missing():
    pass
def dest2():
    pass
dest2.dict_attr = {}
update_wrapper(dest2, missing, ('attr',), ('dict_attr',))
print('attr' in dest2.__dict__, dest2.dict_attr)
del dest2.dict_attr
for expr in [
    lambda: update_wrapper(dest2, source, (), ('dict_attr',)),
    lambda: update_wrapper(),
    lambda: wraps(),
]:
    try:
        expr()
    except (TypeError, AttributeError) as error:
        print(error.__class__.__name__)

dest2.dict_attr = 1
try:
    update_wrapper(dest2, source, (), ('dict_attr',))
except (TypeError, AttributeError) as error:
    print(error.__class__.__name__)

print(callable(wraps(f)), type(wraps(f)).__name__)
@wraps(f)
def decorated():
    return f(4)
print(decorated.__name__, decorated.__wrapped__ is f, decorated.attr, decorated.__annotations__ == {'a': 'new'}, decorated())

@wraps(f, (), ())
def plain():
    pass
print(plain.__name__, hasattr(plain, 'attr'), plain.__wrapped__ is f)"#,
    });
}

#[test]
fn cpython_functools_total_ordering_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py::TestTotalOrdering public subset",
        name: "functools-total-ordering",
        source: r#"from functools import total_ordering

def show(label, cls):
    a = cls(1)
    b = cls(2)
    c = cls(2)
    print(label, a < b, b > a, a <= b, b >= a, c <= b, c >= b)

@total_ordering
class LT:
    def __init__(self, value):
        self.value = value
    def __lt__(self, other):
        return self.value < other.value
    def __eq__(self, other):
        return self.value == other.value
show('lt', LT)
print(LT.__le__.__name__, LT.__gt__.__name__, LT.__ge__.__module__)

@total_ordering
class LE:
    def __init__(self, value):
        self.value = value
    def __le__(self, other):
        return self.value <= other.value
    def __eq__(self, other):
        return self.value == other.value
show('le', LE)

@total_ordering
class GT:
    def __init__(self, value):
        self.value = value
    def __gt__(self, other):
        return self.value > other.value
    def __eq__(self, other):
        return self.value == other.value
show('gt', GT)

@total_ordering
class GE:
    def __init__(self, value):
        self.value = value
    def __ge__(self, other):
        return self.value >= other.value
    def __eq__(self, other):
        return self.value == other.value
show('ge', GE)

@total_ordering
class Keep:
    def __init__(self, value):
        self.value = value
    def __lt__(self, other):
        return self.value < other.value
    def __le__(self, other):
        return 'kept'
    def __eq__(self, other):
        return self.value == other.value
print('keep', Keep(1).__le__(Keep(2)), Keep.__le__.__name__)

try:
    @total_ordering
    class Empty:
        pass
except ValueError as error:
    print('empty', error.__class__.__name__, 'ordering operation' in str(error))

@total_ordering
class N:
    def __init__(self, value):
        self.value = value
    def __eq__(self, other):
        if isinstance(other, N):
            return self.value == other.value
        return False
    def __lt__(self, other):
        if isinstance(other, N):
            return self.value < other.value
        return NotImplemented
n = N(1)
print('notimpl', n.__le__(1) is NotImplemented, n.__gt__(1) is NotImplemented, n.__ge__(1) is NotImplemented)
try:
    n < 1
except TypeError as error:
    print('type', error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_functools_partialmethod_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py::TestPartialMethod public subset",
        name: "functools-partialmethod",
        source: r#"from functools import partial, partialmethod

def normalize(value):
    if isinstance(value, A):
        return 'self'
    if value is A:
        return 'A'
    return value

def capture(*args, **kwargs):
    normalized = []
    for arg in args:
        normalized.append(normalize(arg))
    return tuple(normalized), sorted(kwargs.items())

class A:
    nothing = partialmethod(capture)
    positional = partialmethod(capture, 1)
    keywords = partialmethod(capture, a=2)
    both = partialmethod(capture, 3, b=4)
    spec_keywords = partialmethod(capture, self=1, func=2)
    nested = partialmethod(positional, 5)
    over_partial = partialmethod(partial(capture, c=6), 7)
    static = partialmethod(staticmethod(capture), 8)
    cls = partialmethod(classmethod(capture), d=9)

a = A()
for call in [
    lambda: a.nothing(),
    lambda: a.nothing(5, c=6),
    lambda: a.positional(),
    lambda: a.keywords(c=6),
    lambda: a.both(5, c=6),
    lambda: A.both(a, 5, c=6),
    lambda: a.spec_keywords(),
    lambda: a.nested(6, d=7),
    lambda: A.nested(a, 6, d=7),
    lambda: a.over_partial(5, d=8),
    lambda: A.over_partial(a, 5, d=8),
    lambda: a.static(5, d=8),
    lambda: A.static(5, d=8),
    lambda: a.cls(5, c=8),
    lambda: A.cls(5, c=8),
    lambda: a.keywords(a=3),
]:
    print(call())
print(hasattr(a.both, '__self__'), a.both.__self__ is a)
print(hasattr(a.keywords, '__self__'), a.keywords.__self__ is a)
print(hasattr(A.keywords, '__self__'), hasattr(a.static, '__self__'), hasattr(A.static, '__self__'))
for expr in [
    lambda: partialmethod(None, 1),
    lambda: partialmethod(),
    lambda: partialmethod(func=capture, a=1),
]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)
print(callable(partialmethod(capture)), type(partialmethod(capture)).__name__)"#,
    });
}

#[test]
fn cpython_functools_cached_property_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py cached_property public subset",
        name: "functools-cached-property",
        source: r#"from functools import cached_property

class CachedCostItem:
    _cost = 1
    @cached_property
    def cost(self):
        'The cost of the item.'
        self._cost += 1
        return self._cost

item = CachedCostItem()
print(item.cost, item.cost, sorted(item.__dict__.items()))
print(type(CachedCostItem.cost).__name__, CachedCostItem.cost.__doc__, CachedCostItem.cost.__module__ in ('__main__', 'functools'), CachedCostItem.cost.attrname)

class OptionallyCachedCostItem:
    _cost = 1
    def get_cost(self):
        self._cost += 1
        return self._cost
    cached_cost = cached_property(get_cost)

item = OptionallyCachedCostItem()
print(item.get_cost(), item.cached_cost, item.get_cost(), item.cached_cost, sorted(item.__dict__.items()))

for label, maker in [
    ('reuse-different', lambda: type('ReuseDifferent', (), {'a': cached_property(lambda self: 1), 'b': None})),
    ('manual-set-name', lambda: type('Foo', (), {})()),
    ('slots', lambda: None),
]:
    try:
        if label == 'reuse-different':
            cp = cached_property(lambda self: 1)
            class ReusedCachedProperty:
                a = cp
                b = cp
        elif label == 'manual-set-name':
            cp = cached_property(lambda self: 5)
            class Foo:
                pass
            Foo.cp = cp
            Foo().cp
        else:
            class Slots:
                __slots__ = ('_cost',)
                def __init__(self):
                    self._cost = 1
                @cached_property
                def cost(self):
                    return 9
            Slots().cost
    except (TypeError, RuntimeError):
        print(label, 'error')

counter = 0
@cached_property
def _cp(_self):
    global counter
    counter += 1
    return counter
class A:
    cp = _cp
class B:
    cp = _cp
a = A()
b = B()
print(a.cp, b.cp, a.cp, _cp.attrname)

calls = []
class Descriptor:
    def __set_name__(self, owner, name):
        calls.append((owner.__name__, name))
class WithDescriptor:
    field = Descriptor()
print(calls)

calls = []
class DynamicDescriptor:
    def __set_name__(self, owner, name):
        calls.append((owner.__name__, name))
Dynamic = type('Dynamic', (), {'field': DynamicDescriptor()})
print(calls)"#,
    });
}

#[test]
fn cpython_functools_cache_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py TestCache/TestLRU public stable subset",
        name: "functools-cache-lru-cache",
        source: r#"from functools import cache, lru_cache

@cache
def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)
print([fib(n) for n in range(12)])
info = fib.cache_info()
print(tuple(info), info.hits, info.misses, info.maxsize, info.currsize)
print(sorted(fib.cache_parameters().items()))
fib.cache_clear()
print(tuple(fib.cache_info()))

calls = []
@cache
def double(value):
    calls.append(value)
    return value * 2
print(double(3), double(3), calls, tuple(double.cache_info()))
print(double.__wrapped__(3), calls, tuple(double.cache_info()))
try:
    double([])
except TypeError as error:
    print(error.__class__.__name__)
print(tuple(double.cache_info()))

@lru_cache(maxsize=None)
def by_kw(n):
    if n < 2:
        return n
    return by_kw(n=n - 1) + by_kw(n=n - 2)
print([by_kw(n=number) for number in range(12)])
print(tuple(by_kw.cache_info()))
print(sorted(by_kw.cache_parameters().items()))
by_kw.cache_clear()
print(tuple(by_kw.cache_info()))

@lru_cache(maxsize=2)
def identity(value):
    return value
print(identity(1), identity(2), identity(1), identity(3), tuple(identity.cache_info()))
print(identity(2), tuple(identity.cache_info()))

@lru_cache
def square(x):
    return x ** 2
print([square(x) for x in [10, 20, 10]], tuple(square.cache_info()))

zero_calls = []
@lru_cache(0)
def never():
    zero_calls.append(1)
    return 20
print([never() for _ in range(3)], len(zero_calls), tuple(never.cache_info()))

@lru_cache(maxsize=-10)
def neg(value):
    return value
for _ in range(2):
    for value in range(3):
        neg(value)
print(tuple(neg.cache_info()))

@lru_cache(maxsize=None)
def bad(index):
    return 'abc'[index]
print(bad(0), tuple(bad.cache_info()))
for _ in range(2):
    try:
        bad(15)
    except IndexError as error:
        print(error.__class__.__name__)
print(tuple(bad.cache_info()))

@lru_cache(maxsize=None, typed=True)
def identify(value):
    return type(value).__name__, value
print(identify(3), identify(3.0), identify(value=3), identify(value=3.0), tuple(identify.cache_info()))
cached_repr = lru_cache(typed=True)(repr)
print(cached_repr(1), cached_repr(True), cached_repr(1.0), tuple(cached_repr.cache_info()))

@lru_cache(maxsize=10)
def kwargs_order(**kwargs):
    return list(kwargs.items())
print(kwargs_order(a=1, b=2), kwargs_order(b=2, a=1), tuple(kwargs_order.cache_info()))"#,
    });
}

#[test]
fn cpython_functools_singledispatch_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py singledispatch public stable subset",
        name: "functools-singledispatch",
        source: r#"from functools import singledispatch
from collections.abc import Sized, MutableMapping, MutableSequence

@singledispatch
def g(obj):
    return 'base:' + type(obj).__name__
def g_int(i):
    return 'int:' + str(i)
g.register(int, g_int)
print(g('x'))
print(g(5), g(True), g([1, 2]))
print(g.dispatch(int) is g_int, g.dispatch(object) is g.dispatch(str))

@g.register(str)
def g_str(s):
    return 'str:' + s
print(g('x'), g.dispatch(str) is g_str)

class A: pass
class C(A): pass
class B(A): pass
class D(C, B): pass
def g_a(a): return 'A'
def g_b(b): return 'B'
g.register(A, g_a)
g.register(B, g_b)
print(g(A()), g(B()), g(C()), g(D()))

@singledispatch
def h(obj):
    'Simple test'
    return 'base'
print(h.__name__, h.__doc__, h.__wrapped__(None))
print(callable(h), type(h.registry).__name__, h.registry[object] is h.dispatch(object))
h.register(Sized, lambda obj: 'sized')
print(h({}), h([]), h(()))
h.register(MutableMapping, lambda obj: 'mapping')
h.register(MutableSequence, lambda obj: 'sequence')
h.register(tuple, lambda obj: 'tuple')
print(h({}), h([]), h(()))
print(h.dispatch(dict)({}), h.dispatch(list)([]), h.dispatch(tuple)(()))
print(h._clear_cache())
print(singledispatch(42).dispatch(object))
print(h.register(float, 42))
try:
    h(1.5)
except TypeError as error:
    print(error.__class__.__name__)
print('done')"#,
    });
}

#[test]
fn cpython_functools_singledispatchmethod_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_functools.py TestSingleDispatchMethod public stable subset",
        name: "functools-singledispatchmethod",
        source: r#"from functools import singledispatchmethod

class C:
    @singledispatchmethod
    def m(self, arg):
        return 'base:' + type(arg).__name__
    @m.register(int)
    def _(self, arg):
        return 'int:' + str(arg)
    @m.register(str)
    @classmethod
    def _(cls, arg):
        return 'cls:' + cls.__name__ + ':' + arg

c = C()
print(c.m(1), c.m(True), c.m([]), c.m('x'))
print(C.m(c, 1), C.m(c, 'x'))
descriptor = C.__dict__['m']
print(callable(descriptor), type(descriptor).__name__, descriptor.func.__name__, descriptor.dispatcher.dispatch(int)(c, 2))

def c_float(self, arg):
    return 'float:' + str(arg)
print(descriptor.register(float, c_float) is c_float, c.m(1.5))

@C.m.register(tuple)
def _(self, arg):
    return 'tuple:' + str(len(arg))
@c.m.register(bytes)
def _(self, arg):
    return 'bytes:' + str(len(arg))
print(c.m((1, 2)), c.m(b'abc'))
print(C.m.__name__, c.m.__name__)
try:
    singledispatchmethod()
except TypeError as error:
    print(error.__class__.__name__)

class S:
    @singledispatchmethod
    @staticmethod
    def m(arg):
        return 'base:' + str(arg)
    @m.register(int)
    @staticmethod
    def _(arg):
        return 'int:' + str(arg)
print(S.m(1), S().m(1), S.m('x'), S().m('x'))

class K:
    @singledispatchmethod
    @classmethod
    def m(cls, arg):
        return 'base:' + cls.__name__ + ':' + str(arg)
    @m.register(int)
    @classmethod
    def _(cls, arg):
        return 'int:' + cls.__name__ + ':' + str(arg)
print(K.m(1), K().m(1), K.m('x'), K().m('x'))"#,
    });
}

#[test]
fn cpython_itertools_core_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_itertools.py public pure-memory iterator core subset",
        name: "itertools-core-iterators",
        source: r#"import itertools
c = itertools.count(2, 3)
print(type(c).__name__, iter(c) is c, next(c), next(c), next(c))
ck = itertools.count(start=-1, step=2)
print(next(ck), next(ck), next(ck))
r = itertools.repeat('x', 3)
print(type(r).__name__, iter(r) is r, list(r), list(r))
print(list(itertools.repeat(object='y', times=2)))
print(list(itertools.repeat('z', -1)))
ch = itertools.chain([1, 2], (), 'ab', itertools.repeat(9, 2))
print(type(ch).__name__, iter(ch) is ch, list(ch), list(ch))
cf = itertools.chain.from_iterable([[1, 2], (), 'ab', itertools.repeat(9, 2)])
print(callable(itertools.chain.from_iterable), type(cf).__name__, iter(cf) is cf, list(cf), list(cf))
print(list(itertools.chain.from_iterable([])))
print(list(itertools.chain.from_iterable(items for items in [[3], [4, 5], []])))
comp = itertools.compress('abcdef', [1, 0, True, False, [], [1]])
print(type(comp).__name__, iter(comp) is comp, list(comp), list(comp))
print(list(itertools.compress([1, 2, 3], [0, 1])))
print(list(itertools.compress(data='abc', selectors=[1, 0, 1])))
print(list(itertools.compress((x for x in range(5)), (x % 2 for x in range(5)))))
class Flag:
    def __init__(self, value):
        self.value = value
    def __bool__(self):
        return self.value
print(list(itertools.compress('xy', [Flag(False), Flag(True)])))
ff = itertools.filterfalse(None, [0, 1, '', 'x', [], [1], False, True])
print(type(ff).__name__, iter(ff) is ff, list(ff), list(ff))
print(list(itertools.filterfalse(lambda value: value % 2, range(6))))
print(list(itertools.filterfalse(lambda value: value, (value for value in [0, 1, 2, 0]))))
tw = itertools.takewhile(lambda value: value < 3, [1, 2, 3, 1])
print(type(tw).__name__, iter(tw) is tw, list(tw), list(tw))
print(list(itertools.takewhile(lambda value: value, (value for value in [1, 2, 0, 3]))))
dw = itertools.dropwhile(lambda value: value < 3, [1, 2, 3, 1])
print(type(dw).__name__, iter(dw) is dw, list(dw), list(dw))
print(list(itertools.dropwhile(lambda value: value, (value for value in [1, 2, 0, 3]))))
sm = itertools.starmap(lambda left, right: left + right, [(1, 2), [3, 4], ('a', 'b')])
print(type(sm).__name__, iter(sm) is sm, list(sm), list(sm))
print(list(itertools.starmap(lambda left, right: left * right, ((value, 2) for value in range(4)))))
acc = itertools.accumulate([1, 2, 3])
print(type(acc).__name__, iter(acc) is acc, list(acc), list(acc))
print(list(itertools.accumulate([1, 2, 3], lambda left, right: left * right)))
print(list(itertools.accumulate([], initial=10)))
print(list(itertools.accumulate([1, 2], initial=10)))
print(list(itertools.accumulate(iterable=[1, 2], func=lambda left, right: left * right, initial=10)))
zl = itertools.zip_longest([1, 2], 'ab')
print(type(zl).__name__, iter(zl) is zl, list(zl), list(zl))
print(list(itertools.zip_longest()))
print(list(itertools.zip_longest([1], [2, 3], fillvalue='x')))
print(list(itertools.zip_longest((value for value in [1, 2]), [3])))
cy = itertools.cycle('ab')
print(type(cy).__name__, iter(cy) is cy, list(itertools.islice(cy, 6)), list(itertools.islice(cy, 3)))
print(list(itertools.islice(itertools.cycle([]), 3)))
print(list(itertools.islice(itertools.cycle(value for value in [1, 2]), 7)))
print(list(itertools.islice((value for value in range(5)), 1, 5, 2)))
print(list(itertools.islice(range(10), 4)))
print(list(itertools.islice(range(10), 2, None)))
print(list(itertools.islice(range(10), 1, 8, 3)))
print(list(itertools.islice(itertools.count(10), 2, 8, 2)))
it = iter(range(10))
print(list(itertools.islice(it, 2, 5)), next(it))
s = itertools.islice(range(3), 1)
print(type(s).__name__, iter(s) is s)
try:
    itertools.chain(iterable=[1])
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.chain.from_iterable()
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.chain.from_iterable(iterable=[[1]])
except TypeError as error:
    print(error.__class__.__name__)
try:
    list(itertools.chain.from_iterable([1]))
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.compress('abc')
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.filterfalse(None)
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.filterfalse(function=None, iterable=[])
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.takewhile(lambda value: True)
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.takewhile(predicate=lambda value: True, iterable=[])
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.dropwhile(lambda value: True)
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.dropwhile(predicate=lambda value: True, iterable=[])
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.starmap(lambda left, right: left + right)
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.starmap(function=lambda left, right: left + right, iterable=[])
except TypeError as error:
    print(error.__class__.__name__)
try:
    list(itertools.starmap(lambda value: value, [1]))
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.accumulate()
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.accumulate([1], lambda left, right: left + right, 0)
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.accumulate([1], bad=1)
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.zip_longest(iterable=[1])
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.cycle()
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.cycle(iterable=[1])
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.count(0, 1, 2)
except TypeError as error:
    print(error.__class__.__name__)
try:
    itertools.repeat()
except TypeError as error:
    print(error.__class__.__name__)
for expr in [
    lambda: itertools.islice(range(3)),
    lambda: itertools.islice(range(3), -1),
    lambda: itertools.islice(range(3), 1, -1),
    lambda: itertools.islice(range(3), 1, 2, 0),
    lambda: itertools.islice(iterable=range(3), stop=2),
]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_itertools_keyword_error_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_itertools.py public duplicate-keyword error subset",
        name: "itertools-keyword-errors",
        source: r#"import itertools
try:
    itertools.accumulate([1], func=lambda left, right: left + right, **{'func': lambda left, right: left})
except TypeError as error:
    print(error.__class__.__name__, 'multiple values' in str(error))
try:
    itertools.zip_longest([1], fillvalue=0, **{'fillvalue': 1})
except TypeError as error:
    print(error.__class__.__name__, 'multiple values' in str(error))"#,
    });
}

#[test]
fn cpython_itertools_pairwise_diff_subset() {
    let probe = run_cpython("import itertools; print(hasattr(itertools, 'pairwise'))")
        .expect("failed to probe CPython itertools.pairwise support");
    if !probe.status.success() || probe.stdout.as_slice() != b"True\n" {
        eprintln!("skipping itertools.pairwise diff: CPython oracle lacks itertools.pairwise");
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_itertools.py public pairwise core subset",
        name: "itertools-pairwise-core",
        source: r#"import itertools
p = itertools.pairwise('abcd')
print(type(p).__name__, iter(p) is p, list(p), list(p))
print(list(itertools.pairwise([1])), list(itertools.pairwise([])))
print(list(itertools.islice(itertools.pairwise(itertools.count(5)), 3)))
print(list(itertools.pairwise(value for value in [1, 2, 3, 4])))
for expr in [
    lambda: itertools.pairwise(),
    lambda: itertools.pairwise(range(3), range(3)),
    lambda: itertools.pairwise(iterable=range(3)),
]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
    });
}

// Differential smoke tests for CPython-compatible program behavior. These are
// intentionally written with syntax accepted by Python 3.9+ so the default
// `python3` on this machine can act as the oracle. Set MINIPYTHON_CPYTHON to a
// local CPython build when migrating newer syntax.
#[test]
fn cpython_program_output_parity_smoke_subset() {
    for case in [
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_additive_ops",
            name: "arithmetic-precedence",
            source: "print(1 + 2 * 3)\nprint((1 + 2) * 3)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_binary_mask_ops / ::test_shift_ops / ::test_additive_ops / ::test_multiplicative_ops / ::test_unary_ops",
            name: "operator-precedence-and-associativity",
            source: "print(1 & 1, 1 ^ 1, 1 | 1)\nprint(1 << 1, 8 >> 1, 1 << 1 >> 1)\nprint(1 - 1 - 1, 1 - 1 + 1 - 1 + 1)\nprint(1 / 1 * 1 % 1)\nprint(~1, ~1 ^ 1 & 1 | 1 & 1 ^ -1)\nprint(-1*1/1 + 1*1 - ---1*1)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_matrix_mul and Lib/test/test_operator.py::test_matmul",
            name: "matrix-multiply-special-methods",
            source: "class M:\n    def __matmul__(self, other):\n        return 4\n    def __imatmul__(self, other):\n        self.other = other\n        return self\nm = M()\nprint(m @ m)\nm @= 42\nprint(m.other)\nclass Left:\n    def __matmul__(self, other):\n        return other - 1\nprint(Left() @ 42)\nclass Right:\n    def __rmatmul__(self, other):\n        return other + 2\nprint(40 @ Right())",
        },
        DiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_match_call_does_not_raise_syntax_error",
            name: "syntax-match-soft-keyword-call-compiles",
            source: "\ndef match(x):\n    return 1+1\n\nmatch(34)\n",
        },
        DiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_case_call_does_not_raise_syntax_error",
            name: "syntax-case-soft-keyword-call-compiles",
            source: "\ndef case(x):\n    return 1+1\n\ncase(34)\n",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_plain_integers",
            name: "prefixed-integers-and-underscores",
            source: "print(0xff, 0o10, 0b101)\nprint(1_000 + 2)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_plain_integers",
            name: "grammar-token-plain-integers-method",
            source: r#"print(type(000) is type(0))
print(0xff == 255, 0o377 == 255, 2147483647 == 0o17777777777, 0b1001 == 9)
try:
    eval("0x")
except SyntaxError as error:
    print(error.__class__.__name__)
else:
    print("accepted")
from sys import maxsize
print(maxsize)
if maxsize == 2147483647:
    print(-2147483647 - 1 == -0o20000000000)
    print(0o37777777777 > 0, 0xffffffff > 0, 0b1111111111111111111111111111111 > 0)
    for s in ("2147483648", "0o40000000000", "0x100000000", "0b10000000000000000000000000000000"):
        try:
            x = eval(s)
            print(x > 0, isinstance(x, int))
        except OverflowError:
            print("OverflowError")
elif maxsize == 9223372036854775807:
    print(-9223372036854775807 - 1 == -0o1000000000000000000000)
    print(0o1777777777777777777777 > 0, 0xffffffffffffffff > 0, 0b11111111111111111111111111111111111111111111111111111111111111 > 0)
    for s in ("9223372036854775808", "0o2000000000000000000000", "0x10000000000000000", "0b100000000000000000000000000000000000000000000000000000000000000"):
        try:
            x = eval(s)
            print(x > 0, isinstance(x, int))
        except OverflowError:
            print("OverflowError")
else:
    print("weird maxsize")"#,
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_long_integers",
            name: "grammar-token-long-integers-method",
            source: r#"values = [
    0,
    0xffffffffffffffff,
    0Xffffffffffffffff,
    0o77777777777777777,
    0O77777777777777777,
    123456789012345678901234567890,
    0b100000000000000000000000000000000000000000000000000000000000000000000,
    0B111111111111111111111111111111111111111111111111111111111111111111111,
]
print(len(values))
for value in values:
    print(value, isinstance(value, int), value >= 0)
print(values[1] == values[2])
print(values[3] == values[4])
print(values[6] < values[7], values[7] - values[6])"#,
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_floats",
            name: "grammar-token-floats-method",
            source: r#"values = [
    3.14,
    314.,
    0.314,
    000.314,
    .314,
    3e14,
    3E14,
    3e-14,
    3e+14,
    3.e14,
    .3e14,
    3.1e4,
]
print(len(values))
for value in values:
    print(repr(value), isinstance(value, float), value == value)
print(values[2] == values[3] == values[4])
print(values[5] == values[6] == values[8] == values[9])
print(values[7] < 1, values[10] == 30000000000000.0, values[11] == 31000.0)"#,
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_float_exponent_tokenization",
            name: "grammar-token-float-exponent-tokenization-method",
            source: "print(1 if 1else 0)\nprint(1 if 0else 0)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_underscore_literals",
            name: "grammar-token-underscore-literals-method",
            source: r#"valid_literals = [
    '0_0_0',
    '4_2',
    '1_0000_0000',
    '0b1001_0100',
    '0xffff_ffff',
    '0o5_7_7',
    '1_00_00.5',
    '1_00_00.5e5',
    '1_00_00e5_1',
    '1e1_0',
    '.1_4',
    '.1_4e1',
    '0b_0',
    '0x_f',
    '0o_5',
    '1_00_00j',
    '1_00_00.5j',
    '1_00_00e5_1j',
    '.1_4j',
    '(1_2.5+3_3j)',
    '(.5_6j)',
]
invalid_literals = [
    '0_',
    '42_',
    '1.4j_',
    '0x_',
    '0b1_',
    '0xf_',
    '0o5_',
    '0 if 1_Else 1',
    '0_b0',
    '0_xf',
    '0_o5',
    '0_7',
    '09_99',
    '4_______2',
    '0.1__4',
    '0.1__4j',
    '0b1001__0100',
    '0xffff__ffff',
    '0x___',
    '0o5__77',
    '1e1__0',
    '1e1__0j',
    '1_.4',
    '1_.4j',
    '1._4',
    '1._4j',
    '._5',
    '._5j',
    '1.0e+_1',
    '1.0e+_1j',
    '1.4_j',
    '1.4e5_j',
    '1_e1',
    '1.4_e1',
    '1.4_e1j',
    '1e_1',
    '1.4e_1',
    '1.4e_1j',
    '(1+1.5_j_)',
    '(1+1.5_j)',
]
print(len(valid_literals), len(invalid_literals))
for literal in valid_literals:
    print(literal, eval(literal) == eval(literal.replace('_', '')))
for literal in invalid_literals:
    try:
        eval(literal)
    except SyntaxError as error:
        print(literal, error.__class__.__name__)
    else:
        print(literal, 'accepted')
try:
    eval('_0')
except NameError as error:
    print('_0', error.__class__.__name__)
else:
    print('_0', 'accepted')"#,
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_backslash",
            name: "grammar-token-backslash-line-continuation",
            source: "# Backslash means line continuation.\nx = 1 \\\n+ 1\nprint(x)\n# Backslash does not continue comments. \\\ny = 0\nprint(y)",
        },
        DiffCase {
            origin: "Lib/test/test_long.py::test_bit_length",
            name: "integer-bit-length-method",
            source: "for value in [0, 1, -1, 2 ** 63, 2 ** 234, -(2 ** 234) - 1]:\n    print(value.bit_length())\nprint(True.bit_length())",
        },
        DiffCase {
            origin: "Lib/test/test_long.py::test_as_integer_ratio and integer component attrs",
            name: "integer-ratio-and-components",
            source: "for value in [0, -2, 2 ** 80, True]:\n    print(value.numerator, value.denominator, value.real, value.imag, value.conjugate(), value.as_integer_ratio())",
        },
        DiffCase {
            origin: "Lib/test/test_float.py::test_is_integer and ::test_floatasratio",
            name: "float-ratio-and-components",
            source: "for value in [0.875, -0.875, 0.0, 11.5, 2.1, -2.1, -2100.0]:\n    print(value.real, value.imag, value.conjugate(), value.is_integer(), value.as_integer_ratio())",
        },
        DiffCase {
            origin: "Lib/test/test_float.py::HexFloatTestCase public hex/fromhex behavior",
            name: "float-hex-and-fromhex",
            source: "import math\nfor text in ['inf', '-INF', 'nan', '-NaN', '1.0', '0x.1p4', '0x1.921fb54442d18p1', '0x0.0000000000001p-1022', '0x3p-1076', '0x0.fffffffffffffcp0']:\n    value = float.fromhex(text)\n    if value != value:\n        print(text, 'nan', math.copysign(1.0, value))\n    else:\n        print(text, value.hex(), math.copysign(1.0, value))\nprint((1.5).hex(), float.hex(1.5), (-0.0).hex())\nfor text in ['infi', '0x.p0', '0x1p+', '0x1p1024']:\n    try:\n        float.fromhex(text)\n    except (ValueError, OverflowError) as error:\n        print(text, error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_float.py::HexFloatTestCase::test_from_hex accepted spellings",
            name: "float-fromhex-accepted-variants",
            source: concat!(
                "import math\n",
                "INF = float('inf')\n",
                "NAN = float('nan')\n",
                "def identical(x, y):\n",
                "    if x != x or y != y:\n",
                "        return x != x and y != y\n",
                "    if x == y:\n",
                "        if x != 0.0:\n",
                "            return True\n",
                "        return math.copysign(1.0, x) == math.copysign(1.0, y)\n",
                "    return False\n",
                "variant_cases = [\n",
                "    ('inf', INF), ('+Inf', INF), ('-INF', -INF), ('iNf', INF),\n",
                "    ('Infinity', INF), ('+INFINITY', INF), ('-infinity', -INF), ('-iNFiNitY', -INF),\n",
                "    ('nan', NAN), ('+NaN', NAN), ('-NaN', NAN), ('-nAN', NAN),\n",
                "    ('1', 1.0), ('+1', 1.0), ('1.', 1.0), ('1.0', 1.0), ('1.0p0', 1.0),\n",
                "    ('01', 1.0), ('01.', 1.0), ('0x1', 1.0), ('0x1.', 1.0), ('0x1.0', 1.0),\n",
                "    ('+0x1.0', 1.0), ('0x1p0', 1.0), ('0X1p0', 1.0), ('0X1P0', 1.0), ('0x1P0', 1.0),\n",
                "    ('0x1.p0', 1.0), ('0x1.0p0', 1.0), ('0x.1p4', 1.0), ('0x.1p04', 1.0),\n",
                "    ('0x.1p004', 1.0), ('0x1p+0', 1.0), ('0x1P-0', 1.0), ('+0x1p0', 1.0),\n",
                "    ('0x01p0', 1.0), ('0x1p00', 1.0), (' 0x1p0 ', 1.0), ('\\n 0x1p0', 1.0),\n",
                "    ('0x1p0 \\t', 1.0), ('0xap0', 10.0), ('0xAp0', 10.0), ('0xaP0', 10.0),\n",
                "    ('0xAP0', 10.0), ('0xbep0', 190.0), ('0xBep0', 190.0), ('0xbEp0', 190.0),\n",
                "    ('0XBE0P-4', 190.0), ('0xBEp0', 190.0), ('0xB.Ep4', 190.0), ('0x.BEp8', 190.0),\n",
                "    ('0x.0BEp12', 190.0),\n",
                "]\n",
                "ok = True\n",
                "for text, expected in variant_cases:\n",
                "    ok = ok and identical(float.fromhex(text), expected)\n",
                "print('variants', ok, len(variant_cases), float.fromhex('0x.BEp8').hex(), float.fromhex('-iNFiNitY'))\n",
                "pi = float.fromhex('0x1.921fb54442d18p1')\n",
                "pi_spellings = [\n",
                "    '0x.006487ed5110b46p11', '0x.00c90fdaa22168cp10', '0x.01921fb54442d18p9',\n",
                "    '0x.03243f6a8885a3p8', '0x.06487ed5110b46p7', '0x.0c90fdaa22168cp6',\n",
                "    '0x.1921fb54442d18p5', '0x.3243f6a8885a3p4', '0x.6487ed5110b46p3',\n",
                "    '0x.c90fdaa22168cp2', '0x1.921fb54442d18p1', '0x3.243f6a8885a3p0',\n",
                "    '0x6.487ed5110b46p-1', '0xc.90fdaa22168cp-2', '0x19.21fb54442d18p-3',\n",
                "    '0x32.43f6a8885a3p-4', '0x64.87ed5110b46p-5', '0xc9.0fdaa22168cp-6',\n",
                "    '0x192.1fb54442d18p-7', '0x324.3f6a8885a3p-8', '0x648.7ed5110b46p-9',\n",
                "    '0xc90.fdaa22168cp-10', '0x1921.fb54442d18p-11', '0x1921fb54442d1.8p-47',\n",
                "    '0x3243f6a8885a3p-48', '0x6487ed5110b46p-49', '0xc90fdaa22168cp-50',\n",
                "    '0x1921fb54442d18p-51', '0x3243f6a8885a30p-52', '0x6487ed5110b460p-53',\n",
                "    '0xc90fdaa22168c0p-54', '0x1921fb54442d180p-55',\n",
                "]\n",
                "ok = True\n",
                "for text in pi_spellings:\n",
                "    ok = ok and identical(float.fromhex(text), pi)\n",
                "print('pi-shifts', ok, len(pi_spellings), pi.hex(), float.fromhex(pi_spellings[0]).hex(), float.fromhex(pi_spellings[-1]).hex())",
            ),
        },
        DiffCase {
            origin: "Lib/test/test_float.py::HexFloatTestCase::test_from_hex overflow, zero, and underflow groups",
            name: "float-fromhex-overflow-zero-underflow",
            source: r#"import math
MAX = float.fromhex('0x.fffffffffffff8p+1024')
TINY = float.fromhex('0x0.0000000000001p-1022')

def identical(x, y):
    if x != x or y != y:
        return x != x and y != y
    if x == y:
        if x != 0.0:
            return True
        return math.copysign(1.0, x) == math.copysign(1.0, y)
    return False

overflow_inputs = [
    '-0x1p1024', '0x1p+1025', '+0X1p1030', '-0x1p+1100', '0X1p123456789123456789',
    '+0X.8p+1025', '+0x0.8p1025', '-0x0.4p1026', '0X2p+1023', '0x2.p1023',
    '-0x2.0p+1023', '+0X4p+1022', '0x1.ffffffffffffffp+1023',
    '-0X1.fffffffffffff9p1023', '0X1.fffffffffffff8p1023', '+0x3.fffffffffffffp1022',
    '0x3fffffffffffffp+970', '0x10000000000000000p960', '-0Xffffffffffffffffp960',
]
ok = True
overflows = 0
wrong = []
for index, text in enumerate(overflow_inputs):
    try:
        float.fromhex(text)
    except OverflowError:
        overflows += 1
    except Exception as error:
        ok = False
        wrong.append((index, error.__class__.__name__))
    else:
        ok = False
        wrong.append((index, 'accepted'))
print('overflow', ok, overflows, len(overflow_inputs), len(wrong))

round_to_max = [
    ('+0x1.fffffffffffffp+1023', MAX),
    ('-0X1.fffffffffffff7p1023', -MAX),
    ('0X1.fffffffffffff7fffffffffffffp1023', MAX),
]
ok = True
for text, expected in round_to_max:
    ok = ok and identical(float.fromhex(text), expected)
print('round-to-max', ok, len(round_to_max), float.fromhex(round_to_max[2][0]).hex())

zero_cases = [
    ('0x0p0', 0.0), ('0x0p1000', 0.0), ('-0x0p1023', -0.0), ('0X0p1024', 0.0),
    ('-0x0p1025', -0.0), ('0X0p2000', 0.0), ('0x0p123456789123456789', 0.0),
    ('-0X0p-0', -0.0), ('-0X0p-1000', -0.0), ('0x0p-1023', 0.0),
    ('-0X0p-1024', -0.0), ('-0x0p-1025', -0.0), ('-0x0p-1072', -0.0),
    ('0X0p-1073', 0.0), ('-0x0p-1074', -0.0), ('0x0p-1075', 0.0),
    ('0X0p-1076', 0.0), ('-0X0p-2000', -0.0), ('-0x0p-123456789123456789', -0.0),
]
ok = True
for text, expected in zero_cases:
    ok = ok and identical(float.fromhex(text), expected)
print('zeros', ok, len(zero_cases), float.fromhex(zero_cases[2][0]).hex(), float.fromhex(zero_cases[-1][0]).hex())

underflow_cases = [
    ('0X1p-1075', 0.0), ('-0X1p-1075', -0.0), ('-0x1p-123456789123456789', -0.0),
    ('0x1.00000000000000001p-1075', TINY), ('-0x1.1p-1075', -TINY),
    ('0x1.fffffffffffffffffp-1075', TINY),
]
ok = True
for text, expected in underflow_cases:
    ok = ok and identical(float.fromhex(text), expected)
print('underflow', ok, len(underflow_cases), float.fromhex(underflow_cases[3][0]).hex(), float.fromhex(underflow_cases[4][0]).hex())"#,
        },
        DiffCase {
            origin: "Lib/test/test_float.py::HexFloatTestCase::test_from_hex round-half-even groups",
            name: "float-fromhex-rounding-boundaries",
            source: r#"import math
MIN = float.fromhex('0x1p-1022')
TINY = float.fromhex('0x0.0000000000001p-1022')
EPS = float.fromhex('0x0.0000000000001p0')

def identical(x, y):
    if x != x or y != y:
        return x != x and y != y
    if x == y:
        if x != 0.0:
            return True
        return math.copysign(1.0, x) == math.copysign(1.0, y)
    return False

def check_group(name, cases):
    ok = True
    bad = []
    for index, (text, expected) in enumerate(cases):
        value = float.fromhex(text)
        if not identical(value, expected):
            ok = False
            bad.append((index, value.hex(), expected.hex()))
    print(name, ok, len(cases), float.fromhex(cases[0][0]).hex(), float.fromhex(cases[-1][0]).hex(), len(bad))
    if bad:
        print(name + '-bad', bad[:3])

near_zero = [
    ('0x1p-1076', 0.0), ('0X2p-1076', 0.0), ('0X3p-1076', TINY),
    ('0x4p-1076', TINY), ('0X5p-1076', TINY), ('0X6p-1076', 2*TINY),
    ('0x7p-1076', 2*TINY), ('0X8p-1076', 2*TINY), ('0X9p-1076', 2*TINY),
    ('0xap-1076', 2*TINY), ('0Xbp-1076', 3*TINY), ('0xcp-1076', 3*TINY),
    ('0Xdp-1076', 3*TINY), ('0Xep-1076', 4*TINY), ('0xfp-1076', 4*TINY),
    ('0x10p-1076', 4*TINY), ('-0x1p-1076', -0.0), ('-0X2p-1076', -0.0),
    ('-0x3p-1076', -TINY), ('-0X4p-1076', -TINY), ('-0x5p-1076', -TINY),
    ('-0x6p-1076', -2*TINY), ('-0X7p-1076', -2*TINY), ('-0X8p-1076', -2*TINY),
    ('-0X9p-1076', -2*TINY), ('-0Xap-1076', -2*TINY), ('-0xbp-1076', -3*TINY),
    ('-0xcp-1076', -3*TINY), ('-0Xdp-1076', -3*TINY), ('-0xep-1076', -4*TINY),
    ('-0Xfp-1076', -4*TINY), ('-0X10p-1076', -4*TINY),
]
near_min = [
    ('0x0.ffffffffffffd6p-1022', MIN-3*TINY), ('0x0.ffffffffffffd8p-1022', MIN-2*TINY),
    ('0x0.ffffffffffffdap-1022', MIN-2*TINY), ('0x0.ffffffffffffdcp-1022', MIN-2*TINY),
    ('0x0.ffffffffffffdep-1022', MIN-2*TINY), ('0x0.ffffffffffffe0p-1022', MIN-2*TINY),
    ('0x0.ffffffffffffe2p-1022', MIN-2*TINY), ('0x0.ffffffffffffe4p-1022', MIN-2*TINY),
    ('0x0.ffffffffffffe6p-1022', MIN-2*TINY), ('0x0.ffffffffffffe8p-1022', MIN-2*TINY),
    ('0x0.ffffffffffffeap-1022', MIN-TINY), ('0x0.ffffffffffffecp-1022', MIN-TINY),
    ('0x0.ffffffffffffeep-1022', MIN-TINY), ('0x0.fffffffffffff0p-1022', MIN-TINY),
    ('0x0.fffffffffffff2p-1022', MIN-TINY), ('0x0.fffffffffffff4p-1022', MIN-TINY),
    ('0x0.fffffffffffff6p-1022', MIN-TINY), ('0x0.fffffffffffff8p-1022', MIN),
    ('0x0.fffffffffffffap-1022', MIN), ('0x0.fffffffffffffcp-1022', MIN),
    ('0x0.fffffffffffffep-1022', MIN), ('0x1.00000000000000p-1022', MIN),
    ('0x1.00000000000002p-1022', MIN), ('0x1.00000000000004p-1022', MIN),
    ('0x1.00000000000006p-1022', MIN), ('0x1.00000000000008p-1022', MIN),
    ('0x1.0000000000000ap-1022', MIN+TINY), ('0x1.0000000000000cp-1022', MIN+TINY),
    ('0x1.0000000000000ep-1022', MIN+TINY), ('0x1.00000000000010p-1022', MIN+TINY),
    ('0x1.00000000000012p-1022', MIN+TINY), ('0x1.00000000000014p-1022', MIN+TINY),
    ('0x1.00000000000016p-1022', MIN+TINY), ('0x1.00000000000018p-1022', MIN+2*TINY),
]
near_one = [
    ('0x0.fffffffffffff0p0', 1.0-EPS), ('0x0.fffffffffffff1p0', 1.0-EPS),
    ('0X0.fffffffffffff2p0', 1.0-EPS), ('0x0.fffffffffffff3p0', 1.0-EPS),
    ('0X0.fffffffffffff4p0', 1.0-EPS), ('0X0.fffffffffffff5p0', 1.0-EPS/2),
    ('0X0.fffffffffffff6p0', 1.0-EPS/2), ('0x0.fffffffffffff7p0', 1.0-EPS/2),
    ('0x0.fffffffffffff8p0', 1.0-EPS/2), ('0X0.fffffffffffff9p0', 1.0-EPS/2),
    ('0X0.fffffffffffffap0', 1.0-EPS/2), ('0x0.fffffffffffffbp0', 1.0-EPS/2),
    ('0X0.fffffffffffffcp0', 1.0), ('0x0.fffffffffffffdp0', 1.0),
    ('0X0.fffffffffffffep0', 1.0), ('0x0.ffffffffffffffp0', 1.0),
    ('0X1.00000000000000p0', 1.0), ('0X1.00000000000001p0', 1.0),
    ('0x1.00000000000002p0', 1.0), ('0X1.00000000000003p0', 1.0),
    ('0x1.00000000000004p0', 1.0), ('0X1.00000000000005p0', 1.0),
    ('0X1.00000000000006p0', 1.0), ('0X1.00000000000007p0', 1.0),
    ('0x1.00000000000007ffffffffffffffffffffp0', 1.0), ('0x1.00000000000008p0', 1.0),
    ('0x1.00000000000008000000000000000001p0', 1+EPS), ('0X1.00000000000009p0', 1.0+EPS),
    ('0x1.0000000000000ap0', 1.0+EPS), ('0x1.0000000000000bp0', 1.0+EPS),
    ('0X1.0000000000000cp0', 1.0+EPS), ('0x1.0000000000000dp0', 1.0+EPS),
    ('0x1.0000000000000ep0', 1.0+EPS), ('0X1.0000000000000fp0', 1.0+EPS),
    ('0x1.00000000000010p0', 1.0+EPS), ('0X1.00000000000011p0', 1.0+EPS),
    ('0x1.00000000000012p0', 1.0+EPS), ('0X1.00000000000013p0', 1.0+EPS),
    ('0X1.00000000000014p0', 1.0+EPS), ('0x1.00000000000015p0', 1.0+EPS),
    ('0x1.00000000000016p0', 1.0+EPS), ('0X1.00000000000017p0', 1.0+EPS),
    ('0x1.00000000000017ffffffffffffffffffffp0', 1.0+EPS), ('0x1.00000000000018p0', 1.0+2*EPS),
    ('0X1.00000000000018000000000000000001p0', 1.0+2*EPS), ('0x1.00000000000019p0', 1.0+2*EPS),
    ('0X1.0000000000001ap0', 1.0+2*EPS), ('0X1.0000000000001bp0', 1.0+2*EPS),
    ('0x1.0000000000001cp0', 1.0+2*EPS), ('0x1.0000000000001dp0', 1.0+2*EPS),
    ('0x1.0000000000001ep0', 1.0+2*EPS), ('0X1.0000000000001fp0', 1.0+2*EPS),
    ('0x1.00000000000020p0', 1.0+2*EPS),
]
check_group('near-zero', near_zero)
check_group('near-min', near_min)
check_group('near-one', near_one)"#,
        },
        DiffCase {
            origin: "Lib/test/test_float.py::HexFloatTestCase::test_invalid_inputs",
            name: "float-hex-fromhex-invalid-inputs",
            source: concat!(
                "invalid_inputs = [\n",
                "    'infi',\n",
                "    '-Infinit',\n",
                "    '++inf',\n",
                "    '-+Inf',\n",
                "    '--nan',\n",
                "    '+-NaN',\n",
                "    'snan',\n",
                "    'NaNs',\n",
                "    'nna',\n",
                "    'an',\n",
                "    'nf',\n",
                "    'nfinity',\n",
                "    'inity',\n",
                "    'iinity',\n",
                "    '0xnan',\n",
                "    '',\n",
                "    ' ',\n",
                "    'x1.0p0',\n",
                "    '0xX1.0p0',\n",
                "    '+ 0x1.0p0',\n",
                "    '- 0x1.0p0',\n",
                "    '0 x1.0p0',\n",
                "    '0x 1.0p0',\n",
                "    '0x1 2.0p0',\n",
                "    '+0x1 .0p0',\n",
                "    '0x1. 0p0',\n",
                "    '-0x1.0 1p0',\n",
                "    '-0x1.0 p0',\n",
                "    '+0x1.0p +0',\n",
                "    '0x1.0p -0',\n",
                "    '0x1.0p 0',\n",
                "    '+0x1.0p+ 0',\n",
                "    '-0x1.0p- 0',\n",
                "    '++0x1.0p-0',\n",
                "    '--0x1.0p0',\n",
                "    '+-0x1.0p+0',\n",
                "    '-+0x1.0p0',\n",
                "    '0x1.0p++0',\n",
                "    '+0x1.0p+-0',\n",
                "    '-0x1.0p-+0',\n",
                "    '0x1.0p--0',\n",
                "    '0x1.0.p0',\n",
                "    '0x.p0',\n",
                "    '0x1,p0',\n",
                "    '0x1pa',\n",
                "    '0x1p\\uff10',\n",
                "    '\\uff10x1p0',\n",
                "    '0x\\uff11p0',\n",
                "    '0x1.\\uff10p0',\n",
                "    '0x1p0 \\n 0x2p0',\n",
                "    '0x1p0\\0 0x1p0',\n",
                "]\n",
                "ok = True\n",
                "value_errors = 0\n",
                "accepted = []\n",
                "wrong = []\n",
                "for index, text in enumerate(invalid_inputs):\n",
                "    try:\n",
                "        result = float.fromhex(text)\n",
                "    except ValueError:\n",
                "        value_errors += 1\n",
                "    except Exception as error:\n",
                "        ok = False\n",
                "        wrong.append((index, error.__class__.__name__))\n",
                "    else:\n",
                "        ok = False\n",
                "        accepted.append((index, repr(result)))\n",
                "print('invalid-inputs', ok, value_errors, len(invalid_inputs), len(accepted), len(wrong))\n",
                "print('sample', repr(invalid_inputs[0]), repr(invalid_inputs[15]), repr(invalid_inputs[-1]))",
            ),
        },
        DiffCase {
            origin: "Lib/test/test_float.py::HexFloatTestCase::test_ends and ::test_whitespace",
            name: "float-hex-fromhex-ends-whitespace",
            source: concat!(
                "import math\n",
                "INF = float('inf')\n",
                "NAN = float('nan')\n",
                "MAX = float.fromhex('0x.fffffffffffff8p+1024')\n",
                "MIN = float.fromhex('0x1p-1022')\n",
                "TINY = float.fromhex('0x0.0000000000001p-1022')\n",
                "EPS = float.fromhex('0x0.0000000000001p0')\n",
                "def identical(x, y):\n",
                "    if x != x or y != y:\n",
                "        return x != x and y != y\n",
                "    if x == y:\n",
                "        if x != 0.0:\n",
                "            return True\n",
                "        return math.copysign(1.0, x) == math.copysign(1.0, y)\n",
                "    return False\n",
                "ends = [\n",
                "    ('MIN', MIN, math.ldexp(1.0, -1022)),\n",
                "    ('TINY', TINY, math.ldexp(1.0, -1074)),\n",
                "    ('EPS', EPS, math.ldexp(1.0, -52)),\n",
                "    ('MAX', MAX, 2.0 * (math.ldexp(1.0, 1023) - math.ldexp(1.0, 970))),\n",
                "]\n",
                "for name, actual, expected in ends:\n",
                "    print(name, actual.hex(), expected.hex(), identical(actual, expected))\n",
                "value_pairs = [('inf', INF), ('-Infinity', -INF), ('nan', NAN), ('1.0', 1.0), ('-0x.2', -0.125), ('-0.0', -0.0)]\n",
                "whitespace = ['', ' ', '\\t', '\\n', '\\n \\t', '\\f', '\\v', '\\r']\n",
                "ok = True\n",
                "count = 0\n",
                "for text, expected in value_pairs:\n",
                "    for lead in whitespace:\n",
                "        for trail in whitespace:\n",
                "            got = float.fromhex(lead + text + trail)\n",
                "            ok = ok and identical(got, expected)\n",
                "            count += 1\n",
                "print('whitespace', ok, count)\n",
                "for text in ['\\f-0.0\\v', '\\rnan\\n', '\\n \\t-0x.2\\f']:\n",
                "    value = float.fromhex(text)\n",
                "    if value != value:\n",
                "        print(repr(text), 'nan')\n",
                "    else:\n",
                "        print(repr(text), value.hex(), math.copysign(1.0, value))",
            ),
        },
        DiffCase {
            origin: "Lib/test/test_float.py::HexFloatTestCase::test_roundtrip deterministic sweep",
            name: "float-hex-fromhex-roundtrip-matrix",
            source: concat!(
                "import math\n",
                "NAN = float('nan')\n",
                "INF = float('inf')\n",
                "MAX = float.fromhex('0x1.fffffffffffffp+1023')\n",
                "MIN = float.fromhex('0x1p-1022')\n",
                "TINY = float.fromhex('0x0.0000000000001p-1022')\n",
                "def identical(x, y):\n",
                "    if x != x or y != y:\n",
                "        return x != x and y != y\n",
                "    if x == y:\n",
                "        if x != 0.0:\n",
                "            return True\n",
                "        return math.copysign(1.0, x) == math.copysign(1.0, y)\n",
                "    return False\n",
                "def roundtrip(x):\n",
                "    return float.fromhex(x.hex())\n",
                "for x in [NAN, INF, MAX, MIN, MIN - TINY, TINY, 0.0]:\n",
                "    print(x.hex(), identical(x, roundtrip(x)), (-x).hex(), identical(-x, roundtrip(-x)))\n",
                "ok = True\n",
                "count = 0\n",
                "skipped = 0\n",
                "for i in range(10000):\n",
                "    exponent = ((i * 1543 + 17) % 2400) - 1200\n",
                "    mantissa_bits = (i * 6364136223846793005 + 1442695040888963407) % (2 ** 53)\n",
                "    mantissa = mantissa_bits / float(2 ** 53)\n",
                "    sign = -1.0 if ((i * 1103515245 + 12345) % 2) else 1.0\n",
                "    try:\n",
                "        x = sign * math.ldexp(mantissa, exponent)\n",
                "    except OverflowError:\n",
                "        skipped += 1\n",
                "    else:\n",
                "        count += 1\n",
                "        if not identical(x, roundtrip(x)):\n",
                "            print('mismatch', i, exponent, mantissa_bits, x.hex(), roundtrip(x).hex())\n",
                "            ok = False\n",
                "print('deterministic-sweep', ok, count, skipped)\n",
                "print(roundtrip(-0.0).hex(), identical(-0.0, roundtrip(-0.0)))",
            ),
        },
        DiffCase {
            origin: "Lib/test/test_float.py::HexFloatTestCase::test_subclass",
            name: "float-fromhex-subclass-construction",
            source: "class F(float):\n    def __new__(cls, value):\n        return float.__new__(cls, value + 1)\nf = F.fromhex((1.5).hex())\nprint(type(f) is F, f, f == 2.5, isinstance(f, float), issubclass(F, float), f.hex())\nprint(float.__new__(F, 1.5), type(float.__new__(F, 1.5)) is F)\nclass F2(float):\n    def __init__(self, value):\n        self.foo = 'bar'\nf = F2.fromhex((1.5).hex())\nprint(type(f) is F2, f, f == 1.5, getattr(f, 'foo', 'none'), bool(F2(0.0)), bool(F2(0.25)))",
        },
        DiffCase {
            origin: "Lib/test/test_bool.py::test_math",
            name: "bool-arithmetic-and-bitwise",
            source: "print(False + 2, True + 2, True - False, False - True)\nprint(True * 1, False * 1, True % 2)\nprint(True & False, True | False, True ^ True)",
        },
        DiffCase {
            origin: "Lib/test/test_tokenize.py string literal explicit line joining",
            name: "string-line-continuations",
            source: concat!(
                "print('a\\\n",
                "b')\n",
                "print(repr(r'a\\\n",
                "b'))\n",
                "print(b'a\\\n",
                "b')\n",
                "print(f'a\\\n",
                "{1}b')",
            ),
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_string_literals",
            name: "grammar-token-string-literals-method",
            source: r###"x = ''
y = ""
print(len(x) == 0 and x == y)
x = '\''
y = "'"
print(len(x) == 1 and x == y and ord(x) == 39)
x = '"'
y = "\""
print(len(x) == 1 and x == y and ord(x) == 34)
x = "doesn't \"shrink\" does it"
y = 'doesn\'t "shrink" does it'
print(len(x) == 24 and x == y)
x = "does \"shrink\" doesn't it"
y = 'does "shrink" doesn\'t it'
print(len(x) == 24 and x == y)
x = """
The "quick"
brown fox
jumps over
the 'lazy' dog.
"""
y = '\nThe "quick"\nbrown fox\njumps over\nthe \'lazy\' dog.\n'
print(x == y)
y = '''
The "quick"
brown fox
jumps over
the 'lazy' dog.
'''
print(x == y)
y = "\n\
The \"quick\"\n\
brown fox\n\
jumps over\n\
the 'lazy' dog.\n\
"
print(x == y)
y = '\n\
The \"quick\"\n\
brown fox\n\
jumps over\n\
the \'lazy\' dog.\n\
'
print(x == y)"###,
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_string_prefixes",
            name: "grammar-token-string-prefixes-method",
            source: r#"sources = [
    "u'abc'",
    "r'abc\t'",
    "rf'abc\a {1 + 1}'",
    "fr'abc\a {1 + 1}'",
]
for source in sources:
    parsed = eval(source)
    print(type(parsed) is str, len(parsed) > 0, repr(parsed))"#,
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bytes_prefixes",
            name: "grammar-token-bytes-prefixes-method",
            source: r#"sources = [
    "b'abc'",
    "br'abc\t'",
    "rb'abc\a'",
]
for source in sources:
    parsed = eval(source)
    print(type(parsed) is bytes, len(parsed) > 0, repr(parsed))"#,
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_ellipsis",
            name: "grammar-token-ellipsis-method",
            source: r#"x = ...
print(x is Ellipsis)
try:
    eval(".. .")
except SyntaxError as error:
    print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_max_level",
            name: "grammar-token-max-level-method",
            source: r#"MAXLEVEL = 200
result = eval("(" * MAXLEVEL + ")" * MAXLEVEL)
print(result == (), repr(result))
try:
    eval("(" * (MAXLEVEL + 1) + ")" * (MAXLEVEL + 1))
except SyntaxError as error:
    print(str(error).startswith("too many nested parentheses"))"#,
        },
        DiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_curly_brace_after_primary_raises_immediately",
            name: "syntax-error-curly-brace-after-primary-single-mode",
            source: r#"try:
    compile("f{}", "<testcase>", "single")
except SyntaxError as error:
    print("invalid syntax" in str(error))"#,
        },
        DiffCase {
            origin: "Lib/test/test_tokenize.py::test_continuation",
            name: "implicit-continuation-keeps-semantics",
            source: "a = (3,4, \n5,6)\ny = [3, 4,\n5]\nz = {'a': 5,\n'b':15, 'c':True}\nx = len(y) + 5 - a[\n3] - a[2]\n+ len(z) - z[\n'b']\nprint(x)",
        },
        DiffCase {
            origin: "Lib/test/test_tokenize.py::test_backslash_continuation",
            name: "comment-backslash-does-not-continue",
            source: "# Comment \\\nx = 0\nprint(x)",
        },
        DiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_empty_line_after_linecont",
            name: "syntax-empty-line-after-line-continuation",
            source: "pass\n        \\\n\npass\nprint(\"ok\")",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_startswith and ::test_endswith",
            name: "string-startswith-endswith",
            source: "print('hello'.startswith('he'), 'hello'.startswith('ello', 1), 'hello'.startswith(('lo', 'he'), 0, -1))\nprint('helloworld'.startswith('lowo', 3, 7), 'helloworld'.startswith('lowo', 3, 6))\nprint('hello'.endswith('lo'), 'helloworld'.endswith('worl', -5, -1), 'hello'.endswith(('he', 'hell'), 0, 4))\nprint(''.startswith('', 1, 0), ''.endswith('', 1, 0))",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_find, ::test_rfind, ::test_index, and ::test_rindex",
            name: "string-find-index",
            source: "print('abcdefghiabc'.find('abc'), 'abcdefghiabc'.find('abc', 1), 'abc'.find('', 4))\nprint('abcdefghiabc'.rfind(''), 'rrarrrrrrrrra'.rfind('a', None, 6), '<......м...'.rfind('<'))\ntry:\n    print('abcdefghi'.index('ghi', 8))\nexcept ValueError as error:\n    print(error.__class__.__name__)\ntry:\n    print('defghiabc'.rindex('abc', 0, -1))\nexcept ValueError as error:\n    print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_count, ::test_lower, and ::test_upper",
            name: "string-count-lower-upper",
            source: "print('aaa'.count('a'), 'aaa'.count('aa'), 'aaa'.count('a', 0, -1))\nprint('aaa'.count('', 1), ''.count(''), ''.count('', 1, 1))\nprint('HeLLo'.lower(), 'Straße'.upper(), 'MIXED 123'.lower())",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_capitalize, ::test_title, ::test_swapcase and Lib/test/test_str.py::test_casefold",
            name: "string-capitalize-title-swapcase-casefold",
            source: "print(' hello '.capitalize(), 'hello '.capitalize(), 'AaAa'.capitalize())\nprint('fOrMaT thIs aS titLe String'.title())\nprint('fOrMaT,thIs-aS*titLe;String'.title(), 'getInt'.title())\nprint('HeLLo cOmpUteRs'.swapcase())\nprint('ß'.casefold(), 'ﬁ'.casefold(), 'Σ'.casefold(), 'AͅΣ'.casefold(), 'µ'.casefold())\nprint('ﬁnnish'.capitalize() == 'Finnish', 'ﬁ'.swapcase() == 'FI', 'ß'.swapcase() == 'SS')\nfor expr in [lambda: 'hello'.capitalize(42), lambda: 'hello'.title(42), lambda: 'hello'.swapcase(42), lambda: 'hello'.casefold(42)]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_is* methods and Lib/test/test_str.py Unicode decimal/numeric predicate checks",
            name: "string-predicate-methods",
            source: "print(''.islower(), 'abc'.islower(), 'aBc'.islower(), 'abc\\n'.islower())\nprint(''.isupper(), 'ABC'.isupper(), 'AbC'.isupper(), 'ABC\\n'.isupper())\nprint('A Titlecased Line'.istitle(), 'Not a capitalized String'.istitle(), 'NOT'.istitle())\nprint(' \\t\\r\\n'.isspace(), ' \\t\\r\\na'.isspace(), 'abc'.isalpha(), 'aBc123'.isalpha(), 'a1b3c'.isalnum())\nprint('0'.isdigit(), '\\u2460'.isdigit(), '\\xbc'.isdigit(), '\\u0660'.isdigit(), '\\U0001d7f6'.isdigit())\nprint('0'.isdecimal(), '\\u2460'.isdecimal(), '\\xbc'.isdecimal(), '\\u0660'.isdecimal(), '\\U00011066'.isdecimal())\nprint('0'.isnumeric(), '\\u2460'.isnumeric(), '\\xbc'.isnumeric(), '\\u0660'.isnumeric(), '12a'.isnumeric())\nprint(''.isascii(), '\\x00'.isascii(), '\\x80'.isascii(), '\\u20ac'.isascii())\nfor expr in [lambda: 'abc'.islower(42), lambda: 'abc'.isupper(42), lambda: 'abc'.istitle(42), lambda: 'abc'.isspace(42), lambda: 'abc'.isalpha(42), lambda: 'abc'.isalnum(42), lambda: 'abc'.isdigit(42), lambda: 'abc'.isdecimal(42), lambda: 'abc'.isnumeric(42), lambda: 'abc'.isascii(42)]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_str.py::test_isidentifier and ::test_isprintable",
            name: "string-identifier-printable-predicates",
            source: "print('a'.isidentifier(), 'Z'.isidentifier(), '_'.isidentifier(), 'b0'.isidentifier(), '\\xb5'.isidentifier())\nprint('\\U0001d518\\U0001d52b\\U0001d526\\U0001d520\\U0001d52c\\U0001d521\\U0001d522'.isidentifier(), 'def'.isidentifier(), ''.isidentifier(), '0'.isidentifier())\nprint(''.isprintable(), ' '.isprintable(), 'abcdefg\\n'.isprintable(), '\\u0374'.isprintable(), '\\u0378'.isprintable())\nprint('\\U0001f46f'.isprintable(), '\\U000e0020'.isprintable())\nfor expr in [lambda: 'abc'.isidentifier(42), lambda: 'abc'.isprintable(42)]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_splitlines",
            name: "string-splitlines",
            source: "print('abc\\ndef\\n\\rghi'.splitlines())\nprint('abc\\ndef\\r\\nghi\\n\\r'.splitlines())\nprint('\\nabc\\ndef\\r\\nghi\\n\\r'.splitlines(True))\nprint(''.splitlines(), 'one'.splitlines(), 'one\\n'.splitlines(), '\\n'.splitlines(), '\\r\\n'.splitlines())\nprint('a\\vb\\fc\\x1cd\\x1ee\\x85f\\u2028g\\u2029h'.splitlines())\nprint('a\\vb\\fc\\x1cd\\x1ee\\x85f\\u2028g\\u2029h'.splitlines(True) == ['a\\v', 'b\\f', 'c\\x1c', 'd\\x1e', 'e\\x85', 'f\\u2028', 'g\\u2029', 'h'])\nfor expr in [lambda: 'abc'.splitlines(True, False), lambda: 'abc'.splitlines(extra=True)]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_expandtabs",
            name: "string-expandtabs",
            source: "print(repr('abc\\rab\\tdef\\ng\\thi'.expandtabs()))\nprint(repr('abc\\rab\\tdef\\ng\\thi'.expandtabs(4)))\nprint(repr('abc\\r\\nab\\tdef\\ng\\thi'.expandtabs(tabsize=4)))\nprint(repr('abc\\r\\nab\\r\\ndef\\ng\\r\\nhi'.expandtabs(4)))\nprint(repr(' \\ta\\n\\tb'.expandtabs(1)))\nprint(repr('ab\\tc'.expandtabs(-1)), repr('ab\\tc'.expandtabs(0)), repr('ab\\tc'.expandtabs(1)), repr('ab\\tc'.expandtabs(2)))\nfor expr in [lambda: 'hello'.expandtabs(42, 42), lambda: 'hello'.expandtabs(None), lambda: 'hello'.expandtabs(size=4), lambda: 'hello'.expandtabs(2147483648)]:\n    try:\n        expr()\n    except (TypeError, OverflowError) as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_replace",
            name: "string-replace",
            source: "print(repr(''.replace('', 'A')), 'AA'.replace('', '*-', 2), 'abc'.replace('', '-', 3))\nprint(repr('AAA'.replace('A', '')), 'ABACADA'.replace('A', '', 2), 'here and there'.replace('the', '', 1))\nprint('Who goes there?'.replace('o', 'O', 2), 'This is a tissue'.replace('is', '**', 2))\nprint('Reykjavik'.replace('k', 'KK'), 'spam, spam, eggs'.replace('spam', 'ham', 1), 'bobobob'.replace('bobob', 'bob'))",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_removeprefix and ::test_removesuffix",
            name: "string-remove-affix",
            source: "print('spam'.removeprefix('sp'), 'spamspamspam'.removeprefix('spam'))\nprint('spam'.removeprefix('python'), 'spam'.removeprefix('spider'), 'spam'.removeprefix('spam and eggs'))\nprint('spam'.removesuffix('am'), 'spamspamspam'.removesuffix('spam'))\nprint('spam'.removesuffix('python'), 'spam'.removesuffix('blam'), 'spam'.removesuffix('eggs and spam'))\nprint((''.removeprefix(''), ''.removesuffix('abcde'), 'abcde'.removeprefix(''), 'abcde'.removesuffix('abcde')))\nprint('āĀspam𐌁𐌀'.removeprefix('āĀ'), 'āĀspam𐌁𐌀'.removesuffix('𐌁𐌀'))\nfor expr in [lambda: 'hello'.removeprefix(), lambda: 'hello'.removeprefix(42), lambda: 'hello'.removesuffix(), lambda: 'hello'.removesuffix(('lo', 'l'))]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_ljust, ::test_rjust, ::test_center, and ::test_zfill",
            name: "string-alignment-zfill",
            source: "print(repr('abc'.ljust(10)), repr('abc'.ljust(6)), repr('abc'.ljust(3)), repr('abc'.ljust(10, '*')))\nprint(repr('abc'.rjust(10)), repr('abc'.rjust(6)), repr('abc'.rjust(3)), repr('abc'.rjust(10, '*')))\nprint(repr('abc'.center(10)), repr('abc'.center(6)), repr('abc'.center(3)), repr('abc'.center(10, '*')))\nprint('x'.center(3, '\\U0010ffff') == '\\U0010ffffx\\U0010ffff')\nprint('123'.zfill(2), '123'.zfill(3), '123'.zfill(4), '+123'.zfill(5), '-123'.zfill(5), ''.zfill(3))\nfor expr in [lambda: 'abc'.ljust(), lambda: 'abc'.rjust('x'), lambda: 'abc'.center(5, '**'), lambda: '123'.zfill()]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_split and ::test_rsplit",
            name: "string-split-rsplit",
            source: "print('a b c d'.split(), 'a  b  c d'.split(), ''.split())\nprint('a|b|c|d'.split('|', 1), 'a||b||c||d'.split('|', 2), 'a//b//c//d'.split('//', 2))\nprint('a b c d'.rsplit(None, 1), 'a|b|c|d'.rsplit('|', 2), 'a////b////c////d'.rsplit('//', 2))\nprint('a|b|c|d'.split(sep='|'), 'a|b|c|d'.rsplit('|', maxsplit=1))",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_strip_whitespace and ::test_strip",
            name: "string-strip",
            source: "text = ' \\t\\n\\r\\f\\vabc \\t\\n\\r\\f\\v'\nprint('   hello   '.strip(), '   hello   '.lstrip(), '   hello   '.rstrip())\nprint(text.strip(), repr(text.lstrip()), repr(text.rstrip()))\nprint('xyzzyhelloxyzzy'.strip('xyz'), 'xyzzyhelloxyzzy'.lstrip('xyz'), 'xyzzyhelloxyzzy'.rstrip('xyz'))\nprint('mississippi'.strip('mississippi'), 'mississippi'.strip('i'), 'abc'.strip(''))",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_partition and ::test_rpartition",
            name: "string-partition-rpartition",
            source: "S = 'http://www.python.org'\nprint(S.partition('://'), S.partition('?'))\nprint(S.partition('http://'), S.partition('org'))\nprint(S.rpartition('://'), S.rpartition('?'))\nprint(S.rpartition('http://'), S.rpartition('org'))\ntext = 'āĀ' * 3 + 'ĂĂ' + '𐌁𐌀' * 3\nprint(text.partition('ĂĂ'))\nprint(text.rpartition('ĂĂ'))\nfor expr in [lambda: S.partition(''), lambda: S.rpartition(''), lambda: S.partition(None), lambda: S.partition('x', 'y')]:\n    try:\n        expr()\n    except (TypeError, ValueError) as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/string_tests.py::test_join",
            name: "string-join",
            source: "class Sequence:\n    def __init__(self, seq='wxyz'):\n        self.seq = seq\n    def __getitem__(self, i):\n        return self.seq[i]\nclass LiesAboutLengthSeq(Sequence):\n    def __init__(self):\n        self.seq = ['a', 'b', 'c']\n    def __len__(self):\n        return 8\nprint(' '.join(['a', 'b', 'c', 'd']))\nprint(''.join(('a', 'b', 'c', 'd')), ''.join(('', 'b', '', 'd')), ''.join(('a', '', 'c', '')))\nprint(' '.join(Sequence()), ' '.join(LiesAboutLengthSeq()), ''.join(x for x in ['a', 'b', 'c']))\nprint('-'.join(['aa'] * 5))\nfor expr in [lambda: ' '.join(), lambda: ' '.join(None), lambda: ' '.join(7), lambda: '.'.join(['a', 'b', 3])]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_str.py::test_maketrans_translate",
            name: "string-maketrans-translate",
            source: "print('abababc'.translate({97: None}))\nprint('abababc'.translate({97: None, 98: 105, 99: 'x'}))\nprint('xzx'.translate({122: 'yy'}))\nprint('abababc'.translate({'b': '<i>'}))\ntbl = str.maketrans({'a': None, 'b': '<i>'})\nprint('abababc'.translate(tbl))\ntbl = str.maketrans('abc', 'xyz', 'd')\nprint('abdcdcbdddd'.translate(tbl))\nprint('[a]'.translate(str.maketrans({'a': 'XXX'})))\nprint('[é]'.translate(str.maketrans({'é': 'a'})))\nfor expr in [lambda: str.maketrans(), lambda: str.maketrans('abc', 'defg'), lambda: str.maketrans({'xy': 2}), lambda: 'hello'.translate(), lambda: '[a]'.translate(str.maketrans({'a': 1114112}))]:\n    try:\n        expr()\n    except (TypeError, ValueError) as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_str.py::test_codecs and Lib/test/test_bytes.py::test_encoding / ::test_decode",
            name: "string-bytes-codecs",
            source: "print('hello'.encode('ascii'), '☃'.encode())\nprint('caf\\xe9'.encode('latin-1'), 'Andr\\x82 x'.encode('ascii', 'ignore'), 'Andr\\x82 x'.encode('ascii', 'replace'))\nprint(b'hello'.decode('ascii'), b'\\xe2\\x98\\x83'.decode(), b'caf\\xe9'.decode('latin-1'))\nprint(b'Hello \\xff world'.decode('utf-8', 'ignore'))\nprint(b'Hello \\xff world'.decode('utf-8', 'replace'))\nprint(str(b'caf\\xe9', 'latin-1'))\nprint(bytes('caf\\xe9', 'latin-1'), bytearray('caf\\xe9', 'latin-1'))\nprint(bytes(source='Andr\\x82 x', encoding='ascii', errors='ignore'))\nprint(bytearray(source='caf\\xe9', encoding='latin-1'))\nprint(str(object=b'caf\\xe9', encoding='latin-1'), str(b'caf\\xe9', errors='ignore'))\nfor expr in [lambda: '\\xe9'.encode('ascii'), lambda: b'\\xff'.decode('utf-8'), lambda: 'x'.encode('unknown'), lambda: bytes('x'), lambda: bytes(3, encoding='utf-8'), lambda: str('x', encoding='utf-8')]:\n    try:\n        expr()\n    except (UnicodeEncodeError, UnicodeDecodeError, LookupError, TypeError) as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_bytes.py::test_fromhex / ::test_hex / hex separator tests",
            name: "bytes-hex-fromhex",
            source: "print(bytes.fromhex(''), bytearray.fromhex(''))\nprint(bytes.fromhex('1a2B30'), bytearray.fromhex('  1A 2B  30   '))\nprint(bytes.fromhex(' 1A\\n2B\\t30 '))\nprint(b''.hex(), b'\\x1a\\x2b\\x30'.hex(), bytearray(b'\\x1a\\x2b\\x30').hex())\nthree = b'\\xb9\\x01\\xef'\nprint(three.hex(), three.hex(':'), three.hex(':', 2), three.hex('*', -2), three.hex(sep=':', bytes_per_sep=2))\nsix = b'\\x03\\x06\\x09\\x0c\\x0f\\x12'\nprint(six.hex('.', 1), six.hex(' ', 2), six.hex('-', 3), six.hex(':', 4), six.hex('_', -3), six.hex(':', -4))\nfor expr in [lambda: bytes.fromhex(), lambda: bytes.fromhex(1), lambda: bytes.fromhex('a'), lambda: bytes.fromhex('rt'), lambda: bytes.fromhex('1a b cd'), lambda: b'abc'.hex(1), lambda: b'abc'.hex(''), lambda: b'abc'.hex('xx'), lambda: b'abc'.hex(chr(0x100)), lambda: b'abc'.hex(b'\\x80'), lambda: b'abc'.hex(sep=':', bytes_per_sep='x')]:\n    try:\n        expr()\n    except (TypeError, ValueError) as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_bytes.py::ByteArrayTest::test_regexps",
            name: "bytearray-regexps-findall",
            source: "import re\ndef by(text):\n    return bytearray(map(ord, text))\nfor source in [by('Hello, world'), b'Hi, Bob_2!', memoryview(b'xy 99')]:\n    matches = re.findall(br'\\w+', source)\n    print(type(source).__name__, matches, [type(item).__name__ for item in matches])",
        },
        DiffCase {
            origin: "Lib/test/test_bytes.py::ByteArraySubclassTest::test_init_override",
            name: "bytearray-subclass-init-override",
            source: "class Sub(bytearray):\n    def __init__(self, newarg=1, *args, **kwargs):\n        print('init', newarg, args, kwargs.get('source', None))\n        bytearray.__init__(self, *args, **kwargs)\nfor factory in [lambda: Sub(4, b'abcd'), lambda: Sub(4, source=b'abcd'), lambda: Sub(newarg=4, source=b'abcd')]:\n    value = factory()\n    print(type(value).__name__, value == b'abcd', bytes(value), isinstance(value, bytearray))\nclass Empty(bytearray):\n    def __init__(self, value):\n        print('empty init', value)\nempty = Empty(b'abc')\nprint(type(empty).__name__, len(empty), bytes(empty))",
        },
        DiffCase {
            origin: "Lib/test/test_bytes.py::SubclassTest::test_pickle",
            name: "bytes-bytearray-subclass-pickle-roundtrip",
            source: "import pickle\nclass B(bytes):\n    pass\nclass BA(bytearray):\n    pass\nfor T in [B, BA]:\n    checked = 0\n    nested = 0\n    independent = 0\n    for proto in range(pickle.HIGHEST_PROTOCOL + 1):\n        a = T(b'abcd')\n        a.x = 10\n        a.z = T(b'efgh')\n        b = pickle.loads(pickle.dumps(a, proto))\n        if type(b) is T and b == a and b is not a and b.x == 10 and not hasattr(b, 'y'):\n            checked += 1\n        if type(b.z) is T and b.z == T(b'efgh'):\n            nested += 1\n        if isinstance(b, bytearray):\n            b.append(ord('!'))\n            if bytes(a) == b'abcd' and bytes(b) == b'abcd!':\n                independent += 1\n        elif bytes(b) == b'abcd':\n            independent += 1\n    print(T.__name__, checked, nested, independent, pickle.HIGHEST_PROTOCOL + 1)",
        },
        DiffCase {
            origin: "Lib/test/test_bytes.py::ByteArrayTest::test_copied / ::test_partition_bytearray_doesnt_share_nullstring",
            name: "bytearray-nonmutating-copy-buffer-semantics",
            source: "b = bytearray(b'abc')\nr = b.replace(b'abc', b'cde', 0)\nprint(r, r is b)\nr += b'!'\nprint(b, r)\nt = bytearray([i for i in range(256)])\nx = bytearray(b'')\ny = x.translate(t)\nprint(y, y is x)\ny += b'!'\nprint(x, y)\na, b, c = bytearray(b'x').partition(b'y')\nprint(a, b, c, b is c)\nb += b'!'\nprint(b, c)\na, b, c = bytearray(b'x').partition(b'y')\nprint(b, c)\nb, c, a = bytearray(b'x').rpartition(b'y')\nprint(a, b, c, b is c)\nb += b'!'\nprint(b, c)\nc, b, a = bytearray(b'x').rpartition(b'y')\nprint(b, c)",
        },
        DiffCase {
            origin: "Lib/test/test_bytes.py::BytearrayPEP3137Test::test_returns_new_copy and AssortedBytesTest::test_return_self",
            name: "bytearray-pep3137-returns-new-copy",
            source: "val = bytearray(b'1234')\nfor methname in ['zfill', 'rjust', 'ljust', 'center']:\n    newval = getattr(val, methname)(3)\n    print(methname, val == newval, val is newval)\nchecks = [\n    ('split', lambda: val.split()[0]),\n    ('rsplit', lambda: val.rsplit()[0]),\n    ('partition', lambda: val.partition(b'.')[0]),\n    ('rpartition', lambda: val.rpartition(b'.')[2]),\n    ('splitlines', lambda: val.splitlines()[0]),\n    ('replace', lambda: val.replace(b'', b'')),\n]\nfor name, maker in checks:\n    newval = maker()\n    print(name, val == newval, val is newval)\nsep = bytearray(b'')\nnewval = sep.join([val])\nprint('join', val == newval, val is newval)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_bytearray_translate / ::test_bytearray_extend_error",
            name: "builtin-bytearray-translate-extend-errors",
            source: "x = bytearray(b'abc')\nfor expr in [lambda: x.translate(b'1', 1), lambda: x.translate(b'1' * 256, 1)]:\n    try:\n        expr()\n    except (TypeError, ValueError) as error:\n        print(error.__class__.__name__)\narray = bytearray()\nbad_iter = map(int, 'X')\ntry:\n    array.extend(bad_iter)\nexcept ValueError as error:\n    print(error.__class__.__name__, array)",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py::test_yield / ::test_yield_send",
            name: "f-string-yield-expressions",
            source: "def fn(y):\n    f'y:{yield y * 2}'\n    f'{yield}'\ng = fn(4)\nprint(next(g))\nprint(next(g))\ndef send_fn(x):\n    yield f'x:{yield (lambda i: x * i)}'\nsent = send_fn(10)\nthe_lambda = next(sent)\nprint(the_lambda(4))\nprint(sent.send('string'))",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py::test_gh129093",
            name: "f-string-debug-comparison-expressions",
            source: "print(f'{1==2=}')\nprint(f'{1 == 2=}')\nprint(f'{1!=2=}')\nprint(f'{1 != 2=}')\nprint(f'{(1) != 2=}')\nprint(f'{(1*2) != (3)=}')\nprint(f'{1 != 2 == 3 != 4=}')\nprint(f'{1 == 2 != 3 == 4=}')",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py::test_format_specifier_expressions",
            name: "f-string-nested-format-spec-expressions",
            source: "width = 10\nprint(f'{10:#{1}0x}')\nprint(f'{10:{\"#\"}1{0}{\"x\"}}')\nprint(f'{-10:-{\"#\"}1{0}x}')\nprint(f'{-10:{\"-\"}#{1}0{\"x\"}}')\nprint(f'{10:#{3 != {4:5} and width}x}')",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py::test_multiple_vars / ::test_closure / ::test_arguments / ::test_locals / ::test_missing_format_spec",
            name: "f-string-scope-and-format-lookup",
            source: "x = 98\ny = 'abc'\nprint(f'{x}{y}')\nprint(f'X{x}{y}')\nprint(f'{x}X{y}')\nprint(f'{x}{y}X')\nprint(f'X{x}Y{y}')\nprint(f'X{x}{y}Y')\nprint(f'{x}X{y}Y')\nprint(f'X{x}Y{y}Z')\ndef outer(x):\n    def inner():\n        return f'x:{x}'\n    return inner\nprint(outer('987')())\nprint(outer(7)())\ny = 2\ndef f(x, width):\n    return f'x={x*y:{width}}'\nprint(f('foo', 10))\nx = 'bar'\nprint(f(10, 10))\nvalue = 123\nprint(f'v:{value}')\nclass O:\n    def __format__(self, spec):\n        if not spec:\n            return '*'\n        return spec\nprint(f'{O():x}', f'{O()}')",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py::test_expressions_with_triple_quoted_strings",
            name: "f-string-triple-quoted-expression-strings",
            source: "print(f\"{'''x'''}\")\nprint(f\"{'''eric's'''}\")\nprint(f'{\"x\" \"\"\"eric\"s\"\"\" \"y\"}')\nprint(f'{\"x\" \"\"\"eric\"s\"\"\"}')\nprint(f'{\"\"\"eric\"s\"\"\" \"y\"}')\nprint(f'{\"\"\"x\"\"\" \"\"\"eric\"s\"\"\" \"y\"}')\nprint(f'{\"\"\"x\"\"\" \"\"\"eric\"s\"\"\" \"\"\"y\"\"\"}')\nprint(f'{r\"\"\"x\"\"\" \"\"\"eric\"s\"\"\" \"\"\"y\"\"\"}')",
        },
        DiffCase {
            origin: "Lib/test/test_tokenize.py nested f-string tokenizer case",
            name: "nested-f-string-tokenizer-expression",
            source: r###"print(f"""{f'''{f'{f"{1+1}"}'}'''}""")"###,
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::TestBooleanExpression",
            name: "boolean-expression-short-circuit-identity",
            source: "class Value:\n    def __init__(self):\n        self.called = 0\n    def __bool__(self):\n        self.called += 1\n        return self.value\nclass Yes(Value):\n    value = True\nclass No(Value):\n    value = False\nv = [Yes(), No(), Yes()]\nres = v[0] and v[1] and v[0]\nprint(res is v[1], [e.called for e in v])\nv = [No(), Yes(), No()]\nres = v[0] or v[1] or v[0]\nprint(res is v[1], [e.called for e in v])\nv = [No(), Yes(), Yes(), Yes()]\nres = v[0] and v[1] or v[2] or v[3]\nprint(res is v[2], [e.called for e in v])\nv = [No(), No(), Yes(), Yes(), No()]\nres = v[0] or v[1] and v[2] or v[3] or v[4]\nprint(res is v[3], [e.called for e in v])\nclass Foo:\n    def __bool__(self):\n        raise NotImplementedError()\na = Foo()\nb = Foo()\ntry:\n    bool(a)\nexcept NotImplementedError:\n    print('bool error')\ntry:\n    a or b\nexcept NotImplementedError:\n    print('or error')",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_while",
            name: "while-else",
            source: "x = 0\nwhile x < 3:\n    print(x)\n    x += 1\nelse:\n    print(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_for",
            name: "for-else-continue",
            source: "for x in range(4):\n    if x == 2:\n        continue\n    print(x)\nelse:\n    print(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_break_stmt",
            name: "for-break-skips-else",
            source: "for x in range(4):\n    print(x)\n    if x == 1:\n        break\nelse:\n    print(\"else\")\nprint(\"after\")",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_for",
            name: "for-over-unparenthesized-tuple",
            source: "total = 0\nfor i in 1, 2, 3:\n    total += i\nprint(total)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_selectors",
            name: "tuple-subscript-keys",
            source: "d = {}\nd[1] = 1\nd[1,] = 2\nd[1, 2] = 3\nd[1, 2, 3] = 4\nkeys = list(d)\nkeys.sort(key=lambda x: (type(x).__name__, x))\nprint(keys)\nprint(d[1], d[1,], d[1, 2], d[1, 2, 3])",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_atoms",
            name: "atom-displays-and-boolean-keys",
            source: "print((1))\nprint((1 or 2 or 3, 2, 3))\nprint([1 or 2 or 3, 2, 3])\nprint({})\nprint({'one' or 'two': 1 or 2, 'two': 2})",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::testFuncdef",
            name: "defaults-varargs-kwargs",
            source: "def add(a, b=2, *args, c=3, **kwargs):\n    print(a + b + c + args[0] + kwargs[\"d\"])\nadd(1, 4, 5, c=6, d=7)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_lambdef",
            name: "closure-default-lambda",
            source: "def outer(x):\n    def inner(y=3):\n        return x + y\n    return inner\nprint(outer(4)())\nprint((lambda x, y=2: x + y)(3))",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_listcomps",
            name: "list-comprehension-filters-and-nesting",
            source: "nums = [1, 2, 3, 4, 5]\nprint([3 * x for x in nums])\nprint([x for x in nums if x > 2])\nprint([(i, j) for i in [1, 2] for j in [3, 4]])",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_comprehension_specials",
            name: "adjacent-if-filters-and-tuple-unpack-targets",
            source: "print([x for x in range(10) if x % 2 if x % 3])\nprint(list(x for x in range(10) if x % 2 if x % 3))\nprint([x for x, in [(4,), (5,), (6,)]])",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_if_else_expr",
            name: "conditional-expression-precedence",
            source: "def check(msg, ret):\n    print(msg)\n    return ret\nprint(5 if 1 else check(\"check 1\", 0))\nprint(check(\"check 2\", 0) if 0 else 5)\nprint(5 and 6 if 0 else 1)\nprint(1 or check(\"check 4\", 2) if 1 else check(\"check 5\", 3))",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_try",
            name: "try-except-finally",
            source: "try:\n    raise ValueError(\"bad\")\nexcept ValueError as error:\n    print(error)\nfinally:\n    print(\"finally\")",
        },
        DiffCase {
            origin: "Lib/test/test_exceptions.py custom exception subclass matching",
            name: "custom-exception-subclass-caught-by-base",
            source: "class BaseError(Exception):\n    pass\nclass ChildError(BaseError):\n    pass\ntry:\n    raise ChildError(\"deep\")\nexcept BaseError as error:\n    print(error.__class__.__name__, error)\n    print(isinstance(error, ChildError), isinstance(error, BaseError), isinstance(error, Exception))\n    print(error.__class__ is ChildError, error.__class__.__bases__[0] is BaseError)",
        },
        DiffCase {
            origin: "Lib/test/test_exceptions.py::testAttributes SystemExit/OSError subset",
            name: "builtin-exception-attributes",
            source: "system = SystemExit('foo')\nprint(system.args, system.code)\nfor error in [OSError('foo'), OSError('foo', 'bar'), OSError('foo', 'bar', 'baz'), OSError('foo', 'bar', 'baz', None, 'quux')]:\n    print(error.args, error.errno, error.strerror, error.filename, error.filename2)\n    print(str(error))",
        },
        DiffCase {
            origin: "Lib/test/test_exceptions.py::testAttributes SyntaxError stable subset",
            name: "syntax-error-basic-attributes",
            source: "for error in [SyntaxError(), SyntaxError('msgStr'), SyntaxError('msgStr', 'filenameStr', 'linenoStr', 'offsetStr', 'textStr', 'endLinenoStr', 'endOffsetStr', 'print_file_and_lineStr')]:\n    print(error.args)\n    print(error.msg, error.text, error.filename, error.lineno, error.offset, getattr(error, 'end_lineno', None), getattr(error, 'end_offset', None), error.print_file_and_line)",
        },
        DiffCase {
            origin: "Lib/test/test_exceptions.py::testAttributes UnicodeError subset",
            name: "unicode-exception-attributes",
            source: "errors = [UnicodeError(), UnicodeEncodeError('ascii', 'a', 0, 1, 'ordinal not in range'), UnicodeDecodeError('ascii', bytearray(b'\\xff'), 0, 1, 'ordinal not in range'), UnicodeDecodeError('ascii', b'\\xff', 0, 1, 'ordinal not in range'), UnicodeTranslateError('\\u3042', 0, 1, 'ouch')]\nprint(errors[0].args)\nfor error in errors[1:4]:\n    print(error.args)\n    print(error.encoding, error.object, error.start, error.end, error.reason)\nerror = errors[4]\nprint(error.args)\nprint(error.object, error.start, error.end, error.reason)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_try finally control flow",
            name: "finally-control-flow-overrides",
            source: "for x in range(3):\n    try:\n        print(\"try\", x)\n        break\n    finally:\n        print(\"finally\", x)\n        continue\nprint(\"after\")\ndef f():\n    for x in range(1):\n        try:\n            return \"try\"\n        finally:\n            break\n    return \"after\"\nprint(f())\ndef g():\n    try:\n        raise ValueError(\"bad\")\n    finally:\n        return \"cleanup\"\nprint(g())",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_control_flow_in_finally issue #37830",
            name: "finally-break-continue-overrides-return",
            source: "def break_after_return(flag):\n    for count in [0, 1]:\n        count2 = 0\n        while count2 < 20:\n            count2 += 10\n            try:\n                return count + count2\n            finally:\n                if flag:\n                    break\n    return \"end\", count, count2\nprint(break_after_return(False))\nresult = break_after_return(True)\nprint(result[0], result[1], result[2])\ndef continue_after_return(flag):\n    count = 0\n    while count < 3:\n        count += 1\n        try:\n            return count\n        finally:\n            if flag:\n                continue\n    return \"end\", count\nprint(continue_after_return(False))\nresult = continue_after_return(True)\nprint(result[0], result[1])",
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::test_for_break_continue_inside_except_block",
            name: "for-break-continue-inside-except",
            source: "for x in [0, 1]:\n    try:\n        if x == 0:\n            raise ValueError(\"skip\")\n        print(\"body\", x)\n    except ValueError:\n        print(\"continue\", x)\n        continue\n    print(\"after\", x)\nelse:\n    print(\"else\")\nfor x in [0, 1, 2]:\n    try:\n        if x == 1:\n            raise ValueError(\"stop\")\n        print(\"body\", x)\n    except ValueError:\n        print(\"break\", x)\n        break\n    print(\"after\", x)\nelse:\n    print(\"else\")\nprint(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::test_for_break_continue_inside_with_block",
            name: "for-break-continue-inside-with",
            source: "class Manager:\n    def __init__(self, label):\n        self.label = label\n    def __enter__(self):\n        print(\"enter\", self.label)\n        return self\n    def __exit__(self, exc_type, exc_value, traceback):\n        print(\"exit\", self.label)\nfor label in [\"continue\", \"break\"]:\n    with Manager(label):\n        print(\"body\", label)\n        if label == \"continue\":\n            continue\n        break\nelse:\n    print(\"else\")\nprint(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::test_for_break_continue_inside_try_finally_block",
            name: "for-break-continue-inside-try-finally",
            source: "for label in [\"continue\", \"break\"]:\n    try:\n        print(\"try\", label)\n        if label == \"continue\":\n            continue\n        break\n    finally:\n        print(\"finally\", label)\nelse:\n    print(\"else\")\nprint(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::test_for_break_continue_inside_finally_block",
            name: "for-break-continue-inside-finally",
            source: "for label in [\"continue\", \"break\"]:\n    try:\n        print(\"try\", label)\n    finally:\n        print(\"finally\", label)\n        if label == \"continue\":\n            continue\n        break\nelse:\n    print(\"else\")\nprint(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::test_for_break_continue_inside_async_with_block",
            name: "for-break-continue-inside-async-with",
            source: "class AsyncManager:\n    def __init__(self, label):\n        self.label = label\n    async def __aenter__(self):\n        print(\"enter\", self.label)\n        return self\n    async def __aexit__(self, exc_type, exc_value, traceback):\n        print(\"exit\", self.label)\n        return False\nasync def main():\n    for label in [\"continue\", \"break\"]:\n        async with AsyncManager(label):\n            print(\"body\", label)\n            if label == \"continue\":\n                continue\n            break\n    else:\n        print(\"else\")\n    print(\"after\")\ncoro = main()\ntry:\n    coro.send(None)\nexcept StopIteration:\n    print(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::test_return_inside_async_with_block",
            name: "return-inside-async-with",
            source: "class AsyncManager:\n    def __init__(self, label):\n        self.label = label\n    async def __aenter__(self):\n        print(\"enter\", self.label)\n        return self\n    async def __aexit__(self, exc_type, exc_value, traceback):\n        print(\"exit\", self.label, exc_type)\n        return False\nasync def main(flag):\n    async with AsyncManager(flag):\n        if flag:\n            return \"returned\"\n        print(\"body\")\n    return \"after\"\nfor flag in [True, False]:\n    coro = main(flag)\n    try:\n        coro.send(None)\n    except StopIteration as done:\n        print(done)",
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::test_return_inside_except_block / ::test_return_inside_with_block",
            name: "return-inside-except-and-with",
            source: "def from_except(flag):\n    try:\n        if flag:\n            raise ValueError(\"stop\")\n        print(\"try body\")\n    except ValueError:\n        print(\"except\")\n        return \"returned from except\"\n    return \"after\"\nprint(from_except(True))\nprint(from_except(False))\nclass Manager:\n    def __enter__(self):\n        print(\"enter\")\n        return self\n    def __exit__(self, exc_type, exc_value, traceback):\n        print(\"exit\", exc_type)\n        return False\ndef from_with(flag):\n    with Manager():\n        if flag:\n            return \"returned from with\"\n        print(\"with body\")\n    return \"after\"\nprint(from_with(True))\nprint(from_with(False))",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_func_2",
            name: "coroutine-raised-stopiteration-becomes-runtimeerror",
            source: "async def raises_stop_iteration():\n    raise StopIteration(42)\nasync def raises_stop_async_iteration():\n    raise StopAsyncIteration(99)\nfor coro in [raises_stop_iteration(), raises_stop_async_iteration()]:\n    try:\n        coro.send(None)\n    except Exception as error:\n        print(error.__class__.__name__, error)\n        if error.__cause__ is None:\n            print(\"cause\", None)\n        else:\n            print(\"cause\", error.__cause__.__class__.__name__, error.__cause__)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_func_18",
            name: "coroutine-await-wrapper-reuse",
            source: "async def f():\n    return \"spam\"\naw = f().__await__()\nprint(aw is iter(aw))\ntry:\n    next(aw)\nexcept StopIteration as done:\n    print(done)\ntry:\n    next(aw)\nexcept RuntimeError as error:\n    print(error)\nprint(aw.close())",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_func_13",
            name: "coroutine-send-non-none-to-just-started",
            source: "async def f():\n    pass\ncoro = f()\ntry:\n    coro.send(\"spam\")\nexcept TypeError as error:\n    print(error)\nprint(coro.close())",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_coro_wrapper_send_stop_iterator",
            name: "coroutine-returned-stopiteration-is-value",
            source: "async def f():\n    return StopIteration(10)\ntry:\n    f().send(None)\nexcept StopIteration as done:\n    print(done.__class__.__name__, done)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_3",
            name: "await-yield-from-iterable-yields-through-coroutine",
            source: "class AsyncYieldFrom:\n    def __init__(self, obj):\n        self.obj = obj\n    def __await__(self):\n        yield from self.obj\nasync def f():\n    await AsyncYieldFrom([1, 2, 3])\ncoro = f()\nitems = []\ntry:\n    while True:\n        items.append(coro.send(None))\nexcept StopIteration:\n    print(items)\n    print(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_6",
            name: "await-returned-iterator-yields-through-coroutine",
            source: "class Awaitable:\n    def __await__(self):\n        return iter([52])\nasync def f():\n    await Awaitable()\ncoro = f()\nitems = []\ntry:\n    while True:\n        items.append(coro.send(None))\nexcept StopIteration:\n    print(items)\n    print(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_7",
            name: "await-generator-return-value-completes-expression",
            source: "class Awaitable:\n    def __await__(self):\n        yield 42\n        return 100\nasync def f():\n    return await Awaitable()\ncoro = f()\nitems = []\ntry:\n    while True:\n        items.append(coro.send(None))\nexcept StopIteration as done:\n    print(items)\n    print(done)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_5",
            name: "await-dunder-await-returning-none-rejected",
            source: "class Awaitable:\n    def __await__(self):\n        return\nasync def f():\n    return await Awaitable()\ntry:\n    f().send(None)\nexcept TypeError as error:\n    print(error)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_12",
            name: "await-dunder-await-returning-coroutine-rejected",
            source: "async def inner():\n    return \"spam\"\ncoro = inner()\nclass Awaitable:\n    def __await__(self):\n        return coro\nasync def f():\n    return await Awaitable()\ntry:\n    f().send(None)\nexcept TypeError as error:\n    print(error)\nprint(coro.close())",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_13",
            name: "await-dunder-await-returning-non-iterator-self-rejected",
            source: "class Awaitable:\n    def __await__(self):\n        return self\nasync def f():\n    return await Awaitable()\ntry:\n    f().send(None)\nexcept TypeError as error:\n    print(error)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_8",
            name: "await-object-without-dunder-await-rejected",
            source: "class Awaitable:\n    pass\nasync def f():\n    return await Awaitable()\ntry:\n    f().send(None)\nexcept TypeError as error:\n    print(error)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_9",
            name: "await-expression-composition",
            source: "def wrap():\n    return bar\nasync def bar():\n    return 42\nasync def f():\n    db = {\"b\": lambda: wrap}\n    class DB:\n        b = wrap\n    return await bar() + await wrap()() + await db[\"b\"]()()() + await bar() * 1000 + await DB.b()()\nasync def g():\n    return -await bar()\nfor coro in [f(), g()]:\n    try:\n        coro.send(None)\n    except StopIteration as done:\n        print(done)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_10",
            name: "nested-await-expression",
            source: "async def baz():\n    return 42\nasync def bar():\n    return baz()\nasync def f():\n    return await (await bar())\ntry:\n    f().send(None)\nexcept StopIteration as done:\n    print(done)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_11",
            name: "await-in-call-keyword-and-tuple",
            source: "def ident(val):\n    return val\nasync def bar():\n    return \"spam\"\nasync def f():\n    return ident(val=await bar())\nasync def g():\n    return await bar(), \"ham\"\nfor coro in [f(), g()]:\n    try:\n        coro.send(None)\n    except StopIteration as done:\n        print(done)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_14",
            name: "await-wrapper-forwards-send-and-throw",
            source: "class Wrapper:\n    def __init__(self, coro):\n        self.coro = coro\n    def __await__(self):\n        return self.coro.__await__()\nclass FutureLike:\n    def __await__(self):\n        return (yield)\nclass Marker(Exception):\n    pass\nasync def coro1():\n    try:\n        return await FutureLike()\n    except ZeroDivisionError:\n        raise Marker\nasync def coro2():\n    return await Wrapper(coro1())\nc = coro2()\nprint(c.send(None))\ntry:\n    c.send(\"spam\")\nexcept StopIteration as done:\n    print(done)\nc = coro2()\nprint(c.send(None))\ntry:\n    c.throw(ZeroDivisionError)\nexcept Marker as error:\n    print(error.__class__.__name__, error)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_15",
            name: "awaiting-active-coroutine-rejected",
            source: "class Awaitable:\n    def __await__(self):\n        yield\nasync def coroutine():\n    await Awaitable()\nasync def waiter(coro):\n    await coro\ncoro = coroutine()\nprint(coro.send(None))\ntry:\n    waiter(coro).send(None)\nexcept RuntimeError as error:\n    print(error)\nprint(coro.close())",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_await_16",
            name: "await-returned-exception-has-no-active-context",
            source: "async def f():\n    return ValueError()\nasync def g():\n    try:\n        raise KeyError\n    except KeyError:\n        result = await f()\n        print(result.__context__)\n        return result\ntry:\n    g().send(None)\nexcept StopIteration as done:\n    print(done.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::AssignmentTargetTestCase target binding cleanup",
            name: "with-target-unpack-error-suppressed-by-exit",
            source: "class Manager:\n    def __enter__(self):\n        print(\"enter\")\n        return (1, 2, 3)\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\", exc_type.__name__)\n        return True\nwith Manager() as (a, b):\n    print(\"body\")\nprint(\"after\")",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::AssignmentTargetTestCase target binding cleanup",
            name: "with-target-unpack-error-reraises-after-exit",
            source: "class Manager:\n    def __enter__(self):\n        print(\"enter\")\n        return (1, 2, 3)\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\", exc_type.__name__)\n        return False\ntry:\n    with Manager() as (a, b):\n        print(\"body\")\nexcept ValueError:\n    print(\"caught\")\nprint(\"after\")",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::AssignmentTargetTestCase target binding cleanup applied to async with",
            name: "async-with-target-unpack-error-suppressed-by-aexit",
            source: "class AsyncManager:\n    async def __aenter__(self):\n        print(\"enter\")\n        return (1, 2, 3)\n    async def __aexit__(self, exc_type, exc, traceback):\n        print(\"exit\", exc_type.__name__)\n        return True\nasync def main():\n    async with AsyncManager() as (a, b):\n        print(\"body\")\n    print(\"after\")\ncoro = main()\ntry:\n    coro.send(None)\nexcept StopIteration:\n    print(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_with_6 / ::test_with_7 / ::test_with_8",
            name: "async-with-non-awaitable-enter-exit-results",
            source: "class BadEnter:\n    def __aenter__(self):\n        return 123\n    async def __aexit__(self, exc_type, exc, traceback):\n        print(\"bad enter exit\")\nclass BadExit:\n    async def __aenter__(self):\n        return self\n    def __aexit__(self, exc_type, exc, traceback):\n        return 456\nasync def bad_enter():\n    async with BadEnter():\n        print(\"enter body\")\nasync def bad_exit_normal():\n    async with BadExit():\n        print(\"exit body\")\nasync def bad_exit_exception():\n    async with BadExit():\n        1 / 0\nfor coro in [bad_enter(), bad_exit_normal(), bad_exit_exception()]:\n    try:\n        coro.send(None)\n    except TypeError as error:\n        print(error)\n        if error.__context__ is None:\n            print(\"context\", None)\n        else:\n            print(\"context\", error.__context__.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_with_9 / ::test_with_10 / ::test_with_11 / ::test_with_12 / ::test_with_13",
            name: "async-with-exit-enter-exception-contexts",
            source: "class ExitRaises:\n    async def __aenter__(self):\n        return self\n    async def __aexit__(self, *exc):\n        1 / 0\nasync def normal_exit():\n    async with ExitRaises():\n        print(\"normal body\")\nclass EnterRaises:\n    async def __aenter__(self):\n        raise NotImplementedError(\"enter\")\n    async def __aexit__(self, *exc):\n        print(\"bad exit\")\n        return True\nasync def enter_case():\n    async with EnterRaises():\n        print(\"bad body\")\nclass Suppress:\n    async def __aenter__(self):\n        return self\n    async def __aexit__(self, exc_type, exc, traceback):\n        print(\"suppress\", exc_type.__name__)\n        return True\nasync def suppress_case():\n    async with Suppress() as cm:\n        print(cm.__class__.__name__)\n        raise RuntimeError(\"hidden\")\n    print(\"after suppress\")\nasync def nested_context():\n    async with ExitRaises():\n        async with ExitRaises():\n            raise RuntimeError(\"inner\")\nfor coro in [normal_exit(), enter_case(), suppress_case(), nested_context()]:\n    try:\n        coro.send(None)\n    except ZeroDivisionError as error:\n        if error.__context__ is None:\n            print(\"zero\", None)\n        else:\n            print(\"zero\", error.__context__.__class__.__name__)\n        if error.__context__ and error.__context__.__context__:\n            print(\"chain\", error.__context__.__context__.__class__.__name__)\n    except NotImplementedError as error:\n        print(\"enter\", error, error.__context__)\n    except StopIteration:\n        print(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_for_2 / ::test_for_3 / ::test_for_4",
            name: "async-for-protocol-errors",
            source: "async def consume(value):\n    async for item in value:\n        print(\"body\", item)\nclass MissingAnext:\n    def __aiter__(self):\n        return self\nclass BadAnext:\n    def __aiter__(self):\n        return self\n    def __anext__(self):\n        return ()\nfor value in [(1, 2), MissingAnext(), BadAnext()]:\n    try:\n        consume(value).send(None)\n    except TypeError as error:\n        print(error)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_for_11",
            name: "async-for-anext-await-raises-protocol-error",
            source: "class AwaitRaises:\n    def __aiter__(self):\n        return self\n    def __anext__(self):\n        return self\n    def __await__(self):\n        1 / 0\nasync def main():\n    async for item in AwaitRaises():\n        print(item)\ntry:\n    main().send(None)\nexcept TypeError as error:\n    print(error)\n    print(error.__cause__.__class__.__name__, error.__cause__)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_for_6 / ::test_for_7 / ::test_for_8",
            name: "async-for-with-manager-and-aiter-exception-control-flow",
            source: "I = 0\nclass Manager:\n    async def __aenter__(self):\n        global I\n        I += 10000\n    async def __aexit__(self, *args):\n        global I\n        I += 100000\nclass Iterable:\n    def __init__(self):\n        self.i = 0\n    def __aiter__(self):\n        return self\n    async def __anext__(self):\n        if self.i > 10:\n            raise StopAsyncIteration\n        self.i += 1\n        return self.i\nmanager = Manager()\niterable = Iterable()\nasync def first():\n    global I\n    async with manager:\n        async for i in iterable:\n            I += 1\n    I += 1000\nasync def second():\n    global I\n    async with Manager():\n        async for i in Iterable():\n            I += 1\n    I += 1000\n    async with Manager():\n        async for i in Iterable():\n            I += 1\n    I += 1000\nasync def third():\n    global I\n    async with Manager():\n        I += 100\n        async for i in Iterable():\n            I += 1\n        else:\n            I += 10000000\n    I += 1000\n    async with Manager():\n        I += 100\n        async for i in Iterable():\n            I += 1\n        else:\n            I += 10000000\n    I += 1000\nfor coro in [first(), second(), third()]:\n    try:\n        coro.send(None)\n    except StopIteration:\n        print(I)\nCNT = 0\nclass AiterRaises:\n    def __aiter__(self):\n        1 / 0\nasync def raises_aiter():\n    global CNT\n    async for i in AiterRaises():\n        CNT += 1\n    CNT += 10\ntry:\n    raises_aiter().send(None)\nexcept ZeroDivisionError as error:\n    print(\"aiter\", error.__class__.__name__, CNT)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_for_assign_raising_stop_async_iteration",
            name: "async-for-subscript-target-stopasynciteration-propagates",
            source: "class BadTarget:\n    def __setitem__(self, key, value):\n        raise StopAsyncIteration(42)\ntgt = BadTarget()\nasync def source():\n    yield 10\nasync def run_for():\n    try:\n        async for tgt[0] in source():\n            print(\"body\")\n    except StopAsyncIteration as error:\n        print(\"target\", error)\n    return \"end\"\nasync def run_list():\n    try:\n        return [0 async for tgt[0] in source()]\n    except StopAsyncIteration as error:\n        print(\"list\", error)\n    return \"end\"\nfor coro in [run_for(), run_list()]:\n    try:\n        coro.send(None)\n    except StopIteration as done:\n        print(done)",
        },
        DiffCase {
            origin: "Lib/test/test_coroutines.py::test_for_assign_raising_stop_async_iteration_2",
            name: "async-for-unpack-target-stopasynciteration-propagates",
            source: "class BadIterable:\n    def __iter__(self):\n        raise StopAsyncIteration(42)\nasync def badpairs():\n    yield BadIterable()\nasync def run_for():\n    try:\n        async for i, j in badpairs():\n            print(\"body\")\n    except StopAsyncIteration as error:\n        print(\"unpack\", error)\n    return \"end\"\nasync def run_list():\n    try:\n        return [0 async for i, j in badpairs()]\n    except StopAsyncIteration as error:\n        print(\"list\", error)\n    return \"end\"\nfor coro in [run_for(), run_list()]:\n    try:\n        coro.send(None)\n    except StopIteration as done:\n        print(done)",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::NestedWith::testExceptionInExprList",
            name: "multi-with-second-manager-init-failure-cleans-first",
            source: "class First:\n    def __enter__(self):\n        print(\"enter first\")\n        return self\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit first\", exc_type.__name__, exc)\n        return False\nclass BadInit:\n    def __init__(self):\n        print(\"init bad\")\n        raise RuntimeError(\"init\")\ntry:\n    with First() as first, BadInit():\n        print(\"body\")\nexcept RuntimeError as error:\n    print(\"caught\", error)",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::NestedWith::testExceptionInEnter",
            name: "multi-with-second-enter-failure-cleans-first",
            source: "class First:\n    def __enter__(self):\n        print(\"enter first\")\n        return self\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit first\", exc_type.__name__, exc)\n        return False\nclass BadEnter:\n    def __enter__(self):\n        print(\"enter bad\")\n        raise RuntimeError(\"enter\")\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit bad\")\ntry:\n    with First() as first, BadEnter():\n        print(\"body\")\nexcept RuntimeError as error:\n    print(\"caught\", error)",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::NestedWith::testExceptionInExit",
            name: "multi-with-second-exit-failure-can-be-suppressed-by-first",
            source: "class First:\n    def __enter__(self):\n        print(\"enter first\")\n        return self\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit first\", exc_type.__name__, exc)\n        return True\nclass BadExit:\n    def __enter__(self):\n        print(\"enter bad\")\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit bad\")\n        raise RuntimeError(\"exit\")\nwith First() as first, BadExit():\n    print(\"body\")\nprint(\"after\")",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::ExceptionalTestCase::testErrorsInBool",
            name: "with-exit-result-truthiness-errors",
            source: "class ExitResult:\n    def __init__(self, mode):\n        self.mode = mode\n    def __bool__(self):\n        print(\"bool\", self.mode)\n        if self.mode == \"raise\":\n            1 // 0\n        return self.mode == \"true\"\nclass Manager:\n    def __init__(self, mode):\n        self.mode = mode\n    def __enter__(self):\n        print(\"enter\", self.mode)\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\", self.mode, exc_type.__name__)\n        return ExitResult(self.mode)\nwith Manager(\"true\"):\n    raise AssertionError(\"hidden\")\nprint(\"after true\")\ntry:\n    with Manager(\"false\"):\n        raise AssertionError(\"visible\")\nexcept AssertionError as error:\n    print(\"caught false\", error)\ntry:\n    with Manager(\"raise\"):\n        raise AssertionError(\"hidden\")\nexcept ZeroDivisionError as error:\n    print(\"caught bool\", error.__class__.__name__, error)",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::AssignmentTargetTestCase::testMultipleComplexTargets",
            name: "with-complex-sequence-targets",
            source: "class Manager:\n    def __enter__(self):\n        return 1, 2, 3\n    def __exit__(self, exc_type, exc, traceback):\n        pass\ntargets = {1: [0, 0, 0]}\nwith Manager() as (targets[1][0], targets[1][1], targets[1][2]):\n    print(targets[1])\nwith Manager() as (targets[1], targets[2], targets[3]):\n    print(targets[1], targets[2], targets[3])\nclass Box:\n    pass\nbox = Box()\nwith Manager() as (box.one, box.two, box.three):\n    print(box.one, box.two, box.three)",
        },
        DiffCase {
            origin: "Lib/test/test_with.py::NonLocalFlowControlTestCase::testWithYield",
            name: "with-yield-keeps-manager-open-until-generator-resumes",
            source: "class Manager:\n    def __enter__(self):\n        print(\"enter\")\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\", exc_type)\ndef gen():\n    with Manager():\n        yield 12\n        yield 13\nvalues = gen()\nprint(next(values))\nprint(next(values))\ntry:\n    next(values)\nexcept StopIteration:\n    print(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_classdef",
            name: "class-inheritance-and-methods",
            source: "class Base:\n    label = \"base\"\nclass Child(Base):\n    def add(self, x):\n        return x + 1\nchild = Child()\nprint(Child.__bases__[0].__name__, Child.label, child.add(4))\nprint(isinstance(child, Child), isinstance(child, Base), isinstance(Child, Base))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_issubclass",
            name: "issubclass-builtin",
            source: "class C:\n    pass\nclass D(C):\n    pass\nclass E:\n    pass\nprint(issubclass(D, C), issubclass(C, C), issubclass(C, D))\nprint(issubclass(D, (E, C)), issubclass(E, (C, D)))\nprint(issubclass(bool, int), issubclass(int, object), issubclass(C, object))\nprint(issubclass(OverflowError, ArithmeticError), issubclass(KeyError, LookupError), issubclass(ValueError, ArithmeticError))\nfor expr in [lambda: issubclass('foo', E), lambda: issubclass(E, 'foo'), lambda: issubclass()]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_isinstance",
            name: "isinstance-builtin",
            source: "class C:\n    pass\nclass D(C):\n    pass\nclass E:\n    pass\nc = C()\nd = D()\ne = E()\nprint(isinstance(c, C), isinstance(d, C), isinstance(e, C), isinstance(c, D))\nprint(isinstance('foo', E), isinstance(d, (E, C)), isinstance(e, (C, D)))\nprint(isinstance(True, int), isinstance(False, bool), isinstance(1, object))\nprint(isinstance(OverflowError('x'), ArithmeticError), isinstance(KeyError('x'), LookupError), isinstance(ValueError('x'), ArithmeticError))\nfor expr in [lambda: isinstance(E, 'foo'), lambda: isinstance(), lambda: isinstance(e, (C, 'bad'))]:\n    try:\n        expr()\n    except TypeError as error:\n        print(error.__class__.__name__)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_callable / ::test_getattr / ::test_hasattr / ::test_setattr / ::test_delattr",
            name: "attribute-introspection-builtins",
            source: "import sys\nprint(callable(len), callable('a'), callable(callable))\ndef f():\n    pass\nprint(callable(f))\nclass C1:\n    def meth(self):\n        pass\nc = C1()\nprint(callable(C1), callable(c.meth), callable(c))\nc.__call__ = None\nprint(callable(c))\nc.__call__ = lambda self: 0\nprint(callable(c))\ndel c.__call__\nprint(callable(c))\nclass C2:\n    def __call__(self, value):\n        return value + 1\nc2 = C2()\nprint(callable(c2), c2(4))\nc2.__call__ = None\nprint(callable(c2), c2(5))\nclass C3(C2):\n    pass\nc3 = C3()\nprint(callable(c3), c3(6))\nsetattr(sys, 'spam', 1)\nprint(getattr(sys, 'spam'), hasattr(sys, 'spam'))\ndelattr(sys, 'spam')\nprint(hasattr(sys, 'spam'), getattr(sys, 'spam', 'missing'))\nclass Box:\n    pass\nbox = Box()\nsetattr(box, 'value', 3)\nprint(getattr(box, 'value'), hasattr(box, 'value'))\nsetattr(Box, 'label', 'box')\nprint(getattr(box, 'label'), getattr(Box, 'label'))\ndelattr(box, 'value')\nprint(hasattr(box, 'value'), getattr(box, 'value', 42))\ntry:\n    print((1).missing)\nexcept AttributeError:\n    print('caught')",
        },
        DiffCase {
            origin: "Lib/test/test_descr.py::ClassPropertiesAndMethods::test_getattr_and_setattr_hooks / Lib/test/test_builtin.py::BuiltinTest::test_hasattr",
            name: "instance-attribute-hooks",
            source: r#"class Hook:
    def __getattr__(self, name):
        if name == 'missing':
            return 'fallback'
        raise AttributeError(name)
    def __setattr__(self, name, value):
        if name == 'blocked':
            raise AttributeError(name)
        object.__setattr__(self, name, value + 1)
    def __delattr__(self, name):
        if name == 'blocked':
            raise AttributeError(name)
        object.__delattr__(self, name)

h = Hook()
print(h.missing, getattr(h, 'missing'), hasattr(h, 'missing'), hasattr(h, 'absent'))
h.value = 3
print(h.value)
setattr(h, 'other', 4)
print(h.other)
del h.value
print(hasattr(h, 'value'), getattr(h, 'value', 42))
try:
    h.blocked = 1
except AttributeError:
    print('set blocked')
object.__setattr__(h, 'blocked', 9)
print(h.blocked)
try:
    del h.blocked
except AttributeError:
    print('delete blocked')
object.__delattr__(h, 'blocked')
print(hasattr(h, 'blocked'))
class Bad:
    def __getattr__(self, name):
        raise RuntimeError('boom')
try:
    hasattr(Bad(), 'x')
except RuntimeError as error:
    print(error)
class Child(Hook):
    pass
c = Child()
c.value = 10
print(c.value, c.missing)"#,
        },
        DiffCase {
            origin: "Lib/test/test_funcattrs.py::InstancemethodAttrTest / Lib/test/test_descr.py::ClassPropertiesAndMethods::test_methods",
            name: "bound-method-metadata",
            source: r#"class F:
    def a(self):
        return 3

fi = F()
method = fi.a
print(method(), method.__name__, method.__qualname__)
print(method.__func__ is F.a, method.__self__ is fi, method.__module__, method.__doc__)
for attr, value in [
    ('__func__', F.a),
    ('__self__', fi),
    ('__name__', 'a'),
    ('__qualname__', 'F.a'),
]:
    try:
        setattr(method, attr, value)
    except (AttributeError, TypeError) as error:
        print(attr, error.__class__.__name__)

class C:
    def __init__(self, x):
        self.x = x
    def foo(self):
        return self.x

c1 = C(1)
class D(C):
    boo = C.foo
    goo = c1.foo

d2 = D(2)
print(d2.foo(), d2.boo(), d2.goo())
class E:
    foo = C.foo
print(E().foo.__func__ is C.foo, E().foo.__qualname__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_funcattrs.py::FunctionPropertiesTest::test___globals__",
            name: "function-globals-attribute",
            source: r#"value = 'module'
def f():
    return value
print(f.__globals__ is globals(), f.__globals__['value'], '__globals__' in dir(f))
try:
    setattr(f, '__globals__', {})
except (AttributeError, TypeError) as error:
    print(error.__class__.__name__)
value = 'updated'
print(f())
exec("def g(): return value", globals())
value = 'exec-global'
print(g.__globals__ is globals(), g())
other = {'f': f, 'value': 'other'}
exec('print(f())', other)"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py::ClassPropertiesAndMethods::test_getattribute / ::ClassPropertiesAndMethods::test_getattr_hooks",
            name: "instance-getattribute-hook",
            source: r#"class A:
    pass
a = A()
a.foo = 42
a.bar = 43
def getattribute(self, name):
    if name == 'foo':
        return 24
    return object.__getattribute__(self, name)
A.__getattribute__ = getattribute
print(a.foo, a.bar)
def getattr_hook(self, name):
    if name in ('spam', 'foo', 'bar'):
        return 'hello'
    raise AttributeError(name)
A.__getattr__ = getattr_hook
print(a.foo, a.spam)
del A.__getattribute__
print(a.foo)
del a.foo
print(a.foo, a.bar)
del A.__getattr__
try:
    a.foo
except AttributeError:
    print('missing')

class Fallback:
    def __getattribute__(self, name):
        if name == 'x':
            return 1
        raise AttributeError(name)
    def __getattr__(self, name):
        if name == 'y':
            return 2
        raise AttributeError(name)
f = Fallback()
print(f.x, f.y, getattr(f, 'z', 3), hasattr(f, 'z'))
try:
    object.__getattribute__(f, 'y')
except AttributeError:
    print('object missing')

class Bad:
    def __getattribute__(self, name):
        raise RuntimeError('boom')
try:
    hasattr(Bad(), 'x')
except RuntimeError as error:
    print(error)"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py::ClassPropertiesAndMethods::test_properties / Lib/test/test_property.py::PropertyTests::test_property_decorator_baseclass",
            name: "property-descriptor",
            source: r#"class C:
    def __init__(self):
        self._x = 1
    @property
    def x(self):
        return self._x
    @x.setter
    def x(self, value):
        self._x = abs(value)
    @x.deleter
    def x(self):
        del self._x

c = C()
print(C.x.fget.__name__, C.x.fset.__name__, C.x.fdel.__name__, isinstance(C.x, property))
print(c.x)
c.x = -5
print(c._x, c.x)
C.x.__set__(c, -7)
print(C.x.__get__(c), c.x)
C.x.__delete__(c)
print(hasattr(c, 'x'), hasattr(c, '_x'))
try:
    print(c.x)
except AttributeError:
    print('missing')

class ReadOnly:
    @property
    def y(self):
        return 1
r = ReadOnly()
print(r.y)
try:
    r.y = 2
except AttributeError:
    print('readonly')
try:
    del r.y
except AttributeError:
    print('nodelete')

class Fallback:
    def __getattr__(self, name):
        return 'fallback'
    @property
    def z(self):
        raise AttributeError('prop')
print(Fallback().z)"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py generic descriptor protocol subset",
            name: "custom-descriptor-protocol",
            source: r#"class NonData:
    def __get__(self, obj, owner):
        if obj is None:
            return 'class nondata ' + owner.__name__
        return 'nondata ' + owner.__name__

class Data:
    def __get__(self, obj, owner):
        if obj is None:
            return 'class data ' + owner.__name__
        return obj.value
    def __set__(self, obj, value):
        obj.value = value + 1
    def __delete__(self, obj):
        obj.value = -1

class DeleteOnly:
    def __delete__(self, obj):
        obj.deleted = True

class SetOnly:
    def __set__(self, obj, value):
        obj.set_value = value

class C:
    nd = NonData()
    dd = Data()
    donly = DeleteOnly()
    sonly = SetOnly()

c = C()
print(c.nd)
c.nd = 'field'
print(c.nd, C.nd)
c.dd = 4
print(c.value, c.dd, C.dd)
del c.dd
print(c.value)
print(c.donly is C.donly)
try:
    c.donly = 1
except AttributeError:
    print('delete-only set blocked')
del c.donly
print(c.deleted)
c.sonly = 8
print(c.set_value)
try:
    del c.sonly
except AttributeError:
    print('set-only delete blocked')

class Child(C):
    pass
child = Child()
print(child.nd)
child.dd = 10
print(child.dd)"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py::ClassPropertiesAndMethods classmethod/staticmethod subset",
            name: "classmethod-staticmethod-descriptors",
            source: r#"class C:
    @staticmethod
    def s(x):
        return x + 1
    @classmethod
    def c(cls, x):
        return cls.__name__ + str(x)

class D(C):
    pass

inst = C()
print(C.s(1), inst.s(2), C.s.__name__, inst.s.__name__)
print(C.c(3), inst.c(4), D.c(5), D().c(6))
print(callable(staticmethod(C.s)), callable(classmethod(C.s)))

sm = staticmethod(lambda x: x + 10)
print(sm.__func__(1), sm.__get__(None, C)(2), sm.__get__(inst, None)(3), isinstance(sm, staticmethod))

cm = classmethod(lambda cls, x: cls.__name__ + str(x))
print(cm.__func__(C, 7), cm.__get__(None, C)(8), cm.__get__(inst, None)(9), isinstance(cm, classmethod))"#,
        },
        DiffCase {
            origin: "Lib/test/test_super.py explicit two-argument super subset",
            name: "explicit-super-descriptor-lookup",
            source: r#"class Base:
    def greet(self):
        return 'Base:' + self.name
    @classmethod
    def label(cls):
        return cls.__name__
    @staticmethod
    def add(x):
        return x + 1

class Child(Base):
    def __init__(self, name):
        self.name = name
    def greet(self):
        return super(Child, self).greet() + ':Child'
    @classmethod
    def label(cls):
        return super(Child, cls).label() + ':child'
    @staticmethod
    def add(x):
        return super(Child, Child).add(x) + 1

c = Child('n')
print(c.greet())
print(Child.label(), c.label())
print(Child.add(2), c.add(3))
print(super(Child, Child).greet(c))
print(super(Child, c).__thisclass__.__name__, super(Child, c).__self__ is c, isinstance(super(Child, c), super))"#,
        },
        DiffCase {
            origin: "Lib/test/test_super.py zero-argument super subset",
            name: "zero-arg-super-descriptor-lookup",
            source: r#"class Base:
    def greet(self):
        return 'Base:' + self.name
    @classmethod
    def label(cls):
        return cls.__name__
    @property
    def value(self):
        return 'base:' + self.name

class Child(Base):
    def __init__(self, name):
        self.name = name
    def greet(self):
        return super().greet() + ':Child:' + __class__.__name__
    @classmethod
    def label(cls):
        return super().label() + ':child:' + __class__.__name__
    @property
    def value(self):
        return super().value + ':child'

c = Child('n')
print(c.greet())
print(Child.label(), c.label())
print(c.value)"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py::ClassPropertiesAndMethods::test_supers / Lib/test/test_super.py super attribute checks",
            name: "unbound-super-descriptor",
            source: r#"class A:
    def meth(self, value):
        return 'A' + str(value)

class C(A):
    def meth(self, value):
        return 'C' + str(value) + self.sup.meth(value)

C.sup = super(C)
c = C()
print(c.meth(3))
s = super(C)
print(s.__thisclass__ is C, s.__self__, s.__self_class__)
bound = s.__get__(c)
print(bound.__thisclass__ is C, bound.__self__ is c, bound.__self_class__ is C, bound.meth(4))
class_bound = s.__get__(C)
print(class_bound.__self__ is C, class_bound.__self_class__ is C)
print(s.__get__(None, C) is s, super(C, None).__self__)
for expr in [lambda: super(1), lambda: super(C).__get__(12), lambda: super(C).__get__(A())]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py::test_instance_method_get_behavior / ::test_bound_method_repr",
            name: "bound-method-descriptor-and-repr",
            source: r#"class A:
    def meth(self):
        return self

class B:
    pass

a = A()
b = B()
bound = a.meth
b.meth = bound.__get__(b, B)
print(b.meth() is a)
print(bound.__get__(b)() is a, bound.__get__(None, B)() is a)

def check(text, method, receiver):
    print(text.startswith('<bound method '), method in text, receiver in text)

check(repr(bound), 'A.meth', 'A object')

class Base:
    def method(self):
        pass

class Derived1(Base):
    pass

class Derived2(Base):
    def method(self):
        pass

base = Base()
derived1 = Derived1()
derived2 = Derived2()
check(repr(base.method), 'Base.method', 'Base object')
check(repr(derived1.method), 'Base.method', 'Derived1 object')
check(repr(derived2.method), 'Derived2.method', 'Derived2 object')
check(repr(super(Derived2, derived2).method), 'Base.method', 'Derived2 object')

class Foo:
    @classmethod
    def method(cls):
        pass

check(repr(Foo().method), 'Foo.method', '<class')
check(repr(Foo.method), 'Foo.method', '<class')
for expr in [lambda: bound.__get__(), lambda: bound.__get__(b, B, A)]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py::test_instance_method_get_behavior bound-method identity behavior",
            name: "bound-method-identity",
            source: r#"class C:
    def f(self):
        return self

c = C()
m = c.f
print(m is m)
print(m.__get__(c) is m, m.__get__(None, C) is m)
print(c.f is c.f, c.f == c.f, m == c.f, m.__get__(c) == m)
print(m.__get__(c).__self__ is c, m.__get__(None, C).__func__ is C.f)"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py / Lib/test/test_super.py C3 MRO subset",
            name: "c3-mro-super-chain",
            source: r#"class A:
    def f(self):
        return 'A'
class B(A):
    def f(self):
        return 'B' + super().f()
class C(A):
    def f(self):
        return 'C' + super().f()
class D(B, C):
    def f(self):
        return 'D' + super().f()
print(D().f())"#,
        },
        DiffCase {
            origin: "Lib/test/test_descr.py __slots__ subset",
            name: "slots-basic-attribute-restrictions",
            source: r#"class Point:
    __slots__ = ('x', 'y')
    def __init__(self, x, y):
        self.x = x
        self.y = y
    def total(self):
        return self.x + self.y

p = Point(2, 3)
print(p.x, p.y, p.total())
p.x = 5
print(p.total())
try:
    p.z = 9
except AttributeError:
    print('slot blocked')
del p.y
try:
    print(p.y)
except AttributeError:
    print('slot missing')
print(Point.x.__name__, Point.x.__doc__, Point.x is Point.x)
print(Point.x.__get__(None, Point))
try:
    Point.x.__get__(p, Point)
except AttributeError:
    print('descriptor missing')
Point.x.__set__(p, 7)
print(p.x, Point.x.__get__(p, Point))
Point.x.__delete__(p)
try:
    Point.x.__get__(p, Point)
except AttributeError:
    print('descriptor deleted')
try:
    Point.x.__set__(object(), 1)
except TypeError:
    print('descriptor type checked')

class Label:
    __slots__ = 'name'
l = Label()
l.name = 'mini'
print(l.name)
try:
    l.other = 1
except AttributeError:
    print('string slot blocked')

class WithDict:
    __slots__ = ('x', '__dict__')
w = WithDict()
w.x = 1
w.extra = 2
print(w.x, w.extra)

class Base:
    __slots__ = ('base',)
class Child(Base):
    __slots__ = ('child',)
c = Child()
c.base = 1
c.child = 2
try:
    c.other = 3
except AttributeError:
    print('inherited slots blocked')
print(c.base, c.child)

class OpenChild(Base):
    pass
o = OpenChild()
o.base = 4
o.other = 5
print(o.base, o.other)

class Plain:
    pass
class SlottedPlainChild(Plain):
    __slots__ = ('slot',)
sp = SlottedPlainChild()
sp.slot = 7
sp.extra = 8
print(sp.slot, sp.extra)

class DuplicateSlot:
    __slots__ = ('x', 'x')
d = DuplicateSlot()
d.x = 11
print(d.x)"#,
        },
        DiffCase {
            origin: "Lib/test/test_augassign.py::testBasic",
            name: "augassign-basic-operators",
            source: "x = 2\nx += 1\nx *= 2\nx **= 2\nx -= 8\nx //= 5\nx %= 3\nx &= 2\nx |= 5\nx ^= 1\nx /= 2\nprint(x)",
        },
        DiffCase {
            origin: "Lib/test/test_augassign.py::testInList",
            name: "augassign-list-subscript-operators",
            source: "x = [2]\nx[0] += 1\nx[0] *= 2\nx[0] **= 2\nx[0] -= 8\nx[0] //= 5\nx[0] %= 3\nx[0] &= 2\nx[0] |= 5\nx[0] ^= 1\nx[0] /= 2\nprint(x[0])",
        },
        DiffCase {
            origin: "Lib/test/test_augassign.py::testInDict",
            name: "augassign-dict-subscript-operators",
            source: "x = {0: 2}\nx[0] += 1\nx[0] *= 2\nx[0] **= 2\nx[0] -= 8\nx[0] //= 5\nx[0] %= 3\nx[0] &= 2\nx[0] |= 5\nx[0] ^= 1\nx[0] /= 2\nprint(x[0])",
        },
        DiffCase {
            origin: "Lib/test/test_augassign.py::testSequences",
            name: "augassign-sequence-concat-repeat-and-alias",
            source: "x = [1, 2]\nx += [3, 4]\nx *= 2\nprint(x)\ny = x\nx[1:2] *= 2\ny[1:2] += [1]\nprint(x, x is y)",
        },
        DiffCase {
            origin: "Lib/test/test_compare.py / object identity semantics",
            name: "mutable-container-identity-and-dict-mutation",
            source: "d = {1: 2}\ny = d\nd[3] = 4\ndel y[1]\nprint(d is y, len(d), 3 in d, 1 in d)\ns = {1}\nz = s\nprint(s is z)",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_update / ::test_copy",
            name: "dict-method-update-copy-get-pop-fromkeys",
            source: "d = {}\nd.update({1: 100})\nd.update({2: 20})\nd.update({1: 1, 2: 2, 3: 3})\nprint(d[1], d[2], d[3])\ncopy = d.copy()\nd[4] = 4\nprint(copy is d, copy == d, 4 in copy, 4 in d)\nd.update(x=5)\nprint(d.get(9, \"missing\"), d[\"x\"])\nprint(d.pop(2), 2 in d, d.pop(9, \"fallback\"))\nfromkeys = dict.fromkeys([\"a\", \"b\"], 7)\nprint(fromkeys[\"a\"], fromkeys[\"b\"])",
        },
        DiffCase {
            origin: "Lib/test/test_set.py set mutation/copy methods",
            name: "set-method-add-update-copy-discard-remove-clear",
            source: "s = set([1])\ncopy = s.copy()\ns.add(2)\ns.update([2, 3], {4})\ns.discard(99)\ns.remove(1)\nprint(copy is s, 1 in copy, 1 in s, 2 in s, 3 in s, 4 in s, len(s))\npopped = s.pop()\nprint(popped in [2, 3, 4], len(s))\ns.clear()\nprint(len(s))",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_setdefault / ::test_popitem / PEP 584 union",
            name: "dict-setdefault-popitem-and-union",
            source: "d = {1: 1}\nprint(d.setdefault(2, 20), d.setdefault(2, 99), d[2], len(d))\nmerged = d | {2: 22, 3: 3}\nprint(merged[1], merged[2], merged[3], 4 in merged)\nalias = d\nd |= {4: 4, 2: 222}\nprint(d is alias, d[2], d[4], len(d))\nkey, value = d.popitem()\nprint(key, value, len(d))",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py dict view objects",
            name: "dict-live-view-objects",
            source: "d = {1: 2, 3: 4}\nkeys = d.keys()\nvalues = d.values()\nitems = d.items()\nprint(keys)\nprint(values)\nprint(items)\nd[5] = 6\nprint(len(keys), 5 in keys, 6 in values, (5, 6) in items)\nprint(list(keys))",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_dictview_set_operations_on_keys / ::test_dictview_set_operations_on_items / ::test_dictview_mixed_set_operations",
            name: "dict-view-set-operations",
            source: "d = {1: 1}\nkeys = d.keys()\nvalues = d.values()\nitems = d.items()\nprint(len(keys), list(keys), list(values), list(items))\nd[2] = 2\nprint(len(keys), list(keys), list(values), list(items))\nprint(2 in keys, 2 in values, (2, 2) in items)\nprint(keys == {1, 2}, {1, 2} == keys, items == {(1, 1), (2, 2)})\nu = keys | {3}\nru = {0} | keys\ni = keys & {2, 4}\ndiff = keys - {1}\nxor = keys ^ {2, 3}\nitem_union = items | {(3, 3)}\nprint(len(u), 3 in u, len(ru), 0 in ru, len(i), 2 in i, len(diff), 2 in diff, len(xor), 1 in xor, 3 in xor)\nprint(len(item_union), (1, 1) in item_union, (3, 3) in item_union)",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py dict view set comparisons",
            name: "dict-view-set-comparisons",
            source: "small = {1: 1}.keys()\nlarge = {1: 1, 2: 2}.keys()\nprint(small < large, small <= large, large > small, large >= small)\nprint(small <= {1}, {1} >= small, {1} < large)\nsmall_items = {1: 1}.items()\nlarge_items = {1: 1, 2: 2}.items()\nprint(small_items < large_items, small_items <= {(1, 1)}, {(1, 1)} >= small_items)",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration existing-value update allowance",
            name: "dict-iterator-allows-existing-value-update",
            source: "d = {0: 0}\nfor key in d:\n    d[0] = 1\n    print(key, d[0])",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_reverse_iterator_for_empty_dict plus sequence reversed tests",
            name: "reversed-sequences-and-dict-views",
            source: "print(list(reversed([1, 2, 3])))\nprint(list(reversed((1, 2, 3))))\nprint(list(reversed(b'ab')))\nprint(list(reversed(range(1, 5))))\nd = {1: 2, 3: 4}\nprint(list(reversed(d)))\nprint(list(reversed(d.keys())))\nprint(list(reversed(d.values())))\nprint(list(reversed(d.items())))\nprint(list(reversed({})), list(reversed({}.keys())), list(reversed({}.values())), list(reversed({}.items())))",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py reverse iterator and Python data model __reversed__ sequence protocol",
            name: "reversed-custom-protocols",
            source: r#"class ReverseCustom:
    def __reversed__(self):
        return iter([3, 2, 1])
print(list(reversed(ReverseCustom())))
class SequenceFallback:
    def __init__(self):
        self.items = [10, 20, 30]
    def __len__(self):
        print('len')
        return len(self.items)
    def __getitem__(self, index):
        print('get', index)
        if index < 0 or index >= len(self.items):
            raise IndexError
        return self.items[index]
values = reversed(SequenceFallback())
print('made')
print(next(values), next(values), next(values, 'done'), next(values, 'done'))
class BadReverse:
    def __reversed__(self):
        return 42
print(reversed(BadReverse()))"#,
        },
        DiffCase {
            origin: "Lib/test/test_dict.py reverse iterator mutation behavior",
            name: "dict-reverse-iterator-same-size-key-change",
            source: "d = {0: 0, 1: 1, 2: 2}\nfor key in reversed(d):\n    print(key)\n    if key == 2:\n        del d[0]\n        d[0] = 0\nprint(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py reverse values/items iterator behavior",
            name: "dict-reverse-view-live-values-and-same-size-key-change",
            source: "d = {0: 0, 1: 1, 2: 2}\nvalues = reversed(d.values())\nprint(next(values))\nd[0] = 9\nprint(next(values))\nprint(next(values))\nd = {0: 0, 1: 1, 2: 2}\nfor item in reversed(d.items()):\n    print(item)\n    if item == (2, 2):\n        del d[1]\n        d[1] = 1\nprint(\"done\")",
        },
        DiffCase {
            origin: "Lib/test/test_tuple.py::TupleTest::test_constructors",
            name: "tuple-constructor",
            source: "print(tuple())\nprint(tuple([]))\nprint(tuple([0, 1, 2, 3]))\nprint(tuple('') == ())\nletters = tuple('spam')\nprint(len(letters), letters[0], letters[1], letters[2], letters[3])\nprint(tuple(x for x in range(10) if x % 2))",
        },
        DiffCase {
            origin: "Lib/test/test_bool.py::BoolTest::test_bool / ::test_int / ::test_float / ::test_str and test_float.py::GeneralFloatCases::test_float",
            name: "scalar-builtin-constructors",
            source: "print(bool(), bool(0), bool(1), bool(''), bool('x'), bool([]), bool([0]))\nprint(int(), int(False), int(True), int(3.9), int(-3.9), int(' 42 '), int(b'7'))\nprint(float(), float(False), float(True), float(314), float('  3.14  '), float(b'2.5'))\nprint(str(), str(False), str(True), str(None), str(12), str(1.5), str('mini'), str(b'ab'))",
        },
        DiffCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_non_numeric_input_types bytearray subset",
            name: "int-constructor-bytearray-input",
            source: r#"value = bytearray(b"100")
print(bytearray(), value, bytes(value), len(value), list(value))
print(int(value), int(value, 2), float(bytearray(b"2.5")), isinstance(value, bytearray))
print(bytearray(3), bytearray(value), value == bytearray(b"100"), value == b"100")
try:
    int(bytearray(b"A" * 0x10))
except ValueError as error:
    print(error.__class__.__name__, error)"#,
        },
        DiffCase {
            origin: "Lib/test/test_int.py / test_float.py / test_index.py numeric conversion protocol subset",
            name: "custom-numeric-conversion-protocols",
            source: r#"class IntOnly:
    def __int__(self):
        return 7
class IndexOnly:
    def __index__(self):
        return 4
class FloatOnly:
    def __float__(self):
        return 2.5
class IntOverridesTrunc:
    def __int__(self):
        return 42
    def __trunc__(self):
        return -12
print(int(IntOnly()), int(IndexOnly()))
print(int(IntOverridesTrunc()))
print(float(FloatOnly()), float(IndexOnly()))
print(list(range(IndexOnly())))
print(bytes(IndexOnly()))
items = [10, 20, 30, 40, 50]
print(items[IndexOnly()], items[IndexOnly():], items[:IndexOnly()])
print(list(enumerate([99], IndexOnly())))"#,
        },
        DiffCase {
            origin: "Lib/test/test_int.py int base conversion subset",
            name: "int-constructor-with-base",
            source: r#"class BaseTwo:
    def __index__(self):
        return 2
print(int("10", 2), int("0b101", 0), int("+0xF", 0), int("-10", 2))
print(int(b"11", 2), int("z", 36), int("10", BaseTwo()), int("10", base=2))
print(int("1_0"), int("0b_1", 0), int("0x_f", 0), int("0o_7", 0))"#,
        },
        DiffCase {
            origin: "Lib/test/test_int.py::test_issue31619",
            name: "int-constructor-underscore-digits-in-non-decimal-bases",
            source: r#"print(int("1_0_1_0_1_0_1_0_1_0_1_0_1_0_1_0_1_0_1_0_1_0_1_0_1_0_1_0_1_0_1", 2))
print(int("1_2_3_4_5_6_7_0_1_2_3", 8))
print(int("1_2_3_4_5_6_7_8_9", 16))
print(int("1_2_3_4_5_6_7", 32))"#,
        },
        DiffCase {
            origin: "Lib/test/test_int.py::test_invalid_signs",
            name: "int-constructor-rejects-sign-only-and-space-separated-signs",
            source: r#"for text in ["+", "-", "- 1", "+ 1", " + 1 "]:
    try:
        int(text)
    except ValueError as error:
        print(error.__class__.__name__, error)"#,
        },
        DiffCase {
            origin: "Lib/test/test_int.py::test_underscores constructor-only cases",
            name: "int-constructor-underscore-only-valid-and-invalid-forms",
            source: r#"print(int("1_00", 3), int("0_100"), int(b"1_00"))
for text in ["_100", "+_100", "1__00", "100_"]:
    try:
        int(text)
    except ValueError as error:
        print(error.__class__.__name__, error)"#,
        },
        DiffCase {
            origin: "Lib/test/test_int.py base conversion regression for 2**32",
            name: "int-constructor-base-conversions-two-to-thirty-six",
            source: r#"cases = [
    ("100000000000000000000000000000000", 2),
    ("102002022201221111211", 3),
    ("10000000000000000", 4),
    ("32244002423141", 5),
    ("1550104015504", 6),
    ("211301422354", 7),
    ("40000000000", 8),
    ("12068657454", 9),
    ("4294967296", 10),
    ("1904440554", 11),
    ("9ba461594", 12),
    ("535a79889", 13),
    ("2ca5b7464", 14),
    ("1a20dcd81", 15),
    ("100000000", 16),
    ("a7ffda91", 17),
    ("704he7g4", 18),
    ("4f5aff66", 19),
    ("3723ai4g", 20),
    ("281d55i4", 21),
    ("1fj8b184", 22),
    ("1606k7ic", 23),
    ("mb994ag", 24),
    ("hek2mgl", 25),
    ("dnchbnm", 26),
    ("b28jpdm", 27),
    ("8pfgih4", 28),
    ("76beigg", 29),
    ("5qmcpqg", 30),
    ("4q0jto4", 31),
    ("4000000", 32),
    ("3aokq94", 33),
    ("2qhxjli", 34),
    ("2br45qb", 35),
    ("1z141z4", 36),
]
for text, base in cases:
    print(int(text, base))"#,
        },
        DiffCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_unicode",
            name: "int-constructor-unicode-decimal-digits",
            source: r#"print(int("१२३४५६७८९०1234567890"))
print(int("١٢٣٤٥٦٧٨٩٠"))
print(int("१२३४५६७८९۰1234567890", 0))
print(int("١٢٣٤٥٦٧٨٩٠", 0))
print(int("１２_３"), int("0b١٠", 0), int("0o٧", 0), int("0x١f", 0))"#,
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py unicode identifier coverage",
            name: "unicode-identifiers-and-f-string-debug-label",
            source: "tenπ = 31.4\n变量 = 8\ndef 加一(x):\n    return x + 1\nclass 盒子:\n    pass\nK = 7\nprint(K)\nK = 8\nprint(K)\nｘ = 3\nprint(x)\nµ = 5\nprint(μ)\nprint(tenπ, 变量, 加一(4), 盒子.__name__)\nprint(f'{tenπ=:.2f}')\nprint(f'{K=}', f'{K=}')",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format arbitrary-precision old-style integers",
            name: "old-style-percent-bigint-formatting",
            source: r#"big = 123456789012345678901234567890
print('%d' % big)
print('%d' % -big)
print('%32d' % -big)
print('%-32d' % -big)
print('%032d' % -big)
print('%034d' % big)
print('%0+34d' % big)
print('%32.31d' % big)
big = 0x1234567890abcdef12345
print('%x' % big)
print('%x' % -big)
print('%23x' % -big)
print('%-23x' % -big)
print('%023x' % -big)
print('%025x' % big)
print('%0+25x' % big)
print('%23.22x' % big)
print('%X' % big)
print('%#X' % big)
print('%#x' % -big)
print('%#027x' % big)
print('%#.23x' % -big)
print('%#-27.23x' % big)
print('%#027.23x' % big)
print('%#+.23x' % big)
print('%# .23x' % big)
print('%#+.23X' % big)
print('%#+027.23X' % big)
print('%# 027.23X' % big)
print('%#+27.23X' % big)
print('%#-+27.23x' % big)
print('%#- 27.23x' % big)
big = 0o12345670123456701234567012345670
print('%o' % big)
print('%o' % -big)
print('%34o' % -big)
print('%-34o' % -big)
print('%034o' % -big)
print('%036o' % big)
print('%0+36o' % big)
print('%34.33o' % big)
print('%#o' % big)
print('%#o' % -big)
print('%#038o' % big)
print('%#.34o' % -big)
print('%#-38.34o' % big)
print('%#+.34o' % big)
print('%# .34o' % big)
print('%#+038.34o' % big)
print('%# 038.34o' % big)
print('%#.33o' % big)
print('%0#35.33o' % big)
print('%d' % 42)
print('%d' % -42)
print('%d' % 42.0)
print('%#x %#X %#o' % (1, 1, 1))
print('%#o %o %d %#x %#X' % (0, 0, 0, 0, 0))
print('%x %x %o %o' % (0x42, -0x42, 0o42, -0o42))
print('%g %#g' % (1.1, 1.1))"#,
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py custom __format__ lookup and Lib/test/test_builtin.py::test_format",
            name: "format-builtin-and-custom-dunder-format",
            source: r#"print(format(3, ''), format('x'), format(1.25, '.1f'))
print(format(17**13, ''), format(17**13) == str(17**13))
print(format(1.0, ''), format(3.1415e104, ''), format(-3.1415e104, ''))
print(format(3.1415e-104, ''), format(-3.1415e-104, ''))
print(format(object, ''), format(object) == str(object), format(None, ''), format(None) == str(None))
class CustomFormat:
    def __format__(self, format_spec):
        return format_spec
print(f'{CustomFormat():abc}')
print(format(CustomFormat(), 'xyz'))
class A(object):
    def __init__(self, x):
        self.x = x
    def __format__(self, format_spec):
        return str(self.x) + format_spec
class DerivedFromA(A):
    pass
class Simple(object):
    pass
class DerivedFromSimple(Simple):
    def __init__(self, x):
        self.x = x
    def __format__(self, format_spec):
        return str(self.x) + format_spec
class DerivedFromSimple2(DerivedFromSimple):
    pass
print(format(A(3), 'spec'))
print(format(DerivedFromA(4), 'spec'))
print(format(DerivedFromSimple(5), 'abc'))
print(format(DerivedFromSimple2(10), 'abcdef'))
class X:
    def __init__(self):
        self.i = 0
    def __format__(self, spec):
        self.i += 1
        return str(self.i) + spec
x = X()
print(f'{x} {x:!}')
class Y:
    def __format__(self, spec):
        return 'class:' + spec
y = Y()
y.__format__ = lambda spec: 'instance:' + spec
print(y.__format__('direct'))
print(format(y, 'real'))
print(f'{y:field}')
class B:
    pass
b = B()
print(format(b) != '', b.__format__('') != '')
class EmptyDelegates:
    def __format__(self, fmt_str):
        return format('', fmt_str)
print(format(EmptyDelegates()), format(EmptyDelegates(), ''), format(EmptyDelegates(), 's'))
class DerivedFromStr(str):
    pass
spec = DerivedFromStr('10')
print(str(spec), repr(spec), len(spec), bool(spec), isinstance(spec, str), type(spec).__name__)
print(format(0, spec))
class ReceivesSpec:
    def __format__(self, format_spec):
        return type(format_spec).__name__ + ' ' + str(isinstance(format_spec, str)) + ' ' + repr(format_spec)
print(format(ReceivesSpec(), spec))
print(object().__format__(DerivedFromStr(''))[:14])
try:
    format(b, 's')
except TypeError as error:
    print(error.__class__.__name__, 'B.__format__' in str(error))
try:
    object().__format__(3)
except TypeError as error:
    print(error.__class__.__name__)
for value in [object(), None]:
    try:
        object().__format__(value)
    except TypeError as error:
        print(error.__class__.__name__)
try:
    format(object(), object())
except TypeError as error:
    print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_types.py::TypesTests::test_float_to_string",
            name: "types-float-to-string-and-numeric-dunder-format",
            source: r#"def expected_exp(i):
    sign = '+' if i >= 0 else '-'
    magnitude = abs(i)
    if magnitude < 10:
        return '1.500000e' + sign + '0' + str(magnitude)
    return '1.500000e' + sign + str(magnitude)
checked = 0
for i in range(-99, 100):
    f = float('1.5e' + str(i))
    expected = expected_exp(i)
    for actual in [f.__format__('e'), float.__format__(f, 'e'), '%e' % f]:
        assert actual == expected
        checked += 1
for f, expected in [(1.5e100, '1.500000e+100'), (1.5e101, '1.500000e+101'), (1.5e-100, '1.500000e-100'), (1.5e-101, '1.500000e-101')]:
    for actual in [f.__format__('e'), float.__format__(f, 'e'), '%e' % f]:
        assert actual == expected
        checked += 1
print(checked)
print('%g' % 1.0, '%#g' % 1.0)
print((1).__format__('d'), int.__format__(1, '04d'), True.__format__(''), True.__format__('d'), bool.__format__(False, 'd'), (1.0).__format__('e'))
print('__format__' in dir(1), '__format__' in dir(1.0), '__format__' in dir(True), '__format__' in dir(int), '__format__' in dir(float))
for expr in [lambda: (1).__format__(1), lambda: (1.0).__format__(1), lambda: float.__format__(1, 'e')]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_types.py::TypesTests::test_normal_integers public arithmetic rows",
            name: "types-normal-integers-public-arithmetic",
            source: r#"import sys
print('add', 12 + 24, 12 + (-24), (-12) + 24, (-12) + (-24))
print('compare', 12 < 24, -24 < -12)
xsize, ysize, zsize = 238, 356, 4
print('mul', xsize * ysize * zsize == zsize * xsize * ysize, xsize * ysize * zsize)
m = -sys.maxsize - 1
min_exact = []
for divisor in (1, 2, 4, 8, 16, 32):
    j = m // divisor
    prod = divisor * j
    min_exact.append((prod == m, type(prod) is int))
print('min-exact', min_exact)
min_under = []
for divisor in (1, 2, 4, 8, 16, 32):
    j = m // divisor - 1
    prod = divisor * j
    min_under.append((prod < m, type(prod) is int))
print('min-under', min_under)
m = sys.maxsize
max_over = []
for divisor in (1, 2, 4, 8, 16, 32):
    j = m // divisor + 1
    prod = divisor * j
    max_over.append((prod > m, type(prod) is int))
print('max-over', max_over)
x = sys.maxsize
print('instances', isinstance(x + 1, int), isinstance(-x - 1, int), isinstance(-x - 2, int))
for label, expr in [('left', lambda: 5 << -5), ('right', lambda: 5 >> -5)]:
    try:
        expr()
    except ValueError as error:
        print('shift', label, error.__class__.__name__, str(error))"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_ascii",
            name: "ascii-builtin",
            source: r#"print(ascii(''))
print(ascii(0), ascii(()), ascii([]), ascii({}))
a = []
a.append(a)
print(ascii(a))
d = {}
d[0] = d
print(ascii(d))
for item in ["'", '"', '"\'', '\0', '\r\n\t .']:
    print(ascii(item))
for item in ['\x85', '\u1fff', '\U00012fff', '\U0001d121', 'é']:
    print(ascii(item))
supplement = '\U0001d121'
print(f'{"é"!a}', f'{supplement!a}')
for expr in [lambda: ascii(), lambda: ascii(1, 2)]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_chr / ::test_ord",
            name: "chr-ord-builtins",
            source: r#"print(chr(65), chr(97))
print(ord(' '), ord('A'), ord('a'), ord('\xff'))
print(ord(b'A'), ord(bytearray(b'\xff')))
print(ord(chr(0)), ord(chr(32)), ord(chr(0xff)), ord(chr(0x10ffff)))
for expr in [lambda: chr(), lambda: chr(65.0), lambda: chr(-1), lambda: chr(0x110000), lambda: ord(), lambda: ord(42), lambda: ord('ab'), lambda: ord(b'ab')]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_type / TestType::test_bad_args",
            name: "type-builtin",
            source: r#"print(type('').__name__, type('123').__name__, type(()).__name__)
print(type('') == type('123'), type('') != type(()))
print(type(None).__name__, type(True).__name__, type(1).__name__, type(1.5).__name__)
print(type([]).__name__, type({}).__name__, type(set()).__name__)
print(type(type('')).__name__, type(len).__name__, type(type(len)).__name__)
class A:
    pass
a = A()
print(type(a) is A, a.__class__ is A, type(A).__name__)
for name in ['A', 'Ä', '🐍', 'B.A', '42', '']:
    A = type(name, (), {})
    print(A.__name__, A.__qualname__, A.__module__)
class Base:
    label = 'base'
def dyn_method(self):
    return self.value + 1
Dynamic = type('Dynamic', (Base,), {'value': 7, 'plus': dyn_method})
inst = Dynamic()
print(Dynamic.__name__, Dynamic.__qualname__, Dynamic.__module__)
print(Dynamic.__bases__[0] is Base, Dynamic.label, inst.value, inst.plus())
for expr in [lambda: type(), lambda: type('A', ()), lambda: type('A', (), {}, extra=1), lambda: type(obj=1), lambda: type(b'A', (), {}), lambda: type('A\0B', (), {}), lambda: type('A', [], {}), lambda: type('A', (), [])]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestType::test_new_type / ::test_bad_args",
            name: "type-dynamic-class",
            source: r#"A = type('A', (), {})
a = A()
print(A.__name__, A.__qualname__, A.__module__)
print(A.__bases__[0].__name__, A.__base__.__name__, '__firstlineno__' in A.__dict__)
print(type(a) is A, a.__class__ is A)
class B:
    def ham(self):
        return 'ham'
C = type('C', (B,), {'spam': lambda self: 'spam'})
x = C()
print(C.__name__, C.__qualname__, C.__module__)
print(C.__bases__[0] is B, C.__base__ is B, 'spam' in C.__dict__, 'ham' in C.__dict__)
print(type(x) is C, x.__class__ is C, x.ham(), x.spam())
import types
for expr in [lambda: type('A', [], {}), lambda: type('A', (), []), lambda: type('A', (), {}, ()), lambda: type('A', (), types.MappingProxyType({})), lambda: type(b'A', (), {}), lambda: type('A\0B', (), {}), lambda: type('A', (None,), {}), lambda: type('A', (bool,), {}), lambda: type('A', (int, str), {})]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestType::test_type_nokwargs",
            name: "type-nokwargs",
            source: r#"for expr in [lambda: type('a', (), {}, x=5), lambda: type('a', (), dict={})]:
    try:
        expr()
        print('ok')
    except TypeError as error:
        print(type(error).__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestType::test_namespace_order",
            name: "type-namespace-order",
            source: r#"from collections import OrderedDict
od = OrderedDict([('a', 1), ('b', 2)])
od.move_to_end('a')
expected = list(od.items())
C = type('C', (), od)
print(expected)
print(list(C.__dict__.items())[:2])
print(expected == list(C.__dict__.items())[:2])
print(type(od).__name__, 'move_to_end' in dir(od), 'move_to_end' in dir({}))"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestType::test_type_name / ::test_type_qualname",
            name: "type-name-qualname",
            source: r#"C = type('C', (), {})
for name in ['A', 'Ä', '🐍', 'B.A', '42', '']:
    C.__name__ = name
    print(C.__name__, C.__qualname__, C.__module__)
A = type('A', (), {'__qualname__': 'B.C'})
print(A.__name__, A.__qualname__, A.__module__, '__qualname__' in A.__dict__)
A.__qualname__ = 'D.E'
print(A.__name__, A.__qualname__)
A = type('A', (), {'__name__': 'B'})
print(A.__name__, A.__dict__['__name__'])
A.__name__ = 'C'
print(A.__name__, A.__dict__['__name__'])
A = type('C', (), {})
for expr in [lambda: setattr(A, '__name__', b'A'), lambda: setattr(A, '__name__', 'A\0B'), lambda: type('A', (), {'__qualname__': b'B'}), lambda: setattr(A, '__qualname__', b'B')]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__, A.__name__, A.__qualname__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestType::test_type_doc / ::test_type_firstlineno",
            name: "type-doc-firstlineno",
            source: r#"for doc in ['x', 'Ä', '🐍', 'x\0y', b'x', 42, None]:
    A = type('A', (), {'__doc__': doc})
    print(A.__doc__)
A = type('A', (), {})
print(A.__doc__)
for doc in ['x', 'Ä', '🐍', 'x\0y', b'x', 42, None]:
    A.__doc__ = doc
    print(A.__doc__)
A = type('A', (), {'__firstlineno__': 42})
print(A.__name__, A.__module__, A.__dict__['__firstlineno__'], A.__firstlineno__)
A.__module__ = 'testmodule'
print(A.__module__)
A.__firstlineno__ = 43
print(A.__dict__['__firstlineno__'], A.__firstlineno__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_dir / ::test_vars",
            name: "vars-dir-builtins",
            source: r#"def local_probe():
    local_var = 1
    print('local_var' in dir(), 'local_var' in vars())
    return 'local_var' in vars()
print(local_probe())
import sys
print(set(vars(sys)) == set(dir(sys)), 'path' in dir(sys), '__name__' in vars(sys))
sys.__dict__['live_probe'] = 7
print(sys.live_probe)
vars(sys)['live_probe2'] = 8
print(sys.live_probe2)
del sys.__dict__['live_probe']
print(hasattr(sys, 'live_probe'), 'live_probe2' in dir(sys))
sys.__dict__['__name__'] = 'renamed_sys'
print(sys.__name__, vars(sys)['__name__'])
print('strip' in dir(str), '__mro__' in dir(str))
class Box:
    class_attr = 3
    def __init__(self):
        self.y = 8
box = Box()
print('y' in dir(box), 'class_attr' in dir(box), sorted(vars(box).keys()))
print('class_attr' in vars(Box), '__module__' in vars(Box))
class TupleDir:
    def __dir__(self):
        return ('b', 'c', 'a')
print(dir(TupleDir()))
class DictProperty:
    def getDict(self):
        return {'a': 2}
    __dict__ = property(getDict)
print(vars(DictProperty()))
class BadDir:
    def __dir__(self):
        return 7
for expr in [lambda: dir(1, 2), lambda: vars(1, 2), lambda: vars(42), lambda: dir(BadDir())]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)
print(sorted([].__dir__()) == dir([]))
print(sorted(object.__dir__([])) == dir([]), '__dir__' in dir([]))"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py namespace builtins and Lib/test/test_scope.py locals behavior",
            name: "globals-locals-builtins",
            source: r#"x = 1
g = globals()
l = locals()
print(g is l, g['x'], l['x'])
g['from_globals'] = 2
l['from_locals'] = 3
print(from_globals, from_locals, globals() is g, locals() is l)
def probe(arg):
    local_value = 4
    snapshot = locals()
    print('arg' in snapshot, snapshot['arg'], 'local_value' in snapshot, snapshot['local_value'])
    print(globals() is locals(), globals()['x'])
probe(3)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_eval",
            name: "eval-builtin",
            source: r#"print(eval('1+1'))
print(eval(' 1+1\n'))
print(eval('"Ä"'))
x = 10
def probe(y):
    z = 5
    print(eval('x + y + z'))
    print(eval('globals()["x"]'), eval('locals()["z"]'))
probe(3)
data = {'a': 1, 'b': 2}
local = {'b': 200, 'c': 300}
print(eval('a', data), eval('a', data, local))
print(eval('b', data, local), eval('c', data, local))
print(eval('globals()["a"]', data, local), eval('locals()["c"]', data, local))
print(eval("print('inside')"))
for expr in [lambda: eval(), lambda: eval((), data), lambda: eval('1+'), lambda: eval('a', ()), lambda: eval('a', data, ())]:
    try:
        expr()
    except (TypeError, SyntaxError) as error:
        print(error.__class__.__name__)
g = {}
try:
    eval('x =', g)
except SyntaxError:
    print('__builtins__' in g)
g = {}
try:
    eval((), g)
except TypeError:
    print('__builtins__' in g)
g = {}
print(eval('(x := 4)', g, g), g['x'], '__builtins__' in g)
g = {}
l = {}
print(eval('(x := 5)', g, l), 'x' in g, l['x'])
g = {}
try:
    eval('(x := 6) / 0', g, g)
except ZeroDivisionError:
    print(g['x'])
class M:
    def __getitem__(self, key):
        if key == 'a':
            return 12
        raise KeyError
    def keys(self):
        return list('xyz')
m = M()
g = globals()
print(eval('a', g, m))
try:
    eval('b', g, m)
except NameError as error:
    print(error.__class__.__name__)
print(eval('dir()', g, m))
print(eval('globals()', g, m) is g, eval('locals()', g, m) is m)
try:
    eval('a', m)
except TypeError as error:
    print(error.__class__.__name__)
class A:
    pass
try:
    eval('a', g, A())
except TypeError as error:
    print(error.__class__.__name__)
class D(dict):
    def __getitem__(self, key):
        if key == 'a':
            return 12
        return dict.__getitem__(self, key)
    def keys(self):
        return list('xyz')
d = D()
print(eval('a', g, d))
try:
    eval('b', g, d)
except NameError as error:
    print(error.__class__.__name__)
print(eval('dir()', g, d))
print(eval('locals()', g, d) is d)
class SpreadSheet:
    _cells = {}
    def __setitem__(self, key, formula):
        self._cells[key] = formula
    def __getitem__(self, key):
        return eval(self._cells[key], globals(), self)
ss = SpreadSheet()
ss['a1'] = '5'
ss['a2'] = 'a1*6'
ss['a3'] = 'a2*7'
print(ss['a3'])
class C:
    def __getitem__(self, item):
        raise KeyError(item)
    def keys(self):
        return 1
try:
    eval('dir()', globals(), C())
except TypeError as error:
    print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_compile.py::TestSpecifics::test_encoding",
            name: "compile-specifics-encoding",
            source: r#"try:
    compile(b'# -*- coding: badencoding -*-\npass\n', 'tmp', 'exec')
except SyntaxError:
    print('bad-bytes-cookie')
code = '# -*- coding: badencoding -*-\n"\xc2\xa4"\n'
compile(code, 'tmp', 'exec')
print(eval(code))
code = '"\xc2\xa4"\n'
print(eval(code))
print(eval(b'"\xc2\xa4"\n'))
print(eval(b'# -*- coding: latin1 -*-\n"\xc2\xa4"\n'))
print(eval(b'# -*- coding: utf-8 -*-\n"\xc2\xa4"\n'))
print(eval(b'# -*- coding: iso8859-15 -*-\n"\xc2\xa4"\n'))
code = '"""\\\n# -*- coding: iso8859-15 -*-\n\xc2\xa4"""\n'
print(eval(code))
print(eval(b'"""\\\n# -*- coding: iso8859-15 -*-\n\xc2\xa4"""\n'))"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_exec",
            name: "exec-builtin",
            source: r#"print(exec('z = 1'))
print(z)
exec('z = z + 1')
print(z)
g = {}
print(exec('z = 1', g))
print(g['z'])
exec('z = 1 + 1', g)
print(g['z'])
g = {}
l = {}
exec('global a\na = 1\nb = 2', g, l)
print(g['a'], 'b' in g, l['b'])
for expr in [lambda: exec(), lambda: exec((), g), lambda: exec('x ='), lambda: exec('x = 1', ())]:
    try:
        expr()
    except (TypeError, SyntaxError) as error:
        print(error.__class__.__name__)
g = {}
try:
    exec('x = 1\n1/0', g)
except ZeroDivisionError:
    print(g['x'], '__builtins__' in g)
g = {}
l = {}
try:
    exec('y = 2\n1/0', g, l)
except ZeroDivisionError:
    print('y' in g, l['y'], '__builtins__' in g, '__builtins__' in l)
g = {}
try:
    exec('x =', g)
except SyntaxError:
    print('__builtins__' in g, 'x' in g)
g = {}
try:
    exec((), g)
except TypeError:
    print('__builtins__' in g)
g = {}
exec('x = 1', g, g)
print(g['x'], '__builtins__' in g)
g = {}
exec('global y\ny = 2\nz = 3', g, g)
print(g['y'], g['z'])
g = {}
try:
    exec('x = 4\n1/0', g, g)
except ZeroDivisionError:
    print(g['x'])
import sys
saved = sys.stdout
sys.stdout = None
try:
    exec('a')
except NameError as error:
    sys.stdout = saved
    print(error.__class__.__name__, 'a' in str(error))
finally:
    sys.stdout = saved"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestBreakpoint custom breakpointhook rows",
            name: "builtin-breakpoint-custom-hook",
            source: r#"import builtins, sys
print(hasattr(builtins, 'breakpoint'), callable(builtins.breakpoint))
print(hasattr(sys, 'breakpointhook'), hasattr(sys, '__breakpointhook__'), sys.breakpointhook is sys.__breakpointhook__)
def hook(*args, **kwargs):
    print('hook', args, kwargs)
    return 'ret'
saved = sys.breakpointhook
sys.breakpointhook = hook
print('call0', breakpoint())
print('callargs', breakpoint(1, 'x', key=3))
print('module-call', builtins.breakpoint())
del sys.breakpointhook
try:
    breakpoint()
except RuntimeError as error:
    print('lost', type(error).__name__, str(error))
finally:
    sys.breakpointhook = saved
print('reset-same', sys.breakpointhook is sys.__breakpointhook__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestBreakpoint passthru error rows",
            name: "builtin-breakpoint-passthru-error",
            source: r#"import sys
def hook(required):
    return required
saved = sys.breakpointhook
sys.breakpointhook = hook
try:
    for label, callback in [
        ('missing', lambda: breakpoint()),
        ('extra', lambda: breakpoint(1, 2)),
        ('keyword', lambda: breakpoint(required=1)),
        ('unknown', lambda: breakpoint(unknown=1)),
    ]:
        try:
            print(label, callback())
        except Exception as error:
            print(label, type(error).__name__, isinstance(error, TypeError))
finally:
    sys.breakpointhook = saved"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_exec_globals_dict_subclass / ::test_exec_builtins_mapping_import",
            name: "exec-eval-builtins-mapping",
            source: r#"class customdict(dict):
    pass
code = compile('result = superglobal', 'test', 'exec')
ns = {'__builtins__': customdict({'superglobal': 1})}
exec(code, ns)
print(ns['result'])
code = compile('import foo.bar', 'test', 'exec')
ns = {'__builtins__': {'__import__': lambda *args: args}}
exec(code, ns)
print(ns['foo'][0], ns['foo'][1] is ns['foo'][2], ns['foo'][3], ns['foo'][4])
class setonlyerror(Exception):
    pass
class setonlydict(dict):
    def __getitem__(self, key):
        raise setonlyerror
try:
    exec(compile('globalname', 'test', 'exec'), setonlydict({'globalname': 1}))
except setonlyerror:
    print('globals getitem error')
try:
    exec(compile('superglobal', 'test', 'exec'), {'__builtins__': setonlydict({'superglobal': 1})})
except setonlyerror:
    print('builtins getitem error')
ns = {}
exec('value = len([1, 2])', ns)
print('__builtins__' in ns, 'len' in ns['__builtins__'], ns['value'])
ns = {}
print(eval('len([1, 2, 3])', ns))
print('__builtins__' in ns, 'len' in ns['__builtins__'])
g = {}
l = {}
exec('value = len([1])', g, l)
print('__builtins__' in g, '__builtins__' in l, l['value'])
class M:
    def __getitem__(self, key):
        if key == 'a':
            return 12
        raise KeyError
    def __setitem__(self, key, value):
        self.results = (key, value)
    def keys(self):
        return list('xyz')
m = M()
g = globals()
exec('z = a', g, m)
print(m.results)
try:
    exec('z = b', g, m)
except NameError:
    print('name-error')
exec('z = dir()', g, m)
print(m.results)
exec('z = locals()', g, m)
print(m.results[0], m.results[1] is m)
g = {}
exec('import math\nname = math.__name__', g)
print(g['name'], '__import__' in g['__builtins__'])
import builtins
class frozendict_error(Exception):
    pass
class frozendict(dict):
    def __setitem__(self, key, value):
        raise frozendict_error('frozendict is readonly')
frozen_builtins = frozendict({'__build_class__': builtins.__build_class__, 'print': print})
print(hasattr(builtins, '__build_class__'))
code = compile("__builtins__['superglobal']=2; print(superglobal)", 'test', 'exec')
try:
    exec(code, {'__builtins__': frozen_builtins, '__name__': 'test'})
except frozendict_error as error:
    print('builtins-write', error.__class__.__name__, error)
class keyfrozendict(dict):
    def __setitem__(self, key, value):
        raise frozendict_error('readonly ' + key)
namespace = keyfrozendict({'__name__': 'test'})
code = compile('x=1', 'test', 'exec')
try:
    exec(code, namespace)
except frozendict_error as error:
    print('globals-write', error.__class__.__name__, error)
print('globals-clean', 'x' in namespace)
code = compile('class A: pass', '', 'exec')
try:
    exec(code, {'__builtins__': {}, '__name__': 'test'})
except NameError as error:
    print('no-build-class', error.__class__.__name__, '__build_class__' in str(error))
try:
    exec(code, {'__builtins__': frozendict(), '__name__': 'test'})
except NameError as error:
    print('empty-frozen-build-class', error.__class__.__name__, '__build_class__' in str(error))
ns = {'__builtins__': frozen_builtins, '__name__': 'test'}
exec(code, ns)
print('build-class-ok', ns['A'].__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_import",
            name: "import-builtin",
            source: r#"for module in [__import__('sys'), __import__('time'), __import__('string'), __import__(name='sys'), __import__(name='time', level=0)]:
    print(module.__name__)
for label, fn, fragment in [
    ('missing', lambda: __import__('spamspam'), 'spamspam'),
    ('non-str', lambda: __import__(1, 2, 3, 4), 'str'),
    ('empty', lambda: __import__(''), 'Empty module name'),
    ('duplicate-name', lambda: __import__('sys', name='sys'), 'name'),
    ('null', lambda: __import__('string\x00'), 'string'),
]:
    try:
        fn()
    except (ModuleNotFoundError, TypeError, ValueError) as error:
        print(label, error.__class__.__name__, fragment in str(error))"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_compile",
            name: "compile-code-object-builtin",
            source: r#"code = compile('1 + 2', '<mini>', 'eval')
print(type(code).__name__)
print(eval(code))
code = compile('value = 4\nprint(value)', '<mini>', 'exec')
print(exec(code))
print(exec(compile(b'\xef\xbb\xbfprint(6)\n', '<mini>', 'exec')))
print(exec(compile('2 + 3', '<mini>', 'single')))
print(eval(compile('z = 9', '<mini>', 'exec')))
print(z)
print(eval(compile(source='a + b', filename='tmp', mode='eval'), {'a': 2}, {'b': 5}))
compile('pass', '?', dont_inherit=True, mode='exec')
compile(dont_inherit=False, filename='tmp', source='0', mode='eval')
for expr in [lambda: compile(), lambda: compile('print(42)', '<string>', 'badmode'), lambda: compile('x =', '<string>', 'exec'), lambda: compile('pass', '?', 'exec', mode='eval', source='0', filename='tmp')]:
    try:
        expr()
    except (TypeError, ValueError, SyntaxError) as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_hash",
            name: "hash-builtin",
            source: r#"print(type(hash(None)).__name__)
print(hash(1) == hash(1), hash(1) == hash(1.0), hash(True) == hash(1))
print(type(hash('spam')).__name__, hash('spam') == hash(b'spam'))
print(type(hash((0, 1, 2, 3))).__name__)
def f(): pass
print(type(hash(f)).__name__)
class X:
    def __hash__(self):
        return 2 ** 100
class Bad:
    def __hash__(self):
        return 1.0
class NoHash:
    __hash__ = None
print(type(hash(X())).__name__)
for expr in [lambda: hash(), lambda: hash(1, 2), lambda: hash([]), lambda: hash({}), lambda: hash(([1],)), lambda: hash(Bad()), lambda: hash(NoHash())]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_id",
            name: "id-builtin",
            source: r#"print(type(id(None)).__name__, type(id(1)).__name__, type(id(1.0)).__name__)
print(type(id('spam')).__name__, type(id((0, 1, 2, 3))).__name__)
items = [0, 1, 2, 3]
alias = items
other = [0, 1, 2, 3]
print(id(items) == id(alias), id(items) == id(other))
d = {'spam': 1, 'eggs': 2, 'ham': 3}
print(type(id(d)).__name__, id(d) == id(d))
for expr in [lambda: id(), lambda: id(1, 2)]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_len",
            name: "len-builtin",
            source: r#"print(len('123'), len(()), len((1, 2, 3, 4)), len([1, 2, 3, 4]))
print(len({}), len({'a': 1, 'b': 2}))
import sys
class BadSeq:
    def __len__(self):
        raise ValueError
class InvalidLen:
    def __len__(self):
        return None
class FloatLen:
    def __len__(self):
        return 4.5
class NegativeLen:
    def __len__(self):
        return -10
class HugeLen:
    def __len__(self):
        return sys.maxsize + 1
class HugeNegativeLen:
    def __len__(self):
        return -sys.maxsize - 10
class NoLenMethod:
    pass
for value in [BadSeq(), InvalidLen(), FloatLen(), NegativeLen(), HugeLen(), HugeNegativeLen(), NoLenMethod()]:
    try:
        len(value)
    except Exception as error:
        print(error.__class__.__name__)
for expr in [lambda: len(), lambda: len([], [])]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_repr / ::test_repr_blocked",
            name: "repr-builtin",
            source: r#"print(repr(''))
print(repr(0))
print(repr(()))
print(repr([]))
print(repr({}))
a = []
a.append(a)
print(repr(a))
d = {}
d[0] = d
print(repr(d))
class Custom:
    def __repr__(self):
        return 'custom repr'
class Blocked:
    __repr__ = None
class Bad:
    def __repr__(self):
        return 42
print(repr(Custom()))
for value in [Blocked(), Bad()]:
    try:
        repr(value)
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_bin / ::test_oct / ::test_hex",
            name: "integer-base-builtins",
            source: r#"print(bin(0), bin(1), bin(-1))
print(bin(2 ** 65))
print(bin(2 ** 65 - 1))
print(bin(-(2 ** 65)))
print(bin(-(2 ** 65 - 1)))
print(oct(100), oct(-100))
print(hex(16), hex(-16))
print(bin(True), oct(False), hex(True))
class Indexable:
    def __index__(self):
        return 255
print(bin(Indexable()), oct(Indexable()), hex(Indexable()))
for expr in [lambda: bin(), lambda: bin(1, 2), lambda: bin(1.0), lambda: oct(()), lambda: hex({})]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_divmod",
            name: "divmod-builtin",
            source: r#"print(divmod(12, 7), divmod(-12, 7), divmod(12, -7), divmod(-12, -7))
print(divmod(-(2 ** 63), -1))
print(divmod(True, 2), divmod(False, 2))
for pair in [(3.25, 1.0), (-3.25, 1.0), (3.25, -1.0), (-3.25, -1.0), (12, 7.0), (12.0, 7)]:
    print(divmod(pair[0], pair[1]))
for expr in [lambda: divmod(), lambda: divmod(1), lambda: divmod(1, 2, 3), lambda: divmod(1, 0), lambda: divmod(1.0, 0), lambda: divmod(1, 0.0), lambda: divmod('x', 2), lambda: divmod(1, y=2)]:
    try:
        expr()
    except (TypeError, ZeroDivisionError) as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_round / ::test_round_large / ::test_bug_27936",
            name: "round-builtin",
            source: r#"print(round(0.0), round(1.0), round(10.0), round(1000000000.0), round(1e20))
print(round(-1.0), round(-10.0), round(-1000000000.0), round(-1e20))
print(round(0.1), round(1.1), round(10.1), round(1000000000.1))
print(round(-1.1), round(-10.1), round(-1000000000.1))
print(round(0.9), round(9.9), round(999999999.9))
print(round(-0.9), round(-9.9), round(-999999999.9))
print(round(5.5), round(6.5), round(-5.5), round(-6.5))
print(round(-8.0, -1), type(round(-8.0, -1)).__name__)
print(type(round(-8.0, 0)).__name__, type(round(-8.0, 1)).__name__)
print(round(15.0, -1), round(25.0, -1), round(35.0, -1))
print(round(0), round(8), round(-8), type(round(0)).__name__)
print(round(-8, -1), type(round(-8, -1)).__name__)
print(round(-8, 0), type(round(-8, 0)).__name__)
print(round(-8, 1), type(round(-8, 1)).__name__)
print(round(1234, None), round(1234.56, None), type(round(1234.56, None)).__name__)
print(round(1234.56, 1), round(1234.56, -1))
print(round(number=-8.0, ndigits=-1))
for value in [5e15 - 1, 5e15, 5e15 + 1, 5e15 + 2, 5e15 + 3]:
    rounded = round(value)
    print(rounded == value, type(rounded).__name__)
class TestRound:
    def __round__(self):
        return 23
class TestRoundWithDigits:
    def __round__(self, ndigits):
        return ndigits
print(round(TestRound()))
print(round(TestRoundWithDigits(), 4))
class TestNoRound:
    pass
t = TestNoRound()
t.__round__ = lambda *args: args
for expr in [lambda: round(), lambda: round(1, 2, 3), lambda: round(TestNoRound()), lambda: round(t), lambda: round(t, 0), lambda: round(1.2, 'x')]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_pow",
            name: "pow-builtin",
            source: r#"print(pow(0, 0), pow(0, 1), pow(1, 0), pow(1, 1))
print(pow(2, 10), pow(2, 20), pow(2, 30))
print(pow(-2, 0), pow(-2, 1), pow(-2, 2), pow(-2, 3))
if True:
    print(pow(0.0, 0), pow(0.0, 1), pow(1.0, 0), pow(1.0, 1))
    print(pow(2.0, 10), pow(2.0, 20), pow(-2.0, 3))
    print(pow(2, -1), pow(-2, -3))
    sqrt_minus_one = pow(-1, 0.5)
    cube_root_minus_one = pow(-1, 1/3)
    print(type(sqrt_minus_one).__name__, abs(sqrt_minus_one - 1j) < 1e-12)
    print(type(cube_root_minus_one).__name__, abs(cube_root_minus_one - (0.5 + 0.8660254037844386j)) < 1e-12)
    print(abs(((-1) ** 0.5) - 1j) < 1e-12, abs(((-1) ** (1/3)) - (0.5 + 0.8660254037844386j)) < 1e-12)
    print(pow(2, 10, 1000), pow(-1, -2, 3), pow(5, 2, 14), pow(2, 3, -5), pow(2, -1, 5))
    print(pow(0, exp=0), pow(base=2, exp=4), pow(base=5, exp=2, mod=14), pow(2, 3, None))
    from functools import partial
    twopow = partial(pow, base=2)
    fifth_power = partial(pow, exp=5)
    mod10 = partial(pow, mod=10)
    print(twopow(exp=5))
    print(fifth_power(2))
    print(mod10(2, 6), mod10(exp=6, base=2))
    print(partial(pow, base=2)(base=3, exp=4))
    print(type(twopow).__name__, callable(twopow))
for expr in [lambda: pow(), lambda: pow(1), lambda: pow(0, -1), lambda: pow(1, 2, 0), lambda: pow(2.0, 3, 5), lambda: pow(2, 3.0, 5), lambda: pow(2, 3, 5.0), lambda: pow(2, -1, 4)]:
    try:
        expr()
    except (TypeError, ValueError, ZeroDivisionError) as error:
        print(error.__class__.__name__)"#,
        },
        DiffCase {
            origin: "Lib/test/test_str.py::test_format and ::test_format_map",
            name: "string-format-and-format-map",
            source: r#"print(''.format())
print('a{{b'.format())
print('My name is {0}'.format('Fred'))
print('{} {}'.format('hello', 'world'))
print('{1} {0}'.format('left', 'right'))
print('{name}={value}'.format(name='answer', value=42))
print('{0:.3s}'.format('abcdef'))
print('{0:04d}'.format(7))
print('{!r} {!s}'.format('x', 'y'))
print('{a} {world}'.format_map({'a': 'hello', 'world': 'earth'}))
print('My name is {0[name]}'.format({'name': 'Fred'}))
class C:
    def __init__(self):
        self._x = 20
print('{foo._x}'.format_map({'foo': C()}))"#,
        },
        DiffCase {
            origin: "Lib/test/test_format.py numeric grouping option rendering",
            name: "format-grouping-rendering",
            source: r#"print(format(1234567, ','))
print(format(1234567, '_'))
print(format(1234567, ',d'), format(-1234567, ',d'))
print(format(1234567, '12,'))
print(f'{1234567:,}', f'{1234567:_}')
print(format(12345.5, ',.1f'), format(-12345.5, ',.1f'))
print(format(0x12345678, '_x'), format(0x12345678, '#_x'), format(0x12345678, '_X'))
print(format(True, ','), format(False, '_d'))
try:
    print(format(255, ',x'))
except ValueError as error:
    print(error)"#,
        },
        DiffCase {
            origin: "Lib/test/test_format.py integer codes and zero alignment",
            name: "format-integer-codes-and-zero-alignment",
            source: r#"print(format(10, 'b'), format(10, '#b'), format(-10, '#b'))
print(format(10, 'o'), format(10, '#o'), format(-10, '#o'))
print(format(0b101010101, '_b'), format(0o12345670, '#_o'))
print(format(65, 'c'), format(65, '5c') == '    A', format(65, '<5c') == 'A    ', format(True, 'c') == '\x01')
print(format(-42, '05d'), format(42, '+05d'), format(42, ' 05d'))
print(format(-42, '=5d'), format(-42, '0=5d'), format(-42, '0>5d'))
print(format(0x2a, '#06x'), format(-0x2a, '#06x'))
print(format(0o52, '#06o'), format(-0o52, '#06o'))
print(format(1234567, '012,'), format(-1234567, '012,'))
print(format(12345.5, '012,.1f'), format(-12345.5, '012,.1f'))
for value, spec in [(10, ',b'), (10, ',o'), (65, ',c'), (65, '#c'), (1, '.2d')]:
    try:
        print(format(value, spec))
    except ValueError as error:
        print(error)"#,
        },
        DiffCase {
            origin: "Lib/test/test_types.py::TypesTests::test_int__format__",
            name: "types-int-dunder-format-matrix",
            source: r#"def check(i, format_spec, expected):
    assert type(i) is int
    assert type(format_spec) is str
    actual = i.__format__(format_spec)
    assert actual == expected
rows = [
    (123456789, 'd', '123456789'), (123456789, 'd', '123456789'), (1, 'c', chr(1)),
    (1, '-', '1'), (-1, '-', '-1'), (1, '-3', '  1'), (-1, '-3', ' -1'),
    (1, '+3', ' +1'), (-1, '+3', ' -1'), (1, ' 3', '  1'), (-1, ' 3', ' -1'),
    (1, ' ', ' 1'), (-1, ' ', '-1'),
    (3, 'x', '3'), (3, 'X', '3'), (1234, 'x', '4d2'), (-1234, 'x', '-4d2'),
    (1234, '8x', '     4d2'), (-1234, '8x', '    -4d2'), (1234, 'x', '4d2'), (-1234, 'x', '-4d2'),
    (-3, 'x', '-3'), (-3, 'X', '-3'), (int('be', 16), 'x', 'be'), (int('be', 16), 'X', 'BE'),
    (-int('be', 16), 'x', '-be'), (-int('be', 16), 'X', '-BE'),
    (3, 'o', '3'), (-3, 'o', '-3'), (65, 'o', '101'), (-65, 'o', '-101'),
    (1234, 'o', '2322'), (-1234, 'o', '-2322'), (1234, '-o', '2322'), (-1234, '-o', '-2322'),
    (1234, ' o', ' 2322'), (-1234, ' o', '-2322'), (1234, '+o', '+2322'), (-1234, '+o', '-2322'),
    (3, 'b', '11'), (-3, 'b', '-11'), (1234, 'b', '10011010010'), (-1234, 'b', '-10011010010'),
    (1234, '-b', '10011010010'), (-1234, '-b', '-10011010010'), (1234, ' b', ' 10011010010'), (-1234, ' b', '-10011010010'),
    (1234, '+b', '+10011010010'), (-1234, '+b', '-10011010010'),
    (0, '#b', '0b0'), (0, '-#b', '0b0'), (1, '-#b', '0b1'), (-1, '-#b', '-0b1'),
    (-1, '-#5b', ' -0b1'), (1, '+#5b', ' +0b1'), (100, '+#b', '+0b1100100'),
    (100, '#012b', '0b0001100100'), (-100, '#012b', '-0b001100100'),
    (0, '#o', '0o0'), (0, '-#o', '0o0'), (1, '-#o', '0o1'), (-1, '-#o', '-0o1'),
    (-1, '-#5o', ' -0o1'), (1, '+#5o', ' +0o1'), (100, '+#o', '+0o144'),
    (100, '#012o', '0o0000000144'), (-100, '#012o', '-0o000000144'),
    (0, '#x', '0x0'), (0, '-#x', '0x0'), (1, '-#x', '0x1'), (-1, '-#x', '-0x1'),
    (-1, '-#5x', ' -0x1'), (1, '+#5x', ' +0x1'), (100, '+#x', '+0x64'),
    (100, '#012x', '0x0000000064'), (-100, '#012x', '-0x000000064'),
    (123456, '#012x', '0x000001e240'), (-123456, '#012x', '-0x00001e240'),
    (0, '#X', '0X0'), (0, '-#X', '0X0'), (1, '-#X', '0X1'), (-1, '-#X', '-0X1'),
    (-1, '-#5X', ' -0X1'), (1, '+#5X', ' +0X1'), (100, '+#X', '+0X64'),
    (100, '#012X', '0X0000000064'), (-100, '#012X', '-0X000000064'),
    (123456, '#012X', '0X000001E240'), (-123456, '#012X', '-0X00001E240'),
    (123, ',', '123'), (-123, ',', '-123'), (1234, ',', '1,234'), (-1234, ',', '-1,234'),
    (123456, ',', '123,456'), (-123456, ',', '-123,456'), (1234567, ',', '1,234,567'), (-1234567, ',', '-1,234,567'),
    (1234, '010,', '00,001,234'),
    (10**100, 'd', '1' + '0' * 100), (10**100 + 100, 'd', '1' + '0' * 97 + '100'),
    (123456, '0<20', '12345600000000000000'), (123456, '1<20', '12345611111111111111'), (123456, '*<20', '123456**************'),
    (123456, '0>20', '00000000000000123456'), (123456, '1>20', '11111111111111123456'), (123456, '*>20', '**************123456'),
    (123456, '0=20', '00000000000000123456'), (123456, '1=20', '11111111111111123456'), (123456, '*=20', '**************123456'),
]
for row in rows:
    check(*row)
print('rows', len(rows))
value_errors = 0
type_errors = 0
for spec in ['1.3', '+c', None, 0, ',n', ',c', '#c']:
    try:
        (3).__format__(spec)
    except ValueError:
        value_errors += 1
    except TypeError:
        type_errors += 1
print('errors', value_errors, type_errors)
invalid_specs = 0
for format_spec in ([chr(x) for x in range(ord('a'), ord('z') + 1)] + [chr(x) for x in range(ord('A'), ord('Z') + 1)]):
    if format_spec not in 'bcdoxXeEfFgGn%':
        for value in [0, 1, -1]:
            try:
                value.__format__(format_spec)
            except ValueError:
                invalid_specs += 1
print('invalid-specs', invalid_specs)
float_specs = 0
for format_spec in 'eEfFgG%':
    for value in [0, 1, -1, 100, -100, 1234567890, -1234567890]:
        assert value.__format__(format_spec) == float(value).__format__(format_spec)
        float_specs += 1
print('float-specs', float_specs)"#,
        },
        DiffCase {
            origin: "Lib/test/test_types.py::TypesTests::test_float__format__",
            name: "types-float-dunder-format-matrix",
            source: r#"def check(f, format_spec, expected):
    assert f.__format__(format_spec) == expected
    assert format(f, format_spec) == expected
rows = [
    (0.0, 'f', '0.000000'),
    (0.0, '', '0.0'), (0.01, '', '0.01'), (0.01, 'g', '0.01'),
    (1.23, '1', '1.23'), (-1.23, '1', '-1.23'), (1.23, '1g', '1.23'), (-1.23, '1g', '-1.23'),
    (1.0, ' g', ' 1'), (-1.0, ' g', '-1'), (1.0, '+g', '+1'), (-1.0, '+g', '-1'),
    (1.1234e200, 'g', '1.1234e+200'), (1.1234e200, 'G', '1.1234E+200'),
    (1.0, 'f', '1.000000'), (-1.0, 'f', '-1.000000'),
    (1.0, ' f', ' 1.000000'), (-1.0, ' f', '-1.000000'), (1.0, '+f', '+1.000000'), (-1.0, '+f', '-1.000000'),
    (1.0, 'e', '1.000000e+00'), (-1.0, 'e', '-1.000000e+00'), (1.0, 'E', '1.000000E+00'), (-1.0, 'E', '-1.000000E+00'),
    (1.1234e20, 'e', '1.123400e+20'), (1.1234e20, 'E', '1.123400E+20'),
    (1.25e200, '+g', '+1.25e+200'), (1.25e200, '+', '+1.25e+200'),
    (1.1e200, '+g', '+1.1e+200'), (1.1e200, '+', '+1.1e+200'),
    (1234.0, '010f', '1234.000000'), (1234.0, '011f', '1234.000000'), (1234.0, '012f', '01234.000000'),
    (-1234.0, '011f', '-1234.000000'), (-1234.0, '012f', '-1234.000000'), (-1234.0, '013f', '-01234.000000'),
    (-1234.12341234, '013f', '-01234.123412'), (-123456.12341234, '011.2f', '-0123456.12'),
    (1.2, '010,.2', '0,000,001.2'),
    (1234.0, '011,f', '1,234.000000'), (1234.0, '012,f', '1,234.000000'), (1234.0, '013,f', '01,234.000000'),
    (-1234.0, '012,f', '-1,234.000000'), (-1234.0, '013,f', '-1,234.000000'), (-1234.0, '014,f', '-01,234.000000'),
    (-12345.0, '015,f', '-012,345.000000'), (-123456.0, '016,f', '-0,123,456.000000'), (-123456.0, '017,f', '-0,123,456.000000'),
    (-123456.12341234, '017,f', '-0,123,456.123412'), (-123456.12341234, '013,.2f', '-0,123,456.12'),
    (-1.0, '%', '-100.000000%'),
    (1.0, '.0e', '1e+00'), (1.0, '#.0e', '1.e+00'), (1.0, '.0f', '1'), (1.0, '#.0f', '1.'),
    (1.1, 'g', '1.1'), (1.1, '#g', '1.10000'), (1.0, '.0%', '100%'), (1.0, '#.0%', '100.%'),
    (1.0, '0e', '1.000000e+00'), (1.0, '#0e', '1.000000e+00'), (1.0, '0f', '1.000000'), (1.0, '#0f', '1.000000'),
    (1.0, '.1e', '1.0e+00'), (1.0, '#.1e', '1.0e+00'), (1.0, '.1f', '1.0'), (1.0, '#.1f', '1.0'),
    (1.0, '.1%', '100.0%'), (1.0, '#.1%', '100.0%'),
    (12345.6, '0<20', '12345.60000000000000'), (12345.6, '1<20', '12345.61111111111111'), (12345.6, '*<20', '12345.6*************'),
    (12345.6, '0>20', '000000000000012345.6'), (12345.6, '1>20', '111111111111112345.6'), (12345.6, '*>20', '*************12345.6'),
    (12345.6, '0=20', '000000000000012345.6'), (12345.6, '1=20', '111111111111112345.6'), (12345.6, '*=20', '*************12345.6'),
]
for row in rows:
    check(*row)
print('rows', len(rows))
huge = 0
for value, expected_len in [(1.1234e90, 98), (1.1234e200, 208)]:
    for fmt in ('f', 'F'):
        result = value.__format__(fmt)
        assert len(result) == expected_len
        assert result[-7] == '.'
        assert result[:12] in ('112340000000', '112339999999')
        huge += 1
print('huge', huge)
type_errors = 0
for spec in [None, 0]:
    try:
        (3.0).__format__(spec)
    except TypeError:
        type_errors += 1
value_errors = 0
for format_spec in 'sbcdoxX':
    for value in [0.0, 1.0, -1.0, 1e100, -1e100, 1e-100, -1e-100]:
        try:
            format(value, format_spec)
        except ValueError:
            value_errors += 1
print('errors', type_errors, value_errors)"#,
        },
        DiffCase {
            origin: "Lib/test/test_types.py::TypesTests::test_format_spec_errors",
            name: "types-format-spec-errors",
            source: r#"large_errors = 0
for label, spec in [('width', '1' * 10000 + 'd'), ('precision', '.' + '1' * 10000 + 'd'), ('both', '1' * 1000 + '.' + '1' * 10000 + 'd')]:
    try:
        format(0, spec)
    except ValueError as error:
        large_errors += 1
        print(label, error.__class__.__name__)
comma_errors = 0
for code in 'xXobns':
    try:
        format(0, ',' + code)
    except ValueError as error:
        comma_errors += 1
        print('comma', code, error.__class__.__name__, str(error))
print('summary', large_errors, comma_errors)"#,
        },
        DiffCase {
            origin: "Lib/test/test_types.py::TypesTests::test_method_descriptor_types",
            name: "types-method-descriptor-types",
            source: r#"import types
print(isinstance(str.join, types.MethodDescriptorType))
print(isinstance(list.append, types.MethodDescriptorType))
print(isinstance(''.join, types.BuiltinMethodType))
print(isinstance([].append, types.BuiltinMethodType))
print(isinstance(int.__dict__['from_bytes'], types.ClassMethodDescriptorType))
print(isinstance(int.from_bytes, types.BuiltinMethodType))
print(isinstance(int.__new__, types.BuiltinMethodType))
print(type(str.join).__name__, type(list.append).__name__, type(int.__dict__['from_bytes']).__name__)
print(int.from_bytes(b'\x01\x00', 'little'), int.from_bytes(b'\xff', 'big', signed=True), bool.from_bytes(b'\x02', 'big'))
print(int.__dict__['from_bytes'](int, b'\x01', 'big'))
print(int.__new__(int, '10'))
items = []
list.append(items, 3)
print(items)"#,
        },
        DiffCase {
            origin: "Lib/test/test_types.py::TypesTests::test_frame_locals_proxy_type",
            name: "types-frame-locals-proxy-currentframe",
            source: r#"import inspect, types
def probe():
    marker = 42
    frame = inspect.currentframe()
    proxy_type = getattr(types, 'FrameLocalsProxyType', dict)
    print(frame is not None, isinstance(frame.f_locals, proxy_type))
    print(type(frame.f_locals).__name__ in ('dict', 'FrameLocalsProxy'))
    print('marker' in frame.f_locals, frame.f_locals['marker'])
    print(sorted(k for k in frame.f_locals if k in ('frame', 'marker')))
probe()"#,
        },
        DiffCase {
            origin: "Lib/test/test_bool.py::BoolTest::test_bool custom truth protocol",
            name: "custom-bool-and-len-truth-protocol",
            source: r#"class FalseByBool:
    def __bool__(self):
        print("bool")
        return False
class TrueByLen:
    def __len__(self):
        print("len")
        return 2
class FalseByLen:
    def __len__(self):
        print("empty")
        return 0
class Default:
    pass
false_value = FalseByBool()
true_len = TrueByLen()
false_len = FalseByLen()
print(bool(false_value))
print(not false_value)
if false_value:
    print("bad")
else:
    print("false branch")
if true_len:
    print("len true")
print(bool(false_len), bool(Default()))
print(all([true_len, false_len]), any([false_len, true_len]))
print(len(true_len))
class BadBool:
    def __bool__(self):
        return 1
try:
    if BadBool():
        print("bad")
except TypeError as error:
    print(error)
class BadLen:
    def __len__(self):
        return -1
try:
    assert BadLen()
except ValueError as error:
    print(error)"#,
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_abs / ::test_min / ::test_sum",
            name: "abs-min-sum-builtins",
            source: "print(abs(0), abs(1234), abs(-1234), abs(True), abs(False))\nprint(abs(0.0), abs(3.14), abs(-3.14), abs(3 + 4j))\nclass AbsClass:\n    def __abs__(self):\n        return -5\nprint(abs(AbsClass()))\nprint(min('123123'))\nprint(min(1, 2, 3), min((1, 2, 3, 1, 2, 3)), min([1, 2, 3, 1, 2, 3]))\nprint(min(1, 2, 3.0), min(1.0, 2, 3))\nprint(sum([]), sum(list(range(2, 8))), sum(iter(list(range(2, 8)))))\nprint(sum(range(10), 1000), sum(i % 2 != 0 for i in range(10)))\nprint(sum([[1], [2], [3]], []))\nprint(sum([0.5, 1]), sum([1, 0.5]))\nprint(min([-3, 2, -1], key=abs), max([-3, 2, -1], key=abs))\nprint(min([1, 2, 3], key=lambda x: -x), max([1, 2, 3], key=lambda x: -x))\nprint(min([], default='empty'), max([], default='empty'))\nprint(min([], default='empty', key=lambda x: 1))\nprint(min([2, 1], key=None), max([2, 1], key=None))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestSorted",
            name: "sorted-builtin",
            source: "copy = [3, 1, 2]\nprint(sorted(copy))\nprint(copy)\nprint(sorted(copy, key=lambda x: -x))\nprint(sorted(copy, reverse=True))\nprint(sorted([], key=None))\nletters = sorted('abracadabra')\nprint(len(letters), letters[0], letters[1], letters[-1])\nfrom_tuple = sorted(tuple('cab'))\nfrom_set = sorted(set('cab'))\nfrom_dict = sorted(dict.fromkeys('cab'))\nprint(from_tuple[0], from_tuple[1], from_tuple[2], from_set[0], from_set[1], from_set[2], from_dict[0], from_dict[1], from_dict[2])\nprint(sorted(x for x in [3, 1, 2]))\nprint(sorted([(1, 10), (1, 20), (0, 30)], key=lambda item: item[0], reverse=True))",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_reverse / ::test_sort and Lib/test/test_sort.py::TestDecorateSortUndecorate::test_reverse",
            name: "list-reverse-and-sort-methods",
            source: "u = [-2, -1, 0, 1, 2]\noriginal = u.copy()\nprint(u.reverse(), u)\nprint(u.reverse(), u == original)\nu = [1, 0]\nprint(u.sort(), u)\nu = [2, 1, 0, -1, -2]\nu.sort()\nprint(u)\nu.sort(key=lambda x: -x)\nprint(u)\nu = [3, 1, 2]\nu.sort(reverse=True)\nprint(u)\nu = [(1, 10), (1, 20), (0, 30)]\nu.sort(key=lambda item: item[0], reverse=True)\nprint(u)\nnums = [2, 1]\nalias = nums\nprint(nums.sort(key=None), alias is nums, alias)",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_insert / ::test_remove / ::test_index and Lib/test/seq_tests.py::CommonTest::test_count / ::test_index",
            name: "list-insert-remove-count-index-methods",
            source: "a = [0, 1, 2]\nprint(a.insert(0, -2), a)\na.insert(1, -1)\na.insert(2, 0)\nprint(a)\nb = a[:]\nb.insert(-2, 99)\nb.insert(-200, -99)\nb.insert(200, 100)\nprint(b[0], b[-1], b.index(99), len(b))\nc = [0, 1, 2] * 3\nprint(c.count(0), c.count(1), c.count(3))\nd = [0, 0, 1]\nprint(d.remove(1), d)\nprint(d.remove(0), d)\nprint(d.remove(0), d)\nu = [-2, -1, 0, 0, 1, 2]\nprint(u.index(0), u.index(0, 2), u.index(-2, -10), u.index(0, 3), u.index(0, 3, 4))\nu.remove(0)\nprint(u)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript and list special method surface",
            name: "list-dunder-sequence-methods",
            source: "a = [10, 11]\nprint(a.__getitem__(0), a.__getitem__(1), a.__getitem__(-2), a.__getitem__(-1))\nprint(a.__getitem__(slice(0, 1)), a.__getitem__(slice(1, 2)), a.__getitem__(slice(0, 2)), a.__getitem__(slice(0, 3)), a.__getitem__(slice(3, 5)))\nb = [1, 2, 3]\nprint(b.__len__(), b.__contains__(2), b.__contains__(4))\nprint(b.__setitem__(1, 9), b)\nprint(b.__setitem__(slice(1, 3), [8, 9]), b)\nprint(b.__delitem__(1), b)\nc = [1, 2, 3, 4]\nprint(c.__delitem__(slice(1, 3)), c)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript plus tuple/str/bytes/range special method surface",
            name: "immutable-sequence-dunder-methods",
            source: "t = (10, 11)\nprint(t.__getitem__(0), t.__getitem__(1), t.__getitem__(-2), t.__getitem__(-1), t.__getitem__(slice(0, 2)))\nprint(t.__len__(), t.__contains__(10), t.__contains__(99))\ns = 'abc'\nprint(s.__getitem__(0), s.__getitem__(-1), s.__getitem__(slice(0, 2)), s.__contains__('b'), s.__contains__('z'), s.__len__())\nb = b'abc'\nprint(b.__getitem__(0), b.__getitem__(-1), b.__getitem__(slice(0, 2)), b.__contains__(98), b.__contains__(120), b.__len__())\nr = range(1, 6, 2)\nprint(r.__getitem__(0), r.__getitem__(-1), list(r.__getitem__(slice(0, 2))), r.__contains__(3), r.__contains__(4), r.__len__())",
        },
        DiffCase {
            origin: "Lib/test/test_slice.py::SliceTest::test_indices",
            name: "slice-indices-method",
            source: "print(slice(None).indices(10))\nprint(slice(None, None, 2).indices(10))\nprint(slice(1, None, 2).indices(10))\nprint(slice(None, None, -1).indices(10))\nprint(slice(None, None, -2).indices(10))\nprint(slice(3, None, -2).indices(10))\nprint(slice(None, -9).indices(10), slice(None, -10).indices(10), slice(None, -11).indices(10))\nprint(slice(None, -10, -1).indices(10), slice(None, -11, -1).indices(10), slice(None, -12, -1).indices(10))\nprint(slice(None, 9).indices(10), slice(None, 10).indices(10), slice(None, 11).indices(10))\nprint(slice(None, 8, -1).indices(10), slice(None, 9, -1).indices(10), slice(None, 10, -1).indices(10))\nprint(slice(-100, 100).indices(10) == slice(None).indices(10))\nprint(slice(100, -100, -1).indices(10) == slice(None, None, -1).indices(10))\nprint(slice(-100, 100, 2).indices(10))\nclass I:\n    def __init__(self, value):\n        self.value = value\n    def __index__(self):\n        return self.value\nprint(slice(I(0), I(10), I(1)).indices(I(5)))\nfor expr in [lambda: slice(None).indices(-1), lambda: slice(0, 10, 0).indices(5), lambda: slice(0.0, 10, 1).indices(5), lambda: slice(0, 10, 1).indices(5.0)]:\n    try:\n        expr()\n    except (TypeError, ValueError) as error:\n        print(error.__class__.__name__, error)",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py mapping special method surface",
            name: "dict-dunder-mapping-methods",
            source: "d = {1: 10, 2: 20}\nprint(d.__getitem__(1), d.__getitem__(2), d.__contains__(1), d.__contains__(3), d.__len__())\nprint(d.__setitem__(3, 30), d)\nprint(d.__setitem__(1, 11), d.__getitem__(1), d.__len__())\nprint(d.__delitem__(2), d, d.__len__())",
        },
        DiffCase {
            origin: "Lib/test/test_set.py set special method surface",
            name: "set-dunder-methods",
            source: "s = {1, 2, 3}\nprint(s.__len__(), s.__contains__(2), s.__contains__(9))\nprint(sorted(s.__or__({3, 4})))\nprint(sorted(s.__and__({2, 4})))\nprint(sorted(s.__sub__({1, 4})))\nprint(sorted(s.__xor__({3, 4})))\nprint(s.__le__({1, 2, 3, 4}), s.__lt__({1, 2, 3, 4}), s.__ge__({1, 2}), s.__gt__({1, 2}), s.__eq__({3, 2, 1}), s.__ne__({1, 2}))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py NotImplemented singleton and Lib/test/test_set.py unsupported set dunders",
            name: "notimplemented-and-unsupported-set-dunders",
            source: "print(NotImplemented)\nprint(NotImplemented is NotImplemented, NotImplemented == NotImplemented)\ns = {1}\nprint(s.__or__([2]), s.__and__([1]), s.__sub__([1]), s.__xor__([1]))\nprint(s.__le__([1]), s.__lt__([1]), s.__ge__([1]), s.__gt__([1]))\nprint(s.__eq__([1]), s.__ne__([1]))",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestSet.test_rich_compare",
            name: "set-rich-compare-reflection",
            source: "class TestRichSetCompare:\n    def __gt__(self, some_set):\n        self.gt_called = True\n        return False\n    def __lt__(self, some_set):\n        self.lt_called = True\n        return False\n    def __ge__(self, some_set):\n        self.ge_called = True\n        return False\n    def __le__(self, some_set):\n        self.le_called = True\n        return False\nmyset = {1, 2, 3}\nmyobj = TestRichSetCompare()\nprint(myset < myobj, myobj.gt_called)\nmyobj = TestRichSetCompare()\nprint(myset > myobj, myobj.lt_called)\nmyobj = TestRichSetCompare()\nprint(myset <= myobj, myobj.ge_called)\nmyobj = TestRichSetCompare()\nprint(myset >= myobj, myobj.le_called)",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestSet.test_unhashable_element",
            name: "set-hash-exception-propagation",
            source: "myset = {'a'}\nclass HashError:\n    def __hash__(self):\n        raise KeyError('error')\nelem = HashError()\nfor op in [lambda: elem in myset, lambda: myset.add(elem), lambda: myset.discard(elem)]:\n    try:\n        op()\n    except KeyError as error:\n        print(error.__class__.__name__, error.args[0])",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestJointOps.test_badcmp",
            name: "set-bad-comparison-errors",
            source: "class BadCmp:\n    def __hash__(self):\n        return 1\n    def __eq__(self, other):\n        raise RuntimeError\ndef result(fn):\n    try:\n        fn()\n    except RuntimeError as error:\n        print(error.__class__.__name__)\ns = set([BadCmp()])\nresult(lambda: set([BadCmp(), BadCmp()]))\nresult(lambda: s.__contains__(BadCmp()))\nresult(lambda: s.add(BadCmp()))\nresult(lambda: s.discard(BadCmp()))\nresult(lambda: s.remove(BadCmp()))",
        },
        DiffCase {
            origin: "Lib/test/test_set.py bad comparison set algebra behavior",
            name: "set-bad-comparison-algebra-errors",
            source: "class BadCmp:\n    def __hash__(self):\n        return 1\n    def __eq__(self, other):\n        raise RuntimeError\ndef result(fn):\n    try:\n        fn()\n    except RuntimeError as error:\n        print(error.__class__.__name__)\nfor typ in (set, frozenset):\n    s = typ([BadCmp()])\n    t = typ([BadCmp()])\n    for op in [\n        lambda: s == t,\n        lambda: s != t,\n        lambda: s <= t,\n        lambda: s >= t,\n        lambda: s.issubset(t),\n        lambda: s.issuperset(t),\n        lambda: s.isdisjoint(t),\n        lambda: s.intersection(t),\n        lambda: s.difference(t),\n        lambda: s.symmetric_difference(t),\n        lambda: s.union(t),\n        lambda: s & t,\n        lambda: s - t,\n        lambda: s ^ t,\n        lambda: s | t,\n    ]:\n        result(op)",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestBasicOps.test_changingSizeWhileIterating and TestWeirdBugs.test_iter_and_mutate",
            name: "set-iterator-mutation",
            source: "def show(fn):\n    try:\n        fn()\n    except RuntimeError as error:\n        print(error.__class__.__name__, error.args[0])\ndef changing_size():\n    s = set([1, 2, 3])\n    for i in s:\n        s.update([4])\nshow(changing_size)\ns = set(range(10))\ns.clear()\ns.update(range(10))\nit = iter(s)\ns.clear()\ns.update(range(10))\nlist(it)\nprint('same-size-ok')",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestWeirdBugs.test_merge_and_mutate and ::test_hash_collision_concurrent_add",
            name: "set-reentrant-mutation",
            source: "class ClearsOther:\n    def __hash__(self):\n        return hash(0)\n    def __eq__(self, other_value):\n        other.clear()\n        return False\nother = set()\nother = {ClearsOther() for i in range(10)}\ns = {0}\ns.update(other)\nprint('merge', len(other))\nclass X:\n    def __hash__(self):\n        return 0\nclass Y:\n    flag = False\n    def __hash__(self):\n        return 0\n    def __eq__(self, other_value):\n        if not self.flag:\n            self.flag = True\n            target.add(X())\n        return self is other_value\na = X()\ntarget = set()\ntarget.add(a)\ntarget.add(X())\ntarget.remove(a)\ntarget.add(Y())\nrepr(target)\nlist(target)\nset() | target\nprint('concurrent', len(target))",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestOperationsMutating deterministic stable subset",
            name: "set-operations-mutating",
            source: "def make_sets():\n    global enabled, set1, set2\n    class Bad:\n        def __eq__(self, other):\n            if enabled:\n                set1.clear()\n                set2.clear()\n            return False\n        def __hash__(self):\n            return 0\n    enabled = False\n    set1 = set(Bad() for i in range(3))\n    set2 = set(Bad() for i in range(3))\n    enabled = True\n    return set1, set2\ndef show(label, fn):\n    a, b = make_sets()\n    try:\n        fn(a, b)\n    except RuntimeError as error:\n        print(label, 'RuntimeError', 'changed size' in str(error))\n    else:\n        print(label, 'ok')\nfor label, fn in [\n    ('eq', lambda a, b: a == b),\n    ('ne', lambda a, b: a != b),\n    ('le', lambda a, b: a <= b),\n    ('ge', lambda a, b: a >= b),\n    ('and', lambda a, b: a & b),\n    ('or', lambda a, b: a | b),\n    ('issubset', lambda a, b: set.issubset(a, b)),\n    ('issuperset', lambda a, b: set.issuperset(a, b)),\n    ('intersection', lambda a, b: set.intersection(a, b)),\n    ('union', lambda a, b: set.union(a, b)),\n    ('isdisjoint', lambda a, b: set.isdisjoint(a, b)),\n    ('diff_update', lambda a, b: set.difference_update(a, b)),\n    ('inter_update', lambda a, b: set.intersection_update(a, b)),\n    ('sym_update', lambda a, b: set.symmetric_difference_update(a, b)),\n    ('update', lambda a, b: set.update(a, b)),\n]:\n    show(label, fn)",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestOnlySetsInBinaryOps representative subset",
            name: "set-only-sets-in-binary-ops",
            source: "def show_type_error(label, fn):\n    try:\n        fn()\n    except TypeError:\n        print(label, 'TypeError')\n    else:\n        print(label, 'ok')\ndef check(label, other):\n    s = set([1, 2, 3])\n    print(label, other == s, s == other, other != s, s != other)\n    for op, fn in [('lt', lambda: s < other), ('le', lambda: s <= other), ('gt', lambda: s > other), ('ge', lambda: s >= other), ('rlt', lambda: other < s), ('rle', lambda: other <= s), ('rgt', lambda: other > s), ('rge', lambda: other >= s), ('or', lambda: s | other), ('ror', lambda: other | s), ('and', lambda: s & other), ('rand', lambda: other & s), ('xor', lambda: s ^ other), ('rxor', lambda: other ^ s), ('sub', lambda: s - other), ('rsub', lambda: other - s)]:\n        show_type_error(label + ' ' + op, fn)\n    def inplace_or():\n        target = set([1, 2, 3])\n        target |= other\n    def inplace_and():\n        target = set([1, 2, 3])\n        target &= other\n    def inplace_xor():\n        target = set([1, 2, 3])\n        target ^= other\n    def inplace_sub():\n        target = set([1, 2, 3])\n        target -= other\n    for op, fn in [('ior', inplace_or), ('iand', inplace_and), ('ixor', inplace_xor), ('isub', inplace_sub)]:\n        show_type_error(label + ' ' + op, fn)\n    for method in ['update', 'union', 'intersection', 'difference', 'symmetric_difference', 'intersection_update', 'difference_update', 'symmetric_difference_update']:\n        s = set([1, 2, 3])\n        show_type_error(label + ' ' + method, lambda method=method: getattr(s, method)(other))\nfor label, other in [('number', 19), ('dict', {1: 2, 3: 4}), ('tuple', (2, 4, 6)), ('string', 'abc'), ('function', (lambda x: x))]:\n    check(label, other)\ndef gen():\n    for i in range(0, 10, 2):\n        yield i\nfor method in ['update', 'union', 'intersection', 'difference', 'symmetric_difference', 'intersection_update', 'difference_update', 'symmetric_difference_update']:\n    s = set([1, 2, 3])\n    show_type_error('generator ' + method, lambda method=method: getattr(s, method)(gen()))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_all / ::test_any",
            name: "all-any-builtins",
            source: "print(all([2, 4, 6]), all([2, None, 6]), all([]))\nprint(any([None, None, None]), any([None, 4, None]), any([]))\ndef false_then_fail():\n    yield 0\n    raise RuntimeError('boom')\ndef true_then_fail():\n    yield 1\n    raise RuntimeError('boom')\nprint(all(false_then_fail()), any(true_then_fail()))\ns = [50, 60]\nprint(all(x > 42 for x in s))\ns = [50, 40, 60]\nprint(all(x > 42 for x in s), any(x > 42 for x in s))",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase and Lib/test/test_builtin.py::BuiltinTest::test_zip / ::test_sorted",
            name: "enumerate-zip-sorted-builtins",
            source: "print(list(enumerate([10, 20, 30])))\nprint(list(enumerate([10, 20], 5)))\ne = enumerate(range(3), start=True)\nprint(next(e), next(e), list(e))\nprint(list(enumerate(iterable=[7, 8], start=2)))\nprint(list(zip((1, 2, 3), (4, 5, 6))))\nprint(list(zip((1, 2, 3), [4, 5, 6, 7])))\nprint(list(zip()), list(zip(*[])))\nprint(list(zip(range(5), range(10))))\nprint(list(zip((x for x in range(3)), (10, 11, 12))))\nprint(sorted([3, 1, 2]))\nprint(sorted([1, 2, 3], key=lambda x: -x))\nprint(sorted([3, 1, 2], reverse=True))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_zip_bad_iterable and sibling iterator-constructor propagation",
            name: "bad-iterable-exception-identity",
            source: "exception = TypeError('sentinel')\nclass BadIterable:\n    def __iter__(self):\n        raise exception\nfor label, fn in [('iter', lambda: iter(BadIterable())), ('enumerate', lambda: enumerate(BadIterable())), ('map', lambda: map(lambda x: x, BadIterable())), ('filter', lambda: filter(None, BadIterable())), ('zip', lambda: zip(BadIterable()))]:\n    try:\n        fn()\n    except TypeError as error:\n        print(label, error is exception, id(error) == id(exception), error.__class__.__name__, str(error))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_map / ::test_filter and Lib/test/test_iter.py map/filter iterator coverage",
            name: "map-filter-builtins",
            source: "print(list(map(lambda x: x * x, range(1, 4))))\nprint(list(map(lambda x, y: x + y, [1, 3, 2], [9, 1, 4])))\ndef plus(*values):\n    total = 0\n    for value in values:\n        total += value\n    return total\nprint(list(map(plus, [1, 3, 7], [4, 9, 2], [1, 1, 0])))\nclass Squares:\n    def __init__(self, stop):\n        self.stop = stop\n    def __getitem__(self, index):\n        if index < 0 or index >= self.stop:\n            raise IndexError\n        return index * index\nprint(list(map(int, Squares(5))))\nprint(list(filter(None, [1, [], [3], None, 9, 0, False, True])))\nprint(list(filter(lambda x: x > 0, [1, -3, 9, 0, 2])))\nprint(list(filter(lambda x: x % 2, Squares(6))))\nm = map(lambda x: x + 1, [1, 2])\nprint(next(m), list(m), iter(m) is m)\nf = filter(lambda x: x % 2, range(6))\nprint(next(f), list(f), iter(f) is f)\ndef echo(*args):\n    print(args)\necho(*map(lambda x: x + 1, [1, 2]))",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase custom iterables and Lib/test/test_builtin.py::BuiltinTest::test_zip sequence fallback",
            name: "custom-iterator-and-sequence-fallback",
            source: "class Counter:\n    def __init__(self, stop):\n        self.current = 0\n        self.stop = stop\n    def __iter__(self):\n        return self\n    def __next__(self):\n        if self.current >= self.stop:\n            raise StopIteration\n        value = self.current\n        self.current += 1\n        return value\ncounter = Counter(3)\nprint(iter(counter) is counter)\nprint(next(counter), list(counter))\nprint(list(enumerate(Counter(3), 5)))\nprint(list(zip(Counter(3), [10, 11, 12, 13])))\nclass Squares:\n    def __init__(self, stop):\n        self.stop = stop\n    def __getitem__(self, index):\n        if index < 0 or index >= self.stop:\n            raise IndexError\n        return index * index\nprint(list(Squares(4)))\nprint(list(enumerate(Squares(3))))\nprint(list(zip(Squares(3), range(10))))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_iter / ::test_next and Lib/test/test_iter.py::TestCase::test_iter_basic / ::test_iter_idempotency",
            name: "iter-next-builtins",
            source: "it = iter([1, 2])\nprint(iter(it) is it)\nprint(next(it), next(it), next(it, 42), next(it, 43))\nit = iter(range(3))\nprint(next(it), list(it))\nletters = iter('ab')\nprint(next(letters), next(letters), next(letters, 'done'))\nd = {1: 2, 3: 4}\nkeys = iter(d)\nprint(next(keys), list(keys))\nrev = reversed([1, 2])\nprint(next(rev), next(rev), next(rev, 'done'))\nempty = iter([])\ntry:\n    next(empty)\nexcept StopIteration:\n    print('stopped')",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_iter / ::test_next direct special methods",
            name: "iterator-dunder-methods",
            source: "it = [1, 2].__iter__()\nprint(it.__iter__() is it)\nprint(it.__next__(), it.__next__())\ntry:\n    it.__next__()\nexcept StopIteration:\n    print('stopped')\nprint(list((3, 4).__iter__()))\nprint(list(range(3).__iter__()))\nprint(list(b'ab'.__iter__()))\nletters = 'ab'.__iter__()\nprint(letters.__next__(), letters.__next__())\nd = {1: 10, 2: 20}\nprint(list(d.__iter__()))\nprint(list(d.keys().__iter__()), list(d.values().__iter__()), list(d.items().__iter__()))\nprint(sorted({2, 1}.__iter__()))\nz = zip([1, 2], [3, 4])\nprint(z.__iter__() is z, z.__next__(), list(z))\nm = map(lambda x: x + 1, [1, 2])\nprint(m.__next__(), list(m))\nf = filter(None, [0, 3, 0, 4])\nprint(f.__next__(), list(f))\ndef gen():\n    yield 5\n    yield 6\ng = gen()\nprint(g.__iter__() is g, g.__next__(), list(g))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_iter two-argument sentinel form",
            name: "callable-sentinel-iter",
            source: "values = [1, 2, 'stop', 99]\ndef next_value():\n    return values.pop(0)\nit = iter(next_value, 'stop')\nprint(iter(it) is it)\nprint(next(it), list(it))\ntry:\n    next(it)\nexcept StopIteration:\n    print('stopped')\nagain = [7, 'stop']\ndef next_again():\n    return again.pop(0)\nagain_it = iter(next_again, 'stop')\nprint(again_it.__iter__() is again_it, again_it.__next__())\nclass Counter:\n    def __init__(self):\n        self.value = 0\n    def __call__(self):\n        self.value += 1\n        return self.value\nprint(list(iter(Counter(), 4)))\ndef always_five():\n    return 5\nprint(next(iter(always_five, 5), 'done'))\nitems = [[1], [2], []]\ndef next_list():\n    return items.pop(0)\nprint(list(iter(next_list, [])))\ncounter = [0]\ndef stop_by_exception():\n    counter[0] += 1\n    if counter[0] == 3:\n        raise StopIteration\n    return counter[0]\nprint(list(iter(stop_by_exception, 99)))",
        },
        DiffCase {
            origin: "Lib/test/test_iter.py iterator __length_hint__ coverage",
            name: "iterator-length-hints",
            source: "it = iter([1, 2, 3])\nprint(it.__length_hint__(), next(it), it.__length_hint__(), list(it), it.__length_hint__())\nprint(iter((1, 2)).__length_hint__(), iter('abc').__length_hint__(), iter(b'ab').__length_hint__(), iter(range(4)).__length_hint__())\nd = {1: 10, 2: 20}\nkeys = iter(d)\nprint(keys.__length_hint__(), next(keys), keys.__length_hint__())\nprint(iter(d.values()).__length_hint__(), iter(d.items()).__length_hint__())\ngrowing = {1: 1}\ngrowing_keys = iter(growing)\ngrowing[2] = 2\nprint(growing_keys.__length_hint__())\nrev = reversed([1, 2, 3])\nprint(rev.__length_hint__(), next(rev), rev.__length_hint__())\nclass S:\n    def __init__(self):\n        self.items = [10, 20, 30]\n    def __getitem__(self, index):\n        if index >= len(self.items):\n            raise IndexError\n        return self.items[index]\n    def __len__(self):\n        return len(self.items)\nseq = iter(S())\nprint(seq.__length_hint__(), next(seq), seq.__length_hint__())\nclass N:\n    def __getitem__(self, index):\n        if index >= 2:\n            raise IndexError\n        return index\nprint(iter(N()).__length_hint__())",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::test_union / ::test_intersection / ::test_difference / ::test_symmetric_difference",
            name: "set-algebra-methods-and-operators",
            source: "s = set([1, 2, 3])\nu = s.union([3, 4], (5,))\ni = s.intersection([2, 3, 4], set([3, 4]))\nd = s.difference([1], set([3]))\nx = s.symmetric_difference([3, 4])\nprint(len(u), 5 in u, len(i), 3 in i, len(d), 2 in d, len(x), 4 in x, 3 in x)\nprint(s.issubset([1, 2, 3, 4]), s.issuperset([2, 3]), s.isdisjoint([4, 5]))\nalias = s\ns |= set([4])\ns &= set([2, 3, 4])\ns ^= set([3, 5])\ns -= set([2])\nprint(s is alias, len(s), 4 in s, 5 in s, 2 in s, 3 in s)\ns.intersection_update([4, 5])\ns.difference_update([4])\ns.symmetric_difference_update([5, 6])\nprint(len(s), 6 in s, 5 in s)",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::test_issubset / ::test_issuperset",
            name: "set-ordering-comparisons",
            source: "print({1} < {1, 2}, {1, 2} <= {1, 2}, {1, 2} > {1}, {1, 2} >= {1, 2})\nprint({1} < {1}, {1} <= {2}, {1} > {2}, {1} >= {2})",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_with_statement",
            name: "context-manager",
            source: "class Manager:\n    def __enter__(self):\n        print(\"enter\")\n        return 5\n    def __exit__(self, exc_type, exc, tb):\n        print(\"exit\")\nwith Manager() as value:\n    print(value)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_del_stmt",
            name: "starred-unpack-and-slice-assignment",
            source: "a, *middle, b = range(5)\nprint(a, middle, b)\nitems = [1, 2, 3]\nitems[1] = 5\nprint(items)\nprint(items[1:])",
        },
    ] {
        assert_cpython_output_parity(&case);
    }
}

#[test]
fn cpython_float_fromhex_bpo44954_diff_subset() {
    let oracle_probe = run_cpython("print(float.fromhex('0x.8p-1074').hex())")
        .expect("failed to run CPython bpo-44954 capability probe");
    let oracle_stdout = String::from_utf8(oracle_probe.stdout)
        .expect("CPython bpo-44954 probe emitted non-UTF-8 output");
    if oracle_stdout.trim() != "0x0.0p+0" {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::HexFloatTestCase::test_from_hex bpo-44954 regression",
        name: "float-fromhex-bpo-44954-regression",
        source: r#"import math
TINY = float.fromhex('0x0.0000000000001p-1022')

def identical(x, y):
    if x == y:
        if x != 0.0:
            return True
        return math.copysign(1.0, x) == math.copysign(1.0, y)
    return False

cases = [
    ('0x.8p-1074', 0.0), ('0x.80p-1074', 0.0), ('0x.81p-1074', TINY),
    ('0x8p-1078', 0.0), ('0x8.0p-1078', 0.0), ('0x8.1p-1078', TINY),
    ('0x80p-1082', 0.0), ('0x81p-1082', TINY), ('.8p-1074', 0.0),
    ('8p-1078', 0.0), ('-.8p-1074', -0.0), ('+8p-1078', 0.0),
]
ok = True
bad = []
for index, (text, expected) in enumerate(cases):
    value = float.fromhex(text)
    if not identical(value, expected):
        ok = False
        bad.append((index, value.hex(), expected.hex()))
print('bpo-44954', ok, len(cases), float.fromhex(cases[0][0]).hex(), float.fromhex(cases[-1][0]).hex(), len(bad))"#,
    });
}

#[test]
fn cpython_float_constructor_core_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_float, ::test_noargs, ::test_error_message, and ::test_float_with_comma public subset",
        name: "float-constructor-core",
        source: r#"def show(label, expr, allowed=None):
    try:
        value = expr()
        print(label, repr(value), type(value).__name__)
    except BaseException as error:
        if allowed is None:
            print(label, type(error).__name__, error.args[0])
        else:
            print(label, type(error).__name__, error.args[0] in allowed)

for item in [
    ('noargs', lambda: float()),
    ('float-float', lambda: float(3.14)),
    ('float-int', lambda: float(314)),
    ('float-spaces', lambda: float('  3.14  ')),
    ('arabic-digits', lambda: float('  ٣.١٤  ')),
    ('unicode-space', lambda: float('\u20033.14\u2002')),
    ('long-bytes', lambda: float(b'.' + b'1' * 1000)),
    ('long-str', lambda: float('.' + '1' * 1000)),
    ('comma', lambda: float('  3,14  ')),
    ('plus-comma', lambda: float('  +3,14  ')),
    ('minus-comma', lambda: float('  -3,14  ')),
    ('bad-hex-a', lambda: float('  0x3.1  ')),
    ('bad-hex-b', lambda: float('  -0x3.p-1  ')),
    ('bad-hex-c', lambda: float('  +0x3.p-1  ')),
    ('bad-sign-pp', lambda: float('++3.14')),
    ('bad-sign-pm', lambda: float('+-3.14')),
    ('bad-sign-mp', lambda: float('-+3.14')),
    ('bad-sign-mm', lambda: float('--3.14')),
    ('bad-dotnan', lambda: float('.nan')),
    ('bad-dotinf', lambda: float('+.inf')),
    ('bad-dot', lambda: float('.')),
    ('bad-negdot', lambda: float('-.')),
    ('bad-dict', lambda: float({}), [
        "float() argument must be a string or a real number, not 'dict'",
        "float() argument must be a string or a number, not 'dict'",
    ]),
    ('bad-d-exp', lambda: float('-1.7d29')),
    ('bad-D-exp', lambda: float('3D-14')),
    ('bad-japanese', lambda: float('こんにちは')),
    ('bad-half', lambda: float('½')),
    ('bad-mixed-half', lambda: float('123½')),
    ('bad-embedded-space', lambda: float('  123 456  ')),
    ('bad-bytes-space', lambda: float(b'  123 456  ')),
    ('bad-empty', lambda: float('')),
    ('bad-space', lambda: float(' '), [
        "could not convert string to float: ' '",
        "could not convert string to float: ''",
    ]),
    ('bad-whitespace', lambda: float('\t \n'), [
        "could not convert string to float: '\\t \\n'",
        "could not convert string to float: ''",
    ]),
    ('bad-arabic-suffix', lambda: float('٣١٤!')),
    ('bad-nul-a', lambda: float('123\0')),
    ('bad-nul-b', lambda: float('123\0 245')),
    ('bad-nul-c', lambda: float('123\x00245')),
    ('bad-bytes-nul', lambda: float(b'123\0')),
    ('bad-bytes-nonutf8', lambda: float(b'123\xa0')),
]:
    show(*item)"#,
    });
}

#[test]
fn cpython_float_conversion_protocol_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_floatconversion",
        name: "float-conversion-protocol",
        source: r#"class FloatSubclass(float):
    pass
class OtherFloatSubclass(float):
    pass
class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class Foo2(float):
    def __float__(self):
        return 42.0
class Foo3(float):
    def __new__(cls, value=0.0):
        return float.__new__(cls, 2 * value)
    def __float__(self):
        return self
class Foo4(float):
    def __float__(self):
        return 42
class FooStr(str):
    def __float__(self):
        return float(str(self)) + 1

def show(label, expr):
    try:
        value = expr()
        print(label, value, type(value).__name__, type(value) is float, type(value) is FloatSubclass)
    except Exception as error:
        print(label, error.__class__.__name__, str(error))

show('floatlike', lambda: float(FloatLike(42.0)))
show('foo2', lambda: float(Foo2()))
show('foo3', lambda: float(Foo3(21)))
show('foostr', lambda: float(FooStr('8')))
show('subclass-return', lambda: float(FloatLike(OtherFloatSubclass(42.0))))
show('subclass-ctor', lambda: FloatSubclass(FloatLike(OtherFloatSubclass(42.0))))
show('bad', lambda: float(Foo4(42)))"#,
    });
}

#[test]
fn cpython_float_bytes_like_input_types_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_non_numeric_input_types and ::test_float_memoryview",
        name: "float-bytes-like-input-types",
        source: r#"import array
class CustomStr(str):
    pass
class CustomBytes(bytes):
    pass
class CustomByteArray(bytearray):
    pass

for label, source in [
    ('bytes', bytes(b' 3.14  ')),
    ('bytearray', bytearray(b' 3.14  ')),
    ('CustomStr', CustomStr(' 3.14  ')),
    ('CustomBytes', CustomBytes(b' 3.14  ')),
    ('CustomByteArray', CustomByteArray(b' 3.14  ')),
    ('memoryview', memoryview(b' 3.14  ')),
    ('arrayB', array.array('B', b' 3.14  ')),
]:
    value = float(source)
    print(label, value, type(value) is float)

for label, source in [
    ('memoryview-core', memoryview(b'12.3')[1:4]),
    ('memoryview-nul', memoryview(b'12.3\0')[1:4]),
    ('memoryview-space', memoryview(b'12.3 ')[1:4]),
    ('memoryview-letter', memoryview(b'12.3A')[1:4]),
    ('memoryview-extra', memoryview(b'12.34')[1:4]),
]:
    print(label, float(source))

bad_cases = [
    ('bytes', bytes(b'AAAA'), "could not convert string to float: b'AAAA'"),
    ('bytearray', bytearray(b'AAAA'), "could not convert string to float: bytearray(b'AAAA')"),
    ('CustomStr', CustomStr('AAAA'), "could not convert string to float: 'AAAA'"),
    ('CustomBytes', CustomBytes(b'AAAA'), "could not convert string to float: b'AAAA'"),
    ('CustomByteArray', CustomByteArray(b'AAAA'), "could not convert string to float: CustomByteArray(b'AAAA')"),
    ('arrayB', array.array('B', b'AAAA'), "could not convert string to float: array('B', [65, 65, 65, 65])"),
]
for label, source, expected in bad_cases:
    try:
        float(source)
    except ValueError as error:
        print(label, error.__class__.__name__, error.args[0] == expected)

try:
    float(memoryview(b'AAAA'))
except ValueError as error:
    print('memoryview-bad', error.__class__.__name__, error.args[0].startswith('could not convert string to float: <memory at 0x'))
try:
    float({})
except TypeError as error:
    print('dict-bad', error.__class__.__name__, error.args[0] in [
        "float() argument must be a string or a number, not 'dict'",
        "float() argument must be a string or a real number, not 'dict'",
    ])"#,
    });
}

#[test]
fn cpython_bytes_basics_and_empty_index_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_basics, ::test_ord, and ::test_empty_sequence public subset",
        name: "bytes-basics-and-empty-index",
        source: r#"import sys
for ctor in [bytes, bytearray]:
    b = ctor()
    print(ctor.__name__, type(b) is ctor, b.__class__ is ctor, object.__getattribute__(b, '__class__') is ctor)
    b = ctor(b'\0A\x7f\x80\xff')
    print(ctor.__name__, [ord(b[i:i+1]) for i in range(len(b))])

indices = [0, 1, sys.maxsize, sys.maxsize + 1, 10**100, -1, -2, -sys.maxsize, -sys.maxsize - 1, -sys.maxsize - 2, -10**100]
for ctor in [bytes, bytearray]:
    b = ctor()
    results = []
    for index in indices:
        try:
            b[index]
            results.append('ok')
        except Exception as error:
            results.append(error.__class__.__name__)
    print(ctor.__name__, len(b), results)"#,
    });
}

#[test]
fn cpython_bytes_literal_runtime_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_tokenize.py bytes literal tokenization, Lib/test/test_bytes.py bytes runtime subset, and Lib/test/test_ast/test_ast.py bytes constants",
        name: "bytes-literal-runtime",
        source: r#"print(b'abc', B"abc")
print(br'\n', bR'\n', Rb'\n', RB'\n')
print(b'a' b'b' b'c')
print(b'\x41\n\377')
print(len(b'abc'), b'abc'[0], b'abc'[1:])
print(b'ab' + b'cd', b'ab' * 2)
print(b'a' == b'a', b'a' != b'b', b'a' == 'a')
print(bytes(), bytes(3), bytes(b'abc'))"#,
    });
}

#[test]
fn cpython_bytes_search_compare_slice_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest search, compare, reversed, and slice public subset",
        name: "bytes-search-compare-slice",
        source: r#"import sys

for ctor in [bytes, bytearray]:
    b = ctor(b'mississippi')
    print(b.count(b'i'), b.count(b'ss'), b.count(b'w'))
    print(b.count(105), b.count(119))
    print(b.count(b'i', 6), b.count(b'p', 6), b.count(b'i', 1, 3), b.count(b'p', 7, 9))
    print(b.find(b'ss'), b.find(b'w'), b.find(b'mississippian'))
    print(b.find(105), b.find(119))
    print(b.find(b'ss', 3), b.find(b'ss', 1, 7), b.find(b'ss', 1, 3))
    print(b.rfind(b'ss'), b.rfind(b'w'), b.rfind(b'mississippian'))
    print(b.rfind(105), b.rfind(119))
    print(b.rfind(b'ss', 3), b.rfind(b'ss', 0, 6))
    print(b.index(b'ss'), b.index(105), b.rindex(b'ss'), b.rindex(105))
    print(b.find(bytearray(b'i')), b.find(memoryview(b'i')))
    print(b.find(b'm', None, None), b.rfind(b's', None), b.index(b's', None, -2), b.rindex(b's', None, -2), b.count(b's', None, None))
    for method_name in ['count', 'find', 'index', 'rfind', 'rindex']:
        method = getattr(b, method_name)
        errors = []
        for value in [-1, 256, 9999]:
            try:
                method(value)
            except (TypeError, ValueError) as error:
                errors.append(error.__class__.__name__)
        print(ctor.__name__, method_name, errors)
    for expr in [lambda: b.index(b'w'), lambda: b.rindex(b'w'), lambda: b.find('i')]:
        try:
            expr()
        except (TypeError, ValueError) as error:
            print(ctor.__name__, error.__class__.__name__)

for ctor in [bytes, bytearray]:
    b1 = ctor([1, 2, 3])
    b2 = ctor([1, 2, 3])
    b3 = ctor([1, 3])
    print(b1 == b2, b2 != b3, b1 <= b2, b1 <= b3, b1 < b3)
    print(b1 >= b2, b3 >= b2, b3 > b2)
    print(b1 != b2, b2 == b3, b1 > b2, b1 > b3, b1 >= b3, b1 < b2, b3 < b2, b3 <= b2)
    print(ctor(b'\0a\0b\0c') == 'abc', ctor(b'\0\0\0a\0\0\0b\0\0\0c') == 'abc', ctor(b'a\0b\0c\0') == 'abc', ctor(b'a\0\0\0b\0\0\0c\0\0\0') == 'abc', ctor() == str(), ctor() != str())
    input_values = list(map(ord, 'Hello'))
    b = ctor(input_values)
    output = list(reversed(b))
    input_values.reverse()
    print(output == input_values)
    def by(text):
        return ctor(map(ord, text))
    b = by('Hello, world')
    print(b[:5] == by('Hello'), b[1:5] == by('ello'), b[5:7] == by(', '), b[7:] == by('world'), b[7:12] == by('world'), b[7:100] == by('world'))
    print(b[:-7] == by('Hello'), b[-11:-7] == by('ello'), b[-7:-5] == by(', '), b[-5:] == by('world'), b[-5:100] == by('world'), b[-100:5] == by('Hello'))
    L = list(range(255))
    b = ctor(L)
    indices = (0, None, 1, 3, 19, 100, sys.maxsize, -1, -2, -31, -100)
    ok = True
    for start in indices:
        for stop in indices:
            for step in indices[1:]:
                if b[start:stop:step] != ctor(L[start:stop:step]):
                    ok = False
    print(ok)"#,
    });
}

#[test]
fn cpython_bytes_search_bounds_index_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py BaseBytesTest search/prefix/suffix bound __index__ public subset",
        name: "bytes-search-bounds-index",
        source: r#"class One:
    def __index__(self):
        return 1
class Three:
    def __index__(self):
        return 3
class NegTwo:
    def __index__(self):
        return -2
class Bad:
    def __index__(self):
        raise RuntimeError('boom')

for ctor in [bytes, bytearray]:
    b = ctor(b'mississippi')
    print(b.count(b'i', One()), b.count(b'i', One(), Three()))
    print(b.find(b'i', One()), b.find(b'i', One(), Three()))
    print(b.rfind(b'i', One(), Three()), b.index(b'i', One()), b.rindex(b'i', One()))
    print(b.startswith(b'ss', Three()), b.endswith(b'pi', NegTwo()))
    for expr in [
        lambda: b.find(b'i', Bad()),
        lambda: b.count(b'i', One(), Bad()),
        lambda: b.startswith(b'm', Bad()),
        lambda: b.endswith(b'i', One(), Bad()),
    ]:
        try:
            expr()
        except RuntimeError as error:
            print(error.__class__.__name__, error.args[0])"#,
    });
}

#[test]
fn cpython_bytes_prefix_suffix_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_startswith, ::test_endswith, and public None-bound subset",
        name: "bytes-prefix-suffix-methods",
        source: r#"for ctor in [bytes, bytearray]:
    b = ctor(b'hello')
    print(ctor().startswith(b'anything'), b.startswith(b'hello'), b.startswith(b'hel'), b.startswith(b'h'), b.startswith(b'hellow'), b.startswith(b'ha'))
    print(ctor().endswith(b'anything'), b.endswith(b'hello'), b.endswith(b'llo'), b.endswith(b'o'), b.endswith(b'whello'), b.endswith(b'no'))
    print(b.startswith(bytearray(b'he')), b.startswith(memoryview(b'he')), b.startswith((b'x', b'he')), b.startswith((bytearray(b'x'), memoryview(b'he'))))
    print(b.endswith(bytearray(b'lo')), b.endswith(memoryview(b'lo')), b.endswith((b'x', b'lo')), b.endswith((bytearray(b'x'), memoryview(b'lo'))))
    print(b.startswith(b'l', 2), b.startswith(b'l', -2, None), b.startswith(b'h', None, -2), b.startswith(b'x', None, None))
    print(b.endswith(b'o', None), b.endswith(b'o', -2, None), b.endswith(b'l', None, -2), b.endswith(b'x', None, None))
    print(b.startswith((b'h', 'bad')), b.endswith((b'o', 'bad')), b.startswith(()), b.endswith(()))

b = b'hello'
for expr in [
    lambda: b.startswith([b'h']),
    lambda: b.endswith([b'o']),
    lambda: b.startswith((b'x', 'h')),
    lambda: b.endswith((b'x', 'o')),
    lambda: b.startswith(b'h', None, None, None),
    lambda: b.endswith(b'o', None, None, None),
]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_strip_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_strip_bytearray, ::test_strip_string_error, and ::test_strip_int_error public subset",
        name: "bytes-strip-methods",
        source: r#"for ctor in [bytes, bytearray]:
    b = ctor(b'   abc \t\n\r\f\v')
    print(b.strip(), b.lstrip(), b.rstrip())
    b = ctor(b'abc')
    print(b.strip(memoryview(b'ac')), b.lstrip(memoryview(b'ac')), b.rstrip(memoryview(b'ac')))
    print(ctor(b'xyzzyhelloxyzzy').strip(b'xyz'), ctor(b'xyzzyhelloxyzzy').lstrip(b'xyz'), ctor(b'xyzzyhelloxyzzy').rstrip(b'xyz'))
    print(ctor(b'abc').strip(bytearray(b'ac')), ctor(b'abc').strip(b''), ctor(b'abc').strip(None))
    for expr in [
        lambda: ctor(b'abc').strip('ac'),
        lambda: ctor(b'abc').lstrip('ac'),
        lambda: ctor(b'abc').rstrip('ac'),
        lambda: ctor(b' abc ').strip(32),
        lambda: ctor(b' abc ').lstrip(32),
        lambda: ctor(b' abc ').rstrip(32),
        lambda: ctor(b'abc').strip(b'ac', b'bad'),
    ]:
        try:
            expr()
        except TypeError as error:
            print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_remove_affix_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/string_tests.py::test_removeprefix and ::test_removesuffix bytes/bytearray public subset",
        name: "bytes-remove-affix-methods",
        source: r#"for ctor in [bytes, bytearray]:
    b = ctor(b'spam')
    print(b.removeprefix(b'sp'), b.removeprefix(bytearray(b'sp')), b.removeprefix(memoryview(b'sp')))
    print(b.removeprefix(b'python'), b.removeprefix(b'spider'), b.removeprefix(b'spam and eggs'))
    print(ctor(b'').removeprefix(b''), ctor(b'').removeprefix(b'abcde'), ctor(b'abcde').removeprefix(b''), ctor(b'abcde').removeprefix(b'abcde'))
    print(b.removesuffix(b'am'), ctor(b'spamspamspam').removesuffix(b'spam'), b.removesuffix(b'python'), b.removesuffix(b'blam'), b.removesuffix(b'eggs and spam'))
    print(ctor(b'').removesuffix(b''), ctor(b'').removesuffix(b'abcde'), ctor(b'abcde').removesuffix(b''), ctor(b'abcde').removesuffix(b'abcde'))
    print(ctor(b'abc').removesuffix(bytearray(b'bc')), ctor(b'abc').removesuffix(memoryview(b'bc')))
    for expr in [
        lambda: b.removeprefix(),
        lambda: b.removeprefix('sp'),
        lambda: b.removeprefix(42),
        lambda: b.removeprefix(42, b'sp'),
        lambda: b.removeprefix(b'sp', 42),
        lambda: b.removeprefix((b'sp',)),
        lambda: b.removesuffix(),
        lambda: b.removesuffix('am'),
        lambda: b.removesuffix(42),
        lambda: b.removesuffix(42, b'am'),
        lambda: b.removesuffix(b'am', 42),
        lambda: b.removesuffix((b'am',)),
    ]:
        try:
            expr()
        except TypeError as error:
            print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_alignment_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_center, ::test_ljust, ::test_rjust, and ::test_xjust_int_error public subset",
        name: "bytes-alignment-methods",
        source: r#"for ctor in [bytes, bytearray]:
    b = ctor(b'abc')
    for fill_type in [bytes, bytearray]:
        print(b.center(7, fill_type(b'-')), b.ljust(7, fill_type(b'-')), b.rjust(7, fill_type(b'-')))
    print(b.center(6), b.center(3), b.center(2))
    print(b.ljust(6), b.ljust(3), b.ljust(2))
    print(b.rjust(6), b.rjust(3), b.rjust(2))
    for expr in [
        lambda: b.center(),
        lambda: b.ljust(),
        lambda: b.rjust(),
        lambda: b.center(7, 32),
        lambda: b.ljust(7, 32),
        lambda: b.rjust(7, 32),
        lambda: b.center(7, b''),
        lambda: b.ljust(7, b'--'),
        lambda: b.rjust(7, bytearray(b'--')),
    ]:
        try:
            expr()
        except TypeError as error:
            print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_replace_partition_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_replace, ::test_partition, ::test_rpartition, and public error subset",
        name: "bytes-replace-partition-methods",
        source: r#"for ctor in [bytes, bytearray]:
    b = ctor(b'mississippi')
    print(b.replace(b'i', b'a'))
    print(b.replace(b'ss', b'x'))
    print(b.replace(bytearray(b'i'), memoryview(b'a')))
    print(b.replace(b'i', b'a', 2), b.replace(b'i', b'a', 0))
    print(b.replace(b'', b'-'))
    print(b.replace(b'', b'-', 2))
    print(b.partition(b'ss'))
    print(b.partition(b'w'))
    print(b.rpartition(b'ss'))
    print(b.rpartition(b'i'))
    print(b.rpartition(b'w'))

b = b'a b'
for expr in [
    lambda: b.replace(32, b''),
    lambda: b.partition(' '),
    lambda: b.partition(32),
    lambda: b.rpartition(' '),
    lambda: b.rpartition(32),
    lambda: b.partition(b''),
    lambda: b.rpartition(b''),
]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)

for ctor in [bytes, bytearray]:
    b = ctor(b'aa')
    for count in [0, 1, 2, 3]:
        print(ctor.__name__, count, b.replace(b'a', b'b', count))"#,
    });
}

#[test]
fn cpython_bytes_split_rsplit_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest split/rsplit public subset",
        name: "bytes-split-rsplit-methods",
        source: r#"for ctor in [bytes, bytearray]:
    print(ctor(b'a b c').split())
    print(ctor(b' a  b c ').split())
    print(ctor(b'a b c').split(None, 1))
    print(ctor(b'  a b  ').split(None, 0))
    print(ctor(b'a|b|c').split(b'|'))
    print(ctor(b'a|b|c').split(bytearray(b'|'), 1))
    print(ctor(b'a|b|c').split(memoryview(b'|'), 1))
    print(ctor(b'a||b||c').split(b'||', 1))
    print(ctor(b'a b c').rsplit())
    print(ctor(b'a b c').rsplit(None, 1))
    print(ctor(b'a|b|c').rsplit(b'|', 1))
    print(ctor(b'a||b||c').rsplit(b'||', 1))
    print(ctor(b'a|b|c').split(sep=b'|'), ctor(b'a|b|c').split(b'|', maxsplit=1), ctor(b'a b c').split(maxsplit=1))
    print(ctor(b'a|b|c').rsplit(sep=b'|'), ctor(b'a|b|c').rsplit(b'|', maxsplit=1), ctor(b'a b c').rsplit(maxsplit=1))

for ctor in [bytes, bytearray]:
    for b in [ctor(b'a\x1cb'), ctor(b'a\x1db'), ctor(b'a\x1eb'), ctor(b'a\x1fb')]:
        print(b.split())
    b = ctor(b'\x09\x0a\x0b\x0c\x0d\x1c\x1d\x1e\x1f')
    print(b.split())
    print(b.rsplit())

b = b'a b'
for expr in [
    lambda: b.split(' '),
    lambda: b.rsplit(' '),
    lambda: b.split(32),
    lambda: b.rsplit(32),
    lambda: b.split(b''),
    lambda: b.rsplit(b''),
    lambda: b.split(maxsplit=None),
    lambda: b.rsplit(maxsplit=None),
]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_splitlines_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/string_tests.py::test_splitlines bytes/bytearray public subset",
        name: "bytes-splitlines-methods",
        source: r#"for ctor in [bytes, bytearray]:
    print(ctor(b'abc\ndef\n\rghi').splitlines())
    print(ctor(b'abc\ndef\n\r\nghi').splitlines())
    print(ctor(b'abc\ndef\r\nghi').splitlines())
    print(ctor(b'abc\ndef\r\nghi\n').splitlines())
    print(ctor(b'abc\ndef\r\nghi\n\r').splitlines())
    print(ctor(b'\nabc\ndef\r\nghi\n\r').splitlines())
    print(ctor(b'\nabc\ndef\r\nghi\n\r').splitlines(False))
    print(ctor(b'\nabc\ndef\r\nghi\n\r').splitlines(keepends=True))
    print(ctor(b'').splitlines(), ctor(b'one').splitlines(), ctor(b'one\n').splitlines(), ctor(b'\n').splitlines(), ctor(b'\r\n').splitlines())
    print(ctor(b'a\vb\fc\x1cd\x1ee\x85f').splitlines())
    print(ctor(b'a\nb\r\nc\rd').splitlines(True))
    for expr in [
        lambda: ctor(b'abc').splitlines(True, False),
        lambda: ctor(b'abc').splitlines(keepends=True, extra=False),
        lambda: ctor(b'abc').splitlines(extra=True),
    ]:
        try:
            expr()
        except TypeError as error:
            print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_ascii_case_predicate_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/string_tests.py ASCII case and predicate methods as applied to bytes/bytearray",
        name: "bytes-ascii-case-predicate-methods",
        source: r#"for ctor in [bytes, bytearray]:
    b = ctor(b'HeLLo cOmPuTeRs 123\t\xff')
    print(b.lower())
    print(b.upper())
    print(b.capitalize())
    print(ctor(b'fOrMaT thIs aS titLe String').title())
    print(ctor(b'fOrMaT,thIs-aS*titLe;String').title())
    print(ctor(b'HeLLo cOmpUteRs').swapcase())
    print(ctor(b'hello').islower(), ctor(b'abc\n').islower(), ctor(b'aBc').islower(), ctor(b'').islower())
    print(ctor(b'ABC').isupper(), ctor(b'ABC\n').isupper(), ctor(b'AbC').isupper(), ctor(b'').isupper())
    print(ctor(b'A Titlecased Line').istitle(), ctor(b'A\nTitlecased Line').istitle(), ctor(b'Not a capitalized String').istitle(), ctor(b'NOT').istitle())
    print(ctor(b'abc').isalpha(), ctor(b'aBc123').isalpha(), ctor(b'').isalpha())
    print(ctor(b'123abc456').isalnum(), ctor(b'aBc000 ').isalnum(), ctor(b'').isalnum())
    print(ctor(b'0123456789').isdigit(), ctor(b'0123456789a').isdigit(), ctor(b'').isdigit())
    print(ctor(b' \t\n\r\v\f').isspace(), ctor(b' \t\n\r\v\fx').isspace(), ctor(b'').isspace())
    print(ctor(b'\x00\x7f').isascii(), ctor(b'\x80').isascii(), ctor(b'').isascii())
    for expr in [
        lambda: b.lower(42),
        lambda: b.upper(42),
        lambda: b.capitalize(42),
        lambda: b.title(42),
        lambda: b.swapcase(42),
        lambda: b.islower(42),
        lambda: b.isupper(42),
        lambda: b.istitle(42),
        lambda: b.isalpha(42),
        lambda: b.isalnum(42),
        lambda: b.isdigit(42),
        lambda: b.isspace(42),
        lambda: b.isascii(42),
    ]:
        try:
            expr()
        except TypeError as error:
            print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_expandtabs_zfill_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/string_tests.py::test_expandtabs and ::test_zfill bytes/bytearray public subset",
        name: "bytes-expandtabs-zfill-methods",
        source: r#"for ctor in [bytes, bytearray]:
    sample = ctor(b'abc\rab\tdef\ng\thi')
    print(sample.expandtabs())
    print(sample.expandtabs(8))
    print(sample.expandtabs(4))
    print(ctor(b'abc\r\nab\tdef\ng\thi').expandtabs())
    print(ctor(b'abc\r\nab\tdef\ng\thi').expandtabs(tabsize=4))
    print(ctor(b'abc\r\nab\r\ndef\ng\r\nhi').expandtabs(4))
    print(ctor(b' \ta\n\tb').expandtabs(1))
    print(ctor(b'ab\tc').expandtabs(-1), ctor(b'ab\tc').expandtabs(0), ctor(b'ab\tc').expandtabs(1), ctor(b'ab\tc').expandtabs(2))
    print(ctor(b'\t\ta').expandtabs(4), ctor(b'a\tb\tc').expandtabs(3))
    print(ctor(b'ab\tc').expandtabs(True), ctor(b'ab\tc').expandtabs(False))
    print(ctor(b'123').zfill(2), ctor(b'123').zfill(3), ctor(b'123').zfill(4))
    print(ctor(b'+123').zfill(3), ctor(b'+123').zfill(4), ctor(b'+123').zfill(5))
    print(ctor(b'-123').zfill(3), ctor(b'-123').zfill(4), ctor(b'-123').zfill(5))
    print(ctor(b'').zfill(3), ctor(b'34').zfill(1), ctor(b'34').zfill(4))
    for expr in [
        lambda: sample.expandtabs(42, 42),
        lambda: sample.expandtabs(None),
        lambda: sample.expandtabs('4'),
        lambda: sample.expandtabs(size=4),
        lambda: sample.zfill(),
        lambda: sample.zfill(4, 0),
        lambda: sample.zfill('4'),
        lambda: sample.zfill(width=4),
    ]:
        try:
            expr()
        except (TypeError, OverflowError) as error:
            print(error.__class__.__name__)
print('expandtabs' in dir(bytes), 'zfill' in dir(bytes), 'expandtabs' in dir(bytearray), 'zfill' in dir(bytearray))"#,
    });
}

#[test]
fn cpython_bytes_maketrans_translate_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_maketrans and ::test_translate public subset",
        name: "bytes-maketrans-translate",
        source: r#"for ctor in [bytes, bytearray]:
    table = ctor.maketrans(b'abc', b'xyz')
    print(type(table).__name__, len(table), table[ord('a')], table[ord('b')], table[ord('c')])
    table = ctor.maketrans(memoryview(b'\xfd\xfe\xff'), bytearray(b'xyz'))
    print(type(table).__name__, len(table), table[0xfd], table[0xfe], table[0xff])
    table = ctor().maketrans(bytearray(b'a'), memoryview(b'b'))
    print(type(table).__name__, table[ord('a')])
    b = ctor(b'hello')
    rosetta = bytearray(range(256))
    rosetta[ord('o')] = ord('e')
    print(b.translate(rosetta))
    print(b.translate(rosetta, b''))
    print(b.translate(rosetta, b'l'))
    print(b.translate(None, b'e'))
    print(b.translate(rosetta, delete=b''))
    print(b.translate(rosetta, delete=b'l'))
    print(b.translate(None, delete=b'e'))
    print(b.translate(memoryview(bytes(range(256)))))
    for expr in [
        lambda: ctor.maketrans(b'abc', b'xyzq'),
        lambda: ctor.maketrans('abc', 'def'),
        lambda: b.translate(),
        lambda: b.translate(None, None),
        lambda: b.translate(bytes(range(255))),
        lambda: b.translate(None, b'l', delete=b'e'),
        lambda: b.translate(None, bad=b'e'),
    ]:
        try:
            expr()
        except (TypeError, ValueError) as error:
            print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_join_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_join public subset",
        name: "bytes-join",
        source: r#"for ctor in [bytes, bytearray]:
    empty = ctor(b'')
    print(empty.join([]))
    print(empty.join([b'']))
    for parts in [[b'abc'], [b'a', b'bc'], [b'ab', b'c'], [b'a', b'b', b'c']]:
        values = list(map(ctor, parts))
        print(empty.join(values), empty.join(tuple(values)), empty.join(iter(values)))
    dot_join = ctor(b'.:').join
    print(dot_join([b'ab', b'cd']))
    print(dot_join([memoryview(b'ab'), b'cd']))
    print(dot_join([b'ab', memoryview(b'cd')]))
    print(dot_join([bytearray(b'ab'), b'cd']))
    print(dot_join([b'ab', bytearray(b'cd')]))
    seq = [b'abc'] * 100
    joined = dot_join(seq)
    expected = b'abc' + b'.:abc' * 99
    print(joined == ctor(expected), len(joined))
    joined = empty.join(seq)
    expected = b'abc' * 100
    print(joined == ctor(expected), len(joined))
    for expr in [
        lambda: ctor(b' ').join(None),
        lambda: dot_join([bytearray(b'ab'), 'cd', b'ef']),
        lambda: dot_join([memoryview(b'ab'), 'cd', b'ef']),
    ]:
        try:
            expr()
        except TypeError as error:
            print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_copy_module_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_copy public subset",
        name: "bytes-copy-module",
        source: r#"import copy
for ctor in [bytes, bytearray]:
    original = ctor(b'abcd')
    shallow = copy.copy(original)
    deep = copy.deepcopy(original)
    print(type(shallow).__name__, shallow == original, type(deep).__name__, deep == original)
    if isinstance(original, bytearray):
        print(shallow is original, deep is original)
        shallow.append(ord('x'))
        deep.append(ord('y'))
        print(original, shallow, deep)
print(copy.copy(x=b'kw'))
for expr in [
    lambda: copy.copy(),
    lambda: copy.copy(b'a', b'b'),
    lambda: copy.copy(b'a', x=b'b'),
]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_constructor_concat_repeat_contains_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_from_int, ::test_concat, ::test_repeat, and ::test_contains public subset",
        name: "bytes-constructor-concat-repeat-contains",
        source: r#"import sys
for ctor in [bytes, bytearray]:
    print(ctor(0), len(ctor(10)), len(ctor(10000)))
    print(ctor(10) == ctor([0] * 10), ctor(10000) == ctor([0] * 10000))
    b1 = ctor(b'abc')
    b2 = ctor(b'def')
    print(b1 + b2)
    print(b1 + bytes(b'def'))
    print(bytes(b'def') + b1)
    for expr in [lambda: b1 + 'def', lambda: 'abc' + b2]:
        try:
            expr()
        except TypeError as error:
            print(error.__class__.__name__)
    for sample in [b'abc', ctor(b'abc')]:
        print(sample * 3, sample * 0, sample * -1)
        for expr in [lambda: sample * 3.14, lambda: 3.14 * sample]:
            try:
                expr()
            except TypeError as error:
                print(error.__class__.__name__)
    print(ctor(b'x') * 100 == ctor([ord('x')] * 100))
    b = ctor(b'abc')
    for needle in [ord('a'), int(ord('a')), 200]:
        print(needle in b)
    for needle in [300, -1, sys.maxsize + 1, None, float(ord('a')), 'a']:
        try:
            needle in b
        except (TypeError, ValueError) as error:
            print(error.__class__.__name__)
    for needle in [bytes(b''), bytearray(b''), memoryview(b''), bytes(b'a'), bytearray(b'b'), memoryview(b'c'), bytes(b'ab'), bytearray(b'bc'), bytes(b'ac'), bytes(b'd')]:
        print(needle in b)"#,
    });
}

#[test]
fn cpython_bytes_iterable_constructor_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest iterable-constructor public subset",
        name: "bytes-iterable-constructor",
        source: r#"import sys

class Indexable:
    def __init__(self, value=0):
        self.value = value
    def __index__(self):
        return self.value
class S:
    def __getitem__(self, index):
        return (1, 2, 3)[index]
class C:
    pass

for ctor in [bytes, bytearray]:
    print(ctor(range(5)))
    print(list(ctor(iter(range(5)))))
    print(ctor({42}))
    print(sorted(list(ctor({43, 45}))))
    odd = ctor(i for i in range(10) if i % 2)
    print(len(odd), list(odd))
    print(ctor([1, 2, 3]))
    print(ctor((1, 2, 3)))
    print(ctor(S()))
    print(list(ctor([Indexable(), Indexable(1), Indexable(254), Indexable(255)])))

for expr in [
    lambda: bytes([Indexable(-1)]),
    lambda: bytes([Indexable(256)]),
    lambda: bytearray([Indexable(-1)]),
    lambda: bytearray([Indexable(256)]),
    lambda: bytes(0.0),
    lambda: bytearray(0.0),
    lambda: bytes(['0']),
    lambda: bytes([0.0]),
    lambda: bytes([None]),
    lambda: bytes([C()]),
    lambda: bytearray(['0']),
    lambda: bytearray([0.0]),
    lambda: bytearray([None]),
    lambda: bytearray([C()]),
]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)

cases = [
    ('encoding-only', lambda ctor: ctor(encoding='ascii')),
    ('errors-only', lambda ctor: ctor(errors='ignore')),
    ('int-encoding', lambda ctor: ctor(0, 'ascii')),
    ('bytes-encoding', lambda ctor: ctor(b'', 'ascii')),
    ('int-errors', lambda ctor: ctor(0, errors='ignore')),
    ('bytes-errors', lambda ctor: ctor(b'', errors='ignore')),
    ('empty-str', lambda ctor: ctor('')),
    ('empty-str-errors', lambda ctor: ctor('', errors='ignore')),
    ('encoding-bytes', lambda ctor: ctor('', b'ascii')),
    ('errors-bytes', lambda ctor: ctor('', 'ascii', b'ignore')),
    ('min-item', lambda ctor: ctor([-sys.maxsize])),
    ('min1-item', lambda ctor: ctor([-sys.maxsize - 1])),
    ('underflow-item', lambda ctor: ctor([-sys.maxsize - 2])),
    ('huge-neg-item', lambda ctor: ctor([-10**100])),
    ('257-item', lambda ctor: ctor([257])),
    ('max-item', lambda ctor: ctor([sys.maxsize])),
    ('overflow-item', lambda ctor: ctor([sys.maxsize + 1])),
    ('huge-pos-item', lambda ctor: ctor([10**100])),
]
for ctor in [bytes, bytearray]:
    for label, factory in cases:
        try:
            factory(ctor)
        except (TypeError, ValueError) as error:
            print(ctor.__name__, label, error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_constructor_exception_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_constructor_exceptions public subset",
        name: "bytes-constructor-exception",
        source: r#"class BadInt:
    def __index__(self):
        1/0
class BadIterable:
    def __iter__(self):
        1/0
for ctor in [bytes, bytearray]:
    for expr in [
        lambda ctor=ctor: ctor(BadInt()),
        lambda ctor=ctor: ctor([BadInt()]),
        lambda ctor=ctor: ctor(BadIterable()),
    ]:
        try:
            expr()
        except ZeroDivisionError as error:
            print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_mutating_list_constructor_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_from_mutating_list public subset",
        name: "bytes-mutating-list-constructor",
        source: r#"for ctor in [bytes, bytearray]:
    class X:
        def __index__(self):
            a.clear()
            return 42
    a = [X(), X()]
    print(ctor(a))

    class Y:
        def __index__(self):
            if len(a) < 1000:
                a.append(self)
            return 42
    a = [Y()]
    result = ctor(a)
    print(len(result), result[:5], result[-5:])"#,
    });
}

#[test]
fn cpython_bytes_bytearray_index_error_and_hash_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BytesTest::test_getitem_error and ByteArrayTest getitem/setitem/nohash public subset",
        name: "bytes-bytearray-index-error-and-hash",
        source: r#"for name, expr in [
    ('bytes-getitem', lambda: b'python'['a']),
    ('bytearray-getitem', lambda: bytearray(b'python')['a']),
    ('bytearray-hash', lambda: hash(bytearray())),
]:
    try:
        expr()
    except TypeError as error:
        print(name, str(error))
b = bytearray(b'python')
try:
    b['a'] = 'python'
except TypeError as error:
    print('bytearray-setitem', str(error))"#,
    });
}

#[test]
fn cpython_bytes_bytearray_subclass_repr_and_compare_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_custom and AssortedBytesTest repr/compare public subset",
        name: "bytes-bytearray-subclass-repr-and-compare",
        source: r#"class B(bytes):
    pass
class B2(bytes):
    pass
class BA(bytearray):
    class Nested(bytearray):
        pass
b = B(b'abc')
print(str(b), repr(b), B(B2(b'abc')) == B(b'abc'))
print(b == b'abc', b == bytearray(b'abc'), b == memoryview(b'abc'))
ba = BA(b'abc')
print(str(ba), repr(ba), BA.Nested(b'abc'))
print(ba == BA(b'abc'), ba == bytearray(b'abc'), ba == b'abc', ba == memoryview(b'abc'))
print(B(b'a') < b'b', BA(b'a') < b'b')"#,
    });
}

#[test]
fn cpython_bytes_bytearray_assorted_public_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::AssortedBytesTest::test_from_bytearray and ::test_compare_bytes_to_bytearray",
        name: "bytes-bytearray-assorted-public",
        source: r#"sample = bytes(b'Hello world\n\x80\x81\xfe\xff')
converted = bytearray(memoryview(sample))
print('from-bytearray', converted == bytearray(sample), bytes(converted) == sample, len(converted))
print('bytes-left', b'abc' == bytearray(b'abc'), b'ab' != bytearray(b'abc'), b'ab' <= bytearray(b'abc'), b'ab' < bytearray(b'abc'), b'abc' >= bytearray(b'ab'), b'abc' > bytearray(b'ab'))
print('bytes-left-false', b'abc' != bytearray(b'abc'), b'ab' == bytearray(b'abc'), b'ab' > bytearray(b'abc'), b'ab' >= bytearray(b'abc'), b'abc' < bytearray(b'ab'), b'abc' <= bytearray(b'ab'))
print('bytearray-left', bytearray(b'abc') == b'abc', bytearray(b'ab') != b'abc', bytearray(b'ab') <= b'abc', bytearray(b'ab') < b'abc', bytearray(b'abc') >= b'ab', bytearray(b'abc') > b'ab')
print('bytearray-left-false', bytearray(b'abc') != b'abc', bytearray(b'ab') == b'abc', bytearray(b'ab') > b'abc', bytearray(b'ab') >= b'abc', bytearray(b'abc') < b'ab', bytearray(b'abc') <= b'ab')"#,
    });
}

#[test]
fn cpython_bytes_format_method_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::AssortedBytesTest::test_format",
        name: "bytes-format-method",
        source: r#"for value in [b'abc', bytearray(b'abc')]:
    print(type(value).__name__, format(value), format(value, ''), value.__format__(''))
    print(f'{value!s:>18}')
    for spec in ['s', '>8', '.2s']:
        try:
            format(value, spec)
        except TypeError as error:
            print(type(value).__name__, spec, error.__class__.__name__, type(value).__name__ in str(error))
    try:
        value.__format__('s')
    except TypeError as error:
        print(type(value).__name__, 'dunder', error.__class__.__name__, type(value).__name__ in str(error))"#,
    });
}

#[test]
fn cpython_bytes_bytearray_type_doc_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::AssortedBytesTest::test_doc public subset",
        name: "bytes-bytearray-type-doc",
        source: r#"for typ, prefix in [(bytes, 'bytes('), (bytearray, 'bytearray(')]:
    doc = typ.__doc__
    print(typ.__name__, doc is not None, doc.startswith(prefix), doc.splitlines()[0], '__doc__' in dir(typ))"#,
    });
}

#[test]
fn cpython_bytes_percent_format_and_rmod_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_mod, ::test_imod, and ::test_rmod public subset",
        name: "bytes-percent-format-and-rmod",
        source: r#"for ctor in [bytes, bytearray]:
    fmt = ctor(b'hello, %b!')
    result = fmt % b'world'
    print(type(result).__name__, result, fmt == ctor(b'hello, %b!'))
    fmt = ctor(b'%s / 100 = %d%%')
    print(type((fmt % (b'seventy-nine', 79))).__name__, fmt % (b'seventy-nine', 79))
    print(ctor(b'hello,\x00%b!') % b'world')
    print(ctor(b'...%(foo)b...') % {b'foo': b'abc'})
    print(ctor(b'...%(f(o)o)b...') % {b'f(o)o': b'abc', b'foo': b'bar'})
    print(ctor(b'%*b') % (5, b'abc'))
    print(ctor(b'%*b') % (-5, b'abc'))
    print(ctor(b'%*.*b') % (5, 2, b'abc'))
    print(ctor(b'%i%b %*.*b') % (10, b'3', 5, 3, b'abc'))
    print(ctor(b'%b %s') % (memoryview(b'ab'), memoryview(b'cd')))
    print(ctor(b'%c') % b'a')
    print(ctor(b'%d') % 3.14)
    print(ctor(b'%f %F %.2f %e %E %g %G') % (1.0, 1.0, 1.25, 1234.5, 1234.5, 1.25, 1.25))
    print(ctor(b'%+08.2f') % 1.25)
    holder = ctor(b'hello, %b!')
    alias = holder
    holder %= b'world'
    print(type(holder).__name__, holder, alias == ctor(b'hello, %b!'))
    if isinstance(alias, bytearray):
        print(holder is alias, alias)
    for expr in [
        lambda: ctor(b'%x') % 3.14,
        lambda: ctor(b'%c') % b'ab',
        lambda: ctor(b'%c') % 256,
        lambda: ctor(b'%b') % 'text',
        lambda: ctor(b'%c') % memoryview(b'a'),
    ]:
        try:
            expr()
        except (TypeError, OverflowError, ValueError) as error:
            print(error.__class__.__name__)
    try:
        object() % ctor(b'abc')
    except TypeError as error:
        print(error.__class__.__name__)
    print(ctor(b'abc').__rmod__('%r') is NotImplemented)
    print(ctor.__rmod__(ctor(b'abc'), '%r') is NotImplemented)"#,
    });
}

#[test]
fn cpython_bytes_percent_dunder_bytes_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_mod direct dunder behavior public subset",
        name: "bytes-percent-dunder-bytes",
        source: r#"class BytesResult:
    def __bytes__(self):
        return b'xx'
class StrResult:
    def __bytes__(self):
        return 'not bytes'
class Raises:
    def __bytes__(self):
        raise RuntimeError('boom')
class NonAsciiRepr:
    def __repr__(self):
        return chr(233)
class B(bytes):
    pass
class BA(bytearray):
    pass

for ctor in [bytes, bytearray]:
    print(ctor(b'%b') % BytesResult())
    print(ctor(b'%s') % BytesResult())
    for expr in [
        lambda: ctor(b'%b') % StrResult(),
        lambda: ctor(b'%b') % Raises(),
        lambda: ctor(b'%(x)*b') % {b'x': b'a'},
        lambda: ctor(b'%(x)b %b') % {b'x': b'a'},
        lambda: ctor(b'%(x)*b') % ({b'x': b'a'}, 3),
        lambda: ctor(b'%f') % 'text',
    ]:
        try:
            expr()
        except Exception as error:
            print(error.__class__.__name__, error.args[0])
    try:
        ctor(b'%(x)b') % {b'y': b'a'}
    except KeyError as error:
        print(error.__class__.__name__, error.args)
    print(ctor(b'%r') % NonAsciiRepr())
    print(ctor(b'%a') % NonAsciiRepr())

for receiver in [b'%s', B(b'%s'), bytearray(b'%s'), BA(b'%s')]:
    result = receiver.__mod__(b'a')
    print(type(receiver).__name__, type(result).__name__, result, result is receiver)
for typ, receiver in [(bytes, b'%s'), (bytes, B(b'%s')), (bytearray, bytearray(b'%s')), (bytearray, BA(b'%s'))]:
    result = typ.__mod__(receiver, b'a')
    print('unbound', typ.__name__, type(receiver).__name__, type(result).__name__, result)
print('__mod__' in dir(bytes), '__mod__' in dir(bytearray), '__mod__' in dir(B), '__mod__' in dir(BA))
print(bytearray.__mod__(BA(b'%s'), b'a'))"#,
    });
}

#[test]
fn cpython_bytes_hex_separator_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest hex/fromhex separator public output subset",
        name: "bytes-hex-separator",
        source: r#"class Indexable:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value

for ctor in [bytes, bytearray]:
    three = ctor(b'\xb9\x01\xef')
    print(ctor.__name__, repr(three.hex(b'\x00')), repr(three.hex('\x00')))
    print(ctor.__name__, repr(three.hex(b'\x7f')), repr(three.hex('\x7f')), three.hex(b'$'))
    print(ctor.__name__, three.hex(':', 3), three.hex(':', -3), three.hex(':', 2**31 - 1), three.hex(':', -(2**31 - 1)), three.hex(':', -2**31))
    print(ctor.__name__, three.hex(':', Indexable(2)), three.hex(':', True), three.hex(':', False))
    for n in [2**31, -2**31 - 1, 2**1000, -2**1000]:
        try:
            three.hex(':', n)
        except OverflowError as error:
            print(ctor.__name__, 'overflow', n.bit_length(), error.__class__.__name__)
    six = ctor(x * 3 for x in range(1, 7))
    print(ctor.__name__, six.hex(':', 5), six.hex(b'@', -5), six.hex(':', -6), six.hex(' ', -95))

five = bytes(range(90, 95))
print('five', five.hex())
value = b'{s\005\000\000\000worldi\002\000\000\000s\005\000\000\000helloi\001\000\000\0000'
print('long', value.hex('.', 8))
boundary_bytes = bytes([0x09, 0x0a, 0x90, 0x99, 0x9a, 0xa0, 0xa9, 0xaa, 0x00, 0xff])
full = bytes(range(65)).hex()
for ctor in [bytes, bytearray]:
    ok = True
    for length in (14, 15, 16, 17, 31, 32, 33, 64, 65):
        data = ctor(bytes(range(length)))
        if data.hex() != full[:length * 2]:
            ok = False
    boundary = ctor(boundary_bytes)
    print(ctor.__name__, ok, boundary.hex(), (boundary * 2).hex() == '090a90999aa0a9aa00ff' * 2)"#,
    });
}

#[test]
fn cpython_bytes_fromhex_string_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_fromhex stable string-input public subset",
        name: "bytes-fromhex-string",
        source: r#"print(bytes.fromhex(''), bytearray.fromhex(''))
print(bytes.fromhex('1a2B30'))
print(bytearray.fromhex('  1A 2B  30   '))
print(bytes.fromhex(' 1A\n2B\t30\v'))
print(bytes.fromhex('0000'))
for ctor in [bytes, bytearray]:
    for label, source in [
        ('odd', 'a'),
        ('letters', 'rt'),
        ('split-pair', '1a b cd'),
        ('nul', '\x00'),
        ('next-line', '\u0085'),
        ('nbsp', '\u00a0'),
    ]:
        try:
            ctor.fromhex(source)
        except ValueError as error:
            print(ctor.__name__, label, error.__class__.__name__)
    for source in [1, ()]:
        try:
            ctor.fromhex(source)
        except TypeError as error:
            print(ctor.__name__, type(source).__name__, error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytes_repeat_id_preserving_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BytesTest::test_repeat_id_preserving",
        name: "bytes-repeat-id-preserving",
        source: r#"a = b'123abc1@'
b = b'456zyx-+'
print(id(a) == id(a), id(a) != id(b), id(a) != id(a * -4), id(a) != id(a * 0))
print(id(a) == id(a * 1), id(a) == id(1 * a), id(a) == id(a * True), id(a) != id(a * 2))
print(b'' is bytes(), id(b'') == id(bytes()))
class SubBytes(bytes):
    pass
s = SubBytes(b'qwerty()')
print(id(s) == id(s), id(s) != id(s * -4), id(s) != id(s * 0))
print(id(s) != id(s * 1), id(s) != id(1 * s), id(s) != id(s * True), id(s) != id(s * 2))
print(type(s * 1).__name__, s * 1 == b'qwerty()')"#,
    });
}

#[test]
fn cpython_bytearray_inplace_concat_repeat_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_iconcat, ::test_irepeat, and ::test_irepeat_1char",
        name: "bytearray-inplace-concat-repeat",
        source: r#"class I:
    def __index__(self):
        return 2
b = bytearray(b'abc')
b1 = b
b += b'def'
print(b, b == b1, b is b1)
b += bytearray(b'xyz')
print(b, b == b1, b is b1)
b += memoryview(b'!')
print(b, b is b1)
try:
    b += ''
except TypeError as error:
    print(error.__class__.__name__)
b = bytearray(b'abc')
b1 = b
b *= 3
print(b, b == b1, b is b1)
b = bytearray(b'x')
b1 = b
b *= 100
print(len(b), b[:5], b is b1)
for count in [0, -1, False, True]:
    b = bytearray(b'ab')
    alias = b
    b *= count
    print(count, b, b is alias)
for value in [b'b', bytearray(b'b'), memoryview(b'b')]:
    b = bytearray(b'a')
    result = b.__iadd__(value)
    print(b, result is b)
b = bytearray(b'a')
result = b.__imul__(3)
print(b, result is b)
b = bytearray(b'a')
result = b.__imul__(I())
print(b, result is b)
for expr in [lambda: bytearray(b'a').__iadd__('b'), lambda: bytearray(b'a').__iadd__([98]), lambda: bytearray(b'a').__imul__('3'), lambda: bytearray(b'a').__imul__(None), lambda: bytearray(b'a').__iadd__(), lambda: bytearray(b'a').__imul__()]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)
print('__iadd__' in dir(bytearray), '__imul__' in dir(bytearray))"#,
    });
}

#[test]
fn cpython_bytearray_extended_slice_assignment_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_extended_set_del_slice and ::test_setslice_trap",
        name: "bytearray-extended-slice-assignment",
        source: r#"import sys
indices = (0, None, 1, 3, 19, 300, 1<<333, sys.maxsize, -1, -2, -31, -300)
ok = True
for start in indices:
    for stop in indices:
        for step in indices[1:]:
            L = list(range(255))
            b = bytearray(L)
            data = L[start:stop:step]
            data.reverse()
            L[start:stop:step] = data
            b[start:stop:step] = data
            if b != bytearray(L):
                ok = False
            del L[start:stop:step]
            del b[start:stop:step]
            if b != bytearray(L):
                ok = False
print(ok)
for rhs in [(65, 66), range(65, 68), [65, True, False], [65, 'x'], [256], [-1]]:
    b = bytearray(b'abc')
    try:
        b[1:2] = rhs
        print(type(rhs).__name__, b)
    except (TypeError, ValueError) as error:
        print(type(rhs).__name__, error.__class__.__name__)
b = bytearray(b'abcdef')
print(b.__setitem__(slice(1, 3), [88, 89]), b)
print(b.__delitem__(slice(None, None, 2)), b)
b = bytearray(range(10))
b[8:] = b
print(len(b), b[:5], b[8:13])
print('__setitem__' in dir(bytearray), '__delitem__' in dir(bytearray))"#,
    });
}

#[test]
fn cpython_bytearray_mutation_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest public mutation methods subset",
        name: "bytearray-mutation-methods",
        source: r#"b = bytearray(b'abc')
alias = b
print(b.append(ord('d')), b, alias)
print(b.extend(b'ef'), b)
print(b.extend(bytearray(b'gh')), b)
print(b.extend(memoryview(b'ij')), b)
print(b.extend([75, 76]), b)
print(b.insert(0, 65), b)
print(b.insert(2, 66), b)
print(b.insert(-100, 67), b)
print(b.insert(100, 90), b)
print(b.pop(), b.pop(0), b.pop(-1), b)
print(b.remove(66), b)
print(b.reverse(), b)
copy = b.copy()
print(copy, copy == b, copy is b)
print(b.clear(), b, copy)
for expr in [lambda: bytearray(b'a').append(), lambda: bytearray(b'a').append(1, 2), lambda: bytearray(b'a').append('x'), lambda: bytearray(b'a').append(256), lambda: bytearray(b'a').extend(1), lambda: bytearray(b'a').insert(0), lambda: bytearray(b'a').insert(0, 'x'), lambda: bytearray(b'a').pop(9), lambda: bytearray().pop(), lambda: bytearray(b'a').remove(98), lambda: bytearray(b'a').remove('a'), lambda: bytearray(b'a').reverse(1), lambda: bytearray(b'a').clear(1), lambda: bytearray(b'a').copy(1)]:
    try:
        expr()
    except (TypeError, ValueError, IndexError) as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_bytearray_extend_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_extend",
        name: "bytearray-extend",
        source: r#"orig = b'hello'
a = bytearray(orig)
print(a.extend(a), a == orig + orig, a[5:])
a = bytearray(b'')
a.extend(map(int, orig * 25))
a.extend(int(x) for x in orig * 25)
print(a == orig * 50, a[-5:])
a = bytearray(b'')
a.extend(iter(map(int, orig * 50)))
print(a == orig * 50, a[-5:])
a = bytearray(b'')
a.extend(list(map(int, orig * 50)))
print(a == orig * 50, a[-5:])
a = bytearray(b'')
for source in [[0, 1, 2, 256], [0, 1, 2, -1]]:
    try:
        a.extend(source)
    except ValueError as error:
        print(error.__class__.__name__, len(a))
class Indexable:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
a = bytearray(b'')
print(a.extend([Indexable(ord('a'))]), a)
a = bytearray(b'abc')
for expr in [lambda: a.extend('def'), lambda: a.extend(1.0)]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__, type(error).__name__ == 'TypeError')"#,
    });
}

#[test]
fn cpython_bytearray_alloc_and_subclass_mutation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_alloc, ::test_init_alloc, and public subclass mutation slice",
        name: "bytearray-alloc-and-subclass-mutation",
        source: r#"b = bytearray()
print(b.__alloc__(), b.__alloc__() >= len(b))
ok = True
for _ in range(5):
    b += b'x'
    ok = ok and b.__alloc__() > len(b)
print(len(b), ok, b.__alloc__() > len(b))
b = bytearray()
checks = []
def g():
    for i in range(1, 5):
        yield i
        checks.append((list(b), len(b), b.__alloc__() > len(b)))
print(b.__init__(g()), list(b), len(b), b.__alloc__() > len(b), checks)
class BA(bytearray):
    pass
ba = BA(b'abc')
print(ba.append(100), type(ba).__name__, ba)
print(ba.extend(memoryview(b'ef')), ba)
print(ba.insert(1, 90), ba)
print(ba.pop(), ba)
print(ba.remove(ord('Z')), ba)
print(ba.reverse(), ba)
copy = ba.copy()
print(type(copy).__name__, copy == ba, copy is ba)
ba = BA(b'ab')
result = ba.__iadd__(b'c')
print(type(result).__name__, result is ba, ba)
result = ba.__imul__(2)
print(type(result).__name__, result is ba, ba)
print(ba.__setitem__(slice(1, 4), b'XYZ'), ba)
print(ba.__delitem__(slice(None, None, 2)), ba)
print(BA(b'abc').__alloc__() > len(BA(b'abc')))
print('__alloc__' in dir(bytearray), '__alloc__' in dir(BA), '__alloc__' in dir(BA()))"#,
    });
}

#[test]
fn cpython_bytearray_pep3137_returns_new_copy_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BytearrayPEP3137Test::test_returns_new_copy and AssortedBytesTest::test_return_self",
        name: "bytearray-pep3137-returns-new-copy-named",
        source: r#"val = bytearray(b'1234')
for methname in ['zfill', 'rjust', 'ljust', 'center']:
    newval = getattr(val, methname)(3)
    print(methname, val == newval, val is newval)
checks = [
    ('split', lambda: val.split()[0]),
    ('rsplit', lambda: val.rsplit()[0]),
    ('partition', lambda: val.partition(b'.')[0]),
    ('rpartition', lambda: val.rpartition(b'.')[2]),
    ('splitlines', lambda: val.splitlines()[0]),
    ('replace', lambda: val.replace(b'', b'')),
]
for name, maker in checks:
    newval = maker()
    print(name, val == newval, val is newval)
sep = bytearray(b'')
newval = sep.join([val])
print('join', val == newval, val is newval)"#,
    });
}

#[test]
fn cpython_bytearray_nonmutating_copy_buffers_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_copied and ::test_partition_bytearray_doesnt_share_nullstring",
        name: "bytearray-nonmutating-copy-buffers",
        source: r#"b = bytearray(b'abc')
r = b.replace(b'abc', b'cde', 0)
print(r, r is b)
r += b'!'
print(b, r)
t = bytearray([i for i in range(256)])
x = bytearray(b'')
y = x.translate(t)
print(y, y is x)
y += b'!'
print(x, y)
a, b, c = bytearray(b'x').partition(b'y')
print(a, b, c, b is c)
b += b'!'
print(b, c)
a, b, c = bytearray(b'x').partition(b'y')
print(b, c)
b, c, a = bytearray(b'x').rpartition(b'y')
print(a, b, c, b is c)
b += b'!'
print(b, c)
c, b, a = bytearray(b'x').rpartition(b'y')
print(b, c)"#,
    });
}

#[test]
fn cpython_builtin_bytearray_translate_extend_errors_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_builtin.py::BuiltinTest::test_bytearray_translate and ::test_bytearray_extend_error",
        name: "builtin-bytearray-translate-extend-errors-named",
        source: r#"x = bytearray(b'abc')
for expr in [lambda: x.translate(b'1', 1), lambda: x.translate(b'1' * 256, 1)]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)
array = bytearray()
bad_iter = map(int, 'X')
try:
    array.extend(bad_iter)
except ValueError as error:
    print(error.__class__.__name__, array)"#,
    });
}

#[test]
fn cpython_bytearray_regexps_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_regexps",
        name: "bytearray-regexps-findall-named",
        source: r#"import re
def by(text):
    return bytearray(map(ord, text))
for source in [by('Hello, world'), b'Hi, Bob_2!', memoryview(b'xy 99')]:
    matches = re.findall(br'\w+', source)
    print(type(source).__name__, matches, [type(item).__name__ for item in matches])"#,
    });
}

#[test]
fn cpython_bytearray_subclass_init_override_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArraySubclassTest::test_init_override",
        name: "bytearray-subclass-init-override-named",
        source: r#"class Sub(bytearray):
    def __init__(self, newarg=1, *args, **kwargs):
        print('init', newarg, args, kwargs.get('source', None))
        bytearray.__init__(self, *args, **kwargs)
for factory in [lambda: Sub(4, b'abcd'), lambda: Sub(4, source=b'abcd'), lambda: Sub(newarg=4, source=b'abcd')]:
    value = factory()
    print(type(value).__name__, value == b'abcd', bytes(value), isinstance(value, bytearray))
class Empty(bytearray):
    def __init__(self, value):
        print('empty init', value)
empty = Empty(b'abc')
print(type(empty).__name__, len(empty), bytes(empty))"#,
    });
}

#[test]
fn cpython_bytes_bytearray_subclass_copy_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::SubclassTest::test_copy",
        name: "bytes-bytearray-subclass-copy",
        source: r#"import copy
class B(bytes):
    pass
class BA(bytearray):
    pass
for T in [B, BA]:
    a = T(b'abcd')
    a.x = 10
    a.z = T(b'efgh')
    for label, method in [('copy', copy.copy), ('deepcopy', copy.deepcopy)]:
        b = method(a)
        print(T.__name__, label, type(b).__name__, b == a, b is a, getattr(b, 'x', 'none'), type(getattr(b, 'z', None)).__name__, getattr(b, 'z', b'') == T(b'efgh'), hasattr(b, 'y'))"#,
    });
}

#[test]
fn cpython_bytes_bytearray_subclass_pickle_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::SubclassTest::test_pickle public round-trip subset",
        name: "bytes-bytearray-subclass-pickle-roundtrip-named",
        source: r#"import pickle
class B(bytes):
    pass
class BA(bytearray):
    pass
for T in [B, BA]:
    checked = 0
    nested = 0
    independent = 0
    for proto in range(pickle.HIGHEST_PROTOCOL + 1):
        a = T(b'abcd')
        a.x = 10
        a.z = T(b'efgh')
        b = pickle.loads(pickle.dumps(a, proto))
        if type(b) is T and b == a and b is not a and b.x == 10 and not hasattr(b, 'y'):
            checked += 1
        if type(b.z) is T and b.z == T(b'efgh'):
            nested += 1
        if isinstance(b, bytearray):
            b.append(ord('!'))
            if bytes(a) == b'abcd' and bytes(b) == b'abcd!':
                independent += 1
        elif bytes(b) == b'abcd':
            independent += 1
    print(T.__name__, checked, nested, independent, pickle.HIGHEST_PROTOCOL + 1)"#,
    });
}

#[test]
fn cpython_bytes_bytearray_subclass_basics_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_custom and module-level bytes/bytearray subclass definitions public subset",
        name: "bytes-bytearray-subclass-basics",
        source: r#"class B(bytes):
    pass
class BA(bytearray):
    pass
b = B(b'ab')
ba = BA(b'cd')
print(isinstance(b, bytes), issubclass(B, bytes), bytes(b), len(b), bool(B(b'')), bool(b))
print(isinstance(ba, bytearray), issubclass(BA, bytearray), bytes(ba), len(ba), bool(BA()), bool(ba))"#,
    });
}

#[test]
fn cpython_bytes_bytearray_subclass_ops_and_join_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::SubclassTest::test_basic and ::test_join as applied to BytesSubclassTest and ByteArraySubclassTest",
        name: "bytes-bytearray-subclass-ops-and-join",
        source: r#"class B(bytes):
    pass
class BA(bytearray):
    pass
for T, base in [(B, bytes), (BA, bytearray)]:
    a = b'abcd'
    c = b'efgh'
    ta = T(a)
    tc = T(c)
    print(T.__name__, issubclass(T, base), isinstance(T(), base))
    print(ta == ta, not (ta == tc), ta < tc, ta <= tc, tc >= ta, tc > ta, ta is a)
    print(a + c == ta + tc, a + c == a + tc, a + c == ta + c)
    repeated = ta * 5
    print(a * 5 == repeated, type(repeated).__name__)
    s2 = base().join([ta])
    print(type(s2).__name__, s2 == ta, s2 is ta)
    s3 = ta.join([b'abcd'])
    print(type(s3).__name__, s3 == b'abcd')
print(hasattr(B(b''), 'join'), hasattr(BA(b''), 'join'), hasattr(bytes, 'join'), hasattr(bytearray, 'join'))"#,
    });
}

#[test]
fn cpython_bytes_bytearray_subclass_fromhex_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::SubclassTest::test_fromhex as applied to BytesSubclassTest and ByteArraySubclassTest",
        name: "bytes-bytearray-subclass-fromhex",
        source: r#"class B(bytes):
    pass
class BA(bytearray):
    pass
for T in [B, BA]:
    x = T.fromhex('1a2B30')
    print(T.__name__, type(x).__name__, x == b'\x1a+0', isinstance(x, T), getattr(x, 'foo', 'none'))
class B1(bytes):
    def __new__(cls, value):
        me = bytes.__new__(cls, value)
        me.foo = 'bar'
        return me
class BA1(bytearray):
    def __new__(cls, value):
        me = bytearray.__new__(cls, value)
        me.foo = 'bar'
        return me
for T in [B1, BA1]:
    x = T.fromhex('1a2B30')
    print(T.__name__, type(x).__name__, x == b'\x1a+0', getattr(x, 'foo', 'none'))
class B2(bytes):
    def __init__(self, value):
        self.foo = 'bar'
class BA2(bytearray):
    def __init__(self, *args):
        bytearray.__init__(self, *args)
        self.foo = 'bar'
for T in [B2, BA2]:
    x = T.fromhex('1a2B30')
    print(T.__name__, type(x).__name__, x == b'\x1a+0', getattr(x, 'foo', 'none'))
y = bytearray.__new__(BA, b'abc', spam='ignored')
print(type(y).__name__, y, y == b'abc')
print(bytearray.__init__(y, b'abc'), y, y == b'abc')
print(hasattr(bytes, '__new__'), hasattr(bytearray, '__new__'), hasattr(bytearray, '__init__'))"#,
    });
}

#[test]
fn cpython_bytes_dunder_bytes_dispatch_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BytesTest::test_bytes_blocking and BaseBytesTest::test_custom dispatch subset",
        name: "bytes-dunder-bytes-dispatch",
        source: r#"class B(bytes):
    pass
class WithBytes:
    def __init__(self, value):
        self.value = value
    def __bytes__(self):
        return self.value
class IndexWithBytes:
    def __bytes__(self):
        return b'a'
    def __index__(self):
        return 42
class Iterable:
    def __iter__(self):
        return iter([0, 1, 2])
class IterableBlocked:
    __bytes__ = None
    def __iter__(self):
        return iter([0, 1, 2])
class IntBlocked(int):
    __bytes__ = None
class BytesSubclassBlocked(bytes):
    __bytes__ = None
class BufferBlocked(bytearray):
    __bytes__ = None

print(bytes(WithBytes(b'abc')))
result = bytes(WithBytes(B(b'abc')))
print(type(result).__name__, result == b'abc')
print(bytes(IndexWithBytes()))
print(bytes(Iterable()))
print(bytes(3), bytes(b'ab'), bytes(bytearray(b'ab')))
for expr in [
    lambda: bytes(WithBytes(bytearray(b'abc'))),
    lambda: bytes(WithBytes('abc')),
    lambda: bytes(WithBytes(None)),
    lambda: bytes(IterableBlocked()),
    lambda: bytes(IntBlocked(3)),
    lambda: bytes(BytesSubclassBlocked(b'ab')),
    lambda: bytes(BufferBlocked(b'ab')),
    lambda: bytearray(WithBytes(b'abc')),
]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)

class BytesSubclass(bytes):
    pass
class OtherBytesSubclass(bytes):
    pass
class StrWithBytes(str):
    def __new__(cls, value):
        self = str.__new__(cls, '\u20ac')
        self.value = value
        return self
    def __bytes__(self):
        return self.value
class BytesWithBytes(bytes):
    def __new__(cls, value):
        self = bytes.__new__(cls, b'\xa4')
        self.value = value
        return self
    def __bytes__(self):
        return self.value
samples = [
    ('str-bytes', lambda: bytes(StrWithBytes(b'abc')), b'abc'),
    ('str-encoding', lambda: bytes(StrWithBytes(b'abc'), 'iso8859-15'), b'\xa4'),
    ('str-subbytes', lambda: bytes(StrWithBytes(BytesSubclass(b'abc'))), b'abc'),
    ('sub-str-bytes', lambda: BytesSubclass(StrWithBytes(b'abc')), b'abc'),
    ('sub-str-encoding', lambda: BytesSubclass(StrWithBytes(b'abc'), 'iso8859-15'), b'\xa4'),
    ('sub-str-subbytes', lambda: BytesSubclass(StrWithBytes(BytesSubclass(b'abc'))), b'abc'),
    ('sub-str-other', lambda: BytesSubclass(StrWithBytes(OtherBytesSubclass(b'abc'))), b'abc'),
    ('byteswithbytes', lambda: bytes(BytesWithBytes(b'abc')), b'abc'),
    ('sub-byteswithbytes', lambda: BytesSubclass(BytesWithBytes(b'abc')), b'abc'),
    ('byteswithbytes-sub', lambda: bytes(BytesWithBytes(BytesSubclass(b'abc'))), b'abc'),
    ('sub-byteswithbytes-sub', lambda: BytesSubclass(BytesWithBytes(BytesSubclass(b'abc'))), b'abc'),
    ('sub-byteswithbytes-other', lambda: BytesSubclass(BytesWithBytes(OtherBytesSubclass(b'abc'))), b'abc'),
]
for label, callback, expected in samples:
    result = callback()
    print(label, type(result).__name__, result == expected, result)
plain = str.__new__(str, 'plain')
custom = str.__new__(StrWithBytes, 'stored')
print(type(plain).__name__, plain)
print(type(custom).__name__, str(custom), hasattr(custom, 'value'))"#,
    });
}

#[test]
fn cpython_bytes_buffer_constructor_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::BaseBytesTest::test_from_buffer portable public subset",
        name: "bytes-buffer-constructor",
        source: r#"import array

class B(bytes):
    def __index__(self):
        raise TypeError

for ctor in [bytes, bytearray]:
    for source in [
        b'\x01\x02\x03',
        bytearray(b'\x01\x02\x03'),
        memoryview(b'\x01\x02\x03'),
        B(b'foobar'),
    ]:
        value = ctor(source)
        print(ctor.__name__, type(source).__name__, type(value).__name__, value == source, value)
    source = array.array('B', [1, 2, 3])
    value = ctor(source)
    print(ctor.__name__, type(source).__name__, type(value).__name__, value == b'\x01\x02\x03', value)

arr = array.array('B', b'ab')
print(bytes(arr), bytearray(arr))
print(b'abc'.find(array.array('B', b'b')))
print(b'a' + array.array('B', b'b'))
ba = bytearray(b'x')
ba += arr
print(ba)
ba[1:] = array.array('B', b'YZ')
print(ba)"#,
    });
}

#[test]
fn cpython_bytearray_join_custom_iterator_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_builtin.py::BuiltinTest::test_bytearray_join_with_custom_iterator",
        name: "bytearray-join-custom-iterator",
        source: r#"array = bytearray(b',')
def iterator():
    yield b'A'
    yield b'B'
print(array.join(iterator()))"#,
    });
}

#[test]
fn cpython_bytearray_iterator_length_hint_and_repeat_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_iterator_length_hint / ::test_repeat_after_setslice",
        name: "bytearray-iterator-length-hint-and-repeat-regressions",
        source: r#"ba = bytearray(b'ab')
it = iter(ba)
print(next(it), it.__length_hint__())
ba.clear()
print(it.__length_hint__(), list(it))
b = bytearray(b'abc')
b[:2] = b'x'
b1 = b * 1
b3 = b * 3
print(b, b1, b1 == b'xc', b1 == b)
print(b3)"#,
    });
}

#[test]
fn cpython_bytearray_exhausted_iterator_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_exhausted_iterator",
        name: "bytearray-exhausted-iterator",
        source: r#"a = bytearray([1, 2, 3])
exhit = iter(a)
empit = iter(a)
for x in exhit:
    next(empit)
a.append(9)
print(list(exhit), list(empit), a)
exhit = iter(bytearray([1, 2, 3]))
seen = []
for _ in exhit:
    seen.append(next(exhit, 1))
print(seen)"#,
    });
}

#[test]
fn cpython_bytearray_mutating_index_conversion_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_bytes.py::ByteArrayTest::test_mutating_index public __index__ conversion subset",
        name: "bytearray-mutating-index-conversion",
        source: r#"class Indexable:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value

b = bytearray()
print(b.append(Indexable(ord('A'))), b)
b = bytearray(b'xy')
print(b.insert(1, Indexable(ord('A'))), b)"#,
    });
}

#[test]
fn cpython_memoryview_minimal_runtime_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py constructor, equality, hash, and argument errors public subset",
        name: "memoryview-minimal-runtime",
        source: r#"for source in [b'x', bytearray(b'x'), memoryview(b'x')]:
    m = memoryview(source)
    print(type(source).__name__, bool(m), bool(memoryview(object=source)), m.tolist())
print(list(memoryview(b'abc')))
print(memoryview(b'abcdef') == b'abcdef', memoryview(bytearray(b'abcdef')) == bytearray(b'abcdef'))
print(memoryview(b'abcde') == b'abcdef', memoryview(b'abcdef') != b'abcde')
print(hash(memoryview(b'abcdef')) == hash(b'abcdef'))
try:
    hash(memoryview(bytearray(b'abcdef')))
except Exception as error:
    print(error.__class__.__name__)

def show(label, callback):
    try:
        callback()
    except Exception as error:
        print(label, error.__class__.__name__)
    else:
        print(label, 'OK')

show('missing', lambda: memoryview())
show('two-pos', lambda: memoryview(b'x', b'y'))
show('bad-kw', lambda: memoryview(argument=b'x'))
show('two-kw', lambda: memoryview(object=b'x', argument=True))
show('pos-object-kw', lambda: memoryview(b'x', object=b'y'))
show('pos-bad-kw', lambda: memoryview(b'x', argument=True))"#,
    });
}

#[test]
fn cpython_memoryview_methods_release_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py public methods, attributes, context manager, release, and toreadonly subset",
        name: "memoryview-methods-release",
        source: r#"for source in [b'abcdef', bytearray(b'abcdef'), memoryview(b'abcdef')[1:5]]:
    m = memoryview(source)
    print(m.tobytes(), m.tolist(), m.hex(), m.hex(':', 2))
    print(m.format, m.itemsize, m.ndim, m.shape, m.strides, m.suboffsets, m.readonly, m.nbytes)
    print(m.toreadonly().readonly, m.toreadonly().tolist() == m.tolist())

for expr in [
    lambda: memoryview(b'abc').tobytes(1),
    lambda: memoryview(b'abc').tolist(1),
]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)

m = memoryview(b'abcdef')
print(list(reversed(m)), list(m[::-1]))
with m as cm:
    print(cm is m)
for expr in [
    lambda: bytes(m),
    lambda: m.tobytes(),
    lambda: m.tolist(),
    lambda: m[0],
    lambda: len(m),
    lambda: m.format,
    lambda: m.itemsize,
    lambda: m.ndim,
    lambda: m.readonly,
    lambda: m.shape,
    lambda: m.strides,
    lambda: hash(m),
]:
    try:
        expr()
    except ValueError as error:
        print(error.__class__.__name__, 'released' in str(error))
print('released memory' in str(m), 'released memory' in repr(m))
print(m == m, m != memoryview(b'abcdef'), m != b'abcdef')
m.release()
print('released memory' in str(m))
m = memoryview(b'abcdef')
with m:
    m.release()
try:
    with m:
        pass
except ValueError as error:
    print(error.__class__.__name__, 'released' in str(error))
base = memoryview(b'abcdef')
readonly = base.toreadonly()
readonly.release()
print(base.tolist())"#,
    });
}

#[test]
fn cpython_memoryview_count_index_diff_subset() {
    let oracle_probe = run_cpython("print(hasattr(memoryview(b'abc'), 'count'))")
        .expect("failed to run CPython memoryview.count capability probe");
    let oracle_stdout =
        String::from_utf8(oracle_probe.stdout).expect("CPython capability probe emitted non-UTF-8");
    if oracle_stdout.trim() != "True" {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py memoryview count/index public subset",
        name: "memoryview-count-index",
        source: r#"for source in [b'abcdef', bytearray(b'abcdef'), memoryview(b'abcdef')[1:5]]:
    m = memoryview(source)
    print(m.count(ord('a')), m.count(ord('c')), m.count(ord('x')))
    try:
        print(m.index(ord('c')), m.index(ord('c'), -10, 99))
    except ValueError as error:
        print(error.__class__.__name__)

try:
    memoryview(b'abcdef').index(ord('x'))
except ValueError as error:
    print(error.__class__.__name__)

for expr in [
    lambda: memoryview(b'abc').count(),
    lambda: memoryview(b'abc').index(),
    lambda: memoryview(b'abc').index(97, 0, 1, 2),
]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_memoryview_writable_setitem_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py writable bytearray-backed setitem public subset",
        name: "memoryview-writable-setitem",
        source: r#"base = bytearray(b'abcdef')
alias = base
alias[0] = ord('z')
print(base)
base[0] = ord('a')
m = memoryview(base)
m[0] = ord('1')
print(base, m.tolist())
m[0:1] = bytearray(b'0')
print(base)
m[1:3] = bytearray(b'12')
print(base)
m[1:1] = bytearray(b'')
print(base)
m[:] = bytearray(b'abcdef')
print(base)
m[0:3] = m[2:5]
print(base)
m[:] = bytearray(b'abcdef')
m[2:5] = m[0:3]
print(base)

readonly = memoryview(b'abcdef')
for expr in [
    lambda: readonly.__setitem__(0, ord('1')),
    lambda: readonly.__setitem__(0, b'1'),
    lambda: readonly.__setitem__(0, memoryview(b'1')),
]:
    try:
        expr()
    except TypeError as error:
        print(error.__class__.__name__)
for source in [memoryview(b'abcdef'), memoryview(bytearray(b'abcdef'))]:
    for expr in [
        lambda source=source: source.__delitem__(1),
        lambda source=source: source.__delitem__(slice(1, 4)),
    ]:
        try:
            expr()
        except TypeError as error:
            print(error.__class__.__name__)
base = bytearray(b'abcdef')
m = memoryview(base)
for expr in [
    lambda: m.__setitem__(6, b'a'),
    lambda: m.__setitem__(-7, b'a'),
    lambda: m.__setitem__(0, b''),
    lambda: m.__setitem__(0, b'ab'),
    lambda: m.__setitem__(slice(1, 1), b'a'),
    lambda: m.__setitem__(slice(0, 2), b'a'),
]:
    try:
        expr()
    except (IndexError, ValueError, TypeError) as error:
        print(error.__class__.__name__)
print(base)"#,
    });
}

#[test]
fn cpython_memoryview_tuple_key_setitem_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py tuple-key getitem/setitem public subset",
        name: "memoryview-tuple-key-setitem",
        source: r#"base = bytearray(b'abcdef')
m = memoryview(base)
for label, key in [
    ('get-empty', ()),
    ('get-scalar', (0,)),
    ('get-bool', (True,)),
    ('get-oob', (99,)),
    ('get-one-slice', (slice(0, 1, 1),)),
    ('get-two-slice', (slice(0, 1, 1), slice(0, 1, 1))),
    ('get-slice-int', (slice(0, 1, 1), 0)),
    ('get-int-slice', (0, slice(0, 1, 1))),
    ('get-two-int', (0, 0)),
    ('get-float', (0.0,)),
]:
    try:
        result = m.__getitem__(key)
        print(label, result)
    except (TypeError, IndexError, NotImplementedError) as error:
        print(label, error.__class__.__name__)
base = bytearray(b'abcdef')
m = memoryview(base)
for label, key, value in [
    ('set-scalar', (0,), 90),
    ('set-empty', (), 90),
    ('set-one-slice', (slice(0, 1, 1),), b'Z'),
    ('set-two-slice', (slice(0, 1, 1), slice(0, 1, 1)), b'Z'),
    ('set-slice-int', (slice(0, 1, 1), 0), b'Z'),
    ('set-int-slice', (0, slice(0, 1, 1)), b'Z'),
    ('set-two-int', (0, 0), 90),
    ('set-float', (0.0,), 90),
    ('set-bytes-scalar', (0,), b'Z'),
]:
    try:
        result = m.__setitem__(key, value)
        print(label, result, base)
    except (TypeError, IndexError, NotImplementedError) as error:
        print(label, error.__class__.__name__, base)"#,
    });
}

#[test]
fn cpython_memoryview_slice_and_attributes_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py one-dimensional slice reference and public buffer attributes subset",
        name: "memoryview-slice-and-attributes",
        source: r#"base = bytearray(b'XabcdefY')
m = memoryview(base)[1:7]
print(m.tobytes(), m.tolist(), m.readonly, m.nbytes)
m[0] = ord('1')
print(base, m.tobytes())
base[2] = ord('2')
print(base, m.tobytes())
m[2:4] = b'34'
print(base, m.tobytes())
base = bytearray(b'XabcdefY')
s = memoryview(base)[:7][1:]
s[5] = ord('!')
print(base, s.tobytes())
rev_base = bytearray(b'abcdef')
rev = memoryview(rev_base)[::-1]
print(rev.tolist(), rev.tobytes())
rev[0] = ord('Z')
rev[5] = ord('A')
print(rev_base, rev.tobytes())
readonly = memoryview(b'XabcdefY')[1:7]
try:
    readonly[0] = ord('x')
except TypeError as error:
    print(error.__class__.__name__)

base = bytearray(b'ab')
view = memoryview(base)
copy = memoryview(view)
readonly = view.toreadonly()
print(view.obj is base, copy.obj is base, readonly.obj is base, readonly.readonly)
for name, m in [
    ('full', view),
    ('empty', view[0:0]),
    ('empty-step', view[0:0:2]),
    ('empty-neg', view[0:0:-1]),
    ('skip', view[::2]),
    ('reverse', view[::-1]),
    ('one-reverse', view[:0:-1]),
]:
    print(name, m.tolist(), m.strides, m.c_contiguous, m.f_contiguous, m.contiguous, m.obj is base)
bytes_view = memoryview(b'abcdef')[1:5]
print(bytes_view.obj == b'abcdef', bytes_view.obj, bytes_view.strides, bytes_view.c_contiguous)
released = memoryview(base)
released.release()
for expr in [
    lambda: released.obj,
    lambda: released.c_contiguous,
    lambda: released.f_contiguous,
    lambda: released.contiguous,
]:
    try:
        expr()
    except ValueError as error:
        print(error.__class__.__name__, 'released' in str(error))"#,
    });
}

#[test]
fn cpython_memoryview_cast_one_byte_format_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py one-byte cast public format subset",
        name: "memoryview-cast-one-byte-format",
        source: r#"print('cast' in dir(memoryview(b'')))
for fmt in ['B', 'b', 'c']:
    m = memoryview(b'abc').cast(fmt)
    print(fmt, m.format, m.itemsize, m.ndim, m.shape, m.strides, m.tolist(), m[0], type(m[0]).__name__)
print(memoryview(b'abc').cast(format='B').tolist())
print(memoryview(b'abc').cast('B', [3]).tolist())
print(memoryview(b'abc').cast('B', shape=(3,)).tolist())
base = bytearray(b'abc')
m = memoryview(base).cast('c')
m[0] = b'X'
m[1:2] = memoryview(b'Y').cast('c')
print(base, m.tolist(), list(m), list(reversed(m)))
print(b'Y' in m, ord('Y') in m, bytearray(b'Y') in m, memoryview(b'Y') in m, b'YZ' in m)
for expr in [lambda: m.__setitem__(0, 88), lambda: m.__setitem__(slice(0, 1), b'Z')]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)
print(memoryview(m).format, m[:].format, m.toreadonly().format)
try:
    memoryview(b'abcd')[::2].cast('B')
except TypeError as error:
    print(error.__class__.__name__, 'contiguous' in str(error))"#,
    });
}

#[test]
fn cpython_memoryview_hex_separator_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py memoryview hex separator public subset",
        name: "memoryview-hex-separator",
        source: r#"x = bytes(range(97, 102))
m2 = memoryview(x)[::-1]
print(m2.hex())
print(m2.hex(':'))
print(m2.hex(':', 2))
print(m2.hex(':', -2))
print(m2.hex(sep=':', bytes_per_sep=2))
print(m2.hex(sep=':', bytes_per_sep=-2))
for bytes_per_sep in [5, -5, 2147483647, -2147483647]:
    print(m2.hex(':', bytes_per_sep))
print(memoryview(b'0' * 12)[::-1].hex())"#,
    });
}

#[test]
fn cpython_memoryview_rejection_and_hash_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py copy, pickle, and hash public rejection/cache subset",
        name: "memoryview-rejection-and-hash",
        source: r#"import copy
import pickle

for source in [b'abc', bytearray(b'abc')]:
    try:
        copy.copy(memoryview(source))
    except TypeError as error:
        print('copy', error.__class__.__name__, 'memoryview' in str(error))

checked = 0
for source in [b'abc', bytearray(b'abc')]:
    for proto in range(pickle.HIGHEST_PROTOCOL + 1):
        try:
            pickle.dumps(memoryview(source), proto)
        except TypeError as error:
            if 'memoryview' in str(error):
                checked += 1
for obj in [[memoryview(b'abc')], {'view': memoryview(b'abc')}]:
    try:
        pickle.dumps(obj)
    except TypeError as error:
        print('pickle-container', error.__class__.__name__, 'memoryview' in str(error))
print('pickle-checked', checked)

m = memoryview(b'abcdef')
expected = hash(b'abcdef')
print(hash(m) == expected)
m.release()
print(hash(m) == expected)
m.release()
print(hash(m) == expected)
m = memoryview(b'abcdef')
m.release()
try:
    hash(m)
except ValueError as error:
    print(error.__class__.__name__, 'released' in str(error))
try:
    hash(memoryview(bytearray(b'abcdef')))
except ValueError as error:
    print(error.__class__.__name__, 'writable' in str(error))"#,
    });
}

#[test]
fn cpython_memoryview_hex_released_view_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py released-view hex public subset",
        name: "memoryview-hex-released-view",
        source: r#"m = memoryview(b'abc')
m.release()
try:
    m.hex()
except ValueError as error:
    print(error.__class__.__name__, 'released' in str(error))"#,
    });
}

#[test]
fn cpython_memoryview_release_during_index_read_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py release during __index__ public one-dimensional read subset",
        name: "memoryview-release-during-index-read",
        source: r#"size = 8
ba = None
def release():
    global m, ba
    m.release()
    ba = bytearray(size)
class MyIndex:
    def __index__(self):
        release()
        return 4

ba = None
m = memoryview(bytearray(b'\xff' * size))
try:
    m[MyIndex()]
except ValueError as error:
    print('getitem', error.__class__.__name__, 'released' in str(error))

ba = None
m = memoryview(bytearray(b'\xff' * size))
print('slice-stop', list(m[:MyIndex()]), ba is not None)

ba = None
m = memoryview(bytearray(b'\xff' * size))
print('slice-start', list(m[MyIndex():8]), ba is not None)

ba = None
m = memoryview(bytearray(b'\xff' * size))
try:
    m.__getitem__(MyIndex())
except ValueError as error:
    print('getitem-method', error.__class__.__name__, 'released' in str(error))

ba = None
m = memoryview(bytearray(b'\xff' * size))
print('slice-method', list(m.__getitem__(slice(None, MyIndex()))), ba is not None)
"#,
    });
}

#[test]
fn cpython_memoryview_bytesio_readinto_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py writable/read-only BytesIO.readinto public in-memory subset",
        name: "memoryview-bytesio-readinto",
        source: r#"import io
for target in [bytearray(b'abc'), memoryview(bytearray(b'abc'))]:
    bio = io.BytesIO(b'XYZW')
    n = bio.readinto(target)
    print(type(target).__name__, n, bytes(target))
for target in [b'abc', memoryview(b'abc')]:
    bio = io.BytesIO(b'XYZW')
    try:
        bio.readinto(target)
    except TypeError as error:
        print(type(target).__name__, error.__class__.__name__)
bio = io.BytesIO(b'XYZW')
ba = bytearray(b'abc')
print(bio.readinto(ba), ba, bio.readinto(ba), ba, bio.readinto(ba), ba)
for source in [None, b'ab', bytearray(b'ab'), memoryview(b'ab')]:
    bio = io.BytesIO() if source is None else io.BytesIO(source)
    target = bytearray(4)
    print(type(bio).__name__, bio.readinto(target), target)
for label, callback in [
    ('int', lambda: io.BytesIO(123)),
    ('two', lambda: io.BytesIO(b'a', b'b')),
    ('kw', lambda: io.BytesIO(initial_bytes=b'ab')),
    ('dup', lambda: io.BytesIO(b'a', initial_bytes=b'b')),
]:
    try:
        obj = callback()
        target = bytearray(3)
        print(label, 'ok', obj.readinto(target), target)
    except TypeError as error:
        print(label, error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_memoryview_weakref_live_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py live weakref construction public subset",
        name: "memoryview-weakref-live",
        source: r#"import weakref
for source in [b'abcdef', bytearray(b'abcdef')]:
    m = memoryview(source)
    seen = []
    def callback(wr, source=source):
        seen.append(source)
    refs = [weakref.ref(m), weakref.ref(m, callback), weakref.ref(m, None)]
    print(type(source).__name__, all(ref() is m for ref in refs), all(callable(ref) for ref in refs), all(isinstance(ref, weakref.ReferenceType) for ref in refs), len(seen))"#,
    });
}

#[test]
fn cpython_memoryview_array_b_buffer_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py array-backed public one-byte buffer behavior",
        name: "memoryview-array-b-buffer",
        source: r#"import array
arr = array.array('B', [97, 98, 99])
m = memoryview(arr)
print(m.format, m.itemsize, m.ndim, m.shape, m.strides, m.readonly, m.tolist(), m.tobytes(), m.obj is arr)
m[0] = ord('z')
print(arr, repr(arr), bytes(arr), m.tolist())
m[1:3] = b'XY'
print(arr, bytes(arr), m.tobytes())
sub = m[::2]
print(sub.tolist(), sub.strides, sub.obj is arr)
readonly = m.toreadonly()
print(readonly.readonly, readonly.obj is arr, readonly.tolist())"#,
    });
}

#[test]
fn cpython_memoryview_array_signed_byte_buffer_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_memoryview.py array-backed public signed-byte buffer behavior",
        name: "memoryview-array-signed-byte-buffer",
        source: r#"import array

def show(label, expr):
    try:
        value = expr()
        print(label, value)
    except Exception as error:
        print(label, error.__class__.__name__, str(error))

for source in [b'\xff\x00\x7f', bytearray(b'\xff\x00\x7f'), [-1, 0, 127], memoryview(array.array('b', [-1, 2])), memoryview(b'\xff')]:
    show('init-' + type(source).__name__, lambda source=source: (repr(array.array('b', source)), bytes(array.array('b', source)), list(array.array('b', source))))
arr = array.array('b', [-1, 0, 127])
m = memoryview(arr)
print('view', m.format, m.itemsize, m.ndim, m.shape, m.strides, m.readonly, m.tolist(), m.tobytes(), m.obj is arr, m[0])
m[0] = -2
print('set', repr(arr), m.tolist(), m.tobytes())
show('set-high', lambda: m.__setitem__(1, 128))
show('set-type', lambda: m.__setitem__(1, b'X'))
m[1:3] = memoryview(array.array('b', [3, -4]))
print('slice-ok', repr(arr), m.tolist(), m.tobytes())
for label, rhs in [('bytes', b'\x01\x02'), ('B-view', memoryview(array.array('B', [1, 2]))), ('list', [1, 2])]:
    show('slice-' + label, lambda rhs=rhs: m.__setitem__(slice(0, 2), rhs))"#,
    });
}

#[test]
fn cpython_array_module_and_constructor_public_surface_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py::MiscTest public constructor/module behavior",
        name: "array-module-and-constructor-public-surface",
        source: r#"import array
class S(str):
    pass
def show(label, expr):
    try:
        print(label, repr(expr()))
    except BaseException as error:
        print(label, error.__class__.__name__, str(error))
def show_class(label, expr):
    try:
        print(label, repr(expr()))
    except BaseException as error:
        print(label, error.__class__.__name__)
legacy = 'bBuhHiIlLqQfd'
print('module', isinstance(array.typecodes, str), all(tc in array.typecodes for tc in legacy))
print('constructors', array.array('B').typecode, array.array(S('b')).typecode)
print('roundtrip-legacy', ''.join(array.array(tc).typecode for tc in legacy))
show_class('bad-x', lambda: array.array('x'))
show_class('bad-empty', lambda: array.array(''))
show_class('bad-bytes', lambda: array.array(b'B'))
show_class('bad-int', lambda: array.array(65))
show_class('bad-none', lambda: array.array(None))
show('bad-arity0', lambda: array.array())
show('bad-arity3', lambda: array.array('B', [], 3))
show('bad-keyword', lambda: array.array(spam=42))
a = array.array('B')
a[:] = a
print('empty', len(a), len(a + a), len(a * 3), len(a.__iadd__(a)))"#,
    });
}

#[test]
fn cpython_array_subclass_public_construction_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public array subclass construction and __new__ behavior",
        name: "array-subclass-public-construction",
        source: r#"import array, copy
class A(array.array):
    pass
class AInit(array.array):
    def __init__(self, typecode, source=()):
        self.seen = (typecode, list(source) if not isinstance(source, str) else source)
class ANew(array.array):
    def __new__(cls, typecode, source=()):
        self = array.array.__new__(cls, typecode, source)
        self.flag = 'new'
        return self
def show(label, expr):
    try:
        print(label, repr(expr()))
    except BaseException as error:
        print(label, error.__class__.__name__, str(error))
for T in [A, AInit, ANew]:
    a = T('B', [1, 2])
    print('sub', T.__name__, type(a).__name__, isinstance(a, array.array), issubclass(T, array.array), a.typecode, a.itemsize, a.tolist(), a.tobytes(), repr(a), getattr(a, 'seen', None), getattr(a, 'flag', None))
    print('methods', a.append(3), a.tolist(), a.pop(), a.tolist(), list(reversed(a)))
    a.x = 10
    copied = copy.copy(a)
    print('copy', type(copied).__name__, isinstance(copied, array.array), copied.tolist(), getattr(copied, 'x', None), copied is a)
show('new-exact', lambda: (lambda a: (type(a).__name__, a.typecode, a.tolist()))(array.array.__new__(array.array, 'B', [4])))
show('new-sub', lambda: (lambda a: (type(a).__name__, a.typecode, a.tolist()))(array.array.__new__(A, 'B', [5])))
show('new-bad-class', lambda: array.array.__new__(list, 'B'))
print('visible', hasattr(array.array, '__new__'), hasattr(A, '__new__'))"#,
    });
}

#[test]
fn cpython_array_one_byte_public_sequence_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public one-byte array sequence and bytes surface",
        name: "array-one-byte-public-sequence",
        source: r#"import array
for tc, vals in [('B', [1, 255]), ('b', [-1, 0, 127])]:
    a = array.array(tc, vals)
    print(tc, a.typecode, a.itemsize, len(a), bool(a), a.tolist(), a.tobytes(), a[0], a[-1], a[1:], list(reversed(a)))
    print(a.__len__(), a.__getitem__(slice(0, 2)), list(a.__iter__()), a.__contains__(vals[0]), a.__contains__(999))
print(bool(array.array('B')))"#,
    });
}

#[test]
fn cpython_array_short_public_sequence_and_mutation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public signed and unsigned short array sequence and mutation surface",
        name: "array-short-public-sequence-and-mutation",
        source: r#"import array, io
class I:
    def __index__(self):
        return 7
class Bad:
    def __index__(self):
        return 'x'
def show(label, func):
    try:
        print(label, repr(func()))
    except BaseException as error:
        print(label, error.__class__.__name__, str(error))
for tc, vals, bads in [('h', [1, -2, 32767], [-32769, 32768]), ('H', [0, 7, 65535], [-1, 65536])]:
    print('tc', tc)
    show('basic', lambda tc=tc, vals=vals: (array.array(tc).itemsize, array.array(tc, vals).itemsize, len(array.array(tc, vals)), array.array(tc, vals).tolist(), array.array(tc, vals).tobytes(), repr(array.array(tc, vals))))
    show('from-index', lambda tc=tc: array.array(tc, [I()]).tolist())
    for value in bads:
        show('ctor-bad-' + str(value), lambda tc=tc, value=value: array.array(tc, [value]))
    show('ctor-bad-index-result', lambda tc=tc: array.array(tc, [Bad()]))
    a = array.array(tc, vals)
    show('sequence', lambda a=a: (a[0], a[-1], a[1:].tolist(), a[::-1].tolist(), list(a), list(reversed(a)), bytes(a)))
    show('setitem', lambda tc=tc, vals=vals: (lambda a: (a.__setitem__(1, I()), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('append-insert', lambda tc=tc, vals=vals: (lambda a: (a.append(I()), a.insert(-99, vals[-1]), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('fromlist', lambda tc=tc: (lambda a: (a.fromlist([I()]), a.tolist(), a.tobytes()))(array.array(tc)))
    show('frombytes-exact', lambda tc=tc, vals=vals: (lambda src: (lambda a: (a.frombytes(src.tobytes()), a.tolist(), a.tobytes()))(array.array(tc)))(array.array(tc, vals)))
    show('frombytes-short', lambda tc=tc: (lambda a: a.frombytes(b'x'))(array.array(tc)))
    show('byteswap', lambda tc=tc, vals=vals: (lambda a: (a.byteswap(), a.tobytes(), a.tolist()))(array.array(tc, vals)))
    show('pop-count-index', lambda a=a, vals=vals: (a.count(vals[0]), a.index(vals[-1]), a.pop(), a.tolist(), a.tobytes()))
    show('repeat-add', lambda tc=tc, vals=vals: ((array.array(tc, vals) + array.array(tc, vals[:1])).tolist(), (array.array(tc, vals) * 2).tolist()))
    show('fromfile-short-item', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01'), 1), a.tolist()))(array.array(tc)))
    show('fromfile-short-count', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01\x00'), 2), a.tolist(), a.tobytes()))(array.array(tc)))
for dst, src in [('B', array.array('b', [-1])), ('b', array.array('B', [255])), ('h', memoryview(b'\xff')), ('h', b'\x01\x00')]:
    show('ctor-source-' + dst + '-' + type(src).__name__, lambda dst=dst, src=src: (array.array(dst, src).tolist(), array.array(dst, src).tobytes()))"#,
    });
}

#[test]
fn cpython_array_int_public_sequence_and_mutation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public signed and unsigned int array sequence and mutation surface",
        name: "array-int-public-sequence-and-mutation",
        source: r#"import array, io
class I:
    def __index__(self):
        return 7
class Bad:
    def __index__(self):
        return 'x'
def show(label, func):
    try:
        print(label, repr(func()))
    except BaseException as error:
        print(label, error.__class__.__name__, str(error))
for tc, vals, bads in [('i', [1, -2, 2147483647], [-2147483649, 2147483648]), ('I', [0, 7, 4294967295], [-1, 4294967296])]:
    print('tc', tc)
    show('basic', lambda tc=tc, vals=vals: (array.array(tc).itemsize, array.array(tc, vals).itemsize, len(array.array(tc, vals)), array.array(tc, vals).tolist(), array.array(tc, vals).tobytes(), repr(array.array(tc, vals))))
    show('from-index', lambda tc=tc: array.array(tc, [I()]).tolist())
    for value in bads:
        show('ctor-bad-' + str(value), lambda tc=tc, value=value: array.array(tc, [value]))
    show('ctor-bad-index-result', lambda tc=tc: array.array(tc, [Bad()]))
    a = array.array(tc, vals)
    show('sequence', lambda a=a: (a[0], a[-1], a[1:].tolist(), a[::-1].tolist(), list(a), list(reversed(a)), bytes(a)))
    show('setitem', lambda tc=tc, vals=vals: (lambda a: (a.__setitem__(1, I()), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('append-insert', lambda tc=tc, vals=vals: (lambda a: (a.append(I()), a.insert(-99, vals[-1]), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('fromlist', lambda tc=tc: (lambda a: (a.fromlist([I()]), a.tolist(), a.tobytes()))(array.array(tc)))
    show('frombytes-exact', lambda tc=tc, vals=vals: (lambda src: (lambda a: (a.frombytes(src.tobytes()), a.tolist(), a.tobytes()))(array.array(tc)))(array.array(tc, vals)))
    show('frombytes-short', lambda tc=tc: (lambda a: a.frombytes(b'xyz'))(array.array(tc)))
    show('byteswap', lambda tc=tc, vals=vals: (lambda a: (a.byteswap(), a.tobytes(), a.tolist()))(array.array(tc, vals)))
    show('pop-count-index', lambda a=a, vals=vals: (a.count(vals[0]), a.index(vals[-1]), a.pop(), a.tolist(), a.tobytes()))
    show('repeat-add', lambda tc=tc, vals=vals: ((array.array(tc, vals) + array.array(tc, vals[:1])).tolist(), (array.array(tc, vals) * 2).tolist()))
    show('fromfile-short-item', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01'), 1), a.tolist()))(array.array(tc)))
    show('fromfile-short-count', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01\x00\x00\x00'), 2), a.tolist(), a.tobytes()))(array.array(tc)))
for dst, src in [('h', array.array('i', [-32769])), ('H', array.array('I', [65536])), ('i', memoryview(b'\xff')), ('i', b'\x01\x00\x00\x00')]:
    show('ctor-source-' + dst + '-' + type(src).__name__, lambda dst=dst, src=src: (array.array(dst, src).tolist(), array.array(dst, src).tobytes()))"#,
    });
}

#[test]
fn cpython_array_long_long_public_sequence_and_mutation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public signed and unsigned long long array sequence and mutation surface",
        name: "array-long-long-public-sequence-and-mutation",
        source: r#"import array, io
class I:
    def __index__(self):
        return 7
class BigI:
    def __index__(self):
        return 2**63
class Bad:
    def __index__(self):
        return 'x'
def show(label, func):
    try:
        print(label, repr(func()))
    except BaseException as error:
        print(label, error.__class__.__name__, str(error))
for tc, vals, bads in [('q', [1, -2, 9223372036854775807], [-9223372036854775809, 9223372036854775808]), ('Q', [0, 7, 18446744073709551615], [-1, 18446744073709551616])]:
    print('tc', tc)
    show('basic', lambda tc=tc, vals=vals: (array.array(tc).itemsize, array.array(tc, vals).itemsize, len(array.array(tc, vals)), array.array(tc, vals).tolist(), array.array(tc, vals).tobytes(), repr(array.array(tc, vals))))
    show('from-index', lambda tc=tc: array.array(tc, [I()]).tolist())
    show('from-big-index', lambda tc=tc: array.array(tc, [BigI()]).tolist())
    for value in bads:
        show('ctor-bad-' + str(value), lambda tc=tc, value=value: array.array(tc, [value]))
    show('ctor-bad-index-result', lambda tc=tc: array.array(tc, [Bad()]))
    a = array.array(tc, vals)
    show('sequence', lambda a=a: (a[0], a[-1], a[1:].tolist(), a[::-1].tolist(), list(a), list(reversed(a)), bytes(a)))
    show('setitem', lambda tc=tc, vals=vals: (lambda a: (a.__setitem__(1, I()), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('append-insert', lambda tc=tc, vals=vals: (lambda a: (a.append(I()), a.insert(-99, vals[-1]), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('fromlist', lambda tc=tc: (lambda a: (a.fromlist([I()]), a.tolist(), a.tobytes()))(array.array(tc)))
    show('frombytes-exact', lambda tc=tc, vals=vals: (lambda src: (lambda a: (a.frombytes(src.tobytes()), a.tolist(), a.tobytes()))(array.array(tc)))(array.array(tc, vals)))
    show('frombytes-short', lambda tc=tc: (lambda a: a.frombytes(b'abcdefg'))(array.array(tc)))
    show('byteswap', lambda tc=tc, vals=vals: (lambda a: (a.byteswap(), a.tobytes(), a.tolist()))(array.array(tc, vals)))
    show('pop-count-index', lambda a=a, vals=vals: (a.count(vals[0]), a.index(vals[-1]), a.pop(), a.tolist(), a.tobytes()))
    show('repeat-add', lambda tc=tc, vals=vals: ((array.array(tc, vals) + array.array(tc, vals[:1])).tolist(), (array.array(tc, vals) * 2).tolist()))
    show('fromfile-short-item', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01'), 1), a.tolist()))(array.array(tc)))
    show('fromfile-short-count', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01\x00\x00\x00\x00\x00\x00\x00'), 2), a.tolist(), a.tobytes()))(array.array(tc)))
for dst, src in [('i', array.array('q', [-2147483649])), ('I', array.array('Q', [4294967296])), ('q', memoryview(b'\xff')), ('q', b'\x01\x00\x00\x00\x00\x00\x00\x00')]:
    show('ctor-source-' + dst + '-' + type(src).__name__, lambda dst=dst, src=src: (array.array(dst, src).tolist(), array.array(dst, src).tobytes()))"#,
    });
}

#[test]
fn cpython_array_native_long_public_sequence_and_mutation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public native signed and unsigned long array sequence and mutation surface",
        name: "array-native-long-public-sequence-and-mutation",
        source: r#"import array, io
class I:
    def __index__(self):
        return 7
class BigI:
    def __index__(self):
        return 2**63
class Bad:
    def __index__(self):
        return 'x'
def show(label, func):
    try:
        print(label, repr(func()))
    except BaseException as error:
        print(label, error.__class__.__name__, str(error))
for tc, vals, bads in [('l', [1, -2, 9223372036854775807], [-9223372036854775809, 9223372036854775808]), ('L', [0, 7, 18446744073709551615], [-1, 18446744073709551616])]:
    print('tc', tc)
    show('basic', lambda tc=tc, vals=vals: (array.array(tc).itemsize, array.array(tc, vals).itemsize, len(array.array(tc, vals)), array.array(tc, vals).tolist(), array.array(tc, vals).tobytes(), repr(array.array(tc, vals))))
    show('from-index', lambda tc=tc: array.array(tc, [I()]).tolist())
    show('from-big-index', lambda tc=tc: array.array(tc, [BigI()]).tolist())
    for value in bads:
        show('ctor-bad-' + str(value), lambda tc=tc, value=value: array.array(tc, [value]))
    show('ctor-bad-index-result', lambda tc=tc: array.array(tc, [Bad()]))
    a = array.array(tc, vals)
    show('sequence', lambda a=a: (a[0], a[-1], a[1:].tolist(), a[::-1].tolist(), list(a), list(reversed(a)), bytes(a)))
    show('setitem', lambda tc=tc, vals=vals: (lambda a: (a.__setitem__(1, I()), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('append-insert', lambda tc=tc, vals=vals: (lambda a: (a.append(I()), a.insert(-99, vals[-1]), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('fromlist', lambda tc=tc: (lambda a: (a.fromlist([I()]), a.tolist(), a.tobytes()))(array.array(tc)))
    show('frombytes-exact', lambda tc=tc, vals=vals: (lambda src: (lambda a: (a.frombytes(src.tobytes()), a.tolist(), a.tobytes()))(array.array(tc)))(array.array(tc, vals)))
    show('frombytes-short', lambda tc=tc: (lambda a: a.frombytes(b'abcdefg'))(array.array(tc)))
    show('byteswap', lambda tc=tc, vals=vals: (lambda a: (a.byteswap(), a.tobytes(), a.tolist()))(array.array(tc, vals)))
    show('pop-count-index', lambda a=a, vals=vals: (a.count(vals[0]), a.index(vals[-1]), a.pop(), a.tolist(), a.tobytes()))
    show('repeat-add', lambda tc=tc, vals=vals: ((array.array(tc, vals) + array.array(tc, vals[:1])).tolist(), (array.array(tc, vals) * 2).tolist()))
    show('fromfile-short-item', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01'), 1), a.tolist()))(array.array(tc)))
    show('fromfile-short-count', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01\x00\x00\x00\x00\x00\x00\x00'), 2), a.tolist(), a.tobytes()))(array.array(tc)))
for dst, src in [('i', array.array('l', [-2147483649])), ('I', array.array('L', [4294967296])), ('l', memoryview(b'\xff')), ('l', b'\x01\x00\x00\x00\x00\x00\x00\x00')]:
    show('ctor-source-' + dst + '-' + type(src).__name__, lambda dst=dst, src=src: (array.array(dst, src).tolist(), array.array(dst, src).tobytes()))"#,
    });
}

#[test]
fn cpython_array_float_public_sequence_and_mutation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public float and double array sequence and mutation surface",
        name: "array-float-public-sequence-and-mutation",
        source: r#"import array, io
class F:
    def __float__(self):
        return 7.25
class I:
    def __index__(self):
        return 7
class BadFloat:
    def __float__(self):
        return 'x'
class BadIndex:
    def __index__(self):
        return 'x'
class FloatRaises:
    def __float__(self):
        raise RuntimeError('float boom')
def show(label, func):
    try:
        print(label, repr(func()))
    except BaseException as error:
        print(label, error.__class__.__name__, str(error))
for tc, vals in [('f', [1.5, -2.25, 3.5]), ('d', [1.5, -2.25, 3.5])]:
    print('tc', tc)
    show('basic', lambda tc=tc, vals=vals: (array.array(tc).itemsize, array.array(tc, vals).itemsize, len(array.array(tc, vals)), array.array(tc, vals).tolist(), array.array(tc, vals).tobytes(), repr(array.array(tc, vals))))
    show('from-float', lambda tc=tc: array.array(tc, [F()]).tolist())
    show('from-index', lambda tc=tc: array.array(tc, [I()]).tolist())
    show('ctor-bad-float-result', lambda tc=tc: array.array(tc, [BadFloat()]))
    show('ctor-bad-index-result', lambda tc=tc: array.array(tc, [BadIndex()]))
    show('ctor-float-raises', lambda tc=tc: array.array(tc, [FloatRaises()]))
    a = array.array(tc, vals)
    show('sequence', lambda a=a: (a[0], a[-1], a[1:].tolist(), a[::-1].tolist(), list(a), list(reversed(a)), bytes(a)))
    show('setitem', lambda tc=tc, vals=vals: (lambda a: (a.__setitem__(1, F()), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('append-insert', lambda tc=tc, vals=vals: (lambda a: (a.append(F()), a.insert(-99, vals[-1]), a.tolist(), a.tobytes()))(array.array(tc, vals)))
    show('fromlist', lambda tc=tc: (lambda a: (a.fromlist([F()]), a.tolist(), a.tobytes()))(array.array(tc)))
    show('frombytes-exact', lambda tc=tc, vals=vals: (lambda src: (lambda a: (a.frombytes(src.tobytes()), a.tolist(), a.tobytes()))(array.array(tc)))(array.array(tc, vals)))
    show('frombytes-short', lambda tc=tc: (lambda a: a.frombytes(b'abc'))(array.array(tc)))
    show('byteswap', lambda tc=tc, vals=vals: (lambda a: (a.byteswap(), a.tobytes(), a.tolist()))(array.array(tc, vals)))
    show('pop-count-index', lambda a=a, vals=vals: (a.count(vals[0]), a.index(vals[-1]), a.pop(), a.tolist(), a.tobytes()))
    show('repeat-add', lambda tc=tc, vals=vals: ((array.array(tc, vals) + array.array(tc, vals[:1])).tolist(), (array.array(tc, vals) * 2).tolist()))
    show('fromfile-short-item', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01'), 1), a.tolist()))(array.array(tc)))
    payload = b'\x00' * array.array(tc).itemsize
    show('fromfile-short-count', lambda tc=tc, payload=payload: (lambda a: (a.fromfile(io.BytesIO(payload), 2), a.tolist(), a.tobytes()))(array.array(tc)))
for dst, src in [('f', array.array('d', [1.5])), ('d', array.array('f', [1.5])), ('f', memoryview(b'\x00')), ('f', b'\x00\x00\xc0?'), ('d', b'\x00\x00\x00\x00\x00\x00\xf8?')]:
    show('ctor-source-' + dst + '-' + type(src).__name__, lambda dst=dst, src=src: (array.array(dst, src).tolist(), array.array(dst, src).tobytes()))"#,
    });
}

#[test]
fn cpython_array_unicode_public_sequence_and_mutation_diff_subset() {
    let probe = run_cpython(
        "import array\ntry:\n    array.array('w')\n    print('yes')\nexcept ValueError:\n    print('no')",
    )
    .expect("failed to probe CPython array('w') support");
    let supports_w = String::from_utf8(probe.stdout)
        .expect("CPython probe emitted non-UTF-8 output")
        .contains("yes");
    let typecodes = if supports_w { "['u', 'w']" } else { "['u']" };
    let cross_typecode_source = if supports_w {
        "for dst, src in [('u', array.array('w', 'A')), ('w', array.array('u', 'A'))]:\n    show('ctor-source-' + dst + '-' + type(src).__name__, lambda dst=dst, src=src: (array.array(dst, src).tolist(), array.array(dst, src).tobytes()))"
    } else {
        ""
    };
    let source = format!(
        r#"import array, io
def show(label, func):
    try:
        print(label, repr(func()))
    except BaseException as error:
        print(label, error.__class__.__name__, str(error))
def show_class(label, func):
    try:
        print(label, repr(func()))
    except BaseException as error:
        print(label, error.__class__.__name__)
for tc in {typecodes}:
    print('tc', tc)
    show('empty', lambda tc=tc: (array.array(tc).typecode, array.array(tc).itemsize, len(array.array(tc)), array.array(tc).tolist(), array.array(tc).tobytes(), repr(array.array(tc)), array.array(tc).tounicode()))
    for text in ['Az', 'éΩ', '😀']:
        show('basic-' + text, lambda tc=tc, text=text: (array.array(tc, text).itemsize, len(array.array(tc, text)), array.array(tc, text).tolist(), array.array(tc, text).tobytes(), repr(array.array(tc, text)), array.array(tc, text).tounicode()))
    show('fromunicode', lambda tc=tc: (lambda a: (a.fromunicode('A😀'), a.tolist(), a.tobytes(), a.tounicode()))(array.array(tc)))
    show('frombytes-exact', lambda tc=tc: (lambda src: (lambda a: (a.frombytes(src.tobytes()), a.tolist(), a.tounicode(), a.tobytes()))(array.array(tc)))(array.array(tc, 'Az')))
    show('frombytes-short', lambda tc=tc: (lambda a: a.frombytes(b'abc'))(array.array(tc)))
    a = array.array(tc, 'Az')
    show('sequence', lambda a=a: (a[0], a[-1], a[1:].tolist(), a[::-1].tolist(), list(a), list(reversed(a)), bytes(a)))
    show('setitem', lambda tc=tc: (lambda a: (a.__setitem__(1, 'Ω'), a.tolist(), a.tobytes(), a.tounicode()))(array.array(tc, 'Az')))
    show('append-insert', lambda tc=tc: (lambda a: (a.append('😀'), a.insert(-99, 'Ω'), a.tolist(), a.tobytes(), a.tounicode()))(array.array(tc, 'Az')))
    show('fromlist', lambda tc=tc: (lambda a: (a.fromlist(['A', 'Ω']), a.tolist(), a.tobytes(), a.tounicode()))(array.array(tc)))
    show('byteswap', lambda tc=tc: (lambda a: (a.byteswap(), a.tobytes(), a.tolist()))(array.array(tc, 'Az')))
    show('pop-count-index', lambda a=a: (a.count('A'), a.index('z'), a.pop(), a.tolist(), a.tobytes()))
    show('repeat-add', lambda tc=tc: ((array.array(tc, 'Az') + array.array(tc, 'A')).tolist(), (array.array(tc, 'Az') * 2).tolist()))
    show('fromfile-short-item', lambda tc=tc: (lambda a: (a.fromfile(io.BytesIO(b'\x01'), 1), a.tolist()))(array.array(tc)))
    payload = b'\x00' * array.array(tc).itemsize
    show('fromfile-short-count', lambda tc=tc, payload=payload: (lambda a: (a.fromfile(io.BytesIO(payload), 2), a.tolist(), a.tobytes()))(array.array(tc)))
    for value in ['AB', b'A', 65, None, '']:
        show_class('append-bad-' + type(value).__name__ + '-' + repr(value), lambda tc=tc, value=value: array.array(tc).append(value))
    show('fromunicode-type', lambda tc=tc: array.array(tc).fromunicode(b'A'))
    show('constructor-list', lambda tc=tc: array.array(tc, ['A', 'Ω']).tolist())
    show('constructor-bytes', lambda tc=tc: array.array(tc, array.array(tc, 'A').tobytes()).tolist())
{cross_typecode_source}"#
    );
    assert_cpython_output_parity_source(
        "Lib/test/test_array.py public unicode array sequence and mutation surface",
        "array-unicode-public-sequence-and-mutation",
        &source,
    );
}

#[test]
fn cpython_array_one_byte_public_mutation_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public one-byte array mutable sequence methods",
        name: "array-one-byte-public-mutation-methods",
        source: r#"import array
def show(label, expr):
    try:
        value = expr()
        print(label, value)
    except Exception as error:
        print(label, error.__class__.__name__, str(error))
for tc, vals in [('B', [1, 2]), ('b', [-1, 2])]:
    a = array.array(tc, vals)
    print('methods', tc, 'copy' in dir(a), 'insert' in dir(a))
    print('append-insert', tc, a.append(vals[-1]), a.insert(1, vals[0]), repr(a), a.tolist(), bytes(a))
    print('extend', tc, a.extend(vals), repr(a), a.tolist(), bytes(a))
    print('pop-reverse', tc, a.pop(), a.reverse(), repr(a), a.tolist())
    print('count-index-contains', tc, a.count(vals[0]), a.index(vals[0]), vals[0] in a, float(vals[0]) in a)
    show('index-missing-' + tc, lambda a=a: a.index(999))
    print('remove', tc, a.remove(vals[0]), repr(a), a.tolist())
    print('fromlist-frombytes', tc, a.fromlist(vals), a.frombytes(b'\xff\x02'), repr(a), a.tolist(), bytes(a))
for tc, bad in [('B', -1), ('B', 256), ('b', -129), ('b', 128)]:
    a = array.array(tc, [0])
    show('append-bad-' + tc + '-' + str(bad), lambda a=a, bad=bad: a.append(bad))
    print('state', repr(a))
for tc in ['B', 'b']:
    a = array.array(tc)
    show('pop-empty-' + tc, lambda a=a: a.pop())
    a = array.array(tc, [0])
    other = array.array('b' if tc == 'B' else 'B', [-1, 2] if tc == 'B' else [1, 2])
    show('extend-other-' + tc, lambda a=a, other=other: a.extend(other))
    print('state', repr(a))
    show('fromlist-type-' + tc, lambda a=a: a.fromlist((1, 2)))
    show('frombytes-type-' + tc, lambda a=a: a.frombytes([1]))"#,
    });
}

#[test]
fn cpython_array_one_byte_public_subscript_mutation_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public one-byte array subscript mutation",
        name: "array-one-byte-public-subscript-mutation",
        source: r#"import array
class I:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
def show(label, expr):
    try:
        value = expr()
        print(label, value)
    except Exception as error:
        print(label, error.__class__.__name__, str(error))
def show_class(label, expr):
    try:
        value = expr()
        print(label, value)
    except Exception as error:
        print(label, error.__class__.__name__)
for tc, vals in [('B', [1, 2, 3, 4]), ('b', [-1, 2, 3, 4])]:
    a = array.array(tc, vals)
    a[0] = vals[1]
    a[-1] = vals[0]
    a[I(1)] = vals[1]
    print('scalar', tc, repr(a), a.tolist(), bytes(a))
    show('scalar-bounds-' + tc, lambda a=a: a.__setitem__(99, vals[0]))
    show_class('scalar-type-' + tc, lambda a=a: a.__setitem__(0, 'x'))
    b = array.array(tc, vals)
    print('dunder-set', tc, b.__setitem__(0, vals[1]), repr(b))
    b[1:3] = array.array(tc, vals[:1])
    print('slice-shrink', tc, repr(b), b.tolist(), bytes(b))
    b[1:2] = array.array(tc, vals[1:4])
    print('slice-grow', tc, repr(b), b.tolist(), bytes(b))
    show('slice-list-' + tc, lambda b=b: b.__setitem__(slice(0, 1), [vals[0]]))
    other = array.array('b' if tc == 'B' else 'B', [-1, 2] if tc == 'B' else [1, 2])
    show('slice-other-' + tc, lambda b=b, other=other: b.__setitem__(slice(0, 1), other))
    c = array.array(tc, vals)
    c[::2] = array.array(tc, vals[:2])
    print('ext-slice', tc, repr(c), c.tolist(), bytes(c))
    show('ext-len-' + tc, lambda c=c: c.__setitem__(slice(None, None, 2), array.array(tc, vals[:1])))
    d = array.array(tc, vals)
    print('dunder-del', tc, d.__delitem__(1), repr(d), d.tolist(), bytes(d))
    del d[::2]
    print('del-ext', tc, repr(d), d.tolist(), bytes(d))
    e = array.array(tc, vals)
    del e[1:3]
    print('del-contig', tc, repr(e), e.tolist(), bytes(e))
    show('del-bounds-' + tc, lambda e=e: e.__delitem__(99))"#,
    });
}

#[test]
fn cpython_array_one_byte_public_copy_byteswap_compare_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public one-byte array copy, byteswap, and comparisons",
        name: "array-one-byte-public-copy-byteswap-compare",
        source: r#"import array, copy
def show(label, expr):
    try:
        print(label, expr())
    except Exception as error:
        print(label, error.__class__.__name__)
for tc, vals in [('B', [1, 2, 3]), ('b', [-1, 2, 3])]:
    a = array.array(tc, vals)
    print('methods', tc, 'copy' in dir(a), '__copy__' in dir(a), '__deepcopy__' in dir(a), 'byteswap' in dir(a))
    shallow = copy.copy(a)
    deep = copy.deepcopy(a)
    direct = a.__copy__()
    direct_deep = a.__deepcopy__({})
    print('copies', tc, repr(shallow), repr(deep), repr(direct), repr(direct_deep), shallow is a, deep is a, direct is a, direct_deep is a)
    shallow[0] = vals[-1]
    deep[1] = vals[0]
    direct[-1] = vals[0]
    direct_deep[0] = vals[-1]
    print('copy-independent', tc, repr(a), repr(shallow), repr(deep), repr(direct), repr(direct_deep))
    print('byteswap', tc, a.byteswap(), repr(a), bytes(a))
    show('deepcopy-arity0-' + tc, lambda a=a: a.__deepcopy__())
    show('deepcopy-arity2-' + tc, lambda a=a: a.__deepcopy__({}, {}))
base = array.array('B', [1, 2])
for label, other in [
    ('sameB', array.array('B', [1, 2])),
    ('greaterB', array.array('B', [1, 3])),
    ('shortB', array.array('B', [1])),
    ('sameb', array.array('b', [1, 2])),
    ('signed-low', array.array('b', [-1, 2])),
    ('list', [1, 2]),
    ('bytes', b'\x01\x02'),
]:
    print('eq', label, base == other, base != other, base.__eq__(other), base.__ne__(other))
    print('dunder-order', label, base.__lt__(other), base.__le__(other), base.__gt__(other), base.__ge__(other))
    show('op-lt-' + label, lambda other=other: base < other)
    show('op-le-' + label, lambda other=other: base <= other)
    show('op-gt-' + label, lambda other=other: base > other)
    show('op-ge-' + label, lambda other=other: base >= other)"#,
    });
}

#[test]
fn cpython_array_one_byte_public_concat_repeat_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public one-byte array concatenation and repeat",
        name: "array-one-byte-public-concat-repeat",
        source: r#"import array
class I:
    def __index__(self):
        return 3
class BadIndex:
    def __index__(self):
        return 'x'
def show(label, expr):
    try:
        value = expr()
        if hasattr(value, 'tolist'):
            print(label, repr(value), value.tolist(), value.tobytes())
        else:
            print(label, value)
    except Exception as error:
        print(label, error.__class__.__name__, str(error))
def inplace_add(tc, vals):
    a = array.array(tc, vals)
    alias = a
    a += array.array(tc, vals[::-1])
    print('iadd-op', tc, repr(a), a.tolist(), a is alias)
def inplace_mul(tc, vals, count):
    a = array.array(tc, vals)
    alias = a
    a *= count
    print('imul-op', tc, count.__class__.__name__, repr(a), a.tolist(), a is alias)
for tc, vals in [('B', [1, 2]), ('b', [-1, 2])]:
    a = array.array(tc, vals)
    same = array.array(tc, vals[::-1])
    other = array.array('b' if tc == 'B' else 'B', [-1, 2] if tc == 'B' else [1, 2])
    print('methods', tc, '__add__' in dir(a), '__iadd__' in dir(a), '__mul__' in dir(a), '__rmul__' in dir(a), '__imul__' in dir(a))
    show('add-' + tc, lambda a=a, same=same: a + same)
    show('dunder-add-' + tc, lambda a=a, same=same: a.__add__(same))
    show('add-other-' + tc, lambda a=a, other=other: a + other)
    show('add-list-' + tc, lambda a=a: a + [1])
    show('dunder-iadd-list-' + tc, lambda a=a: a.__iadd__([1]))
    show('mul2-' + tc, lambda a=a: a * 2)
    show('rmul2-' + tc, lambda a=a: 2 * a)
    show('mul-index-' + tc, lambda a=a: a * I())
    show('mul0-' + tc, lambda a=a: a * 0)
    show('mulneg-' + tc, lambda a=a: a * -1)
    show('mul-bad-' + tc, lambda a=a: a * 'x')
    show('dunder-mul-bad-' + tc, lambda a=a: a.__mul__('x'))
    show('mul-bad-index-' + tc, lambda a=a: a * BadIndex())
    d = array.array(tc, vals)
    direct = d.__iadd__(same)
    print('iadd-dunder', tc, repr(d), d.tolist(), direct is d)
    inplace_add(tc, vals)
    for count in [3, 0, -1, False, True, I()]:
        inplace_mul(tc, vals, count)
    d = array.array(tc, vals)
    direct = d.__imul__(2)
    print('imul-dunder', tc, repr(d), d.tolist(), direct is d)
    show('imul-bad-' + tc, lambda tc=tc, vals=vals: array.array(tc, vals).__imul__('x'))"#,
    });
}

#[test]
fn cpython_array_one_byte_public_buffer_info_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public one-byte array buffer_info method",
        name: "array-one-byte-public-buffer-info",
        source: r#"import array
def show(label, expr):
    try:
        print(label, expr())
    except Exception as error:
        print(label, error.__class__.__name__)
for tc, vals in [('B', [1, 2, 3]), ('b', [-1, 0, 127])]:
    a = array.array(tc, vals)
    info = a.buffer_info()
    print('info', tc, 'buffer_info' in dir(a), type(info).__name__, len(info), type(info[0]).__name__, info[0] == 0, info[1], len(a), a.itemsize)
    a.append(vals[0])
    info2 = a.buffer_info()
    print('after', tc, info2[1], len(a), type(info2[0]).__name__, info2[0] == 0)
    print('dunder', tc, getattr(a, 'buffer_info')().__class__.__name__)
    show('arity-' + tc, lambda a=a: a.buffer_info(1))"#,
    });
}

#[test]
fn cpython_array_one_byte_public_unicode_method_rejection_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public one-byte array unicode helper method rejection",
        name: "array-one-byte-public-unicode-method-rejection",
        source: r#"import array
def show(label, expr):
    try:
        value = expr()
        print(label, value)
    except Exception as error:
        print(label, error.__class__.__name__, str(error))
def show_class(label, expr):
    try:
        value = expr()
        print(label, value)
    except Exception as error:
        print(label, error.__class__.__name__)
for tc, vals in [('B', [65, 66]), ('b', [65, -1])]:
    a = array.array(tc, vals)
    print('methods', tc, 'fromunicode' in dir(a), 'tounicode' in dir(a))
    show_class('tounicode-' + tc, lambda a=a: a.tounicode())
    show_class('fromunicode-' + tc, lambda a=a: a.fromunicode('AZ'))
    print('state', tc, repr(a), a.tolist(), bytes(a))
    show('fromunicode-arity0-' + tc, lambda a=a: a.fromunicode())
    show('fromunicode-arity2-' + tc, lambda a=a: a.fromunicode('A', 'B'))
    show('fromunicode-type-' + tc, lambda a=a: a.fromunicode(b'A'))
    show('tounicode-arity-' + tc, lambda a=a: a.tounicode(1))"#,
    });
}

#[test]
fn cpython_array_one_byte_public_file_methods_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public one-byte array tofile/fromfile methods and io.BytesIO",
        name: "array-one-byte-public-file-methods",
        source: r#"import array, io
def show_class(label, expr):
    try:
        value = expr()
        print(label, value)
    except Exception as error:
        print(label, error.__class__.__name__)

bio = io.BytesIO(b'abc')
print('bytesio-methods', hasattr(bio, 'read'), hasattr(bio, 'write'), hasattr(bio, 'getvalue'))
print('bytesio-read', bio.read(1), bio.read(None), bio.read())
bio = io.BytesIO(b'abc')
print('bytesio-write', bio.write(b'XY'), bio.getvalue(), bio.read(), bio.write(bytearray(b'Z')), bio.getvalue())
show_class('bytesio-write-type', lambda: io.BytesIO().write('x'))
show_class('bytesio-read-arity', lambda: io.BytesIO().read(1, 2))
show_class('bytesio-getvalue-arity', lambda: io.BytesIO().getvalue(1))

class TextRead:
    def read(self, n):
        return 'abc'
class ByteArrayRead:
    def read(self, n):
        return bytearray(b'ab')

for tc, vals in [('B', [65, 66, 67]), ('b', [65, -1, 0])]:
    a = array.array(tc, vals)
    target = io.BytesIO()
    print('methods', tc, 'tofile' in dir(a), 'fromfile' in dir(a))
    print('tofile', tc, a.tofile(target), target.getvalue(), a.tolist())
    print('append-write', tc, target.write(b'!'), target.getvalue())
    src = io.BytesIO(target.getvalue() + b'Z')
    c = array.array(tc)
    print('fromfile1', tc, c.fromfile(src, 2), c.tolist(), c.tobytes())
    show_class('fromfile-short-' + tc, lambda c=c, src=src: c.fromfile(src, 10))
    print('after-short', tc, c.tolist(), c.tobytes())
    z = array.array(tc, [vals[0]])
    zero = io.BytesIO(b'Q')
    print('fromfile-zero', tc, z.fromfile(zero, 0), z.tolist(), zero.read())
    show_class('tofile-arity0-' + tc, lambda a=a: a.tofile())
    show_class('tofile-arity2-' + tc, lambda a=a: a.tofile(io.BytesIO(), 1))
    show_class('fromfile-arity0-' + tc, lambda tc=tc: array.array(tc).fromfile())
    show_class('fromfile-arity1-' + tc, lambda tc=tc: array.array(tc).fromfile(io.BytesIO()))
    show_class('fromfile-arity3-' + tc, lambda tc=tc: array.array(tc).fromfile(io.BytesIO(), 1, 2))
    show_class('fromfile-neg-' + tc, lambda tc=tc: array.array(tc).fromfile(io.BytesIO(), -1))
    show_class('fromfile-nonint-' + tc, lambda tc=tc: array.array(tc).fromfile(io.BytesIO(), 'x'))
    show_class('fromfile-textread-' + tc, lambda tc=tc: array.array(tc).fromfile(TextRead(), 2))
    show_class('fromfile-bytearrayread-' + tc, lambda tc=tc: array.array(tc).fromfile(ByteArrayRead(), 2))"#,
    });
}

#[test]
fn cpython_array_one_byte_public_clear_diff_subset() {
    let oracle_probe = run_cpython("import array\nprint(hasattr(array.array('B'), 'clear'))")
        .expect("failed to run CPython array.clear capability probe");
    let oracle_stdout = String::from_utf8(oracle_probe.stdout)
        .expect("CPython array.clear probe emitted non-UTF-8");
    if oracle_stdout.trim() != "True" {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_array.py public array.clear method",
        name: "array-one-byte-public-clear",
        source: r#"import array
for tc, vals in [('B', [1, 2]), ('b', [-1, 2])]:
    a = array.array(tc, vals)
    print(tc, 'clear' in dir(a), a.clear(), repr(a), bool(a), len(a))"#,
    });
}

#[test]
fn cpython_float_hash_and_sys_info_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_hash and ::test_hash_nan",
        name: "float-hash-and-sys-info",
        source: r#"import sys
ok = True
for x in range(-30, 30):
    ok = ok and hash(float(x)) == hash(x)
print('small-int-float', ok)
print('minus-one', hash(-1), hash(-1.0))
print('max', hash(float(sys.float_info.max)) == hash(int(sys.float_info.max)))
print('inf', hash(float('inf')) == sys.hash_info.inf, hash(float('-inf')) == -sys.hash_info.inf)
value = float('nan')
print('nan', isinstance(hash(value), int), hash(value) != 42)
class H:
    def __hash__(self):
        return 42
class F(float, H):
    pass
value = F('nan')
print('subnan', isinstance(hash(value), int), hash(value) == 42)
print('sys-float-info', sys.float_info.mant_dig, sys.float_info.radix, sys.float_info.rounds)
print('sys-hash-info', sys.hash_info.inf, sys.hash_info.nan, sys.hash_info.imag)"#,
    });
}

#[test]
fn cpython_float_int_comparison_boundaries_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_issue_gh143006 and public float/int exact-comparison boundaries",
        name: "float-int-comparison-boundaries",
        source: r#"class EvilInt(int):
    def __neg__(self):
        return ''

i = -1 << 50
f = float(i) - 0.5
i = EvilInt(i)
print('evil', f == i, f != i, f < i, f <= i, f > i, f >= i)

huge = 2 ** 200
hf = float(huge)
print('huge-eq', hf == huge, hf == huge + 1, hf == huge - 1, hf != huge + 1)
print('huge-order', hf < huge + 1, hf > huge - 1, hf <= huge, hf >= huge)

small = 2 ** 60
sf = float(small)
print('i64-order', sf == small, sf == small + 1, sf < small + 1, sf > small - 1)

class I(int):
    pass
j = I(small + 1)
print('subclass', sf == j, sf < j, sf <= j, sf > j, sf >= j)

nan = float('nan')
print('nan-order', nan < 1, nan <= 1, nan > 1, nan >= 1, 1 < nan, 1 <= nan, 1 > nan, 1 >= nan)"#,
    });
}

#[test]
fn cpython_float_getformat_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::FormatFunctionsTestCase::test_getformat",
        name: "float-getformat",
        source: r#"for name in ['double', 'float']:
    value = float.__getformat__(name)
    print(name, value in ['unknown', 'IEEE, big-endian', 'IEEE, little-endian'])
try:
    float.__getformat__('chicken')
except ValueError as error:
    print('bad-name', error.__class__.__name__, error.args[0])
try:
    float.__getformat__(1)
except TypeError as error:
    print('bad-type', error.__class__.__name__, error.args[0])
print('dir', '__getformat__' in dir(float), callable(float.__getformat__))
print('instance', (1.0).__getformat__('double') == float.__getformat__('double'))
class F(float):
    pass
print('subclass', F.__getformat__('float') == float.__getformat__('float'), F(1.0).__getformat__('float') == float.__getformat__('float'))"#,
    });
}

#[test]
fn cpython_float_default_precision_format_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::FormatTestCase::test_format and ::test_issue5864",
        name: "float-default-precision-format",
        source: r#"checks = [
    ('issue-a', 123.456, '.4'),
    ('issue-b', 1234.56, '.4'),
    ('issue-c', 12345.6, '.4'),
    ('one-p0', 1.0, '.0'),
    ('one-p1', 1.0, '.1'),
    ('one-p2', 1.0, '.2'),
    ('one-p4', 1.0, '.4'),
    ('sign-plus', 123.456, '+.4'),
    ('sign-space', 123.456, ' .4'),
    ('alternate', 123.456, '#.4'),
    ('nan-sign', float('nan'), '+.4'),
    ('inf-zero', float('inf'), '010,.2'),
]
for label, value, spec in checks:
    print(label, format(value, spec))
x = 100 / 7.0
print('empty-like', format(x, '') == format(x, '-') == format(x, '>') == format(x, '2') == str(x))"#,
    });
}

#[test]
fn cpython_float_fractional_grouping_format_diff_subset() {
    let oracle_probe = run_cpython("print(format(1.23, '._f'))")
        .expect("failed to run CPython fractional grouping capability probe");
    if !oracle_probe.status.success() {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::FormatTestCase::test_format",
        name: "float-fractional-grouping-format",
        source: r#"x = 123_456.123_456
checks = [
    ('frac-under-fixed', '._f'),
    ('frac-comma-fixed', '.,f'),
    ('both-under-fixed', '_._f'),
    ('both-comma-fixed', ',.,f'),
    ('prec-under-fixed', '.10_f'),
    ('prec-comma-fixed', '.10,f'),
    ('right-width', '>21._f'),
    ('left-width', '<21._f'),
    ('signed-under-exp', '+.11_e'),
    ('signed-comma-exp', '+.11,e'),
    ('zero-under-fixed-21', '021_._f'),
    ('zero-under-fixed-20', '020_._f'),
    ('signed-zero-under-fixed', '+021_._f'),
    ('space-width-under-fixed', '21_._f'),
    ('right-zero-under-fixed', '>021_._f'),
    ('left-zero-under-fixed', '<021_._f'),
    ('zero-under-fixed-prec10-23', '023_.10_f'),
    ('zero-under-fixed-prec10-22', '022_.10_f'),
    ('signed-zero-under-fixed-prec10', '+023_.10_f'),
    ('zero-under-fixed-prec9', '023_.9_f'),
    ('zero-under-exp-21', '021_._e'),
    ('zero-under-exp-20', '020_._e'),
    ('signed-zero-under-exp', '+021_._e'),
    ('zero-under-exp-prec10-23', '023_.10_e'),
    ('zero-under-exp-prec10-22', '022_.10_e'),
    ('zero-under-exp-prec9', '023_.9_e'),
]
for label, spec in checks:
    print(label, format(x, spec))
bad_specs = ['._6f', '.,_f', '.6,_f', '.6_,f', '.6_n', '.6,n']
for spec in bad_specs:
    try:
        format(x, spec)
    except ValueError as error:
        print('bad', spec, error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_float_zero_width_format_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::FormatTestCase::test_issue35560",
        name: "float-zero-width-format",
        source: r#"checks = [
    ('pos-empty-zero-width', 123.0, '00'),
    ('pos-fixed-zero-width', 123.34, '00f'),
    ('pos-exp-zero-width', 123.34, '00e'),
    ('pos-general-zero-width', 123.34, '00g'),
    ('pos-fixed-precision', 123.34, '00.10f'),
    ('pos-exp-precision', 123.34, '00.10e'),
    ('pos-general-precision', 123.34, '00.10g'),
    ('pos-width-one-fixed', 123.34, '01f'),
    ('neg-empty-zero-width', -123.0, '00'),
    ('neg-fixed-zero-width', -123.34, '00f'),
    ('neg-exp-zero-width', -123.34, '00e'),
    ('neg-general-zero-width', -123.34, '00g'),
    ('neg-fixed-precision', -123.34, '00.10f'),
    ('neg-exp-precision', -123.34, '00.10e'),
    ('neg-general-precision', -123.34, '00.10g'),
]
for label, value, spec in checks:
    print(label, format(value, spec))"#,
    });
}

#[test]
fn cpython_float_format_testfile_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::FormatTestCase::test_format_testfile",
        name: "float-format-testfile",
        source: r#"data = '''
%.0f 0 -> 0
%.1f 0 -> 0.0
%.50f 0 -> 0.00000000000000000000000000000000000000000000000000
%.0f 1.5 -> 2
%.0f 2.5 -> 2
%.0f 3.5 -> 4
%.0f 1e49 -> 9999999999999999464902769475481793196872414789632
%.0f 1e50 -> 100000000000000007629769841091887003294964970946560
%.1f 0.06 -> 0.1
%.1f 0.25 -> 0.2
%.1f 0.75 -> 0.8
%.2f 0.125 -> 0.12
%.2f 0.375 -> 0.38
%.2f 1234567.8912 -> 1234567.89
%#.0f 0 -> 0.
%#.1f 0 -> 0.0
%#.0f 1.5 -> 2.
%#.0f 2.5 -> 2.
%#.0f 10.1 -> 10.
%#.0f 1234.56 -> 1235.
%#.1f 1.4 -> 1.4
%#.2f 0.375 -> 0.38
%f 0 -> 0.000000
%f 1.23456789 -> 1.234568
%f 0.0000005001 -> 0.000001
%f 0.0000004999 -> 0.000000
%.0e 0 -> 0e+00
%.50e 0 -> 0.00000000000000000000000000000000000000000000000000e+00
%.0e 0.01 -> 1e-02
%.0e 1 -> 1e+00
%.0e 123.456 -> 1e+02
%.0e 0.5 -> 5e-01
%.0e 1.4 -> 1e+00
%.0e 6.5 -> 6e+00
%.1e 0.0001 -> 1.0e-04
%#.0e 0.01 -> 1.e-02
%#.0e 0.1 -> 1.e-01
%#.0e 1 -> 1.e+00
%#.0e 10 -> 1.e+01
%#.0e 100 -> 1.e+02
%#.0e 0.012 -> 1.e-02
%#.0e 0.12 -> 1.e-01
%#.0e 1.2 -> 1.e+00
%#.0e 12 -> 1.e+01
%#.0e 120 -> 1.e+02
%#.0e 123.456 -> 1.e+02
%#.0e 0.000123456 -> 1.e-04
%#.0e 123456000 -> 1.e+08
%#.0e 0.5 -> 5.e-01
%#.0e 1.4 -> 1.e+00
%#.0e 1.5 -> 2.e+00
%#.0e 1.6 -> 2.e+00
%#.0e 2.4999999 -> 2.e+00
%#.0e 2.5 -> 2.e+00
%#.0e 2.5000001 -> 3.e+00
%#.0e 3.499999999999 -> 3.e+00
%#.0e 3.5 -> 4.e+00
%#.0e 4.5 -> 4.e+00
%#.0e 5.5 -> 6.e+00
%#.0e 6.5 -> 6.e+00
%#.0e 7.5 -> 8.e+00
%#.0e 8.5 -> 8.e+00
%#.0e 9.4999 -> 9.e+00
%#.0e 9.5 -> 1.e+01
%#.0e 10.5 -> 1.e+01
%#.0e 14.999 -> 1.e+01
%#.0e 15 -> 2.e+01
%#.1e 123.4 -> 1.2e+02
%#.2e 0.0001357 -> 1.36e-04
%.0g 0 -> 0
%.100g 0 -> 0
%.0g 1000 -> 1e+03
%.0g 1 -> 1
%.0g 1e-3 -> 0.001
%.0g 1e-5 -> 1e-05
%.0g 0.12 -> 0.1
%.1g 1e-6 -> 1e-06
%.1g 0.0012 -> 0.001
%.2g 1e-6 -> 1e-06
%.2g 0.00123 -> 0.0012
%#.0g 0 -> 0.
%#.1g 0 -> 0.
%#.2g 0 -> 0.0
%#.3g 0 -> 0.00
%#.4g 0 -> 0.000
%#.0g 0.2 -> 0.2
%#.1g 0.2 -> 0.2
%#.2g 0.2 -> 0.20
%#.3g 0.2 -> 0.200
%#.4g 0.2 -> 0.2000
%#.10g 0.2 -> 0.2000000000
%#.0g 2 -> 2.
%#.1g 2 -> 2.
%#.2g 2 -> 2.0
%#.3g 2 -> 2.00
%#.4g 2 -> 2.000
%#.0g 20 -> 2.e+01
%#.1g 20 -> 2.e+01
%#.2g 20 -> 20.
%#.3g 20 -> 20.0
%#.4g 20 -> 20.00
%#.0g 234.56 -> 2.e+02
%#.1g 234.56 -> 2.e+02
%#.2g 234.56 -> 2.3e+02
%#.3g 234.56 -> 235.
%#.4g 234.56 -> 234.6
%#.5g 234.56 -> 234.56
%#.6g 234.56 -> 234.560
%r 0 -> 0.0
%r 1 -> 1.0
%r 1e15 -> 1000000000000000.0
%r 9999999999999999 -> 1e+16
%r 1e16 -> 1e+16
%r 1.000000000000001e-4 -> 0.0001000000000000001
%r 0.9999999999999999e-4 -> 9.999999999999999e-05
%r 1e-5 -> 1e-05
'''
cases = []
for line in data.splitlines():
    line = line.strip()
    if not line:
        continue
    lhs, expected = [part.strip() for part in line.split('->')]
    fmt, arg = lhs.split()
    cases.append((fmt, arg, expected))

checks = 0
for fmt, arg, expected in cases:
    value = float(arg)
    for label, got, wanted in [
        ('percent', fmt % value, expected),
        ('percent-neg', fmt % -value, '-' + expected),
    ]:
        checks += 1
        if got != wanted:
            print('mismatch', label, fmt, arg, repr(got), repr(wanted))
    if fmt != '%r':
        spec = fmt[1:]
        for label, got, wanted in [
            ('format', format(value, spec), expected),
            ('format-neg', format(-value, spec), '-' + expected),
        ]:
            checks += 1
            if got != wanted:
                print('mismatch', label, fmt, arg, repr(got), repr(wanted))
print('checked', len(cases), checks)"#,
    });
}

#[test]
fn cpython_float_format_testfile_full_diff_subset() {
    let Some(source) = cpython_formatfloat_testfile_source() else {
        return;
    };
    assert_cpython_output_parity_source(
        "Lib/test/test_float.py::FormatTestCase::test_format_testfile",
        "float-format-testfile-full",
        &source,
    );
}

#[test]
fn cpython_float_repr_roundtrip_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::ReprTestCase::test_repr",
        name: "float-repr-roundtrip",
        source: r#"import math
texts = '''
0E0 -0E0 1E0 15E-1 125E-2 1125E-3 10625E-4
103125E-5 1015625E-6 10078125E-7 100390625E-8
1001953125E-9 10009765625E-10 100048828125E-11
1000244140625E-12 10001220703125E-13 100006103515625E-14
1000030517578125E-15 10000152587890625E-16 +8E153 -1E153
+9E306 -2E153 +7E-304 -3E-49 +7E-303 +50609263E157
+2572981889477453E142 -33584377202279118724E-252
+36992084760177624177E-318 -73984169520355248354E-318
+99257763227713890244E-115 -87336362425182547697E-280 -87E-274
-9821613080E121 -82783038381290406E165 +67536228609141569109E-133
-35620497849450218807E-306 +66550376797582521751E-126 +1721E-17
-68384463429E25 +76E-23 +134976318E25 -2739849386524269E26
+5479698773048538E26 +6124568318523113E-25 -1139777988171071E-24
+6322612303128019E-27 -2955864564844617E-25 -9994029144998961E25
-2971238324022087E27 -1656055679333934E-27 -1445488709150234E-26
+55824717499885172E27 -69780896874856465E26 +84161538867545199E25
-27912358749942586E27 +24711112462926331E-25 -12645224606256038E-27
-12249136637046226E-25 +74874448287465757E27 -35642836832753303E24
-71285673665506606E24 +43723334984997307E-26 +10182419849537963E-24
-93501703572661982E-26 2183167012312112312312.23538020374420446192e-370
0.99999999999999999999999999999999999999999e+23
'''.split()
checked = 0
for text in texts:
    value = eval(text)
    rendered = repr(value)
    roundtrip = eval(rendered)
    if value != roundtrip:
        print('mismatch', text, rendered, roundtrip)
    if value == 0.0 and math.copysign(1.0, value) != math.copysign(1.0, roundtrip):
        print('zero-sign-mismatch', text, rendered)
    if str(value) != rendered:
        print('str-repr-mismatch', text, str(value), rendered)
    checked += 1
print('checked', checked, repr(eval(texts[0])), repr(eval(texts[-1])))"#,
    });
}

#[test]
fn cpython_float_repr_roundtrip_full_diff_subset() {
    let Some(source) = cpython_floating_points_repr_source() else {
        return;
    };
    assert_cpython_output_parity_source(
        "Lib/test/test_float.py::ReprTestCase::test_repr",
        "float-repr-roundtrip-full",
        &source,
    );
}

#[test]
fn cpython_float_short_repr_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::ReprTestCase::test_short_repr",
        name: "float-short-repr",
        source: r#"import sys
test_strings = [
    '0.0',
    '1.0',
    '0.01',
    '0.02',
    '0.03',
    '0.04',
    '0.05',
    '1.23456789',
    '10.0',
    '100.0',
    '1000000000000000.0',
    '9999999999999990.0',
    '1e+16',
    '1e+17',
    '0.001',
    '0.001001',
    '0.00010000000000001',
    '0.0001',
    '9.999999999999e-05',
    '1e-05',
    '8.72293771110361e+25',
    '7.47005307342313e+26',
    '2.86438000439698e+28',
    '8.89142905246179e+28',
    '3.08578087079232e+35',
]
checked = 0
for text in test_strings:
    for candidate in (text, '-' + text):
        value = float(candidate)
        checked += 1
        if repr(value) != candidate:
            print('repr-mismatch', candidate, repr(value))
        if str(value) != repr(value):
            print('str-mismatch', candidate, str(value), repr(value))
        if eval(repr(value)) != value:
            print('roundtrip-mismatch', candidate, repr(value))
print('style', sys.float_repr_style)
print('checked', checked)"#,
    });
}

#[test]
fn cpython_float_round_specials_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::RoundTestCase",
        name: "float-round-specials",
        source: r#"import math
def show(label, fn):
    try:
        value = fn()
        if isinstance(value, float):
            if math.isnan(value):
                print(label, 'nan', math.copysign(1.0, value), repr(value))
            else:
                print(label, repr(value), math.copysign(1.0, value))
        else:
            print(label, repr(value), type(value).__name__)
    except BaseException as error:
        print(label, type(error).__name__, error.args[0])

INF = float('inf')
NAN = float('nan')
for label, fn in [
    ('round-inf', lambda: round(INF)),
    ('round-ninf', lambda: round(-INF)),
    ('round-nan', lambda: round(NAN)),
    ('round-inf-bad-ndigits', lambda: round(INF, 0.0)),
    ('round-nan-bad-ndigits', lambda: round(NAN, "ceci n\\'est pas un integer")),
    ('round-negzero-complex-ndigits', lambda: round(-0.0, 1j)),
    ('round-inf-0', lambda: round(INF, 0)),
    ('round-ninf-0', lambda: round(-INF, 0)),
    ('round-nan-0', lambda: round(NAN, 0)),
    ('round-large-324', lambda: round(123.456, 324)),
    ('round-large-307', lambda: round(1e300, 307)),
    ('round-subnormal-315', lambda: round(1.4e-315, 315)),
    ('round-small-neg308', lambda: round(-123.456, -308)),
    ('round-small-neg309', lambda: round(-123.456, -309)),
    ('round-overflow-pos', lambda: round(1.6e308, -308)),
    ('round-overflow-neg', lambda: round(-1.7e308, -308)),
    ('round-prev-a', lambda: round(562949953421312.5, 1)),
    ('round-prev-b', lambda: round(56294995342131.5, 3)),
    ('round-half-25', lambda: round(25.0, -1)),
    ('round-half-35', lambda: round(35.0, -1)),
    ('round-half-45', lambda: round(45.0, -1)),
    ('round-half-55', lambda: round(55.0, -1)),
    ('round-none-pos', lambda: round(1.23, None)),
    ('round-none-kw', lambda: round(1.78, ndigits=None)),
    ('round-large-big', lambda: round(123.456, 2**100)),
    ('round-small-big', lambda: round(-123.456, -2**100)),
]:
    show(label, fn)

def identical(x, y):
    return x == y and (x != 0.0 or math.copysign(1.0, x) == math.copysign(1.0, y))

large_ok = True
large_count = 0
for n in [324, 325, 400, 2**31-1, 2**31, 2**32, 2**100]:
    for value in [123.456, -123.456, 1e300, 1e-320]:
        large_count += 1
        if round(value, n) != value:
            large_ok = False
for value, n, expected in [
    (1e150, 300, 1e150),
    (1e300, 307, 1e300),
    (-3.1415, 308, -3.1415),
    (1e150, 309, 1e150),
    (1.4e-315, 315, 1e-315),
]:
    large_count += 1
    if round(value, n) != expected:
        large_ok = False

small_ok = True
small_count = 0
for n in [-308, -309, -400, 1-2**31, -2**31, -2**31-1, -2**100]:
    for value, expected in [
        (123.456, 0.0),
        (-123.456, -0.0),
        (1e300, 0.0),
        (1e-320, 0.0),
    ]:
        small_count += 1
        if not identical(round(value, n), expected):
            small_ok = False
print('large-grid', large_ok, large_count)
print('small-grid', small_ok, small_count)"#,
    });
}

#[test]
fn cpython_float_round_dunder_none_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::RoundTestCase::test_round_with_none_arg_direct_call",
        name: "float-round-dunder-none",
        source: r#"def show(label, fn):
    try:
        value = fn()
        print(label, repr(value), type(value).__name__, type(value) is int, type(value) is float)
    except BaseException as error:
        print(label, type(error).__name__)

class MyFloat(float):
    pass

for label, fn in [
    ('bound-noarg', lambda: (1.0).__round__()),
    ('bound-none', lambda: (1.0).__round__(None)),
    ('bound-zero', lambda: (1.25).__round__(0)),
    ('bound-one', lambda: (1.25).__round__(1)),
    ('bound-big-pos', lambda: (123.456).__round__(2**100)),
    ('bound-big-neg', lambda: (-123.456).__round__(-2**100)),
    ('bound-bad', lambda: (1.25).__round__(1.0)),
    ('bound-kw', lambda: (1.25).__round__(ndigits=1)),
    ('desc-none', lambda: float.__round__(1.25, None)),
    ('desc-one', lambda: float.__round__(1.25, 1)),
    ('desc-bad-receiver', lambda: float.__round__(1)),
    ('subclass-none', lambda: MyFloat(1.75).__round__(None)),
    ('subclass-one', lambda: MyFloat(1.75).__round__(1)),
]:
    show(label, fn)
print('has-dir', '__round__' in dir(1.0), '__round__' in dir(float))"#,
    });
}

#[test]
fn cpython_float_round_matches_format_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::RoundTestCase::test_matches_float_format",
        name: "float-round-matches-format",
        source: r#"def check(label, values):
    checked = 0
    for x in values:
        for ndigits in range(4):
            formatted = float(format(x, '.' + str(ndigits) + 'f'))
            rounded = round(x, ndigits)
            if formatted != rounded:
                print('mismatch', label, repr(x), ndigits, repr(formatted), repr(rounded))
            checked += 1
    print(label, checked)

check('thousandths', [i / 1000.0 for i in range(500)])
check('half-cent-grid', [i / 1000.0 for i in range(5, 5000, 10)])
check('deterministic-random-like', [((i * 1103515245 + 12345) % 1000000) / 1000000.0 for i in range(500)])"#,
    });
}

#[test]
fn cpython_float_format_specials_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::RoundTestCase::test_format_specials",
        name: "float-format-specials",
        source: r#"INF = float('inf')
NAN = float('nan')
formats = ['%e', '%f', '%g', '%.0e', '%.6f', '%.20g',
           '%#e', '%#f', '%#g', '%#.20e', '%#.15f', '%#.3g']
rows = []
for fmt in formats:
    for label, value, expected in [
        ('inf', INF, 'inf'),
        ('ninf', -INF, '-inf'),
        ('nan', NAN, 'nan'),
        ('nnan', -NAN, 'nan'),
    ]:
        rows.append((fmt, label, value, expected))
    pfmt = '%+' + fmt[1:]
    for label, value, expected in [
        ('p-inf', INF, '+inf'),
        ('p-ninf', -INF, '-inf'),
        ('p-nan', NAN, '+nan'),
        ('p-nnan', -NAN, '+nan'),
    ]:
        rows.append((pfmt, label, value, expected))
    sfmt = '% ' + fmt[1:]
    for label, value, expected in [
        ('s-inf', INF, ' inf'),
        ('s-ninf', -INF, '-inf'),
        ('s-nan', NAN, ' nan'),
        ('s-nnan', -NAN, ' nan'),
    ]:
        rows.append((sfmt, label, value, expected))

checked = 0
for fmt, label, value, expected in rows:
    percent_value = fmt % value
    format_value = format(value, fmt[1:])
    if percent_value != expected:
        print('percent-mismatch', fmt, label, repr(percent_value), repr(expected))
    if format_value != expected:
        print('format-mismatch', fmt[1:], label, repr(format_value), repr(expected))
    checked += 2
print('checked', checked)"#,
    });
}

#[test]
fn cpython_float_inf_nan_string_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::InfNanTest",
        name: "float-inf-nan-string",
        source: r#"import math
def show_float(label, expr):
    try:
        value = expr()
        print(label, math.isinf(value), math.isnan(value), repr(value), str(value), math.copysign(1.0, value))
    except BaseException as error:
        print(label, type(error).__name__, error.args[0])

for text in ['inf', '+inf', '-inf', 'infinity', '+infinity', '-infinity', 'INF', '+Inf', '-iNF', 'Infinity', '+iNfInItY', '-INFINITY']:
    show_float('parse-inf ' + text, lambda text=text: float(text))
for text in ['info', '+info', '-info', 'in', '+in', '-in', 'infinit', '+Infin', '-INFI', 'infinitys', '++Inf', '-+inf', '+-infinity', '--Infinity']:
    show_float('bad-inf ' + text, lambda text=text: float(text))
for text in ['nan', '+nan', '-nan', 'NAN', '+NAn', '-NaN']:
    show_float('parse-nan ' + text, lambda text=text: float(text))
for text in ['nana', '+nana', '-nana', 'na', '+na', '-na', '++nan', '-+NAN', '+-NaN', '--nAn']:
    show_float('bad-nan ' + text, lambda text=text: float(text))
for label, value in [
    ('inf-as-repr', 1e300 * 1e300),
    ('ninf-as-repr', -1e300 * 1e300),
    ('nan-as-repr', 1e300 * 1e300 * 0),
    ('neg-nan-as-repr', -1e300 * 1e300 * 0),
]:
    print(label, repr(value), str(value), math.isinf(value), math.isnan(value), math.copysign(1.0, value))
print('sign-inf', math.copysign(1.0, float('inf')), math.copysign(1.0, float('-inf')))
print('sign-nan', math.copysign(1.0, float('nan')), math.copysign(1.0, float('-nan')))"#,
    });
}

#[test]
fn cpython_float_from_number_diff_subset() {
    let oracle_probe = run_cpython("print(hasattr(float, 'from_number'))")
        .expect("failed to run CPython float.from_number capability probe");
    let oracle_stdout = String::from_utf8(oracle_probe.stdout)
        .expect("CPython float.from_number probe emitted non-UTF-8 output");
    if oracle_stdout.trim() != "True" {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_from_number and ::test_from_number_subclass",
        name: "float-from-number",
        source: r#"class FloatSubclass(float):
    pass
class OtherFloatSubclass(float):
    pass
class FloatLike:
    def __init__(self, value):
        self.value = value
    def __float__(self):
        return self.value
class MyIndex:
    def __init__(self, value):
        self.value = value
    def __index__(self):
        return self.value
class MyInt:
    def __init__(self, value):
        self.value = value
    def __int__(self):
        return self.value

def show(label, value, typ):
    print(label, value, type(value) is typ)

show('float-float', float.from_number(3.14), float)
show('float-int', float.from_number(314), float)
show('float-subclass-input', float.from_number(OtherFloatSubclass(3.14)), float)
show('float-like', float.from_number(FloatLike(3.14)), float)
show('float-like-subclass-result', float.from_number(FloatLike(OtherFloatSubclass(2.5))), float)
show('float-index', float.from_number(MyIndex(314)), float)
show('subclass-float', FloatSubclass.from_number(3.14), FloatSubclass)
show('subclass-index', FloatSubclass.from_number(MyIndex(314)), FloatSubclass)
print('dir', 'from_number' in dir(float), 'from_number' in dir(1.0), 'from_number' in dir(FloatSubclass), 'from_number' in dir(FloatSubclass(1.0)))
print('instance-call', type((1.0).from_number(2.0)) is float, type(FloatSubclass(1.0).from_number(2.0)) is FloatSubclass)
NAN = float('nan')
x = float.from_number(NAN)
print('nan', x != x, type(x) is float, x is NAN)
y = FloatSubclass.from_number(NAN)
print('subclass-nan', y != y, type(y) is FloatSubclass)
for label, expr in [
    ('str', lambda: float.from_number('3.14')),
    ('bytes', lambda: float.from_number(b'3.14')),
    ('complex', lambda: float.from_number(3.14j)),
    ('myint', lambda: float.from_number(MyInt(314))),
    ('dict', lambda: float.from_number({})),
    ('none', lambda: float.from_number()),
    ('many', lambda: float.from_number(1, 2)),
    ('kw', lambda: float.from_number(x=1)),
]:
    try:
        expr()
    except TypeError as error:
        print(label, error.__class__.__name__)
try:
    float.from_number(MyIndex(2**2000))
except OverflowError as error:
    print('huge-index', error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_complex_subclass_constructor_and_from_number_diff_subset() {
    let oracle_probe = run_cpython("print(hasattr(complex, 'from_number'))")
        .expect("failed to run CPython complex.from_number capability probe");
    let oracle_stdout = String::from_utf8(oracle_probe.stdout)
        .expect("CPython complex.from_number probe emitted non-UTF-8 output");
    if oracle_stdout.trim() != "True" {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_constructor and ::test_from_number subclass rows",
        name: "complex-subclass-constructor-and-from-number",
        source: r#"class ComplexSubclass(complex):
    pass
class ComplexSubclassWithNew(complex):
    def __new__(cls, value=0j):
        return complex.__new__(cls, value + 1j)
class WithComplex:
    def __complex__(self):
        return ComplexSubclass(3+4j)
class WithFloat:
    def __float__(self):
        return 2.5

def show(label, value, typ):
    print(label, value, type(value).__name__, type(value) is complex, type(value) is typ, isinstance(value, complex), value.real, value.imag)

show('default', ComplexSubclass(), ComplexSubclass)
show('real', ComplexSubclass(1.5), ComplexSubclass)
show('complex', ComplexSubclass(1+2j), ComplexSubclass)
show('two-arg', ComplexSubclass(1, 2), ComplexSubclass)
show('kw', ComplexSubclass(real=1, imag=2), ComplexSubclass)
show('new-direct', complex.__new__(ComplexSubclass, 1+2j), ComplexSubclass)
show('new-kw', complex.__new__(ComplexSubclass, real=1, imag=2), ComplexSubclass)
show('custom-new', ComplexSubclassWithNew(1+2j), ComplexSubclassWithNew)
show('from-complex', ComplexSubclass.from_number(1+2j), ComplexSubclass)
show('from-with-complex', ComplexSubclass.from_number(WithComplex()), ComplexSubclass)
show('from-with-float', ComplexSubclass.from_number(WithFloat()), ComplexSubclass)
show('custom-from', ComplexSubclassWithNew.from_number(1+2j), ComplexSubclassWithNew)
show('exact-from-subclass-result', complex.from_number(WithComplex()), complex)
show('exact-constructor-subclass-result', complex(WithComplex()), complex)
z = ComplexSubclass(1+2j)
print('dir', 'from_number' in dir(ComplexSubclass), 'from_number' in dir(z), '__new__' in dir(complex))
print('unbound', type(complex.__complex__(z)) is complex, type(complex.conjugate(z)) is complex, type(complex.__pos__(z)) is complex, type(complex.__neg__(z)) is complex)
print('ops', z + 1, 1 + z, z - 1, 1 - z, z * 2, z / 2)
print('unary-bool-abs', +z, -z, bool(ComplexSubclass()), bool(z), abs(ComplexSubclass(3+4j)))
print('compare-hash', z == 1+2j, z != 1+2j, hash(ComplexSubclass(1+0j)) == hash(1+0j))
print('instance-from', type(z.from_number(5)) is ComplexSubclass, z.from_number(5))"#,
    });
}

#[test]
fn cpython_complex_two_arg_protocol_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_constructor two-argument complex protocol rows",
        name: "complex-two-arg-protocol",
        source: r#"class ComplexSubclass(complex):
    pass
class WithComplex:
    def __init__(self, value):
        self.value = value
    def __complex__(self):
        return self.value

def show(label, expr):
    try:
        z = expr()
        print(label, type(z) is complex, repr(z.real), repr(z.imag), z)
    except TypeError as error:
        print(label, error.__class__.__name__)

for label, expr in [
    ('real-complex-zero', lambda: complex(4.25+0j, 0)),
    ('real-subclass-zero', lambda: complex(ComplexSubclass(4.25+0j), 0)),
    ('real-provider-zero', lambda: complex(WithComplex(4.25+0j), 0)),
    ('imag-complex', lambda: complex(0, 4.25+0j)),
    ('imag-subclass', lambda: complex(0, ComplexSubclass(4.25+0j))),
    ('imag-provider', lambda: complex(0, WithComplex(4.25+0j))),
    ('both-complex', lambda: complex(4.25j, 0j)),
    ('kw-real-complex', lambda: complex(real=4.25+1.5j)),
]:
    show(label, expr)"#,
    });
}

#[test]
fn cpython_complex_subclass_constructor_special_numbers_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_constructor custom subclass __complex__ rows and ::test_constructor_special_numbers subclass rows",
        name: "complex-subclass-constructor-special-numbers",
        source: r#"from math import copysign
INF = float('inf')
NAN = float('nan')
class ComplexSubclass(complex):
    pass
class complex0(complex):
    def __complex__(self):
        return 42j
class complex1(complex):
    def __new__(cls, value=0j):
        return complex.__new__(cls, 2*value)
    def __complex__(self):
        return self
class complex2(complex):
    def __complex__(self):
        return None

def show(label, expr):
    try:
        value = expr()
        print(label, value, type(value).__name__, type(value) is complex, type(value) is ComplexSubclass, repr(value.real), repr(value.imag))
    except TypeError as error:
        print(label, error.__class__.__name__)

show('complex0', lambda: complex(complex0(1j)))
show('complex1', lambda: complex(complex1(1j)))
show('complex2', lambda: complex(complex2(1j)))

def same_float(actual, expected):
    if actual != actual and expected != expected:
        return copysign(1.0, actual) == copysign(1.0, expected)
    return actual == expected and copysign(1.0, actual) == copysign(1.0, expected)
values = [0.0, -0.0, INF, -INF, NAN]
ok_sub = ok_exact = ok_round = True
for x in values:
    for y in values:
        z = ComplexSubclass(x, y)
        ok_sub = ok_sub and type(z) is ComplexSubclass and same_float(z.real, x) and same_float(z.imag, y)
        z = complex(ComplexSubclass(x, y))
        ok_exact = ok_exact and type(z) is complex and same_float(z.real, x) and same_float(z.imag, y)
        z = ComplexSubclass(complex(x, y))
        ok_round = ok_round and type(z) is ComplexSubclass and same_float(z.real, x) and same_float(z.imag, y)
print('special-matrix', ok_sub, ok_exact, ok_round)"#,
    });
}

#[test]
fn cpython_complex_truediv_nonfinite_diff_subset() {
    let oracle_probe = run_cpython(
        "INF = float('inf')\nz = (1+1j) / complex(INF, INF)\nprint(z.real == 0.0 and z.imag == 0.0)",
    )
    .expect("failed to run CPython complex true division non-finite capability probe");
    let oracle_stdout = String::from_utf8(oracle_probe.stdout)
        .expect("CPython complex true division probe emitted non-UTF-8 output");
    if oracle_stdout.trim() != "True" {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_truediv non-finite recovery rows",
        name: "complex-truediv-nonfinite-recovery",
        source: r#"from math import copysign
INF = float('inf')
NAN = float('nan')
def same_float(actual, expected):
    if actual != actual and expected != expected:
        return True
    return actual == expected and copysign(1.0, actual) == copysign(1.0, expected)
def same_complex(actual, expected):
    return same_float(actual.real, expected.real) and same_float(actual.imag, expected.imag)
cases = [
    (complex(INF, NAN) / 2, complex(INF, NAN)),
    (complex(INF, 1)/(0.0+1j), complex(NAN, -INF)),
    (complex(INF, -INF)/(1+0j), complex(INF, -INF)),
    (complex(INF, INF)/(0.0+1j), complex(INF, -INF)),
    (complex(NAN, INF)/complex(2**1000, 2**-1000), complex(INF, INF)),
    (complex(INF, NAN)/complex(2**1000, 2**-1000), complex(INF, -INF)),
    ((1+1j)/complex(INF, INF), (0.0+0j)),
    ((1+1j)/complex(INF, -INF), (0.0+0j)),
    ((1+1j)/complex(-INF, INF), complex(0.0, -0.0)),
    ((1+1j)/complex(-INF, -INF), complex(-0.0, 0)),
    ((INF+1j)/complex(INF, INF), complex(NAN, NAN)),
    (complex(1, INF)/complex(INF, INF), complex(NAN, NAN)),
    (complex(INF, 1)/complex(1, INF), complex(NAN, NAN)),
    (INF/(1+0j), complex(INF, NAN)),
    (INF/(0.0+1j), complex(NAN, -INF)),
    (INF/complex(2**1000, 2**-1000), complex(INF, NAN)),
    (INF/complex(NAN, NAN), complex(NAN, NAN)),
    (float(1)/complex(INF, INF), (0.0-0j)),
    (float(1)/complex(INF, -INF), (0.0+0j)),
    (float(1)/complex(-INF, INF), complex(-0.0, -0.0)),
    (float(1)/complex(-INF, -INF), complex(-0.0, 0)),
    (float(1)/complex(INF, NAN), complex(0.0, -0.0)),
    (float(1)/complex(-INF, NAN), complex(-0.0, -0.0)),
    (float(1)/complex(NAN, INF), complex(0.0, -0.0)),
    (float(INF)/complex(NAN, INF), complex(NAN, NAN)),
]
print(all(same_complex(actual, expected) for actual, expected in cases))
for label, value in [
    ('finite-neginf-inf', (1+1j)/complex(-INF, INF)),
    ('finite-neginf-neginf', (1+1j)/complex(-INF, -INF)),
    ('real-over-inf-inf', float(1)/complex(INF, INF)),
    ('real-over-neginf-inf', float(1)/complex(-INF, INF)),
]:
    print(label, repr(value.real), copysign(1.0, value.real), repr(value.imag), copysign(1.0, value.imag))"#,
    });
}

#[test]
fn cpython_complex_truediv_extreme_inverse_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_truediv huge and tiny inverse rows",
        name: "complex-truediv-extreme-inverse",
        source: r#"def close(a, b):
    return abs(a - b) < 1e-9
ok = True
for x in [complex(1e200, 1e200), complex(1e-200, 1e-200)]:
    y = 1+0j
    z = x * y
    ok = ok and close(z / x, y)
    ok = ok and close(z.__truediv__(x), y)
    ok = ok and close(z / y, x)
    ok = ok and close(z.__truediv__(y), x)
print(ok)"#,
    });
}

#[test]
fn cpython_complex_mul_nonfinite_diff_subset() {
    let oracle_probe = run_cpython(
        "INF = float('inf')\nNAN = float('nan')\nz = (1e300+1j) * complex(NAN, INF)\nprint(z.real == -INF and z.imag == INF)",
    )
    .expect("failed to run CPython complex multiplication non-finite capability probe");
    let oracle_stdout = String::from_utf8(oracle_probe.stdout)
        .expect("CPython complex multiplication probe emitted non-UTF-8 output");
    if oracle_stdout.trim() != "True" {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_mul non-finite complex-by-complex rows",
        name: "complex-mul-nonfinite-recovery",
        source: r#"from math import copysign
INF = float('inf')
NAN = float('nan')
def same_float(actual, expected):
    if actual != actual and expected != expected:
        return True
    return actual == expected and copysign(1.0, actual) == copysign(1.0, expected)
def same_complex(actual, expected):
    return same_float(actual.real, expected.real) and same_float(actual.imag, expected.imag)
cases = [
    (1e300+1j, complex(INF, INF), complex(NAN, INF)),
    (1e300+1j, complex(NAN, INF), complex(-INF, INF)),
    (1e300+1j, complex(INF, NAN), complex(INF, INF)),
    (complex(INF, 1), complex(NAN, INF), complex(NAN, INF)),
    (complex(INF, 1), complex(INF, NAN), complex(INF, NAN)),
    (complex(NAN, 1), complex(1, INF), complex(-INF, NAN)),
    (complex(1, NAN), complex(1, INF), complex(NAN, INF)),
    (complex(1e200, NAN), complex(1e200, NAN), complex(INF, NAN)),
    (complex(1e200, NAN), complex(NAN, 1e200), complex(NAN, INF)),
    (complex(NAN, 1e200), complex(1e200, NAN), complex(NAN, INF)),
    (complex(NAN, 1e200), complex(NAN, 1e200), complex(-INF, NAN)),
    (complex(NAN, NAN), complex(NAN, NAN), complex(NAN, NAN)),
]
ok = True
for z, w, expected in cases:
    ok = ok and same_complex(z * w, expected)
    ok = ok and same_complex(w * z, expected)
print(ok)
for label, value in [
    ('finite-nan-inf', (1e300+1j) * complex(NAN, INF)),
    ('finite-inf-nan', (1e300+1j) * complex(INF, NAN)),
    ('nan-one-one-inf', complex(NAN, 1) * complex(1, INF)),
    ('nan-huge-nan-huge', complex(NAN, 1e200) * complex(NAN, 1e200)),
]:
    print(label, repr(value.real), copysign(1.0, value.real), repr(value.imag), copysign(1.0, value.imag))"#,
    });
}

#[test]
fn cpython_complex_pow_zero_and_stress_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_pow zero exponent and self-comparison rows",
        name: "complex-pow-zero-and-stress",
        source: r#"a = 3.33+4.43j
print(a ** 0j == 1+0j, type(a ** 0j) is complex)
print(a ** (0.0+0.0j) == 1+0j, type(a ** (0.0+0.0j)) is complex)
print(3j ** 0j == 1+0j, type(3j ** 0j) is complex)
print(3j ** 0 == 1+0j, type(3j ** 0) is complex)
print(a ** 105 == a ** 105)
print(a ** -105 == a ** -105)
print(a ** -30 == a ** -30)
print(0.0j ** 0 == 1+0j, type(0.0j ** 0) is complex)"#,
    });
}

#[test]
fn cpython_complex_division_unsupported_zero_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_floordiv_zero_division, ::test_mod_zero_division, and ::test_divmod_zero_division",
        name: "complex-unsupported-division-zero-operands",
        source: r#"ZERO_DIVISION = [(1+1j, 0+0j), (1+1j, 0.0), (1+1j, 0), (1.0, 0+0j), (1, 0+0j)]
for op in ['floordiv', 'mod', 'divmod']:
    for a, b in ZERO_DIVISION:
        try:
            if op == 'floordiv':
                a // b
            elif op == 'mod':
                a % b
            else:
                divmod(a, b)
        except TypeError as error:
            print(op, error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_complex_pow_overflow_boundary_diff_subset() {
    let oracle_probe = run_cpython(
        "try:\n    pow(1e200+1j, 5)\nexcept OverflowError:\n    print('True')\nelse:\n    print('False')",
    )
    .expect("failed to run CPython complex pow overflow capability probe");
    let oracle_stdout = String::from_utf8(oracle_probe.stdout)
        .expect("CPython complex pow overflow probe emitted non-UTF-8 output");
    if oracle_stdout.trim() != "True" {
        return;
    }

    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_complex.py::ComplexTest::test_pow overflow and boundary rows",
        name: "complex-pow-overflow-boundary",
        source: r#"import sys
for label, expr in [
    ('general-overflow', lambda: pow(1e200+1j, 1e200+1j)),
    ('integer-overflow', lambda: pow(1e200+1j, 5)),
    ('large-imag-overflow', lambda: 9j ** (33j**3)),
]:
    try:
        expr()
    except OverflowError as error:
        print(label, error.__class__.__name__)
for label, expr in [
    ('zero-complex-a', lambda: 0j ** (3.33+4.43j)),
    ('zero-complex-b', lambda: 0j ** (3-2j)),
]:
    try:
        expr()
    except ZeroDivisionError as error:
        print(label, error.__class__.__name__)
values = (sys.maxsize, sys.maxsize+1, sys.maxsize-1, -sys.maxsize, -sys.maxsize+1, -sys.maxsize+1)
ok = True
for real in values:
    for imag in values:
        c = complex(real, imag)
        for expr in [lambda c=c, real=real: c ** real, lambda c=c: c ** c]:
            try:
                expr()
            except OverflowError:
                pass
            except BaseException:
                ok = False
print('boundary-no-crash', ok)"#,
    });
}

#[test]
fn cpython_float_keywords_in_subclass_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_keywords_in_subclass",
        name: "float-keywords-in-subclass",
        source: r#"class subclass(float):
    pass
u = subclass(2.5)
print('plain', type(u) is subclass, float(u), repr(u))

class subclass_with_init(float):
    def __init__(self, arg, newarg=None):
        self.newarg = newarg
u = subclass_with_init(2.5, newarg=3)
print('init', type(u) is subclass_with_init, float(u), u.newarg)

class subclass_with_new(float):
    def __new__(cls, arg, newarg=None):
        self = super().__new__(cls, arg)
        self.newarg = newarg
        return self
u = subclass_with_new(2.5, newarg=3)
print('new', type(u) is subclass_with_new, float(u), u.newarg)"#,
    });
}

#[test]
fn cpython_float_containment_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_float_containment",
        name: "float-containment",
        source: r#"INF = float('inf')
NAN = float('nan')
floats = (INF, -INF, 0.0, 1.0, NAN)
for label, value in [('inf', INF), ('-inf', -INF), ('0.0', 0.0), ('1.0', 1.0), ('nan', NAN)]:
    print('contains', label, value in [value], value in (value,), value in {value}, value in {value: None}, [value].count(value), value in floats)
for label, value in [('inf', INF), ('-inf', -INF), ('0.0', 0.0), ('1.0', 1.0), ('nan', NAN)]:
    l, t, s, d = [value], (value,), {value}, {value: None}
    print('selfeq', label, [value] == [value], (value,) == (value,), {value} == {value}, {value: None} == {value: None}, l == l, t == t, s == s, d == d)
other_nan = float('nan')
print('distinct-nan', NAN == other_nan, NAN is other_nan, other_nan in {NAN}, {NAN} == {other_nan})"#,
    });
}

#[test]
fn cpython_float_floor_ceil_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_float_floor and ::test_float_ceil",
        name: "float-floor-ceil",
        source: r#"class FloatSubclass(float):
    pass
for method in ['__floor__', '__ceil__']:
    print('dir', method, method in dir(float), method in dir(1.0), method in dir(FloatSubclass), method in dir(FloatSubclass(1.0)))
    for value in [0.5, 1.0, 1.5, -0.5, -1.0, -1.5, 1.23e20, -1.23e20]:
        result = getattr(value, method)()
        unbound = getattr(float, method)(value)
        subclass_result = getattr(FloatSubclass(value), method)()
        print(method, repr(value), result, type(result).__name__, result == unbound, result == subclass_result)
    for value in [float('nan'), float('inf'), float('-inf')]:
        try:
            getattr(value, method)()
        except Exception as error:
            print(method, repr(value), error.__class__.__name__, str(error))
    for expr in [lambda: getattr(1.0, method)(1), lambda: getattr(float, method)(), lambda: getattr(float, method)('1.0')]:
        try:
            expr()
        except TypeError as error:
            print(method, 'typeerror', error.__class__.__name__)"#,
    });
}

#[test]
fn cpython_float_mod_signed_zero_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_float_mod",
        name: "float-mod-signed-zero",
        source: r#"import math
import operator

def same_float(actual, expected):
    if actual == expected:
        if actual != 0.0:
            return True
        return math.copysign(1.0, actual) == math.copysign(1.0, expected)
    return False

cases = [
    (-1.0, 1.0, 0.0),
    (-1e-100, 1.0, 1.0),
    (-0.0, 1.0, 0.0),
    (0.0, 1.0, 0.0),
    (1e-100, 1.0, 1e-100),
    (1.0, 1.0, 0.0),
    (-1.0, -1.0, -0.0),
    (-1e-100, -1.0, -1e-100),
    (-0.0, -1.0, -0.0),
    (0.0, -1.0, -0.0),
    (1e-100, -1.0, -1.0),
    (1.0, -1.0, -0.0),
]
ok = True
for left, right, expected in cases:
    ok = ok and same_float(left % right, expected)
    ok = ok and same_float(operator.mod(left, right), expected)
print('mod-signs', ok, len(cases), repr((-1.0) % -1.0), math.copysign(1.0, (-1.0) % -1.0), repr((1e-100) % -1.0))"#,
    });
}

#[test]
fn cpython_float_pow_special_cases_diff_subset() {
    assert_cpython_output_parity(&DiffCase {
        origin: "Lib/test/test_float.py::GeneralFloatCases::test_float_pow",
        name: "float-pow-special-cases",
        source: r#"import math
import operator

def same_float(actual, expected):
    if type(expected).__name__ == 'float' and math.isnan(expected):
        return type(actual).__name__ == 'float' and math.isnan(actual)
    if actual == expected:
        if type(expected).__name__ == 'float' and expected == 0.0:
            return math.copysign(1.0, actual) == math.copysign(1.0, expected)
        return True
    return False

def apply_pow(base, exponent, op):
    if op == 0:
        return base ** exponent
    if op == 1:
        return pow(base, exponent)
    return operator.pow(base, exponent)

INF = float('inf')
NAN = float('nan')
nan_cases = [
    (-INF, NAN),
    (-2.0, NAN),
    (-1.0, NAN),
    (-0.5, NAN),
    (-0.0, NAN),
    (0.0, NAN),
    (0.5, NAN),
    (2.0, NAN),
    (INF, NAN),
    (NAN, NAN),
    (NAN, -INF),
    (NAN, -2.0),
    (NAN, -1.0),
    (NAN, -0.5),
    (NAN, 0.5),
    (NAN, 1.0),
    (NAN, 2.0),
    (NAN, INF),
]
float_cases = [
    (-0.0, 1.0, -0.0),
    (0.0, 1.0, 0.0),
    (-0.0, 0.5, 0.0),
    (-0.0, 2.0, 0.0),
    (0.0, 0.5, 0.0),
    (0.0, 2.0, 0.0),
    (-1.0, -INF, 1.0),
    (-1.0, INF, 1.0),
    (1.0, -INF, 1.0),
    (1.0, -2.0, 1.0),
    (1.0, -1.0, 1.0),
    (1.0, -0.5, 1.0),
    (1.0, -0.0, 1.0),
    (1.0, 0.0, 1.0),
    (1.0, 0.5, 1.0),
    (1.0, 1.0, 1.0),
    (1.0, 2.0, 1.0),
    (1.0, INF, 1.0),
    (1.0, NAN, 1.0),
    (-INF, 0.0, 1.0),
    (-2.0, 0.0, 1.0),
    (-1.0, 0.0, 1.0),
    (-0.5, 0.0, 1.0),
    (-0.0, 0.0, 1.0),
    (0.0, 0.0, 1.0),
    (0.5, 0.0, 1.0),
    (1.0, 0.0, 1.0),
    (2.0, 0.0, 1.0),
    (INF, 0.0, 1.0),
    (NAN, 0.0, 1.0),
    (-INF, -0.0, 1.0),
    (-2.0, -0.0, 1.0),
    (-1.0, -0.0, 1.0),
    (-0.5, -0.0, 1.0),
    (-0.0, -0.0, 1.0),
    (0.0, -0.0, 1.0),
    (0.5, -0.0, 1.0),
    (1.0, -0.0, 1.0),
    (2.0, -0.0, 1.0),
    (INF, -0.0, 1.0),
    (NAN, -0.0, 1.0),
    (-0.5, -INF, INF),
    (-0.0, -INF, INF),
    (0.0, -INF, INF),
    (0.5, -INF, INF),
    (-INF, -INF, 0.0),
    (-2.0, -INF, 0.0),
    (2.0, -INF, 0.0),
    (INF, -INF, 0.0),
    (-0.5, INF, 0.0),
    (-0.0, INF, 0.0),
    (0.0, INF, 0.0),
    (0.5, INF, 0.0),
    (-INF, INF, INF),
    (-2.0, INF, INF),
    (2.0, INF, INF),
    (INF, INF, INF),
    (-INF, -1.0, -0.0),
    (-INF, -0.5, 0.0),
    (-INF, -2.0, 0.0),
    (-INF, 1.0, -INF),
    (-INF, 0.5, INF),
    (-INF, 2.0, INF),
    (INF, 0.5, INF),
    (INF, 1.0, INF),
    (INF, 2.0, INF),
    (INF, -2.0, 0.0),
    (INF, -1.0, 0.0),
    (INF, -0.5, 0.0),
    (-2.0, -2.0, 0.25),
    (-2.0, -1.0, -0.5),
    (-2.0, -0.0, 1.0),
    (-2.0, 0.0, 1.0),
    (-2.0, 1.0, -2.0),
    (-2.0, 2.0, 4.0),
    (-1.0, -2.0, 1.0),
    (-1.0, -1.0, -1.0),
    (-1.0, -0.0, 1.0),
    (-1.0, 0.0, 1.0),
    (-1.0, 1.0, -1.0),
    (-1.0, 2.0, 1.0),
    (2.0, -2.0, 0.25),
    (2.0, -1.0, 0.5),
    (2.0, -0.0, 1.0),
    (2.0, 0.0, 1.0),
    (2.0, 1.0, 2.0),
    (2.0, 2.0, 4.0),
    (1.0, -1e100, 1.0),
    (1.0, 1e100, 1.0),
    (-1.0, -1e100, 1.0),
    (-1.0, 1e100, 1.0),
    (-2.0, -2000.0, 0.0),
    (-2.0, -2001.0, -0.0),
    (2.0, -2000.0, 0.0),
    (2.0, -2000.5, 0.0),
    (2.0, -2001.0, 0.0),
    (-0.5, 2000.0, 0.0),
    (-0.5, 2001.0, -0.0),
    (0.5, 2000.0, 0.0),
    (0.5, 2000.5, 0.0),
    (0.5, 2001.0, 0.0),
]
zero_error_cases = [
    (-0.0, -1.0),
    (0.0, -1.0),
    (-0.0, -2.0),
    (-0.0, -0.5),
    (0.0, -2.0),
    (0.0, -0.5),
]
complex_type_cases = [
    (-2.0, -0.5),
    (-2.0, 0.5),
    (-1.0, -0.5),
    (-1.0, 0.5),
    (-0.5, -0.5),
    (-0.5, 0.5),
    (-2.0, -2000.5),
    (-0.5, 2000.5),
]
complex_value_cases = [
    (-2.0, 0.5, complex(0.0, 1.4142135623730951)),
    (-2.0, -0.5, complex(0.0, -0.7071067811865476)),
]

ok_nan = True
nan_checked = 0
for base, exponent in nan_cases:
    for op in range(3):
        result = apply_pow(base, exponent, op)
        nan_checked += 1
        ok_nan = ok_nan and type(result).__name__ == 'float' and math.isnan(result)
print('pow-nan-values', ok_nan, len(nan_cases), nan_checked)

ok_float = True
float_checked = 0
for base, exponent, expected in float_cases:
    for op in range(3):
        result = apply_pow(base, exponent, op)
        float_checked += 1
        ok_float = ok_float and same_float(result, expected)
print('pow-float-values', ok_float, len(float_cases), float_checked)

zero_errors = 0
for base, exponent in zero_error_cases:
    for op in range(3):
        try:
            apply_pow(base, exponent, op)
        except ZeroDivisionError:
            zero_errors += 1
print('pow-zero-errors', zero_errors == len(zero_error_cases) * 3, zero_errors)

ok_complex_type = True
complex_type_checked = 0
for base, exponent in complex_type_cases:
    for op in range(3):
        result = apply_pow(base, exponent, op)
        complex_type_checked += 1
        ok_complex_type = ok_complex_type and type(result) is complex
print('pow-complex-types', ok_complex_type, len(complex_type_cases), complex_type_checked)

ok_complex_value = True
complex_value_checked = 0
for base, exponent, expected in complex_value_cases:
    for op in range(3):
        result = apply_pow(base, exponent, op)
        complex_value_checked += 1
        ok_complex_value = ok_complex_value and type(result) is complex and abs(result - expected) < 1e-12
print('pow-complex-values', ok_complex_value, len(complex_value_cases), complex_value_checked)"#,
    });
}

// Differential source-encoding tests adapted from
// Lib/test/test_source_encoding.py. These execute CPython from an actual bytes
// file so PEP 263 detection, BOM stripping, and codec decoding are tested
// through the same entry point MiniPython exposes as `run_source_bytes()`.
#[test]
fn cpython_bytes_source_output_parity_subset() {
    for case in [
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_compilestring",
            name: "utf8-cookie-byte-source",
            source: b"\n# coding: utf-8\nu = '\xc3\xb3'\nprint(u)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_default_coding",
            name: "default-utf8-source-coding",
            source: b"print(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_first_coding_line",
            name: "iso8859-15-first-line-cookie",
            source: b"#coding:iso8859-15\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_second_coding_line",
            name: "iso8859-15-second-line-cookie",
            source: b"#!/usr/bin/python\n#coding:iso8859-15\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_second_coding_line_empty_first_line",
            name: "iso8859-15-second-line-cookie-empty-first-line",
            source: b"\n#coding:iso8859-15\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_third_coding_line",
            name: "third-line-cookie-ignored",
            source: b"#!/usr/bin/python\n#\n#coding:iso8859-15\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_double_coding_line",
            name: "double-coding-line-first-cookie-wins",
            source: b"#coding:iso8859-15\n#coding:latin1\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_double_coding_same_line",
            name: "double-coding-same-line-first-cookie-wins",
            source: b"#coding:iso8859-15 coding:latin1\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_double_coding_utf8",
            name: "double-coding-utf8-first-cookie-wins",
            source: b"#coding:utf-8\n#coding:latin1\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_first_non_utf8_coding_line",
            name: "iso8859-15-first-line-cookie-non-utf8-comment-byte",
            source: b"#coding:iso-8859-15 \xa4\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_second_non_utf8_coding_line",
            name: "iso8859-15-second-line-cookie-non-utf8-comment-byte",
            source: b"#!/usr/bin/python\n#coding:iso-8859-15 \xa4\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_utf8_bom",
            name: "utf8-bom-default-source",
            source: b"\xef\xbb\xbfprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_utf8_bom_utf8_comments",
            name: "utf8-bom-with-utf8-comment-lines",
            source: b"\xef\xbb\xbf#\xc3\xa4\n#\xc3\xa4\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_utf8_bom_and_utf8_coding_line",
            name: "utf8-bom-and-utf8-cookie",
            source: b"\xef\xbb\xbf#coding:utf-8\nprint(ascii(\"\xc3\xa4\"))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_error_message",
            name: "utf8-bom-empty-source-line",
            source: b"\xef\xbb\xbf\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_exec_valid_coding",
            name: "cp949-source-cookie",
            source: b"# coding: cp949\na = \"\xaa\xa7\"\nprint(a)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_issue2301",
            name: "cp932-source-cookie",
            source: b"# coding: cp932\nvalue = \"\x94\x4e\"\nprint(value)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_file_parse",
            name: "cp1252-source-cookie",
            source: b"# coding: cp1252\nprint(ascii(\"\x80\"))\n",
        },
        BytesDiffCase {
            origin: "CPython source-encoding codec fallback coverage",
            name: "cp1251-source-cookie",
            source: b"# coding: cp1251\nvalue = \"\xcf\"\nprint(ascii(value))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_import_encoded_module via encoded_modules/module_iso_8859_1.py",
            name: "encoded-module-iso-8859-1",
            source: b"# test iso-8859-1 encoding\n# -*- encoding: iso-8859-1 -*-\ntest = (\"Les hommes ont oubli\xe9 cette v\xe9rit\xe9, \"\n        \"dit le renard. Mais tu ne dois pas l'oublier. Tu deviens \"\n        \"responsable pour toujours de ce que tu as apprivois\xe9.\")\nprint(ascii(test))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_import_encoded_module via encoded_modules/module_koi8_r.py",
            name: "encoded-module-koi8-r",
            source: b"# test koi8-r encoding\n# -*- encoding: koi8-r  -*-\ntest = \"\xf0\xcf\xda\xce\xc1\xce\xc9\xc5 \xc2\xc5\xd3\xcb\xcf\xce\xc5\xde\xce\xcf\xd3\xd4\xc9 \xd4\xd2\xc5\xc2\xd5\xc5\xd4 \xc2\xc5\xd3\xcb\xcf\xce\xc5\xde\xce\xcf\xc7\xcf \xd7\xd2\xc5\xcd\xc5\xce\xc9.\"\nprint(ascii(test))\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_20731 via tokenizedata/coding20731.py",
            name: "tokenizedata-coding20731-latin1-crlf",
            source: b"#coding:latin1\r\n\r\n\r\n\r\n",
        },
    ] {
        assert_cpython_bytes_output_parity(&case);
    }
}

// Current CPython also checks source-encoding behavior through `exec(bytes)`.
// These long-line cases are closer to MiniPython's `run_source_bytes()` entry
// point than the host Python 3.9 file reader, whose long-line behavior predates
// the current CPython tests.
#[test]
fn cpython_bytes_exec_source_output_parity_subset() {
    const BUFSIZ: usize = 8192;

    let mut source = b"#".to_vec();
    source.extend(std::iter::repeat(b' ').take(BUFSIZ));
    source.extend_from_slice(b"coding:iso8859-15\nprint(ascii(\"\xc3\xa4\"))\n");
    assert_cpython_owned_bytes_exec_output_parity(
        "Lib/test/test_source_encoding.py::test_long_first_coding_line",
        "long-first-coding-line",
        source,
    );

    let mut source = b"#!/usr/bin/python\n#".to_vec();
    source.extend(std::iter::repeat(b' ').take(BUFSIZ));
    source.extend_from_slice(b"coding:iso8859-15\nprint(ascii(\"\xc3\xa4\"))\n");
    assert_cpython_owned_bytes_exec_output_parity(
        "Lib/test/test_source_encoding.py::test_long_second_coding_line",
        "long-second-coding-line",
        source,
    );

    let mut source = b"#coding:iso-8859-15".to_vec();
    source.extend(std::iter::repeat(b' ').take(BUFSIZ));
    source.extend_from_slice(b"\nprint(ascii(\"\xc3\xa4\"))\n");
    assert_cpython_owned_bytes_exec_output_parity(
        "Lib/test/test_source_encoding.py::test_long_coding_line",
        "long-coding-line",
        source,
    );

    let mut source = b"#coding:iso-8859-1-".to_vec();
    source.extend(std::iter::repeat(b'x').take(BUFSIZ));
    source.extend_from_slice(b"\nprint(ascii(\"\xc3\xa4\"))\n");
    assert_cpython_owned_bytes_exec_output_parity(
        "Lib/test/test_source_encoding.py::test_long_coding_name",
        "long-coding-name",
        source,
    );

    for (name, prefix) in [
        ("long-first-utf8-line-without-space", b"#".as_slice()),
        ("long-first-utf8-line-with-space", b"# ".as_slice()),
        ("long-second-utf8-line-without-space", b"\n#".as_slice()),
        ("long-second-utf8-line-with-space", b"\n# ".as_slice()),
    ] {
        let mut source = prefix.to_vec();
        for _ in 0..(BUFSIZ / 2) {
            source.extend_from_slice(b"\xc3\xa4");
        }
        source.push(b'\n');
        assert_cpython_owned_bytes_exec_output_parity(
            "Lib/test/test_source_encoding.py::AbstractSourceEncodingTest long UTF-8 comment lines",
            name,
            source,
        );
    }
}

// Rejection-side source-encoding parity for bytes files. These cases are
// adapted from Lib/test/test_source_encoding.py and
// Lib/test/test_tokenize.py::TestDetectEncoding. The exact SyntaxError wording
// differs across CPython versions, so this asserts accept/reject parity.
#[test]
fn cpython_bytes_source_rejection_parity_subset() {
    for case in [
        BytesDiffCase {
            origin: "Lib/test/test_tokenize.py::TestDetectEncoding::test_short_files",
            name: "unknown-coding-cookie",
            source: b"# coding: bad\nprint(1)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_bad_coding via tokenizedata/bad_coding.py",
            name: "tokenizedata-bad-coding-uft8",
            source: b"# -*- coding: uft-8 -*-\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_bad_coding2 via tokenizedata/bad_coding2.py",
            name: "tokenizedata-bad-coding2-utf8-bom-mismatch",
            source: b"\xef\xbb\xbf#coding: utf8\nprint('\xe6\x88\x91')\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_error_message",
            name: "utf8-bom-non-utf8-cookie",
            source: b"\xef\xbb\xbf# -*- coding: iso-8859-15 -*-\nprint(1)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_utf8_bom_and_non_utf8_second_coding_line",
            name: "utf8-bom-second-line-non-utf8-cookie",
            source: b"\xef\xbb\xbf#first\n#coding:iso-8859-15\nprint(1)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_error_message",
            name: "utf8-bom-fake-cookie",
            source: b"\xef\xbb\xbf# -*- coding: fake -*-\nprint(1)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_issue7820",
            name: "partial-utf16-le-bom-one-byte",
            source: b"\xff ",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_issue7820",
            name: "partial-utf8-bom-one-byte",
            source: b"\xef ",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_issue7820",
            name: "partial-utf8-bom-two-bytes",
            source: b"\xef\xbb ",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_error_from_string",
            name: "ascii-cookie-non-ascii-source-body",
            source: b"# coding: ascii\nprint('\xdf')\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_non_utf8_third_line_error",
            name: "default-utf8-non-utf8-third-line",
            source: b"#first\n#second\n#third\xa4\nprint(1)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_source_encoding.py::test_non_utf8_second_line_error",
            name: "default-utf8-non-utf8-second-line",
            source: b"#first\n#second\xa4\nprint(1)\n",
        },
        BytesDiffCase {
            origin: "Lib/test/test_tokenize.py::test_invalid_character_in_fstring_middle",
            name: "invalid-default-utf8-fstring-middle-byte",
            source: b"F\"\"\"\n        \xe5\"\"\"",
        },
        BytesDiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis",
            name: "latin-cookie-unclosed-parenthesis-byte-source",
            source: b"# coding=latin\n(aaaaaaaaaaaaaaaaa\naaaaaaaaaaa\xb5",
        },
        BytesDiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_invisible_characters",
            name: "bytes-source-invalid-non-printable-character",
            source: b"with(0,,):\n\x01",
        },
    ] {
        assert_cpython_bytes_rejection_parity(&case);
    }
}

// Differential rejection tests keep the migration honest without depending on
// CPython's exact SyntaxError wording or MiniPython's internal error phase.
#[test]
fn cpython_rejection_parity_smoke_subset() {
    for case in [
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_if",
            name: "missing-if-colon",
            source: "if True\n    print(1)",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_if",
            name: "missing-if-indent",
            source: "if True:\nprint(1)",
        },
        DiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxTestCase::test_syntax_error_for_function_parameter_with_default",
            name: "function-default-before-non-default",
            source: "def f(a=1, b):\n    pass",
        },
        DiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxTestCase::test_syntax_error_for_function_parameter_with_default",
            name: "lambda-default-before-non-default",
            source: "lambda a=1, b: 0",
        },
        DiffCase {
            origin: "Lib/test/test_syntax.py module doctest invalid assignment target",
            name: "invalid-assignment-target",
            source: "x + 1 = 2",
        },
        DiffCase {
            origin: "Lib/test/test_syntax.py::SyntaxTestCase::test_error_for_assignment_to_conditional_expression",
            name: "invalid-for-target",
            source: "for x + 1 in [1]:\n    pass",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_float_exponent_tokenization",
            name: "grammar-token-float-exponent-tokenization-uppercase-else",
            source: "0 if 1Else 0",
        },
        DiffCase {
            origin: "Lib/test/test_augassign.py::test_with_unpacking",
            name: "augassign-unpacking-target",
            source: "x, b += 3",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_listcomps",
            name: "invalid-listcomp-leading-expression-list",
            source: "[i, s for i in [1] for s in [2]]",
        },
        DiffCase {
            origin: "Lib/test/test_grammar.py::test_listcomps",
            name: "invalid-incomplete-conditional-expression",
            source: "[x if y]",
        },
        DiffCase {
            origin: "Lib/test/test_tokenize.py TokenError/ERRORTOKEN coverage",
            name: "invalid-euro-character-token",
            source: "€",
        },
        DiffCase {
            origin: "Lib/test/test_tokenize.py TokenError/ERRORTOKEN coverage",
            name: "invalid-unmatched-right-bracket-token",
            source: "]",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py syntax error coverage",
            name: "invalid-f-string-post-expression-semicolon",
            source: "f'{x;y}'",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py syntax error coverage",
            name: "invalid-f-string-nested-format-spec-expression",
            source: "f'{x:{;}}'",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py conversion syntax error coverage",
            name: "invalid-f-string-missing-conversion-before-format",
            source: "f'{x!:}'",
        },
        DiffCase {
            origin: "Lib/test/test_fstring.py::test_missing_variable",
            name: "f-string-missing-variable",
            source: "f'v:{value}'",
        },
        DiffCase {
            origin: "Lib/test/test_format.py grouping option error coverage",
            name: "format-rejects-duplicate-comma-grouping",
            source: "format(1, ',,')",
        },
        DiffCase {
            origin: "Lib/test/test_format.py grouping option error coverage",
            name: "format-rejects-duplicate-underscore-grouping",
            source: "format(1, '__')",
        },
        DiffCase {
            origin: "Lib/test/test_format.py grouping option error coverage",
            name: "format-rejects-comma-underscore-grouping",
            source: "format(1, ',_')",
        },
        DiffCase {
            origin: "Lib/test/test_format.py grouping option error coverage",
            name: "format-rejects-float-comma-underscore-grouping",
            source: "format(1.1, '.,_f')",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-stray-percent",
            source: "'abc %' % ()",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-space-percent",
            source: "'abc % %s' % 1",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-unsupported-z",
            source: "'abc %z' % 1",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-unsupported-I",
            source: "'abc %Id' % 1",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-quote-flag",
            source: r#""abc %'d" % 1"#,
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-space-after-width",
            source: "'abc %1 d' % 1",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-bad-mapping-key-shape",
            source: "'abc % (x)r' % {}",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-incomplete-mapping-key",
            source: "'abc %((x)r' % {}",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-not-enough-args",
            source: "'%r %r' % 1",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-not-enough-star-width",
            source: "'%r %*r' % (1,)",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-star-width-non-int",
            source: "'%*r' % (3.14, 1)",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-star-precision-non-int",
            source: "'%.*r' % (3.14, 1)",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-d-rejects-str",
            source: "'%d' % '1'",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-x-rejects-float",
            source: "'%x' % 3.14",
        },
        DiffCase {
            origin: "Lib/test/test_format.py::test_common_format old-style error paths",
            name: "old-style-percent-g-rejects-str",
            source: "'%g' % '1'",
        },
        DiffCase {
            origin: "Lib/test/test_descr.py C3 MRO conflict rejection",
            name: "c3-mro-conflict",
            source: "class A:\n    pass\nclass B(A):\n    pass\nclass C(A):\n    pass\nclass D(B, C):\n    pass\nclass E(C, B):\n    pass\nclass F(D, E):\n    pass",
        },
        DiffCase {
            origin: "Lib/test/test_descr.py duplicate base rejection",
            name: "duplicate-base-class",
            source: "class A:\n    pass\nclass B(A, A):\n    pass",
        },
        DiffCase {
            origin: "Lib/test/test_descr.py __slots__ rejection subset",
            name: "slots-rejects-non-string-item",
            source: "class Bad:\n    __slots__ = (1,)",
        },
        DiffCase {
            origin: "Lib/test/test_descr.py __slots__ rejection subset",
            name: "slots-rejects-non-identifier",
            source: "class Bad:\n    __slots__ = ('not valid',)",
        },
        DiffCase {
            origin: "Lib/test/test_descr.py __slots__ rejection subset",
            name: "slots-rejects-duplicate-dict-slot",
            source: "class Bad:\n    __slots__ = ('__dict__', '__dict__')",
        },
        DiffCase {
            origin: "Lib/test/test_descr.py __slots__ rejection subset",
            name: "slots-rejects-inherited-dict-slot",
            source: "class Base:\n    pass\nclass Bad(Base):\n    __slots__ = ('__dict__',)",
        },
        DiffCase {
            origin: "Lib/test/test_descr.py __slots__ rejection subset",
            name: "slots-rejects-class-variable-conflict",
            source: "class Bad:\n    __slots__ = ('x',)\n    x = 1",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestOnlySetsInBinaryOps::test_union",
            name: "set-union-operator-rejects-non-set",
            source: "{1} | [1]",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestOnlySetsInBinaryOps::test_intersection",
            name: "set-intersection-operator-rejects-non-set",
            source: "{1} & [1]",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestOnlySetsInBinaryOps::test_sym_difference",
            name: "set-symmetric-difference-operator-rejects-non-set",
            source: "{1} ^ [1]",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestOnlySetsInBinaryOps::test_difference",
            name: "set-difference-operator-rejects-non-set",
            source: "{1} - [1]",
        },
        DiffCase {
            origin: "Lib/test/test_set.py::TestOnlySetsInBinaryOps::test_ge_gt_le_lt",
            name: "set-ordering-rejects-non-set",
            source: "{1} < [1]",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py dict values view is not set-like",
            name: "dict-values-view-rejects-set-comparison",
            source: "{1: 1}.values() <= {1: 1}.values()",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration",
            name: "dict-iterator-rejects-size-growth",
            source: "d = {1: 1}\nfor key in d:\n    d[key + 1] = 1",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration_delete",
            name: "dict-iterator-rejects-delete-and-reinsert",
            source: "d = {0: 0}\nfor key in d:\n    del d[0]\n    d[0] = 0",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration_delete_over_values",
            name: "dict-values-iterator-rejects-delete-and-reinsert",
            source: "d = {0: 0}\nfor value in d.values():\n    del d[0]\n    d[0] = 0",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration_delete_over_items",
            name: "dict-items-iterator-rejects-delete-and-reinsert",
            source: "d = {0: 0}\nfor item in d.items():\n    del d[0]\n    d[0] = 0",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py reversed rejects unordered inputs",
            name: "reversed-rejects-set",
            source: "reversed({1})",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py reversed custom protocol rejects non-iterable result on consumption",
            name: "reversed-custom-dunder-non-iterable-result",
            source: "class BadReverse:\n    def __reversed__(self):\n        return 42\nlist(reversed(BadReverse()))",
        },
        DiffCase {
            origin: "Lib/test/test_tuple.py::TupleTest::test_keyword_args",
            name: "tuple-rejects-keyword-args",
            source: "tuple(sequence=())",
        },
        DiffCase {
            origin: "Lib/test/test_bool.py keyword arg rejection",
            name: "bool-rejects-keyword-args",
            source: "bool(x=10)",
        },
        DiffCase {
            origin: "Lib/test/test_bool.py custom __bool__ return type rejection",
            name: "bool-rejects-non-bool-dunder-bool",
            source: "class BadBool:\n    def __bool__(self):\n        return 1\nbool(BadBool())",
        },
        DiffCase {
            origin: "Lib/test/test_bool.py custom __len__ negative rejection",
            name: "bool-rejects-negative-dunder-len",
            source: "class BadLen:\n    def __len__(self):\n        return -1\nbool(BadLen())",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py len custom __len__ return type rejection",
            name: "len-rejects-non-int-dunder-len",
            source: "class BadLen:\n    def __len__(self):\n        return 'many'\nlen(BadLen())",
        },
        DiffCase {
            origin: "Lib/test/test_float.py::GeneralFloatCases::test_floatconversion keyword arg rejection",
            name: "float-rejects-keyword-args",
            source: "float(x='3.14')",
        },
        DiffCase {
            origin: "Lib/test/test_float.py invalid literal rejection",
            name: "float-rejects-invalid-string",
            source: "float('not-a-float')",
        },
        DiffCase {
            origin: "Lib/test/test_int.py invalid literal rejection",
            name: "int-rejects-invalid-string",
            source: "int('not-an-int')",
        },
        DiffCase {
            origin: "Lib/test/test_int.py explicit base rejects non-string input",
            name: "int-base-rejects-non-string",
            source: "int(10, 2)",
        },
        DiffCase {
            origin: "Lib/test/test_int.py explicit base range rejection",
            name: "int-base-rejects-out-of-range-base",
            source: "int('10', 1)",
        },
        DiffCase {
            origin: "Lib/test/test_int.py custom __int__ return type rejection",
            name: "int-rejects-non-int-dunder-int",
            source: "class BadInt:\n    def __int__(self):\n        return 1.2\nint(BadInt())",
        },
        DiffCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_intconversion missing methods",
            name: "int-rejects-object-without-numeric-conversion-methods",
            source: "class MissingMethods:\n    pass\nint(MissingMethods())",
        },
        DiffCase {
            origin: "Lib/test/test_float.py custom __float__ return type rejection",
            name: "float-rejects-non-float-dunder-float",
            source: "class BadFloat:\n    def __float__(self):\n        return 1\nfloat(BadFloat())",
        },
        DiffCase {
            origin: "Lib/test/test_index.py custom __index__ return type rejection",
            name: "range-rejects-non-int-dunder-index",
            source: "class BadIndex:\n    def __index__(self):\n        return 1.2\nrange(BadIndex())",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_iter rejects non-iterable",
            name: "iter-rejects-non-iterable",
            source: "iter(1)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_iter rejects non-callable sentinel source",
            name: "iter-sentinel-rejects-non-callable",
            source: "iter(1, 2)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_iter rejects too many arguments",
            name: "iter-rejects-too-many-arguments",
            source: "iter(lambda: 1, 2, 3)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_next rejects non-iterator",
            name: "next-rejects-non-iterator",
            source: "next([1])",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py direct iterator special method arity",
            name: "iter-dunder-rejects-extra-argument",
            source: "[1].__iter__(0)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py direct iterator special method arity",
            name: "next-dunder-rejects-extra-argument",
            source: "iter([1]).__next__(99)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py direct iterator special method lookup",
            name: "list-rejects-next-dunder",
            source: "[1].__next__()",
        },
        DiffCase {
            origin: "Lib/test/test_iter.py iterator __length_hint__ unsupported iterator types",
            name: "enumerate-rejects-length-hint-dunder",
            source: "enumerate([1]).__length_hint__()",
        },
        DiffCase {
            origin: "Lib/test/test_iter.py iterator __length_hint__ unsupported iterator types",
            name: "zip-rejects-length-hint-dunder",
            source: "zip([1]).__length_hint__()",
        },
        DiffCase {
            origin: "Lib/test/test_iter.py iterator __length_hint__ arity",
            name: "length-hint-dunder-rejects-extra-argument",
            source: "iter([1]).__length_hint__(0)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_all rejects non-iterable",
            name: "all-rejects-non-iterable",
            source: "all(10)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_all rejects missing argument",
            name: "all-rejects-missing-argument",
            source: "all()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_all rejects too many arguments",
            name: "all-rejects-too-many-arguments",
            source: "all([2, 4, 6], [])",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_any rejects non-iterable",
            name: "any-rejects-non-iterable",
            source: "any(10)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_any rejects missing argument",
            name: "any-rejects-missing-argument",
            source: "any()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_any rejects too many arguments",
            name: "any-rejects-too-many-arguments",
            source: "any([2, 4, 6], [])",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase::test_argumentcheck",
            name: "enumerate-rejects-missing-iterable",
            source: "enumerate()",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase::test_argumentcheck",
            name: "enumerate-rejects-non-iterable",
            source: "enumerate(1)",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase::test_argumentcheck",
            name: "enumerate-rejects-non-integer-start",
            source: "enumerate('abc', 'a')",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase::test_argumentcheck",
            name: "enumerate-rejects-too-many-arguments",
            source: "enumerate('abc', 2, 3)",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase::test_kwargs",
            name: "enumerate-rejects-unknown-keyword",
            source: "enumerate(iterable=[], x=3)",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase::test_noniterable",
            name: "enumerate-rejects-next-only-object",
            source: "class X:\n    def __next__(self):\n        return 1\nenumerate(X())",
        },
        DiffCase {
            origin: "Lib/test/test_enumerate.py::EnumerateTestCase::test_illformediterable",
            name: "enumerate-rejects-iterator-without-next",
            source: "class N:\n    def __iter__(self):\n        return self\nenumerate(N())",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_zip rejects non-iterable",
            name: "zip-rejects-non-iterable",
            source: "zip(None)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_zip rejects non-iterable argument",
            name: "zip-rejects-later-non-iterable",
            source: "zip((1, 2), None)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_map arity errors",
            name: "map-rejects-missing-arguments",
            source: "map()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_map arity errors",
            name: "map-rejects-missing-iterable",
            source: "map(lambda x: x)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_map rejects non-iterable",
            name: "map-rejects-non-iterable",
            source: "map(lambda x: x, 42)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_map callable errors during iteration",
            name: "map-rejects-non-callable-function-when-iterated",
            source: "list(map(None, [1]))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_filter arity errors",
            name: "filter-rejects-missing-arguments",
            source: "filter()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_filter arity errors",
            name: "filter-rejects-missing-iterable",
            source: "filter(None)",
        },
        DiffCase {
            origin: "Lib/test/test_iter.py::TestCase::test_builtin_filter rejects non-iterable",
            name: "filter-rejects-non-iterable",
            source: "filter(None, 42)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_filter callable errors during iteration",
            name: "filter-rejects-non-callable-function-when-iterated",
            source: "list(filter(42, [1]))",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_getattr arity errors",
            name: "getattr-rejects-missing-arguments",
            source: "getattr()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_getattr arity errors",
            name: "getattr-rejects-one-argument",
            source: "getattr(1)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_getattr attribute-name type",
            name: "getattr-rejects-non-string-name",
            source: "getattr(1, 2)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_getattr arity errors",
            name: "getattr-rejects-too-many-arguments",
            source: "getattr(1, 'x', 2, 3)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_hasattr arity errors",
            name: "hasattr-rejects-missing-arguments",
            source: "hasattr()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_hasattr arity errors",
            name: "hasattr-rejects-one-argument",
            source: "hasattr(1)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_hasattr attribute-name type",
            name: "hasattr-rejects-non-string-name",
            source: "hasattr(1, 2)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_setattr arity errors",
            name: "setattr-rejects-missing-arguments",
            source: "setattr()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_setattr arity errors",
            name: "setattr-rejects-two-arguments",
            source: "setattr(1, 'x')",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_setattr attribute-name type",
            name: "setattr-rejects-non-string-name",
            source: "setattr(1, 2, 3)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_setattr rejects immutable builtin object",
            name: "setattr-rejects-non-attribute-object",
            source: "setattr(1, 'x', 2)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_delattr arity errors",
            name: "delattr-rejects-missing-arguments",
            source: "delattr()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_delattr arity errors",
            name: "delattr-rejects-one-argument",
            source: "delattr(1)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_delattr attribute-name type",
            name: "delattr-rejects-non-string-name",
            source: "delattr(1, 2)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_callable arity errors",
            name: "callable-rejects-missing-argument",
            source: "callable()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_callable arity errors",
            name: "callable-rejects-too-many-arguments",
            source: "callable(1, 2)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_abs arity/type errors",
            name: "abs-rejects-missing-argument",
            source: "abs()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_abs arity/type errors",
            name: "abs-rejects-non-number",
            source: "abs('x')",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_min empty iterable",
            name: "min-rejects-empty-sequence",
            source: "min(())",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_min_max default argument errors",
            name: "min-rejects-default-with-multiple-positionals",
            source: "min(1, 2, default=0)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_min_max default argument errors",
            name: "max-rejects-default-with-multiple-positionals",
            source: "max(1, 2, default=0)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_min empty iterable with key",
            name: "min-rejects-empty-sequence-with-key",
            source: "min([], key=abs)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py min invalid keyword rejection",
            name: "min-rejects-unknown-keyword",
            source: "min([1], unknown=2)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestSorted::test_bad_arguments",
            name: "sorted-rejects-missing-argument",
            source: "sorted()",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestSorted::test_bad_arguments",
            name: "sorted-rejects-positional-only-keyword",
            source: "sorted(iterable=[])",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestSorted::test_bad_arguments",
            name: "sorted-rejects-positional-key",
            source: "sorted([], None)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestSorted::test_bad_arguments",
            name: "sorted-rejects-unknown-keyword",
            source: "sorted([1], bad=2)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::TestSorted reverse argument validation",
            name: "sorted-rejects-non-integer-reverse",
            source: "sorted([1], reverse=[])",
        },
        DiffCase {
            origin: "Lib/test/test_sort.py::TestBase::test_not_all_tuples",
            name: "sorted-rejects-incomparable-items",
            source: "sorted([1, 'a'])",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_reverse",
            name: "list-reverse-rejects-positional-argument",
            source: "u = [1, 2]\nu.reverse(42)",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_reverse",
            name: "list-reverse-rejects-keyword-argument",
            source: "u = [1, 2]\nu.reverse(x=1)",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_sort",
            name: "list-sort-rejects-positional-argument",
            source: "u = [1, 0]\nu.sort(42)",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_sort",
            name: "list-sort-rejects-unknown-keyword",
            source: "u = [1, 0]\nu.sort(bad=2)",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_sort",
            name: "list-sort-rejects-non-integer-reverse",
            source: "u = [1, 0]\nu.sort(reverse=[])",
        },
        DiffCase {
            origin: "Lib/test/test_sort.py::TestBase::test_not_all_tuples",
            name: "list-sort-rejects-incomparable-items",
            source: "u = [1, 'a']\nu.sort()",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_insert",
            name: "list-insert-rejects-missing-arguments",
            source: "a = [0]\na.insert()",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_insert",
            name: "list-insert-rejects-one-argument",
            source: "a = [0]\na.insert(1)",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_insert",
            name: "list-insert-rejects-too-many-arguments",
            source: "a = [0]\na.insert(1, 2, 3)",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_remove",
            name: "list-remove-rejects-missing-value",
            source: "a = []\na.remove()",
        },
        DiffCase {
            origin: "Lib/test/list_tests.py::CommonTest::test_remove",
            name: "list-remove-rejects-absent-value",
            source: "a = []\na.remove(0)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_count",
            name: "list-count-rejects-missing-value",
            source: "a = [1, 2]\na.count()",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_count",
            name: "list-count-rejects-too-many-arguments",
            source: "a = [1, 2]\na.count(1, 2)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_index",
            name: "list-index-rejects-missing-value",
            source: "a = [1, 2]\na.index()",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_index",
            name: "list-index-rejects-absent-value",
            source: "a = [1, 2]\na.index(3)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_index",
            name: "list-index-rejects-outside-stop",
            source: "a = [1, 2]\na.index(2, 0, 1)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_index",
            name: "list-index-rejects-too-many-arguments",
            source: "a = [1, 2]\na.index(1, 0, 2, 3)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "list-dunder-getitem-rejects-low-index",
            source: "a = [10, 11]\na.__getitem__(-3)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "list-dunder-getitem-rejects-high-index",
            source: "a = [10, 11]\na.__getitem__(3)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "list-dunder-getitem-rejects-non-index",
            source: "a = [10, 11]\na.__getitem__('x')",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "list-dunder-getitem-rejects-zero-step-slice",
            source: "a = [10, 11]\na.__getitem__(slice(0, 10, 0))",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-getitem-rejects-missing-index",
            source: "a = [10, 11]\na.__getitem__()",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-getitem-rejects-too-many-arguments",
            source: "a = [10, 11]\na.__getitem__(0, 1)",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-setitem-rejects-missing-arguments",
            source: "a = [1, 2]\na.__setitem__()",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-setitem-rejects-one-argument",
            source: "a = [1, 2]\na.__setitem__(0)",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-setitem-rejects-too-many-arguments",
            source: "a = [1, 2]\na.__setitem__(0, 3, 4)",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-delitem-rejects-missing-index",
            source: "a = [1, 2]\na.__delitem__()",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-delitem-rejects-too-many-arguments",
            source: "a = [1, 2]\na.__delitem__(0, 1)",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-contains-rejects-missing-value",
            source: "a = [1, 2]\na.__contains__()",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-contains-rejects-too-many-arguments",
            source: "a = [1, 2]\na.__contains__(1, 2)",
        },
        DiffCase {
            origin: "list special method arity",
            name: "list-dunder-len-rejects-argument",
            source: "a = [1, 2]\na.__len__(1)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "tuple-dunder-getitem-rejects-high-index",
            source: "t = (10, 11)\nt.__getitem__(3)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "str-dunder-getitem-rejects-high-index",
            source: "s = 'abc'\ns.__getitem__(3)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "bytes-dunder-getitem-rejects-high-index",
            source: "b = b'abc'\nb.__getitem__(3)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "range-dunder-getitem-rejects-high-index",
            source: "r = range(3)\nr.__getitem__(3)",
        },
        DiffCase {
            origin: "Lib/test/seq_tests.py::CommonTest::test_subscript",
            name: "tuple-dunder-getitem-rejects-non-index",
            source: "t = (10, 11)\nt.__getitem__('x')",
        },
        DiffCase {
            origin: "str special method membership type check",
            name: "str-dunder-contains-rejects-non-string",
            source: "s = 'abc'\ns.__contains__(1)",
        },
        DiffCase {
            origin: "bytes special method membership type check",
            name: "bytes-dunder-contains-rejects-string",
            source: "b = b'abc'\nb.__contains__('a')",
        },
        DiffCase {
            origin: "tuple special method arity",
            name: "tuple-dunder-len-rejects-argument",
            source: "t = (1, 2)\nt.__len__(1)",
        },
        DiffCase {
            origin: "str special method arity",
            name: "str-dunder-getitem-rejects-missing-index",
            source: "s = 'abc'\ns.__getitem__()",
        },
        DiffCase {
            origin: "bytes special method arity",
            name: "bytes-dunder-contains-rejects-missing-value",
            source: "b = b'abc'\nb.__contains__()",
        },
        DiffCase {
            origin: "range special method arity",
            name: "range-dunder-getitem-rejects-too-many-arguments",
            source: "r = range(3)\nr.__getitem__(0, 1)",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py mapping special method errors",
            name: "dict-dunder-getitem-rejects-missing-key",
            source: "d = {1: 2}\nd.__getitem__(3)",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-getitem-rejects-missing-argument",
            source: "d = {1: 2}\nd.__getitem__()",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-getitem-rejects-too-many-arguments",
            source: "d = {1: 2}\nd.__getitem__(1, 2)",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-setitem-rejects-missing-arguments",
            source: "d = {1: 2}\nd.__setitem__()",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-setitem-rejects-one-argument",
            source: "d = {1: 2}\nd.__setitem__(1)",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-setitem-rejects-too-many-arguments",
            source: "d = {1: 2}\nd.__setitem__(1, 2, 3)",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-delitem-rejects-missing-argument",
            source: "d = {1: 2}\nd.__delitem__()",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py mapping special method errors",
            name: "dict-dunder-delitem-rejects-missing-key",
            source: "d = {1: 2}\nd.__delitem__(3)",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-delitem-rejects-too-many-arguments",
            source: "d = {1: 2}\nd.__delitem__(1, 2)",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-contains-rejects-missing-key",
            source: "d = {1: 2}\nd.__contains__()",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-contains-rejects-too-many-arguments",
            source: "d = {1: 2}\nd.__contains__(1, 2)",
        },
        DiffCase {
            origin: "Lib/test/test_dict.py mapping special method errors",
            name: "dict-dunder-contains-rejects-unhashable-key",
            source: "d = {1: 2}\nd.__contains__([])",
        },
        DiffCase {
            origin: "dict special method arity",
            name: "dict-dunder-len-rejects-argument",
            source: "d = {1: 2}\nd.__len__(1)",
        },
        DiffCase {
            origin: "Lib/test/test_set.py set membership errors",
            name: "set-dunder-contains-rejects-unhashable-value",
            source: "s = {1}\ns.__contains__([])",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-contains-rejects-missing-value",
            source: "s = {1}\ns.__contains__()",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-contains-rejects-too-many-arguments",
            source: "s = {1}\ns.__contains__(1, 2)",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-or-rejects-missing-argument",
            source: "s = {1}\ns.__or__()",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-or-rejects-too-many-arguments",
            source: "s = {1}\ns.__or__({2}, {3})",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-and-rejects-missing-argument",
            source: "s = {1}\ns.__and__()",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-sub-rejects-missing-argument",
            source: "s = {1}\ns.__sub__()",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-xor-rejects-missing-argument",
            source: "s = {1}\ns.__xor__()",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-le-rejects-missing-argument",
            source: "s = {1}\ns.__le__()",
        },
        DiffCase {
            origin: "set special method arity",
            name: "set-dunder-len-rejects-argument",
            source: "s = {1}\ns.__len__(1)",
        },
        DiffCase {
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_sum string rejection",
            name: "sum-rejects-strings",
            source: "sum(['a'])",
        },
    ] {
        assert_cpython_rejection_parity(&case);
    }
}

#[test]
fn cpython_syntax_error_message_parity_subset() {
    for case in [
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_expression_with_assignment",
            name: "expression-with-assignment-message",
            source: "print(end1 + end2 = ' ')",
            expected_message: "expression cannot contain assignment, perhaps you meant \"==\"?",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_call",
            name: "assign-call-message",
            source: "f() = 1",
            expected_message: "assign",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-empty-tuple-target-message",
            source: "del (,)",
            expected_message: "invalid syntax",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-literal-message",
            source: "del 1",
            expected_message: "cannot delete literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-tuple-literal-message",
            source: "del (1, 2)",
            expected_message: "cannot delete literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-none-message",
            source: "del None",
            expected_message: "cannot delete None",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-bare-starred-message",
            source: "del *x",
            expected_message: "cannot delete starred",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-parenthesized-starred-message",
            source: "del (*x)",
            expected_message: "cannot use starred expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-starred-tuple-message",
            source: "del (*x,)",
            expected_message: "cannot delete starred",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-starred-list-message",
            source: "del [*x,]",
            expected_message: "cannot delete starred",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-call-message",
            source: "del f()",
            expected_message: "cannot delete function call",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-call-with-args-message",
            source: "del f(a, b)",
            expected_message: "cannot delete function call",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-attribute-call-message",
            source: "del o.f()",
            expected_message: "cannot delete function call",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-subscript-call-message",
            source: "del a[0]()",
            expected_message: "cannot delete function call",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-second-call-message",
            source: "del x, f()",
            expected_message: "cannot delete function call",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-first-call-message",
            source: "del f(), x",
            expected_message: "cannot delete function call",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-nested-list-call-message",
            source: "del [a, b, ((c), (d,), e.f())]",
            expected_message: "cannot delete function call",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-conditional-message",
            source: "del (a if True else b)",
            expected_message: "cannot delete conditional",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-unary-expression-message",
            source: "del +a",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-second-unary-expression-message",
            source: "del a, +b",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-binary-expression-message",
            source: "del a + b",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-tuple-binary-expression-message",
            source: "del (a + b, c)",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-tuple-subscript-binary-expression-message",
            source: "del (c[0], a + b)",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-attribute-binary-expression-message",
            source: "del a.b.c + 2",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-attribute-subscript-binary-expression-message",
            source: "del a.b.c[0] + 2",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-nested-tuple-binary-expression-message",
            source: "del (a, b, (c, d.e.f + 2))",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-nested-list-binary-expression-message",
            source: "del [a, b, (c, d.e.f[0] + 2)]",
            expected_message: "cannot delete expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-named-expression-message",
            source: "del (a := 5)",
            expected_message: "cannot delete named expression",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_assign_del",
            name: "del-augassign-message",
            source: "del a += b",
            expected_message: "invalid syntax",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_global_param_err_first",
            name: "global-param-error-first-message",
            source: "if 1:\n            def error(a):\n                global a  # SyntaxError\n            def error2():\n                b = 1\n                global b  # SyntaxError\n            ",
            expected_message: "parameter and global",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_nonlocal_param_err_first",
            name: "nonlocal-param-error-first-message",
            source: "if 1:\n            def error(a):\n                nonlocal a  # SyntaxError\n            def error2():\n                b = 1\n                global b  # SyntaxError\n            ",
            expected_message: "parameter and nonlocal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_raise_from_error_message",
            name: "raise-from-following-invalid-call-message",
            source: "if 1:\n        raise AssertionError() from None\n        print(1,,2)\n        ",
            expected_message: "invalid syntax",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-if-body-message",
            source: "if 0: yield",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-if-body-with-else-message",
            source: "if 0: yield\nelse:  x=1",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-if-else-message",
            source: "if 1: pass\nelse: yield",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-while-body-message",
            source: "while 0: yield",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-while-body-with-else-message",
            source: "while 0: yield\nelse:  x=1",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-class-if-body-message",
            source: "class C:\n  if 0: yield",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-class-if-else-message",
            source: "class C:\n  if 1: pass\n  else: yield",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-class-while-body-message",
            source: "class C:\n  while 0: yield",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_yield_outside_function",
            name: "yield-outside-class-while-body-with-else-message",
            source: "class C:\n  while 0: yield\n  else:  x = 1",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-if-body-message",
            source: "if 0: return",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-if-body-with-else-message",
            source: "if 0: return\nelse:  x=1",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-if-else-message",
            source: "if 1: pass\nelse: return",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-while-body-message",
            source: "while 0: return",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-class-if-body-message",
            source: "class C:\n  if 0: return",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-class-while-body-message",
            source: "class C:\n  while 0: return",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-class-while-body-with-else-message",
            source: "class C:\n  while 0: return\n  else:  x=1",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-class-if-body-with-else-message",
            source: "class C:\n  if 0: return\n  else: x= 1",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_return_outside_function",
            name: "return-outside-class-if-else-message",
            source: "class C:\n  if 1: pass\n  else: return",
            expected_message: "outside function",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_break_outside_loop",
            name: "break-outside-module-message",
            source: "break",
            expected_message: "outside loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_break_outside_loop",
            name: "break-outside-if-body-message",
            source: "if 0: break",
            expected_message: "outside loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_break_outside_loop",
            name: "break-outside-if-body-with-else-message",
            source: "if 0: break\nelse:  x=1",
            expected_message: "outside loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_break_outside_loop",
            name: "break-outside-if-else-message",
            source: "if 1: pass\nelse: break",
            expected_message: "outside loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_break_outside_loop",
            name: "break-outside-class-if-body-message",
            source: "class C:\n  if 0: break",
            expected_message: "outside loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_break_outside_loop",
            name: "break-outside-class-if-else-message",
            source: "class C:\n  if 1: pass\n  else: break",
            expected_message: "outside loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_break_outside_loop",
            name: "break-outside-with-body-message",
            source: "with object() as obj:\n break",
            expected_message: "outside loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_continue_outside_loop",
            name: "continue-outside-if-body-message",
            source: "if 0: continue",
            expected_message: "not properly in loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_continue_outside_loop",
            name: "continue-outside-if-body-with-else-message",
            source: "if 0: continue\nelse:  x=1",
            expected_message: "not properly in loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_continue_outside_loop",
            name: "continue-outside-if-else-message",
            source: "if 1: pass\nelse: continue",
            expected_message: "not properly in loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_continue_outside_loop",
            name: "continue-outside-class-if-body-message",
            source: "class C:\n  if 0: continue",
            expected_message: "not properly in loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_continue_outside_loop",
            name: "continue-outside-class-if-else-message",
            source: "class C:\n  if 1: pass\n  else: continue",
            expected_message: "not properly in loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_continue_outside_loop",
            name: "continue-outside-with-body-message",
            source: "with object() as obj:\n    continue",
            expected_message: "not properly in loop",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_unexpected_indent",
            name: "syntax-unexpected-indent-message",
            source: "foo()\n bar()\n",
            expected_message: "unexpected indent",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_no_indent",
            name: "syntax-no-indent-message",
            source: "if 1:\nfoo()",
            expected_message: "expected an indented block",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_bad_outdent",
            name: "syntax-bad-outdent-message",
            source: "if 1:\n  foo()\n bar()",
            expected_message: "unindent does not match",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_kwargs_last",
            name: "syntax-kwargs-last-message",
            source: "int(base=10, '2')",
            expected_message: "positional argument follows keyword argument",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_kwargs_last2",
            name: "syntax-kwargs-last-after-double-star-message",
            source: "int(**{'base': 10}, '2')",
            expected_message: "positional argument follows keyword argument unpacking",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_kwargs_last3",
            name: "syntax-star-arg-after-double-star-message",
            source: "int(**{'base': 10}, *['2'])",
            expected_message: "iterable argument unpacking follows keyword argument unpacking",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_generator_in_function_call",
            name: "syntax-generator-in-function-call-message",
            source: "foo(x,    y for y in range(3) for z in range(2) if z    , p)",
            expected_message: "Generator expression must be parenthesized",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_except_then_except_star",
            name: "syntax-except-then-except-star-message",
            source: "try: pass\nexcept ValueError: pass\nexcept* TypeError: pass",
            expected_message: "cannot have both 'except' and 'except*' on the same 'try'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_except_star_then_except",
            name: "syntax-except-star-then-except-message",
            source: "try: pass\nexcept* ValueError: pass\nexcept TypeError: pass",
            expected_message: "cannot have both 'except' and 'except*' on the same 'try'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_barry_as_flufl_with_syntax_errors",
            name: "syntax-barry-as-flufl-expected-colon-message",
            source: "\ndef func1():\n    if a != b:\n        raise ValueError\n\ndef func2():\n    try\n        return 1\n    finally:\n        pass\n",
            expected_message: "expected ':'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_invalid_line_continuation_error_position",
            name: "syntax-invalid-line-continuation-basic-message",
            source: r#"a = 3 \ 4"#,
            expected_message: "unexpected character after line continuation character",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_invalid_line_continuation_error_position",
            name: "syntax-invalid-line-continuation-comment-message",
            source: "1,\\#\n2",
            expected_message: "unexpected character after line continuation character",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_invalid_line_continuation_error_position",
            name: "syntax-invalid-line-continuation-comment-later-line-message",
            source: "\nfgdfgf\n1,\\#\n2\n",
            expected_message: "unexpected character after line continuation character",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_invalid_line_continuation_left_recursive",
            name: "syntax-invalid-line-continuation-left-recursive-space-message",
            source: "A.\u{018a}\\ ",
            expected_message: "unexpected character after line continuation character",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis",
            name: "syntax-open-paren-never-closed-message",
            source: "(1 + 2",
            expected_message: "was never closed",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis",
            name: "syntax-open-bracket-never-closed-message",
            source: "[1 + 2",
            expected_message: "was never closed",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis",
            name: "syntax-open-brace-never-closed-message",
            source: "{1 + 2",
            expected_message: "was never closed",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis",
            name: "syntax-unmatched-right-paren-message",
            source: ")1 + 2",
            expected_message: "unmatched ')'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis",
            name: "syntax-unmatched-right-bracket-message",
            source: "]1 + 2",
            expected_message: "unmatched ']'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis",
            name: "syntax-unmatched-right-brace-message",
            source: "}1 + 2",
            expected_message: "unmatched '}'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis",
            name: "syntax-mismatched-parenthesis-opening-message",
            source: "func(\n    a=[\"unclosed], # Need a quote in this comment: \"\n    b=2,\n)\n",
            expected_message: "does not match opening parenthesis",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_string_literal",
            name: "syntax-single-quote-unterminated-string-message",
            source: "'blech",
            expected_message: "unterminated string literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_string_literal",
            name: "syntax-double-quote-unterminated-string-message",
            source: "\"blech",
            expected_message: "unterminated string literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_string_literal",
            name: "syntax-escaped-end-quote-unterminated-string-message",
            source: "\"blech\\\"",
            expected_message: "perhaps you escaped the end quote",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_string_literal",
            name: "syntax-raw-escaped-end-quote-unterminated-string-message",
            source: "r\"blech\\\"",
            expected_message: "perhaps you escaped the end quote",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_string_literal",
            name: "syntax-single-triple-quote-unterminated-message",
            source: "'''blech",
            expected_message: "unterminated triple-quoted string literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_string_literal",
            name: "syntax-double-triple-quote-unterminated-message",
            source: "\"\"\"blech",
            expected_message: "unterminated triple-quoted string literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_invisible_characters",
            name: "syntax-invalid-non-printable-character-message",
            source: "print\x17(\"Hello\")",
            expected_message: "invalid non-printable character",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_multiline_compiler_error_points_to_the_end",
            name: "syntax-multiline-duplicate-keyword-message",
            source: "call(\na=1,\na=1\n)",
            expected_message: "keyword argument repeated",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_multiline_string_concat_missing_comma_points_to_last_string",
            name: "syntax-multiline-string-concat-missing-comma-message",
            source: "print(\n    \"line1\"\n    \"line2\"\n    \"line3\"\n    x=1\n)",
            expected_message: "Perhaps you forgot a comma",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_except_stmt_invalid_as_expr",
            name: "syntax-except-stmt-invalid-as-expr-message",
            source: "\ntry:\n    pass\nexcept ValueError as obj.attr:\n    pass\n",
            expected_message: "cannot use except statement with attribute",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_match_stmt_invalid_as_expr",
            name: "syntax-match-stmt-invalid-as-expr-message",
            source: "\nmatch 1:\n    case x as obj.attr:\n        ...\n",
            expected_message: "cannot use attribute as pattern target",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_ifexp_body_stmt_else_expression",
            name: "syntax-ifexp-pass-body-statement-message",
            source: "x = pass if 1 else 1",
            expected_message: "expected expression before 'if', but statement is given",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_ifexp_body_stmt_else_expression",
            name: "syntax-ifexp-break-body-statement-message",
            source: "x = break if 1 else 1",
            expected_message: "expected expression before 'if', but statement is given",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_ifexp_body_stmt_else_expression",
            name: "syntax-ifexp-continue-body-statement-message",
            source: "x = continue if 1 else 1",
            expected_message: "expected expression before 'if', but statement is given",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_syntax.py::SyntaxErrorTestCase::test_ifexp_body_stmt_else_stmt",
            name: "syntax-ifexp-statement-body-and-statement-else-message",
            source: "x = continue if 1 else import ast",
            expected_message: "expected expression before 'if', but statement is given",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_eof_error",
            name: "def-header-open-paren-eof-message",
            source: "def foo(",
            expected_message: "was never closed",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_eof_error",
            name: "blank-line-def-header-open-paren-eof-message",
            source: "\ndef foo(",
            expected_message: "was never closed",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_eof_error",
            name: "def-header-open-paren-newline-eof-message",
            source: "def foo(\n",
            expected_message: "was never closed",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "binary-invalid-digit-message",
            source: "0b12",
            expected_message: "invalid digit '2' in binary literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "binary-invalid-digit-after-underscore-message",
            source: "0b1_2",
            expected_message: "invalid digit '2' in binary literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "binary-invalid-first-digit-message",
            source: "0b2",
            expected_message: "invalid digit '2' in binary literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "binary-trailing-underscore-message",
            source: "0b1_",
            expected_message: "invalid binary literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "binary-missing-digits-message",
            source: "0b",
            expected_message: "invalid binary literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "octal-invalid-digit-message",
            source: "0o18",
            expected_message: "invalid digit '8' in octal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "octal-invalid-digit-after-underscore-message",
            source: "0o1_8",
            expected_message: "invalid digit '8' in octal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "octal-invalid-first-digit-message",
            source: "0o8",
            expected_message: "invalid digit '8' in octal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "octal-trailing-underscore-message",
            source: "0o1_",
            expected_message: "invalid octal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "octal-missing-digits-message",
            source: "0o",
            expected_message: "invalid octal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "hex-trailing-underscore-message",
            source: "0x1_",
            expected_message: "invalid hexadecimal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "hex-missing-digits-message",
            source: "0x",
            expected_message: "invalid hexadecimal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "decimal-trailing-underscore-message",
            source: "1_",
            expected_message: "invalid decimal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "leading-zero-decimal-message",
            source: "012",
            expected_message: "leading zeros in decimal integer literals are not permitted; use an 0o prefix for octal integers",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "float-trailing-underscore-message",
            source: "1.2_",
            expected_message: "invalid decimal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "exponent-trailing-underscore-message",
            source: "1e2_",
            expected_message: "invalid decimal literal",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_grammar.py::TokenTests::test_bad_numerical_literals",
            name: "exponent-missing-digits-message",
            source: "1e+",
            expected_message: "invalid decimal literal",
        },
    ] {
        assert_cpython_error_message_parity(&case);
    }
}

#[test]
fn cpython_runtime_error_message_parity_subset() {
    for case in [
        ErrorMessageCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration",
            name: "dict-iterator-size-growth-message",
            source: "d = {1: 1}\nfor key in d:\n    d[key + 1] = 1",
            expected_message: "RuntimeError: dictionary changed size during iteration",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration_delete",
            name: "dict-iterator-keys-changed-message",
            source: "d = {0: 0}\nfor key in d:\n    del d[0]\n    d[0] = 0",
            expected_message: "RuntimeError: dictionary keys changed during iteration",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration_delete_over_values",
            name: "dict-values-iterator-keys-changed-message",
            source: "d = {0: 0}\nfor value in d.values():\n    del d[0]\n    d[0] = 0",
            expected_message: "RuntimeError: dictionary keys changed during iteration",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_dict.py::test_mutating_iteration_delete_over_items",
            name: "dict-items-iterator-keys-changed-message",
            source: "d = {0: 0}\nfor item in d.items():\n    del d[0]\n    d[0] = 0",
            expected_message: "RuntimeError: dictionary keys changed during iteration",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_dict.py::test_reverse_iterator_for_empty_dict",
            name: "dict-reverse-iterator-size-growth-message",
            source: "d = {1: 1}\nfor key in reversed(d):\n    d[key + 1] = 1",
            expected_message: "RuntimeError: dictionary changed size during iteration",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py int constructor argument clinic errors",
            name: "int-base-rejects-too-many-positional-args-message",
            source: "int('10', 2, 3)",
            expected_message: "TypeError: int() takes at most 2 arguments (3 given)",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py int constructor argument clinic errors",
            name: "int-base-rejects-positional-and-keyword-base-message",
            source: "int('10', 2, base=10)",
            expected_message: "TypeError: int() takes at most 2 arguments (3 given)",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py int constructor argument clinic errors",
            name: "int-base-rejects-extra-keyword-after-base-message",
            source: "int('10', base=2, other=3)",
            expected_message: "TypeError: int() takes at most 2 arguments (3 given)",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py invalid literal diagnostics",
            name: "int-rejects-invalid-decimal-literal-message",
            source: "int('not-an-int')",
            expected_message: "ValueError: invalid literal for int() with base 10: 'not-an-int'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_string_float",
            name: "int-rejects-string-float-message",
            source: "int('1.2')",
            expected_message: "ValueError: invalid literal for int() with base 10: '1.2'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message non-ASCII string",
            name: "int-rejects-non-ascii-string-message",
            source: "int('½')",
            expected_message: "ValueError: invalid literal for int() with base 10: '½'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message non-ASCII suffix",
            name: "int-rejects-non-ascii-suffix-message",
            source: "int('123½')",
            expected_message: "ValueError: invalid literal for int() with base 10: '123½'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message embedded whitespace",
            name: "int-rejects-embedded-whitespace-message",
            source: "int('  123 456  ')",
            expected_message: "ValueError: invalid literal for int() with base 10: '  123 456  '",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py invalid literal diagnostics",
            name: "int-base-rejects-invalid-binary-digit-message",
            source: "int('2', 2)",
            expected_message: "ValueError: invalid literal for int() with base 2: '2'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py invalid underscore diagnostics",
            name: "int-rejects-double-underscore-message",
            source: "int('1__0')",
            expected_message: "ValueError: invalid literal for int() with base 10: '1__0'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py base-zero invalid decimal diagnostics",
            name: "int-base-zero-rejects-leading-zero-decimal-message",
            source: "int('010', 0)",
            expected_message: "ValueError: invalid literal for int() with base 0: '010'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py bytes invalid literal diagnostics",
            name: "int-base-rejects-invalid-binary-bytes-message",
            source: "int(b'2', 2)",
            expected_message: "ValueError: invalid literal for int() with base 2: b'2'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_int_base_limits",
            name: "int-base-rejects-huge-positive-base-message",
            source: "int('0', 2 ** 100)",
            expected_message: "ValueError: int() base must be >= 2 and <= 36, or 0",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_int_base_limits",
            name: "int-base-rejects-huge-negative-base-message",
            source: "int('0', -(2 ** 100))",
            expected_message: "ValueError: int() base must be >= 2 and <= 36, or 0",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_int_base_bad_types",
            name: "int-base-rejects-float-base-message",
            source: "int('0', 5.5)",
            expected_message: "TypeError: 'float' object cannot be interpreted as an integer",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_int_base_indexable",
            name: "int-base-rejects-indexable-out-of-range-base-message",
            source: "class MyIndexable:\n    def __index__(self):\n        return 2 ** 100\nint('43', MyIndexable())",
            expected_message: "ValueError: int() base must be >= 2 and <= 36, or 0",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_keyword_args",
            name: "int-base-keyword-without-value-message",
            source: "int(base=10)",
            expected_message: "TypeError: int() missing string argument",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_keyword_args",
            name: "int-rejects-invalid-x-keyword-message",
            source: "int(x=1.2)",
            expected_message: "TypeError: 'x' is an invalid keyword argument for int()",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_unicode invalid base",
            name: "int-unicode-decimal-rejects-out-of-base-digit-message",
            source: "int('١٢٣', 2)",
            expected_message: "ValueError: invalid literal for int() with base 2: '١٢٣'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py unicode numeric rejection",
            name: "int-rejects-non-decimal-unicode-number-message",
            source: "int('²')",
            expected_message: "ValueError: invalid literal for int() with base 10: '²'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message embedded NUL",
            name: "int-rejects-embedded-nul-string-message",
            source: "int('123\\x00')",
            expected_message: "ValueError: invalid literal for int() with base 10: '123\\x00'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message embedded NUL explicit base",
            name: "int-rejects-embedded-nul-string-explicit-base-message",
            source: "int('123\\x00 245', 20)",
            expected_message: "ValueError: invalid literal for int() with base 20: '123\\x00 245'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message embedded NUL explicit base",
            name: "int-rejects-embedded-nul-string-explicit-decimal-base-message",
            source: "int('123\\x00', 10)",
            expected_message: "ValueError: invalid literal for int() with base 10: '123\\x00'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message embedded NUL explicit base",
            name: "int-rejects-embedded-nul-string-base-sixteen-message",
            source: "int('123\\x00 245', 16)",
            expected_message: "ValueError: invalid literal for int() with base 16: '123\\x00 245'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message embedded NUL explicit base",
            name: "int-rejects-embedded-nul-without-space-base-twenty-message",
            source: "int('123\\x00245', 20)",
            expected_message: "ValueError: invalid literal for int() with base 20: '123\\x00245'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message embedded NUL explicit base",
            name: "int-rejects-embedded-nul-without-space-base-sixteen-message",
            source: "int('123\\x00245', 16)",
            expected_message: "ValueError: invalid literal for int() with base 16: '123\\x00245'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message embedded NUL bytes",
            name: "int-rejects-embedded-nul-bytes-message",
            source: "int(b'123\\x00')",
            expected_message: "ValueError: invalid literal for int() with base 10: b'123\\x00'",
        },
        ErrorMessageCase {
            origin: "Lib/test/test_int.py::IntTestCases::test_error_message non-UTF-8 bytes",
            name: "int-rejects-non-utf8-bytes-message",
            source: "int(b'123\\xbd', 10)",
            expected_message: "ValueError: invalid literal for int() with base 10: b'123\\xbd'",
        },
    ] {
        assert_cpython_error_message_parity(&case);
    }
}
