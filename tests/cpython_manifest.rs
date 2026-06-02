use std::collections::BTreeMap;
use std::fs;

const MANIFEST: &str = include_str!("cpython_test_manifest.md");
const CPYTHON_TEST_AST_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_ast/test_ast.py";
const CPYTHON_TEST_COMPILE_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_compile.py";

#[derive(Debug)]
struct ManifestGroup<'a> {
    source: &'a str,
    group: &'a str,
    methods: usize,
    status: &'a str,
}

#[derive(Debug)]
struct ManifestMethod<'a> {
    method: &'a str,
    status: &'a str,
}

#[test]
fn cpython_test_manifest_summary_matches_source_groups() {
    let groups = manifest_groups();
    let summary = summary_rows();

    let total_groups: usize = summary.values().map(|(groups, _)| groups).sum();
    let total_methods: usize = summary.values().map(|(_, methods)| methods).sum();

    assert_eq!(groups.len(), total_groups, "summary group total drifted");
    assert_eq!(
        groups.iter().map(|group| group.methods).sum::<usize>(),
        total_methods,
        "summary method total drifted"
    );

    for (status, (summary_groups, summary_methods)) in summary {
        let actual_groups = groups.iter().filter(|group| group.status == status).count();
        let actual_methods = groups
            .iter()
            .filter(|group| group.status == status)
            .map(|group| group.methods)
            .sum::<usize>();
        assert_eq!(
            actual_groups, summary_groups,
            "summary group count drifted for `{status}`"
        );
        assert_eq!(
            actual_methods, summary_methods,
            "summary method count drifted for `{status}`"
        );
    }
}

#[test]
fn cpython_test_manifest_source_totals_match_extracted_baseline() {
    let groups = manifest_groups();
    assert_source_total(&groups, "Lib/test/test_grammar.py", 75);
    assert_source_total(&groups, "Lib/test/test_syntax.py", 55);
    assert_source_total(&groups, "Lib/test/test_compile.py", 186);
    assert_source_total(&groups, "Lib/test/test_ast/test_ast.py", 216);
    assert_source_total(&groups, "Lib/test/test_ast/snippets.py", 0);
}

#[test]
fn cpython_test_manifest_compile_group_counts_match_current_source() {
    let source = fs::read_to_string(CPYTHON_TEST_COMPILE_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_COMPILE_SOURCE}: {error}"));
    let class_counts = python_test_class_method_counts(&source);
    let groups = manifest_groups();

    for group in [
        "TestSpecifics",
        "TestBooleanExpression",
        "TestSourcePositions",
        "TestStaticAttributes",
        "TestExpressionStackSize",
        "TestStackSizeStability",
        "TestInstructionSequence",
    ] {
        let expected = class_counts
            .get(group)
            .copied()
            .unwrap_or_else(|| panic!("missing class `{group}` in {CPYTHON_TEST_COMPILE_SOURCE}"));
        assert_manifest_group_count(&groups, "Lib/test/test_compile.py", group, expected);
    }
}

#[test]
fn cpython_test_manifest_ast_group_counts_match_current_source() {
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
    let class_counts = python_test_class_method_counts(&source);
    let groups = manifest_groups();

    assert_manifest_group_count(
        &groups,
        "Lib/test/test_ast/test_ast.py",
        "module-level `test_*` functions",
        module_level_test_function_count(&source),
    );

    for group in [
        "LazyImportTest",
        "AST_Tests",
        "CopyTests",
        "ASTHelpers_Test",
        "ASTValidatorTests",
        "ConstantTests",
        "EndPositionTests",
        "NodeTransformerTests",
        "ASTConstructorTests",
        "ModuleStateTests",
        "CommandLineTests",
        "ASTOptimizationTests",
    ] {
        let expected = class_counts
            .get(group)
            .copied()
            .unwrap_or_else(|| panic!("missing class `{group}` in {CPYTHON_TEST_AST_SOURCE}"));
        assert_manifest_group_count(&groups, "Lib/test/test_ast/test_ast.py", group, expected);
    }
}

