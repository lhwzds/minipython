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
    let cpython_output = run_cpython(case.source).unwrap_or_else(|message| {
        panic!(
            "failed to run CPython for {}::{}\nsource:\n{}\n\n{}",
            case.origin, case.name, case.source, message
        )
    });
    assert!(
        cpython_output.status.success(),
        "expected CPython to accept {}::{}\nsource:\n{}\n\nstderr:\n{}",
        case.origin,
        case.name,
        case.source,
        String::from_utf8_lossy(&cpython_output.stderr)
    );

    let cpython_stdout = String::from_utf8(cpython_output.stdout)
        .unwrap_or_else(|error| panic!("CPython emitted non-UTF-8 output: {error}"));
    let cpython_lines: Vec<String> = cpython_stdout.lines().map(str::to_string).collect();

    assert_eq!(
        run_minipython_source(case.source),
        Ok(cpython_lines),
        "MiniPython output differs from CPython for {}::{}\nsource:\n{}",
        case.origin,
        case.name,
        case.source
    );
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
        Ok(result) => result,
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
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
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
            source: "import sys\nprint(callable(len), callable('a'), callable(callable))\ndef f():\n    pass\nprint(callable(f))\nclass C1:\n    def meth(self):\n        pass\nc = C1()\nprint(callable(C1), callable(c.meth), callable(c))\nc.__call__ = lambda self: 0\nprint(callable(c))\nclass C2:\n    def __call__(self, value):\n        return value + 1\nc2 = C2()\nprint(callable(c2), c2(4))\nc2.__call__ = None\nprint(callable(c2), c2(5))\nsetattr(sys, 'spam', 1)\nprint(getattr(sys, 'spam'), hasattr(sys, 'spam'))\ndelattr(sys, 'spam')\nprint(hasattr(sys, 'spam'), getattr(sys, 'spam', 'missing'))\nclass Box:\n    pass\nbox = Box()\nsetattr(box, 'value', 3)\nprint(getattr(box, 'value'), hasattr(box, 'value'))\nsetattr(Box, 'label', 'box')\nprint(getattr(box, 'label'), getattr(Box, 'label'))\ndelattr(box, 'value')\nprint(hasattr(box, 'value'), getattr(box, 'value', 42))\ntry:\n    print((1).missing)\nexcept AttributeError:\n    print('caught')",
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
class CustomFormat:
    def __format__(self, format_spec):
        return format_spec
print(f'{CustomFormat():abc}')
print(format(CustomFormat(), 'xyz'))
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
try:
    format(b, 's')
except TypeError as error:
    print(error.__class__.__name__, 'B.__format__' in str(error))
try:
    object().__format__(3)
except TypeError as error:
    print(error.__class__.__name__)"#,
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
for expr in [lambda: type('A', [], {}), lambda: type('A', (), []), lambda: type(b'A', (), {}), lambda: type('A\0B', (), {}), lambda: type('A', (None,), {}), lambda: type('A', (bool,), {}), lambda: type('A', (int, str), {})]:
    try:
        expr()
    except (TypeError, ValueError) as error:
        print(error.__class__.__name__)"#,
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
    print(g['x'])"#,
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
    print(g['x'])"#,
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
g = {}
exec('import math\nname = math.__name__', g)
print(g['name'], '__import__' in g['__builtins__'])"#,
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
        return 2 ** 100
class HugeNegativeLen:
    def __len__(self):
        return -(2 ** 100)
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
            origin: "Lib/test/test_builtin.py::BuiltinTest::test_round / ::test_bug_27936",
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
print(pow(0.0, 0), pow(0.0, 1), pow(1.0, 0), pow(1.0, 1))
print(pow(2.0, 10), pow(2.0, 20), pow(-2.0, 3))
print(pow(2, -1), pow(-2, -3))
print(pow(2, 10, 1000), pow(-1, -2, 3), pow(5, 2, 14), pow(2, 3, -5), pow(2, -1, 5))
print(pow(0, exp=0), pow(base=2, exp=4), pow(base=5, exp=2, mod=14), pow(2, 3, None))
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
            source: "print(NotImplemented)\nprint(bool(NotImplemented), NotImplemented is NotImplemented, NotImplemented == NotImplemented)\ns = {1}\nprint(s.__or__([2]), s.__and__([1]), s.__sub__([1]), s.__xor__([1]))\nprint(s.__le__([1]), s.__lt__([1]), s.__ge__([1]), s.__gt__([1]))\nprint(s.__eq__([1]), s.__ne__([1]))",
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
