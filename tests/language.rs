use minipython::run_source;

#[test]
fn prints_number() {
    assert_eq!(run_source("print(123)"), Ok(vec!["123".to_string()]));
}

#[test]
fn prints_addition() {
    assert_eq!(run_source("print(1 + 2)"), Ok(vec!["3".to_string()]));
}

#[test]
fn prints_grouped_addition() {
    assert_eq!(run_source("print(1 + (2 + 3))"), Ok(vec!["6".to_string()]));
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
fn reassigns_variable() {
    assert_eq!(
        run_source("x = 1\nx = x + 2\nprint(x)"),
        Ok(vec!["3".to_string()])
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
fn reports_unknown_name() {
    assert_eq!(
        run_source("unknown(1)"),
        Err("runtime error: unknown name: unknown".to_string())
    );
}

#[test]
fn reports_non_callable_value() {
    assert_eq!(
        run_source("1(2)"),
        Err("runtime error: 1 is not callable".to_string())
    );
}
