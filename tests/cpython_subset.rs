use minipython::run_source;

fn assert_output(source: &str, expected: &[&str]) {
    let expected = expected.iter().map(|line| line.to_string()).collect();
    assert_eq!(run_source(source), Ok(expected), "source:\n{source}");
}

fn assert_error(source: &str, expected: &str) {
    assert_eq!(
        run_source(source),
        Err(expected.to_string()),
        "source:\n{source}"
    );
}

// Adapted from CPython's tokenizer basic operator coverage for `1 + 1` in
// Lib/test/test_tokenize.py and single-statement compile coverage for `1 + 2`
// in Lib/test/test_compile.py. MiniPython checks the same source shape through
// observable output.
#[test]
fn cpython_basic_number_addition_subset() {
    assert_output("print(1 + 1)", &["2"]);
    assert_output("\nprint(1 + 2)\n\n", &["3"]);
    assert_output("x = 1 + 1\nprint(x)", &["2"]);
    assert_output("print(1 + 2 + 3)", &["6"]);
    assert_output("print(1 + (2 + 3))", &["6"]);
}

// Adapted from CPython AST constant coverage in Lib/test/test_ast/test_ast.py.
// MiniPython currently supports int, str, True, and False constants.
#[test]
fn cpython_ast_constant_values_subset() {
    assert_output(
        "print(123, \"unicode\", True, False)",
        &["123 unicode True False"],
    );
}

// Adapted from CPython AST assignment comparison coverage for `x = 10` and
// multi-statement parsing in Lib/test/test_ast/test_ast.py.
#[test]
fn cpython_ast_assignment_and_name_load_subset() {
    assert_output("x = 10\nprint(x)", &["10"]);
    assert_output(
        "x = \"mini\"\ny = \"python\"\nprint(x + y)",
        &["minipython"],
    );
    assert_output("x = 1\nx = x + 2\nprint(x)", &["3"]);
    assert_output("x = False\nx = True\nprint(x)", &["True"]);
}

// Adapted from CPython's AST CLI coverage for `print(1, 2, 3)` in
// Lib/test/test_ast/test_ast.py. We verify the same call/argument semantics
// through MiniPython's observable output instead of CPython's AST dump.
#[test]
fn cpython_ast_print_multiple_arguments() {
    assert_output("print(1, 2, 3)", &["1 2 3"]);
    assert_output("print()", &[""]);
}

// Adapted from CPython AST redundant-parentheses and trailer coverage in
// Lib/test/test_ast/test_ast.py. MiniPython supports grouped expressions and a
// parenthesized callee for calls.
#[test]
fn cpython_ast_redundant_parentheses_and_call_trailer_subset() {
    assert_output("a = 1\nb = 2\nprint(((a + b)))", &["3"]);
    assert_output("(((print)))(4, \"x\")", &["4 x"]);
}

// Adapted from CPython tokenizer and grammar string-literal coverage in
// Lib/test/test_tokenize.py and Lib/test/test_grammar.py. MiniPython currently
// supports double-quoted strings and string `+`.
#[test]
fn cpython_string_literal_and_concat_subset() {
    assert_output("x = \"x\"\nprint(x)", &["x"]);
    assert_output("y = \"ABC\" + \"ABC\"\nprint(y)", &["ABCABC"]);
    assert_output("print(\"mini\" + \"python\")", &["minipython"]);
}

// Adapted from CPython grammar comparison smoke tests in
// Lib/test/test_grammar.py. MiniPython currently supports only `==`, so this
// keeps the equal-comparison cases and checks both expression and if contexts.
#[test]
fn cpython_grammar_equal_comparison_subset() {
    assert_output("x = 1 == 1\nprint(x)", &["True"]);
    assert_output("print(1 == 1, 1 == 2)", &["True False"]);
    assert_output("print(1 == \"1\")", &["False"]);
    assert_output("print(\"x\" == \"x\", True == False)", &["True False"]);
    assert_output("if 1 == 1:\n    print(\"equal\")", &["equal"]);
    assert_output("x = 1\nif x == 1:\n    print(x)", &["1"]);
}

// Adapted from CPython grammar `if_stmt` coverage in
// Lib/test/test_grammar.py. These cases use prints to make branch selection
// observable while keeping CPython's if/else shape.
#[test]
fn cpython_grammar_if_else_subset() {
    assert_output("if True:\n    print(\"then\")", &["then"]);
    assert_output("if False:\n    print(\"then\")", &[]);
    assert_output(
        "if False:\n    print(\"then\")\nelse:\n    print(\"else\")",
        &["else"],
    );
    assert_output(
        "if True:\n    print(\"then\")\nelse:\n    print(\"else\")",
        &["then"],
    );
}