#[test]
fn cpython_test_manifest_statuses_are_well_formed() {
    for group in manifest_groups() {
        assert!(
            matches!(
                group.status,
                "ported"
                    | "partial"
                    | "blocked_by_runtime"
                    | "blocked_by_ast_module"
                    | "blocked_by_cpython_internal"
                    | "not_started"
                    | "source_data"
            ),
            "unknown manifest status for `{}`: `{}`",
            group.source,
            group.status
        );
    }

    for method in token_tests_methods() {
        assert!(
            matches!(method.status, "ported" | "partial"),
            "unknown TokenTests method status for `{}`: `{}`",
            method.method,
            method.status
        );
    }
}

#[test]
fn cpython_test_manifest_token_tests_method_audit_is_complete() {
    let methods = token_tests_methods();

    assert_eq!(
        methods.len(),
        14,
        "TokenTests method audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        14,
        "ported TokenTests method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial TokenTests method count drifted"
    );

    for expected in [
        "test_backslash",
        "test_plain_integers",
        "test_long_integers",
        "test_floats",
        "test_float_exponent_tokenization",
        "test_underscore_literals",
        "test_bad_numerical_literals",
        "test_end_of_numerical_literals",
        "test_string_literals",
        "test_string_prefixes",
        "test_bytes_prefixes",
        "test_ellipsis",
        "test_eof_error",
        "test_max_level",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing TokenTests method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_grammar_tests_method_audit_is_complete() {
    let methods = method_audit_methods("## `Lib/test/test_grammar.py::GrammarTests` Method Audit");

    assert_eq!(
        methods.len(),
        61,
        "GrammarTests method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "partial")),
        "GrammarTests method statuses should be ported or partial"
    );

    for expected in [
        "test_eval_input",
        "test_var_annot_basics",
        "test_var_annot_syntax_errors",
        "test_var_annot_basic_semantics",
        "test_annotations_inheritance",
        "test_var_annot_module_semantics",
        "test_var_annot_in_module",
        "test_var_annot_simple_exec",
        "test_var_annot_rhs",
        "test_funcdef",
        "test_lambdef",
        "test_simple_stmt",
        "test_expr_stmt",
        "test_former_statements_refer_to_builtins",
        "test_del_stmt",
        "test_pass_stmt",
        "test_break_stmt",
        "test_continue_stmt",
        "test_break_continue_loop",
        "test_return",
        "test_control_flow_in_finally",
        "test_yield",
        "test_yield_in_comprehensions",
        "test_raise",
        "test_import",
        "test_global",
        "test_nonlocal",
        "test_assert",
        "test_assert_failures",
        "test_assert_syntax_warnings",
        "test_assert_warning_promotes_to_syntax_error",
        "test_if",
        "test_while",
        "test_for",
        "test_try",
        "test_try_star",
        "test_suite",
        "test_test",
        "test_comparison",
        "test_comparison_is_literal",
        "test_warn_missed_comma",
        "test_binary_mask_ops",
        "test_shift_ops",
        "test_additive_ops",
        "test_multiplicative_ops",
        "test_unary_ops",
        "test_selectors",
        "test_atoms",
        "test_classdef",
        "test_dictcomps",
        "test_listcomps",
        "test_genexps",
        "test_comprehension_specials",
        "test_with_statement",
        "test_if_else_expr",
        "test_paren_evaluation",
        "test_matrix_mul",
        "test_async_await",
        "test_async_for",
        "test_async_with",
        "test_complex_lambda",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing GrammarTests method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_syntax_warning_method_audit_is_complete() {
    let methods =
        method_audit_methods("## `Lib/test/test_syntax.py::SyntaxWarningTest` Method Audit");

    assert_eq!(
        methods.len(),
        2,
        "SyntaxWarningTest method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "all SyntaxWarningTest methods should be ported"
    );

    for expected in [
        "test_return_in_finally",
        "test_break_and_continue_in_finally",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing SyntaxWarningTest method audit row for `{expected}`"
        );
    }
}

fn assert_source_total(groups: &[ManifestGroup<'_>], source: &str, expected: usize) {
    let actual = groups
        .iter()
        .filter(|group| group.source == source)
        .map(|group| group.methods)
        .sum::<usize>();
    assert_eq!(actual, expected, "source total drifted for `{source}`");
}

fn assert_manifest_group_count(
    groups: &[ManifestGroup<'_>],
    source: &str,
    group: &str,
    expected: usize,
) {
    let actual = groups
        .iter()
        .find(|entry| entry.source == source && entry.group == group)
        .unwrap_or_else(|| panic!("missing manifest group `{source}` / `{group}`"))
        .methods;
    assert_eq!(
        actual, expected,
        "manifest method count drifted for `{source}` / `{group}`"
    );
}

fn summary_rows() -> BTreeMap<&'static str, (usize, usize)> {
    let mut rows = BTreeMap::new();
    for line in MANIFEST.lines() {
        let cells = table_cells(line);
        if cells.len() != 3 {
            continue;
        }
        let Some(status) = strip_backticks(cells[0]) else {
            continue;
        };
        if status == "Status" {
            continue;
        }
        let groups = cells[1]
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid group count for `{status}`"));
        let methods = cells[2]
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid method count for `{status}`"));
        rows.insert(status, (groups, methods));
    }
    rows
}

fn manifest_groups() -> Vec<ManifestGroup<'static>> {
    MANIFEST
        .lines()
        .filter_map(|line| {
            let cells = table_cells(line);
            if cells.len() != 5 {
                return None;
            }
            let source = strip_backticks(cells[0])?;
            if source == "Source" {
                return None;
            }
            let group = strip_backticks(cells[1]).unwrap_or(cells[1]);
            let methods = cells[2].parse::<usize>().ok()?;
            let status = strip_backticks(cells[3])?;
            Some(ManifestGroup {
                source,
                group,
                methods,
                status,
            })
        })
        .collect()
}

fn token_tests_methods() -> Vec<ManifestMethod<'static>> {
    method_audit_methods("## `Lib/test/test_grammar.py::TokenTests` Method Audit")
}

fn method_audit_methods(section_heading: &str) -> Vec<ManifestMethod<'static>> {
    let mut in_section = false;
    let mut methods = Vec::new();

    for line in MANIFEST.lines() {
        if line == section_heading {
            in_section = true;
            continue;
        }

        if in_section && line.starts_with("## ") {
            break;
        }

        if !in_section {
            continue;
        }

        let cells = table_cells(line);
        if cells.len() != 4 {
            continue;
        }
        let Some(method) = strip_backticks(cells[0]) else {
            continue;
        };
        if !method.starts_with("test_") {
            continue;
        }
        let Some(status) = strip_backticks(cells[1]) else {
            continue;
        };
        methods.push(ManifestMethod { method, status });
    }

    methods
}

fn python_test_class_method_counts(source: &str) -> BTreeMap<String, usize> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut classes = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some(rest) = line.strip_prefix("class ") {
            let name = rest
                .split(['(', ':'])
                .next()
                .expect("split always yields a first item");
            classes.push((index, name.to_string()));
        }
    }

    let mut counts = BTreeMap::new();
    for (class_index, (start, name)) in classes.iter().enumerate() {
        let end = classes
            .get(class_index + 1)
            .map(|(index, _)| *index)
            .unwrap_or(lines.len());
        let methods = lines[*start + 1..end]
            .iter()
            .filter(|line| line.starts_with("    def test"))
            .count();
        counts.insert(name.clone(), methods);
    }
    counts
}

fn module_level_test_function_count(source: &str) -> usize {
    source
        .lines()
        .filter(|line| line.starts_with("def test"))
        .count()
}

fn table_cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

fn strip_backticks(cell: &str) -> Option<&str> {
    cell.strip_prefix('`')?.strip_suffix('`')
}
