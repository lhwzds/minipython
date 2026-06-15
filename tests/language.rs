use minipython::{
    SandboxPolicy, VirtualModule, eval_source, parse_func_type_source, run_interactive_source,
    run_source, run_source_with_sandbox_dir, run_source_with_sandbox_dir_and_policy,
    run_source_with_virtual_modules, run_source_with_virtual_modules_and_policy,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const REQUIRED_SANDBOX_STDLIB_MODULES: &[&str] = &[
    "builtins",
    "sys",
    "types",
    "collections",
    "collections.abc",
    "math",
    "math.integer",
    "array",
    "copy",
    "io",
    "operator",
    "functools",
    "itertools",
    "json",
];

const COMPATIBILITY_STDLIB_MODULES: &[&str] = &[
    "_types",
    "_weakref",
    "annotationlib",
    "ast",
    "decimal",
    "dis",
    "enum",
    "fractions",
    "inspect",
    "os",
    "os.path",
    "pickle",
    "re",
    "string",
    "string.templatelib",
    "test",
    "test.typinganndata",
    "test.typinganndata.ann_module",
    "test.typinganndata.ann_module2",
    "test.typinganndata.ann_module3",
    "time",
    "typing",
    "unittest",
    "unittest.mock",
    "warnings",
    "weakref",
];

struct TestSandboxDir {
    path: PathBuf,
}

impl TestSandboxDir {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("minipython-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("failed to create test sandbox directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative_path: &str, contents: &str) {
        let path = self.path.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create test module parent directory");
        }
        fs::write(path, contents).expect("failed to write test module");
    }
}

impl Drop for TestSandboxDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_source_with_stack(source: &str, stack_size: usize) -> Result<Vec<String>, String> {
    let source = source.to_string();
    std::thread::Builder::new()
        .stack_size(stack_size)
        .spawn(move || run_source(&source))
        .expect("failed to spawn MiniPython test thread")
        .join()
        .expect("MiniPython test thread panicked")
}