// Adapted from CPython grammar `pass_stmt` and `if_stmt` coverage in
// Lib/test/test_grammar.py. `pass` is a no-op statement.
#[test]
fn cpython_grammar_pass_statement_subset() {
    assert_output("pass\nprint(\"after\")", &["after"]);
    assert_output("if True:\n    pass\nprint(\"after\")", &["after"]);
    assert_output("if False:\n    pass\nelse:\n    print(\"else\")", &["else"]);
}

// Adapted from CPython grammar `suite` coverage in Lib/test/test_grammar.py.
// This checks that indented suites can contain multiple statements and then
// dedent back to the outer program.
#[test]
fn cpython_grammar_suite_and_dedent_subset() {
    assert_output("if True:\n    print(\"a\")\n    print(\"b\")", &["a", "b"]);
    assert_output(
        "if True:\n    print(\"inside\")\nprint(\"after\")",
        &["inside", "after"],
    );
    assert_output(
        "if False:\n    print(\"skip\")\nprint(\"after\")",
        &["after"],
    );
    assert_output("if True:\n    x = \"set\"\nprint(x)", &["set"]);
}

// Adapted from CPython grammar `suite` coverage in Lib/test/test_grammar.py.
// CPython's examples include comment-only lines inside an indented suite.
#[test]
fn cpython_grammar_suite_comments_and_pass_subset() {
    assert_output(
        "if True:\n    #\n    #\n    pass\n    print(\"body\")\n    #",
        &["body"],
    );
}

// Adapted from CPython tokenizer INDENT/DEDENT and blank-line examples in
// Lib/test/test_tokenize.py. This keeps block semantics across blank lines.
#[test]
fn cpython_tokenize_indentation_blank_line_subset() {
    assert_output("if True:\n\n    print(\"body\")", &["body"]);
    assert_output("if True:\r\n    print(\"body\")\r\n\r\n", &["body"]);
    assert_output(
        "if True:\n    print(\"a\")\n\n    print(\"b\")",
        &["a", "b"],
    );
}

// Adapted from CPython tokenizer COMMENT/NL examples in Lib/test/test_tokenize.py.
// MiniPython does not expose comment tokens; it preserves the same executable
// structure by ignoring comments and retaining newlines as statement boundaries.
#[test]
fn cpython_tokenize_comments_subset() {
    assert_output("# leading\nprint(1) # inline\n# trailing", &["1"]);
    assert_output(
        "if True:\n    # NL\n    \n    print(\"body\") # NEWLINE\n",
        &["body"],
    );
}

// Adapted from CPython compile single-statement coverage in
// Lib/test/test_compile.py, which accepts CRLF and multiple physical lines.
#[test]
fn cpython_compile_crlf_newlines_subset() {
    assert_output("print(1)\r\nprint(2)", &["1", "2"]);
}

// Adapted from CPython tokenizer nested INDENT/DEDENT examples in
// Lib/test/test_tokenize.py.
#[test]
fn cpython_tokenize_nested_indentation_subset() {
    assert_output(
        "if True:\n  if True:\n    print(\"nested\")\n  print(\"outer\")\nprint(\"done\")",
        &["nested", "outer", "done"],
    );
}

// Adapted from CPython tokenizer indentation-error coverage in
// Lib/test/test_tokenize.py. The exact exception type/message is MiniPython's,
// but the retained semantic is that a dedent must match an outer indentation
// level.
#[test]
fn cpython_tokenize_unmatched_indentation_subset() {
    assert_error(
        "if True:\n    print(1)\n  print(2)",
        "lex error: unmatched indentation",
    );
}

// Adapted from CPython compile smoke tests for `if x` and `if x else` in
// Lib/test/test_compile.py. MiniPython uses boolean variables because its
// current truthiness intentionally only accepts Bool conditions.
#[test]
fn cpython_compile_if_smoke_subset() {
    assert_output("x = True\nif x:\n    print(\"then\")", &["then"]);
    assert_output(
        "x = False\nif x:\n    print(\"then\")\nelse:\n    print(\"else\")",
        &["else"],
    );
}

// Adapted from CPython compile AST sample `<ifblock>` in
// Lib/test/test_compile.py.
#[test]
fn cpython_compile_pass_ifblock_subset() {
    assert_output("if True:\n    pass", &[]);
}

// Adapted from CPython compile indentation coverage for nested blocks in
// Lib/test/test_compile.py.
#[test]
fn cpython_compile_nested_if_subset() {
    assert_output(
        "if True:\n    if True:\n        print(\"nested\")",
        &["nested"],
    );
    assert_output(
        "x = True\nif x:\n    if False:\n        print(\"skip\")\n    print(\"after\")",
        &["after"],
    );
}