fn output_lines(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

#[test]
fn prints_number() {
    assert_eq!(run_source("print(123)"), Ok(vec!["123".to_string()]));
}

#[test]
fn runs_prefixed_integer_literals() {
    assert_eq!(
        run_source("print(0xff, 0o377, 0b1001)\nprint(0x_f, 0o_7, 0b_1010)"),
        Ok(vec!["255 255 9".to_string(), "15 7 10".to_string()])
    );
}

#[test]
fn prints_addition() {
    assert_eq!(run_source("print(1 + 2)"), Ok(vec!["3".to_string()]));
}

#[test]
fn evaluates_eval_input_expression() {
    assert_eq!(eval_source("1 + 2\n"), Ok("3".to_string()));
    assert_eq!(eval_source("1, 2,"), Ok("(1, 2)".to_string()));
    assert_eq!(eval_source("# comment\n1"), Ok("1".to_string()));
    assert_eq!(
        eval_source("\"mini\" + \"python\""),
        Ok("minipython".to_string())
    );
}

#[test]
fn runs_interactive_input_mode() {
    assert_eq!(run_interactive_source("1 + 2\n"), Ok(vec!["3".to_string()]));
    assert_eq!(
        run_interactive_source("1; 2"),
        Ok(vec!["1".to_string(), "2".to_string()])
    );
    assert_eq!(
        run_interactive_source("if True:\n    print(\"ok\")"),
        Ok(vec!["ok".to_string()])
    );
    assert_eq!(run_interactive_source("\n"), Ok(Vec::new()));
}

#[test]
fn rejects_interactive_multiple_physical_statements() {
    assert_eq!(
        run_interactive_source("1\n2"),
        Err("parse error: expected end of input, found Number(2)".to_string())
    );
    assert_eq!(
        run_interactive_source("x = 1\ny = 2"),
        Err("parse error: expected end of input, found Identifier(\"y\")".to_string())
    );
}

#[test]
fn parses_func_type_input_mode() {
    assert_eq!(
        parse_func_type_source("(int, *str, **Any) -> float"),
        Ok(
            "FunctionType { arg_types: [Name(\"int\"), Name(\"str\"), Name(\"Any\")], returns: Name(\"float\") }"
                .to_string()
        )
    );
    assert!(
        parse_func_type_source("(List[str]) -> None")
            .unwrap()
            .contains("Subscript")
    );
    assert_eq!(
        parse_func_type_source("(*int, **str) -> None"),
        Ok("FunctionType { arg_types: [Name(\"int\"), Name(\"str\")], returns: None }".to_string())
    );
}

#[test]
fn prints_grouped_addition() {
    assert_eq!(run_source("print(1 + (2 + 3))"), Ok(vec!["6".to_string()]));
}

#[test]
fn runs_arithmetic_precedence() {
    assert_eq!(
        run_source("print(1 + 2 * 3)\nprint((1 + 2) * 3)\nprint(10 - 3 - 2)"),
        Ok(vec!["7".to_string(), "9".to_string(), "5".to_string()])
    );
}

#[test]
fn runs_division_modulo_and_power() {
    assert_eq!(
        run_source("print(5 // 2)\nprint(5 % 2)\nprint(2 ** 3 ** 2)\nprint(5 / 2)"),
        Ok(vec![
            "2".to_string(),
            "1".to_string(),
            "512".to_string(),
            "2.5".to_string(),
        ])
    );
}

#[test]
fn runs_sequence_repetition_and_basic_len_list_builtins() {
    assert_eq!(
        run_source(
            "print([1, 2] * 2)\nprint(2 * (3, 4))\nprint(\"ab\" * 3)\nprint([1] * 0, \"x\" * -1)\nprint(len([1, 2]), len((3, 4)), len(\"abc\"), len({\"a\": 1}), len(range(3)))\nprint(list((i for i in range(3))))\nprint(list(\"ab\"))\nprint(list())\nprint(max(1, 3, 2), max([1, 3, 2]))"
        ),
        Ok(vec![
            "[1, 2, 1, 2]".to_string(),
            "(3, 4, 3, 4)".to_string(),
            "ababab".to_string(),
            "[] ".to_string(),
            "2 2 3 1 3".to_string(),
            "[0, 1, 2]".to_string(),
            "['a', 'b']".to_string(),
            "[]".to_string(),
            "3 3".to_string(),
        ])
    );
}

#[test]
fn runs_unary_arithmetic() {
    assert_eq!(
        run_source("print(-2 ** 2)\nprint((-2) ** 2)\nprint(+1)\nprint(+True)"),
        Ok(vec![
            "-4".to_string(),
            "4".to_string(),
            "1".to_string(),
            "1".to_string()
        ])
    );
}

#[test]
fn runs_bitwise_and_shift_expressions() {
    assert_eq!(
        run_source(
            "print(~1)\nprint(1 | 2, 3 ^ 1, 6 & 3)\nprint(1 << 3, 8 >> 1)\nprint(1 + 2 << 2, 8 >> 1 + 1)"
        ),
        Ok(vec![
            "-2".to_string(),
            "3 2 2".to_string(),
            "8 4".to_string(),
            "12 2".to_string(),
        ])
    );
}

#[test]
fn prints_boolean_literals() {
    assert_eq!(
        run_source("print(True, False)"),
        Ok(vec!["True False".to_string()])
    );
}

#[test]
fn prints_none_literal() {
    assert_eq!(run_source("print(None)"), Ok(vec!["None".to_string()]));
}

#[test]
fn prints_ellipsis_literal_and_builtin_name() {
    assert_eq!(
        run_source("print(...)\nprint(Ellipsis)\nprint(... is Ellipsis)"),
        Ok(vec![
            "Ellipsis".to_string(),
            "Ellipsis".to_string(),
            "True".to_string(),
        ])
    );
}

#[test]
fn prints_not_implemented_builtin_singleton() {
    assert_eq!(
        run_source(
            "print(NotImplemented)\n\
             print(NotImplemented is NotImplemented, NotImplemented == NotImplemented)"
        ),
        Ok(vec!["NotImplemented".to_string(), "True True".to_string()])
    );
    assert_eq!(
        run_source("print(bool(NotImplemented))"),
        Err(
            "runtime error: TypeError: NotImplemented should not be used in a boolean context"
                .to_string()
        )
    );
}

#[test]
fn runs_pass_statement() {
    assert_eq!(run_source("pass\nprint(1)"), Ok(vec!["1".to_string()]));
}

#[test]
fn runs_assert_statement_when_condition_is_truthy() {
    assert_eq!(
        run_source("assert True\nassert 1\nprint(\"after\")"),
        Ok(vec!["after".to_string()])
    );
}

#[test]
fn reports_assertion_errors() {
    assert_eq!(
        run_source("assert False"),
        Err("runtime error: AssertionError".to_string())
    );
    assert_eq!(
        run_source("assert 0, \"bad\""),
        Err("runtime error: AssertionError: bad".to_string())
    );
}

#[test]
fn reports_unhandled_raise() {
    assert_eq!(
        run_source("raise Exception(\"boom\")"),
        Err("runtime error: Exception: boom".to_string())
    );
}

#[test]
fn catches_raised_exceptions() {
    assert_eq!(
        run_source(
            "try:\n    raise Exception(\"boom\")\nexcept Exception as error:\n    print(\"caught\", error)\nprint(\"after\")"
        ),
        Ok(vec!["caught boom".to_string(), "after".to_string()])
    );
}

#[test]
fn preserves_explicit_exception_cause() {
    assert_eq!(
        run_source(
            "try:\n    raise ValueError(\"bad\") from Exception(\"root\")\nexcept ValueError as error:\n    print(error, error.__cause__, error.__cause__.__class__.__name__, error.__suppress_context__)"
        ),
        Ok(vec!["bad root Exception True".to_string()])
    );
}

#[test]
fn preserves_implicit_exception_context() {
    assert_eq!(
        run_source(
            "try:\n    try:\n        raise TypeError(\"root\")\n    except TypeError:\n        raise ValueError(\"bad\")\nexcept ValueError as error:\n    print(error, error.__context__, error.__context__.__class__.__name__, error.__suppress_context__)"
        ),
        Ok(vec!["bad root TypeError False".to_string()])
    );
}

#[test]
fn supports_raise_from_none() {
    assert_eq!(
        run_source(
            "try:\n    try:\n        raise TypeError(\"root\")\n    except TypeError:\n        raise ValueError(\"bad\") from None\nexcept ValueError as error:\n    print(error.__context__, error.__cause__, error.__suppress_context__)"
        ),
        Ok(vec!["root None True".to_string()])
    );
}

#[test]
fn rejects_invalid_exception_cause() {
    assert_eq!(
        run_source("raise ValueError(\"bad\") from 1"),
        Err("runtime error: exception cause must be None or derive from BaseException".to_string())
    );
}

#[test]
fn runs_try_else_when_no_exception() {
    assert_eq!(
        run_source(
            "try:\n    print(\"body\")\nexcept Exception:\n    print(\"except\")\nelse:\n    print(\"else\")"
        ),
        Ok(vec!["body".to_string(), "else".to_string()])
    );
}

#[test]
fn catches_assertion_with_bare_except() {
    assert_eq!(
        run_source("try:\n    assert False, \"bad\"\nexcept:\n    print(\"caught\")"),
        Ok(vec!["caught".to_string()])
    );
}

#[test]
fn propagates_unmatched_exception_handlers() {
    assert_eq!(
        run_source(
            "try:\n    raise ValueError(\"bad\")\nexcept AssertionError:\n    print(\"wrong\")"
        ),
        Err("runtime error: ValueError: bad".to_string())
    );
}

#[test]
fn catches_tuple_exception_handlers() {
    assert_eq!(
        run_source(
            "try:\n    raise TypeError(\"bad\")\nexcept (ValueError, TypeError) as error:\n    print(\"caught\", error)\ntry:\n    raise AssertionError(\"no\")\nexcept (ValueError, TypeError):\n    print(\"wrong\")\nexcept AssertionError as error:\n    print(\"second\", error)"
        ),
        Ok(vec!["caught bad".to_string(), "second no".to_string()])
    );
}

#[test]
fn catches_dotted_exception_handler_type() {
    assert_eq!(
        run_source(
            "class Box:\n    pass\nclass CustomError(Exception):\n    pass\nBox.CustomError = CustomError\ntry:\n    raise CustomError(\"bad\")\nexcept Box.CustomError as error:\n    print(\"caught\", error)"
        ),
        Ok(vec!["caught bad".to_string()])
    );
}

#[test]
fn catches_dynamic_exception_handler_type_expression() {
    assert_eq!(
        run_source(
            "class Box:\n    pass\nclass CustomError(Exception):\n    pass\nBox.CustomError = CustomError\nenabled = True\ntry:\n    raise CustomError(\"bad\")\nexcept Box.CustomError if enabled else () as error:\n    print(\"caught\", error)\nenabled = False\ntry:\n    raise CustomError(\"again\")\nexcept Box.CustomError if enabled else ():\n    print(\"wrong\")\nexcept CustomError as error:\n    print(\"fallback\", error)"
        ),
        Ok(vec!["caught bad".to_string(), "fallback again".to_string()])
    );
}

#[test]
fn runs_except_star_handlers() {
    assert_eq!(
        run_source(
            "try:\n    raise ValueError(\"bad\")\nexcept* TypeError:\n    print(\"type\")\nexcept* ValueError:\n    print(\"value\")\nelse:\n    print(\"else\")\nprint(\"after\")"
        ),
        Ok(vec!["value".to_string(), "after".to_string()])
    );
    assert_eq!(
        run_source(
            "try:\n    raise TypeError(\"bad\")\nexcept* ValueError, TypeError:\n    print(\"multi\")"
        ),
        Ok(vec!["multi".to_string()])
    );
}

#[test]
fn splits_exception_groups_for_except_star_handlers() {
    assert_eq!(
        run_source(
            "group = ExceptionGroup(\"eg\", [ValueError(\"V\"), TypeError(\"T\")])\nprint(group.__class__.__name__, group.message)\nprint(group.exceptions[0].__class__.__name__, group.exceptions[0])\nprint(group.exceptions[1].__class__.__name__, group.exceptions[1])"
        ),
        Ok(vec![
            "ExceptionGroup eg".to_string(),
            "ValueError V".to_string(),
            "TypeError T".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "try:\n    try:\n        raise ExceptionGroup(\"eg\", [ValueError(\"V\"), TypeError(\"T\")])\n    except* ValueError as matched:\n        print(\"matched\", matched.message, matched.exceptions[0].__class__.__name__, matched.exceptions[0])\nexcept ExceptionGroup as rest:\n    print(\"rest\", rest.message, rest.exceptions[0].__class__.__name__, rest.exceptions[0])"
        ),
        Ok(vec![
            "matched eg ValueError V".to_string(),
            "rest eg TypeError T".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "try:\n    raise ExceptionGroup(\"eg\", [ValueError(\"V\"), TypeError(\"T\")])\nexcept* ValueError as value_group:\n    print(\"value\", value_group.exceptions[0])\nexcept* TypeError as type_group:\n    print(\"type\", type_group.exceptions[0])\nprint(\"done\")"
        ),
        Ok(vec![
            "value V".to_string(),
            "type T".to_string(),
            "done".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "try:\n    raise ValueError(\"V\")\nexcept* ValueError as group:\n    print(group.__class__.__name__, group.exceptions[0].__class__.__name__, group.exceptions[0])"
        ),
        Ok(vec!["ExceptionGroup ValueError V".to_string()])
    );
}

#[test]
fn rejects_exception_group_types_in_except_star_handlers() {
    let message = "catching ExceptionGroup with except* is not allowed. Use except instead.";
    assert_eq!(
        run_source(
            "try:\n    try:\n        raise OSError(\"blah\")\n    except* ExceptionGroup as error:\n        print(\"wrong\", error)\nexcept TypeError as error:\n    print(error)"
        ),
        Ok(vec![message.to_string()])
    );
    assert_eq!(
        run_source(
            "try:\n    try:\n        raise ExceptionGroup(\"eg\", [ValueError(\"V\")])\n    except* (TypeError, ExceptionGroup):\n        print(\"wrong\")\nexcept TypeError as error:\n    print(error)"
        ),
        Ok(vec![message.to_string()])
    );
    assert_eq!(
        run_source(
            "try:\n    try:\n        raise BaseExceptionGroup(\"eg\", [ValueError(\"stop\")])\n    except* BaseExceptionGroup:\n        print(\"wrong\")\nexcept TypeError as error:\n    print(error)"
        ),
        Ok(vec![message.to_string()])
    );
}

#[test]
fn rejects_default_except_before_typed_handler() {
    assert_eq!(
        run_source("try:\n    pass\nexcept:\n    pass\nexcept ValueError:\n    pass"),
        Err("parse error: default 'except:' must be last".to_string())
    );
}

#[test]
fn rejects_invalid_except_star_control_flow() {
    assert_eq!(
        run_source("try:\n    raise ValueError(\"bad\")\nexcept* ValueError:\n    return"),
        Err(
            "compile error: 'break', 'continue' and 'return' cannot appear in an except* block"
                .to_string()
        )
    );
    assert_eq!(
        run_source(
            "for i in range(1):\n    try:\n        raise ValueError(\"bad\")\n    except* ValueError:\n        break"
        ),
        Err(
            "compile error: 'break', 'continue' and 'return' cannot appear in an except* block"
                .to_string()
        )
    );
}

#[test]
fn runs_finally_without_exception() {
    assert_eq!(
        run_source("try:\n    print(\"body\")\nfinally:\n    print(\"finally\")"),
        Ok(vec!["body".to_string(), "finally".to_string()])
    );
}

#[test]
fn runs_finally_after_handled_exception() {
    assert_eq!(
        run_source(
            "try:\n    raise Exception(\"boom\")\nexcept Exception:\n    print(\"except\")\nfinally:\n    print(\"finally\")"
        ),
        Ok(vec!["except".to_string(), "finally".to_string()])
    );
}

#[test]
fn runs_finally_before_reraising_exception() {
    assert_eq!(
        run_source(
            "try:\n    try:\n        raise Exception(\"boom\")\n    finally:\n        print(\"cleanup\")\nexcept Exception as error:\n    print(\"caught\", error)"
        ),
        Ok(vec!["cleanup".to_string(), "caught boom".to_string()])
    );
}

#[test]
fn catches_exception_raised_inside_function_call() {
    assert_eq!(
        run_source(
            "def fail():\n    raise ValueError(\"bad\")\ntry:\n    fail()\nexcept ValueError as error:\n    print(\"caught\", error)"
        ),
        Ok(vec!["caught bad".to_string()])
    );
}

#[test]
fn runs_finally_before_returning_from_function() {
    assert_eq!(
        run_source(
            "def f():\n    try:\n        return \"value\"\n    finally:\n        print(\"cleanup\")\nprint(f())"
        ),
        Ok(vec!["cleanup".to_string(), "value".to_string()])
    );
}

#[test]
fn finally_return_overrides_try_return() {
    assert_eq!(
        run_source(
            "def f():\n    try:\n        return 1\n    finally:\n        return 2\nprint(f())"
        ),
        Ok(vec!["2".to_string()])
    );
}

#[test]
fn runs_finally_before_breaking_from_loop() {
    assert_eq!(
        run_source(
            "while True:\n    try:\n        break\n    finally:\n        print(\"cleanup\")\nprint(\"after\")"
        ),
        Ok(vec!["cleanup".to_string(), "after".to_string()])
    );
}

#[test]
fn runs_finally_before_continuing_loop() {
    assert_eq!(
        run_source(
            "i = 0\nwhile i < 3:\n    i += 1\n    try:\n        continue\n    finally:\n        print(\"cleanup\", i)\nprint(\"done\", i)"
        ),
        Ok(vec![
            "cleanup 1".to_string(),
            "cleanup 2".to_string(),
            "cleanup 3".to_string(),
            "done 3".to_string(),
        ])
    );
}

#[test]
fn runs_finally_control_flow_overrides() {
    assert_eq!(
        run_source(
            "for x in range(3):\n    try:\n        print(\"try\", x)\n        break\n    finally:\n        print(\"finally\", x)\n        continue\nprint(\"after\")"
        ),
        Ok(vec![
            "try 0".to_string(),
            "finally 0".to_string(),
            "try 1".to_string(),
            "finally 1".to_string(),
            "try 2".to_string(),
            "finally 2".to_string(),
            "after".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "def f():\n    for x in range(1):\n        try:\n            return \"try\"\n        finally:\n            break\n    return \"after\"\nprint(f())"
        ),
        Ok(vec!["after".to_string()])
    );
    assert_eq!(
        run_source(
            "def f():\n    try:\n        raise ValueError(\"bad\")\n    finally:\n        return \"cleanup\"\nprint(f())"
        ),
        Ok(vec!["cleanup".to_string()])
    );
}

#[test]
fn runs_with_statement() {
    assert_eq!(
        run_source(
            "class Manager:\n    def __enter__(self):\n        print(\"enter\")\n        return \"value\"\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\", exc_type, exc)\nwith Manager() as value:\n    print(\"body\", value)\nprint(\"after\")"
        ),
        Ok(vec![
            "enter".to_string(),
            "body value".to_string(),
            "exit None None".to_string(),
            "after".to_string(),
        ])
    );
}

#[test]
fn runs_multiple_with_items_as_nested_managers() {
    assert_eq!(
        run_source(
            "class Manager:\n    def __init__(self, name):\n        self.name = name\n    def __enter__(self):\n        print(\"enter\", self.name)\n        return self.name\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\", self.name)\nwith Manager(\"a\") as a, Manager(\"b\") as b:\n    print(\"body\", a, b)"
        ),
        Ok(vec![
            "enter a".to_string(),
            "enter b".to_string(),
            "body a b".to_string(),
            "exit b".to_string(),
            "exit a".to_string(),
        ])
    );
}

#[test]
fn runs_parenthesized_with_items() {
    assert_eq!(
        run_source(
            "class Manager:\n    def __init__(self, name):\n        self.name = name\n    def __enter__(self):\n        print(\"enter\", self.name)\n        return self.name\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\", self.name)\nwith (\n    Manager(\"a\") as a,\n    Manager(\"b\") as b,\n):\n    print(\"body\", a, b)"
        ),
        Ok(vec![
            "enter a".to_string(),
            "enter b".to_string(),
            "body a b".to_string(),
            "exit b".to_string(),
            "exit a".to_string(),
        ])
    );
}

#[test]
fn calls_with_exit_before_propagating_exception() {
    assert_eq!(
        run_source(
            "class Manager:\n    def __enter__(self):\n        print(\"enter\")\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\", exc_type.__name__, exc)\ntry:\n    with Manager():\n        raise ValueError(\"bad\")\nexcept ValueError as error:\n    print(\"caught\", error)"
        ),
        Ok(vec![
            "enter".to_string(),
            "exit ValueError bad".to_string(),
            "caught bad".to_string(),
        ])
    );
}

#[test]
fn runs_grouped_with_item_as_target() {
    assert_eq!(
        run_source(
            "class Manager:\n    def __enter__(self):\n        print(\"enter\")\n        return \"value\"\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\")\nwith (Manager()) as value:\n    print(\"body\", value)"
        ),
        Ok(vec![
            "enter".to_string(),
            "body value".to_string(),
            "exit".to_string(),
        ])
    );
}

#[test]
fn with_exit_can_suppress_exception() {
    assert_eq!(
        run_source(
            "class Manager:\n    def __enter__(self):\n        pass\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"suppress\", exc_type.__name__, exc)\n        return True\nwith Manager():\n    raise ValueError(\"bad\")\nprint(\"after\")"
        ),
        Ok(vec![
            "suppress ValueError bad".to_string(),
            "after".to_string()
        ])
    );
}

#[test]
fn runs_with_exit_before_returning_from_function() {
    assert_eq!(
        run_source(
            "class Manager:\n    def __enter__(self):\n        return \"value\"\n    def __exit__(self, exc_type, exc, traceback):\n        print(\"exit\")\ndef f():\n    with Manager() as value:\n        return value\nprint(f())"
        ),
        Ok(vec!["exit".to_string(), "value".to_string()])
    );
}

#[test]
fn with_exit_can_suppress_exceptions() {
    assert_eq!(
        run_source(
            "class Swallow:\n    def __enter__(self):\n        print(\"enter\")\n        return self\n    def __exit__(self, exc_type, exc_value, traceback):\n        print(\"exit\", exc_type.__name__, exc_value)\n        return True\nwith Swallow():\n    raise ValueError(\"bad\")\nprint(\"after\")"
        ),
        Ok(vec![
            "enter".to_string(),
            "exit ValueError bad".to_string(),
            "after".to_string(),
        ])
    );
}

#[test]
fn with_exit_can_propagate_exceptions() {
    assert_eq!(
        run_source(
            "class Propagate:\n    def __enter__(self):\n        return self\n    def __exit__(self, exc_type, exc_value, traceback):\n        return False\nwith Propagate():\n    raise ValueError(\"bad\")"
        ),
        Err("runtime error: ValueError: bad".to_string())
    );
}

#[test]
fn with_exit_runs_before_return_break_and_continue() {
    assert_eq!(
        run_source(
            "class Manager:\n    def __init__(self, name):\n        self.name = name\n    def __enter__(self):\n        print(\"enter\", self.name)\n        return self\n    def __exit__(self, exc_type, exc_value, traceback):\n        print(\"exit\", self.name)\n\ndef f():\n    with Manager(\"return\"):\n        return 3\nprint(f())\nwhile True:\n    with Manager(\"break\"):\n        break\nprint(\"after break\")\ni = 0\nwhile i < 2:\n    i += 1\n    with Manager(\"continue\"):\n        continue\nprint(\"after continue\")"
        ),
        Ok(vec![
            "enter return".to_string(),
            "exit return".to_string(),
            "3".to_string(),
            "enter break".to_string(),
            "exit break".to_string(),
            "after break".to_string(),
            "enter continue".to_string(),
            "exit continue".to_string(),
            "enter continue".to_string(),
            "exit continue".to_string(),
            "after continue".to_string(),
        ])
    );
}

#[test]
fn runs_import_statement() {
    assert_eq!(
        run_source(
            "import sys\nimport time, math as m\nprint(sys.__name__, time.__name__, m.__name__)\nprint(m.pi)"
        ),
        Ok(vec![
            "sys time math".to_string(),
            "3.141592653589793".to_string(),
        ])
    );
}

#[test]
fn caches_imported_modules_in_sys_modules() {
    assert_eq!(
        run_source(
            "import sys\nprint('math' in sys.modules)\nimport math\nprint('math' in sys.modules, sys.modules['math'] is math)\nagain = __import__('math')\nprint(again is math)\nimport os.path\nprint(sys.modules['os'].path is sys.modules['os.path'])"
        ),
        Ok(vec![
            "False".to_string(),
            "True True".to_string(),
            "True".to_string(),
            "True".to_string(),
        ])
    );
}

#[test]
fn binds_dotted_imports_to_parent_modules() {
    assert_eq!(
        run_source(
            "import sys\nsub = __import__('os.path', fromlist=['sep'])\nprint(sub.__name__, 'os' in sys.modules, sys.modules['os'].path is sub)\nabc = __import__('collections.abc', fromlist=['Hashable'])\nprint(abc.__name__, 'collections' in sys.modules, sys.modules['collections'].abc is abc)"
        ),
        Ok(vec![
            "os.path True True".to_string(),
            "collections.abc True True".to_string(),
        ])
    );
}

#[test]
fn uses_fromlist_truthiness_in_import_builtin() {
    assert_eq!(
        run_source(
            "print(__import__('os.path', fromlist='').__name__)\nprint(__import__('os.path', fromlist=0).__name__)\nprint(__import__('os.path', fromlist=False).__name__)\nprint(__import__('os.path', fromlist={}).__name__)\nprint(__import__('os.path', fromlist=set()).__name__)\nprint(__import__('os.path', fromlist='x').__name__)\nprint(__import__('os.path', fromlist=1).__name__)\nprint(__import__('os.path', fromlist=True).__name__)\nclass FalseBool:\n    def __bool__(self):\n        return False\nclass FalseLen:\n    def __len__(self):\n        return 0\nclass TrueLen:\n    def __len__(self):\n        return 1\nclass BadBool:\n    def __bool__(self):\n        return 1\nprint(__import__('os.path', fromlist=FalseBool()).__name__)\nprint(__import__('os.path', fromlist=FalseLen()).__name__)\nprint(__import__('os.path', fromlist=TrueLen()).__name__)\ntry:\n    __import__('os.path', fromlist=BadBool())\nexcept TypeError as error:\n    print(error)"
        ),
        Ok(vec![
            "os".to_string(),
            "os".to_string(),
            "os".to_string(),
            "os".to_string(),
            "os".to_string(),
            "os.path".to_string(),
            "os.path".to_string(),
            "os.path".to_string(),
            "os".to_string(),
            "os".to_string(),
            "os.path".to_string(),
            "__bool__ should return bool, returned int".to_string(),
        ])
    );
}

#[test]
fn isolates_sys_modules_between_runs() {
    assert_eq!(
        run_source("import sys\nsys.modules['math'] = 'changed'\nimport math\nprint(math)"),
        Ok(vec!["changed".to_string()])
    );
    assert_eq!(
        run_source("import math\nprint(math.__name__)"),
        Ok(vec!["math".to_string()])
    );
}

#[test]
fn runs_from_import_statement() {
    assert_eq!(
        run_source(
            "from time import time\nfrom sys import (path, argv,)\nprint(time.__name__, path, argv)\nprint(time())"
        ),
        Ok(vec!["time [''] []".to_string(), "0.0".to_string()])
    );
}

#[test]
fn runs_import_aliases_and_star_import() {
    assert_eq!(
        run_source(
            "from math import pi as circle_pi, sqrt\nprint(circle_pi)\nprint(sqrt(9))\nfrom math import *\nprint(tau)"
        ),
        Ok(vec![
            "3.141592653589793".to_string(),
            "3.0".to_string(),
            "6.283185307179586".to_string(),
        ])
    );
}

#[test]
fn runs_lazy_import_syntax() {
    assert_eq!(
        run_source("lazy import sys\nlazy import math as m\nprint(sys.__name__, m.sqrt(25))"),
        Ok(vec!["sys 5.0".to_string()])
    );
    assert_eq!(
        run_source(
            "lazy from sys import path\nlazy from math import sqrt as root\nprint(path, root(36))"
        ),
        Ok(vec!["[''] 6.0".to_string()])
    );
}

#[test]
fn imports_virtual_source_modules() {
    assert_eq!(
        run_source_with_virtual_modules(
            "import tools\nfrom tools import VALUE\nprint(tools.__name__, VALUE, tools.inc(2))\nimport tools\nprint('done')",
            vec![VirtualModule::module(
                "tools",
                "print('loading tools')\nVALUE = 41\ndef inc(value):\n    return value + 1",
            )],
        ),
        Ok(output_lines(&["loading tools", "tools 41 3", "done"]))
    );
}

#[test]
fn imports_virtual_packages_with_relative_imports() {
    assert_eq!(
        run_source_with_virtual_modules(
            "import pkg\nprint(pkg.__name__, pkg.__package__, pkg.VALUE)\nfrom pkg import util\nprint(pkg.util is util, util.VALUE)",
            vec![
                VirtualModule::package("pkg", "from . import util\nVALUE = util.VALUE + 1",),
                VirtualModule::module("pkg.util", "VALUE = 2"),
            ],
        ),
        Ok(output_lines(&["pkg pkg 3", "True 2"]))
    );
}

#[test]
fn imports_sandbox_directory_modules() {
    let sandbox = TestSandboxDir::new("imports");
    sandbox.write(
        "tools.py",
        "print('loading tools')\nVALUE = 41\ndef inc(value):\n    return value + 1",
    );
    sandbox.write(
        "pkg/__init__.py",
        "from . import util\nVALUE = util.VALUE + 1",
    );
    sandbox.write("pkg/util.py", "VALUE = 2");

    assert_eq!(
        run_source_with_sandbox_dir(
            "import tools\nfrom tools import VALUE\nprint(tools.__file__)\nprint(tools.__name__, VALUE, tools.inc(2))\nimport pkg\nfrom pkg import util\nprint(pkg.__name__, pkg.__package__, pkg.VALUE)\nprint(pkg.util is util, util.VALUE)",
            sandbox.path(),
        ),
        Ok(output_lines(&[
            "loading tools",
            "<virtual>/tools.py",
            "tools 41 3",
            "pkg pkg 3",
            "True 2",
        ]))
    );
}

#[test]
fn ignores_non_package_sandbox_directories() {
    let sandbox = TestSandboxDir::new("non-package");
    sandbox.write("loose/util.py", "VALUE = 9");

    let error = run_source_with_sandbox_dir("import loose.util", sandbox.path()).unwrap_err();
    assert!(
        error.contains("ModuleNotFoundError"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_invalid_sandbox_module_names() {
    let sandbox = TestSandboxDir::new("invalid-module");
    sandbox.write("bad-name.py", "print('bad')");

    let error = run_source_with_sandbox_dir("print('safe')", sandbox.path()).unwrap_err();
    assert!(
        error.contains("sandbox error: invalid module path component 'bad-name'"),
        "unexpected error: {error}"
    );
}

#[cfg(unix)]
#[test]
fn rejects_sandbox_directory_symlink_escape() {
    let sandbox = TestSandboxDir::new("symlink-root");
    let outside = TestSandboxDir::new("symlink-outside");
    outside.write("escape.py", "print('escaped')");
    std::os::unix::fs::symlink(
        outside.path().join("escape.py"),
        sandbox.path().join("escape.py"),
    )
    .expect("failed to create test symlink");

    let error = run_source_with_sandbox_dir("print('safe')", sandbox.path()).unwrap_err();
    assert!(
        error.contains("sandbox error: module path escapes sandbox root"),
        "unexpected error: {error}"
    );
}

#[test]
fn sandbox_policy_denies_stdlib_imports() {
    let sandbox = TestSandboxDir::new("deny-stdlib");
    sandbox.write("tools.py", "VALUE = 3");

    assert_eq!(
        run_source_with_sandbox_dir_and_policy(
            "import tools\nprint(tools.VALUE)",
            sandbox.path(),
            SandboxPolicy::deny_stdlib(),
        ),
        Ok(output_lines(&["3"]))
    );

    assert_eq!(
        run_source_with_sandbox_dir_and_policy(
            "import math",
            sandbox.path(),
            SandboxPolicy::deny_stdlib(),
        ),
        Err("runtime error: ModuleNotFoundError: No module named 'math'".to_string())
    );
}

#[test]
fn sandbox_policy_denies_required_sandbox_stdlib_surface() {
    let sandbox = TestSandboxDir::new("deny-required-stdlib");

    for module in REQUIRED_SANDBOX_STDLIB_MODULES {
        let error = run_source_with_sandbox_dir_and_policy(
            &format!("import {module}"),
            sandbox.path(),
            SandboxPolicy::deny_stdlib(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            format!("runtime error: ModuleNotFoundError: No module named '{module}'")
        );
    }
}

#[test]
fn sandbox_policy_allows_selected_stdlib_imports() {
    let sandbox = TestSandboxDir::new("allow-stdlib");
    let policy = SandboxPolicy::allow_stdlib_modules(["math"]).unwrap();

    assert_eq!(
        run_source_with_sandbox_dir_and_policy(
            "import math\nprint(math.sqrt(9))",
            sandbox.path(),
            policy.clone(),
        ),
        Ok(output_lines(&["3.0"]))
    );

    assert_eq!(
        run_source_with_sandbox_dir_and_policy("import sys", sandbox.path(), policy),
        Err("runtime error: ModuleNotFoundError: No module named 'sys'".to_string())
    );
}

#[test]
fn sandbox_policy_allows_selected_stdlib_package_children() {
    let sandbox = TestSandboxDir::new("allow-stdlib-children");
    let policy = SandboxPolicy::allow_stdlib_modules(["collections"]).unwrap();

    assert_eq!(
        run_source_with_sandbox_dir_and_policy(
            "import collections.abc\nprint(collections.abc.Sequence.__name__)",
            sandbox.path(),
            policy.clone(),
        ),
        Ok(output_lines(&["Sequence"]))
    );

    assert_eq!(
        run_source_with_sandbox_dir_and_policy("import math", sandbox.path(), policy),
        Err("runtime error: ModuleNotFoundError: No module named 'math'".to_string())
    );
}

#[test]
fn sandbox_policy_allows_required_sandbox_stdlib_surface() {
    let sandbox = TestSandboxDir::new("allow-required-stdlib");
    let policy =
        SandboxPolicy::allow_stdlib_modules(REQUIRED_SANDBOX_STDLIB_MODULES.to_vec()).unwrap();

    assert_eq!(
        run_source_with_sandbox_dir_and_policy(
            "import builtins, sys, types, collections, collections.abc, math, math.integer, array, copy, io, operator, functools, itertools, json\nprint(builtins.len([1, 2]))\nprint(isinstance(sys.modules, dict))\nprint(types.SimpleNamespace(x=1).x)\nprint(collections.Counter('aa')['a'], collections.abc.Sequence.__name__)\nprint(math.sqrt(4), math.integer.gcd(12, 18))\nprint(array.array('B', [65]).tobytes())\nprint(copy.copy([1]) == [1])\nbio = io.BytesIO(b'ab')\nprint(bio.read(1))\nprint(operator.add(2, 3), functools.reduce(lambda a, b: a + b, [1, 2, 3]))\nprint(next(itertools.count(4)))\nprint(json.loads('{\"a\": 1}')['a'])",
            sandbox.path(),
            policy,
        ),
        Ok(output_lines(&[
            "2",
            "True",
            "1",
            "2 Sequence",
            "2.0 6",
            "b'A'",
            "True",
            "b'a'",
            "5 6",
            "4",
            "1",
        ]))
    );
}

#[test]
fn io_bytesio_sandbox_subset_excludes_host_io_apis() {
    assert_eq!(
        run_source(
            "import io\nfor name in ['BytesIO', 'UnsupportedOperation', 'SEEK_SET', 'SEEK_CUR', 'SEEK_END']:\n    print(name, hasattr(io, name))\nfor name in ['open', 'FileIO', 'TextIOWrapper', 'StringIO', 'BufferedReader', 'BufferedWriter', 'RawIOBase', 'IOBase', '__all__']:\n    print(name, hasattr(io, name))\nprint(dir(io))"
        ),
        Ok(output_lines(&[
            "BytesIO True",
            "UnsupportedOperation True",
            "SEEK_SET True",
            "SEEK_CUR True",
            "SEEK_END True",
            "open False",
            "FileIO False",
            "TextIOWrapper False",
            "StringIO False",
            "BufferedReader False",
            "BufferedWriter False",
            "RawIOBase False",
            "IOBase False",
            "__all__ False",
            "['BytesIO', 'SEEK_CUR', 'SEEK_END', 'SEEK_SET', 'UnsupportedOperation', '__name__']",
        ]))
    );
}

#[test]
fn copy_sandbox_subset_excludes_pickle_dispatch_internals() {
    assert_eq!(
        run_source(
            "import copy\nfor name in ['Error', 'error', 'copy', 'deepcopy', 'replace', 'dispatch_table']:\n    print(name, hasattr(copy, name))\nfor name in ['_copy_dispatch', '_deepcopy_dispatch', '_keep_alive', '_reconstruct', '__all__']:\n    print(name, hasattr(copy, name))\nprint(dir(copy))"
        ),
        Ok(output_lines(&[
            "Error True",
            "error True",
            "copy True",
            "deepcopy True",
            "replace True",
            "dispatch_table True",
            "_copy_dispatch False",
            "_deepcopy_dispatch False",
            "_keep_alive False",
            "_reconstruct False",
            "__all__ False",
            "['Error', '__name__', 'copy', 'deepcopy', 'dispatch_table', 'error', 'replace']",
        ]))
    );
}

#[test]
fn array_sandbox_subset_excludes_pickle_module_internals() {
    assert_eq!(
        run_source(
            "import array\nfor name in ['array', 'typecodes']:\n    print(name, hasattr(array, name))\nfor name in ['ArrayType', '_array_reconstructor', '__all__']:\n    print(name, hasattr(array, name))\nprint(dir(array))"
        ),
        Ok(output_lines(&[
            "array True",
            "typecodes True",
            "ArrayType False",
            "_array_reconstructor False",
            "__all__ False",
            "['__name__', 'array', 'typecodes']",
        ]))
    );
}

#[test]
fn math_sandbox_subset_keeps_integer_submodule_narrow() {
    assert_eq!(
        run_source(
            "import math\nimport math.integer as mi\nfor name in ['sqrt', 'gcd', 'prod', 'sumprod', 'nextafter', 'ulp']:\n    print('math', name, hasattr(math, name))\nprint('math __all__', hasattr(math, '__all__'))\nfor name in ['comb', 'factorial', 'gcd', 'isqrt', 'lcm', 'perm']:\n    print('integer', name, hasattr(mi, name))\nfor name in ['sqrt', 'prod', 'sumprod', 'nextafter', 'ulp', '__all__']:\n    print('integer', name, hasattr(mi, name))\nprint(dir(mi))"
        ),
        Ok(output_lines(&[
            "math sqrt True",
            "math gcd True",
            "math prod True",
            "math sumprod True",
            "math nextafter True",
            "math ulp True",
            "math __all__ False",
            "integer comb True",
            "integer factorial True",
            "integer gcd True",
            "integer isqrt True",
            "integer lcm True",
            "integer perm True",
            "integer sqrt False",
            "integer prod False",
            "integer sumprod False",
            "integer nextafter False",
            "integer ulp False",
            "integer __all__ False",
            "['__name__', 'comb', 'factorial', 'gcd', 'isqrt', 'lcm', 'perm']",
        ]))
    );
}

#[test]
fn itertools_sandbox_subset_keeps_export_surface_explicit() {
    assert_eq!(
        run_source(
            "import itertools\nfor name in ['accumulate', 'batched', 'chain', 'combinations', 'combinations_with_replacement', 'compress', 'count', 'cycle', 'dropwhile', 'filterfalse', 'groupby', 'islice', 'pairwise', 'permutations', 'product', 'repeat', 'starmap', 'takewhile', 'tee', 'zip_longest']:\n    print(name, hasattr(itertools, name))\nfor name in ['__all__', 'imap', 'izip', 'ifilter', 'ifilterfalse']:\n    print(name, hasattr(itertools, name))\nprint(dir(itertools))"
        ),
        Ok(output_lines(&[
            "accumulate True",
            "batched True",
            "chain True",
            "combinations True",
            "combinations_with_replacement True",
            "compress True",
            "count True",
            "cycle True",
            "dropwhile True",
            "filterfalse True",
            "groupby True",
            "islice True",
            "pairwise True",
            "permutations True",
            "product True",
            "repeat True",
            "starmap True",
            "takewhile True",
            "tee True",
            "zip_longest True",
            "__all__ False",
            "imap False",
            "izip False",
            "ifilter False",
            "ifilterfalse False",
            "['__name__', 'accumulate', 'batched', 'chain', 'combinations', 'combinations_with_replacement', 'compress', 'count', 'cycle', 'dropwhile', 'filterfalse', 'groupby', 'islice', 'pairwise', 'permutations', 'product', 'repeat', 'starmap', 'takewhile', 'tee', 'zip_longest']",
        ]))
    );
}

#[test]
fn functools_sandbox_subset_keeps_export_surface_explicit() {
    assert_eq!(
        run_source(
            "import functools\nfor name in ['WRAPPER_ASSIGNMENTS', 'WRAPPER_UPDATES', 'cache', 'cached_property', 'cmp_to_key', 'lru_cache', 'partial', 'partialmethod', 'reduce', 'singledispatch', 'singledispatchmethod', 'total_ordering', 'update_wrapper', 'wraps']:\n    print(name, hasattr(functools, name))\nfor name in ['__all__', '_CacheInfo', '_lru_cache_wrapper', '_make_key', '_unwrap_partial']:\n    print(name, hasattr(functools, name))\nprint(dir(functools))"
        ),
        Ok(output_lines(&[
            "WRAPPER_ASSIGNMENTS True",
            "WRAPPER_UPDATES True",
            "cache True",
            "cached_property True",
            "cmp_to_key True",
            "lru_cache True",
            "partial True",
            "partialmethod True",
            "reduce True",
            "singledispatch True",
            "singledispatchmethod True",
            "total_ordering True",
            "update_wrapper True",
            "wraps True",
            "__all__ False",
            "_CacheInfo False",
            "_lru_cache_wrapper False",
            "_make_key False",
            "_unwrap_partial False",
            "['WRAPPER_ASSIGNMENTS', 'WRAPPER_UPDATES', '__name__', 'cache', 'cached_property', 'cmp_to_key', 'lru_cache', 'partial', 'partialmethod', 'reduce', 'singledispatch', 'singledispatchmethod', 'total_ordering', 'update_wrapper', 'wraps']",
        ]))
    );
}

#[test]
fn collections_sandbox_subset_keeps_export_surface_explicit() {
    assert_eq!(
        run_source(
            "import collections\nfor name in ['ChainMap', 'Counter', 'OrderedDict', 'UserDict', 'UserList', 'UserString', '_count_elements', 'abc', 'deque', 'namedtuple']:\n    print(name, hasattr(collections, name))\nfor name in ['defaultdict', '__all__', '_tuplegetter', '_Link']:\n    print(name, hasattr(collections, name))\nprint(dir(collections))\nimport collections.abc as abc\nfor name in ['AsyncGenerator', 'AsyncIterable', 'AsyncIterator', 'Awaitable', 'Buffer', 'ByteString', 'Callable', 'Collection', 'Container', 'Coroutine', 'Generator', 'Hashable', 'ItemsView', 'Iterable', 'Iterator', 'KeysView', 'Mapping', 'MappingView', 'MutableMapping', 'MutableSequence', 'MutableSet', 'Reversible', 'Sequence', 'Set', 'Sized', 'ValuesView']:\n    print('abc', name, hasattr(abc, name))\nprint('abc __all__', hasattr(abc, '__all__'))\nprint(dir(abc))"
        ),
        Ok(output_lines(&[
            "ChainMap True",
            "Counter True",
            "OrderedDict True",
            "UserDict True",
            "UserList True",
            "UserString True",
            "_count_elements True",
            "abc True",
            "deque True",
            "namedtuple True",
            "defaultdict False",
            "__all__ False",
            "_tuplegetter False",
            "_Link False",
            "['ChainMap', 'Counter', 'OrderedDict', 'UserDict', 'UserList', 'UserString', '__name__', '_count_elements', 'abc', 'deque', 'namedtuple']",
            "abc AsyncGenerator True",
            "abc AsyncIterable True",
            "abc AsyncIterator True",
            "abc Awaitable True",
            "abc Buffer True",
            "abc ByteString True",
            "abc Callable True",
            "abc Collection True",
            "abc Container True",
            "abc Coroutine True",
            "abc Generator True",
            "abc Hashable True",
            "abc ItemsView True",
            "abc Iterable True",
            "abc Iterator True",
            "abc KeysView True",
            "abc Mapping True",
            "abc MappingView True",
            "abc MutableMapping True",
            "abc MutableSequence True",
            "abc MutableSet True",
            "abc Reversible True",
            "abc Sequence True",
            "abc Set True",
            "abc Sized True",
            "abc ValuesView True",
            "abc __all__ False",
            "['AsyncGenerator', 'AsyncIterable', 'AsyncIterator', 'Awaitable', 'Buffer', 'ByteString', 'Callable', 'Collection', 'Container', 'Coroutine', 'Generator', 'Hashable', 'ItemsView', 'Iterable', 'Iterator', 'KeysView', 'Mapping', 'MappingView', 'MutableMapping', 'MutableSequence', 'MutableSet', 'Reversible', 'Sequence', 'Set', 'Sized', 'ValuesView', '__name__']",
        ]))
    );
}

#[test]
fn operator_sandbox_subset_keeps_export_surface_explicit() {
    assert_eq!(
        run_source(
            "import operator\nfor name in operator.__all__:\n    print(name, hasattr(operator, name))\nfor name in ['__file__', '__loader__', '__spec__', '__cached__', '_operator']:\n    print(name, hasattr(operator, name))\nprint(operator.__all__)\nprint(dir(operator))\nfor name in ['add', 'not_', 'iconcat', 'attrgetter', 'itemgetter', 'methodcaller', 'length_hint']:\n    value = getattr(operator, name)\n    print(name, getattr(value, '__name__', None), getattr(value, '__qualname__', None), getattr(value, '__module__', None))\nfor helper in [operator.attrgetter('x'), operator.itemgetter(0), operator.methodcaller('strip')]:\n    print(type(helper).__name__, hasattr(helper, '__dict__'), hasattr(helper, '__reduce__'), getattr(helper, '__module__', None))"
        ),
        Ok(output_lines(&[
            "abs True",
            "add True",
            "and_ True",
            "attrgetter True",
            "call True",
            "concat True",
            "contains True",
            "countOf True",
            "delitem True",
            "eq True",
            "floordiv True",
            "ge True",
            "getitem True",
            "gt True",
            "iadd True",
            "iand True",
            "iconcat True",
            "ifloordiv True",
            "ilshift True",
            "imatmul True",
            "imod True",
            "imul True",
            "index True",
            "indexOf True",
            "inv True",
            "invert True",
            "ior True",
            "ipow True",
            "irshift True",
            "is_ True",
            "is_none True",
            "is_not True",
            "is_not_none True",
            "isub True",
            "itemgetter True",
            "itruediv True",
            "ixor True",
            "le True",
            "length_hint True",
            "lshift True",
            "lt True",
            "matmul True",
            "methodcaller True",
            "mod True",
            "mul True",
            "ne True",
            "neg True",
            "not_ True",
            "or_ True",
            "pos True",
            "pow True",
            "rshift True",
            "setitem True",
            "sub True",
            "truediv True",
            "truth True",
            "xor True",
            "__file__ False",
            "__loader__ False",
            "__spec__ False",
            "__cached__ False",
            "_operator False",
            "['abs', 'add', 'and_', 'attrgetter', 'call', 'concat', 'contains', 'countOf', 'delitem', 'eq', 'floordiv', 'ge', 'getitem', 'gt', 'iadd', 'iand', 'iconcat', 'ifloordiv', 'ilshift', 'imatmul', 'imod', 'imul', 'index', 'indexOf', 'inv', 'invert', 'ior', 'ipow', 'irshift', 'is_', 'is_none', 'is_not', 'is_not_none', 'isub', 'itemgetter', 'itruediv', 'ixor', 'le', 'length_hint', 'lshift', 'lt', 'matmul', 'methodcaller', 'mod', 'mul', 'ne', 'neg', 'not_', 'or_', 'pos', 'pow', 'rshift', 'setitem', 'sub', 'truediv', 'truth', 'xor']",
            "['__abs__', '__add__', '__all__', '__and__', '__call__', '__concat__', '__contains__', '__delitem__', '__eq__', '__floordiv__', '__ge__', '__getitem__', '__gt__', '__iadd__', '__iand__', '__iconcat__', '__ifloordiv__', '__ilshift__', '__imatmul__', '__imod__', '__imul__', '__index__', '__inv__', '__invert__', '__ior__', '__ipow__', '__irshift__', '__isub__', '__itruediv__', '__ixor__', '__le__', '__lshift__', '__lt__', '__matmul__', '__mod__', '__mul__', '__name__', '__ne__', '__neg__', '__not__', '__or__', '__pos__', '__pow__', '__rshift__', '__setitem__', '__sub__', '__truediv__', '__xor__', 'abs', 'add', 'and_', 'attrgetter', 'call', 'concat', 'contains', 'countOf', 'delitem', 'eq', 'floordiv', 'ge', 'getitem', 'gt', 'iadd', 'iand', 'iconcat', 'ifloordiv', 'ilshift', 'imatmul', 'imod', 'imul', 'index', 'indexOf', 'inv', 'invert', 'ior', 'ipow', 'irshift', 'is_', 'is_none', 'is_not', 'is_not_none', 'isub', 'itemgetter', 'itruediv', 'ixor', 'le', 'length_hint', 'lshift', 'lt', 'matmul', 'methodcaller', 'mod', 'mul', 'ne', 'neg', 'not_', 'or_', 'pos', 'pow', 'rshift', 'setitem', 'sub', 'truediv', 'truth', 'xor']",
            "add add add operator",
            "not_ not_ not_ operator",
            "iconcat iconcat iconcat operator",
            "attrgetter attrgetter attrgetter operator",
            "itemgetter itemgetter itemgetter operator",
            "methodcaller methodcaller methodcaller operator",
            "length_hint length_hint length_hint operator",
            "attrgetter False False operator",
            "itemgetter False False operator",
            "methodcaller False False operator",
        ]))
    );
}

#[test]
fn sys_sandbox_subset_keeps_export_surface_explicit() {
    assert_eq!(
        run_source(
            "import sys\nfor name in ['argv', 'breakpointhook', '__breakpointhook__', 'builtin_module_names', 'byteorder', 'dont_write_bytecode', 'exc_info', 'flags', 'float_info', 'float_repr_style', 'get_int_max_str_digits', 'getdefaultencoding', 'hash_info', 'hexversion', 'implementation', 'is_finalizing', 'maxsize', 'modules', 'path', 'set_int_max_str_digits', 'stderr', 'stdin', 'stdout', 'version', 'version_info', 'warnoptions', '_getframe']:\n    print(name, hasattr(sys, name))\nfor name in ['platform', 'getrefcount', 'getallocatedblocks', 'settrace', 'gettrace', 'setswitchinterval', 'getfilesystemencoding', 'executable', 'prefix', 'base_prefix', '_base_executable', '__all__', '__file__']:\n    print(name, hasattr(sys, name))\nprint(dir(sys))\nprint(type(sys.argv).__name__, sys.argv)\nprint(type(sys.path).__name__, sys.path)\nprint(type(sys.warnoptions).__name__, all(type(option).__name__ == 'str' for option in sys.warnoptions))\nprint(type(sys.dont_write_bytecode).__name__, sys.dont_write_bytecode)\nprint(type(sys.modules).__name__, 'sys' in sys.modules, sys.modules['sys'] is sys)\nprint(type(sys.builtin_module_names).__name__, sys.builtin_module_names, sys.builtin_module_names == tuple(sorted(sys.builtin_module_names)))\nprint(type(sys.version_info).__name__, tuple(sys.version_info), sys.version_info.major, sys.version_info.releaselevel, type(sys.version_info.n_fields).__name__, sys.version_info.n_fields, sys.version_info.n_sequence_fields, sys.version_info.n_unnamed_fields)\nprint(type(sys.version_info).n_fields, type(sys.version_info).n_sequence_fields, type(sys.version_info).n_unnamed_fields)\nprint(type(sys.implementation).__name__, sys.implementation.name, sys.implementation.version == sys.version_info, type(sys.implementation.hexversion).__name__, type(sys.implementation.cache_tag).__name__)\nprint(sys.implementation.version.n_fields, sys.implementation.version.n_sequence_fields, sys.implementation.version.n_unnamed_fields)\nprint(type(sys.flags).__name__, sys.flags.debug, sys.flags.bytes_warning, sys.flags.dont_write_bytecode, sys.flags.dev_mode, sys.flags.hash_randomization, sys.flags.ignore_environment, sys.flags.inspect, sys.flags.interactive, sys.flags.no_user_site, sys.flags.no_site, sys.flags.isolated, sys.flags.optimize, sys.flags.quiet, sys.flags.utf8_mode, sys.flags.verbose)\nprint(type(sys.float_info).__name__, sys.float_info.max_exp, sys.float_info.radix)\nprint(type(sys.hash_info).__name__, sys.hash_info.width, sys.hash_info.algorithm)\nprint(type(sys.byteorder).__name__, sys.byteorder in ('little', 'big'))\nprint(type(sys.hexversion).__name__, sys.hexversion == sys.implementation.hexversion)\nprint(sys.maxsize)\nprint(sys.version)\nprint(sys.float_repr_style)\nprint(sys.get_int_max_str_digits())\nprint(sys.getdefaultencoding())\nprint(sys.is_finalizing())\nprint(sys.exc_info() == (None, None, None))\ntry:\n    raise ValueError('bad')\nexcept ValueError as error:\n    info = sys.exc_info()\n    print(info[0].__name__, info[1] is error, type(info[2]).__name__, info[1].__traceback__ is info[2])\nprint(sys.exc_info() == (None, None, None))\nfor name in ['stdin', 'stdout', 'stderr']:\n    value = getattr(sys, name)\n    print(name, type(value).__name__, sorted(vars(value).items()))"
        ),
        Ok(output_lines(&[
            "argv True",
            "breakpointhook True",
            "__breakpointhook__ True",
            "builtin_module_names True",
            "byteorder True",
            "dont_write_bytecode True",
            "exc_info True",
            "flags True",
            "float_info True",
            "float_repr_style True",
            "get_int_max_str_digits True",
            "getdefaultencoding True",
            "hash_info True",
            "hexversion True",
            "implementation True",
            "is_finalizing True",
            "maxsize True",
            "modules True",
            "path True",
            "set_int_max_str_digits True",
            "stderr True",
            "stdin True",
            "stdout True",
            "version True",
            "version_info True",
            "warnoptions True",
            "_getframe True",
            "platform False",
            "getrefcount False",
            "getallocatedblocks False",
            "settrace False",
            "gettrace False",
            "setswitchinterval False",
            "getfilesystemencoding False",
            "executable False",
            "prefix False",
            "base_prefix False",
            "_base_executable False",
            "__all__ False",
            "__file__ False",
            "['__breakpointhook__', '__name__', '_getframe', 'argv', 'breakpointhook', 'builtin_module_names', 'byteorder', 'dont_write_bytecode', 'exc_info', 'flags', 'float_info', 'float_repr_style', 'get_int_max_str_digits', 'getdefaultencoding', 'hash_info', 'hexversion', 'implementation', 'is_finalizing', 'maxsize', 'modules', 'path', 'set_int_max_str_digits', 'stderr', 'stdin', 'stdout', 'version', 'version_info', 'warnoptions']",
            "list []",
            "list ['']",
            "list True",
            "bool False",
            "dict True True",
            "tuple ('builtins', 'sys', 'time') True",
            "version_info (0, 1, 0, 'final', 0) 0 final int 5 5 0",
            "5 5 0",
            "SimpleNamespace minipython True int str",
            "5 5 0",
            "SimpleNamespace 0 0 0 False 0 0 0 0 0 0 0 0 0 0 0",
            "SimpleNamespace 1024 2",
            "SimpleNamespace 64 siphash13",
            "str True",
            "int True",
            "9223372036854775807",
            "minipython",
            "short",
            "4300",
            "utf-8",
            "False",
            "True",
            "ValueError True traceback True",
            "True",
            "stdin SimpleNamespace [('name', '<stdin>')]",
            "stdout SimpleNamespace [('name', '<stdout>')]",
            "stderr SimpleNamespace [('name', '<stderr>')]",
        ]))
    );
    assert_eq!(
        run_source(
            "import sys\nprint(type(sys.flags.n_fields).__name__, sys.flags.n_fields, sys.flags.n_sequence_fields, sys.flags.n_unnamed_fields)"
        ),
        Ok(output_lines(&["int 15 15 0"]))
    );
    assert_eq!(
        run_source(
            "import sys\nprint(type(sys.float_info.n_fields).__name__, sys.float_info.n_fields, sys.float_info.n_sequence_fields, sys.float_info.n_unnamed_fields)"
        ),
        Ok(output_lines(&["int 11 11 0"]))
    );
    assert_eq!(
        run_source(
            "import sys\nprint(type(sys.hash_info.n_fields).__name__, sys.hash_info.n_fields, sys.hash_info.n_sequence_fields, sys.hash_info.n_unnamed_fields)"
        ),
        Ok(output_lines(&["int 9 9 0"]))
    );
    assert_eq!(
        run_source(
            "import sys\nversion_helpers = ['_fields', '_field_defaults', '_asdict', '_replace', '_make', '__match_args__']\nversion_metadata = ['n_fields', 'n_sequence_fields', 'n_unnamed_fields']\nprint(any(hasattr(sys.version_info, name) for name in version_helpers), any(hasattr(type(sys.version_info), name) for name in version_helpers))\nprint(any(name in dir(sys.version_info) for name in version_helpers), any(name in dir(type(sys.version_info)) for name in version_helpers), all(name in dir(sys.version_info) for name in version_metadata), all(name in dir(type(sys.version_info)) for name in version_metadata))\nprint(sys.version_info.__getnewargs__() == (tuple(sys.version_info),), type(sys.version_info).__getnewargs__(sys.version_info) == (tuple(sys.version_info),))"
        ),
        Ok(output_lines(&[
            "False False",
            "False False True True",
            "True True",
        ]))
    );
}

#[test]
fn types_sandbox_subset_keeps_export_surface_explicit() {
    assert_eq!(
        run_source(
            "import types\nfor name in types.__all__:\n    print(name, hasattr(types, name))\nfor name in ['DictProxyType', 'StringTypes', 'StringType', 'ListType', 'TupleType', 'IntType', 'LongType', 'TypeType', 'ObjectType', 'XRangeType', 'FileType', 'SliceType', 'BufferType', 'ClassType', 'InstanceType', 'UnboundMethodType', 'CoroutineWrapper', 'new_class_internal', '__file__']:\n    print(name, hasattr(types, name))\nprint(types.__all__)\nprint(dir(types))\nfor name in ['SimpleNamespace', 'MappingProxyType', 'ModuleType', 'FunctionType', 'FrameLocalsProxyType', 'CapsuleType', 'CellType', 'LazyImportType', 'new_class', 'resolve_bases', 'prepare_class', 'get_original_bases', 'coroutine']:\n    value = getattr(types, name)\n    print(name, getattr(value, '__name__', None), getattr(value, '__module__', None))\nns = types.SimpleNamespace(x=1)\nprint(type(ns).__name__, repr(ns), sorted(vars(ns).items()))\nmp = types.MappingProxyType({'a': 1})\nprint(type(mp).__name__, len(mp), mp['a'], list(mp.keys()))\nclass Base:\n    pass\nMade = types.new_class('Made', (Base,), {}, lambda ns: ns.update({'x': 3}))\nprint(Made.__name__, issubclass(Made, Base), Made.x)\nprint(types.resolve_bases((Base,)))"
        ),
        Ok(output_lines(&[
            "AsyncGeneratorType True",
            "BuiltinFunctionType True",
            "BuiltinMethodType True",
            "CapsuleType True",
            "CellType True",
            "ClassMethodDescriptorType True",
            "CodeType True",
            "CoroutineType True",
            "DynamicClassAttribute True",
            "EllipsisType True",
            "FrameLocalsProxyType True",
            "FrameType True",
            "FunctionType True",
            "GeneratorType True",
            "GenericAlias True",
            "GetSetDescriptorType True",
            "LambdaType True",
            "LazyImportType True",
            "MappingProxyType True",
            "MemberDescriptorType True",
            "MethodDescriptorType True",
            "MethodType True",
            "MethodWrapperType True",
            "ModuleType True",
            "NoneType True",
            "NotImplementedType True",
            "SimpleNamespace True",
            "TracebackType True",
            "UnionType True",
            "WrapperDescriptorType True",
            "coroutine True",
            "get_original_bases True",
            "new_class True",
            "prepare_class True",
            "resolve_bases True",
            "DictProxyType False",
            "StringTypes False",
            "StringType False",
            "ListType False",
            "TupleType False",
            "IntType False",
            "LongType False",
            "TypeType False",
            "ObjectType False",
            "XRangeType False",
            "FileType False",
            "SliceType False",
            "BufferType False",
            "ClassType False",
            "InstanceType False",
            "UnboundMethodType False",
            "CoroutineWrapper False",
            "new_class_internal False",
            "__file__ False",
            "['AsyncGeneratorType', 'BuiltinFunctionType', 'BuiltinMethodType', 'CapsuleType', 'CellType', 'ClassMethodDescriptorType', 'CodeType', 'CoroutineType', 'DynamicClassAttribute', 'EllipsisType', 'FrameLocalsProxyType', 'FrameType', 'FunctionType', 'GeneratorType', 'GenericAlias', 'GetSetDescriptorType', 'LambdaType', 'LazyImportType', 'MappingProxyType', 'MemberDescriptorType', 'MethodDescriptorType', 'MethodType', 'MethodWrapperType', 'ModuleType', 'NoneType', 'NotImplementedType', 'SimpleNamespace', 'TracebackType', 'UnionType', 'WrapperDescriptorType', 'coroutine', 'get_original_bases', 'new_class', 'prepare_class', 'resolve_bases']",
            "['AsyncGeneratorType', 'BuiltinFunctionType', 'BuiltinMethodType', 'CapsuleType', 'CellType', 'ClassMethodDescriptorType', 'CodeType', 'CoroutineType', 'DynamicClassAttribute', 'EllipsisType', 'FrameLocalsProxyType', 'FrameType', 'FunctionType', 'GeneratorType', 'GenericAlias', 'GetSetDescriptorType', 'LambdaType', 'LazyImportType', 'MappingProxyType', 'MemberDescriptorType', 'MethodDescriptorType', 'MethodType', 'MethodWrapperType', 'ModuleType', 'NoneType', 'NotImplementedType', 'SimpleNamespace', 'TracebackType', 'UnionType', 'WrapperDescriptorType', '_GeneratorWrapper', '__all__', '__name__', 'coroutine', 'get_original_bases', 'new_class', 'prepare_class', 'resolve_bases']",
            "SimpleNamespace SimpleNamespace types",
            "MappingProxyType mappingproxy None",
            "ModuleType module builtins",
            "FunctionType function builtins",
            "FrameLocalsProxyType FrameLocalsProxy builtins",
            "CapsuleType PyCapsule None",
            "CellType CellType None",
            "LazyImportType lazy_import None",
            "new_class new_class None",
            "resolve_bases resolve_bases None",
            "prepare_class prepare_class None",
            "get_original_bases get_original_bases None",
            "coroutine coroutine None",
            "SimpleNamespace namespace(x=1) [('x', 1)]",
            "mappingproxy 1 1 ['a']",
            "Made True 3",
            "(<class Base>,)",
        ]))
    );
}

#[test]
fn builtins_sandbox_subset_keeps_export_surface_explicit() {
    assert_eq!(
        run_source(
            "import builtins\nfor name in ['abs', 'aiter', 'all', 'anext', 'any', 'ascii', 'bin', 'bool', 'breakpoint', 'bytearray', 'bytes', 'callable', 'chr', 'classmethod', 'compile', 'complex', 'delattr', 'dict', 'dir', 'divmod', 'enumerate', 'eval', 'exec', 'filter', 'float', 'format', 'frozenset', 'getattr', 'globals', 'hasattr', 'hash', 'hex', 'id', 'int', 'isinstance', 'issubclass', 'iter', 'len', 'list', 'locals', 'map', 'max', 'memoryview', 'min', 'next', 'object', 'oct', 'ord', 'pow', 'print', 'property', 'range', 'repr', 'reversed', 'round', 'set', 'setattr', 'slice', 'sorted', 'staticmethod', 'str', 'sum', 'super', 'tuple', 'type', 'vars', 'zip']:\n    print(name, hasattr(builtins, name))\nfor name in ['BaseException', 'Exception', 'TypeError', 'ValueError', 'OSError', 'SyntaxError', 'StopIteration', 'StopAsyncIteration', 'ImportError', 'ModuleNotFoundError', 'ExceptionGroup', 'BaseExceptionGroup']:\n    print(name, hasattr(builtins, name))\nfor name in ['open', 'input', 'help', 'license', 'credits', 'exit', 'quit', '__all__', '__file__']:\n    print(name, hasattr(builtins, name))\nprint('__import__', hasattr(builtins, '__import__'))\nprint('__build_class__', hasattr(builtins, '__build_class__'))\nprint('__debug__', hasattr(builtins, '__debug__'))\nprint(dir(builtins))"
        ),
        Ok(output_lines(&[
            "abs True",
            "aiter True",
            "all True",
            "anext True",
            "any True",
            "ascii True",
            "bin True",
            "bool True",
            "breakpoint True",
            "bytearray True",
            "bytes True",
            "callable True",
            "chr True",
            "classmethod True",
            "compile True",
            "complex True",
            "delattr True",
            "dict True",
            "dir True",
            "divmod True",
            "enumerate True",
            "eval True",
            "exec True",
            "filter True",
            "float True",
            "format True",
            "frozenset True",
            "getattr True",
            "globals True",
            "hasattr True",
            "hash True",
            "hex True",
            "id True",
            "int True",
            "isinstance True",
            "issubclass True",
            "iter True",
            "len True",
            "list True",
            "locals True",
            "map True",
            "max True",
            "memoryview True",
            "min True",
            "next True",
            "object True",
            "oct True",
            "ord True",
            "pow True",
            "print True",
            "property True",
            "range True",
            "repr True",
            "reversed True",
            "round True",
            "set True",
            "setattr True",
            "slice True",
            "sorted True",
            "staticmethod True",
            "str True",
            "sum True",
            "super True",
            "tuple True",
            "type True",
            "vars True",
            "zip True",
            "BaseException True",
            "Exception True",
            "TypeError True",
            "ValueError True",
            "OSError True",
            "SyntaxError True",
            "StopIteration True",
            "StopAsyncIteration True",
            "ImportError True",
            "ModuleNotFoundError True",
            "ExceptionGroup True",
            "BaseExceptionGroup True",
            "open False",
            "input False",
            "help False",
            "license False",
            "credits False",
            "exit False",
            "quit False",
            "__all__ False",
            "__file__ False",
            "__import__ True",
            "__build_class__ True",
            "__debug__ True",
            "['ArithmeticError', 'AssertionError', 'AttributeError', 'BaseException', 'BaseExceptionGroup', 'BlockingIOError', 'BytesWarning', 'DeprecationWarning', 'EOFError', 'Ellipsis', 'EncodingWarning', 'Exception', 'ExceptionGroup', 'FileExistsError', 'FileNotFoundError', 'FloatingPointError', 'FutureWarning', 'GeneratorExit', 'ImportError', 'ImportWarning', 'IndentationError', 'IndexError', 'InterruptedError', 'IsADirectoryError', 'KeyError', 'KeyboardInterrupt', 'LookupError', 'MemoryError', 'ModuleNotFoundError', 'NameError', 'NotADirectoryError', 'NotImplemented', 'NotImplementedError', 'OSError', 'OverflowError', 'PendingDeprecationWarning', 'PermissionError', 'ProcessLookupError', 'RecursionError', 'ReferenceError', 'ResourceWarning', 'RuntimeError', 'RuntimeWarning', 'StopAsyncIteration', 'StopIteration', 'SyntaxError', 'SyntaxWarning', 'SystemError', 'SystemExit', 'TabError', 'TimeoutError', 'TypeError', 'UnicodeDecodeError', 'UnicodeEncodeError', 'UnicodeError', 'UnicodeTranslateError', 'UnicodeWarning', 'UserWarning', 'ValueError', 'Warning', 'ZeroDivisionError', '__build_class__', '__debug__', '__import__', '__name__', 'abs', 'aiter', 'all', 'anext', 'any', 'ascii', 'bin', 'bool', 'breakpoint', 'bytearray', 'bytes', 'callable', 'chr', 'classmethod', 'compile', 'complex', 'delattr', 'dict', 'dir', 'divmod', 'enumerate', 'eval', 'exec', 'filter', 'float', 'format', 'frozenset', 'getattr', 'globals', 'hasattr', 'hash', 'hex', 'id', 'int', 'isinstance', 'issubclass', 'iter', 'len', 'list', 'locals', 'map', 'max', 'memoryview', 'min', 'next', 'object', 'oct', 'ord', 'pow', 'print', 'property', 'range', 'repr', 'reversed', 'round', 'set', 'setattr', 'slice', 'sorted', 'staticmethod', 'str', 'sum', 'super', 'tuple', 'type', 'vars', 'zip']",
        ]))
    );
}

#[test]
fn sandbox_policy_required_stdlib_allow_list_excludes_compatibility_shims() {
    let sandbox = TestSandboxDir::new("required-stdlib-excludes-shims");
    let policy =
        SandboxPolicy::allow_stdlib_modules(REQUIRED_SANDBOX_STDLIB_MODULES.to_vec()).unwrap();

    for module in COMPATIBILITY_STDLIB_MODULES {
        let error = run_source_with_sandbox_dir_and_policy(
            &format!("import {module}"),
            sandbox.path(),
            policy.clone(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            format!("runtime error: ModuleNotFoundError: No module named '{module}'")
        );
    }
}

#[test]
fn sandbox_policy_requires_explicit_allow_for_extra_stdlib_shims() {
    let sandbox = TestSandboxDir::new("allow-extra-stdlib-shim");

    assert_eq!(
        run_source_with_sandbox_dir_and_policy(
            "import weakref",
            sandbox.path(),
            SandboxPolicy::deny_stdlib(),
        ),
        Err("runtime error: ModuleNotFoundError: No module named 'weakref'".to_string())
    );

    assert_eq!(
        run_source_with_sandbox_dir_and_policy(
            "import weakref\nprint(hasattr(weakref, 'ref'))",
            sandbox.path(),
            SandboxPolicy::allow_stdlib_modules(["weakref"]).unwrap(),
        ),
        Ok(output_lines(&["True"]))
    );
}

#[test]
fn out_of_scope_host_io_network_and_process_surfaces_stay_unavailable() {
    assert_eq!(
        run_source(
            "for name in ['open', 'input']:\n    try:\n        eval(name)\n    except NameError as error:\n        print(name, error.__class__.__name__)"
        ),
        Ok(output_lines(&["open NameError", "input NameError"]))
    );

    assert_eq!(
        run_source(
            "import builtins\nfor name in ['open', 'input', 'help', 'license', 'credits', 'exit', 'quit']:\n    print(name, hasattr(builtins, name))\nprint('breakpoint', hasattr(builtins, 'breakpoint'))\nprint('__all__', hasattr(builtins, '__all__'))"
        ),
        Ok(output_lines(&[
            "open False",
            "input False",
            "help False",
            "license False",
            "credits False",
            "exit False",
            "quit False",
            "breakpoint True",
            "__all__ False",
        ]))
    );

    for module in [
        "asyncio",
        "http",
        "multiprocessing",
        "ssl",
        "socket",
        "subprocess",
        "signal",
        "threading",
        "pty",
        "urllib",
        "_ssl",
        "_socket",
        "_ctypes",
        "_testcapi",
        "locale",
        "pdb",
    ] {
        assert_eq!(
            run_source(&format!("import {module}")),
            Err(format!(
                "runtime error: ModuleNotFoundError: No module named '{module}'"
            ))
        );
    }

    assert_eq!(
        run_source(
            "import sys\nfor name in ['getrefcount', 'gettotalrefcount', 'getallocatedblocks']:\n    print(name, hasattr(sys, name))\ndef f():\n    pass\nfor name in ['co_code', 'co_stacksize']:\n    print(name, hasattr(f.__code__, name))"
        ),
        Ok(output_lines(&[
            "getrefcount False",
            "gettotalrefcount False",
            "getallocatedblocks False",
            "co_code False",
            "co_stacksize False",
        ]))
    );

    assert_eq!(
        run_source(
            "import sys\nfor expr in [lambda: breakpoint(), lambda: sys.breakpointhook(), lambda: sys.__breakpointhook__(1, key=2)]:\n    print(expr())"
        ),
        Ok(output_lines(&["None", "None", "None"]))
    );
}

#[test]
fn sandbox_required_stdlib_allow_list_keeps_stop_line_modules_blocked() {
    let sandbox = TestSandboxDir::new("allow-required-stdlib-stop-lines");
    let policy =
        SandboxPolicy::allow_stdlib_modules(REQUIRED_SANDBOX_STDLIB_MODULES.to_vec()).unwrap();

    for module in [
        "asyncio",
        "http",
        "multiprocessing",
        "ssl",
        "socket",
        "subprocess",
        "signal",
        "threading",
        "pty",
        "urllib",
        "_ssl",
        "_socket",
        "_ctypes",
        "_testcapi",
        "locale",
        "pdb",
    ] {
        assert_eq!(
            run_source_with_sandbox_dir_and_policy(
                &format!("import {module}"),
                sandbox.path(),
                policy.clone(),
            ),
            Err(format!(
                "runtime error: ModuleNotFoundError: No module named '{module}'"
            ))
        );
    }
}

#[test]
fn sandbox_policy_propagates_into_virtual_modules() {
    assert_eq!(
        run_source_with_virtual_modules_and_policy(
            "import tools\nprint(tools.VALUE)",
            vec![VirtualModule::module("tools", "VALUE = 5")],
            SandboxPolicy::deny_stdlib(),
        ),
        Ok(output_lines(&["5"]))
    );

    assert_eq!(
        run_source_with_virtual_modules_and_policy(
            "import tools",
            vec![VirtualModule::module("tools", "import math")],
            SandboxPolicy::deny_stdlib(),
        ),
        Err("runtime error: ModuleNotFoundError: No module named 'math'".to_string())
    );
}

#[test]
fn sandbox_policy_checks_sys_modules_cache() {
    let sandbox = TestSandboxDir::new("sys-modules-policy");
    let policy = SandboxPolicy::allow_stdlib_modules(["sys"]).unwrap();

    assert_eq!(
        run_source_with_sandbox_dir_and_policy(
            "import sys\nsys.modules['math'] = 'fake'\nimport math",
            sandbox.path(),
            policy,
        ),
        Err("runtime error: ModuleNotFoundError: No module named 'math'".to_string())
    );
}

#[test]
fn rejects_invalid_sandbox_policy_module_names() {
    assert_eq!(
        SandboxPolicy::allow_stdlib_modules(["bad-name"]).unwrap_err(),
        "sandbox error: invalid stdlib module name 'bad-name'".to_string()
    );
}

#[test]
fn reports_import_errors() {
    assert_eq!(
        run_source("import missing"),
        Err("runtime error: ModuleNotFoundError: No module named 'missing'".to_string())
    );
    assert_eq!(
        run_source(
            "try:\n    import missing\nexcept ModuleNotFoundError as error:\n    print(error.__class__.__name__, error)"
        ),
        Ok(output_lines(&[
            "ModuleNotFoundError No module named 'missing'"
        ]))
    );
    assert_eq!(
        run_source("from sys import missing"),
        Err("runtime error: ImportError: cannot import name 'missing' from 'sys'".to_string())
    );
    assert_eq!(
        run_source(
            "try:\n    from sys import missing\nexcept ImportError as error:\n    print(error.__class__.__name__, error)"
        ),
        Ok(output_lines(&[
            "ImportError cannot import name 'missing' from 'sys'"
        ]))
    );
    assert_eq!(
        run_source("import sys\nsys.modules['math'] = None\nimport math"),
        Err(
            "runtime error: ModuleNotFoundError: import of math halted; None in sys.modules"
                .to_string()
        )
    );
    assert_eq!(
        run_source("import sys\nsys.modules['os.path'] = None\nimport os.path"),
        Err(
            "runtime error: ModuleNotFoundError: import of os.path halted; None in sys.modules"
                .to_string()
        )
    );
    assert_eq!(
        run_source("import sys\nsys.modules['os'] = 'changed'\nimport os.path"),
        Err(
            "runtime error: ModuleNotFoundError: No module named 'os.path'; 'os' is not a package"
                .to_string()
        )
    );
    assert_eq!(
        run_source("import sys\nsys.modules['os'] = sys\n__import__('os.path', fromlist=['sep'])"),
        Err(
            "runtime error: ModuleNotFoundError: No module named 'os.path'; 'os' is not a package"
                .to_string()
        )
    );
    assert_eq!(
        run_source("__import__('math', level=-1)"),
        Err("runtime error: ValueError: level must be >= 0".to_string())
    );
    assert_eq!(
        run_source("__import__('math', level=1.2)"),
        Err("runtime error: TypeError: integer argument expected, got float".to_string())
    );
    assert_eq!(
        run_source("__import__('math', level='1')"),
        Err("runtime error: TypeError: an integer is required (got type str)".to_string())
    );
    assert_eq!(
        run_source("print(__import__('math', level=False).__name__)"),
        Ok(vec!["math".to_string()])
    );
    assert_eq!(
        run_source("from . import path"),
        Err(
            "runtime error: ImportError: attempted relative import with no known parent package"
                .to_string()
        )
    );
    assert_eq!(
        run_source(
            "ns = {'__name__': 'test.typinganndata.runner', '__package__': 'test.typinganndata'}\nexec('from . import ann_module\\nprint(ann_module.__name__)', ns)"
        ),
        Ok(vec!["test.typinganndata.ann_module".to_string()])
    );
    assert_eq!(
        run_source(
            "ns = {'__name__': 'test.typinganndata.runner', '__package__': 'test.typinganndata'}\nexec('from .ann_module import M\\nprint(M.__name__)', ns)"
        ),
        Ok(vec!["M".to_string()])
    );
    assert_eq!(
        run_source(
            "ns = {'__name__': 'test.typinganndata.runner', '__package__': 'test.typinganndata'}\nexec(\"print(__import__('ann_module', globals(), locals(), ['*'], 1).__name__)\", ns)"
        ),
        Ok(vec!["test.typinganndata.ann_module".to_string()])
    );
    assert_eq!(
        run_source(
            "ns = {'__name__': 'test.typinganndata.runner', '__package__': 'test.typinganndata'}\nexec('from ...pkg import name', ns)"
        ),
        Err(
            "runtime error: ImportError: attempted relative import beyond top-level package"
                .to_string()
        )
    );
    assert_eq!(
        run_source("def f():\n    lazy import sys"),
        Err("compile error: lazy import not allowed inside functions".to_string())
    );
    assert_eq!(
        run_source("class C:\n    lazy from sys import path"),
        Err("compile error: lazy from ... import not allowed inside classes".to_string())
    );
    assert_eq!(
        run_source("try:\n    lazy import sys\nexcept:\n    pass"),
        Err("compile error: lazy import not allowed inside try/except blocks".to_string())
    );
    assert_eq!(
        run_source("lazy from math import *"),
        Err("compile error: lazy from ... import * is not allowed".to_string())
    );
    assert_eq!(
        run_source("import"),
        Err("parse error: Expected one or more names after 'import'".to_string())
    );
    assert_eq!(
        run_source("from sys import path,"),
        Err("parse error: trailing comma not allowed without surrounding parentheses".to_string())
    );
    assert_eq!(
        run_source("import sys from time"),
        Err("parse error: Did you mean to use 'from ... import ...' instead?".to_string())
    );
    assert_eq!(
        run_source("import sys as 1"),
        Err("parse error: cannot use literal as import target".to_string())
    );
    assert_eq!(
        run_source("from sys import path as None"),
        Err("parse error: cannot use literal as import target".to_string())
    );
}

#[test]
fn runs_generator_yield_with_next() {
    assert_eq!(
        run_source(
            "def g():\n    yield 1\n    yield 2\ngen = g()\nprint(next(gen))\nprint(next(gen))"
        ),
        Ok(vec!["1".to_string(), "2".to_string()])
    );
}

#[test]
fn lambda_with_yield_is_generator() {
    assert_eq!(
        run_source(
            "g = (lambda: (yield))()\nprint(next(g))\ntry:\n    g.send(42)\nexcept StopIteration as done:\n    print(done)\nprint(list((lambda: (yield))()))"
        ),
        Ok(vec![
            "None".to_string(),
            "42".to_string(),
            "[None]".to_string(),
        ])
    );
}

#[test]
fn runs_generator_body_lazily() {
    assert_eq!(
        run_source(
            "def g():\n    print(\"start\")\n    yield 1\n    print(\"resume\")\n    yield 2\ngen = g()\nprint(\"after call\")\nprint(next(gen))\nprint(next(gen))"
        ),
        Ok(vec![
            "after call".to_string(),
            "start".to_string(),
            "1".to_string(),
            "resume".to_string(),
            "2".to_string(),
        ])
    );
}

#[test]
fn runs_for_loop_over_generator() {
    assert_eq!(
        run_source(
            "def g():\n    for x in range(3):\n        yield x\nfor x in g():\n    print(x)"
        ),
        Ok(vec!["0".to_string(), "1".to_string(), "2".to_string()])
    );
}

#[test]
fn runs_generator_expressions() {
    assert_eq!(
        run_source(
            "g = (i * i for i in range(4))\nprint(next(g))\nprint(next(g))\nfor x in g:\n    print(x)"
        ),
        Ok(vec![
            "0".to_string(),
            "1".to_string(),
            "4".to_string(),
            "9".to_string(),
        ])
    );
    assert_eq!(
        run_source("for x in (i for i in range(5) if i % 2):\n    print(x)"),
        Ok(vec!["1".to_string(), "3".to_string()])
    );
    assert_eq!(
        run_source("for pair in ((i, j) for i in range(3) for j in range(i)):\n    print(pair)"),
        Ok(vec![
            "(1, 0)".to_string(),
            "(2, 0)".to_string(),
            "(2, 1)".to_string()
        ])
    );
    assert_eq!(
        run_source("print(next(i for i in range(3)))"),
        Ok(vec!["0".to_string()])
    );
}

#[test]
fn generator_expression_binds_outer_iterable_at_creation() {
    assert_eq!(
        run_source("x = 4\ng = (i for i in range(x))\nx = 2\nfor i in g:\n    print(i)"),
        Ok(vec![
            "0".to_string(),
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
        ])
    );
}

#[test]
fn catches_stop_iteration_from_next() {
    assert_eq!(
        run_source(
            "def g():\n    yield 1\ngen = g()\nprint(next(gen))\ntry:\n    print(next(gen))\nexcept StopIteration:\n    print(\"done\")"
        ),
        Ok(vec!["1".to_string(), "done".to_string()])
    );
}

#[test]
fn runs_yield_from_expression() {
    assert_eq!(
        run_source(
            "def g():\n    yield from [1, 2]\n    yield 3\nfor value in g():\n    print(value)"
        ),
        Ok(vec!["1".to_string(), "2".to_string(), "3".to_string()])
    );
    assert_eq!(
        run_source(
            "def inner():\n    yield \"a\"\n    yield \"b\"\ndef outer():\n    yield 0\n    yield from inner()\n    yield 3\nfor value in outer():\n    print(value)"
        ),
        Ok(vec![
            "0".to_string(),
            "a".to_string(),
            "b".to_string(),
            "3".to_string(),
        ])
    );
    assert_eq!(
        run_source("def g():\n    yield from ()\n    yield \"done\"\nprint(next(g()))"),
        Ok(vec!["done".to_string()])
    );
    assert_eq!(
        run_source(
            "def g():\n    result = yield from [1]\n    yield result\nfor value in g():\n    print(value)"
        ),
        Ok(vec!["1".to_string(), "None".to_string()])
    );
    assert_eq!(
        run_source(
            "def inner():\n    yield 1\n    return 42\ndef outer():\n    result = yield from inner()\n    yield result\nfor value in outer():\n    print(value)"
        ),
        Ok(vec!["1".to_string(), "42".to_string()])
    );
}

#[test]
fn runs_generator_send_values() {
    assert_eq!(
        run_source(
            "def g():\n    value = yield \"ready\"\n    yield value\ngen = g()\nprint(next(gen))\nprint(gen.send(42))"
        ),
        Ok(vec!["ready".to_string(), "42".to_string()])
    );
    assert_eq!(
        run_source(
            "def g():\n    value = yield \"ready\"\n    yield value\ngen = g()\nprint(gen.send(None))\nprint(gen.send(\"done\"))"
        ),
        Ok(vec!["ready".to_string(), "done".to_string()])
    );
    assert_eq!(
        run_source(
            "def g():\n    yield 1\ngen = g()\ntry:\n    gen.send(1)\nexcept TypeError as error:\n    print(error)"
        ),
        Ok(vec![
            "can't send non-None value to a just-started generator".to_string()
        ])
    );
}

#[test]
fn runs_generator_throw_values() {
    assert_eq!(
        run_source(
            "def g():\n    try:\n        yield \"ready\"\n    except ValueError as error:\n        yield error\ngen = g()\nprint(next(gen))\nprint(gen.throw(ValueError(\"bad\")))"
        ),
        Ok(vec!["ready".to_string(), "bad".to_string()])
    );
    assert_eq!(
        run_source(
            "def g():\n    yield 1\ngen = g()\nprint(next(gen))\ntry:\n    gen.throw(ValueError(\"boom\"))\nexcept ValueError as error:\n    print(error)\ntry:\n    next(gen)\nexcept StopIteration:\n    print(\"done\")"
        ),
        Ok(vec![
            "1".to_string(),
            "boom".to_string(),
            "done".to_string()
        ])
    );
}

#[test]
fn runs_generator_close_values() {
    assert_eq!(
        run_source(
            "def g():\n    try:\n        yield 1\n    finally:\n        print(\"cleanup\")\ngen = g()\nprint(next(gen))\nprint(gen.close())\ntry:\n    next(gen)\nexcept StopIteration:\n    print(\"done\")"
        ),
        Ok(vec![
            "1".to_string(),
            "cleanup".to_string(),
            "None".to_string(),
            "done".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "def g():\n    print(\"body\")\n    yield 1\ngen = g()\nprint(gen.close())\ntry:\n    next(gen)\nexcept StopIteration:\n    print(\"done\")"
        ),
        Ok(vec!["None".to_string(), "done".to_string()])
    );
    assert_eq!(
        run_source(
            "def g():\n    try:\n        yield 1\n    finally:\n        raise ValueError(\"bad\")\ngen = g()\nprint(next(gen))\ntry:\n    gen.close()\nexcept ValueError as error:\n    print(error)"
        ),
        Ok(vec!["1".to_string(), "bad".to_string()])
    );
    assert_eq!(
        run_source(
            "def g():\n    try:\n        yield 1\n    except GeneratorExit:\n        yield 2\ngen = g()\nprint(next(gen))\ntry:\n    gen.close()\nexcept RuntimeError as error:\n    print(error)"
        ),
        Ok(vec![
            "1".to_string(),
            "generator ignored GeneratorExit".to_string(),
        ])
    );
}

#[test]
fn runs_async_function_and_await_expression() {
    assert_eq!(
        run_source(
            "async def value():\n    return 3\nasync def main():\n    result = await value()\n    print(result)\n    return 5\ncoro = main()\ntry:\n    coro.send(None)\nexcept StopIteration as done:\n    print(done)"
        ),
        Ok(vec!["3".to_string(), "5".to_string()])
    );
    assert_eq!(
        run_source(
            "async def f():\n    print(\"body\")\n    return 1\ncoro = f()\nprint(\"after call\")\ntry:\n    coro.send(None)\nexcept StopIteration as done:\n    print(done)"
        ),
        Ok(vec![
            "after call".to_string(),
            "body".to_string(),
            "1".to_string(),
        ])
    );
}

#[test]
fn reports_async_await_errors() {
    assert_eq!(
        run_source("await 1"),
        Err("compile error: 'await' outside async function".to_string())
    );
    assert_eq!(
        run_source("def f():\n    await 1"),
        Err("compile error: 'await' outside async function".to_string())
    );
    assert_eq!(
        run_source(
            "async def main():\n    await 1\ntry:\n    main().send(None)\nexcept TypeError as error:\n    print(error)"
        ),
        Ok(vec![
            "object int can't be used in 'await' expression".to_string()
        ])
    );
}

#[test]
fn runs_coroutine_throw_and_close_methods() {
    assert_eq!(
        run_source(
            "async def f():\n    print(\"body\")\n    return 1\ncoro = f()\ntry:\n    coro.throw(ValueError(\"bad\"))\nexcept ValueError as error:\n    print(error)\ntry:\n    coro.send(None)\nexcept RuntimeError as error:\n    print(error)"
        ),
        Ok(vec![
            "bad".to_string(),
            "cannot reuse already awaited coroutine".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "async def f():\n    return 1\ncoro = f()\ntry:\n    coro.send(None)\nexcept StopIteration as done:\n    print(done)\ntry:\n    coro.throw(ValueError(\"bad\"))\nexcept RuntimeError as error:\n    print(error)"
        ),
        Ok(vec![
            "1".to_string(),
            "cannot reuse already awaited coroutine".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "async def f():\n    print(\"body\")\n    return 1\ncoro = f()\nprint(coro.close())\ntry:\n    coro.send(None)\nexcept RuntimeError as error:\n    print(error)"
        ),
        Ok(vec![
            "None".to_string(),
            "cannot reuse already awaited coroutine".to_string(),
        ])
    );
}

#[test]
fn runs_collections_abc_coroutine_mixins() {
    assert_eq!(
        run_source(
            "from collections.abc import Coroutine\nclass DefaultCoro(Coroutine):\n    def __await__(self):\n        yield\n    def send(self, value):\n        return super().send(value)\n    def throw(self, typ, val=None, tb=None):\n        return super().throw(typ, val, tb)\ncoro = DefaultCoro()\ntry:\n    coro.send(None)\nexcept StopIteration:\n    print('send-stopped')\ntry:\n    coro.throw(ValueError('bad'))\nexcept ValueError as error:\n    print(error)\nprint(coro.close())\nclass IgnoreExit(Coroutine):\n    def __await__(self):\n        yield\n    def send(self, value):\n        return value\n    def throw(self, typ, val=None, tb=None):\n        return 'ignored'\ntry:\n    IgnoreExit().close()\nexcept RuntimeError as error:\n    print(error)"
        ),
        Ok(vec![
            "send-stopped".to_string(),
            "bad".to_string(),
            "None".to_string(),
            "coroutine ignored GeneratorExit".to_string(),
        ])
    );
}

#[test]
fn runs_collections_abc_async_iterator_mixin() {
    assert_eq!(
        run_source(
            "from collections.abc import AsyncIterable, AsyncIterator\nclass AI(AsyncIterator):\n    async def __anext__(self):\n        raise StopAsyncIteration\nai = AI()\nprint(isinstance(ai, AsyncIterable), isinstance(ai, AsyncIterator))\nprint(ai.__aiter__() is ai)\nprint(AsyncIterator.__aiter__(ai) is ai)"
        ),
        Ok(vec![
            "True True".to_string(),
            "True".to_string(),
            "True".to_string(),
        ])
    );
}

#[test]
fn runs_collections_abc_async_generator_core_mixins() {
    assert_eq!(
        run_source(
            "from collections.abc import AsyncGenerator, AsyncIterator\nclass AGen(AsyncGenerator):\n    async def asend(self, value):\n        return value\n    async def athrow(self, typ, val=None, tb=None):\n        pass\nasync def main():\n    agen = AGen()\n    print(isinstance(agen, AsyncIterator), isinstance(agen, AsyncGenerator))\n    print(agen.__aiter__() is agen)\n    print(await agen.__anext__())\n    print(await agen.asend(2))\ncoro = main()\ntry:\n    coro.send(None)\nexcept StopIteration:\n    pass"
        ),
        Ok(vec![
            "True True".to_string(),
            "True".to_string(),
            "None".to_string(),
            "2".to_string(),
        ])
    );
}

#[test]
fn runs_collections_abc_async_generator_throw_and_close_mixins() {
    assert_eq!(
        run_source(
            "from collections.abc import AsyncGenerator\nclass MinimalAGen(AsyncGenerator):\n    async def asend(self, value):\n        return value\n    async def athrow(self, typ, val=None, tb=None):\n        await super().athrow(typ, val, tb)\nclass FailOnClose(AsyncGenerator):\n    async def asend(self, value):\n        return value\n    async def athrow(self, *args):\n        raise ValueError('bad close')\nclass IgnoreGeneratorExit(AsyncGenerator):\n    async def asend(self, value):\n        return value\n    async def athrow(self, *args):\n        pass\nasync def main():\n    mgen = MinimalAGen()\n    print(await mgen.aclose())\n    try:\n        await mgen.athrow(ValueError)\n    except ValueError:\n        print('athrow ValueError')\n    try:\n        await mgen.athrow(ValueError, ValueError('explicit'), None)\n    except ValueError as error:\n        print('athrow explicit', error)\n    try:\n        await mgen.athrow(ValueError, None, 5)\n    except TypeError as error:\n        print('athrow traceback', error)\n    try:\n        await FailOnClose().aclose()\n    except ValueError as error:\n        print('close-error', error)\n    try:\n        await IgnoreGeneratorExit().aclose()\n    except RuntimeError as error:\n        print('close-runtime', error)\ncoro = main()\ntry:\n    coro.send(None)\nexcept StopIteration:\n    pass"
        ),
        Ok(vec![
            "None".to_string(),
            "athrow ValueError".to_string(),
            "athrow explicit explicit".to_string(),
            "athrow traceback __traceback__ must be a traceback or None".to_string(),
            "close-error bad close".to_string(),
            "close-runtime asynchronous generator ignored GeneratorExit".to_string(),
        ])
    );
}

#[test]
fn runs_async_for_loop() {
    let source = "class AsyncCounter:\n    def __init__(self, stop):\n        self.current = 0\n        self.stop = stop\n    def __aiter__(self):\n        return self\n    async def __anext__(self):\n        if self.current >= self.stop:\n            raise StopAsyncIteration\n        value = self.current\n        self.current += 1\n        return value\nasync def main():\n    async for value in AsyncCounter(3):\n        print(value)\n    async for value in AsyncCounter(0):\n        print(\"skip\")\n    else:\n        print(\"empty\")\n    async for value in AsyncCounter(3):\n        print(value)\n        break\n    else:\n        print(\"else\")\ncoro = main()\ntry:\n    coro.send(None)\nexcept StopIteration:\n    print(\"done\")";
    assert_eq!(
        run_source(source),
        Ok(vec![
            "0".to_string(),
            "1".to_string(),
            "2".to_string(),
            "empty".to_string(),
            "0".to_string(),
            "done".to_string(),
        ])
    );
}

#[test]
fn runs_async_with_statement() {
    let source = "class AsyncManager:\n    def __init__(self, name, suppress=False):\n        self.name = name\n        self.suppress = suppress\n    async def __aenter__(self):\n        print(\"enter\", self.name)\n        return self.name\n    async def __aexit__(self, exc_type, exc, traceback):\n        if exc_type is None:\n            print(\"exit\", self.name, exc_type, exc)\n        else:\n            print(\"exit\", self.name, exc_type.__name__, exc)\n        return self.suppress\nasync def main():\n    async with AsyncManager(\"a\") as value:\n        print(\"body\", value)\n    async with AsyncManager(\"b\") as b, AsyncManager(\"c\") as c:\n        print(\"body2\", b, c)\n    try:\n        async with AsyncManager(\"d\", True):\n            raise ValueError(\"bad\")\n        print(\"suppressed\")\n    except ValueError as error:\n        print(\"unexpected\", error)\n    try:\n        async with AsyncManager(\"e\"):\n            raise ValueError(\"boom\")\n    except ValueError as error:\n        print(\"caught\", error)\ncoro = main()\ntry:\n    coro.send(None)\nexcept StopIteration:\n    print(\"done\")";
    assert_eq!(
        run_source(source),
        Ok(vec![
            "enter a".to_string(),
            "body a".to_string(),
            "exit a None None".to_string(),
            "enter b".to_string(),
            "enter c".to_string(),
            "body2 b c".to_string(),
            "exit c None None".to_string(),
            "exit b None None".to_string(),
            "enter d".to_string(),
            "exit d ValueError bad".to_string(),
            "suppressed".to_string(),
            "enter e".to_string(),
            "exit e ValueError boom".to_string(),
            "caught boom".to_string(),
            "done".to_string(),
        ])
    );
}

#[test]
fn runs_parenthesized_async_with_items() {
    let source = "class AsyncManager:\n    def __init__(self, name):\n        self.name = name\n    async def __aenter__(self):\n        print(\"enter\", self.name)\n        return self.name\n    async def __aexit__(self, exc_type, exc, traceback):\n        print(\"exit\", self.name)\nasync def main():\n    async with (\n        AsyncManager(\"a\") as a,\n        AsyncManager(\"b\") as b,\n    ):\n        print(\"body\", a, b)\ncoro = main()\ntry:\n    coro.send(None)\nexcept StopIteration:\n    print(\"done\")";
    assert_eq!(
        run_source(source),
        Ok(vec![
            "enter a".to_string(),
            "enter b".to_string(),
            "body a b".to_string(),
            "exit b".to_string(),
            "exit a".to_string(),
            "done".to_string(),
        ])
    );
}

#[test]
fn reports_unsupported_yield_forms() {
    assert_eq!(
        run_source("yield 1"),
        Err("compile error: yield outside function".to_string())
    );
}

#[test]
fn runs_match_literal_cases() {
    assert_eq!(
        run_source(
            "x = 2\nmatch x:\n    case 1:\n        print(\"one\")\n    case 2:\n        print(\"two\")\n    case _:\n        print(\"other\")\nprint(\"after\")"
        ),
        Ok(vec!["two".to_string(), "after".to_string()])
    );
}

#[test]
fn runs_match_wildcard_case() {
    assert_eq!(
        run_source(
            "match \"missing\":\n    case \"found\":\n        print(\"found\")\n    case _:\n        print(\"default\")"
        ),
        Ok(vec!["default".to_string()])
    );
}

#[test]
fn runs_match_singleton_and_negative_literal_patterns() {
    assert_eq!(
        run_source(
            "match None:\n    case True:\n        print(\"true\")\n    case None:\n        print(\"none\")\nmatch -1:\n    case -1:\n        print(\"negative\")"
        ),
        Ok(vec!["none".to_string(), "negative".to_string()])
    );
    assert_eq!(
        run_source(
            "match 0:\n    case False:\n        print(\"false\")\n    case 0:\n        print(\"zero\")\nmatch 1:\n    case True:\n        print(\"true\")\n    case 1:\n        print(\"one\")"
        ),
        Ok(vec!["zero".to_string(), "one".to_string()])
    );
    assert_eq!(
        run_source(
            "match False:\n    case False:\n        print(\"false\")\nmatch True:\n    case True:\n        print(\"true\")"
        ),
        Ok(vec!["false".to_string(), "true".to_string()])
    );
}

#[test]
fn runs_match_complex_literal_patterns() {
    assert_eq!(
        run_source(
            "match -1.5j:\n    case -1.5j:\n        print(\"imag\")\nmatch 0.25 + 1.75j:\n    case 0.25 + 1.75j:\n        print(\"plus\")\nmatch -0.25 - 1.75j:\n    case -0.25 - 1.75j:\n        print(\"minus\")\nmatch {1 + 2j: \"value\"}:\n    case {1 + 2j: item}:\n        print(item)"
        ),
        Ok(vec![
            "imag".to_string(),
            "plus".to_string(),
            "minus".to_string(),
            "value".to_string(),
        ])
    );
}

#[test]
fn runs_match_or_pattern() {
    assert_eq!(
        run_source(
            "match 2:\n    case 0 | 1:\n        print(\"small\")\n    case 2 | 3:\n        print(\"middle\")\n    case _:\n        print(\"other\")"
        ),
        Ok(vec!["middle".to_string()])
    );
    assert_eq!(
        run_source("match \"b\":\n    case \"a\" | \"b\":\n        print(\"letter\")"),
        Ok(vec!["letter".to_string()])
    );
    assert_eq!(
        run_source("match [1, 2]:\n    case [0, x] | [1, x]:\n        print(x)"),
        Ok(vec!["2".to_string()])
    );
    assert_eq!(
        run_source("match [1, 2, 3]:\n    case [0, x, y] | [1, y, x]:\n        print(x, y)"),
        Ok(vec!["3 2".to_string()])
    );
}

#[test]
fn runs_match_sequence_patterns() {
    assert_eq!(
        run_source(
            "match [1, 2]:\n    case [1, value]:\n        print(value)\n    case _:\n        print(\"other\")"
        ),
        Ok(vec!["2".to_string()])
    );
    assert_eq!(
        run_source(
            "match (3, 4):\n    case (1, 2):\n        print(\"first\")\n    case (3, value):\n        print(value)"
        ),
        Ok(vec!["4".to_string()])
    );
    assert_eq!(
        run_source(
            "match \"ab\":\n    case [\"a\", \"b\"]:\n        print(\"sequence\")\n    case _:\n        print(\"string\")"
        ),
        Ok(vec!["string".to_string()])
    );
}

#[test]
fn runs_match_star_sequence_patterns() {
    assert_eq!(
        run_source("match (0, 1, 2):\n    case [*values]:\n        print(values)"),
        Ok(vec!["[0, 1, 2]".to_string()])
    );
    assert_eq!(
        run_source("match (0, 1, 2):\n    case [0, *values]:\n        print(values)"),
        Ok(vec!["[1, 2]".to_string()])
    );
    assert_eq!(
        run_source("match (0, 1, 2):\n    case [*values, 2]:\n        print(values)"),
        Ok(vec!["[0, 1]".to_string()])
    );
    assert_eq!(
        run_source("match (0, 1, 2):\n    case [0, *values, 2]:\n        print(values)"),
        Ok(vec!["[1]".to_string()])
    );
    assert_eq!(
        run_source("match range(4):\n    case [first, *_, last]:\n        print(first, last)"),
        Ok(vec!["0 3".to_string()])
    );
    assert_eq!(
        run_source(
            "match range(5):\n    case (first, second, *rest):\n        print(first, second, rest)"
        ),
        Ok(vec!["0 1 [2, 3, 4]".to_string()])
    );
}

#[test]
fn runs_match_as_patterns() {
    assert_eq!(
        run_source("match 0:\n    case 0 as z:\n        print(z)"),
        Ok(vec!["0".to_string()])
    );
    assert_eq!(
        run_source("match [1, 2]:\n    case [1, x] as pair:\n        print(x, pair)"),
        Ok(vec!["2 [1, 2]".to_string()])
    );
    assert_eq!(
        run_source(
            "match 1:\n    case 0 as z:\n        print(\"bad\")\n    case _:\n        print(\"miss\")"
        ),
        Ok(vec!["miss".to_string()])
    );
    assert_eq!(
        run_source(
            "z = \"outer\"\nmatch 1:\n    case 0 as z:\n        print(\"bad\")\n    case _:\n        print(z)"
        ),
        Ok(vec!["outer".to_string()])
    );
    assert_eq!(
        run_source("match 0:\n    case 0 as z if False:\n        print(\"bad\")\nprint(z)"),
        Ok(vec!["0".to_string()])
    );
    assert_eq!(
        run_source(
            "match [1, 2]:\n    case ([a, b] as pair) as outer:\n        print(a, b, pair, outer)"
        ),
        Ok(vec!["1 2 [1, 2] [1, 2]".to_string()])
    );
    assert_eq!(
        run_source("match 9:\n    case _ as captured:\n        print(captured)"),
        Ok(vec!["9".to_string()])
    );
}

#[test]
fn runs_match_value_patterns() {
    assert_eq!(
        run_source("class A:\n    B = 0\nmatch 0:\n    case A.B:\n        print(\"value\")"),
        Ok(vec!["value".to_string()])
    );
    assert_eq!(
        run_source(
            "class A:\n    class B:\n        C = 1\nmatch 1:\n    case A.B.C:\n        print(\"nested\")"
        ),
        Ok(vec!["nested".to_string()])
    );
    assert_eq!(
        run_source(
            "class A:\n    B = 0\n    C = 1\nmatch 1:\n    case A.B | A.C:\n        print(\"or\")"
        ),
        Ok(vec!["or".to_string()])
    );
    assert_eq!(
        run_source("class A:\n    B = 1\nmatch [1]:\n    case [A.B]:\n        print(\"sequence\")"),
        Ok(vec!["sequence".to_string()])
    );
    assert_eq!(
        run_source("class A:\n    B = 1\nmatch 1:\n    case A.B as z:\n        print(z)"),
        Ok(vec!["1".to_string()])
    );
    assert_eq!(
        run_source(
            "z = \"outer\"\nclass A:\n    B = 1\nmatch 2:\n    case A.B as z:\n        print(\"bad\")\n    case _:\n        print(z)"
        ),
        Ok(vec!["outer".to_string()])
    );
}

#[test]
fn runs_match_mapping_patterns() {
    assert_eq!(
        run_source("match {\"x\": 1}:\n    case {}:\n        print(\"mapping\")"),
        Ok(vec!["mapping".to_string()])
    );
    assert_eq!(
        run_source("match {\"x\": 1, \"y\": 2}:\n    case {\"x\": value}:\n        print(value)"),
        Ok(vec!["1".to_string()])
    );
    assert_eq!(
        run_source(
            "match {\"x\": [1, 2], \"y\": 0}:\n    case {\"x\": [1, value]}:\n        print(value)"
        ),
        Ok(vec!["2".to_string()])
    );
    assert_eq!(
        run_source(
            "match {\"bandwidth\": 100, \"latency\": 20, \"name\": \"link\"}:\n    case {\"bandwidth\": b, \"latency\": l, **rest}:\n        print(b, l, rest)"
        ),
        Ok(vec!["100 20 {'name': 'link'}".to_string()])
    );
    assert_eq!(
        run_source(
            "x = \"outer\"\nmatch {\"x\": 1}:\n    case {\"x\": x, \"missing\": y}:\n        print(\"bad\")\n    case _:\n        print(x)"
        ),
        Ok(vec!["outer".to_string()])
    );
    assert_eq!(
        run_source(
            "match []:\n    case {}:\n        print(\"mapping\")\n    case _:\n        print(\"other\")"
        ),
        Ok(vec!["other".to_string()])
    );
}

#[test]
fn raises_value_error_for_dynamic_duplicate_match_mapping_keys() {
    assert_eq!(
        run_source(
            "class Keys:\n    KEY = \"a\"\nw = y = z = None\ntry:\n    match {\"a\": 0, \"b\": 1}:\n        case {Keys.KEY: y, \"a\": z}:\n            w = 0\nexcept ValueError as error:\n    print(error)\nprint(w, y, z)"
        ),
        Ok(vec![
            "mapping pattern checks duplicate key ('a')".to_string(),
            "None None None".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "class Keys:\n    KEY = \"a\"\nmatch {\"a\": 0}:\n    case {Keys.KEY: y, \"a\": z}:\n        print(\"bad\")\n    case _:\n        print(\"fallback\")"
        ),
        Ok(vec!["fallback".to_string()])
    );
}

#[test]
fn runs_match_class_patterns() {
    let source = "class Point:\n    __match_args__ = (\"x\", \"y\")\n    def __init__(self, x, y):\n        self.x = x\n        self.y = y\ndef whereis(point):\n    match point:\n        case Point(0, 0):\n            return \"Origin\"\n        case Point(0, y):\n            return f\"Y={y}\"\n        case Point(x, 0):\n            return f\"X={x}\"\n        case Point():\n            return \"Somewhere else\"\n        case _:\n            return \"Not a point\"\nprint(whereis(Point(1, 0)))\nprint(whereis(Point(0, 0)))\nprint(whereis(Point(0, -1.0)))\nprint(whereis(Point(None, 1j)))\nprint(whereis(Point))\nprint(whereis(42))";

    assert_eq!(
        run_source(source),
        Ok(vec![
            "X=1".to_string(),
            "Origin".to_string(),
            "Y=-1.0".to_string(),
            "Somewhere else".to_string(),
            "Not a point".to_string(),
            "Not a point".to_string(),
        ])
    );

    assert_eq!(
        run_source(
            "class Point:\n    __match_args__ = (\"x\", \"y\")\n    def __init__(self, x, y):\n        self.x = x\n        self.y = y\ndef whereis(point):\n    match point:\n        case Point(1, y=var):\n            return var\n        case Point(x=1, y=var):\n            return var\nprint(whereis(Point(1, 0)))\nprint(whereis(Point(0, 0)))"
        ),
        Ok(vec!["0".to_string(), "None".to_string()])
    );

    assert_eq!(
        run_source(
            "class Parent:\n    __match_args__ = (\"a\", \"b\")\nclass Child(Parent):\n    pass\nc = Child()\nc.a = 0\nc.b = 1\nmatch c:\n    case Parent(x, y):\n        print(x, y)"
        ),
        Ok(vec!["0 1".to_string()])
    );

    assert_eq!(
        run_source(
            "match 41:\n    case int(x):\n        print(x)\nmatch \"mini\":\n    case str(text):\n        print(text)"
        ),
        Ok(vec!["41".to_string(), "mini".to_string()])
    );
    assert_eq!(
        run_source(
            "match range(3):\n    case range():\n        print(\"range\")\nmatch slice(1):\n    case slice():\n        print(\"slice\")"
        ),
        Ok(vec!["range".to_string(), "slice".to_string()])
    );
    assert_eq!(
        run_source(
            "y = None\ntry:\n    match range(10):\n        case range(10):\n            y = 0\nexcept TypeError as error:\n    print(error)\nprint(y)"
        ),
        Ok(vec![
            "range() accepts 0 positional sub-patterns".to_string(),
            "None".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "y = None\ntry:\n    match object():\n        case object(y):\n            pass\nexcept TypeError as error:\n    print(error)\nprint(y)"
        ),
        Ok(vec![
            "object() accepts 0 positional sub-patterns".to_string(),
            "None".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "class Class:\n    __match_args__ = ()\nx = Class()\nmatch x:\n    case Class(y):\n        print(y)"
        ),
        Err("runtime error: TypeError: Class() accepts 0 positional sub-patterns".to_string())
    );
    assert_eq!(
        run_source(
            "class Class:\n    __match_args__ = (\"a\", \"a\")\n    a = None\nx = Class()\nmatch x:\n    case Class(y, z):\n        print(y, z)"
        ),
        Err(
            "runtime error: TypeError: Class() got multiple sub-patterns for attribute 'a'"
                .to_string()
        )
    );
    assert_eq!(
        run_source(
            "class Class:\n    __match_args__ = (\"a\",)\n    a = None\nx = Class()\nmatch x:\n    case Class(y, a=z):\n        print(y, z)"
        ),
        Err(
            "runtime error: TypeError: Class() got multiple sub-patterns for attribute 'a'"
                .to_string()
        )
    );
    assert_eq!(
        run_source(
            "class Class:\n    __match_args__ = (None,)\nx = Class()\nmatch x:\n    case Class(y):\n        print(y)"
        ),
        Err(
            "runtime error: TypeError: Class.__match_args__ elements must be strings, got None"
                .to_string()
        )
    );
    assert_eq!(
        run_source(
            "class Class:\n    __match_args__ = None\nx = Class()\nmatch x:\n    case Class(y):\n        print(y)"
        ),
        Err("runtime error: TypeError: Class.__match_args__ must be a tuple".to_string())
    );
    assert_eq!(
        run_source(
            "class Class:\n    __match_args__ = \"XYZ\"\nx = Class()\ny = z = None\ntry:\n    match x:\n        case Class(y):\n            z = 0\nexcept TypeError as error:\n    print(error)\nprint(y, z)"
        ),
        Ok(vec![
            "Class.__match_args__ must be a tuple".to_string(),
            "None None".to_string(),
        ])
    );
    assert_eq!(
        run_source(
            "class Class:\n    __match_args__ = [\"spam\", \"eggs\"]\n    spam = 0\n    eggs = 1\nx = Class()\nw = y = z = None\ntry:\n    match x:\n        case Class(y, z):\n            w = 0\nexcept TypeError as error:\n    print(error)\nprint(w, y, z)"
        ),
        Ok(vec![
            "Class.__match_args__ must be a tuple".to_string(),
            "None None None".to_string(),
        ])
    );
}

#[test]
fn runs_match_open_sequence_patterns() {
    assert_eq!(
        run_source("match 1,:\n    case value,:\n        print(value)"),
        Ok(vec!["1".to_string()])
    );
    assert_eq!(
        run_source("match 1, 2:\n    case left, right:\n        print(left, right)"),
        Ok(vec!["1 2".to_string()])
    );
    assert_eq!(
        run_source("match range(4):\n    case first, *rest:\n        print(first, rest)"),
        Ok(vec!["0 [1, 2, 3]".to_string()])
    );
    assert_eq!(
        run_source("match [1]:\n    case value as captured,:\n        print(value, captured)"),
        Ok(vec!["1 1".to_string()])
    );
}

#[test]
fn match_sequence_patterns_do_not_bind_on_failure() {
    assert_eq!(
        run_source(
            "x = \"outer\"\nmatch [1, 2]:\n    case [x, 3]:\n        print(\"bad\")\n    case _:\n        print(x)"
        ),
        Ok(vec!["outer".to_string()])
    );
    assert_eq!(
        run_source(
            "x = \"outer\"\nmatch [0, 2]:\n    case [0, *x, 3]:\n        print(\"bad\")\n    case _:\n        print(x)"
        ),
        Ok(vec!["outer".to_string()])
    );
}

#[test]
fn runs_match_capture_pattern() {
    assert_eq!(
        run_source("match \"value\":\n    case captured:\n        print(captured)"),
        Ok(vec!["value".to_string()])
    );
}

#[test]
fn runs_match_guards() {
    assert_eq!(
        run_source(
            "match 2:\n    case value if value == 1:\n        print(\"one\")\n    case value if value == 2:\n        print(\"two\", value)\n    case _:\n        print(\"other\")"
        ),
        Ok(vec!["two 2".to_string()])
    );
    assert_eq!(
        run_source(
            "match [\"go\", \"n\"]:\n    case [command, direction] if direction in \"nesw\" and (seen := command):\n        print(seen, direction)"
        ),
        Ok(vec!["go n".to_string()])
    );
}

#[test]
fn continues_after_false_wildcard_guard() {
    assert_eq!(
        run_source(
            "match 1:\n    case _ if False:\n        print(\"bad\")\n    case 1:\n        print(\"one\")"
        ),
        Ok(vec!["one".to_string()])
    );
}

#[test]
fn keeps_match_and_case_as_soft_keywords() {
    assert_eq!(
        run_source(
            "match = 1\ncase = 2\nprint(match, case)\ndef match(value):\n    return value\nprint(match(3))"
        ),
        Ok(vec!["1 2".to_string(), "3".to_string()])
    );
}

#[test]
fn reports_unsupported_match_patterns() {
    assert_eq!(
        run_source("match 1:\n    case object.attr:\n        print(\"attr\")"),
        Err("runtime error: AttributeError: <class 'object'> has no attribute 'attr'".to_string())
    );
    assert_eq!(
        run_source(
            "match 1:\n    case _:\n        print(\"default\")\n    case 1:\n        print(\"one\")"
        ),
        Err("parse error: irrefutable match case must be last".to_string())
    );
    assert_eq!(
        run_source(
            "match 1:\n    case value:\n        print(value)\n    case 1:\n        print(\"one\")"
        ),
        Err("parse error: irrefutable match case must be last".to_string())
    );
    assert_eq!(
        run_source("match 1:\n    case 0 | value:\n        print(value)"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match [1]:\n    case [x] | [1]:\n        print(x)"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match [1, 2]:\n    case [*a, *b]:\n        print(a, b)"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match [1, 2]:\n    case (*a):\n        print(a)"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match 1:\n    case 1 as _:\n        print(\"bad\")"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match [1, 2]:\n    case x, x:\n        print(x)"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match 1:\n    case x as x:\n        print(x)"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("class A:\n    B = 1\nmatch 1:\n    case A.B():\n        print(\"bad\")"),
        Err("runtime error: TypeError: called match pattern must be a class".to_string())
    );
    assert_eq!(
        run_source(
            "w = None\ntry:\n    match 1:\n        case max(0, 1):\n            w = 0\nexcept TypeError as error:\n    print(error)\nprint(w)"
        ),
        Ok(vec![
            "called match pattern must be a class".to_string(),
            "None".to_string(),
        ])
    );
    assert_eq!(
        run_source("match {\"x\": 1}:\n    case {\"x\": a, \"y\": a}:\n        print(a)"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match {\"x\": 1}:\n    case {\"x\": _, \"x\": _}:\n        print(\"bad\")"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match {0: \"zero\"}:\n    case {0: _, False: _}:\n        print(\"bad\")"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match {0: \"zero\"}:\n    case {0: _, 0.0: _}:\n        print(\"bad\")"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match {0: \"zero\"}:\n    case {0: _, -0: _}:\n        print(\"bad\")"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source("match {0: \"zero\"}:\n    case {0: _, 0j: _}:\n        print(\"bad\")"),
        Err("parse error: unsupported match pattern".to_string())
    );
    assert_eq!(
        run_source(
            "match {\"x\": 1}:\n    case {**rest, \"x\": value}:\n        print(rest, value)"
        ),
        Err("parse error: unsupported match pattern".to_string())
    );
}

#[test]
fn skips_comments() {
    assert_eq!(
        run_source("# leading\nprint(1) # inline\n# trailing"),
        Ok(vec!["1".to_string()])
    );
}

#[test]
fn skips_type_comments_and_type_ignores() {
    assert_eq!(
        run_source(concat!(
            "x = 1  # type: int\n",
            "# type: ignore[assignment]\n",
            "if True:  # type: ignore\n",
            "    # type: ignore whatever\n",
            "    print(x)"
        )),
        Ok(vec!["1".to_string()])
    );
}

#[test]
fn compares_numbers() {
    assert_eq!(
        run_source("print(1 + 2 == 3)\nprint(1 == 2)"),
        Ok(vec!["True".to_string(), "False".to_string()])
    );
}

#[test]
fn compares_numbers_with_ordering_operators() {
    assert_eq!(
        run_source("print(1 != 2, 1 < 2, 2 > 1, 1 <= 1, 2 >= 2)"),
        Ok(vec!["True True True True True".to_string()])
    );
}

#[test]
fn compares_sequences_with_rich_ordering_items() {
    assert_eq!(
        run_source(concat!(
            "class Box:\n",
            "    def __init__(self, value):\n",
            "        self.value = value\n",
            "    def __eq__(self, other):\n",
            "        return self.value == other.value\n",
            "    def __lt__(self, other):\n",
            "        return self.value < other.value\n",
            "    def __le__(self, other):\n",
            "        return self.value <= other.value\n",
            "    def __gt__(self, other):\n",
            "        return self.value > other.value\n",
            "    def __ge__(self, other):\n",
            "        return self.value >= other.value\n",
            "print([Box(1)] < [Box(2)], [Box(1)] <= [Box(1)])\n",
            "print([Box(2)] > [Box(1)], [Box(1)] >= [Box(1)])\n",
            "print([Box(1), Box(3)] > [Box(1), Box(2)])\n",
            "print((Box(1), Box(2)) < (Box(1), Box(3)))",
        )),
        Ok(vec![
            "True True".to_string(),
            "True True".to_string(),
            "True".to_string(),
            "True".to_string(),
        ])
    );
}

#[test]
fn compares_sequence_like_values_with_rich_ordering_items() {
    assert_eq!(
        run_source(concat!(
            "from collections import UserList, namedtuple\n",
            "class Box:\n",
            "    def __init__(self, value):\n",
            "        self.value = value\n",
            "    def __eq__(self, other):\n",
            "        return self.value == other.value\n",
            "    def __lt__(self, other):\n",
            "        return self.value < other.value\n",
            "    def __le__(self, other):\n",
            "        return self.value <= other.value\n",
            "    def __gt__(self, other):\n",
            "        return self.value > other.value\n",
            "    def __ge__(self, other):\n",
            "        return self.value >= other.value\n",
            "Pair = namedtuple('Pair', 'left right')\n",
            "print(UserList([Box(1)]) < [Box(2)], [Box(2)] > UserList([Box(1)]))\n",
            "print(UserList([Box(1), Box(3)]) > UserList([Box(1), Box(2)]))\n",
            "print(Pair(Box(1), Box(2)) < (Box(1), Box(3)))\n",
            "print((Box(1), Box(3)) > Pair(Box(1), Box(2)))",
        )),
        Ok(vec![
            "True True".to_string(),
            "True".to_string(),
            "True".to_string(),
            "True".to_string(),
        ])
    );
}

#[test]
fn compares_strings() {
    assert_eq!(
        run_source("print(\"mini\" + \"python\" == \"minipython\")"),
        Ok(vec!["True".to_string()])
    );
}

#[test]
fn compares_booleans() {
    assert_eq!(
        run_source("print(True == True, True == False)"),
        Ok(vec!["True False".to_string()])
    );
}

#[test]
fn runs_membership_comparisons() {
    assert_eq!(
        run_source(
            "print(1 in [1, 2], 3 not in [1, 2])\nprint(\"y\" in \"python\", \"z\" not in \"python\")\nprint(2 in range(1, 4), 4 not in range(1, 4))\nprint(\"a\" in {\"a\": 1}, \"x\" not in {\"a\": 1})"
        ),
        Ok(vec![
            "True True".to_string(),
            "True True".to_string(),
            "True True".to_string(),
            "True True".to_string(),
        ])
    );
}

#[test]
fn runs_identity_comparisons() {
    assert_eq!(
        run_source(
            "x = None\nprint(x is None, x is not None)\nprint(None is None, True is True, False is not True)\nvalues = (1, 2)\ncopy = tuple(values)\nother = (1, 2)\nprint(values is copy, values is other, values == other)\nif x is None:\n    print(\"none\")"
        ),
        Ok(vec![
            "True False".to_string(),
            "True True True".to_string(),
            "True False True".to_string(),
            "none".to_string(),
        ])
    );
}

#[test]
fn runs_chained_comparisons() {
    assert_eq!(
        run_source("print(1 < 2 < 3, 1 < 2 > 3, 3 > 2 >= 2)\nprint(2 < 1 < unknown)"),
        Ok(vec!["True False True".to_string(), "False".to_string()])
    );
}

#[test]
fn runs_conditional_expressions() {
    assert_eq!(
        run_source(
            "print(1 if True else 2)\nprint(1 if False else 2)\nprint(\"yes\" if 1 < 2 else \"no\")"
        ),
        Ok(vec!["1".to_string(), "2".to_string(), "yes".to_string()])
    );
}

#[test]
fn short_circuits_conditional_expressions() {
    assert_eq!(
        run_source("print(1 if True else unknown)\nprint(2 if False else 3)"),
        Ok(vec!["1".to_string(), "3".to_string()])
    );
}

#[test]
fn runs_nested_conditional_expressions() {
    assert_eq!(
        run_source("print(1 if False else 2 if True else 3)"),
        Ok(vec!["2".to_string()])
    );
}

#[test]
fn runs_boolean_operators() {
    assert_eq!(
        run_source("print(not True, not False)\nprint(True and False, True or False)"),
        Ok(vec!["False True".to_string(), "False True".to_string(),])
    );
}

#[test]
fn runs_boolean_operators_in_if_condition() {
    assert_eq!(
        run_source("if True and not False:\n    print(\"yes\")"),
        Ok(vec!["yes".to_string()])
    );
}

#[test]
fn short_circuits_boolean_operators() {
    assert_eq!(
        run_source(
            "print(False and unknown)\nprint(True or unknown)\nprint(0 and unknown)\nprint(\"x\" or unknown)"
        ),
        Ok(vec![
            "False".to_string(),
            "True".to_string(),
            "0".to_string(),
            "x".to_string(),
        ])
    );
}

#[test]
fn returns_logical_operands_with_truthiness() {
    assert_eq!(
        run_source(
            "print(True and 1, False or \"fallback\")\nprint(0 or \"fallback\", \"x\" and \"y\", \"x\" or 2)"
        ),
        Ok(vec!["1 fallback".to_string(), "fallback y x".to_string(),])
    );
}

#[test]
fn runs_not_with_truthy_values() {
    assert_eq!(
        run_source("print(not 1, not 0, not \"\", not \"x\")"),
        Ok(vec!["False True True False".to_string()])
    );
}

#[test]
fn compares_different_value_types_as_false() {
    assert_eq!(
        run_source("print(1 == \"1\")"),
        Ok(vec!["False".to_string()])
    );
}

#[test]
fn prints_nested_grouped_expression() {
    assert_eq!(run_source("print(((1)))"), Ok(vec!["1".to_string()]));
}

#[test]
fn prints_multiple_arguments() {
    assert_eq!(run_source("print(1 + 2, 3)"), Ok(vec!["3 3".to_string()]));
}

#[test]
fn runs_multiple_statements() {
    assert_eq!(
        run_source("print(1)\nprint(2)"),
        Ok(vec!["1".to_string(), "2".to_string()])
    );
}

#[test]
fn runs_semicolon_separated_simple_statements() {
    assert_eq!(
        run_source("x = 1; x += 2; print(x); print(x + 1)"),
        Ok(vec!["3".to_string(), "4".to_string()])
    );
}

#[test]
fn skips_blank_lines_between_statements() {
    assert_eq!(
        run_source("\nprint(1)\n\nprint(2)\n"),
        Ok(vec!["1".to_string(), "2".to_string()])
    );
}

#[test]
fn assigns_and_reads_variable() {
    assert_eq!(run_source("x = 1 + 2\nprint(x)"), Ok(vec!["3".to_string()]));
}

#[test]
fn runs_chained_assignment() {
    assert_eq!(
        run_source("def value():\n    print(\"make\")\n    return 3\na = b = value()\nprint(a, b)"),
        Ok(vec!["make".to_string(), "3 3".to_string()])
    );
}

#[test]
fn assigns_and_reads_boolean() {
    assert_eq!(
        run_source("ok = 1 + 2 == 3\nprint(ok)"),
        Ok(vec!["True".to_string()])
    );
}

#[test]
fn reassigns_variable() {
    assert_eq!(
        run_source("x = 1\nx = x + 2\nprint(x)"),
        Ok(vec!["3".to_string()])
    );
}

#[test]
fn runs_named_expressions_in_allowed_expression_contexts() {
    assert_eq!(
        run_source(
            "(a := 10)\nprint(a)\nprint((total := 1 + 2), total)\n(x := 1, 2)\nprint(x)\n(z := (y := (inner := 0)))\nprint(inner, y, z)"
        ),
        Ok(vec![
            "10".to_string(),
            "3 3".to_string(),
            "1".to_string(),
            "0 0 0".to_string(),
        ])
    );
}

#[test]
fn runs_named_expressions_in_conditions_calls_and_subscripts() {
    assert_eq!(
        run_source(
            "if spam := \"eggs\":\n    print(spam)\nif True and (ok := True):\n    print(ok)\nwhile flag := False:\n    print(\"skip\")\nprint(flag)\ndef pair(a, b):\n    print(a, b)\npair(c := 2, b=1)\nprint(c)\nitems = [10]\nprint(items[index := 0], index)"
        ),
        Ok(vec![
            "eggs".to_string(),
            "True".to_string(),
            "False".to_string(),
            "2 1".to_string(),
            "2".to_string(),
            "10 0".to_string(),
        ])
    );
}

#[test]
fn rejects_named_expressions_in_disallowed_contexts() {
    assert_eq!(
        run_source("x := 1"),
        Err(
            "parse error: expected statement separator or end of input, found ColonEqual"
                .to_string()
        )
    );
    assert_eq!(
        run_source("x = y := 1"),
        Err(
            "parse error: expected statement separator or end of input, found ColonEqual"
                .to_string()
        )
    );
    assert_eq!(
        run_source("def spam(a):\n    pass\nspam(a=b := \"c\")"),
        Err("parse error: expected ')', found ColonEqual".to_string())
    );
    assert_eq!(
        run_source("def spam(a):\n    pass\nspam(a=1, b := 2)"),
        Err("parse error: positional argument follows keyword argument".to_string())
    );
}

#[test]
fn runs_augmented_assignments() {
    assert_eq!(
        run_source(
            "x = 1\nx += 2\nprint(x)\nx -= 1\nprint(x)\nx *= 3\nprint(x)\nx //= 2\nprint(x)\nx %= 4\nprint(x)\nx **= 3\nprint(x)\ny = 5\ny /= 2\nprint(y)"
        ),
        Ok(vec![
            "3".to_string(),
            "2".to_string(),
            "6".to_string(),
            "3".to_string(),
            "3".to_string(),
            "27".to_string(),
            "2.5".to_string(),
        ])
    );
}

#[test]
fn runs_float_literals() {
    assert_eq!(
        run_source(
            "print(1.5, .25, 1., 1e3)\nprint(1.5 + .5)\nprint(1 == 1.0)\nprint(.5 < 1)\nprint(1_000, 1_000.5, 1.5_0, 1e1_0)"
        ),
        Ok(vec![
            "1.5 0.25 1.0 1000.0".to_string(),
            "2.0".to_string(),
            "True".to_string(),
            "True".to_string(),
            "1000 1000.5 1.5 10000000000.0".to_string(),
        ])
    );
}

#[test]
fn runs_imaginary_literals() {
    assert_eq!(
        run_source(
            "print(1j, .5j, 1.j, 1e3j)\nprint(1 + 2j, 1 - 2j, 2j * 3)\nprint(1 + 0j == 1, 0j or 'fallback')"
        ),
        Ok(vec![
            "1j 0.5j 1j 1000j".to_string(),
            "(1+2j) (1-2j) 6j".to_string(),
            "True fallback".to_string(),
        ])
    );
}

#[test]
fn runs_augmented_bitwise_assignments() {
    assert_eq!(
        run_source(
            "x = 1\nx |= 2\nprint(x)\nx ^= 1\nprint(x)\nx &= 6\nprint(x)\nx <<= 2\nprint(x)\nx >>= 1\nprint(x)"
        ),
        Ok(vec![
            "3".to_string(),
            "2".to_string(),
            "2".to_string(),
            "8".to_string(),
            "4".to_string(),
        ])
    );
}

#[test]
fn prints_string_literal() {
    assert_eq!(
        run_source("print(\"hello\")"),
        Ok(vec!["hello".to_string()])
    );
}

#[test]
fn runs_python_string_literal_forms() {
    assert_eq!(
        run_source(
            "print('single')\nprint(\"mini\" 'python')\nprint('line\\nbreak')\nprint(r\"\\n\" == \"\\\\n\")\nprint(\"\\x41\", \"\\u0042\", \"\\U00000043\")\nprint('''triple\nquote''')"
        ),
        Ok(vec![
            "single".to_string(),
            "minipython".to_string(),
            "line\nbreak".to_string(),
            "True".to_string(),
            "A B C".to_string(),
            "triple\nquote".to_string(),
        ])
    );
}

#[test]
fn runs_python_bytes_literal_forms() {
    assert_eq!(
        run_source(
            "print(b'abc')\nprint(len(b'abc'), b'abc'[0], b'abc'[1:])\nprint(list(b'ab'))\nprint(b'a' + b'b', b'ab' * 2, 2 * b'x')\nprint(97 in b'abc', b'b' in b'abc', b'z' in b'abc')\nif b'':\n    print('bad')\nelse:\n    print('empty')\nif b'x':\n    print('nonempty')"
        ),
        Ok(vec![
            "b'abc'".to_string(),
            "3 97 b'bc'".to_string(),
            "[97, 98]".to_string(),
            "b'ab' b'abab' b'xx'".to_string(),
            "True True False".to_string(),
            "empty".to_string(),
            "nonempty".to_string(),
        ])
    );
}

#[test]
fn runs_f_strings() {
    assert_eq!(
        run_source(
            "x = \"mini\"\ny = 3\nprint(f\"hello {x}\")\nprint(f\"{y + 4} {{ok}} {3 != 4}\")\nprint(\"pre\" f\"{x}\" \"post\")\nprint(f\"{'a'!r}\", f\"{'a'!s}\", f\"{'a'!a}\")\nprint(f\"{x:}\")"
        ),
        Ok(vec![
            "hello mini".to_string(),
            "7 {ok} True".to_string(),
            "preminipost".to_string(),
            "'a' a 'a'".to_string(),
            "mini".to_string(),
        ])
    );
}

#[test]
fn runs_f_string_expressions() {
    assert_eq!(
        run_source(
            "name = \"mini\"\nprint(f\"hello {name} {1 + 2}\")\nprint(f\"{{{name}}}\")\nprint(f\"{name!r} {name!s  } {name!a:}\")\nprint(f\"{3!=4!s}\")\nprint(f\"{name:}\" \"python\")\nprint(f\"{name:6}{3:4}\")\nprint(f\"{3.14159:.2f}\")\nwidth = 5\nprint(f\"{7:{width}}\")\nprint(f\"{name=} {name =} {name=!s}\")\nvalue = 9\nprint(f\"{3*value+15=}\")"
        ),
        Ok(vec![
            "hello mini 3".to_string(),
            "{mini}".to_string(),
            "'mini' mini 'mini'".to_string(),
            "True".to_string(),
            "minipython".to_string(),
            "mini     3".to_string(),
            "3.14".to_string(),
            "    7".to_string(),
            "name='mini' name ='mini' name=mini".to_string(),
            "3*value+15=42".to_string(),
        ])
    );
}

#[test]
fn runs_unicode_identifiers() {
    assert_eq!(
        run_source(
            "tenπ = 31.4\n变量 = 8\ndef 加一(x):\n    return x + 1\nclass 盒子:\n    pass\nK = 7\nprint(K)\nK = 8\nprint(K)\nｘ = 3\nprint(x)\nµ = 5\nprint(μ)\nprint(tenπ, 变量, 加一(4), 盒子.__name__)\nprint(f\"{tenπ=:.2f}\")\nprint(f\"{K=}\", f\"{K=}\")"
        ),
        Ok(vec![
            "7".to_string(),
            "8".to_string(),
            "3".to_string(),
            "5".to_string(),
            "31.4 8 5 盒子".to_string(),
            "tenπ=31.40".to_string(),
            "K=8 K=8".to_string(),
        ])
    );
}

#[test]
fn runs_t_strings() {
    assert_eq!(
        run_source(
            "name = \"Python\"\n\
             t = t\"Hello, {name}\"\n\
             print(t)\n\
             print(t.strings)\n\
             print(t.interpolations[0].value, t.interpolations[0].expression, t.interpolations[0].conversion, t.interpolations[0].format_spec == \"\")\n\
             value = 42\n\
             debug = t\"Value: {value=:.2f}\"\n\
             print(debug.strings, debug.interpolations[0].expression, debug.interpolations[0].conversion, debug.interpolations[0].format_spec)\n\
             print(t\"Hello, \" t\"{name}\")\n\
             print(t\"Hello, \" + t\"world\")"
        ),
        Ok(vec![
            "Template(strings=('Hello, ', ''), interpolations=(Interpolation('Python', 'name', None, ''),))".to_string(),
            "('Hello, ', '')".to_string(),
            "Python name None True".to_string(),
            "('Value: value=', '') value None .2f".to_string(),
            "Template(strings=('Hello, ', ''), interpolations=(Interpolation('Python', 'name', None, ''),))".to_string(),
            "Template(strings=('Hello, world',), interpolations=())".to_string(),
        ])
    );
}

#[test]
fn reports_unsupported_f_string_forms() {
    assert_eq!(
        run_source("print(f\"{1:q}\")"),
        Err(
            "runtime error: ValueError: Unknown format code 'q' for object of type 'int'"
                .to_string()
        )
    );
    assert_eq!(
        run_source("print(f\"{}\")"),
        Err("lex error: f-string: valid expression required before '}'".to_string())
    );
}

#[test]
fn prints_list_literals() {
    assert_eq!(
        run_source("print([])\nprint([1, 2, 3])\nprint([\"a\", True, 1 + 2])"),
        Ok(vec![
            "[]".to_string(),
            "[1, 2, 3]".to_string(),
            "['a', True, 3]".to_string(),
        ])
    );
}

#[test]
fn runs_list_comprehensions() {
    assert_eq!(
        run_source(
            "nums = [1, 2, 3, 4, 5]\nprint([3 * x for x in nums])\nprint([x for x in nums if x > 2])\nprint([(i, j) for i in range(3) for j in range(i)])\nprint([x for x, in [(4,), (5,), (6,)]])"
        ),
        Ok(vec![
            "[3, 6, 9, 12, 15]".to_string(),
            "[3, 4, 5]".to_string(),
            "[(1, 0), (2, 0), (2, 1)]".to_string(),
            "[4, 5, 6]".to_string(),
        ])
    );
}

#[test]
fn runs_dict_comprehensions() {
    assert_eq!(
        run_source(
            "nums = [1, 2, 3]\nprint({i: i + 1 for i in nums})\nprint({k: v for k in range(4) for v in range(4) if k == v})\nprint({j + k: j * k for i in range(4) for j, k in [(i + 1, i + 2)]})"
        ),
        Ok(vec![
            "{1: 2, 2: 3, 3: 4}".to_string(),
            "{0: 0, 1: 1, 2: 2, 3: 3}".to_string(),
            "{3: 2, 5: 6, 7: 12, 9: 20}".to_string(),
        ])
    );
}

#[test]
fn runs_set_literals_and_comprehensions() {
    assert_eq!(
        run_source(
            "print({1, 2, 1})\nprint(2 in {1, 2, 3})\nprint({i * i for i in range(5) if i % 2 == 1})\nprint({(i, j) for i in range(3) for j in range(i)})\nprint({j * k for i in range(4) for j, k in [(i + 1, i + 2)]})"
        ),
        Ok(vec![
            "{1, 2}".to_string(),
            "True".to_string(),
            "{1, 9}".to_string(),
            "{(1, 0), (2, 0), (2, 1)}".to_string(),
            "{2, 6, 12, 20}".to_string(),
        ])
    );
}

#[test]
fn runs_list_mutation_methods() {
    assert_eq!(
        run_source(
            "items = [1]\nalias = items\nprint(items.append(2), items)\nitems.extend((3, 4))\nprint(items.pop(), items.pop(0), items, items is alias)\ncopy = items.copy()\nitems.clear()\nprint(items, copy, copy is items)"
        ),
        Ok(vec![
            "None [1, 2]".to_string(),
            "4 1 [2, 3] True".to_string(),
            "[] [2, 3] False".to_string(),
        ])
    );
}

#[test]
fn runs_dict_mutation_methods() {
    assert_eq!(
        run_source(
            "d = {1: 2}\nalias = d\nprint(d.get(1), d.get(9), d.get(9, 10))\nd.update({3: 4}, five=5)\nprint(d[3], d[\"five\"], d is alias)\nprint(d.keys())\nprint(d.values())\nprint(d.items())\ncopy = d.copy()\nprint(copy == d, copy is d)\nprint(d.pop(1), d.pop(9, \"missing\"))\nd.clear()\nprint(len(d), len(copy))\nprint(dict.fromkeys([1, 2], 7))"
        ),
        Ok(vec![
            "2 None 10".to_string(),
            "4 5 True".to_string(),
            "dict_keys([1, 3, 'five'])".to_string(),
            "dict_values([2, 4, 5])".to_string(),
            "dict_items([(1, 2), (3, 4), ('five', 5)])".to_string(),
            "True False".to_string(),
            "2 missing".to_string(),
            "0 3".to_string(),
            "{1: 7, 2: 7}".to_string(),
        ])
    );
}

#[test]
fn runs_dict_dunder_mapping_methods() {
    assert_eq!(
        run_source(
            "d = {1: 'one', 'two': 2}\n\
             print(d.__getitem__(1), d.__getitem__('two'), d.__contains__(1), d.__contains__(3), d.__len__())\n\
             print(d.__setitem__(3, 'three'), d)\n\
             print(d.__setitem__(1, 'uno'), d.__getitem__(1), d.__len__())\n\
             print(d.__delitem__('two'), d, d.__len__())"
        ),
        Ok(vec![
            "one 2 True False 2".to_string(),
            "None {1: 'one', 'two': 2, 3: 'three'}".to_string(),
            "None uno 3".to_string(),
            "None {1: 'uno', 3: 'three'} 2".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_dict_dunder_mapping_method_calls() {
    assert!(run_source("d = {1: 2}\nd.__getitem__(3)").is_err());
    assert!(run_source("d = {1: 2}\nd.__getitem__()").is_err());
    assert!(run_source("d = {1: 2}\nd.__getitem__(1, 2)").is_err());
    assert!(run_source("d = {1: 2}\nd.__setitem__()").is_err());
    assert!(run_source("d = {1: 2}\nd.__setitem__(1)").is_err());
    assert!(run_source("d = {1: 2}\nd.__setitem__(1, 2, 3)").is_err());
    assert!(run_source("d = {1: 2}\nd.__delitem__()").is_err());
    assert!(run_source("d = {1: 2}\nd.__delitem__(3)").is_err());
    assert!(run_source("d = {1: 2}\nd.__delitem__(1, 2)").is_err());
    assert!(run_source("d = {1: 2}\nd.__contains__()").is_err());
    assert!(run_source("d = {1: 2}\nd.__contains__(1, 2)").is_err());
    assert!(run_source("d = {1: 2}\nd.__contains__([])").is_err());
    assert!(run_source("d = {1: 2}\nd.__len__(1)").is_err());
}

#[test]
fn runs_dict_setdefault_popitem_and_union() {
    assert_eq!(
        run_source(
            "d = {}\nprint(d.setdefault(\"key0\"))\nd.setdefault(\"key\", []).append(3)\nd.setdefault(\"key\", []).append(4)\nprint(d[\"key\"])\nprint(d.setdefault(\"key\", [9]))\nleft = {1: 1, 2: 2}\nprint(left | {2: 20, 3: 3})\nalias = left\nleft |= {4: 4, 2: 22}\nprint(left, left is alias)\nprint(left.popitem(), left)"
        ),
        Ok(vec![
            "None".to_string(),
            "[3, 4]".to_string(),
            "[3, 4]".to_string(),
            "{1: 1, 2: 20, 3: 3}".to_string(),
            "{1: 1, 2: 22, 4: 4} True".to_string(),
            "(4, 4) {1: 1, 2: 22}".to_string(),
        ])
    );
}

#[test]
fn runs_dynamic_dict_views_and_set_operations() {
    assert_eq!(
        run_source(
            "d = {1: 1}\nkeys = d.keys()\nvalues = d.values()\nitems = d.items()\nprint(len(keys), list(keys), list(values), list(items))\nd[2] = 2\nprint(len(keys), list(keys), list(values), list(items))\nprint(2 in keys, 2 in values, (2, 2) in items)\nprint(keys == {1, 2}, items == {(1, 1), (2, 2)})\nprint(keys | {3})\nprint({0} | keys)\nprint(keys & {2, 4})\nprint(keys - {1})\nprint(keys ^ {2, 3})\nprint(items | {(3, 3)})\nprint({1: 1}.keys() < keys, {1: 1}.keys() <= keys, keys > {1: 1}.keys(), keys >= {1: 1}.keys())\nprint({1: 1}.items() < items, {1: 1}.items() <= {(1, 1)}, {(1, 1)} >= {1: 1}.items())"
        ),
        Ok(vec![
            "1 [1] [1] [(1, 1)]".to_string(),
            "2 [1, 2] [1, 2] [(1, 1), (2, 2)]".to_string(),
            "True True True".to_string(),
            "True True".to_string(),
            "{1, 2, 3}".to_string(),
            "{0, 1, 2}".to_string(),
            "{2}".to_string(),
            "{2}".to_string(),
            "{1, 3}".to_string(),
            "{(1, 1), (2, 2), (3, 3)}".to_string(),
            "True True True True".to_string(),
            "True True True".to_string(),
        ])
    );
}

#[test]
fn runs_reversed_builtin_for_sequences_and_dict_views() {
    assert_eq!(
        run_source(
            "print(list(reversed([1, 2, 3])))\n\
             print(list(reversed((1, 2, 3))))\n\
             print(list(reversed(\"abc\")))\n\
             print(list(reversed(b\"ab\")))\n\
             print(list(reversed(range(1, 5))))\n\
             d = {1: 2, 3: 4}\n\
             print(list(reversed(d)))\n\
             print(list(reversed(d.keys())))\n\
             print(list(reversed(d.values())))\n\
             print(list(reversed(d.items())))\n\
             print(list(reversed({})), list(reversed({}.keys())), list(reversed({}.values())), list(reversed({}.items())))"
        ),
        Ok(vec![
            "[3, 2, 1]".to_string(),
            "[3, 2, 1]".to_string(),
            "['c', 'b', 'a']".to_string(),
            "[98, 97]".to_string(),
            "[4, 3, 2, 1]".to_string(),
            "[3, 1]".to_string(),
            "[3, 1]".to_string(),
            "[4, 2]".to_string(),
            "[(3, 4), (1, 2)]".to_string(),
            "[] [] [] []".to_string(),
        ])
    );
}

#[test]
fn runs_reversed_protocol_for_custom_objects() {
    assert_eq!(
        run_source(
            r#"class ReverseCustom:
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
print(reversed(BadReverse()))"#
        ),
        Ok(vec![
            "[3, 2, 1]".to_string(),
            "len".to_string(),
            "made".to_string(),
            "get 2".to_string(),
            "get 1".to_string(),
            "get 0".to_string(),
            "30 20 10 done".to_string(),
            "42".to_string(),
        ])
    );
}

#[test]
fn allows_same_size_key_changes_during_reverse_dict_iteration() {
    assert_eq!(
        run_source(
            "d = {0: 0, 1: 1, 2: 2}\nfor key in reversed(d):\n    print(key)\n    if key == 2:\n        del d[0]\n        d[0] = 0\nprint(\"done\")"
        ),
        Ok(vec!["2".to_string(), "1".to_string(), "done".to_string()])
    );
    assert_eq!(
        run_source(
            "d = {0: 0, 1: 1, 2: 2}\nfor item in reversed(d.items()):\n    print(item)\n    if item == (2, 2):\n        del d[1]\n        d[1] = 1\nprint(\"done\")"
        ),
        Ok(vec![
            "(2, 2)".to_string(),
            "(0, 0)".to_string(),
            "done".to_string(),
        ])
    );
}

#[test]
fn reverse_dict_values_iterator_reads_updated_values() {
    assert_eq!(
        run_source(
            "d = {0: 0, 1: 1, 2: 2}\nvalues = reversed(d.values())\nprint(next(values))\nd[0] = 9\nprint(next(values))\nprint(next(values))"
        ),
        Ok(vec!["2".to_string(), "1".to_string(), "9".to_string()])
    );
}

#[test]
fn rejects_dict_mutation_during_iteration() {
    let cases = [
        "d = {1: 1}\nfor key in d:\n    d[key + 1] = 1",
        "d = {0: 0}\nfor key in d:\n    del d[0]\n    d[0] = 0",
        "d = {0: 0}\nfor value in d.values():\n    del d[0]\n    d[0] = 0",
        "d = {0: 0}\nfor item in d.items():\n    del d[0]\n    d[0] = 0",
        "d = {1: 1}\nfor key in reversed(d):\n    d[key + 1] = 1",
        "d = {1: 1}\nfor value in reversed(d.values()):\n    d[2] = 2",
        "d = {1: 1}\nfor item in reversed(d.items()):\n    d[2] = 2",
    ];

    for source in cases {
        assert!(
            matches!(
                run_source(source),
                Err(message)
                    if message.contains("dictionary")
                        && message.contains("changed")
                        && message.contains("during iteration")
            ),
            "expected dictionary mutation during iteration to fail for:\n{source}"
        );
    }
}

#[test]
fn allows_dict_value_update_during_iteration() {
    assert_eq!(
        run_source("d = {0: 0}\nfor key in d:\n    d[0] = 1\n    print(key, d[0])"),
        Ok(vec!["0 1".to_string()])
    );
}

#[test]
fn rejects_dict_size_change_during_iteration() {
    assert_eq!(
        run_source("d = {0: 0}\nfor key in d:\n    d[key] = key + 1\nprint(d[0])"),
        Ok(vec!["1".to_string()])
    );
    assert_eq!(
        run_source("d = {1: 1}\nfor key in d:\n    d[key + 1] = 1"),
        Err("runtime error: RuntimeError: dictionary changed size during iteration".to_string())
    );
    assert_eq!(
        run_source("d = {0: 0}\nfor key in d:\n    del d[0]\n    d[0] = 0"),
        Err("runtime error: RuntimeError: dictionary keys changed during iteration".to_string())
    );
    assert_eq!(
        run_source("d = {0: 0}\nfor value in d.values():\n    del d[0]\n    d[0] = 0"),
        Err("runtime error: RuntimeError: dictionary keys changed during iteration".to_string())
    );
    assert_eq!(
        run_source("d = {0: 0}\nfor item in d.items():\n    del d[0]\n    d[0] = 0"),
        Err("runtime error: RuntimeError: dictionary keys changed during iteration".to_string())
    );
}

#[test]
fn runs_set_mutation_methods() {
    assert_eq!(
        run_source(
            "s = set([1])\nalias = s\ncopy = s.copy()\nprint(s.add(2), s)\ns.update([2, 3], (4,))\nprint(s is alias, copy is s, 1 in copy, 4 in s, len(s))\nprint(s.discard(9), s.remove(3), 3 in s)\npopped = s.pop()\nprint(popped in [1, 2, 4], len(s))\ns.clear()\nprint(len(s))"
        ),
        Ok(vec![
            "None {1, 2}".to_string(),
            "True False True True 4".to_string(),
            "None None False".to_string(),
            "True 2".to_string(),
            "0".to_string(),
        ])
    );
}

#[test]
fn runs_set_algebra_methods_and_operators() {
    assert_eq!(
        run_source(
            "s = {1, 2, 3}\nprint(s.union([3, 4], (5,)))\nprint(s.intersection([2, 3, 4], {3, 4}))\nprint(s.difference([1], {3}))\nprint(s.symmetric_difference([3, 4]))\nprint(s.issubset([1, 2, 3, 4]), s.issuperset([2, 3]), s.isdisjoint([4, 5]))\nalias = s\ns |= {4}\nprint(s, s is alias)\ns &= {2, 3, 4}\nprint(s)\ns ^= {3, 5}\nprint(s)\ns -= {2}\nprint(s)\ns.intersection_update([4, 5])\nprint(s)\ns.difference_update([4])\nprint(s)\ns.symmetric_difference_update([5, 6])\nprint(s)"
        ),
        Ok(vec![
            "{1, 2, 3, 4, 5}".to_string(),
            "{3}".to_string(),
            "{2}".to_string(),
            "{1, 2, 4}".to_string(),
            "True True True".to_string(),
            "{1, 2, 3, 4} True".to_string(),
            "{2, 3, 4}".to_string(),
            "{2, 4, 5}".to_string(),
            "{4, 5}".to_string(),
            "{4, 5}".to_string(),
            "{5}".to_string(),
            "{6}".to_string(),
        ])
    );
}

#[test]
fn runs_set_dunder_methods() {
    assert_eq!(
        run_source(
            "s = {1, 2, 3}\n\
             print(s.__len__(), s.__contains__(2), s.__contains__(9))\n\
             print(sorted(s.__or__({3, 4})))\n\
             print(sorted(s.__and__({2, 4})))\n\
             print(sorted(s.__sub__({1, 4})))\n\
             print(sorted(s.__xor__({3, 4})))\n\
             print(s.__le__({1, 2, 3, 4}), s.__lt__({1, 2, 3, 4}), s.__ge__({1, 2}), s.__gt__({1, 2}), s.__eq__({3, 2, 1}), s.__ne__({1, 2}))"
        ),
        Ok(vec![
            "3 True False".to_string(),
            "[1, 2, 3, 4]".to_string(),
            "[2]".to_string(),
            "[2, 3]".to_string(),
            "[1, 2, 4]".to_string(),
            "True True True True True True".to_string(),
        ])
    );
}

#[test]
fn returns_not_implemented_for_unsupported_set_dunder_operands() {
    assert_eq!(
        run_source(
            "s = {1}\n\
             print(s.__or__([2]), s.__and__([1]), s.__sub__([1]), s.__xor__([1]))\n\
             print(s.__le__([1]), s.__lt__([1]), s.__ge__([1]), s.__gt__([1]))\n\
             print(s.__eq__([1]), s.__ne__([1]))"
        ),
        Ok(vec![
            "NotImplemented NotImplemented NotImplemented NotImplemented".to_string(),
            "NotImplemented NotImplemented NotImplemented NotImplemented".to_string(),
            "NotImplemented NotImplemented".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_set_dunder_method_calls() {
    assert!(run_source("s = {1}\ns.__contains__([])").is_err());
    assert!(run_source("s = {1}\ns.__contains__()").is_err());
    assert!(run_source("s = {1}\ns.__contains__(1, 2)").is_err());
    assert!(run_source("s = {1}\ns.__or__()").is_err());
    assert!(run_source("s = {1}\ns.__or__({2}, {3})").is_err());
    assert!(run_source("s = {1}\ns.__and__()").is_err());
    assert!(run_source("s = {1}\ns.__sub__()").is_err());
    assert!(run_source("s = {1}\ns.__xor__()").is_err());
    assert!(run_source("s = {1}\ns.__le__()").is_err());
    assert!(run_source("s = {1}\ns.__len__(1)").is_err());
}

#[test]
fn runs_comprehension_unpacking() {
    assert_eq!(
        run_source(
            "print([*x for x in [[1, 2], (3, 4), {5: None}]])\nprint({*x for x in [[1, 2], [2, 3]]})\nprint({**d for d in [{\"a\": 1}, {\"b\": 2}, {\"a\": 3}]})\ng = (*(0, 1) for i in range(2))\nfor x in g:\n    print(x)"
        ),
        Ok(vec![
            "[1, 2, 3, 4, 5]".to_string(),
            "{1, 2, 3}".to_string(),
            "{'a': 3, 'b': 2}".to_string(),
            "0".to_string(),
            "1".to_string(),
            "0".to_string(),
            "1".to_string(),
        ])
    );
}

#[test]
fn prints_tuple_literals() {
    assert_eq!(
        run_source("print(())\nprint((1,))\nprint((1, 2, 3))\nprint((\"a\", True, 1 + 2))"),
        Ok(vec![
            "()".to_string(),
            "(1,)".to_string(),
            "(1, 2, 3)".to_string(),
            "('a', True, 3)".to_string(),
        ])
    );
}

#[test]
fn prints_dict_literals() {
    assert_eq!(
        run_source("print({})\nprint({\"a\": 1, \"b\": 2})\nprint({1 + 1: \"two\", True: [1, 2]})"),
        Ok(vec![
            "{}".to_string(),
            "{'a': 1, 'b': 2}".to_string(),
            "{2: 'two', True: [1, 2]}".to_string(),
        ])
    );
}

#[test]
fn runs_dict_display_unpacking() {
    assert_eq!(
        run_source(
            "base = {\"a\": 1, \"b\": 2}\noverride = {\"b\": 20, \"c\": 3}\nprint({**base, \"x\": 0, **override})\nprint({**{1: \"one\"}, 1: \"uno\"})"
        ),
        Ok(vec![
            "{'a': 1, 'b': 20, 'x': 0, 'c': 3}".to_string(),
            "{1: 'uno'}".to_string(),
        ])
    );
    assert_eq!(
        run_source("print({**1})"),
        Err("runtime error: dict update source must be a dict, got 1".to_string())
    );
}

#[test]
fn keeps_latest_dict_value_for_duplicate_key() {
    assert_eq!(
        run_source("print({\"a\": 1, \"a\": 2})"),
        Ok(vec!["{'a': 2}".to_string()])
    );
}

#[test]
fn prints_naked_tuple_expression() {
    assert_eq!(
        run_source("x = 1, 2, 3\nprint(x)\nprint((1, 2), 3)"),
        Ok(vec!["(1, 2, 3)".to_string(), "(1, 2) 3".to_string()])
    );
}

#[test]
fn assigns_and_reads_string() {
    assert_eq!(
        run_source("name = \"minipython\"\nprint(\"hello\", name)"),
        Ok(vec!["hello minipython".to_string()])
    );
}

#[test]
fn concatenates_strings() {
    assert_eq!(
        run_source("name = \"mini\" + \"python\"\nprint(name)"),
        Ok(vec!["minipython".to_string()])
    );
}

#[test]
fn runs_augmented_string_concat() {
    assert_eq!(
        run_source("name = \"mini\"\nname += \"python\"\nprint(name)"),
        Ok(vec!["minipython".to_string()])
    );
}

#[test]
fn assigns_and_reads_list() {
    assert_eq!(
        run_source("items = [1, 2, 3]\nprint(items)"),
        Ok(vec!["[1, 2, 3]".to_string()])
    );
}

#[test]
fn assigns_and_reads_tuple() {
    assert_eq!(
        run_source("items = (1, 2, 3)\nprint(items)"),
        Ok(vec!["(1, 2, 3)".to_string()])
    );
}

#[test]
fn runs_tuple_constructor() {
    assert_eq!(
        run_source(
            "print(tuple())\n\
             print(tuple([]))\n\
             print(tuple([0, 1, 2, 3]))\n\
             print(tuple(''))\n\
             print(tuple('spam'))\n\
             print(tuple(x for x in range(10) if x % 2))"
        ),
        Ok(vec![
            "()".to_string(),
            "()".to_string(),
            "(0, 1, 2, 3)".to_string(),
            "()".to_string(),
            "('s', 'p', 'a', 'm')".to_string(),
            "(1, 3, 5, 7, 9)".to_string(),
        ])
    );
}

#[test]
fn runs_scalar_builtin_constructors() {
    assert_eq!(
        run_source(
            "print(bool(), bool(0), bool(1), bool(''), bool('x'), bool([]), bool([0]))\n\
             print(int(), int(False), int(True), int(3.9), int(-3.9), int(' 42 '), int(b'7'))\n\
             print(float(), float(False), float(True), float(314), float('  3.14  '), float(b'2.5'))\n\
             print(str(), str(None), str(True), str(12), str(1.5), str('mini'), str(b'ab'))"
        ),
        Ok(vec![
            "False False True False True False True".to_string(),
            "0 0 1 3 -3 42 7".to_string(),
            "0.0 0.0 1.0 314.0 3.14 2.5".to_string(),
            " None True 12 1.5 mini b'ab'".to_string(),
        ])
    );
}

#[test]
fn runs_custom_numeric_conversion_protocols() {
    assert_eq!(
        run_source(
            r#"class IntOnly:
    def __int__(self):
        return 7
class IndexOnly:
    def __index__(self):
        return 4
class FloatOnly:
    def __float__(self):
        return 2.5
print(int(IntOnly()), int(IndexOnly()))
print(float(FloatOnly()), float(IndexOnly()))
print(list(range(IndexOnly())))
print(bytes(IndexOnly()))
items = [10, 20, 30, 40, 50]
print(items[IndexOnly()], items[IndexOnly():], items[:IndexOnly()])
print(list(enumerate([99], IndexOnly())))"#
        ),
        Ok(vec![
            "7 4".to_string(),
            "2.5 4.0".to_string(),
            "[0, 1, 2, 3]".to_string(),
            "b'\\x00\\x00\\x00\\x00'".to_string(),
            "50 [50] [10, 20, 30, 40]".to_string(),
            "[(4, 99)]".to_string(),
        ])
    );
}

#[test]
fn runs_int_constructor_with_base() {
    assert_eq!(
        run_source(
            r#"class BaseTwo:
    def __index__(self):
        return 2
print(int("10", 2), int("0b101", 0), int("+0xF", 0), int("-10", 2))
print(int(b"11", 2), int("z", 36), int("10", BaseTwo()), int("10", base=2))
print(int("1_0"), int("0b_1", 0), int("0x_f", 0), int("0o_7", 0))
print(int("१२३"), int("١٢٣"), int("１２_３"), int("0b١٠", 0))"#
        ),
        Ok(vec![
            "2 5 15 -2".to_string(),
            "3 35 2 2".to_string(),
            "10 1 15 7".to_string(),
            "123 123 123 2".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_custom_numeric_conversion_protocols() {
    assert_eq!(
        run_source(
            "class BadInt:\n    def __int__(self):\n        return 1.2\nprint(int(BadInt()))"
        ),
        Err("runtime error: TypeError: __int__ returned non-int (type float)".to_string())
    );
    assert_eq!(
        run_source(
            "class BadFloat:\n    def __float__(self):\n        return 1\nprint(float(BadFloat()))"
        ),
        Err(
            "runtime error: TypeError: BadFloat.__float__ returned non-float (type int)"
                .to_string()
        )
    );
    assert_eq!(
        run_source(
            "class BadIndex:\n    def __index__(self):\n        return 1.2\nprint(range(BadIndex()))"
        ),
        Err("runtime error: TypeError: __index__ returned non-int (type float)".to_string())
    );
    assert_eq!(
        run_source("print(int(10, 2))"),
        Err(
            "runtime error: TypeError: int() can't convert non-string with explicit base"
                .to_string()
        )
    );
    assert_eq!(
        run_source("print(int('10', 1))"),
        Err("runtime error: ValueError: int() base must be >= 2 and <= 36, or 0".to_string())
    );
    assert_eq!(
        run_source("print(int('0', 2 ** 100))"),
        Err("runtime error: ValueError: int() base must be >= 2 and <= 36, or 0".to_string())
    );
    assert_eq!(
        run_source("print(int('123\\x00'))"),
        Err(
            "runtime error: ValueError: invalid literal for int() with base 10: '123\\x00'"
                .to_string()
        )
    );
}

#[test]
fn runs_complex_builtin_constructor() {
    assert_eq!(
        run_source(concat!(
            "print(complex(), complex(1), complex(1.5), complex(True), complex(False))\n",
            "print(complex(1, -0.0), complex(1.5, 2), complex(real=1, imag=-0.0), complex(imag=-0.0))\n",
            "print(complex(1 + 2j), complex(1 + 2j, 3 + 4j))\n",
            "class FloatLike:\n",
            "    def __float__(self):\n",
            "        return 2.5\n",
            "class IndexLike:\n",
            "    def __index__(self):\n",
            "        return 7\n",
            "class ComplexLike:\n",
            "    def __complex__(self):\n",
            "        return 1 - 0.0j\n",
            "print(complex(FloatLike()), complex(IndexLike()), complex(ComplexLike()))\n",
            "print(complex.from_number(3.14), complex.from_number(3.14j), complex.from_number(314))\n",
            "print(complex.from_number(FloatLike()), complex.from_number(IndexLike()), complex.from_number(ComplexLike()))\n",
            "value = complex(1.25, -0.0)\n",
            "print(value.real, value.imag, 'real' in dir(value), 'imag' in dir(value))\n",
            "print(complex('1'), complex('1j'), complex('1+2j'), complex(' ( +4.25-6J )'))\n",
            "z = 3 + 4j\n",
            "print(type(complex.from_number(z)) is complex, 'from_number' in dir(complex), 'from_number' in dir(z), z.from_number(5))\n",
            "print(complex(5.3, 9.8).conjugate(), z.__complex__(), type(z.__complex__()) is complex)\n",
            "print(complex(1, -0.0).conjugate(), repr(complex(1, -0.0).conjugate().imag))\n",
            "print((1+2j).__getnewargs__(), complex(0, -0.0).__getnewargs__())\n",
            "print(hash(z) == z.__hash__(), '__hash__' in dir(z))\n",
            "print(z.__abs__(), complex.__abs__(z), z.__bool__(), (0j).__bool__())\n",
            "print(z.__pos__(), z.__neg__(), z.__repr__(), z.__str__())\n",
            "print(z.__eq__(3 + 4j), z.__ne__(3 + 5j), z.__lt__(3 + 4j) is NotImplemented)\n",
            "print('conjugate' in dir(z), '__complex__' in dir(z), '__getnewargs__' in dir(z))\n",
            "print('__abs__' in dir(z), '__bool__' in dir(z), '__eq__' in dir(z), '__ne__' in dir(z), '__lt__' in dir(z), '__pos__' in dir(z), '__neg__' in dir(z), '__repr__' in dir(z), '__str__' in dir(z))\n",
            "underflow = complex('-1e-500-1e-500j')\n",
            "print(repr(underflow.real), repr(underflow.imag), underflow)",
        )),
        Ok(vec![
            "0j (1+0j) (1.5+0j) (1+0j) 0j".to_string(),
            "(1-0j) (1.5+2j) (1-0j) -0j".to_string(),
            "(1+2j) (-3+5j)".to_string(),
            "(2.5+0j) (7+0j) (1-0j)".to_string(),
            "(3.14+0j) 3.14j (314+0j)".to_string(),
            "(2.5+0j) (7+0j) (1-0j)".to_string(),
            "1.25 -0.0 True True".to_string(),
            "(1+0j) 1j (1+2j) (4.25-6j)".to_string(),
            "True True True (5+0j)".to_string(),
            "(5.3-9.8j) (3+4j) True".to_string(),
            "(1+0j) 0.0".to_string(),
            "(1.0, 2.0) (0.0, -0.0)".to_string(),
            "True True".to_string(),
            "5.0 5.0 True False".to_string(),
            "(3+4j) (-3-4j) (3+4j) (3+4j)".to_string(),
            "True True True".to_string(),
            "True True True".to_string(),
            "True True True True True True True True True".to_string(),
            "-0.0 -0.0 (-0-0j)".to_string(),
        ])
    );

    assert_eq!(
        run_source(
            "class BadComplex:\n    def __complex__(self):\n        return 1\nDBL_MAX = 1.7976931348623157e308\nfor expr in [lambda: complex(1, 2, 3), lambda: complex([]), lambda: complex(10**1000), lambda: complex(BadComplex()), lambda: complex(''), lambda: complex('1+1j+1j'), lambda: complex('1+2j', 0), lambda: abs(complex(DBL_MAX, DBL_MAX)), lambda: complex.from_number('3.14'), lambda: complex.from_number(b'3.14'), lambda: complex.from_number()]:\n    try:\n        expr()\n    except (OverflowError, TypeError, ValueError) as error:\n        print(error.__class__.__name__)"
        ),
        Ok(vec![
            "TypeError".to_string(),
            "TypeError".to_string(),
            "OverflowError".to_string(),
            "TypeError".to_string(),
            "ValueError".to_string(),
            "ValueError".to_string(),
            "TypeError".to_string(),
            "OverflowError".to_string(),
            "TypeError".to_string(),
            "TypeError".to_string(),
            "TypeError".to_string(),
        ])
    );
}

#[test]
fn runs_abs_min_sum_builtins() {
    assert_eq!(
        run_source(
            "print(abs(0), abs(1234), abs(-1234), abs(True), abs(False))\n\
             print(abs(0.0), abs(3.14), abs(-3.14), abs(3 + 4j))\n\
             print(min('123123'))\n\
             print(min(1, 2, 3), min((1, 2, 3, 1, 2, 3)), min([1, 2, 3, 1, 2, 3]))\n\
             print(min(1, 2, 3.0), min(1.0, 2, 3))\n\
             print(sum([]), sum(list(range(2, 8))), sum(iter(list(range(2, 8)))))\n\
             print(sum(range(10), 1000), sum(i % 2 != 0 for i in range(10)))\n\
             print(sum([[1], [2], [3]], []))\n\
             print(sum([0.5, 1]), sum([1, 0.5]))\n\
             print(min([-3, 2, -1], key=abs), max([-3, 2, -1], key=abs))\n\
             print(min([1, 2, 3], key=lambda x: -x), max([1, 2, 3], key=lambda x: -x))\n\
             print(min([], default=\"empty\"), max([], default=\"empty\"))\n\
             print(min([], default=\"empty\", key=lambda x: 1))\n\
             print(min([2, 1], key=None), max([2, 1], key=None))"
        ),
        Ok(vec![
            "0 1234 1234 1 0".to_string(),
            "0.0 3.14 3.14 5.0".to_string(),
            "1".to_string(),
            "1 1 1".to_string(),
            "1 1.0".to_string(),
            "0 27 27".to_string(),
            "1045 5".to_string(),
            "[1, 2, 3]".to_string(),
            "1.5 1.5".to_string(),
            "-1 -3".to_string(),
            "3 1".to_string(),
            "empty empty".to_string(),
            "empty".to_string(),
            "1 2".to_string(),
        ])
    );
}

#[test]
fn runs_numeric_aggregate_builtins() {
    assert_eq!(
        run_source(concat!(
            "print(min([3, 1, 2]), max([3, 1, 2]))\n",
            "print(min('cab'), max('cab'))\n",
            "print(min(3, 1, 2), max(3, 1, 2))\n",
            "print(sum([1, 2, 3]))\n",
            "print(sum([True, False, 2]))\n",
            "print(sum([1, 2], 10))\n",
            "print(sum([[1], [2]], []))\n",
            "print(sum([(1,)], ()))\n",
            "print(abs(-3), abs(3), abs(False), abs(True))\n",
            "print(abs(3 + 4j))",
        )),
        Ok(vec![
            "1 3".to_string(),
            "a c".to_string(),
            "1 3".to_string(),
            "6".to_string(),
            "3".to_string(),
            "13".to_string(),
            "[1, 2]".to_string(),
            "(1,)".to_string(),
            "3 3 0 1".to_string(),
            "5.0".to_string(),
        ])
    );

    assert_eq!(
        run_source(
            "class AddBox:\n    def __init__(self, value):\n        self.value = value\n    def __radd__(self, other):\n        return other + self.value\nprint(sum([AddBox(2), AddBox(3)]))"
        ),
        Ok(vec!["5".to_string()])
    );

    assert_eq!(
        run_source(concat!(
            "events = []\n",
            "def values():\n",
            "    for value in [2, 1]:\n",
            "        events.append('yield ' + str(value))\n",
            "        yield value\n",
            "def key(value):\n",
            "    events.append('key ' + str(value))\n",
            "    return value\n",
            "print(min(values(), key=key), events)",
        )),
        Ok(vec![
            "1 ['yield 2', 'key 2', 'yield 1', 'key 1']".to_string()
        ])
    );
}

#[test]
fn rejects_invalid_numeric_aggregate_builtin_calls() {
    assert!(run_source("min()").is_err());
    assert!(run_source("min([])").is_err());
    assert!(run_source("min(1, 2, default=0)").is_err());
    assert!(run_source("max(1, 2, default=0)").is_err());
    assert!(run_source("min([], key=abs)").is_err());
    assert!(run_source("min([1], unknown=2)").is_err());
    assert!(run_source("sum()").is_err());
    assert!(run_source("sum([b'a'], b'')").is_err());
    assert!(run_source("abs()").is_err());
    assert!(run_source("abs(1, 2)").is_err());
}

#[test]
fn runs_rich_comparison_for_aggregate_and_sort_keys() {
    assert_eq!(
        run_source(concat!(
            "class Rank:\n",
            "    def __init__(self, value):\n",
            "        self.value = value\n",
            "    def __lt__(self, other):\n",
            "        return self.value < other.value\n",
            "    def __gt__(self, other):\n",
            "        return self.value > other.value\n",
            "class Item:\n",
            "    def __init__(self, label, rank):\n",
            "        self.label = label\n",
            "        self.rank = Rank(rank)\n",
            "def rank(item):\n",
            "    return item.rank\n",
            "items = [Item('c', 3), Item('a', 1), Item('b', 2), Item('a2', 1)]\n",
            "print(min(items, key=rank).label, max(items, key=rank).label)\n",
            "print([item.label for item in sorted(items, key=rank)])\n",
            "items.sort(key=rank)\n",
            "print([item.label for item in items])\n",
            "print([item.label for item in sorted(items, key=rank, reverse=True)])",
        )),
        Ok(vec![
            "a c".to_string(),
            "['a', 'a2', 'b', 'c']".to_string(),
            "['a', 'a2', 'b', 'c']".to_string(),
            "['c', 'b', 'a', 'a2']".to_string(),
        ])
    );
}

#[test]
fn runs_sorted_builtin() {
    assert_eq!(
        run_source(
            "copy = [3, 1, 2]\n\
             print(sorted(copy))\n\
             print(copy)\n\
             print(sorted(copy, key=lambda x: -x))\n\
             print(sorted(copy, reverse=True))\n\
             print(sorted([], key=None))\n\
             letters = sorted('abracadabra')\n\
             print(len(letters), letters[0], letters[1], letters[-1])\n\
             from_tuple = sorted(tuple('cab'))\n\
             from_set = sorted(set('cab'))\n\
             from_dict = sorted(dict.fromkeys('cab'))\n\
             print(from_tuple[0], from_tuple[1], from_tuple[2], from_set[0], from_set[1], from_set[2], from_dict[0], from_dict[1], from_dict[2])\n\
             print(sorted(x for x in [3, 1, 2]))\n\
             print(sorted([(1, 10), (1, 20), (0, 30)], key=lambda item: item[0], reverse=True))"
        ),
        Ok(vec![
            "[1, 2, 3]".to_string(),
            "[3, 1, 2]".to_string(),
            "[3, 2, 1]".to_string(),
            "[3, 2, 1]".to_string(),
            "[]".to_string(),
            "11 a a r".to_string(),
            "a b c a b c a b c".to_string(),
            "[1, 2, 3]".to_string(),
            "[(1, 10), (1, 20), (0, 30)]".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_sorted_builtin_calls() {
    assert!(run_source("sorted()").is_err());
    assert!(run_source("sorted(iterable=[])").is_err());
    assert!(run_source("sorted([], None)").is_err());
    assert!(run_source("sorted([1], bad=2)").is_err());
    assert!(run_source("sorted([1], reverse=[])").is_err());
    assert!(run_source("sorted([1, 'a'])").is_err());
}

#[test]
fn runs_list_reverse_and_sort_methods() {
    assert_eq!(
        run_source(
            "u = [-2, -1, 0, 1, 2]\n\
             original = u.copy()\n\
             print(u.reverse(), u)\n\
             print(u.reverse(), u == original)\n\
             u = [1, 0]\n\
             print(u.sort(), u)\n\
             u = [2, 1, 0, -1, -2]\n\
             u.sort()\n\
             print(u)\n\
             u.sort(key=lambda x: -x)\n\
             print(u)\n\
             u = [3, 1, 2]\n\
             u.sort(reverse=True)\n\
             print(u)\n\
             u = [(1, 10), (1, 20), (0, 30)]\n\
             u.sort(key=lambda item: item[0], reverse=True)\n\
             print(u)\n\
             nums = [2, 1]\n\
             alias = nums\n\
             print(nums.sort(key=None), alias is nums, alias)"
        ),
        Ok(vec![
            "None [2, 1, 0, -1, -2]".to_string(),
            "None True".to_string(),
            "None [0, 1]".to_string(),
            "[-2, -1, 0, 1, 2]".to_string(),
            "[2, 1, 0, -1, -2]".to_string(),
            "[3, 2, 1]".to_string(),
            "[(1, 10), (1, 20), (0, 30)]".to_string(),
            "None True [1, 2]".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_list_reverse_and_sort_calls() {
    assert!(run_source("u = [1, 2]\nu.reverse(42)").is_err());
    assert!(run_source("u = [1, 2]\nu.reverse(x=1)").is_err());
    assert!(run_source("u = [1, 0]\nu.sort(42)").is_err());
    assert!(run_source("u = [1, 0]\nu.sort([], [])").is_err());
    assert!(run_source("u = [1, 0]\nu.sort(bad=2)").is_err());
    assert!(run_source("u = [1, 0]\nu.sort(reverse=[])").is_err());
    assert!(run_source("u = [1, 'a']\nu.sort()").is_err());
}

#[test]
fn runs_list_insert_remove_count_and_index_methods() {
    assert_eq!(
        run_source(
            "a = [0, 1, 2]\n\
             print(a.insert(0, -2), a)\n\
             a.insert(1, -1)\n\
             a.insert(2, 0)\n\
             print(a)\n\
             b = a[:]\n\
             b.insert(-2, 99)\n\
             b.insert(-200, -99)\n\
             b.insert(200, 100)\n\
             print(b[0], b[-1], b.index(99), len(b))\n\
             c = [0, 1, 2] * 3\n\
             print(c.count(0), c.count(1), c.count(3))\n\
             d = [0, 0, 1]\n\
             print(d.remove(1), d)\n\
             print(d.remove(0), d)\n\
             print(d.remove(0), d)\n\
             u = [-2, -1, 0, 0, 1, 2]\n\
             print(u.index(0), u.index(0, 2), u.index(-2, -10), u.index(0, 3), u.index(0, 3, 4))\n\
             u.remove(0)\n\
             print(u)"
        ),
        Ok(vec![
            "None [-2, 0, 1, 2]".to_string(),
            "[-2, -1, 0, 0, 1, 2]".to_string(),
            "-99 100 5 9".to_string(),
            "3 3 0".to_string(),
            "None [0, 0]".to_string(),
            "None [0]".to_string(),
            "None []".to_string(),
            "2 2 0 3 3".to_string(),
            "[-2, -1, 0, 1, 2]".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_list_insert_remove_count_and_index_calls() {
    assert!(run_source("a = [0]\na.insert()").is_err());
    assert!(run_source("a = [0]\na.insert(1)").is_err());
    assert!(run_source("a = [0]\na.insert(1, 2, 3)").is_err());
    assert!(run_source("a = []\na.remove(0)").is_err());
    assert!(run_source("a = []\na.remove()").is_err());
    assert!(run_source("a = [1, 2]\na.index(3)").is_err());
    assert!(run_source("a = [1, 2]\na.index(2, 0, 1)").is_err());
    assert!(run_source("a = [1, 2]\na.index()").is_err());
    assert!(run_source("a = [1, 2]\na.index(1, 0, 2, 3)").is_err());
    assert!(run_source("a = [1, 2]\na.count()").is_err());
    assert!(run_source("a = [1, 2]\na.count(1, 2)").is_err());
}

#[test]
fn runs_list_dunder_sequence_methods() {
    assert_eq!(
        run_source(
            "a = [10, 11]\n\
             print(a.__getitem__(0), a.__getitem__(1), a.__getitem__(-2), a.__getitem__(-1))\n\
             print(a.__getitem__(slice(0, 1)), a.__getitem__(slice(1, 2)), a.__getitem__(slice(0, 2)), a.__getitem__(slice(0, 3)), a.__getitem__(slice(3, 5)))\n\
             b = [1, 2, 3]\n\
             print(b.__len__(), b.__contains__(2), b.__contains__(4))\n\
             print(b.__setitem__(1, 9), b)\n\
             print(b.__setitem__(slice(1, 3), [8, 9]), b)\n\
             print(b.__delitem__(1), b)\n\
             c = [1, 2, 3, 4]\n\
             print(c.__delitem__(slice(1, 3)), c)"
        ),
        Ok(vec![
            "10 11 10 11".to_string(),
            "[10] [11] [10, 11] [10, 11] []".to_string(),
            "3 True False".to_string(),
            "None [1, 9, 3]".to_string(),
            "None [1, 8, 9]".to_string(),
            "None [1, 9]".to_string(),
            "None [1, 4]".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_list_dunder_sequence_method_calls() {
    assert!(run_source("a = [10, 11]\na.__getitem__(-3)").is_err());
    assert!(run_source("a = [10, 11]\na.__getitem__(3)").is_err());
    assert!(run_source("a = [10, 11]\na.__getitem__('x')").is_err());
    assert!(run_source("a = [10, 11]\na.__getitem__(slice(0, 10, 0))").is_err());
    assert!(run_source("a = [10, 11]\na.__getitem__()").is_err());
    assert!(run_source("a = [10, 11]\na.__getitem__(0, 1)").is_err());
    assert!(run_source("a = [1, 2]\na.__setitem__()").is_err());
    assert!(run_source("a = [1, 2]\na.__setitem__(0)").is_err());
    assert!(run_source("a = [1, 2]\na.__setitem__(0, 3, 4)").is_err());
    assert!(run_source("a = [1, 2]\na.__delitem__()").is_err());
    assert!(run_source("a = [1, 2]\na.__delitem__(0, 1)").is_err());
    assert!(run_source("a = [1, 2]\na.__contains__()").is_err());
    assert!(run_source("a = [1, 2]\na.__contains__(1, 2)").is_err());
    assert!(run_source("a = [1, 2]\na.__len__(1)").is_err());
}

#[test]
fn runs_immutable_sequence_dunder_methods() {
    assert_eq!(
        run_source(
            "t = (10, 11)\n\
             print(t.__getitem__(0), t.__getitem__(1), t.__getitem__(-2), t.__getitem__(-1), t.__getitem__(slice(0, 2)))\n\
             print(t.__len__(), t.__contains__(10), t.__contains__(99))\n\
             print(t.count(10), t.count(99), t.index(11))\n\
             s = 'abc'\n\
             print(s.__getitem__(0), s.__getitem__(-1), s.__getitem__(slice(0, 2)), s.__contains__('b'), s.__contains__('z'), s.__len__())\n\
             b = b'abc'\n\
             print(b.__getitem__(0), b.__getitem__(-1), b.__getitem__(slice(0, 2)), b.__contains__(98), b.__contains__(120), b.__len__())\n\
             r = range(1, 6, 2)\n\
             print(r.__getitem__(0), r.__getitem__(-1), list(r.__getitem__(slice(0, 2))), r.__contains__(3), r.__contains__(4), r.__len__())\n\
             print(r.count(3), r.count(4), r.index(5))"
        ),
        Ok(vec![
            "10 11 10 11 (10, 11)".to_string(),
            "2 True False".to_string(),
            "1 0 1".to_string(),
            "a c ab True False 3".to_string(),
            "97 99 b'ab' True False 3".to_string(),
            "1 5 [1, 3] True False 3".to_string(),
            "1 0 2".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_immutable_sequence_dunder_method_calls() {
    assert!(run_source("t = (10, 11)\nt.__getitem__(3)").is_err());
    assert!(run_source("s = 'abc'\ns.__getitem__(3)").is_err());
    assert!(run_source("b = b'abc'\nb.__getitem__(3)").is_err());
    assert!(run_source("r = range(3)\nr.__getitem__(3)").is_err());
    assert!(run_source("t = (10, 11)\nt.__getitem__('x')").is_err());
    assert!(run_source("s = 'abc'\ns.__contains__(1)").is_err());
    assert!(run_source("b = b'abc'\nb.__contains__('a')").is_err());
    assert!(run_source("t = (1, 2)\nt.__len__(1)").is_err());
    assert!(run_source("s = 'abc'\ns.__getitem__()").is_err());
    assert!(run_source("b = b'abc'\nb.__contains__()").is_err());
    assert!(run_source("r = range(3)\nr.__getitem__(0, 1)").is_err());
    assert!(run_source("t = (1, 2)\nt.count()").is_err());
    assert!(run_source("t = (1, 2)\nt.index()").is_err());
    assert!(run_source("t = (1, 2)\nt.index(3)").is_err());
    assert!(run_source("r = range(3)\nr.index(3)").is_err());
}

#[test]
fn runs_iter_and_next_builtins() {
    assert_eq!(
        run_source(
            "it = iter([1, 2])\nprint(iter(it) is it)\nprint(next(it), next(it), next(it, 42), next(it, 43))\nit = iter(range(3))\nprint(next(it), list(it))\nletters = iter('ab')\nprint(next(letters), next(letters), next(letters, 'done'))\nd = {1: 2, 3: 4}\nkeys = iter(d)\nprint(next(keys), list(keys))\nrev = reversed([1, 2])\nprint(next(rev), next(rev), next(rev, 'done'))\nempty = iter([])\ntry:\n    next(empty)\nexcept StopIteration:\n    print('stopped')"
        ),
        Ok(vec![
            "True".to_string(),
            "1 2 42 43".to_string(),
            "0 [1, 2]".to_string(),
            "a b done".to_string(),
            "1 [3]".to_string(),
            "2 1 done".to_string(),
            "stopped".to_string(),
        ])
    );
}

#[test]
fn runs_all_and_any_builtins() {
    assert_eq!(
        run_source(
            "print(all([2, 4, 6]), all([2, None, 6]), all([]))\nprint(any([None, None, None]), any([None, 4, None]), any([]))\ndef false_then_fail():\n    yield 0\n    raise RuntimeError('boom')\ndef true_then_fail():\n    yield 1\n    raise RuntimeError('boom')\nprint(all(false_then_fail()), any(true_then_fail()))\ns = [50, 60]\nprint(all(x > 42 for x in s))\ns = [50, 40, 60]\nprint(all(x > 42 for x in s), any(x > 42 for x in s))"
        ),
        Ok(vec![
            "True False True".to_string(),
            "False True False".to_string(),
            "False True".to_string(),
            "True".to_string(),
            "False True".to_string(),
        ])
    );
}

#[test]
fn runs_enumerate_zip_and_sorted_builtins() {
    assert_eq!(
        run_source(
            "print(list(enumerate('abc')))\nprint(list(enumerate(['a', 'b'], 5)))\ne = enumerate(range(3), start=True)\nprint(next(e), next(e), list(e))\nprint(list(enumerate(iterable='ab', start=2)))\nprint(list(zip((1, 2, 3), (4, 5, 6))))\nprint(list(zip((1, 2, 3), [4, 5, 6, 7])))\nprint(list(zip()), list(zip(*[])))\nprint(list(zip(range(5), range(10))))\nprint(list(zip((x for x in range(3)), 'abc')))\nprint(sorted([3, 1, 2]))\nprint(sorted([1, 2, 3], key=lambda x: -x))\nprint(sorted([3, 1, 2], reverse=True))"
        ),
        Ok(vec![
            "[(0, 'a'), (1, 'b'), (2, 'c')]".to_string(),
            "[(5, 'a'), (6, 'b')]".to_string(),
            "(1, 0) (2, 1) [(3, 2)]".to_string(),
            "[(2, 'a'), (3, 'b')]".to_string(),
            "[(1, 4), (2, 5), (3, 6)]".to_string(),
            "[(1, 4), (2, 5), (3, 6)]".to_string(),
            "[] []".to_string(),
            "[(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]".to_string(),
            "[(0, 'a'), (1, 'b'), (2, 'c')]".to_string(),
            "[1, 2, 3]".to_string(),
            "[3, 2, 1]".to_string(),
            "[3, 2, 1]".to_string(),
        ])
    );
}

#[test]
fn runs_map_and_filter_builtins() {
    assert_eq!(
        run_source(
            "print(list(map(lambda x: x * x, range(1, 4))))\ndef plus(*values):\n    total = 0\n    for value in values:\n        total += value\n    return total\nprint(list(map(plus, [1, 3, 7], [4, 9, 2], [1, 1, 0])))\nm = map(lambda x: x + 1, [1, 2])\nprint(next(m), list(m), iter(m) is m)\nprint(list(map(lambda x: x + 10, (i for i in range(3)))))\nprint(list(filter(None, [1, [], [3], None, 9, 0, False, True])))\nprint(list(filter(lambda x: x > 0, [1, -3, 9, 0, 2])))\nf = filter(lambda x: x % 2, range(6))\nprint(next(f), list(f), iter(f) is f)\nclass Squares:\n    def __init__(self, stop):\n        self.stop = stop\n    def __getitem__(self, index):\n        if index < 0 or index >= self.stop:\n            raise IndexError\n        return index * index\nprint(list(map(int, Squares(5))))\nprint(list(filter(lambda x: x % 2, Squares(6))))\ndef pair(left, right):\n    return left + right\nprint(list(map(pair, [1, 2, 3], [10, 20])))\ndef echo(*args):\n    print(args)\necho(*map(lambda x: x + 1, [1, 2]))\nprint(set(map(lambda x: x + 1, [1, 1, 2])))"
        ),
        Ok(vec![
            "[1, 4, 9]".to_string(),
            "[6, 13, 9]".to_string(),
            "2 [3] True".to_string(),
            "[10, 11, 12]".to_string(),
            "[1, [3], 9, True]".to_string(),
            "[1, 9, 2]".to_string(),
            "1 [3, 5] True".to_string(),
            "[0, 1, 4, 9, 16]".to_string(),
            "[1, 9, 25]".to_string(),
            "[11, 22]".to_string(),
            "(2, 3)".to_string(),
            "{2, 3}".to_string(),
        ])
    );

    assert!(run_source("map()").is_err());
    assert!(run_source("map(lambda x: x)").is_err());
    assert!(run_source("map(lambda x: x, 42)").is_err());
    assert!(run_source("list(map(None, [1]))").is_err());
    assert!(run_source("filter()").is_err());
    assert!(run_source("filter(None)").is_err());
    assert!(run_source("filter(None, 42)").is_err());
    assert!(run_source("list(filter(42, [1]))").is_err());
}

#[test]
fn runs_user_defined_iterators_and_sequence_fallback() {
    assert_eq!(
        run_source(
            "class Counter:\n    def __init__(self, stop):\n        self.current = 0\n        self.stop = stop\n    def __iter__(self):\n        return self\n    def __next__(self):\n        if self.current >= self.stop:\n            raise StopIteration\n        value = self.current\n        self.current += 1\n        return value\ncounter = Counter(3)\nprint(iter(counter) is counter)\nprint(next(counter), list(counter))\nprint(list(enumerate(Counter(3), 5)))\nprint(list(zip(Counter(3), [10, 11, 12, 13])))\nclass Squares:\n    def __init__(self, stop):\n        self.stop = stop\n    def __getitem__(self, index):\n        if index < 0 or index >= self.stop:\n            raise IndexError\n        return index * index\nprint(list(Squares(4)))\nprint(list(enumerate(Squares(3))))\nprint(list(zip(Squares(3), range(10))))"
        ),
        Ok(vec![
            "True".to_string(),
            "0 [1, 2]".to_string(),
            "[(5, 0), (6, 1), (7, 2)]".to_string(),
            "[(0, 10), (1, 11), (2, 12)]".to_string(),
            "[0, 1, 4, 9]".to_string(),
            "[(0, 0), (1, 1), (2, 4)]".to_string(),
            "[(0, 0), (1, 1), (4, 2)]".to_string(),
        ])
    );
}

#[test]
fn runs_collections_abc_iterable_iterator_mixins() {
    assert_eq!(
        run_source(
            "from collections.abc import Iterable, Iterator\nclass I(Iterable):\n    def __iter__(self):\n        return super().__iter__()\nprint(list(I()))\nclass N(Iterator):\n    def __next__(self):\n        return super().__next__()\nn = N()\nprint(iter(n) is n)\ntry:\n    next(n)\nexcept StopIteration:\n    print('stopped')"
        ),
        Ok(vec![
            "[]".to_string(),
            "True".to_string(),
            "stopped".to_string(),
        ])
    );
}

#[test]
fn runs_builtin_iterator_dunder_methods() {
    assert_eq!(
        run_source(
            "it = [1, 2].__iter__()\nprint(it.__iter__() is it)\nprint(it.__next__(), it.__next__())\ntry:\n    it.__next__()\nexcept StopIteration:\n    print('stopped')\nprint(list((3, 4).__iter__()))\nprint(list(range(3).__iter__()))\nprint(list(b'ab'.__iter__()))\nletters = 'ab'.__iter__()\nprint(letters.__next__(), letters.__next__())\nd = {1: 10, 2: 20}\nprint(list(d.__iter__()))\nprint(list(d.keys().__iter__()), list(d.values().__iter__()), list(d.items().__iter__()))\nprint(sorted({2, 1}.__iter__()))\nz = zip([1, 2], [3, 4])\nprint(z.__iter__() is z, z.__next__(), list(z))\nm = map(lambda x: x + 1, [1, 2])\nprint(m.__next__(), list(m))\nf = filter(None, [0, 3, 0, 4])\nprint(f.__next__(), list(f))\ndef gen():\n    yield 5\n    yield 6\ng = gen()\nprint(g.__iter__() is g, g.__next__(), list(g))"
        ),
        Ok(vec![
            "True".to_string(),
            "1 2".to_string(),
            "stopped".to_string(),
            "[3, 4]".to_string(),
            "[0, 1, 2]".to_string(),
            "[97, 98]".to_string(),
            "a b".to_string(),
            "[1, 2]".to_string(),
            "[1, 2] [10, 20] [(1, 10), (2, 20)]".to_string(),
            "[1, 2]".to_string(),
            "True (1, 3) [(2, 4)]".to_string(),
            "2 [3]".to_string(),
            "3 [4]".to_string(),
            "True 5 [6]".to_string(),
        ])
    );

    assert!(run_source("[1].__iter__(0)").is_err());
    assert!(run_source("iter([1]).__next__(99)").is_err());
    assert!(run_source("[1].__next__()").is_err());
}

#[test]
fn runs_callable_sentinel_iterators() {
    assert_eq!(
        run_source(
            "values = [1, 2, 'stop', 99]\ndef next_value():\n    return values.pop(0)\nit = iter(next_value, 'stop')\nprint(iter(it) is it)\nprint(next(it), list(it))\ntry:\n    next(it)\nexcept StopIteration:\n    print('stopped')\nagain = [7, 'stop']\ndef next_again():\n    return again.pop(0)\nagain_it = iter(next_again, 'stop')\nprint(again_it.__iter__() is again_it, again_it.__next__())\nclass Counter:\n    def __init__(self):\n        self.value = 0\n    def __call__(self):\n        self.value += 1\n        return self.value\nprint(list(iter(Counter(), 4)))\ndef always_five():\n    return 5\nprint(next(iter(always_five, 5), 'done'))\nitems = [[1], [2], []]\ndef next_list():\n    return items.pop(0)\nprint(list(iter(next_list, [])))"
        ),
        Ok(vec![
            "True".to_string(),
            "1 [2]".to_string(),
            "stopped".to_string(),
            "True 7".to_string(),
            "[1, 2, 3]".to_string(),
            "done".to_string(),
            "[[1], [2]]".to_string(),
        ])
    );

    assert!(run_source("iter(1, 2)").is_err());
    assert!(run_source("iter(lambda: 1, 2, 3)").is_err());
}

#[test]
fn runs_iterator_length_hints() {
    assert_eq!(
        run_source(
            "it = iter([1, 2, 3])\nprint(it.__length_hint__(), next(it), it.__length_hint__(), list(it), it.__length_hint__())\nprint(iter((1, 2)).__length_hint__(), iter('abc').__length_hint__(), iter(b'ab').__length_hint__(), iter(range(4)).__length_hint__())\nd = {1: 10, 2: 20}\nkeys = iter(d)\nprint(keys.__length_hint__(), next(keys), keys.__length_hint__())\nprint(iter(d.values()).__length_hint__(), iter(d.items()).__length_hint__())\ngrowing = {1: 1}\ngrowing_keys = iter(growing)\ngrowing[2] = 2\nprint(growing_keys.__length_hint__())\nrev = reversed([1, 2, 3])\nprint(rev.__length_hint__(), next(rev), rev.__length_hint__())\nclass S:\n    def __init__(self):\n        self.items = [10, 20, 30]\n    def __getitem__(self, index):\n        if index >= len(self.items):\n            raise IndexError\n        return self.items[index]\n    def __len__(self):\n        return len(self.items)\nseq = iter(S())\nprint(seq.__length_hint__(), next(seq), seq.__length_hint__())\nclass N:\n    def __getitem__(self, index):\n        if index >= 2:\n            raise IndexError\n        return index\nprint(iter(N()).__length_hint__())"
        ),
        Ok(vec![
            "3 1 2 [2, 3] 0".to_string(),
            "2 3 2 4".to_string(),
            "2 1 1".to_string(),
            "2 2".to_string(),
            "0".to_string(),
            "3 3 2".to_string(),
            "3 10 2".to_string(),
            "NotImplemented".to_string(),
        ])
    );

    assert!(run_source("enumerate([1]).__length_hint__()").is_err());
    assert!(run_source("zip([1]).__length_hint__()").is_err());
    assert!(run_source("iter([1]).__length_hint__(0)").is_err());
}

#[test]
fn assigns_and_reads_dict() {
    assert_eq!(
        run_source("items = {\"a\": 1, \"b\": 2}\nprint(items)"),
        Ok(vec!["{'a': 1, 'b': 2}".to_string()])
    );
}

#[test]
fn assigns_to_list_subscripts() {
    assert_eq!(
        run_source("items = [1, 2, 3]\nitems[1] = 20\nitems[0] += 4\nitems[-1] = 30\nprint(items)"),
        Ok(vec!["[5, 20, 30]".to_string()])
    );
}

#[test]
fn assigns_to_dict_subscripts() {
    assert_eq!(
        run_source("items = {\"a\": 1}\nitems[\"b\"] = 2\nitems[\"a\"] += 3\nprint(items)"),
        Ok(vec!["{'a': 4, 'b': 2}".to_string()])
    );
}

#[test]
fn assigns_to_nested_subscripts() {
    assert_eq!(
        run_source("items = [[1], [2]]\nitems[0][0] = 9\nprint(items)"),
        Ok(vec!["[[9], [2]]".to_string()])
    );
}

#[test]
fn assigns_to_list_slices() {
    assert_eq!(
        run_source(
            "items = [0, 1, 2, 3]\nitems[1:3] = [9, 8, 7]\nprint(items)\nitems[:0] = (5, 6)\nprint(items)\nitems[::2] = [10, 20, 30, 40]\nprint(items)"
        ),
        Ok(vec![
            "[0, 9, 8, 7, 3]".to_string(),
            "[5, 6, 0, 9, 8, 7, 3]".to_string(),
            "[10, 6, 20, 9, 30, 7, 40]".to_string(),
        ])
    );
}

#[test]
fn assigns_to_nested_list_slices() {
    assert_eq!(
        run_source("items = [[0, 1], [2, 3]]\nitems[0][0:2] = [9]\nprint(items)"),
        Ok(vec!["[[9], [2, 3]]".to_string()])
    );
}

#[test]
fn deletes_names_attributes_and_subscripts() {
    assert_eq!(
        run_source(
            "items = [1, 2, 3]\ndel items[1]\nvalues = {\"a\": 1, \"b\": 2}\ndel values[\"a\"]\nclass Box:\n    pass\nbox = Box()\nbox.value = 5\ndel box.value\nx = 1\ndel x\nprint(items, values)"
        ),
        Ok(vec!["[1, 3] {'b': 2}".to_string()])
    );
}

#[test]
fn deletes_list_slices() {
    assert_eq!(
        run_source(
            "items = [0, 1, 2, 3, 4, 5]\ndel items[1:4]\nprint(items)\ndel items[::2]\nprint(items)"
        ),
        Ok(vec!["[0, 4, 5]".to_string(), "[4]".to_string()])
    );
}

#[test]
fn reports_delete_errors() {
    assert_eq!(
        run_source("x = 1\ndel x\nprint(x)"),
        Err("runtime error: NameError: unknown name: x".to_string())
    );
    assert_eq!(
        run_source("items = [1]\ndel items[2]"),
        Err("runtime error: IndexError: list index out of range".to_string())
    );
    assert_eq!(
        run_source("items = {}\ndel items[\"missing\"]"),
        Err("runtime error: KeyError: 'missing'".to_string())
    );
}

#[test]
fn reports_slice_assignment_errors() {
    assert_eq!(
        run_source("items = [0, 1, 2]\nitems[::2] = [9]"),
        Err(
            "runtime error: attempt to assign sequence of size 1 to extended slice of size 2"
                .to_string()
        )
    );
    assert_eq!(
        run_source("items = (0, 1, 2)\nitems[0:1] = [9]"),
        Err(
            "runtime error: TypeError: 'tuple' object does not support item assignment".to_string()
        )
    );
    assert_eq!(
        run_source("items = (0, 1, 2)\ndel items[0:1]"),
        Err("runtime error: TypeError: 'tuple' object does not support item deletion".to_string())
    );
}

#[test]
fn unpacks_tuple_and_list_assignments() {
    assert_eq!(
        run_source(
            "a, b = (1, 2)\nprint(a, b)\n(c, d) = [3, 4]\nprint(c, d)\n[e, f] = (5, 6)\nprint(e, f)"
        ),
        Ok(vec![
            "1 2".to_string(),
            "3 4".to_string(),
            "5 6".to_string(),
        ])
    );
}

#[test]
fn unpacks_starred_assignments() {
    assert_eq!(
        run_source(
            "a, *rest = [1, 2, 3]\nprint(a, rest)\n*prefix, last = (1, 2, 3)\nprint(prefix, last)\nfirst, *middle, last = range(5)\nprint(first, middle, last)\n[a, *tail] = \"xy\"\nprint(a, tail)"
        ),
        Ok(vec![
            "1 [2, 3]".to_string(),
            "[1, 2] 3".to_string(),
            "0 [1, 2, 3] 4".to_string(),
            "x ['y']".to_string(),
        ])
    );
}

#[test]
fn reports_unpack_assignment_errors() {
    assert_eq!(
        run_source("a, b = (1,)"),
        Err(
            "runtime error: ValueError: not enough values to unpack (expected 2, got 1)"
                .to_string()
        )
    );
    assert_eq!(
        run_source("a, b = (1, 2, 3)"),
        Err("runtime error: ValueError: too many values to unpack (expected 2, got 3)".to_string())
    );
    assert_eq!(
        run_source("a, *rest, b = [1]"),
        Err(
            "runtime error: ValueError: not enough values to unpack (expected at least 2, got 1)"
                .to_string()
        )
    );
    assert_eq!(
        run_source("a, *b, *c = [1, 2, 3]"),
        Err("parse error: multiple starred expressions in assignment".to_string())
    );
    assert_eq!(
        run_source("*rest = [1, 2]"),
        Err("parse error: starred assignment target must be in a list or tuple".to_string())
    );
}

#[test]
fn indexes_lists_and_strings() {
    assert_eq!(
        run_source(
            "items = [10, 20, 30]\nprint(items[0], items[1], items[-1])\nprint([1, 2, 3][2])\nprint(\"abc\"[1], \"abc\"[-1])"
        ),
        Ok(vec![
            "10 20 30".to_string(),
            "3".to_string(),
            "b c".to_string(),
        ])
    );
}

#[test]
fn indexes_range_values() {
    assert_eq!(
        run_source("values = range(1, 6, 2)\nprint(values[0], values[1], values[-1])"),
        Ok(vec!["1 3 5".to_string()])
    );
}

#[test]
fn indexes_and_slices_tuples() {
    assert_eq!(
        run_source(
            "items = (10, 20, 30, 40)\nprint(items[0], items[-1])\nprint(items[1:3])\nprint(items[::-1])"
        ),
        Ok(vec![
            "10 40".to_string(),
            "(20, 30)".to_string(),
            "(40, 30, 20, 10)".to_string(),
        ])
    );
}

#[test]
fn indexes_dicts() {
    assert_eq!(
        run_source(
            "items = {\"a\": 1, \"b\": 2}\nprint(items[\"a\"], items[\"b\"])\nprint({1: \"one\"}[1])"
        ),
        Ok(vec!["1 2".to_string(), "one".to_string()])
    );
}

#[test]
fn slices_lists_and_strings() {
    assert_eq!(
        run_source(
            "items = [0, 1, 2, 3, 4]\nprint(items[1:4])\nprint(items[:2])\nprint(items[2:])\nprint(items[:])\nprint(items[-4:-1])\nprint(items[::2])\nprint(items[::-1])\nprint(\"01234\"[1:4], \"01234\"[:2], \"01234\"[2:], \"01234\"[::-1])"
        ),
        Ok(vec![
            "[1, 2, 3]".to_string(),
            "[0, 1]".to_string(),
            "[2, 3, 4]".to_string(),
            "[0, 1, 2, 3, 4]".to_string(),
            "[1, 2, 3]".to_string(),
            "[0, 2, 4]".to_string(),
            "[4, 3, 2, 1, 0]".to_string(),
            "123 01 234 43210".to_string(),
        ])
    );
}

#[test]
fn slices_ranges() {
    assert_eq!(
        run_source(
            "values = range(1, 8, 2)\nprint(values[1:])\nfor value in values[::-1]:\n    print(value)"
        ),
        Ok(vec![
            "range(3, 9, 2)".to_string(),
            "7".to_string(),
            "5".to_string(),
            "3".to_string(),
            "1".to_string(),
        ])
    );
}

#[test]
fn reports_unknown_name() {
    assert_eq!(
        run_source("unknown(1)"),
        Err("runtime error: NameError: unknown name: unknown".to_string())
    );
}

#[test]
fn reports_non_callable_value() {
    assert_eq!(
        run_source("1(2)"),
        Err("runtime error: TypeError: 1 is not callable".to_string())
    );
}

#[test]
fn reports_arithmetic_type_errors() {
    assert_eq!(
        run_source("print(\"a\" - \"b\")"),
        Err("runtime error: cannot subtract a and b".to_string())
    );
    assert_eq!(
        run_source("print(1 @ 2)"),
        Err(
            "runtime error: TypeError: unsupported operand type(s) for @: 'int' and 'int'"
                .to_string()
        )
    );
    assert_eq!(
        run_source("x = 1\nx @= 2"),
        Err(
            "runtime error: TypeError: unsupported operand type(s) for @: 'int' and 'int'"
                .to_string()
        )
    );
}

#[test]
fn runs_matrix_multiply_special_methods() {
    assert_eq!(
        run_source(
            "class M:\n    def __matmul__(self, other):\n        return other - 1\n    def __rmatmul__(self, other):\n        return other + 2\n    def __imatmul__(self, other):\n        self.other = other\n        return self\nm = M()\nprint(m @ 42)\nprint(40 @ m)\nm @= 7\nprint(m.other)"
        ),
        Ok(vec!["41".to_string(), "42".to_string(), "7".to_string()])
    );
}

#[test]
fn reports_division_by_zero() {
    assert_eq!(
        run_source("print(1 / 0)"),
        Err("runtime error: ZeroDivisionError: division by zero".to_string())
    );
    assert_eq!(
        run_source("print(1 // 0)"),
        Err("runtime error: ZeroDivisionError: integer division or modulo by zero".to_string())
    );
    assert_eq!(
        run_source("print(1 % 0)"),
        Err("runtime error: ZeroDivisionError: integer division or modulo by zero".to_string())
    );
    assert_eq!(
        run_source(
            "try:\n    1 / 0\nexcept ZeroDivisionError as error:\n    print(error.__class__.__name__, error)"
        ),
        Ok(vec!["ZeroDivisionError division by zero".to_string()])
    );
}

#[test]
fn reports_empty_grouped_expression() {
    assert_eq!(
        run_source("print((,))"),
        Err("parse error: expected expression, found Comma".to_string())
    );
}

#[test]
fn reports_unclosed_grouped_expression() {
    assert_eq!(
        run_source("print((1 + 2"),
        Err("parse error: '(' was never closed".to_string())
    );
}

#[test]
fn runs_starred_sequence_displays() {
    assert_eq!(
        run_source(
            "items = [1, 2]\nprint([0, *items, 3])\nprint((*items, 3))\nprint({*items, 2, 3})\nprint([*\"ab\"])\nprint((*range(3),))"
        ),
        Ok(vec![
            "[0, 1, 2, 3]".to_string(),
            "(1, 2, 3)".to_string(),
            "{1, 2, 3}".to_string(),
            "['a', 'b']".to_string(),
            "(0, 1, 2)".to_string(),
        ])
    );
}

#[test]
fn reports_invalid_starred_group_expressions() {
    assert_eq!(
        run_source("(*x)"),
        Err("parse error: cannot use starred expression here".to_string())
    );
    assert_eq!(
        run_source("(**x)"),
        Err("parse error: cannot use double starred expression here".to_string())
    );
}

#[test]
fn runs_if_then_branch() {
    assert_eq!(
        run_source("if True:\n    print(\"yes\")"),
        Ok(vec!["yes".to_string()])
    );
}

#[test]
fn runs_pass_inside_if_branch() {
    assert_eq!(
        run_source("if True:\n    pass\nprint(\"after\")"),
        Ok(vec!["after".to_string()])
    );
}

#[test]
fn skips_comment_only_lines_inside_block() {
    assert_eq!(
        run_source("if True:\n    # comment\n    print(\"yes\")"),
        Ok(vec!["yes".to_string()])
    );
}

#[test]
fn skips_if_then_branch_when_false() {
    assert_eq!(run_source("if False:\n    print(\"yes\")"), Ok(Vec::new()));
}

#[test]
fn runs_if_else_branch() {
    assert_eq!(
        run_source("if False:\n    print(\"yes\")\nelse:\n    print(\"no\")"),
        Ok(vec!["no".to_string()])
    );
}

#[test]
fn runs_elif_branch() {
    assert_eq!(
        run_source(
            "x = 2\nif x == 1:\n    print(\"one\")\nelif x == 2:\n    print(\"two\")\nelse:\n    print(\"other\")"
        ),
        Ok(vec!["two".to_string()])
    );
}

#[test]
fn skips_elif_after_true_if_branch() {
    assert_eq!(
        run_source(
            "if True:\n    print(\"then\")\nelif True:\n    print(\"elif\")\nelse:\n    print(\"else\")"
        ),
        Ok(vec!["then".to_string()])
    );
}

#[test]
fn runs_else_after_false_elif_chain() {
    assert_eq!(
        run_source(
            "if False:\n    print(\"then\")\nelif False:\n    print(\"first elif\")\nelif 0:\n    print(\"second elif\")\nelse:\n    print(\"else\")"
        ),
        Ok(vec!["else".to_string()])
    );
}

#[test]
fn runs_if_condition_from_comparison() {
    assert_eq!(
        run_source("x = 3\nif x == 3:\n    print(\"match\")"),
        Ok(vec!["match".to_string()])
    );
}

#[test]
fn runs_truthy_if_conditions() {
    assert_eq!(
        run_source(
            "if 1:\n    print(\"one\")\nif 0:\n    print(\"zero\")\nelse:\n    print(\"zero false\")"
        ),
        Ok(vec!["one".to_string(), "zero false".to_string()])
    );
}

#[test]
fn runs_string_truthy_if_conditions() {
    assert_eq!(
        run_source(
            "if \"x\":\n    print(\"nonempty\")\nif \"\":\n    print(\"empty\")\nelse:\n    print(\"empty false\")"
        ),
        Ok(vec!["nonempty".to_string(), "empty false".to_string()])
    );
}

#[test]
fn runs_while_loop() {
    assert_eq!(
        run_source("x = 0\nwhile x < 3:\n    print(x)\n    x = x + 1"),
        Ok(vec!["0".to_string(), "1".to_string(), "2".to_string()])
    );
}

#[test]
fn runs_for_loop_over_range() {
    assert_eq!(
        run_source("for x in range(3):\n    print(x)"),
        Ok(vec!["0".to_string(), "1".to_string(), "2".to_string()])
    );
}

#[test]
fn runs_for_loop_over_list() {
    assert_eq!(
        run_source("for x in [1, 2, 3]:\n    print(x)"),
        Ok(vec!["1".to_string(), "2".to_string(), "3".to_string()])
    );
}

#[test]
fn runs_for_loop_over_list_variable() {
    assert_eq!(
        run_source("items = [\"a\", \"b\"]\nfor item in items:\n    print(item)"),
        Ok(vec!["a".to_string(), "b".to_string()])
    );
}

#[test]
fn runs_for_loop_over_mutating_list() {
    assert_eq!(
        run_source(
            "items = [1]\nfor item in items:\n    print(item)\n    if len(items) < 3:\n        items.append(len(items) + 1)\nprint(items)\nitems = [1, 2]\nfor item in items:\n    print('clear', item)\n    items.clear()\nprint(items)"
        ),
        Ok(vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "[1, 2, 3]".to_string(),
            "clear 1".to_string(),
            "[]".to_string(),
        ])
    );
}

#[test]
fn runs_for_loop_over_tuple() {
    assert_eq!(
        run_source("for x in (1, 2, 3):\n    print(x)"),
        Ok(vec!["1".to_string(), "2".to_string(), "3".to_string()])
    );
}

#[test]
fn runs_for_loop_over_naked_tuple() {
    assert_eq!(
        run_source("for x in 1, 2, 3:\n    print(x)"),
        Ok(vec!["1".to_string(), "2".to_string(), "3".to_string()])
    );
}

#[test]
fn runs_for_loop_with_unpack_target() {
    assert_eq!(
        run_source(
            "for a, b in [(1, 2), (3, 4)]:\n    print(a, b)\nfor (c, d) in [(5, 6)]:\n    print(c, d)\nfor [e, f] in [(7, 8)]:\n    print(e, f)"
        ),
        Ok(vec![
            "1 2".to_string(),
            "3 4".to_string(),
            "5 6".to_string(),
            "7 8".to_string(),
        ])
    );
}

#[test]
fn runs_for_loop_over_dict_keys() {
    assert_eq!(
        run_source("items = {\"a\": 1, \"b\": 2}\nfor key in items:\n    print(key)"),
        Ok(vec!["a".to_string(), "b".to_string()])
    );
}

#[test]
fn uses_list_truthiness() {
    assert_eq!(
        run_source(
            "if []:\n    print(\"empty\")\nelse:\n    print(\"empty false\")\nif [1]:\n    print(\"nonempty\")"
        ),
        Ok(vec!["empty false".to_string(), "nonempty".to_string()])
    );
}

#[test]
fn uses_tuple_truthiness() {
    assert_eq!(
        run_source(
            "if ():\n    print(\"empty\")\nelse:\n    print(\"empty false\")\nif (1,):\n    print(\"nonempty\")"
        ),
        Ok(vec!["empty false".to_string(), "nonempty".to_string()])
    );
}

#[test]
fn uses_dict_truthiness() {
    assert_eq!(
        run_source(
            "if {}:\n    print(\"empty\")\nelse:\n    print(\"empty false\")\nif {\"x\": 1}:\n    print(\"nonempty\")"
        ),
        Ok(vec!["empty false".to_string(), "nonempty".to_string()])
    );
}

#[test]
fn uses_custom_truth_protocols() {
    assert_eq!(
        run_source(
            r#"class FalseByBool:
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
    print(error)"#
        ),
        Ok(vec![
            "bool".to_string(),
            "False".to_string(),
            "bool".to_string(),
            "True".to_string(),
            "bool".to_string(),
            "false branch".to_string(),
            "len".to_string(),
            "len true".to_string(),
            "empty".to_string(),
            "False True".to_string(),
            "len".to_string(),
            "empty".to_string(),
            "empty".to_string(),
            "len".to_string(),
            "False True".to_string(),
            "len".to_string(),
            "2".to_string(),
            "__bool__ should return bool, returned int".to_string(),
            "__len__() should return >= 0".to_string(),
        ])
    );
}

#[test]
fn rejects_invalid_custom_truth_protocols() {
    assert_eq!(
        run_source(
            "class BadBool:\n    def __bool__(self):\n        return 1\nprint(bool(BadBool()))"
        ),
        Err("runtime error: TypeError: __bool__ should return bool, returned int".to_string())
    );
    assert_eq!(
        run_source(
            "class BadLen:\n    def __len__(self):\n        return -1\nprint(bool(BadLen()))"
        ),
        Err("runtime error: ValueError: __len__() should return >= 0".to_string())
    );
    assert_eq!(
        run_source(
            "class BadLen:\n    def __len__(self):\n        return \"many\"\nprint(len(BadLen()))"
        ),
        Err(
            "runtime error: TypeError: 'str' object cannot be interpreted as an integer"
                .to_string()
        )
    );
}

#[test]
fn runs_for_loop_over_range_start_stop_step() {
    assert_eq!(
        run_source(
            "for x in range(1, 6, 2):\n    print(x)\nfor y in range(3, 0, -1):\n    print(y)"
        ),
        Ok(vec![
            "1".to_string(),
            "3".to_string(),
            "5".to_string(),
            "3".to_string(),
            "2".to_string(),
            "1".to_string(),
        ])
    );
}

#[test]
fn runs_for_else_after_iterator_finishes() {
    assert_eq!(
        run_source("for x in range(2):\n    print(x)\nelse:\n    print(\"done\")"),
        Ok(vec!["0".to_string(), "1".to_string(), "done".to_string()])
    );
}

#[test]
fn skips_for_else_after_break() {
    assert_eq!(
        run_source("for x in range(3):\n    print(x)\n    break\nelse:\n    print(\"else\")"),
        Ok(vec!["0".to_string()])
    );
}

#[test]
fn runs_continue_inside_for_loop() {
    assert_eq!(
        run_source("for x in range(4):\n    if x == 2:\n        continue\n    print(x)"),
        Ok(vec!["0".to_string(), "1".to_string(), "3".to_string()])
    );
}

#[test]
fn reports_non_iterable_for_source() {
    assert_eq!(
        run_source("for x in 1:\n    print(x)"),
        Err("runtime error: TypeError: 1 is not iterable".to_string())
    );
}

#[test]
fn reports_range_argument_errors() {
    assert_eq!(
        run_source("print(range())"),
        Err("runtime error: range expected at least 1 argument, got 0".to_string())
    );
    assert_eq!(
        run_source("for x in range(1, 3, 0):\n    print(x)"),
        Err("runtime error: range() arg 3 must not be zero".to_string())
    );
}

#[test]
fn reports_subscript_errors() {
    assert_eq!(
        run_source("print(1[0])"),
        Err("runtime error: TypeError: 1 is not subscriptable".to_string())
    );
    assert_eq!(
        run_source("print([1][1])"),
        Err("runtime error: IndexError: list index out of range".to_string())
    );
    assert_eq!(
        run_source("print([1][\"0\"])"),
        Err("runtime error: TypeError: list indices must be integers, got 0".to_string())
    );
    assert_eq!(
        run_source("print([1][::0])"),
        Err("runtime error: ValueError: slice step cannot be zero".to_string())
    );
}

#[test]
fn defines_and_calls_function() {
    assert_eq!(
        run_source("def add(a, b):\n    return a + b\nprint(add(2, 3))"),
        Ok(vec!["5".to_string()])
    );
}

#[test]
fn returns_none_from_function_without_explicit_return() {
    assert_eq!(
        run_source("def f():\n    pass\nprint(f())"),
        Ok(vec!["None".to_string()])
    );
}

#[test]
fn tracks_runtime_frame_line_after_skipped_if_body() {
    assert_eq!(
        run_source(
            r#"import sys
frame = None

def capture():
    global frame
    frame = sys._getframe(1)

def f():
    capture()
    if False:
        pass

f()
print(frame.f_lineno - frame.f_code.co_firstlineno)"#
        ),
        Ok(vec!["2".to_string()])
    );
}

#[test]
fn json_sandbox_subset_excludes_file_apis_and_encoder_decoder_classes() {
    assert_eq!(
        run_source(
            "import json\nfor name in ['load', 'dump', 'JSONDecodeError', 'JSONDecoder', 'JSONEncoder', '__all__']:\n    print(name, hasattr(json, name))\nprint(dir(json))"
        ),
        Ok(output_lines(&[
            "load False",
            "dump False",
            "JSONDecodeError False",
            "JSONDecoder False",
            "JSONEncoder False",
            "__all__ False",
            "['__name__', 'dumps', 'loads']",
        ]))
    );

    assert_eq!(
        run_source(
            "import json\nchecks = [\n    ('object_hook', lambda: json.loads('{}', object_hook=lambda value: value)),\n    ('object_pairs_hook', lambda: json.loads('{}', object_pairs_hook=lambda value: value)),\n    ('parse_float', lambda: json.loads('1.5', parse_float=float)),\n    ('parse_int', lambda: json.loads('1', parse_int=int)),\n    ('parse_constant', lambda: json.loads('NaN', parse_constant=lambda value: value)),\n    ('default', lambda: json.dumps(object(), default=lambda value: None)),\n    ('loads_cls', lambda: json.loads('{}', cls=object)),\n    ('dumps_cls', lambda: json.dumps({'a': 1}, cls=object)),\n]\nfor label, check in checks:\n    try:\n        check()\n    except TypeError as error:\n        print(label, error.__class__.__name__)\n    else:\n        print(label, 'OK')"
        ),
        Ok(output_lines(&[
            "object_hook OK",
            "object_pairs_hook OK",
            "parse_float OK",
            "parse_int OK",
            "parse_constant OK",
            "default OK",
            "loads_cls TypeError",
            "dumps_cls TypeError",
        ]))
    );
}

#[test]
fn json_sandbox_subset_rejects_unpaired_surrogate_escapes() {
    assert_eq!(
        run_source(
            "import json\nfor source in ['\"\\\\ud800\"', '\"\\\\udc00\"', '\"\\\\ud800x\"']:\n    try:\n        json.loads(source)\n    except ValueError as error:\n        print(type(error).__name__, 'Unpaired surrogate escape' in str(error))\nvalue = json.loads('\"\\\\ud800\\\\udc00\"')\nprint(len(value), json.dumps(value))"
        ),
        Ok(output_lines(&[
            "ValueError True",
            "ValueError True",
            "ValueError True",
            "1 \"\\ud800\\udc00\"",
        ]))
    );
}

#[test]
fn reads_runtime_frame_depths() {
    assert_eq!(
        run_source(
            r#"import sys

def outer():
    def inner():
        print(sys._getframe(0).f_code.co_firstlineno)
        print(sys._getframe(1).f_code.co_firstlineno)
    inner()

outer()"#
        ),
        Ok(vec!["4".to_string(), "3".to_string()])
    );
}

#[test]
fn reports_runtime_frame_argument_errors() {
    assert_eq!(
        run_source(
            r#"import sys

for expr in [
    lambda: sys._getframe(-1),
    lambda: sys._getframe(999),
    lambda: sys._getframe("bad"),
    lambda: sys._getframe(x=0),
    lambda: sys._getframe(0, 1),
]:
    try:
        expr()
    except Exception as error:
        print(error)"#
        ),
        Ok(output_lines(&[
            "call stack is not deep enough",
            "call stack is not deep enough",
            "'str' object cannot be interpreted as an integer",
            "_getframe() does not accept keyword arguments",
            "_getframe() expected at most 1 argument, got 2",
        ]))
    );
}

#[test]
fn returns_none_from_bare_return() {
    assert_eq!(
        run_source("def f():\n    return\nprint(f())"),
        Ok(vec!["None".to_string()])
    );
}

#[test]
fn keeps_function_assignments_local() {
    assert_eq!(
        run_source("x = 1\ndef f():\n    x = 2\n    return x\nprint(f(), x)"),
        Ok(vec!["2 1".to_string()])
    );
}

#[test]
fn runs_recursive_function() {
    assert_eq!(
        run_source_with_stack(
            "def fact(n):\n    if n <= 1:\n        return 1\n    return n * fact(n - 1)\nprint(fact(5))",
            8 * 1024 * 1024,
        ),
        Ok(vec!["120".to_string()])
    );
}

#[test]
fn reports_return_outside_function() {
    assert_eq!(
        run_source("return 1"),
        Err("compile error: return outside function".to_string())
    );
}

#[test]
fn reports_function_argument_count_errors() {
    assert_eq!(
        run_source("def add(a, b):\n    return a + b\nprint(add(1))"),
        Err("runtime error: TypeError: add() missing required argument 'b'".to_string())
    );
}

#[test]
fn runs_function_default_parameters() {
    assert_eq!(
        run_source(
            "def greet(name, suffix=\"!\"):\n    return name + suffix\nprint(greet(\"mini\"), greet(\"mini\", \"?\"))"
        ),
        Ok(vec!["mini! mini?".to_string()])
    );
}

#[test]
fn evaluates_function_defaults_at_definition_time() {
    assert_eq!(
        run_source("x = 1\ndef f(a=x):\n    return a\nx = 2\nprint(f(), f(3))"),
        Ok(vec!["1 3".to_string()])
    );
}

#[test]
fn runs_function_keyword_arguments() {
    assert_eq!(
        run_source(
            "def sub(a, b):\n    return a - b\nprint(sub(b=2, a=5))\ndef total(a, b=2, c=3):\n    return a + b + c\nprint(total(1), total(1, c=4), total(a=1, b=2, c=3))"
        ),
        Ok(vec!["3".to_string(), "6 7 6".to_string()])
    );
}

#[test]
fn runs_function_decorators() {
    assert_eq!(
        run_source(
            "def callnum(num):\n    def deco(func):\n        return lambda: num\n    return deco\n@callnum(2)\n@callnum(1)\ndef foo():\n    return 42\nprint(foo())"
        ),
        Ok(vec!["2".to_string()])
    );
    assert_eq!(
        run_source(
            "actions = \"\"\ndef make(tag):\n    global actions\n    actions = actions + \"make\" + tag\n    def decorate(func):\n        global actions\n        actions = actions + \"call\" + tag\n        return func\n    return decorate\n@make(\"1\")\n@make(\"2\")\ndef foo():\n    return 42\nprint(foo(), actions)"
        ),
        Ok(vec!["42 make1make2call2call1".to_string()])
    );
}

#[test]
fn runs_expression_decorators() {
    assert_eq!(
        run_source(
            "def identity(value):\n    return value\n@False or identity\ndef f():\n    return 3\n@lambda func: func\ndef g():\n    return 4\n@d := identity\ndef h():\n    return 5\nprint(f(), g(), d.__name__, h())"
        ),
        Ok(vec!["3 4 identity 5".to_string()])
    );
}

#[test]
fn runs_class_decorators() {
    assert_eq!(
        run_source(
            "def identity(cls):\n    print(cls.__name__)\n    return cls\n@identity\nclass C:\n    pass\nprint(C.__name__)"
        ),
        Ok(vec!["C".to_string(), "C".to_string()])
    );
}

#[test]
fn reports_function_keyword_argument_errors() {
    assert_eq!(
        run_source("def f(a):\n    return a\nprint(f(1, a=2))"),
        Err("runtime error: TypeError: f() got multiple values for argument 'a'".to_string())
    );
    assert_eq!(
        run_source("def f(a):\n    return a\nprint(f(b=1))"),
        Err("runtime error: TypeError: f() got an unexpected keyword argument 'b'".to_string())
    );
    assert_eq!(
        run_source("def f(a):\n    return a\nprint(f())"),
        Err("runtime error: TypeError: f() missing required argument 'a'".to_string())
    );
}

#[test]
fn rejects_invalid_function_parameter_and_argument_order() {
    assert_eq!(
        run_source("def f(a=1, b):\n    return a"),
        Err(
            "parse error: parameter without a default follows parameter with a default".to_string()
        )
    );
    assert_eq!(
        run_source("def f(a):\n    return a\nprint(f(a=1, 2))"),
        Err("parse error: positional argument follows keyword argument".to_string())
    );
}

#[test]
fn runs_varargs_functions() {
    assert_eq!(
        run_source("def collect(a, *items):\n    print(a, items)\ncollect(1)\ncollect(1, 2, 3)"),
        Ok(vec!["1 ()".to_string(), "1 (2, 3)".to_string()])
    );
}

#[test]
fn runs_keyword_only_parameters() {
    assert_eq!(
        run_source("def f(a, *, b, c=3):\n    return a + b + c\nprint(f(1, b=2), f(1, b=2, c=4))"),
        Ok(vec!["6 7".to_string()])
    );
}

#[test]
fn runs_positional_only_parameters() {
    assert_eq!(
        run_source(
            "def f(a, b=10, /, c=100):\n    return a + b + c\nprint(f(1, 2, 3), f(1, 2, c=3), f(1, 2), f(1, c=2))"
        ),
        Ok(vec!["6 6 103 13".to_string()])
    );
    assert_eq!(
        run_source("def f(a, /, b):\n    return a + b\nprint(f(1, b=2))"),
        Ok(vec!["3".to_string()])
    );
}

#[test]
fn runs_positional_only_lambdas() {
    assert_eq!(
        run_source("x = lambda a, /, b=2: a + b\nprint(x(1), x(1, b=3))"),
        Ok(vec!["3 4".to_string()])
    );
}

#[test]
fn allows_positional_only_name_in_kwargs() {
    assert_eq!(
        run_source("def f(a, /, **kwargs):\n    print(a, kwargs)\nf(42, a=99)"),
        Ok(vec!["42 {'a': 99}".to_string()])
    );
}

#[test]
fn runs_kwargs_functions() {
    assert_eq!(
        run_source(
            "def collect(**kwargs):\n    print(kwargs)\ncollect(a=1, b=2)\ndef both(a, *items, **kwargs):\n    print(a, items, kwargs)\nboth(1, 2, 3, x=4)"
        ),
        Ok(vec![
            "{'a': 1, 'b': 2}".to_string(),
            "1 (2, 3) {'x': 4}".to_string(),
        ])
    );
}

#[test]
fn reports_keyword_only_argument_errors() {
    assert_eq!(
        run_source("def f(*, a):\n    return a\nprint(f())"),
        Err("runtime error: TypeError: f() missing required argument 'a'".to_string())
    );
    assert_eq!(
        run_source("def f(*, a):\n    return a\nprint(f(1))"),
        Err(
            "runtime error: TypeError: f() expected at most 0 positional arguments, got 1"
                .to_string()
        )
    );
}

#[test]
fn reports_positional_only_argument_errors() {
    assert_eq!(
        run_source("def f(a, /):\n    return a\nprint(f(a=1))"),
        Err(
            "runtime error: TypeError: f() got positional-only arguments passed as keyword arguments: 'a'"
                .to_string()
        )
    );
    assert_eq!(
        run_source("def f(a, b, /):\n    return a + b\nprint(f(a=1, b=2))"),
        Err(
            "runtime error: TypeError: f() got positional-only arguments passed as keyword arguments: 'a, b'"
                .to_string()
        )
    );
    assert_eq!(
        run_source("def f(a, /, **kwargs):\n    return a\nprint(f(a=1))"),
        Err("runtime error: TypeError: f() missing required argument 'a'".to_string())
    );
}

#[test]
fn rejects_invalid_star_parameter_forms() {
    assert_eq!(
        run_source("def f(*):\n    pass"),
        Err("parse error: named parameters must follow bare *".to_string())
    );
    assert_eq!(
        run_source("def f(**kwargs, a):\n    pass"),
        Err("parse error: parameters cannot follow var-keyword parameter".to_string())
    );
    assert_eq!(
        run_source("def f(*, **kwargs):\n    pass"),
        Err("parse error: named parameters must follow bare *".to_string())
    );
}

#[test]
fn rejects_invalid_positional_only_parameter_forms() {
    assert_eq!(
        run_source("def f(/):\n    pass"),
        Err("parse error: at least one parameter must precede /".to_string())
    );
    assert_eq!(
        run_source("def f(a, /, b, /):\n    pass"),
        Err("parse error: / may appear only once".to_string())
    );
    assert_eq!(
        run_source("def f(*args, /):\n    pass"),
        Err("parse error: / must be ahead of *".to_string())
    );
}

#[test]
fn runs_starred_call_arguments() {
    assert_eq!(
        run_source(
            "def add(a, b):\n    return a + b\nargs = [2, 3]\nprint(add(*args))\nprint(add(*[1], 2))\nprint(add(b=4, *[1]))"
        ),
        Ok(vec!["5".to_string(), "3".to_string(), "5".to_string()])
    );
}

#[test]
fn runs_double_starred_call_arguments() {
    assert_eq!(
        run_source(
            "def add(a, b):\n    return a + b\nkwargs = {\"a\": 2, \"b\": 3}\nprint(add(**kwargs))\ndef f(*, a, b=2):\n    return a + b\nprint(f(**{\"a\": 4}))"
        ),
        Ok(vec!["5".to_string(), "6".to_string()])
    );
}

#[test]
fn runs_mixed_call_unpacking() {
    assert_eq!(
        run_source(
            "def collect(a, *items, **kwargs):\n    print(a, items, kwargs)\ndata = [1, 2, 3]\nextra = {\"x\": 4}\ncollect(*data, **extra)\nprint(*[\"mini\", \"python\"])"
        ),
        Ok(vec![
            "1 (2, 3) {'x': 4}".to_string(),
            "mini python".to_string(),
        ])
    );
}

#[test]
fn reports_call_unpacking_errors() {
    assert_eq!(
        run_source(
            "def f(**kwargs):\n    return kwargs\ntry:\n    f(**1)\nexcept TypeError as error:\n    print(type(error).__name__, error)"
        ),
        Ok(vec![
            "TypeError ** argument must be a dict, got 1".to_string()
        ])
    );
    assert_eq!(
        run_source(
            "def f(**kwargs):\n    return kwargs\ntry:\n    f(**{1: 2})\nexcept TypeError as error:\n    print(type(error).__name__, error)"
        ),
        Ok(vec![
            "TypeError ** argument keys must be strings, got 1".to_string()
        ])
    );
    assert_eq!(
        run_source("def f(**kwargs):\n    return kwargs\nprint(f(a=1, **{\"a\": 2}))"),
        Err(
            "runtime error: TypeError: f() got multiple values for keyword argument 'a'"
                .to_string()
        )
    );
    assert_eq!(
        run_source("def f(*args):\n    return args\nprint(f(**{}, *[1]))"),
        Err(
            "parse error: iterable argument unpacking follows keyword argument unpacking"
                .to_string()
        )
    );
}

#[test]
fn reads_global_name_from_function() {
    assert_eq!(
        run_source("x = 1\ndef f():\n    return x\nprint(f())"),
        Ok(vec!["1".to_string()])
    );
}

#[test]
fn writes_global_name_from_function() {
    assert_eq!(
        run_source("x = 1\ndef f():\n    global x\n    x = 2\nf()\nprint(x)"),
        Ok(vec!["2".to_string()])
    );
}

#[test]
fn augassigns_global_name_from_function() {
    assert_eq!(
        run_source("x = 1\ndef inc():\n    global x\n    x += 1\ninc()\ninc()\nprint(x)"),
        Ok(vec!["3".to_string()])
    );
}

#[test]
fn global_statement_is_noop_at_module_level() {
    assert_eq!(
        run_source("global x\nx = 4\nprint(x)"),
        Ok(vec!["4".to_string()])
    );
}

#[test]
fn reads_closure_name_from_nested_function() {
    assert_eq!(
        run_source(
            "def outer():\n    x = 1\n    def inner():\n        return x\n    return inner()\nprint(outer())"
        ),
        Ok(vec!["1".to_string()])
    );
}

#[test]
fn keeps_closure_alive_after_outer_returns() {
    assert_eq!(
        run_source(
            "def make_value():\n    x = 7\n    def get():\n        return x\n    return get\nget = make_value()\nprint(get())"
        ),
        Ok(vec!["7".to_string()])
    );
}

#[test]
fn writes_nonlocal_name_from_nested_function() {
    assert_eq!(
        run_source(
            "def make_counter():\n    count = 0\n    def inc():\n        nonlocal count\n        count += 1\n        return count\n    return inc\ncounter = make_counter()\nprint(counter(), counter())"
        ),
        Ok(vec!["1 2".to_string()])
    );
}

#[test]
fn nonlocal_writes_nearest_enclosing_scope() {
    assert_eq!(
        run_source(
            "def outer():\n    x = \"outer\"\n    def middle():\n        x = \"middle\"\n        def inner():\n            nonlocal x\n            x = \"changed\"\n        inner()\n        return x\n    y = middle()\n    return x + \" \" + y\nprint(outer())"
        ),
        Ok(vec!["outer changed".to_string()])
    );
}

#[test]
fn reports_nonlocal_errors() {
    assert_eq!(
        run_source("nonlocal x"),
        Err("compile error: nonlocal declaration not allowed at module level".to_string())
    );
    assert_eq!(
        run_source("def f():\n    nonlocal missing\n    missing = 1\nf()"),
        Err("compile error: no binding for nonlocal 'missing' found".to_string())
    );
}

#[test]
fn runs_lambda_expression() {
    assert_eq!(
        run_source("inc = lambda x: x + 1\nprint(inc(2))\nprint((lambda: 5)())"),
        Ok(vec!["3".to_string(), "5".to_string()])
    );
}

#[test]
fn runs_lambda_defaults_keywords_and_starred_parameters() {
    assert_eq!(
        run_source(
            "f = lambda x, y=2: x + y\nprint(f(1), f(1, 3), f(y=4, x=1))\ncollect = lambda *items, **kwargs: print(items, kwargs)\ncollect(1, 2, x=3)"
        ),
        Ok(vec!["3 4 5".to_string(), "(1, 2) {'x': 3}".to_string()])
    );
}

#[test]
fn lambda_captures_closure() {
    assert_eq!(
        run_source(
            "def make_adder(n):\n    return lambda x: x + n\nadd2 = make_adder(2)\nprint(add2(3))"
        ),
        Ok(vec!["5".to_string()])
    );
}

#[test]
fn runs_assert_statement() {
    assert_eq!(
        run_source("assert True\nassert 1\nprint(\"ok\")"),
        Ok(vec!["ok".to_string()])
    );
}

#[test]
fn reports_assert_errors() {
    assert_eq!(
        run_source("assert False"),
        Err("runtime error: AssertionError".to_string())
    );
    assert_eq!(
        run_source("assert False, \"bad\""),
        Err("runtime error: AssertionError: bad".to_string())
    );
}

#[test]
fn defines_and_instantiates_class() {
    assert_eq!(
        run_source("class Box:\n    pass\nbox = Box()\nbox.value = 3\nprint(box.value)"),
        Ok(vec!["3".to_string()])
    );
}

#[test]
fn runs_class_attributes() {
    assert_eq!(
        run_source(
            "class Thing:\n    kind = \"thing\"\nprint(Thing.__name__, Thing.kind)\nThing.kind = \"changed\"\nprint(Thing.kind)"
        ),
        Ok(vec!["Thing thing".to_string(), "changed".to_string()])
    );
}

#[test]
fn runs_instance_methods() {
    assert_eq!(
        run_source(
            "class Counter:\n    def __init__(self, start):\n        self.value = start\n    def inc(self):\n        self.value += 1\n        return self.value\nc = Counter(2)\nprint(c.value, c.inc(), c.value)"
        ),
        Ok(vec!["2 3 3".to_string()])
    );
}

#[test]
fn runs_class_inheritance_and_header_arguments() {
    assert_eq!(
        run_source(
            "class Base:\n    kind = \"base\"\n    def __init__(self, value):\n        self.value = value\n    def describe(self):\n        return self.kind + \":\" + self.value\nclass Child(Base):\n    kind = \"child\"\nc = Child(\"ok\")\nprint(Child.__bases__[0].__name__, c.describe(), c.value)"
        ),
        Ok(vec!["Base child:ok ok".to_string()])
    );
    assert_eq!(
        run_source(
            "class Base:\n    label = \"base\"\nbases = (Base,)\nkwargs = {\"metaclass\": type}\nclass FromUnpack(*bases, **kwargs):\n    pass\nprint(FromUnpack.__bases__[0].__name__, FromUnpack.label)"
        ),
        Ok(vec!["Base base".to_string()])
    );
    assert_eq!(
        run_source("class Broken(1):\n    pass"),
        Err("runtime error: class base must be a class, got 1".to_string())
    );
}

#[test]
fn runs_function_annotations() {
    assert_eq!(
        run_source(
            "def f(x: int, y: str = \"a\", *args: bool, z: float = 1, **kwargs: list) -> str:\n    return y\nprint(f.__annotations__[\"x\"].__name__, f.__annotations__[\"y\"].__name__, f.__annotations__[\"args\"].__name__, f.__annotations__[\"z\"].__name__, f.__annotations__[\"kwargs\"].__name__, f.__annotations__[\"return\"].__name__)"
        ),
        Ok(vec!["int str bool float list str".to_string()])
    );
}

#[test]
fn runs_variable_annotations() {
    assert_eq!(
        run_source(
            "x: int\ny: str = \"ok\"\nprint(__annotations__[\"x\"].__name__, __annotations__[\"y\"].__name__, y)"
        ),
        Ok(vec!["int str ok".to_string()])
    );
}

#[test]
fn runs_class_annotations() {
    assert_eq!(
        run_source(
            "class C:\n    x: int\n    y: str = \"value\"\nprint(C.__annotations__[\"x\"].__name__, C.__annotations__[\"y\"].__name__, C.y)"
        ),
        Ok(vec!["int str value".to_string()])
    );
}

#[test]
fn runs_function_class_and_alias_type_params() {
    assert_eq!(
        run_source(
            "def f[T: int = str](x: T) -> T:\n    return x\nclass Box[T: int = str]:\n    pass\ntype Alias[T: int = str, *Ts = list, **P = bool] = int\nprint(f.__type_params__[0].__name__, f.__type_params__[0].__kind__, f.__type_params__[0].__bound__.__name__, f.__type_params__[0].__default__.__name__)\nprint(f.__annotations__[\"x\"].__name__, f.__annotations__[\"return\"].__name__)\nprint(Box.__type_params__[0].__name__, Box.__type_params__[0].__bound__.__name__)\nprint(Alias.__name__, Alias.__value__.__name__)\nprint(Alias.__type_params__[1].__name__, Alias.__type_params__[1].__kind__, Alias.__type_params__[1].__default__.__name__)\nprint(Alias.__type_params__[2].__name__, Alias.__type_params__[2].__kind__, Alias.__type_params__[2].__default__.__name__)"
        ),
        Ok(vec![
            "T TypeVar int str".to_string(),
            "T T".to_string(),
            "T int".to_string(),
            "Alias int".to_string(),
            "Ts TypeVarTuple list".to_string(),
            "P ParamSpec bool".to_string(),
        ])
    );
}

#[test]
fn runs_generic_alias_type_subscripts() {
    assert_eq!(
        run_source(
            "print(list[int].__origin__.__name__, list[int].__args__[0].__name__)\nprint(dict[str, int].__origin__.__name__, dict[str, int].__args__[0].__name__, dict[str, int].__args__[1].__name__)"
        ),
        Ok(vec!["list int".to_string(), "dict str int".to_string()])
    );
    assert_eq!(
        run_source(
            "class Base[T]:\n    label = \"base\"\nclass Child[T](Base[T]):\n    pass\nprint(Child.__type_params__[0].__name__, Child.__bases__[0].__name__, Child.label)"
        ),
        Ok(vec!["T Base base".to_string()])
    );
    assert_eq!(
        run_source(
            "class Meta(type):\n    pass\nclass Box[T](metaclass=Meta):\n    pass\nprint(type(Box) is Meta, Box.__class__ is Meta, Box.__type_params__[0].__name__)"
        ),
        Ok(vec!["True True T".to_string()])
    );
    assert_eq!(
        run_source(
            "type Alias[T] = list[T]\nprint(Alias.__value__.__origin__.__name__, Alias.__value__.__args__[0].__name__)"
        ),
        Ok(vec!["list T".to_string()])
    );
}

#[test]
fn rejects_duplicate_type_params() {
    assert_eq!(
        run_source("def f[T, T]():\n    pass"),
        Err("parse error: duplicate type parameter name: T".to_string())
    );
    assert_eq!(
        run_source("class C[T, *T]:\n    pass"),
        Err("parse error: duplicate type parameter name: T".to_string())
    );
    assert_eq!(
        run_source("type Alias[*T, **T] = int"),
        Err("parse error: duplicate type parameter name: T".to_string())
    );
}

#[test]
fn rejects_invalid_expressions_in_type_scopes() {
    assert_eq!(
        run_source("def f[T: (x := 1)]():\n    pass"),
        Err("parse error: named expression cannot be used within a TypeVar bound".to_string())
    );
    assert_eq!(
        run_source("type Alias = (x := int)"),
        Err("parse error: named expression cannot be used within a type alias".to_string())
    );
    assert_eq!(
        run_source("class C[T]((yield 1)):\n    pass"),
        Err(
            "parse error: yield expression cannot be used within the definition of a generic"
                .to_string()
        )
    );
}

#[test]
fn reports_attribute_errors() {
    assert_eq!(
        run_source("print((1).value)"),
        Err("runtime error: AttributeError: 1 has no attribute 'value'".to_string())
    );
    assert_eq!(
        run_source("x = 1\nx.value = 2"),
        Err("runtime error: AttributeError: cannot set attribute 'value' on 1".to_string())
    );
}

#[test]
fn runs_attribute_introspection_builtins() {
    assert_eq!(
        run_source(
            "import sys\nprint(callable(len), callable('a'), callable(callable))\ndef f():\n    pass\nprint(callable(f))\nclass C1:\n    def meth(self):\n        pass\nc = C1()\nprint(callable(C1), callable(c.meth), callable(c))\nc.__call__ = lambda self: 0\nprint(callable(c))\nclass C2:\n    def __call__(self, value):\n        return value + 1\nc2 = C2()\nprint(callable(c2), c2(4))\nc2.__call__ = None\nprint(callable(c2), c2(5))\nsetattr(sys, 'spam', 1)\nprint(getattr(sys, 'spam'), hasattr(sys, 'spam'))\ndelattr(sys, 'spam')\nprint(hasattr(sys, 'spam'), getattr(sys, 'spam', 'missing'))\nclass Box:\n    pass\nbox = Box()\nsetattr(box, 'value', 3)\nprint(getattr(box, 'value'), hasattr(box, 'value'))\nsetattr(Box, 'label', 'box')\nprint(getattr(box, 'label'), getattr(Box, 'label'))\ndelattr(box, 'value')\nprint(hasattr(box, 'value'), getattr(box, 'value', 42))\ntry:\n    print((1).missing)\nexcept AttributeError:\n    print('caught')"
        ),
        Ok(vec![
            "True False True".to_string(),
            "True".to_string(),
            "True True False".to_string(),
            "False".to_string(),
            "True 5".to_string(),
            "True 6".to_string(),
            "1 True".to_string(),
            "False missing".to_string(),
            "3 True".to_string(),
            "box box".to_string(),
            "False 42".to_string(),
            "caught".to_string(),
        ])
    );

    assert!(run_source("getattr()").is_err());
    assert!(run_source("getattr(1)").is_err());
    assert!(run_source("getattr(1, 2)").is_err());
    assert!(run_source("hasattr()").is_err());
    assert!(run_source("hasattr(1)").is_err());
    assert!(run_source("hasattr(1, 2)").is_err());
    assert!(run_source("setattr()").is_err());
    assert!(run_source("setattr(1, 'x')").is_err());
    assert!(run_source("setattr(1, 'x', 2)").is_err());
    assert!(run_source("delattr()").is_err());
    assert!(run_source("delattr(1)").is_err());
    assert!(run_source("delattr(1, 2)").is_err());
    assert!(run_source("callable()").is_err());
    assert!(run_source("callable(1, 2)").is_err());
}

#[test]
fn runs_instance_attribute_hooks() {
    assert_eq!(
        run_source(
            r#"class Hook:
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
print(c.value, c.missing)"#
        ),
        Ok(vec![
            "fallback fallback True False".to_string(),
            "4".to_string(),
            "5".to_string(),
            "False 42".to_string(),
            "set blocked".to_string(),
            "9".to_string(),
            "delete blocked".to_string(),
            "False".to_string(),
            "boom".to_string(),
            "11 fallback".to_string(),
        ])
    );
}

#[test]
fn runs_instance_getattribute_hook() {
    assert_eq!(
        run_source(
            r#"class A:
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
    print(error)"#
        ),
        Ok(vec![
            "24 43".to_string(),
            "24 hello".to_string(),
            "42".to_string(),
            "hello 43".to_string(),
            "missing".to_string(),
            "1 2 3 False".to_string(),
            "object missing".to_string(),
            "boom".to_string(),
        ])
    );
}

#[test]
fn runs_property_descriptor() {
    assert_eq!(
        run_source(
            r#"class C:
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
print(Fallback().z)"#
        ),
        Ok(vec![
            "x x x True".to_string(),
            "1".to_string(),
            "5 5".to_string(),
            "7 7".to_string(),
            "False False".to_string(),
            "missing".to_string(),
            "1".to_string(),
            "readonly".to_string(),
            "nodelete".to_string(),
            "fallback".to_string(),
        ])
    );
}

#[test]
fn runs_custom_descriptor_protocol() {
    assert_eq!(
        run_source(
            r#"class NonData:
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
print(child.dd)"#
        ),
        Ok(vec![
            "nondata C".to_string(),
            "field class nondata C".to_string(),
            "5 5 class data C".to_string(),
            "-1".to_string(),
            "True".to_string(),
            "delete-only set blocked".to_string(),
            "True".to_string(),
            "8".to_string(),
            "set-only delete blocked".to_string(),
            "nondata Child".to_string(),
            "11".to_string(),
        ])
    );
}

#[test]
fn runs_staticmethod_and_classmethod_descriptors() {
    assert_eq!(
        run_source(
            r#"class C:
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
print(cm.__func__(C, 7), cm.__get__(None, C)(8), cm.__get__(inst, None)(9), isinstance(cm, classmethod))"#
        ),
        Ok(vec![
            "2 3 s s".to_string(),
            "C3 C4 D5 D6".to_string(),
            "False False".to_string(),
            "11 12 13 True".to_string(),
            "C7 C8 C9 True".to_string(),
        ])
    );
}

#[test]
fn runs_explicit_super_descriptor_lookup() {
    assert_eq!(
        run_source(
            r#"class Base:
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
print(super(Child, c).__thisclass__.__name__, super(Child, c).__self__ is c, isinstance(super(Child, c), super))"#
        ),
        Ok(vec![
            "Base:n:Child".to_string(),
            "Child:child Child:child".to_string(),
            "4 5".to_string(),
            "Base:n".to_string(),
            "Child True True".to_string(),
        ])
    );
}

#[test]
fn runs_zero_arg_super_descriptor_lookup() {
    assert_eq!(
        run_source(
            r#"class Base:
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
print(c.value)"#
        ),
        Ok(vec![
            "Base:n:Child:Child".to_string(),
            "Child:child:Child Child:child:Child".to_string(),
            "base:n:child".to_string(),
        ])
    );
}

#[test]
fn runs_c3_mro_for_multiple_inheritance() {
    assert_eq!(
        run_source_with_stack(
            r#"class A:
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
            8 * 1024 * 1024,
        ),
        Ok(vec!["DBCA".to_string()])
    );

    assert!(
        run_source_with_stack(
            "class A:\n    pass\nclass B(A):\n    pass\nclass C(A):\n    pass\nclass D(B, C):\n    pass\nclass E(C, B):\n    pass\nclass F(D, E):\n    pass",
            8 * 1024 * 1024,
        )
        .is_err()
    );
    assert!(
        run_source_with_stack(
            "class A:\n    pass\nclass B(A, A):\n    pass",
            8 * 1024 * 1024
        )
        .is_err()
    );
}

#[test]
fn runs_slots_attribute_restrictions() {
    assert_eq!(
        run_source(
            r#"class Point:
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
print(Point.x.__get__(p, None))
for label, expected, expr in [
    ('missing', ' expected at least 1 argument, got 0', lambda: Point.x.__get__()),
    ('too-many', ' expected at most 2 arguments, got 3', lambda: Point.x.__get__(p, Point, 1)),
    ('keyword', 'wrapper __get__() takes no keyword arguments', lambda: Point.x.__get__(obj=p, type=Point)),
    ('bad-keyword', 'wrapper __get__() takes no keyword arguments', lambda: Point.x.__get__(bad=1)),
    ('none-only', '__get__(None, None) is invalid', lambda: Point.x.__get__(None)),
    ('none-none', '__get__(None, None) is invalid', lambda: Point.x.__get__(None, None)),
]:
    try:
        expr()
    except TypeError as error:
        print(label, error.__class__.__name__, str(error), str(error) == expected)
for label, expected, expr in [
    ('set-missing', ' expected 2 arguments, got 0', lambda: Point.x.__set__()),
    ('set-one', ' expected 2 arguments, got 1', lambda: Point.x.__set__(p)),
    ('set-too-many', ' expected 2 arguments, got 3', lambda: Point.x.__set__(p, 1, 2)),
    ('set-keyword', 'wrapper __set__() takes no keyword arguments', lambda: Point.x.__set__(obj=p, value=1)),
    ('set-bad-keyword', 'wrapper __set__() takes no keyword arguments', lambda: Point.x.__set__(bad=1)),
    ('delete-missing', 'expected 1 argument, got 0', lambda: Point.x.__delete__()),
    ('delete-too-many', 'expected 1 argument, got 2', lambda: Point.x.__delete__(p, 1)),
    ('delete-keyword', 'wrapper __delete__() takes no keyword arguments', lambda: Point.x.__delete__(obj=p)),
    ('delete-bad-keyword', 'wrapper __delete__() takes no keyword arguments', lambda: Point.x.__delete__(bad=1)),
]:
    try:
        expr()
    except TypeError as error:
        print(label, error.__class__.__name__, str(error), str(error) == expected)
Point.x.__delete__(p)
try:
    Point.x.__get__(p, Point)
except AttributeError:
    print('descriptor deleted')
try:
    Point.x.__set__(object(), 1)
except TypeError:
    print('descriptor type checked')
try:
    Point.x.__delete__(object())
except TypeError:
    print('descriptor delete type checked')

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

try:
    class BadItem:
        __slots__ = (1,)
except TypeError:
    print('bad slot item')
try:
    class BadName:
        __slots__ = ('not valid',)
except TypeError:
    print('bad slot name')
try:
    class BadDict:
        __slots__ = ('__dict__', '__dict__')
except TypeError:
    print('bad dict slot')
try:
    class Conflict:
        __slots__ = ('x',)
        x = 1
except ValueError:
    print('slot conflict')
class DuplicateSlot:
    __slots__ = ('x', 'x')
d = DuplicateSlot()
d.x = 11
print(d.x)"#
        ),
        Ok(vec![
            "2 3 5".to_string(),
            "8".to_string(),
            "slot blocked".to_string(),
            "slot missing".to_string(),
            "x None True".to_string(),
            "<member 'x' of 'Point' objects>".to_string(),
            "7 7".to_string(),
            "7".to_string(),
            "missing TypeError  expected at least 1 argument, got 0 True".to_string(),
            "too-many TypeError  expected at most 2 arguments, got 3 True".to_string(),
            "keyword TypeError wrapper __get__() takes no keyword arguments True".to_string(),
            "bad-keyword TypeError wrapper __get__() takes no keyword arguments True".to_string(),
            "none-only TypeError __get__(None, None) is invalid True".to_string(),
            "none-none TypeError __get__(None, None) is invalid True".to_string(),
            "set-missing TypeError  expected 2 arguments, got 0 True".to_string(),
            "set-one TypeError  expected 2 arguments, got 1 True".to_string(),
            "set-too-many TypeError  expected 2 arguments, got 3 True".to_string(),
            "set-keyword TypeError wrapper __set__() takes no keyword arguments True".to_string(),
            "set-bad-keyword TypeError wrapper __set__() takes no keyword arguments True"
                .to_string(),
            "delete-missing TypeError expected 1 argument, got 0 True".to_string(),
            "delete-too-many TypeError expected 1 argument, got 2 True".to_string(),
            "delete-keyword TypeError wrapper __delete__() takes no keyword arguments True"
                .to_string(),
            "delete-bad-keyword TypeError wrapper __delete__() takes no keyword arguments True"
                .to_string(),
            "descriptor deleted".to_string(),
            "descriptor type checked".to_string(),
            "descriptor delete type checked".to_string(),
            "mini".to_string(),
            "string slot blocked".to_string(),
            "1 2".to_string(),
            "inherited slots blocked".to_string(),
            "1 2".to_string(),
            "4 5".to_string(),
            "7 8".to_string(),
            "bad slot item".to_string(),
            "bad slot name".to_string(),
            "bad dict slot".to_string(),
            "slot conflict".to_string(),
            "11".to_string(),
        ])
    );
}

#[test]
fn skips_while_loop_when_condition_is_false() {
    assert_eq!(
        run_source("while False:\n    print(\"body\")\nprint(\"after\")"),
        Ok(vec!["after".to_string()])
    );
}

#[test]
fn runs_break_inside_while_loop() {
    assert_eq!(
        run_source(
            "while True:\n    print(\"body\")\n    break\n    print(\"skip\")\nprint(\"after\")"
        ),
        Ok(vec!["body".to_string(), "after".to_string()])
    );
}

#[test]
fn runs_continue_inside_while_loop() {
    assert_eq!(
        run_source(
            "x = 0\nwhile x < 3:\n    x = x + 1\n    if x == 2:\n        continue\n    print(x)"
        ),
        Ok(vec!["1".to_string(), "3".to_string()])
    );
}

#[test]
fn runs_while_else_after_condition_finishes_loop() {
    assert_eq!(
        run_source("x = 0\nwhile x < 2:\n    print(x)\n    x = x + 1\nelse:\n    print(\"done\")"),
        Ok(vec!["0".to_string(), "1".to_string(), "done".to_string()])
    );
}

#[test]
fn skips_while_else_after_break() {
    assert_eq!(
        run_source("while True:\n    print(\"body\")\n    break\nelse:\n    print(\"else\")"),
        Ok(vec!["body".to_string()])
    );
}

#[test]
fn reports_break_and_continue_outside_loop() {
    assert_eq!(
        run_source("break"),
        Err("compile error: break outside loop".to_string())
    );
    assert_eq!(
        run_source("continue"),
        Err("compile error: 'continue' not properly in loop".to_string())
    );
}
