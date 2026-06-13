use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const MANIFEST: &str = include_str!("cpython_test_manifest.md");
const CPYTHON_COVERAGE: &str = include_str!("cpython_coverage.md");
const CPYTHON_MIGRATION: &str = include_str!("cpython_migration.md");
const CPYTHON_DIFF: &str = include_str!("cpython_diff.rs");
const CPYTHON_SUBSET: &str = include_str!("cpython_subset.rs");
const LANGUAGE_TESTS: &str = include_str!("language.rs");
const README: &str = include_str!("../README.md");
const README_CN: &str = include_str!("../README_CN.md");
const STDLIB_SOURCE: &str = include_str!("../src/stdlib.rs");
const CPYTHON_TEST_AST_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_ast/test_ast.py";
const CPYTHON_TEST_BUILTIN_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_builtin.py";
const CPYTHON_TEST_COMPLEX_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_complex.py";
const CPYTHON_TEST_FLOAT_SOURCE: &str = "/Volumes/samsung/GitHub/cpython/Lib/test/test_float.py";
const CPYTHON_TEST_COLLECTIONS_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_collections.py";
const CPYTHON_TEST_COMPILE_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_compile.py";
const CPYTHON_TEST_MEMORYVIEW_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_memoryview.py";
const CPYTHON_TEST_BYTES_SOURCE: &str = "/Volumes/samsung/GitHub/cpython/Lib/test/test_bytes.py";
const CPYTHON_TEST_TYPES_SOURCE: &str = "/Volumes/samsung/GitHub/cpython/Lib/test/test_types.py";
const CPYTHON_TEST_TYPE_COMMENTS_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_type_comments.py";
const CPYTHON_TEST_TYPE_PARAMS_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_type_params.py";

macro_rules! cpython_source_or_skip {
    ($path:expr) => {
        match fs::read_to_string($path) {
            Ok(source) => source,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("skipping CPython manifest source audit: missing {}", $path);
                return;
            }
            Err(error) => panic!("failed to read {}: {}", $path, error),
        }
    };
}

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

#[derive(Debug)]
struct SandboxStdlibRow<'a> {
    module: String,
    supported_surface: &'a str,
    diff_evidence: &'a str,
    excluded_surface: &'a str,
}

#[test]
fn cpython_diff_test_names_use_diff_subset_suffix() {
    let mut pending_test_attr = false;
    let mut offenders = Vec::new();

    for line in CPYTHON_DIFF.lines() {
        let trimmed = line.trim();
        if trimmed == "#[test]" {
            pending_test_attr = true;
            continue;
        }

        if !pending_test_attr {
            continue;
        }

        pending_test_attr = false;
        let Some(rest) = trimmed.strip_prefix("fn cpython_") else {
            continue;
        };
        let Some((suffix, _)) = rest.split_once('(') else {
            continue;
        };
        let name = format!("cpython_{suffix}");
        if !name.ends_with("_diff_subset") {
            offenders.push(name);
        }
    }

    assert!(
        offenders.is_empty(),
        "cpython_diff.rs CPython test functions must use `_diff_subset` suffix: {offenders:?}"
    );
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
    assert_source_total(&groups, "Lib/test/test_builtin.py", 133);
    assert_source_total(&groups, "Lib/test/test_complex.py", 37);
    assert_source_total(&groups, "Lib/test/test_float.py", 54);
    assert_source_total(&groups, "Lib/test/test_collections.py", 103);
    assert_source_total(&groups, "Lib/test/test_types.py", 131);
    assert_source_total(&groups, "Lib/test/test_type_comments.py", 17);
    assert_source_total(&groups, "Lib/test/test_type_params.py", 107);
    assert_source_total(&groups, "Lib/test/test_memoryview.py", 42);
    assert_source_total(&groups, "Lib/test/test_bytes.py", 143);
    assert_source_total(&groups, "Lib/test/test_ast/test_ast.py", 216);
    assert_source_total(&groups, "Lib/test/test_ast/snippets.py", 0);
}

#[test]
fn cpython_test_manifest_compile_group_counts_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPILE_SOURCE);
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
fn cpython_test_manifest_compile_specifics_method_audit_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPILE_SOURCE);
    let expected = python_test_class_method_names(&source, "TestSpecifics");
    let methods = method_audit_methods("## `Lib/test/test_compile.py::TestSpecifics` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestSpecifics method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| {
            matches!(
                method.status,
                "ported"
                    | "partial"
                    | "blocked_by_runtime"
                    | "blocked_by_ast_module"
                    | "blocked_by_cpython_internal"
                    | "not_started"
            )
        }),
        "TestSpecifics method audit contains an unknown status"
    );

    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).collect::<Vec<_>>();
    let extra = actual.difference(&expected).collect::<Vec<_>>();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "TestSpecifics method audit drifted; missing={missing:?}; extra={extra:?}"
    );
}

#[test]
fn cpython_test_manifest_compile_source_positions_method_audit_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPILE_SOURCE);
    let expected = python_test_class_method_names(&source, "TestSourcePositions");
    let methods =
        method_audit_methods("## `Lib/test/test_compile.py::TestSourcePositions` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestSourcePositions method audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        4,
        "ported TestSourcePositions method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial TestSourcePositions method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "not_started")
            .count(),
        0,
        "not_started TestSourcePositions method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_cpython_internal")
            .count(),
        29,
        "blocked_by_cpython_internal TestSourcePositions method count drifted"
    );
    assert!(
        methods.iter().all(|method| {
            matches!(
                method.status,
                "ported" | "partial" | "blocked_by_cpython_internal" | "not_started"
            )
        }),
        "TestSourcePositions method audit contains an unknown status"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "TestSourcePositions method audit drifted");
}

#[test]
fn cpython_test_manifest_compile_boolean_expression_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPILE_SOURCE);
    let expected = python_test_class_method_names(&source, "TestBooleanExpression");
    let methods =
        method_audit_methods("## `Lib/test/test_compile.py::TestBooleanExpression` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestBooleanExpression method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TestBooleanExpression methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TestBooleanExpression method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_compile_static_attributes_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPILE_SOURCE);
    let expected = python_test_class_method_names(&source, "TestStaticAttributes");
    let methods =
        method_audit_methods("## `Lib/test/test_compile.py::TestStaticAttributes` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestStaticAttributes method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TestStaticAttributes methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TestStaticAttributes method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_compile_expression_stack_size_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPILE_SOURCE);
    let expected = python_test_class_method_names(&source, "TestExpressionStackSize");
    let methods =
        method_audit_methods("## `Lib/test/test_compile.py::TestExpressionStackSize` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestExpressionStackSize method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TestExpressionStackSize methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TestExpressionStackSize method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_compile_stack_size_stability_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPILE_SOURCE);
    let expected = python_test_class_method_names(&source, "TestStackSizeStability");
    let methods =
        method_audit_methods("## `Lib/test/test_compile.py::TestStackSizeStability` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestStackSizeStability method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TestStackSizeStability methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TestStackSizeStability method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_compile_instruction_sequence_method_audit_is_classified() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPILE_SOURCE);
    let expected = python_test_class_method_names(&source, "TestInstructionSequence");
    let methods =
        method_audit_methods("## `Lib/test/test_compile.py::TestInstructionSequence` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestInstructionSequence method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| method.status == "blocked_by_cpython_internal"),
        "TestInstructionSequence methods should stay classified as blocked_by_cpython_internal"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TestInstructionSequence method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_type_comments_group_count_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPE_COMMENTS_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let expected = class_counts
        .get("TypeCommentTests")
        .copied()
        .unwrap_or_else(|| {
            panic!("missing class `TypeCommentTests` in {CPYTHON_TEST_TYPE_COMMENTS_SOURCE}")
        });
    let groups = manifest_groups();

    assert_manifest_group_count(
        &groups,
        "Lib/test/test_type_comments.py",
        "TypeCommentTests",
        expected,
    );
}

#[test]
fn cpython_test_manifest_type_comments_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPE_COMMENTS_SOURCE);
    let expected = python_test_class_method_names(&source, "TypeCommentTests");
    let methods =
        method_audit_methods("## `Lib/test/test_type_comments.py::TypeCommentTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TypeCommentTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TypeCommentTests methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "TypeCommentTests method audit drifted");
}

#[test]
fn cpython_test_manifest_type_params_group_counts_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPE_PARAMS_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let groups = manifest_groups();

    for group in [
        "TypeParamsInvalidTest",
        "TypeParamsNonlocalTest",
        "TypeParamsAccessTest",
        "GlobalGenericClass",
        "TypeParamsLazyEvaluationTest",
        "TypeParamsClassScopeTest",
        "DynamicClassTest",
        "TypeParamsManglingTest",
        "TypeParamsComplexCallsTest",
        "TypeParamsTraditionalTypeVarsTest",
        "TypeParamsTypeVarTest",
        "TypeParamsTypeVarTupleTest",
        "TypeParamsTypeVarParamSpecTest",
        "TypeParamsTypeParamsDunder",
        "Class1",
        "Class2",
        "Class3",
        "Class4",
        "TypeParamsPickleTest",
        "TypeParamsWeakRefTest",
        "TypeParamsRuntimeTest",
        "DefaultsTest",
        "TestEvaluateFunctions",
    ] {
        let expected = class_counts.get(group).copied().unwrap_or_else(|| {
            panic!("missing class `{group}` in {CPYTHON_TEST_TYPE_PARAMS_SOURCE}")
        });
        assert_manifest_group_count(&groups, "Lib/test/test_type_params.py", group, expected);
    }
}

#[test]
fn cpython_test_manifest_type_params_invalid_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPE_PARAMS_SOURCE);
    let expected = python_test_class_method_names(&source, "TypeParamsInvalidTest");
    let methods = method_audit_methods(
        "## `Lib/test/test_type_params.py::TypeParamsInvalidTest` Method Audit",
    );

    assert_eq!(
        methods.len(),
        expected.len(),
        "TypeParamsInvalidTest method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TypeParamsInvalidTest methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TypeParamsInvalidTest method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_type_params_nonlocal_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPE_PARAMS_SOURCE);
    let expected = python_test_class_method_names(&source, "TypeParamsNonlocalTest");
    let methods = method_audit_methods(
        "## `Lib/test/test_type_params.py::TypeParamsNonlocalTest` Method Audit",
    );

    assert_eq!(
        methods.len(),
        expected.len(),
        "TypeParamsNonlocalTest method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TypeParamsNonlocalTest methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TypeParamsNonlocalTest method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_type_params_dunder_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPE_PARAMS_SOURCE);
    let expected = python_test_class_method_names(&source, "TypeParamsTypeParamsDunder");
    let methods = method_audit_methods(
        "## `Lib/test/test_type_params.py::TypeParamsTypeParamsDunder` Method Audit",
    );

    assert_eq!(
        methods.len(),
        expected.len(),
        "TypeParamsTypeParamsDunder method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TypeParamsTypeParamsDunder methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TypeParamsTypeParamsDunder method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_type_params_weakref_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPE_PARAMS_SOURCE);
    let expected = python_test_class_method_names(&source, "TypeParamsWeakRefTest");
    let methods = method_audit_methods(
        "## `Lib/test/test_type_params.py::TypeParamsWeakRefTest` Method Audit",
    );

    assert_eq!(
        methods.len(),
        expected.len(),
        "TypeParamsWeakRefTest method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "TypeParamsWeakRefTest methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "TypeParamsWeakRefTest method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_builtin_group_counts_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let groups = manifest_groups();

    assert_manifest_group_count(
        &groups,
        "Lib/test/test_builtin.py",
        "module-level `test_*` functions",
        module_level_test_function_count(&source),
    );

    for group in [
        "BuiltinTest",
        "TestBreakpoint",
        "PtyTests",
        "TestSorted",
        "ShutdownTest",
        "ImmortalTests",
        "TestType",
    ] {
        let expected = class_counts
            .get(group)
            .copied()
            .unwrap_or_else(|| panic!("missing class `{group}` in {CPYTHON_TEST_BUILTIN_SOURCE}"));
        assert_manifest_group_count(&groups, "Lib/test/test_builtin.py", group, expected);
    }
}

#[test]
fn cpython_test_manifest_complex_group_count_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPLEX_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let expected = class_counts
        .get("ComplexTest")
        .copied()
        .unwrap_or_else(|| panic!("missing class `ComplexTest` in {CPYTHON_TEST_COMPLEX_SOURCE}"));
    let groups = manifest_groups();

    assert_manifest_group_count(&groups, "Lib/test/test_complex.py", "ComplexTest", expected);
}

#[test]
fn cpython_test_manifest_complex_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COMPLEX_SOURCE);
    let expected = python_test_class_method_names(&source, "ComplexTest")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods = method_audit_methods("## `Lib/test/test_complex.py::ComplexTest` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "ComplexTest method audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        37,
        "ported ComplexTest method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial ComplexTest method count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "partial")),
        "ComplexTest method audit contains an unexpected status"
    );

    let expected_statuses = BTreeMap::from([
        ("test___complex__", "ported"),
        ("test_abs", "ported"),
        ("test_add", "ported"),
        ("test_boolcontext", "ported"),
        ("test_conjugate", "ported"),
        ("test_constructor", "ported"),
        ("test_constructor_from_string", "ported"),
        ("test_constructor_negative_nans_from_string", "ported"),
        ("test_constructor_special_numbers", "ported"),
        ("test_divmod", "ported"),
        ("test_divmod_zero_division", "ported"),
        ("test_floordiv", "ported"),
        ("test_floordiv_zero_division", "ported"),
        ("test_format", "ported"),
        ("test_from_number", "ported"),
        ("test_from_number_subclass", "ported"),
        ("test_getnewargs", "ported"),
        ("test_hash", "ported"),
        ("test_mod", "ported"),
        ("test_mod_zero_division", "ported"),
        ("test_mul", "ported"),
        ("test_neg", "ported"),
        ("test_negated_imaginary_literal", "ported"),
        ("test_negative_zero_repr_str", "ported"),
        ("test_overflow", "ported"),
        ("test_plus_minus_0j", "ported"),
        ("test_pos", "ported"),
        ("test_pow", "ported"),
        ("test_pow_with_small_integer_exponents", "ported"),
        ("test_repr_roundtrip", "ported"),
        ("test_repr_str", "ported"),
        ("test_richcompare", "ported"),
        ("test_richcompare_boundaries", "ported"),
        ("test_sub", "ported"),
        ("test_truediv", "ported"),
        ("test_truediv_zero_division", "ported"),
        ("test_underscores", "ported"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "ComplexTest method audit statuses drifted"
    );

    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "ComplexTest method audit drifted");
}

#[test]
fn cpython_test_manifest_float_group_counts_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_FLOAT_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let groups = manifest_groups();

    assert_manifest_group_count(
        &groups,
        "Lib/test/test_float.py",
        "module-level `test_*` functions",
        module_level_test_function_count(&source),
    );

    for group in [
        "GeneralFloatCases",
        "FormatFunctionsTestCase",
        "IEEEFormatTestCase",
        "FormatTestCase",
        "ReprTestCase",
        "RoundTestCase",
        "InfNanTest",
        "HexFloatTestCase",
    ] {
        let expected = class_counts
            .get(group)
            .copied()
            .unwrap_or_else(|| panic!("missing class `{group}` in {CPYTHON_TEST_FLOAT_SOURCE}"));
        assert_manifest_group_count(&groups, "Lib/test/test_float.py", group, expected);
    }
}

#[test]
fn cpython_test_manifest_float_general_cases_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_FLOAT_SOURCE);
    let expected = python_test_class_method_names(&source, "GeneralFloatCases")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods =
        method_audit_methods("## `Lib/test/test_float.py::GeneralFloatCases` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "GeneralFloatCases method audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        21,
        "ported GeneralFloatCases method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        1,
        "partial GeneralFloatCases method count drifted"
    );

    let expected_statuses = BTreeMap::from([
        ("test_float", "ported"),
        ("test_noargs", "ported"),
        ("test_underscores", "ported"),
        ("test_non_numeric_input_types", "ported"),
        ("test_float_memoryview", "ported"),
        ("test_error_message", "ported"),
        ("test_float_with_comma", "partial"),
        ("test_floatconversion", "ported"),
        ("test_keyword_args", "ported"),
        ("test_keywords_in_subclass", "ported"),
        ("test_from_number", "ported"),
        ("test_from_number_subclass", "ported"),
        ("test_is_integer", "ported"),
        ("test_floatasratio", "ported"),
        ("test_float_containment", "ported"),
        ("test_float_floor", "ported"),
        ("test_float_ceil", "ported"),
        ("test_float_mod", "ported"),
        ("test_float_pow", "ported"),
        ("test_hash", "ported"),
        ("test_hash_nan", "ported"),
        ("test_issue_gh143006", "ported"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "GeneralFloatCases method audit statuses drifted"
    );

    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "GeneralFloatCases method audit drifted");
}

#[test]
fn cpython_test_manifest_float_method_audit_statuses_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_FLOAT_SOURCE);

    for (class_name, heading, expected_statuses) in [
        (
            "FormatFunctionsTestCase",
            "## `Lib/test/test_float.py::FormatFunctionsTestCase` Method Audit",
            BTreeMap::from([("test_getformat", "ported")]),
        ),
        (
            "IEEEFormatTestCase",
            "## `Lib/test/test_float.py::IEEEFormatTestCase` Method Audit",
            BTreeMap::from([
                ("test_double_specials_do_unpack", "blocked_by_runtime"),
                ("test_float_specials_do_unpack", "blocked_by_runtime"),
                (
                    "test_serialized_float_rounding",
                    "blocked_by_cpython_internal",
                ),
            ]),
        ),
        (
            "FormatTestCase",
            "## `Lib/test/test_float.py::FormatTestCase` Method Audit",
            BTreeMap::from([
                ("test_format", "ported"),
                ("test_format_testfile", "ported"),
                ("test_issue5864", "ported"),
                ("test_issue35560", "ported"),
            ]),
        ),
        (
            "ReprTestCase",
            "## `Lib/test/test_float.py::ReprTestCase` Method Audit",
            BTreeMap::from([("test_repr", "ported"), ("test_short_repr", "ported")]),
        ),
        (
            "RoundTestCase",
            "## `Lib/test/test_float.py::RoundTestCase` Method Audit",
            BTreeMap::from([
                ("test_inf_nan", "ported"),
                ("test_inf_nan_ndigits", "ported"),
                ("test_large_n", "ported"),
                ("test_small_n", "ported"),
                ("test_overflow", "ported"),
                ("test_previous_round_bugs", "ported"),
                ("test_matches_float_format", "ported"),
                ("test_format_specials", "ported"),
                ("test_None_ndigits", "ported"),
                ("test_round_with_none_arg_direct_call", "ported"),
            ]),
        ),
        (
            "InfNanTest",
            "## `Lib/test/test_float.py::InfNanTest` Method Audit",
            BTreeMap::from([
                ("test_inf_from_str", "ported"),
                ("test_inf_as_str", "ported"),
                ("test_nan_from_str", "ported"),
                ("test_nan_as_str", "ported"),
                ("test_inf_signs", "ported"),
                ("test_nan_signs", "ported"),
            ]),
        ),
        (
            "HexFloatTestCase",
            "## `Lib/test/test_float.py::HexFloatTestCase` Method Audit",
            BTreeMap::from([
                ("test_ends", "ported"),
                ("test_invalid_inputs", "ported"),
                ("test_whitespace", "ported"),
                ("test_from_hex", "ported"),
                ("test_roundtrip", "ported"),
                ("test_subclass", "ported"),
            ]),
        ),
    ] {
        let expected = python_test_class_method_names(&source, class_name)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let methods = method_audit_methods(heading);
        assert_eq!(
            methods.len(),
            expected.len(),
            "{class_name} method audit row count drifted"
        );

        let actual_statuses = methods
            .iter()
            .map(|method| (method.method, method.status))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            actual_statuses, expected_statuses,
            "{class_name} method audit statuses drifted"
        );

        let actual = methods
            .iter()
            .map(|method| method.method.to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{class_name} method audit drifted");
    }
}

#[test]
fn cpython_test_manifest_float_fromhex_matrix_inputs_have_runtime_evidence() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_FLOAT_SOURCE);
    let method_source = python_test_method_source(&source, "HexFloatTestCase", "test_from_hex");
    let mut inputs = python_call_string_arguments(&method_source, "fromHex");
    inputs.extend(python_reference_string_arguments(&method_source, "fromHex"));
    let inputs = inputs.into_iter().collect::<BTreeSet<_>>();

    assert_eq!(
        inputs.len(),
        262,
        "CPython HexFloatTestCase::test_from_hex input matrix drifted"
    );

    let evidence = format!("{CPYTHON_SUBSET}\n{CPYTHON_DIFF}");
    let missing = inputs
        .iter()
        .filter(|input| !python_string_literal_has_rust_evidence(input, &evidence))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing Rust runtime evidence for CPython HexFloatTestCase::test_from_hex inputs: {missing:?}"
    );

    for name in [
        "cpython_float_fromhex_accepted_variants_subset",
        "cpython_float_fromhex_overflow_zero_underflow_subset",
        "cpython_float_fromhex_rounding_boundaries_subset",
        "cpython_float_fromhex_bpo44954_regression_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(name),
            "missing subset evidence `{name}` for HexFloatTestCase::test_from_hex"
        );
    }

    for name in [
        "float-fromhex-accepted-variants",
        "float-fromhex-overflow-zero-underflow",
        "float-fromhex-rounding-boundaries",
        "float-fromhex-bpo-44954-regression",
    ] {
        assert!(
            CPYTHON_DIFF.contains(name),
            "missing differential evidence `{name}` for HexFloatTestCase::test_from_hex"
        );
    }
}

#[test]
fn cpython_test_manifest_builtin_eval_exec_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let builtin_methods = python_test_class_method_names(&source, "BuiltinTest")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods =
        method_audit_methods("## `Lib/test/test_builtin.py::BuiltinTest Eval/Exec Method Audit`");

    assert_eq!(
        methods.len(),
        15,
        "BuiltinTest eval/exec audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        15,
        "ported BuiltinTest eval/exec audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "not_started")
            .count(),
        0,
        "not_started BuiltinTest eval/exec audit count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "not_started")),
        "BuiltinTest eval/exec audit contains an unexpected status"
    );

    let expected_statuses = BTreeMap::from([
        ("test_eval", "ported"),
        ("test_eval_kwargs", "ported"),
        ("test_general_eval", "ported"),
        ("test_exec", "ported"),
        ("test_exec_kwargs", "ported"),
        ("test_exec_globals", "ported"),
        ("test_exec_globals_frozen", "ported"),
        ("test_exec_globals_error_on_get", "ported"),
        ("test_exec_globals_dict_subclass", "ported"),
        ("test_eval_builtins_mapping", "ported"),
        ("test_exec_builtins_mapping_import", "ported"),
        ("test_eval_builtins_mapping_reduce", "ported"),
        ("test_exec_redirected", "ported"),
        ("test_exec_closure", "ported"),
        ("test_exec_filter_syntax_warnings_by_module", "ported"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "BuiltinTest eval/exec audit statuses drifted"
    );
    for method in expected_statuses.keys() {
        assert!(
            builtin_methods.contains(*method),
            "BuiltinTest eval/exec audit references missing CPython method `{method}`"
        );
    }
}

#[test]
fn cpython_test_manifest_builtin_core_runtime_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let builtin_methods = python_test_class_method_names(&source, "BuiltinTest")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods = method_audit_methods(
        "## `Lib/test/test_builtin.py::BuiltinTest Core Runtime Method Audit`",
    );

    assert_eq!(
        methods.len(),
        27,
        "BuiltinTest core runtime audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        26,
        "ported BuiltinTest core runtime audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        1,
        "partial BuiltinTest core runtime audit count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "partial")),
        "BuiltinTest core runtime audit contains an unexpected status"
    );

    let expected_statuses = BTreeMap::from([
        ("test___ne__", "ported"),
        ("test_abs", "ported"),
        ("test_all", "ported"),
        ("test_all_any_tuple_list_set_optimization", "ported"),
        ("test_any", "ported"),
        ("test_ascii", "partial"),
        ("test_bin", "ported"),
        ("test_bug_27936", "ported"),
        ("test_callable", "ported"),
        ("test_chr", "ported"),
        ("test_cmp", "ported"),
        ("test_divmod", "ported"),
        ("test_format", "ported"),
        ("test_hash", "ported"),
        ("test_hex", "ported"),
        ("test_id", "ported"),
        ("test_invalid_hash_typeerror", "ported"),
        ("test_len", "ported"),
        ("test_neg", "ported"),
        ("test_next", "ported"),
        ("test_oct", "ported"),
        ("test_ord", "ported"),
        ("test_pow", "ported"),
        ("test_repr", "ported"),
        ("test_repr_blocked", "ported"),
        ("test_round", "ported"),
        ("test_round_large", "ported"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "BuiltinTest core runtime audit statuses drifted"
    );
    for method in expected_statuses.keys() {
        assert!(
            builtin_methods.contains(*method),
            "BuiltinTest core runtime audit references missing CPython method `{method}`"
        );
    }
}

#[test]
fn cpython_test_manifest_builtin_attribute_introspection_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let builtin_methods = python_test_class_method_names(&source, "BuiltinTest")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods = method_audit_methods(
        "## `Lib/test/test_builtin.py::BuiltinTest Attribute/Introspection Method Audit`",
    );

    assert_eq!(
        methods.len(),
        9,
        "BuiltinTest attribute/introspection audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        8,
        "ported BuiltinTest attribute/introspection audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        1,
        "partial BuiltinTest attribute/introspection audit count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "partial")),
        "BuiltinTest attribute/introspection audit contains an unexpected status"
    );

    let expected_statuses = BTreeMap::from([
        ("test_delattr", "ported"),
        ("test_dir", "ported"),
        ("test_getattr", "partial"),
        ("test_hasattr", "ported"),
        ("test_isinstance", "ported"),
        ("test_issubclass", "ported"),
        ("test_setattr", "ported"),
        ("test_type", "ported"),
        ("test_vars", "ported"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "BuiltinTest attribute/introspection audit statuses drifted"
    );
    for method in expected_statuses.keys() {
        assert!(
            builtin_methods.contains(*method),
            "BuiltinTest attribute/introspection audit references missing CPython method `{method}`"
        );
    }
}

#[test]
fn cpython_test_manifest_memoryview_direct_methods_are_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_MEMORYVIEW_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let expected_count = class_counts.values().sum::<usize>();
    let groups = manifest_groups();

    assert_manifest_group_count(
        &groups,
        "Lib/test/test_memoryview.py",
        "direct test method definitions",
        expected_count,
    );

    let evidence = format!("{MANIFEST}\n{CPYTHON_SUBSET}\n{CPYTHON_COVERAGE}\n{CPYTHON_MIGRATION}");
    let missing = class_counts
        .keys()
        .flat_map(|class_name| python_test_class_method_names(&source, class_name))
        .filter(|method_name| !evidence.contains(method_name))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "memoryview direct methods are not tracked in manifest: {missing:?}"
    );
}

#[test]
fn cpython_memoryview_methods_release_diff_covers_basic_methods_runtime_subset() {
    let diff_name = "cpython_memoryview_methods_release_diff_subset";
    let subset_name = "cpython_memoryview_basic_methods_and_release_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "memoryview method/release direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "memoryview basic method/release runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "memoryview method/release docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("memoryview method/release diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "m.tobytes()",
        "m.tolist()",
        "m.toreadonly()",
        "with m as cm",
        "m.release()",
        "list(reversed(m))",
    ] {
        assert!(
            body.contains(required),
            "memoryview method/release diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_memoryview_slice_attributes_diff_covers_runtime_subsets() {
    let diff_name = "cpython_memoryview_slice_and_attributes_diff_subset";
    let runtime_subsets = [
        "cpython_memoryview_slice_reference_subset",
        "cpython_memoryview_public_buffer_attributes_subset",
    ];

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "memoryview slice/attributes direct CPython diff evidence must exist"
    );

    for subset in runtime_subsets {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "memoryview runtime subset evidence `{subset}` must exist"
        );
        for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
            assert!(
                document.contains(diff_name) && document.contains(subset),
                "memoryview slice/attributes docs must link `{diff_name}` to `{subset}`"
            );
        }
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("memoryview slice/attributes diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "memoryview(base)[1:7]",
        "m[0] = ord('1')",
        "view.obj is base",
        "m.strides",
        "m.c_contiguous",
        "released.obj",
    ] {
        assert!(
            body.contains(required),
            "memoryview slice/attributes diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_memoryview_count_index_diff_covers_runtime_subset() {
    let diff_name = "cpython_memoryview_count_index_diff_subset";
    let subset_name = "cpython_memoryview_getitem_index_count_compare_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "memoryview count/index direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "memoryview getitem/index/count/compare runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "memoryview count/index docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("memoryview count/index diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "memoryview.count",
        "m.count(ord('a'))",
        "m.index(ord('c'))",
        "memoryview(b'abc').count()",
        "skipping memoryview.count/index diff",
    ] {
        assert!(
            body.contains(required),
            "memoryview count/index diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_memoryview_rejection_and_hash_diff_covers_split_runtime_subsets() {
    let diff_name = "cpython_memoryview_rejection_and_hash_diff_subset";
    let runtime_subsets = [
        "cpython_memoryview_copy_rejection_subset",
        "cpython_memoryview_pickle_rejection_subset",
        "cpython_memoryview_hash_release_cache_subset",
    ];

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "memoryview rejection/hash direct CPython diff evidence must exist"
    );

    for subset in runtime_subsets {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "memoryview runtime subset evidence `{subset}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(subset),
            "coverage document must mention memoryview runtime subset `{subset}`"
        );
        assert!(
            CPYTHON_MIGRATION.contains(subset),
            "migration document must mention memoryview runtime subset `{subset}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("copy rejection")
                && document.contains("pickle rejection")
                && document.contains("hash/release-cache"),
            "memoryview docs must explain that `{diff_name}` covers copy, pickle, and hash/release-cache behavior"
        );
    }
}

#[test]
fn cpython_memoryview_hex_reentrant_release_diff_is_capability_gated() {
    let diff_name = "cpython_memoryview_hex_reentrant_release_diff_subset";
    let subset_name = "cpython_memoryview_hex_reentrant_release_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "memoryview hex re-entrant release direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "memoryview hex re-entrant release runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "memoryview hex re-entrant docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("memoryview hex re-entrant release diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "mv.hex(S(b':'))",
        "mv.release()",
        "BufferError",
        "skipping memoryview.hex re-entrant release diff",
        "accepted",
    ] {
        assert!(
            body.contains(required),
            "memoryview hex re-entrant gated diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_test_manifest_bytes_base_methods_are_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BYTES_SOURCE);
    let expected_count = python_test_class_method_counts(&source)
        .get("BaseBytesTest")
        .copied()
        .unwrap_or_else(|| panic!("missing class `BaseBytesTest` in {CPYTHON_TEST_BYTES_SOURCE}"));
    let groups = manifest_groups();

    assert_manifest_group_count(
        &groups,
        "Lib/test/test_bytes.py",
        "BaseBytesTest",
        expected_count,
    );
    assert_manifest_group_status(
        &groups,
        "Lib/test/test_bytes.py",
        "BaseBytesTest",
        "partial",
    );

    let evidence = format!("{MANIFEST}\n{CPYTHON_SUBSET}\n{CPYTHON_COVERAGE}\n{CPYTHON_MIGRATION}");
    let missing = python_test_class_method_names(&source, "BaseBytesTest")
        .into_iter()
        .filter(|method_name| !evidence.contains(method_name))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "BaseBytesTest methods are not tracked in manifest evidence: {missing:?}"
    );
}

#[test]
fn cpython_bytes_search_compare_slice_diff_covers_compare_slice_reversed_runtime_subset() {
    let diff_name = "cpython_bytes_search_compare_slice_diff_subset";
    let subset_name = "cpython_bytes_compare_slice_reversed_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes search/compare/slice direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes compare/slice/reversed runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytes search/compare/slice diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "b1 == b2",
        "ctor(b'\\0a\\0b\\0c') == 'abc'",
        "list(reversed(b))",
        "b[:5]",
        "L[start:stop:step]",
    ] {
        assert!(
            body.contains(required),
            "bytes search/compare/slice diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_basics_diff_covers_ord_and_empty_index_runtime_subsets() {
    let diff_name = "cpython_bytes_basics_and_empty_index_diff_subset";
    let runtime_subsets = [
        "cpython_bytes_basics_and_ord_subset",
        "cpython_bytes_empty_sequence_index_subset",
    ];

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes basics direct CPython diff evidence must exist"
    );

    for subset in runtime_subsets {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "bytes runtime subset evidence `{subset}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(subset),
            "coverage document must mention bytes runtime subset `{subset}`"
        );
        assert!(
            CPYTHON_MIGRATION.contains(subset),
            "migration document must mention bytes runtime subset `{subset}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("empty")
                && document.contains("[0, 65, 127, 128, 255]"),
            "bytes docs must explain that `{diff_name}` covers one-byte ord samples and empty-index behavior"
        );
    }
}

#[test]
fn cpython_bytes_core_typeerror_diff_covers_runtime_subset() {
    let subset_name = "cpython_bytes_core_method_typeerror_messages_subset";
    let diff_names = [
        "cpython_bytes_core_method_typeerror_messages_diff_subset",
        "cpython_bytes_search_missing_typeerror_messages_diff_subset",
    ];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes core TypeError runtime subset evidence must exist"
    );

    for diff_name in diff_names {
        assert!(
            CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
            "bytes core TypeError direct CPython diff evidence `{diff_name}` must exist"
        );
        for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
            assert!(
                document.contains(diff_name) && document.contains(subset_name),
                "bytes core TypeError docs must link `{diff_name}` to `{subset_name}`"
            );
        }
    }

    let start = CPYTHON_DIFF
        .find("fn cpython_bytes_core_method_typeerror_messages_diff_subset(")
        .expect("bytes core TypeError diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "unbound method",
        "slice indices must be integers or None or have an __index__ method",
        "replace expected at least 2 arguments",
        "failures = []",
    ] {
        assert!(
            body.contains(required),
            "bytes core TypeError diff evidence must contain `{required}`"
        );
    }

    let start = CPYTHON_DIFF
        .find("fn cpython_bytes_search_missing_typeerror_messages_diff_subset(")
        .expect("bytes search missing-argument TypeError diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "count expected at least 1 argument, got 0",
        "startswith",
        "skipping bytes search missing-argument TypeError text diff",
    ] {
        assert!(
            body.contains(required),
            "bytes search missing-argument TypeError diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_more_typeerror_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytes_more_method_typeerror_messages_diff_subset";
    let subset_name = "cpython_bytes_more_method_typeerror_messages_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes additional TypeError direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes additional TypeError runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes additional TypeError docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytes additional TypeError diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "lower",
        "splitlines",
        "expandtabs",
        "zfill",
        "removeprefix",
        "removesuffix",
    ] {
        assert!(
            body.contains(required),
            "bytes additional TypeError diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_join_translate_maketrans_typeerror_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytes_join_translate_maketrans_typeerror_messages_diff_subset";
    let subset_name = "cpython_bytes_join_translate_maketrans_typeerror_messages_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes join/translate/maketrans TypeError direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes join/translate/maketrans TypeError runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes join/translate/maketrans TypeError docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytes join/translate/maketrans TypeError diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "join.unbound",
        "join.noniter",
        "translate.missing-table",
        "translate.descriptor-missing-table",
        "maketrans.no-args",
    ] {
        assert!(
            body.contains(required),
            "bytes join/translate/maketrans TypeError diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_prefix_suffix_typeerror_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytes_prefix_suffix_typeerror_messages_diff_subset";
    let subset_name = "cpython_bytes_prefix_suffix_typeerror_messages_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes prefix/suffix TypeError direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes prefix/suffix TypeError runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes prefix/suffix TypeError docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytes prefix/suffix TypeError diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "startswith",
        "endswith",
        "first arg must be bytes or a tuple of bytes",
        "a bytes-like object is required",
        "tuple-str",
    ] {
        assert!(
            body.contains(required),
            "bytes prefix/suffix TypeError diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_method_typeerror_diff_covers_runtime_subset() {
    let subset_name = "cpython_bytes_method_typeerror_messages_subset";
    let diff_names = [
        "cpython_bytes_method_typeerror_messages_diff_subset",
        "cpython_bytes_fill_length_typeerror_messages_diff_subset",
    ];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes method TypeError runtime subset evidence must exist"
    );

    for diff_name in diff_names {
        assert!(
            CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
            "bytes method TypeError direct CPython diff evidence `{diff_name}` must exist"
        );
        for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
            assert!(
                document.contains(diff_name) && document.contains(subset_name),
                "bytes method TypeError docs must link `{diff_name}` to `{subset_name}`"
            );
        }
    }

    let start = CPYTHON_DIFF
        .find("fn cpython_bytes_method_typeerror_messages_diff_subset(")
        .expect("bytes method TypeError diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "split",
        "partition",
        "strip",
        "center",
        "a bytes-like object is required",
        "not memoryview",
    ] {
        assert!(
            body.contains(required),
            "bytes method TypeError diff evidence must contain `{required}`"
        );
    }

    let start = CPYTHON_DIFF
        .find("fn cpython_bytes_fill_length_typeerror_messages_diff_subset(")
        .expect("bytes fill-length TypeError diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "empty-bytes-fill",
        "long-bytearray-fill",
        "skipping bytes fill length TypeError text diff",
    ] {
        assert!(
            body.contains(required),
            "bytes fill-length TypeError diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_bytearray_index_error_hash_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytes_bytearray_index_error_and_hash_diff_subset";
    let subset_name = "cpython_bytes_bytearray_index_error_and_hash_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes/bytearray index error and hash direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes/bytearray index error and hash runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes/bytearray index error and hash docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytes/bytearray index error and hash diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "bytes-getitem",
        "bytearray-getitem",
        "bytearray-hash",
        "bytearray-setitem",
    ] {
        assert!(
            body.contains(required),
            "bytes/bytearray index error and hash diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_dunder_bytes_dispatch_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytes_dunder_bytes_dispatch_diff_subset";
    let subset_name = "cpython_bytes_dunder_bytes_and_blocking_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes __bytes__ dispatch direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes __bytes__ dispatch runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes __bytes__ dispatch docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytes __bytes__ dispatch diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "bytes(WithBytes(b'abc'))",
        "bytes(IndexWithBytes())",
        "__bytes__ = None",
        "bytearray(WithBytes(b'abc'))",
        "StrWithBytes",
        "BytesWithBytes",
    ] {
        assert!(
            body.contains(required),
            "bytes __bytes__ dispatch diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_warning_compare_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytes_warning_compare_diff_subset";
    let subset_name = "cpython_bytes_warning_compare_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes warning-compare direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes warning-compare runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes warning-compare docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    for required in [
        "sys.flags.bytes_warning",
        "BytesWarning",
        "bytearray(b'') == ''",
        "b'' == ''",
        "&[\"-b\"]",
        "&[\"-bb\"]",
    ] {
        assert!(
            CPYTHON_DIFF.contains(required),
            "bytes warning-compare diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytearray_hex_reentrant_separator_diff_is_capability_gated() {
    let diff_name = "cpython_bytearray_hex_reentrant_separator_buffererror_diff_subset";
    let subset_name = "cpython_bytearray_hex_reentrant_separator_buffererror_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytearray hex re-entrant separator direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytearray hex re-entrant separator runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytearray hex re-entrant docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytearray hex re-entrant diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "ba.hex(S(b':'))",
        "BufferError",
        "skipping bytearray.hex re-entrant separator diff",
        "accepted",
    ] {
        assert!(
            body.contains(required),
            "bytearray hex re-entrant gated diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytearray_iterator_pickle_shared_exporter_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytearray_iterator_pickle_shared_exporter_diff_subset";
    let subset_name = "cpython_bytearray_iterator_pickle_shared_exporter_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytearray iterator shared-exporter pickle direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytearray iterator shared-exporter pickle runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytearray iterator shared-exporter docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytearray iterator shared-exporter diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "pickle.dumps((itorig, orig), proto)",
        "pickle.loads(payload)",
        "b[:] = data",
        "list(it) == data[1:]",
        "list(it) == []",
        "print(counts)",
    ] {
        assert!(
            body.contains(required),
            "bytearray iterator shared-exporter diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_pickle_roundtrip_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytes_pickle_roundtrip_diff_subset";
    let subset_name = "cpython_bytes_pickle_roundtrip_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes/bytearray pickle round-trip direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes/bytearray pickle round-trip runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes/bytearray pickle docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytes/bytearray pickle diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "pickle.dumps(original, proto)",
        "pickle.loads",
        "type(restored) is type(original)",
        "restored.append(ord('x'))",
        "restored is mutable",
    ] {
        assert!(
            body.contains(required),
            "bytes/bytearray pickle diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_bytes_iterator_pickle_roundtrip_diff_covers_runtime_subset() {
    let diff_name = "cpython_bytes_iterator_pickle_roundtrip_diff_subset";
    let subset_name = "cpython_bytes_iterator_pickle_roundtrip_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "bytes/bytearray iterator pickle round-trip direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "bytes/bytearray iterator pickle round-trip runtime subset evidence must exist"
    );

    for document in [MANIFEST, CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "bytes/bytearray iterator pickle docs must link `{diff_name}` to `{subset_name}`"
        );
    }

    let start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("bytes/bytearray iterator pickle diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    for required in [
        "pickle.dumps(itorg, proto)",
        "pickle.loads(payload)",
        "type(itorg) is type(it)",
        "list(again) == data",
        "list(it) == data[1:]",
        "print(ctor.__name__, initial, repeated, running)",
    ] {
        assert!(
            body.contains(required),
            "bytes/bytearray iterator pickle diff evidence must contain `{required}`"
        );
    }
}

#[test]
fn cpython_test_manifest_bytes_group_counts_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BYTES_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let groups = manifest_groups();

    for (group, status) in [
        ("BaseBytesTest", "partial"),
        ("BytesTest", "partial"),
        ("ByteArrayTest", "partial"),
        ("AssortedBytesTest", "ported"),
        ("BytearrayPEP3137Test", "ported"),
        ("SubclassTest", "ported"),
        ("ByteArraySubclassTest", "ported"),
        ("FreeThreadingTest", "blocked_by_cpython_internal"),
    ] {
        let expected = class_counts
            .get(group)
            .copied()
            .unwrap_or_else(|| panic!("missing class `{group}` in {CPYTHON_TEST_BYTES_SOURCE}"));
        assert_manifest_group_count(&groups, "Lib/test/test_bytes.py", group, expected);
        assert_manifest_group_status(&groups, "Lib/test/test_bytes.py", group, status);
    }
}

#[test]
fn cpython_test_manifest_builtin_aggregate_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let builtin_methods = python_test_class_method_names(&source, "BuiltinTest")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods = method_audit_methods(
        "## `Lib/test/test_builtin.py::BuiltinTest Aggregate Builtins Method Audit`",
    );

    assert_eq!(
        methods.len(),
        4,
        "BuiltinTest aggregate audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        3,
        "ported BuiltinTest aggregate audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial BuiltinTest aggregate audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_cpython_internal")
            .count(),
        1,
        "blocked BuiltinTest aggregate audit count drifted"
    );
    assert!(
        methods.iter().all(|method| matches!(
            method.status,
            "ported" | "partial" | "blocked_by_cpython_internal"
        )),
        "BuiltinTest aggregate audit contains an unexpected status"
    );

    let expected_statuses = BTreeMap::from([
        ("test_max", "ported"),
        ("test_min", "ported"),
        ("test_sum", "ported"),
        ("test_sum_accuracy", "blocked_by_cpython_internal"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "BuiltinTest aggregate audit statuses drifted"
    );
    for method in expected_statuses.keys() {
        assert!(
            builtin_methods.contains(*method),
            "BuiltinTest aggregate audit references missing CPython method `{method}`"
        );
    }
}

#[test]
fn cpython_test_manifest_builtin_iterator_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let builtin_methods = python_test_class_method_names(&source, "BuiltinTest")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods = method_audit_methods(
        "## `Lib/test/test_builtin.py::BuiltinTest Iterator Builtins Method Audit`",
    );

    assert_eq!(
        methods.len(),
        22,
        "BuiltinTest iterator audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        20,
        "ported BuiltinTest iterator audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_cpython_internal")
            .count(),
        2,
        "blocked BuiltinTest iterator audit count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "blocked_by_cpython_internal")),
        "BuiltinTest iterator audit contains an unexpected status"
    );

    let expected_statuses = BTreeMap::from([
        ("test_filter", "ported"),
        ("test_filter_pickle", "ported"),
        ("test_filter_dealloc", "blocked_by_cpython_internal"),
        ("test_iter", "ported"),
        ("test_map", "ported"),
        ("test_map_pickle", "ported"),
        ("test_map_pickle_strict", "ported"),
        ("test_map_pickle_strict_fail", "ported"),
        ("test_map_strict", "ported"),
        ("test_map_strict_iterators", "ported"),
        ("test_map_strict_error_handling", "ported"),
        ("test_map_strict_error_handling_stopiteration", "ported"),
        ("test_zip", "ported"),
        ("test_zip_pickle", "ported"),
        ("test_zip_pickle_strict", "ported"),
        ("test_zip_pickle_strict_fail", "ported"),
        ("test_zip_bad_iterable", "ported"),
        ("test_zip_strict", "ported"),
        ("test_zip_strict_iterators", "ported"),
        ("test_zip_strict_error_handling", "ported"),
        ("test_zip_strict_error_handling_stopiteration", "ported"),
        ("test_zip_result_gc", "blocked_by_cpython_internal"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "BuiltinTest iterator audit statuses drifted"
    );
    for method in expected_statuses.keys() {
        assert!(
            builtin_methods.contains(*method),
            "BuiltinTest iterator audit references missing CPython method `{method}`"
        );
    }
}

#[test]
fn cpython_test_manifest_builtin_compile_io_regression_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let builtin_methods = python_test_class_method_names(&source, "BuiltinTest")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods = method_audit_methods(
        "## `Lib/test/test_builtin.py::BuiltinTest Compile/I/O/Regression Method Audit`",
    );

    assert_eq!(
        methods.len(),
        18,
        "BuiltinTest compile/I/O/regression audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        13,
        "ported BuiltinTest compile/I/O/regression audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial BuiltinTest compile/I/O/regression audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "not_started")
            .count(),
        0,
        "not_started BuiltinTest compile/I/O/regression audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_runtime")
            .count(),
        5,
        "blocked_by_runtime BuiltinTest compile/I/O/regression audit count drifted"
    );
    assert!(
        methods.iter().all(|method| matches!(
            method.status,
            "ported" | "not_started" | "blocked_by_runtime"
        )),
        "BuiltinTest compile/I/O/regression audit contains an unexpected status"
    );

    let expected_statuses = BTreeMap::from([
        ("test_bool_notimplemented", "ported"),
        ("test_bytearray_extend_error", "ported"),
        ("test_bytearray_join_with_custom_iterator", "ported"),
        ("test_bytearray_join_with_misbehaving_iterator", "ported"),
        ("test_bytearray_translate", "ported"),
        ("test_compile", "ported"),
        ("test_compile_ast", "ported"),
        ("test_compile_async_generator", "ported"),
        ("test_compile_top_level_await", "ported"),
        ("test_compile_top_level_await_invalid_cases", "ported"),
        ("test_compile_top_level_await_no_coro", "ported"),
        ("test_construct_singletons", "ported"),
        ("test_input", "blocked_by_runtime"),
        ("test_input_gh130163", "blocked_by_runtime"),
        ("test_open", "blocked_by_runtime"),
        ("test_open_default_encoding", "blocked_by_runtime"),
        ("test_open_non_inheritable", "blocked_by_runtime"),
        ("test_singleton_attribute_access", "ported"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "BuiltinTest compile/I/O/regression audit statuses drifted"
    );
    for method in expected_statuses.keys() {
        assert!(
            builtin_methods.contains(*method),
            "BuiltinTest compile/I/O/regression audit references missing CPython method `{method}`"
        );
    }
}

#[test]
fn cpython_test_manifest_builtin_test_breakpoint_method_audit_is_classified() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let expected_methods = python_test_class_method_names(&source, "TestBreakpoint")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let methods =
        method_audit_methods("## `Lib/test/test_builtin.py::TestBreakpoint` Method Audit");
    let actual_methods = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual_methods, expected_methods,
        "TestBreakpoint method audit drifted"
    );

    let expected_statuses = BTreeMap::from([
        ("test_breakpoint", "partial"),
        ("test_breakpoint_with_breakpointhook_set", "ported_public"),
        ("test_breakpoint_with_breakpointhook_reset", "partial"),
        ("test_breakpoint_with_args_and_keywords", "ported_public"),
        ("test_breakpoint_with_passthru_error", "ported_public"),
        ("test_envar_good_path_builtin", "blocked_by_runtime"),
        ("test_envar_good_path_other", "blocked_by_runtime"),
        ("test_envar_good_path_noop_0", "blocked_by_runtime"),
        ("test_envar_good_path_empty_string", "blocked_by_runtime"),
        ("test_envar_unimportable", "blocked_by_runtime"),
        ("test_envar_ignored_when_hook_is_set", "blocked_by_runtime"),
        ("test_runtime_error_when_hook_is_lost", "ported_public"),
    ]);
    let actual_statuses = methods
        .iter()
        .map(|method| (method.method, method.status))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_statuses, expected_statuses,
        "TestBreakpoint audit statuses drifted"
    );
}

#[test]
fn cpython_test_manifest_builtin_pty_tests_method_audit_is_classified() {
    assert_builtin_method_audit_status_matches_current_source(
        "PtyTests",
        "## `Lib/test/test_builtin.py::PtyTests` Method Audit",
        "blocked_by_runtime",
    );
}

#[test]
fn cpython_test_manifest_builtin_shutdown_test_method_audit_is_classified() {
    assert_builtin_method_audit_status_matches_current_source(
        "ShutdownTest",
        "## `Lib/test/test_builtin.py::ShutdownTest` Method Audit",
        "blocked_by_cpython_internal",
    );
}

#[test]
fn cpython_test_manifest_builtin_immortal_tests_method_audit_is_classified() {
    assert_builtin_method_audit_status_matches_current_source(
        "ImmortalTests",
        "## `Lib/test/test_builtin.py::ImmortalTests` Method Audit",
        "blocked_by_cpython_internal",
    );
}

#[test]
fn cpython_test_manifest_builtin_test_type_method_audit_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let expected = python_test_class_method_names(&source, "TestType");
    let methods = method_audit_methods("## `Lib/test/test_builtin.py::TestType` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestType method audit row count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        8,
        "ported TestType method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        2,
        "partial TestType method count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "partial")),
        "TestType method audit contains an unknown status"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "TestType method audit drifted");
}

#[test]
fn cpython_test_type_name_doc_diff_evidence_is_documented() {
    for (diff_name, subset_name, required_source) in [
        (
            "cpython_type_name_qualname_diff_subset",
            "cpython_type_name_qualname_subset",
            "setattr(A, '__name__', 'A\\0B')",
        ),
        (
            "cpython_type_doc_and_firstlineno_diff_subset",
            "cpython_type_doc_and_firstlineno_subset",
            "A.__firstlineno__ = 43",
        ),
    ] {
        let diff_start = CPYTHON_DIFF
            .find(&format!("fn {diff_name}("))
            .unwrap_or_else(|| panic!("TestType CPython diff evidence `{diff_name}` must exist"));
        let diff_end = CPYTHON_DIFF[diff_start..]
            .find("\n#[test]")
            .map(|offset| diff_start + offset)
            .unwrap_or(CPYTHON_DIFF.len());
        let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

        assert!(
            diff_source.contains(required_source),
            "TestType diff evidence `{diff_name}` must cover `{required_source}`"
        );
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
            "TestType runtime subset evidence `{subset_name}` must exist"
        );
        for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION, MANIFEST] {
            assert!(
                document.contains(diff_name) && document.contains(subset_name),
                "TestType docs must link `{diff_name}` to `{subset_name}`"
            );
        }
    }
}

#[test]
fn cpython_test_manifest_ast_group_counts_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
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
fn cpython_test_manifest_ast_tests_method_audit_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "AST_Tests");
    let methods =
        method_audit_methods("## `Lib/test/test_ast/test_ast.py::AST_Tests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "AST_Tests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| {
            matches!(
                method.status,
                "ported"
                    | "partial"
                    | "blocked_by_runtime"
                    | "blocked_by_ast_module"
                    | "blocked_by_cpython_internal"
                    | "not_started"
            )
        }),
        "AST_Tests method audit contains an unknown status"
    );

    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).collect::<Vec<_>>();
    let extra = actual.difference(&expected).collect::<Vec<_>>();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "AST_Tests method audit drifted; missing={missing:?}; extra={extra:?}"
    );
}

#[test]
fn cpython_test_manifest_lazy_import_test_method_audit_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "LazyImportTest");
    let methods =
        method_audit_methods("## `Lib/test/test_ast/test_ast.py::LazyImportTest` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "LazyImportTest method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| {
            matches!(
                method.status,
                "ported"
                    | "partial"
                    | "blocked_by_runtime"
                    | "blocked_by_ast_module"
                    | "blocked_by_cpython_internal"
                    | "not_started"
            )
        }),
        "LazyImportTest method audit contains an unknown status"
    );

    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).collect::<Vec<_>>();
    let extra = actual.difference(&expected).collect::<Vec<_>>();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "LazyImportTest method audit drifted; missing={missing:?}; extra={extra:?}"
    );
}

#[test]
fn cpython_test_manifest_copy_tests_method_audit_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "CopyTests");
    let methods =
        method_audit_methods("## `Lib/test/test_ast/test_ast.py::CopyTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "CopyTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| {
            matches!(
                method.status,
                "ported"
                    | "partial"
                    | "blocked_by_runtime"
                    | "blocked_by_ast_module"
                    | "blocked_by_cpython_internal"
                    | "not_started"
            )
        }),
        "CopyTests method audit contains an unknown status"
    );

    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).collect::<Vec<_>>();
    let extra = actual.difference(&expected).collect::<Vec<_>>();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "CopyTests method audit drifted; missing={missing:?}; extra={extra:?}"
    );
}

#[test]
fn cpython_test_manifest_node_transformer_tests_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "NodeTransformerTests");
    let methods = method_audit_methods(
        "## `Lib/test/test_ast/test_ast.py::NodeTransformerTests` Method Audit",
    );

    assert_eq!(
        methods.len(),
        expected.len(),
        "NodeTransformerTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "NodeTransformerTests methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "NodeTransformerTests method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_constant_tests_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "ConstantTests");
    let methods =
        method_audit_methods("## `Lib/test/test_ast/test_ast.py::ConstantTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "ConstantTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "ConstantTests methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "ConstantTests method audit drifted");
}

#[test]
fn cpython_test_manifest_end_position_tests_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "EndPositionTests");
    let methods =
        method_audit_methods("## `Lib/test/test_ast/test_ast.py::EndPositionTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "EndPositionTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "EndPositionTests methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "EndPositionTests method audit drifted");
}

#[test]
fn cpython_test_manifest_ast_constructor_tests_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "ASTConstructorTests");
    let methods = method_audit_methods(
        "## `Lib/test/test_ast/test_ast.py::ASTConstructorTests` Method Audit",
    );

    assert_eq!(
        methods.len(),
        expected.len(),
        "ASTConstructorTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "ASTConstructorTests methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "ASTConstructorTests method audit drifted");
}

#[test]
fn cpython_test_manifest_module_state_tests_method_audit_is_classified() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "ModuleStateTests");
    let methods =
        method_audit_methods("## `Lib/test/test_ast/test_ast.py::ModuleStateTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "ModuleStateTests method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| method.status == "blocked_by_ast_module"),
        "ModuleStateTests methods should stay classified as blocked_by_ast_module"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "ModuleStateTests method audit drifted");
}

#[test]
fn cpython_test_manifest_command_line_tests_method_audit_is_classified() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "CommandLineTests");
    let methods =
        method_audit_methods("## `Lib/test/test_ast/test_ast.py::CommandLineTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "CommandLineTests method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| method.status == "blocked_by_ast_module"),
        "CommandLineTests methods should stay classified as blocked_by_ast_module"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "CommandLineTests method audit drifted");
}

#[test]
fn cpython_test_manifest_ast_optimization_tests_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_AST_SOURCE);
    let expected = python_test_class_method_names(&source, "ASTOptimizationTests");
    let methods = method_audit_methods(
        "## `Lib/test/test_ast/test_ast.py::ASTOptimizationTests` Method Audit",
    );

    assert_eq!(
        methods.len(),
        expected.len(),
        "ASTOptimizationTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "ASTOptimizationTests methods should all be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(
        actual, expected,
        "ASTOptimizationTests method audit drifted"
    );
}

#[test]
fn cpython_test_manifest_statuses_are_well_formed() {
    for group in manifest_groups() {
        assert!(
            matches!(
                group.status,
                "ported"
                    | "ported_public"
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
fn cpython_test_manifest_ported_public_groups_are_explicitly_classified() {
    let groups = manifest_groups();

    for (source, group) in [
        ("Lib/test/test_compile.py", "TestSourcePositions"),
        ("Lib/test/test_collections.py", "TestNamedTuple"),
        ("Lib/test/test_collections.py", "TestCollectionABCs"),
        ("Lib/test/test_types.py", "UnionTests"),
    ] {
        assert_manifest_group_status(&groups, source, group, "ported_public");
    }
}

#[test]
fn cpython_migration_sandbox_stdlib_manifest_is_guarded_by_diff_evidence() {
    let rows = sandbox_stdlib_rows();
    let actual_modules = rows
        .iter()
        .map(|row| row.module.as_str())
        .collect::<BTreeSet<_>>();
    let expected_modules = [
        "builtins",
        "sys",
        "types",
        "collections / collections.abc",
        "math / math.integer",
        "array",
        "copy",
        "io.BytesIO",
        "operator",
        "functools",
        "itertools",
        "json",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();

    assert_eq!(
        actual_modules, expected_modules,
        "sandbox stdlib manifest modules drifted"
    );

    for row in rows {
        assert!(
            !row.supported_surface.is_empty(),
            "sandbox stdlib row `{}` must document its supported surface",
            row.module
        );
        assert!(
            !row.excluded_surface.is_empty(),
            "sandbox stdlib row `{}` must document its excluded surface",
            row.module
        );

        let evidence_names = backtick_tokens(row.diff_evidence);
        assert!(
            !evidence_names.is_empty(),
            "sandbox stdlib row `{}` must cite concrete cpython_diff evidence",
            row.module
        );
        let has_direct_diff_evidence = evidence_names
            .iter()
            .any(|evidence| evidence.starts_with("cpython_"));
        assert!(
            has_direct_diff_evidence,
            "sandbox stdlib row `{}` must cite at least one direct CPython diff evidence test",
            row.module
        );

        for evidence in evidence_names {
            let function_name = evidence.replace('-', "_");
            assert!(
                !evidence.contains('*'),
                "sandbox stdlib row `{}` must cite concrete evidence, not wildcard `{evidence}`",
                row.module
            );
            assert!(
                CPYTHON_DIFF.contains(evidence) || CPYTHON_DIFF.contains(&function_name),
                "sandbox stdlib row `{}` cites missing cpython_diff evidence `{evidence}`",
                row.module
            );
            assert!(
                sandbox_stdlib_evidence_has_runtime_subset(evidence),
                "sandbox stdlib row `{}` cites evidence `{evidence}` without matching runtime subset evidence",
                row.module
            );
        }
    }
}

fn sandbox_stdlib_evidence_has_runtime_subset(evidence: &str) -> bool {
    let candidates = sandbox_stdlib_runtime_subset_candidates(evidence);
    candidates
        .iter()
        .any(|candidate| CPYTHON_SUBSET.contains(candidate) || LANGUAGE_TESTS.contains(candidate))
}

fn sandbox_stdlib_runtime_subset_candidates(evidence: &str) -> Vec<String> {
    let snake_case = evidence.replace('-', "_");
    let mut candidates = Vec::new();

    if let Some(stripped) = snake_case.strip_suffix("_diff_subset") {
        candidates.push(format!("{stripped}_subset"));
        candidates.push(format!("{stripped}_methods_subset"));
    } else {
        candidates.push(snake_case.clone());
    }
    if !snake_case.starts_with("cpython_") {
        candidates.push(format!("cpython_{snake_case}_subset"));
    }
    if evidence == "cpython_itertools_core_diff_subset" {
        candidates.push("cpython_itertools_core_iterator_subset".to_string());
    }
    if evidence == "cpython_itertools_keyword_error_diff_subset" {
        candidates.push("cpython_itertools_keyword_error_subset".to_string());
    }
    if evidence == "cpython_itertools_pairwise_diff_subset" {
        candidates.push("cpython_itertools_pairwise_subset".to_string());
    }
    if evidence == "cpython_itertools_product_diff_subset" {
        candidates.push("cpython_itertools_product_subset".to_string());
    }
    if evidence == "cpython_itertools_combinations_diff_subset" {
        candidates.push("cpython_itertools_combinations_subset".to_string());
    }
    if evidence == "cpython_itertools_combinations_with_replacement_diff_subset" {
        candidates.push("cpython_itertools_combinations_with_replacement_subset".to_string());
    }
    if evidence == "cpython_itertools_permutations_diff_subset" {
        candidates.push("cpython_itertools_permutations_subset".to_string());
    }
    if evidence == "cpython_itertools_tee_diff_subset" {
        candidates.push("cpython_itertools_tee_subset".to_string());
    }
    if evidence == "cpython_itertools_batched_diff_subset" {
        candidates.push("cpython_itertools_batched_subset".to_string());
    }
    if evidence == "cpython_itertools_groupby_diff_subset" {
        candidates.push("cpython_itertools_groupby_subset".to_string());
    }
    if evidence == "cpython_json_loads_dumps_diff_subset" {
        candidates.push("cpython_json_loads_dumps_basic_subset".to_string());
    }
    if evidence == "cpython_builtin_singleton_construction_and_attributes_diff_subset" {
        candidates.push("cpython_builtin_construct_singletons_subset".to_string());
        candidates.push("cpython_builtin_singleton_attribute_access_subset".to_string());
    }
    if evidence == "cpython_hash_id_builtins_diff_subset" {
        candidates.push("cpython_hash_builtin_subset".to_string());
        candidates.push("cpython_id_builtin_subset".to_string());
    }
    if evidence == "cpython_array_one_byte_public_clear_diff_subset" {
        candidates.push("cpython_array_one_byte_public_mutation_methods_subset".to_string());
    }
    if evidence == "cpython_types_simple_namespace_recursive_diff_subset" {
        candidates.push("cpython_types_simple_namespace_recursive_and_replace_subset".to_string());
    }
    if evidence == "cpython_types_simple_namespace_state_order_diff_subset" {
        candidates.push("cpython_types_simple_namespace_remaining_public_subset".to_string());
    }
    if evidence == "cpython_types_simple_namespace_fake_comparison_diff_subset" {
        candidates.push("cpython_types_simple_namespace_remaining_public_subset".to_string());
    }

    candidates
}

#[test]
fn cpython_coverage_links_sandbox_stdlib_scope_to_manifest() {
    for required in [
        "Sandbox Stdlib Manifest",
        "tests/cpython_migration.md",
        "cpython_diff",
        "cpython_subset",
        "runtime guard evidence",
        "builtins",
        "sys",
        "types",
        "collections",
        "math",
        "array",
        "copy",
        "io.BytesIO",
        "operator",
        "functools",
        "itertools",
        "json",
        "Runtime Compatibility Module Registry",
        "src/stdlib.rs::create_module()",
        "sandbox_policy_denies_stdlib_imports",
        "sandbox_policy_denies_required_sandbox_stdlib_surface",
        "sandbox_policy_allows_required_sandbox_stdlib_surface",
        "sandbox_policy_required_stdlib_allow_list_excludes_compatibility_shims",
        "sandbox_policy_requires_explicit_allow_for_extra_stdlib_shims",
        "stdlib_create_module_registry_is_classified_by_scope",
    ] {
        assert!(
            CPYTHON_COVERAGE.contains(required),
            "coverage document must mention sandbox stdlib scope term `{required}`"
        );
    }
}

#[test]
fn cpython_coverage_mentions_all_sandbox_stdlib_diff_evidence() {
    for row in sandbox_stdlib_rows() {
        for evidence in backtick_tokens(row.diff_evidence) {
            assert!(
                CPYTHON_COVERAGE.contains(evidence),
                "coverage document must mention sandbox stdlib evidence `{evidence}` from row `{}`",
                row.module
            );
        }
    }
}

#[test]
fn cpython_coverage_mentions_all_sandbox_stdlib_runtime_evidence() {
    let mut missing = Vec::new();

    for row in sandbox_stdlib_rows() {
        for evidence in backtick_tokens(row.diff_evidence) {
            let candidates = sandbox_stdlib_runtime_subset_candidates(evidence);
            if !candidates
                .iter()
                .any(|candidate| CPYTHON_COVERAGE.contains(candidate))
            {
                missing.push(format!(
                    "{}: `{evidence}` expects one of {:?}",
                    row.module, candidates
                ));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "coverage document must mention runtime subset evidence for every sandbox stdlib evidence:\n{}",
        missing.join("\n")
    );
}

fn assert_sandbox_manifest_subset_evidence(
    module: &str,
    required_evidence: &[&str],
    excluded_terms: &[&str],
) {
    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == module)
        .unwrap_or_else(|| panic!("sandbox stdlib manifest must include {module}"));

    for required in required_evidence {
        assert!(
            row.supported_surface.contains(required),
            "{module} sandbox manifest must list runtime subset evidence `{required}`"
        );
        assert!(
            CPYTHON_COVERAGE.contains(required),
            "coverage document must describe {module} runtime subset evidence `{required}`"
        );
    }

    for excluded in excluded_terms {
        assert!(
            row.excluded_surface.contains(excluded),
            "{module} sandbox manifest must keep `{excluded}` outside the default surface"
        );
    }
}

#[test]
fn functools_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "functools",
        &[
            "cpython_functools_public_helpers_subset",
            "cpython_functools_partial_subset",
            "cpython_functools_partialmethod_subset",
            "cpython_functools_cmp_to_key_subset",
            "cpython_functools_update_wrapper_wraps_subset",
            "cpython_functools_total_ordering_subset",
            "cpython_functools_cache_subset",
            "cpython_functools_cached_property_subset",
            "cpython_functools_reduce_subset",
            "cpython_functools_singledispatch_subset",
            "cpython_functools_singledispatchmethod_subset",
        ],
        &[],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "functools")
        .expect("sandbox stdlib manifest must include functools");
    for evidence in [
        "cpython_functools_public_helpers_diff_subset",
        "cpython_functools_partial_diff_subset",
        "cpython_functools_partialmethod_diff_subset",
        "cpython_functools_cmp_to_key_diff_subset",
        "cpython_functools_update_wrapper_wraps_diff_subset",
        "cpython_functools_total_ordering_diff_subset",
        "cpython_functools_cache_diff_subset",
        "cpython_functools_cached_property_diff_subset",
        "cpython_functools_reduce_diff_subset",
        "cpython_functools_singledispatch_diff_subset",
        "cpython_functools_singledispatchmethod_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "functools sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn functools_descriptor_helpers_diff_cover_runtime_subsets() {
    for (subset, diff) in [
        (
            "cpython_functools_partialmethod_subset",
            "cpython_functools_partialmethod_diff_subset",
        ),
        (
            "cpython_functools_cached_property_subset",
            "cpython_functools_cached_property_diff_subset",
        ),
        (
            "cpython_functools_singledispatchmethod_subset",
            "cpython_functools_singledispatchmethod_diff_subset",
        ),
    ] {
        assert!(
            CPYTHON_SUBSET.contains(subset),
            "functools descriptor runtime subset evidence `{subset}` must exist"
        );
        assert!(
            CPYTHON_DIFF.contains(diff),
            "functools descriptor CPython diff evidence `{diff}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(subset) && CPYTHON_COVERAGE.contains(diff),
            "coverage document must link functools descriptor evidence `{subset}` / `{diff}`"
        );
        assert!(
            CPYTHON_MIGRATION.contains(subset),
            "migration document must describe functools descriptor subset `{subset}`"
        );
    }

    let partialmethod_diff = CPYTHON_DIFF
        .split("fn cpython_functools_partialmethod_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_functools_cached_property_diff_subset()")
                .next()
        })
        .expect("functools partialmethod diff evidence must be extractable");
    for required in [
        "partialmethod(staticmethod(capture), 8)",
        "partialmethod(classmethod(capture), d=9)",
        "A.both(a, 5, c=6)",
        "hasattr(a.both, '__self__')",
    ] {
        assert!(
            partialmethod_diff.contains(required),
            "partialmethod diff evidence must cover descriptor binding detail `{required}`"
        );
    }

    let cached_property_diff = CPYTHON_DIFF
        .split("fn cpython_functools_cached_property_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_functools_cache_diff_subset()")
                .next()
        })
        .expect("functools cached_property diff evidence must be extractable");
    for required in [
        "CachedCostItem.cost.__doc__",
        "CachedCostItem.__dict__['cost'].__dict__",
        "Dynamic = type('Dynamic', (), {'field': DynamicDescriptor()})",
        "def __set_name__(self, owner, name):",
    ] {
        assert!(
            cached_property_diff.contains(required),
            "cached_property diff evidence must cover descriptor detail `{required}`"
        );
    }

    let singledispatchmethod_diff = CPYTHON_DIFF
        .split("fn cpython_functools_singledispatchmethod_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn assert_cpython_itertools_core_iterator_diff")
                .next()
        })
        .expect("functools singledispatchmethod diff evidence must be extractable");
    for required in [
        "descriptor = C.__dict__['m']",
        "descriptor.func.__name__",
        "@staticmethod",
        "@classmethod",
        "@c.m.register(bytes)",
    ] {
        assert!(
            singledispatchmethod_diff.contains(required),
            "singledispatchmethod diff evidence must cover descriptor detail `{required}`"
        );
    }
}

#[test]
fn itertools_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "itertools",
        &[
            "cpython_itertools_core_iterator_subset",
            "cpython_itertools_keyword_error_subset",
            "cpython_itertools_pairwise_subset",
            "cpython_itertools_product_subset",
            "cpython_itertools_combinations_subset",
            "cpython_itertools_combinations_with_replacement_subset",
            "cpython_itertools_permutations_subset",
            "cpython_itertools_tee_subset",
            "cpython_itertools_batched_subset",
            "cpython_itertools_groupby_subset",
            "cpython_itertools_repr_subset",
        ],
        &[],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "itertools")
        .expect("sandbox stdlib manifest must include itertools");
    for evidence in [
        "cpython_itertools_core_diff_subset",
        "cpython_itertools_core_iterator_diff_subset",
        "cpython_itertools_keyword_error_diff_subset",
        "cpython_itertools_pairwise_diff_subset",
        "cpython_itertools_product_diff_subset",
        "cpython_itertools_combinations_diff_subset",
        "cpython_itertools_combinations_with_replacement_diff_subset",
        "cpython_itertools_permutations_diff_subset",
        "cpython_itertools_tee_diff_subset",
        "cpython_itertools_batched_diff_subset",
        "cpython_itertools_groupby_diff_subset",
        "cpython_itertools_repr_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "itertools sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn itertools_core_and_pairwise_runtime_evidence_stay_split() {
    let core_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_core_iterator_subset()")
        .expect("itertools core runtime subset evidence must exist");
    let keyword_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_keyword_error_subset()")
        .expect("itertools keyword error runtime subset evidence must exist");
    let pairwise_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_pairwise_subset()")
        .expect("itertools pairwise runtime subset evidence must exist");
    let product_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_product_subset()")
        .expect("itertools product runtime subset evidence must exist");
    let combinations_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_combinations_subset()")
        .expect("itertools combinations runtime subset evidence must exist");
    let replacement_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_combinations_with_replacement_subset()")
        .expect("itertools combinations_with_replacement runtime subset evidence must exist");
    let permutations_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_permutations_subset()")
        .expect("itertools permutations runtime subset evidence must exist");
    let tee_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_tee_subset()")
        .expect("itertools tee runtime subset evidence must exist");
    let batched_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_batched_subset()")
        .expect("itertools batched runtime subset evidence must exist");
    let groupby_start = CPYTHON_SUBSET
        .find("fn cpython_itertools_groupby_subset()")
        .expect("itertools groupby runtime subset evidence must exist");
    let groupby_end = CPYTHON_SUBSET[groupby_start..]
        .find("\n// Adapted from CPython Lib/test/test_list.py")
        .map(|offset| groupby_start + offset)
        .expect("itertools groupby subset must end before sequence constructor tests");

    let core_source = &CPYTHON_SUBSET[core_start..keyword_start];
    let keyword_source = &CPYTHON_SUBSET[keyword_start..pairwise_start];
    let pairwise_source = &CPYTHON_SUBSET[pairwise_start..product_start];
    let product_source = &CPYTHON_SUBSET[product_start..combinations_start];
    let combinations_source = &CPYTHON_SUBSET[combinations_start..replacement_start];
    let replacement_source = &CPYTHON_SUBSET[replacement_start..permutations_start];
    let permutations_source = &CPYTHON_SUBSET[permutations_start..tee_start];
    let tee_source = &CPYTHON_SUBSET[tee_start..batched_start];
    let batched_source = &CPYTHON_SUBSET[batched_start..groupby_start];
    let groupby_source = &CPYTHON_SUBSET[groupby_start..groupby_end];

    assert!(
        !core_source.contains("pairwise"),
        "itertools core runtime evidence must not cover pairwise()"
    );
    assert!(
        keyword_source.contains("multiple values"),
        "itertools keyword-error runtime evidence must assert duplicate keyword diagnostics"
    );
    assert!(
        pairwise_source.contains("itertools.pairwise"),
        "itertools pairwise runtime evidence must cover pairwise()"
    );
    assert!(
        product_source.contains("itertools.product"),
        "itertools product runtime evidence must cover product()"
    );
    assert!(
        combinations_source.contains("itertools.combinations"),
        "itertools combinations runtime evidence must cover combinations()"
    );
    assert!(
        replacement_source.contains("itertools.combinations_with_replacement"),
        "itertools combinations_with_replacement runtime evidence must cover combinations_with_replacement()"
    );
    assert!(
        permutations_source.contains("itertools.permutations"),
        "itertools permutations runtime evidence must cover permutations()"
    );
    assert!(
        tee_source.contains("itertools.tee"),
        "itertools tee runtime evidence must cover tee()"
    );
    assert!(
        batched_source.contains("itertools.batched"),
        "itertools batched runtime evidence must cover batched()"
    );
    assert!(
        groupby_source.contains("itertools.groupby"),
        "itertools groupby runtime evidence must cover groupby()"
    );
}

#[test]
fn itertools_core_and_pairwise_diff_evidence_stay_split() {
    let core_start = CPYTHON_DIFF
        .find("fn cpython_itertools_core_diff_subset()")
        .expect("itertools core diff evidence must exist");
    let pairwise_start = CPYTHON_DIFF
        .find("fn cpython_itertools_pairwise_diff_subset()")
        .expect("itertools pairwise diff evidence must exist");
    let keyword_start = CPYTHON_DIFF
        .find("fn cpython_itertools_keyword_error_diff_subset()")
        .expect("itertools keyword-error diff evidence must exist");
    let product_start = CPYTHON_DIFF
        .find("fn cpython_itertools_product_diff_subset()")
        .expect("itertools product diff evidence must exist");
    let combinations_start = CPYTHON_DIFF
        .find("fn cpython_itertools_combinations_diff_subset()")
        .expect("itertools combinations diff evidence must exist");
    let replacement_start = CPYTHON_DIFF
        .find("fn cpython_itertools_combinations_with_replacement_diff_subset()")
        .expect("itertools combinations_with_replacement diff evidence must exist");
    let permutations_start = CPYTHON_DIFF
        .find("fn cpython_itertools_permutations_diff_subset()")
        .expect("itertools permutations diff evidence must exist");
    let tee_start = CPYTHON_DIFF
        .find("fn cpython_itertools_tee_diff_subset()")
        .expect("itertools tee diff evidence must exist");
    let batched_start = CPYTHON_DIFF
        .find("fn cpython_itertools_batched_diff_subset()")
        .expect("itertools batched diff evidence must exist");
    let groupby_start = CPYTHON_DIFF
        .find("fn cpython_itertools_groupby_diff_subset()")
        .expect("itertools groupby diff evidence must exist");
    let repr_start = CPYTHON_DIFF
        .find("fn cpython_itertools_repr_diff_subset()")
        .expect("itertools repr diff evidence must exist");
    let repr_end = CPYTHON_DIFF[repr_start..]
        .find("\n// Differential smoke tests")
        .map(|offset| repr_start + offset)
        .expect("itertools repr diff subset must end before smoke tests");

    let core_source = &CPYTHON_DIFF[core_start..keyword_start];
    let keyword_source = &CPYTHON_DIFF[keyword_start..pairwise_start];
    let pairwise_source = &CPYTHON_DIFF[pairwise_start..product_start];
    let product_source = &CPYTHON_DIFF[product_start..combinations_start];
    let combinations_source = &CPYTHON_DIFF[combinations_start..replacement_start];
    let replacement_source = &CPYTHON_DIFF[replacement_start..permutations_start];
    let permutations_source = &CPYTHON_DIFF[permutations_start..tee_start];
    let tee_source = &CPYTHON_DIFF[tee_start..batched_start];
    let batched_source = &CPYTHON_DIFF[batched_start..groupby_start];
    let groupby_source = &CPYTHON_DIFF[groupby_start..repr_start];
    let repr_source = &CPYTHON_DIFF[repr_start..repr_end];

    assert!(
        !core_source.contains("pairwise"),
        "itertools core CPython diff evidence must not cover pairwise()"
    );
    assert!(
        keyword_source.contains("multiple values"),
        "itertools keyword-error CPython diff evidence must assert duplicate keyword diagnostics"
    );
    assert!(
        pairwise_source.contains("itertools.pairwise"),
        "itertools pairwise CPython diff evidence must cover pairwise()"
    );
    assert!(
        pairwise_source.contains("hasattr(itertools, 'pairwise')")
            && pairwise_source.contains("skipping itertools.pairwise diff"),
        "itertools pairwise CPython diff evidence must stay gated for older CPython oracles"
    );
    assert!(
        product_source.contains("itertools.product"),
        "itertools product CPython diff evidence must cover product()"
    );
    assert!(
        combinations_source.contains("itertools.combinations"),
        "itertools combinations CPython diff evidence must cover combinations()"
    );
    assert!(
        replacement_source.contains("itertools.combinations_with_replacement"),
        "itertools combinations_with_replacement CPython diff evidence must cover combinations_with_replacement()"
    );
    assert!(
        permutations_source.contains("itertools.permutations"),
        "itertools permutations CPython diff evidence must cover permutations()"
    );
    assert!(
        tee_source.contains("itertools.tee"),
        "itertools tee CPython diff evidence must cover tee()"
    );
    assert!(
        batched_source.contains("itertools.batched"),
        "itertools batched CPython diff evidence must cover batched()"
    );
    assert!(
        groupby_source.contains("itertools.groupby"),
        "itertools groupby CPython diff evidence must cover groupby()"
    );
    assert!(
        repr_source.contains("repr("),
        "itertools repr CPython diff evidence must cover public repr() behavior"
    );
}

#[test]
fn sandbox_stdlib_subset_without_same_named_diff_is_explicitly_classified() {
    let sandbox_prefixes = [
        "cpython_array_",
        "cpython_collections_",
        "cpython_copy_",
        "cpython_functools_",
        "cpython_io_",
        "cpython_itertools_",
        "cpython_json_",
        "cpython_operator_",
    ];

    let subset_names = rust_test_names(CPYTHON_SUBSET)
        .into_iter()
        .filter(|name| {
            sandbox_prefixes
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .filter_map(|name| name.strip_suffix("_subset").map(str::to_string))
        .collect::<BTreeSet<_>>();
    let diff_names = rust_test_names(CPYTHON_DIFF)
        .into_iter()
        .filter_map(|name| name.strip_suffix("_diff_subset").map(str::to_string))
        .collect::<BTreeSet<_>>();

    let missing_same_named_diff = subset_names
        .difference(&diff_names)
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected = [
        "cpython_collections_chainmap_copy_pickle_eval_identity",
        "cpython_collections_namedtuple_pickle",
        "cpython_operator_pickle_helper",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<BTreeSet<_>>();
    assert_eq!(
        missing_same_named_diff, expected,
        "sandbox stdlib subset evidence without same-named CPython diff must be explicitly classified"
    );

    for subset in [
        "cpython_operator_pickle_helper_subset",
        "cpython_collections_chainmap_copy_pickle_eval_identity_subset",
        "cpython_collections_namedtuple_pickle_subset",
    ] {
        assert!(
            CPYTHON_COVERAGE.contains(subset) && CPYTHON_MIGRATION.contains(subset),
            "pickle/eval identity subset `{subset}` must stay documented as subset-only support"
        );
    }
}

#[test]
fn json_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "json",
        &[
            "cpython_json_loads_dumps_basic_subset",
            "cpython_json_keyword_argument_binding_subset",
            "cpython_json_loads_escape_and_duplicate_key_subset",
            "cpython_json_loads_unicode_escape_roundtrip_subset",
            "cpython_json_loads_strict_subset",
            "cpython_json_dumps_string_escape_subset",
            "cpython_json_dumps_key_coercion_subset",
            "cpython_json_dumps_sequence_subclass_iter_subset",
            "cpython_json_dumps_allow_nan_subset",
            "cpython_json_dumps_check_circular_subset",
            "cpython_json_dumps_ensure_ascii_subset",
            "cpython_json_dumps_indent_subset",
            "cpython_json_dumps_skipkeys_subset",
            "cpython_json_dumps_sort_keys_subset",
            "cpython_json_dumps_separators_subset",
            "cpython_json_dumps_float_spelling_subset",
            "cpython_json_loads_number_and_whitespace_subset",
            "cpython_json_loads_int_digit_limit_subset",
            "cpython_json_loads_top_level_scalar_and_empty_container_subset",
            "cpython_json_loads_nonfinite_constants_subset",
            "cpython_json_loads_dumps_error_boundary_subset",
            "cpython_json_loads_string_error_boundary_subset",
        ],
        &[
            "object_hook",
            "object_pairs_hook",
            "parse_float",
            "parse_int",
            "parse_constant",
            "default",
            "cls",
            "File APIs",
            "JSONDecodeError",
            "full `JSONDecodeError` compatibility",
        ],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "json")
        .expect("sandbox stdlib manifest must include json");
    for evidence in [
        "cpython_json_loads_dumps_diff_subset",
        "cpython_json_loads_dumps_basic_diff_subset",
        "cpython_json_keyword_argument_binding_diff_subset",
        "cpython_json_loads_escape_and_duplicate_key_diff_subset",
        "cpython_json_loads_unicode_escape_roundtrip_diff_subset",
        "cpython_json_loads_strict_diff_subset",
        "cpython_json_dumps_string_escape_diff_subset",
        "cpython_json_dumps_key_coercion_diff_subset",
        "cpython_json_dumps_sequence_subclass_iter_diff_subset",
        "cpython_json_dumps_allow_nan_diff_subset",
        "cpython_json_dumps_check_circular_diff_subset",
        "cpython_json_dumps_ensure_ascii_diff_subset",
        "cpython_json_dumps_indent_diff_subset",
        "cpython_json_dumps_skipkeys_diff_subset",
        "cpython_json_dumps_sort_keys_diff_subset",
        "cpython_json_dumps_separators_diff_subset",
        "cpython_json_dumps_float_spelling_diff_subset",
        "cpython_json_loads_number_and_whitespace_diff_subset",
        "cpython_json_loads_int_digit_limit_diff_subset",
        "cpython_json_loads_top_level_scalar_and_empty_container_diff_subset",
        "cpython_json_loads_nonfinite_constants_diff_subset",
        "cpython_json_loads_dumps_error_boundary_diff_subset",
        "cpython_json_loads_string_error_boundary_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "json sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn json_loads_dumps_basic_diff_covers_core_runtime_subset() {
    let diff_name = "cpython_json_loads_dumps_diff_subset";
    let direct_diff_name = "cpython_json_loads_dumps_basic_diff_subset";
    let helper_name = "assert_cpython_json_loads_dumps_basic_diff";
    let subset_name = "cpython_json_loads_dumps_basic_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "json loads/dumps direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_DIFF.contains(&format!("fn {direct_diff_name}(")),
        "json loads/dumps same-named direct CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "json loads/dumps runtime subset evidence must exist"
    );

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains(direct_diff_name)
                && document.contains(subset_name),
            "json docs must link `{diff_name}` / `{direct_diff_name}` to `{subset_name}`"
        );
        assert!(
            document.contains("UTF-16")
                && document.contains("UTF-32")
                && document.contains("IntEnum")
                && document.contains("namedtuple")
                && document.contains("non-finite"),
            "json docs must describe the core encoded-input, subclass/container, and non-finite value surface"
        );
    }

    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {helper_name}("))
        .expect("json loads/dumps shared diff helper must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for required in [
        "json.loads(b'",
        "json.loads(bytearray",
        "u16le",
        "u32le",
        "IntEnum",
        "namedtuple",
        "json.dumps(float('nan'))",
    ] {
        assert!(
            diff_source.contains(required),
            "json loads/dumps diff evidence must cover `{required}`"
        );
    }
}

#[test]
fn json_hook_boundaries_stay_sandbox_classified() {
    let diff_name = "cpython_json_keyword_argument_binding_diff_subset";
    let subset_name = "cpython_json_keyword_argument_binding_subset";

    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "json keyword binding CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "json keyword binding local subset evidence must exist"
    );

    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("json keyword binding diff evidence must be extractable");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];
    for required in [
        "cls=None",
        "object_hook=None",
        "parse_float=None",
        "parse_int=None",
        "parse_constant=None",
        "object_pairs_hook=None",
        "default=None",
    ] {
        assert!(
            diff_source.contains(required),
            "json CPython diff evidence must cover supported None hook keyword `{required}`"
        );
    }

    let subset_start = CPYTHON_SUBSET
        .find(&format!("fn {subset_name}("))
        .expect("json keyword binding subset evidence must be extractable");
    let subset_end = CPYTHON_SUBSET[subset_start..]
        .find("\n#[test]")
        .map(|offset| subset_start + offset)
        .unwrap_or(CPYTHON_SUBSET.len());
    let subset_source = &CPYTHON_SUBSET[subset_start..subset_end];
    for required in [
        "loads-object-hook",
        "loads-parse-int",
        "dumps-cls",
        "dumps-default",
        "TypeError True",
    ] {
        assert!(
            subset_source.contains(required),
            "json local subset evidence must keep non-None hook boundary `{required}`"
        );
    }

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "json")
        .expect("sandbox stdlib manifest must include json");
    for excluded in [
        "object_hook",
        "object_pairs_hook",
        "parse_float",
        "parse_int",
        "parse_constant",
        "default",
        "cls",
        "File APIs",
        "full `JSONDecodeError` compatibility",
    ] {
        assert!(
            row.excluded_surface.contains(excluded),
            "json sandbox manifest must keep `{excluded}` outside the supported surface"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("encoder/decoder") && document.contains("hooks"),
            "json docs must keep encoder/decoder hooks outside the default sandbox surface"
        );
        for required in [
            "File APIs",
            "JSONDecodeError",
            "object_hook",
            "parse_int",
            "default",
            "cls",
        ] {
            assert!(
                document.contains(required),
                "json docs must keep sandbox hook/file/error boundary `{required}` documented"
            );
        }
    }
}

#[test]
fn json_error_boundary_diff_covers_subset_surface() {
    for (diff_name, subset_name) in [
        (
            "cpython_json_loads_dumps_error_boundary_diff_subset",
            "cpython_json_loads_dumps_error_boundary_subset",
        ),
        (
            "cpython_json_loads_string_error_boundary_diff_subset",
            "cpython_json_loads_string_error_boundary_subset",
        ),
    ] {
        assert!(
            CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
            "json error-boundary CPython diff evidence `{diff_name}` must exist"
        );
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
            "json error-boundary subset evidence `{subset_name}` must exist"
        );
        for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
            assert!(
                document.contains(diff_name) && document.contains(subset_name),
                "json docs must link `{diff_name}` to `{subset_name}`"
            );
        }
    }

    for required in [
        "loads-memoryview",
        "loads-invalid-utf8",
        "loads-trailing-data",
        "loads-array-trailing-comma",
        "loads-invalid-escape",
        "dumps-bytearray",
        "dumps-memoryview",
        "dumps-list-cycle",
        "dumps-dict-cycle",
        "dumps-namedtuple-cycle",
        "short-unicode-escape",
        "nonhex-unicode-escape",
        "raw-newline",
        "raw-tab",
    ] {
        assert!(
            CPYTHON_DIFF.contains(required) && CPYTHON_SUBSET.contains(required),
            "json error-boundary diff and subset evidence must both cover `{required}`"
        );
    }
}

#[test]
fn operator_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "operator",
        &[
            "cpython_operator_public_helpers_subset",
            "cpython_operator_length_hint_subset",
            "cpython_operator_comparison_predicate_subset",
            "cpython_operator_is_none_predicates_subset",
            "cpython_operator_arithmetic_bitwise_subset",
            "cpython_operator_sequence_member_subset",
            "cpython_operator_callable_helper_subset",
            "cpython_operator_call_helper_subset",
            "cpython_operator_inplace_helper_subset",
            "cpython_operator_module_metadata_subset",
            "cpython_operator_signature_helper_subset",
            "cpython_operator_helper_repr_subset",
        ],
        &["Full pickle metadata"],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "operator")
        .expect("sandbox stdlib manifest must include operator");
    assert!(
        !row.supported_surface
            .contains("cpython_operator_pickle_helper_subset"),
        "operator pickle helper subset must stay outside the default sandbox manifest surface"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_public_helpers_diff_subset"),
        "operator sandbox manifest must cite CPython public helper diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_length_hint_diff_subset"),
        "operator sandbox manifest must cite CPython length_hint diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_comparison_predicate_diff_subset"),
        "operator sandbox manifest must cite CPython comparison/predicate diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_is_none_predicates_diff_subset"),
        "operator sandbox manifest must cite CPython is_none/is_not_none predicate diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_arithmetic_bitwise_diff_subset"),
        "operator sandbox manifest must cite CPython arithmetic/bitwise diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_sequence_member_diff_subset"),
        "operator sandbox manifest must cite CPython sequence/member diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_callable_helper_diff_subset"),
        "operator sandbox manifest must cite CPython callable helper diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_call_helper_diff_subset"),
        "operator sandbox manifest must cite CPython call helper diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_inplace_helper_diff_subset"),
        "operator sandbox manifest must cite CPython inplace helper diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_module_metadata_diff_subset"),
        "operator sandbox manifest must cite CPython module metadata diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_signature_helper_diff_subset"),
        "operator sandbox manifest must cite CPython signature helper diff evidence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_operator_helper_repr_diff_subset"),
        "operator sandbox manifest must cite CPython helper repr diff evidence"
    );
}

#[test]
fn operator_signature_diff_evidence_stays_capability_gated() {
    let start = CPYTHON_DIFF
        .find("fn cpython_operator_signature_helper_diff_subset()")
        .expect("operator signature diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    assert!(
        body.contains("inspect.signature(operator.attrgetter)")
            && body.contains("skipping operator signature helper diff"),
        "operator signature diff evidence must stay gated for older CPython oracles"
    );
}

#[test]
fn operator_newer_helpers_and_pickle_stop_line_stay_classified() {
    for (subset, diff) in [
        (
            "cpython_operator_is_none_predicates_subset",
            "cpython_operator_is_none_predicates_diff_subset",
        ),
        (
            "cpython_operator_call_helper_subset",
            "cpython_operator_call_helper_diff_subset",
        ),
        (
            "cpython_operator_helper_repr_subset",
            "cpython_operator_helper_repr_diff_subset",
        ),
        (
            "cpython_operator_signature_helper_subset",
            "cpython_operator_signature_helper_diff_subset",
        ),
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "operator helper subset evidence `{subset}` must exist"
        );
        assert!(
            CPYTHON_DIFF.contains(&format!("fn {diff}(")),
            "operator helper CPython diff evidence `{diff}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(subset) && CPYTHON_COVERAGE.contains(diff),
            "coverage document must link operator evidence `{subset}` / `{diff}`"
        );
        assert!(
            CPYTHON_MIGRATION.contains(subset),
            "migration document must describe operator subset `{subset}`"
        );
    }

    let is_none_diff = CPYTHON_DIFF
        .split("fn cpython_operator_is_none_predicates_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_operator_arithmetic_bitwise_diff_subset()")
                .next()
        })
        .expect("operator is_none diff evidence must be extractable");
    for required in [
        "hasattr(operator, 'is_none')",
        "skipping operator.is_none diff",
        "operator.is_none(value)",
        "operator.is_not_none(value)",
        "name in operator.__all__",
    ] {
        assert!(
            is_none_diff.contains(required),
            "operator is_none diff evidence must cover `{required}`"
        );
    }

    let call_diff = CPYTHON_DIFF
        .split("fn cpython_operator_call_helper_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_operator_inplace_helper_diff_subset()")
                .next()
        })
        .expect("operator.call diff evidence must be extractable");
    for required in [
        "hasattr(operator, 'call')",
        "skipping operator.call diff",
        "operator.call(func, 0, 1, a=2, obj=3)",
        "operator.__call__ is operator.call",
        "operator.call(func, unknown=1, **{'unknown': 2})",
    ] {
        assert!(
            call_diff.contains(required),
            "operator.call diff evidence must cover `{required}`"
        );
    }

    let repr_diff = CPYTHON_DIFF
        .split("fn cpython_operator_helper_repr_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_functools_partialmethod_diff_subset()")
                .next()
        })
        .expect("operator helper repr diff evidence must be extractable");
    for required in [
        "operator.attrgetter('x', 'y', 't.u.v')",
        "operator.itemgetter(slice(2, 4))",
        "operator.methodcaller('baz', self='eggs', name='spam')",
        "str(helper) == repr(helper)",
    ] {
        assert!(
            repr_diff.contains(required),
            "operator helper repr diff evidence must cover `{required}`"
        );
    }

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "operator")
        .expect("sandbox stdlib manifest must include operator");
    assert!(
        row.excluded_surface.contains("Full pickle metadata"),
        "operator sandbox manifest must keep full pickle metadata outside the supported surface"
    );
    assert!(
        !row.supported_surface
            .contains("cpython_operator_pickle_helper_subset"),
        "operator pickle helper subset must remain outside the default sandbox manifest surface"
    );
    assert!(
        CPYTHON_SUBSET.contains("fn cpython_operator_pickle_helper_subset(")
            && CPYTHON_COVERAGE.contains("cpython_operator_pickle_helper_subset")
            && CPYTHON_MIGRATION.contains("cpython_operator_pickle_helper_subset"),
        "operator pickle helper subset-only evidence must remain documented"
    );
    assert!(
        !CPYTHON_DIFF.contains("fn cpython_operator_pickle_helper_diff_subset("),
        "operator pickle helper must not claim direct CPython diff parity while using MiniPython pickle payloads"
    );
}

#[test]
fn array_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "array",
        &[
            "cpython_array_module_and_constructor_public_surface_subset",
            "cpython_array_subclass_public_construction_subset",
            "cpython_array_one_byte_public_sequence_subset",
            "cpython_array_short_public_sequence_and_mutation_subset",
            "cpython_array_int_public_sequence_and_mutation_subset",
            "cpython_array_long_long_public_sequence_and_mutation_subset",
            "cpython_array_native_long_public_sequence_and_mutation_subset",
            "cpython_array_float_public_sequence_and_mutation_subset",
            "cpython_array_unicode_public_sequence_and_mutation_subset",
            "cpython_array_one_byte_public_mutation_methods_subset",
            "cpython_array_one_byte_public_subscript_mutation_subset",
            "cpython_array_one_byte_public_copy_byteswap_compare_subset",
            "cpython_array_one_byte_public_concat_repeat_subset",
            "cpython_array_one_byte_public_buffer_info_subset",
            "cpython_array_one_byte_public_unicode_method_rejection_subset",
            "cpython_array_one_byte_public_file_methods_subset",
        ],
        &["Real file descriptors"],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "array")
        .expect("sandbox stdlib manifest must include array");
    for evidence in [
        "cpython_array_module_and_constructor_public_surface_diff_subset",
        "cpython_array_subclass_public_construction_diff_subset",
        "cpython_array_one_byte_public_sequence_diff_subset",
        "cpython_array_short_public_sequence_and_mutation_diff_subset",
        "cpython_array_int_public_sequence_and_mutation_diff_subset",
        "cpython_array_long_long_public_sequence_and_mutation_diff_subset",
        "cpython_array_native_long_public_sequence_and_mutation_diff_subset",
        "cpython_array_float_public_sequence_and_mutation_diff_subset",
        "cpython_array_unicode_public_sequence_and_mutation_diff_subset",
        "cpython_array_one_byte_public_mutation_methods_diff_subset",
        "cpython_array_one_byte_public_clear_diff_subset",
        "cpython_array_one_byte_public_subscript_mutation_diff_subset",
        "cpython_array_one_byte_public_copy_byteswap_compare_diff_subset",
        "cpython_array_one_byte_public_concat_repeat_diff_subset",
        "cpython_array_one_byte_public_buffer_info_diff_subset",
        "cpython_array_one_byte_public_unicode_method_rejection_diff_subset",
        "cpython_array_one_byte_public_file_methods_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "array sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn array_clear_diff_evidence_stays_capability_gated() {
    let start = CPYTHON_DIFF
        .find("fn cpython_array_one_byte_public_clear_diff_subset()")
        .expect("array.clear diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    assert!(
        body.contains("hasattr(array.array('B'), 'clear')")
            && body.contains("skipping array.clear diff"),
        "array.clear diff evidence must stay gated for older CPython oracles"
    );
}

#[test]
fn runtime_newer_oracle_diff_evidence_stays_capability_gated() {
    for (function, required) in [
        (
            "fn cpython_memoryview_count_index_diff_subset()",
            &[
                "hasattr(memoryview(b'abc'), 'count')",
                "skipping memoryview.count/index diff",
            ][..],
        ),
        (
            "fn cpython_float_from_number_diff_subset()",
            &[
                "hasattr(float, 'from_number')",
                "skipping float.from_number diff",
            ],
        ),
        (
            "fn cpython_complex_subclass_constructor_and_from_number_diff_subset()",
            &[
                "hasattr(complex, 'from_number')",
                "skipping complex.from_number diff",
            ],
        ),
        (
            "fn cpython_types_simple_namespace_new_and_invalid_replace_diff_subset()",
            &[
                "hasattr(copy, 'replace')",
                "hasattr(types.SimpleNamespace, '__replace__')",
                "skipping SimpleNamespace.__replace__ diff",
            ],
        ),
    ] {
        let start = CPYTHON_DIFF
            .find(function)
            .unwrap_or_else(|| panic!("runtime gated diff evidence `{function}` must exist"));
        let body = &CPYTHON_DIFF[start..];
        let end = body.find("\n#[test]").unwrap_or(body.len());
        let body = &body[..end];

        for text in required {
            assert!(
                body.contains(text),
                "runtime gated diff evidence `{function}` must contain `{text}`"
            );
        }
    }
}

#[test]
fn collections_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "collections / collections.abc",
        &[
            "cpython_collections_counter_basics_subset",
            "cpython_collections_counter_public_subset",
            "cpython_collections_counter_conversions_subset",
            "cpython_collections_counter_init_update_subset",
            "cpython_collections_counter_comparison_subset",
            "cpython_collections_counter_fromkeys_subset",
            "cpython_collections_counter_most_common_subset",
            "cpython_collections_counter_mapping_mutation_subset",
            "cpython_collections_counter_repr_nonsortable_subset",
            "cpython_collections_counter_subtract_unary_subset",
            "cpython_collections_counter_copy_subclass_subset",
            "cpython_collections_counter_copying_subset",
            "cpython_collections_counter_order_preservation_subset",
            "cpython_collections_counter_update_reentrant_add_clears_counter_subset",
            "cpython_collections_counter_helper_function_subset",
            "cpython_collections_counter_multiset_operations_subset",
            "cpython_collections_counter_multiset_operations_matrix_subset",
            "cpython_collections_counter_multiset_operations_equivalent_to_set_operations_subset",
            "cpython_collections_counter_symmetric_difference_subset",
            "cpython_collections_counter_inplace_operations_subset",
            "cpython_collections_counter_inplace_operations_matrix_subset",
            "cpython_collections_chainmap_public_methods_subset",
            "cpython_collections_chainmap_copy_sharing_subset",
            "cpython_collections_namedtuple_factory_instance_subset",
            "cpython_collections_namedtuple_public_subset",
            "cpython_collections_namedtuple_defaults_rename_readonly_subset",
            "cpython_collections_namedtuple_repr_subset",
            "cpython_collections_namedtuple_name_conflicts_subset",
            "cpython_collections_namedtuple_subclass_issue_24931_subset",
            "cpython_collections_namedtuple_match_args_subset",
            "cpython_collections_namedtuple_large_size_subset",
            "cpython_collections_namedtuple_field_doc_subset",
            "cpython_collections_namedtuple_copy_keyword_generic_alias_subset",
            "cpython_collections_namedtuple_new_builtins_issue_43102_subset",
            "cpython_collections_namedtuple_new_builtins_globals_subset",
            "cpython_collections_userdict_userlist_public_subset",
            "cpython_collections_userdict_public_methods_subset",
            "cpython_collections_userlist_public_methods_subset",
            "cpython_collections_userlist_namedtuple_sequence_order_subset",
            "cpython_collections_userstring_protocol_and_userdict_missing_subset",
            "cpython_collections_deque_public_surface_subset",
            "cpython_collections_chainmap_missing_and_first_map_mutation_subset",
            "cpython_collections_chainmap_iter_does_not_call_getitem_subset",
            "cpython_collections_chainmap_new_child_custom_mapping_subset",
            "cpython_collections_chainmap_order_preservation_subset",
            "cpython_collections_chainmap_union_operators_subset",
            "cpython_collections_abc_core_runtime_subset",
            "cpython_collections_abc_iterable_iterator_subset",
            "cpython_collections_abc_iterable_sample_matrix_subset",
            "cpython_collections_abc_iterator_sample_matrix_subset",
            "cpython_collections_abc_sequence_subset",
            "cpython_collections_abc_sequence_mixins_subset",
            "cpython_collections_abc_mapping_subset",
            "cpython_collections_abc_mapping_view_subset",
            "cpython_collections_abc_mutable_sequence_subset",
            "cpython_collections_abc_mapping_mixins_subset",
            "cpython_collections_abc_mapping_mixin_views_subset",
            "cpython_collections_abc_userdict_view_snapshot_subset",
            "cpython_collections_abc_set_mutable_set_mixins_subset",
            "cpython_collections_abc_set_from_iterable_operator_subset",
            "cpython_collections_abc_set_real_set_interoperability_subset",
            "cpython_collections_abc_set_hash_matches_frozenset_subset",
            "cpython_collections_abc_issue26915_identity_first_object_subset",
            "cpython_collections_abc_set_noncomparable_comparison_subset",
            "cpython_collections_abc_reversible_subset",
            "cpython_collections_abc_reversible_direct_subclass_subset",
            "cpython_collections_abc_collection_direct_subclass_subset",
            "cpython_collections_abc_async_runtime_subset",
            "cpython_collections_abc_async_iterator_mixin_subset",
            "cpython_collections_abc_async_generator_core_mixin_subset",
            "cpython_collections_abc_async_generator_throw_close_mixin_subset",
            "cpython_collections_abc_generator_mixin_subset",
            "cpython_collections_abc_generator_sample_matrix_subset",
            "cpython_collections_abc_generator_runtime_subset",
            "cpython_collections_abc_types_coroutine_subset",
            "cpython_collections_abc_coroutine_mixin_subset",
            "cpython_collections_abc_abstract_methods_subset",
            "cpython_collections_abc_validate_isinstance_subset",
            "cpython_collections_abc_direct_subclassing_subset",
            "cpython_collections_abc_hashable_direct_subclass_subset",
            "cpython_collections_abc_registration_subset",
            "cpython_collections_abc_bytestring_buffer_subset",
            "cpython_collections_abc_bytestring_deprecation_warnings_subset",
            "cpython_collections_abc_composite_abstract_methods_subset",
        ],
        &["pickle/eval identity matrices"],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "collections / collections.abc")
        .expect("sandbox stdlib manifest must include collections / collections.abc");
    assert!(
        row.diff_evidence
            .contains("cpython_collections_deque_public_surface_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for deque public surface"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_chainmap_public_methods_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for ChainMap public methods"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_chainmap_copy_sharing_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for ChainMap copy sharing"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_namedtuple_copy_keyword_generic_alias_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for namedtuple copy/generic alias behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_namedtuple_factory_instance_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for namedtuple factory/instance behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_namedtuple_new_builtins_globals_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for namedtuple new builtins globals"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_namedtuple_new_builtins_issue_43102_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for namedtuple new builtins issue 43102"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_namedtuple_match_args_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for namedtuple match args"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_subtract_unary_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter subtract/unary behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_basics_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter basics behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_fromkeys_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter fromkeys behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_most_common_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter most_common behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_mapping_mutation_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter mapping mutation behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_comparison_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter comparison behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_copy_subclass_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter subclass copying"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_copying_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter copying"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_order_preservation_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter order preservation"
    );
    assert!(
        row.diff_evidence.contains(
            "cpython_collections_counter_update_reentrant_add_clears_counter_diff_subset"
        ),
        "collections sandbox manifest must cite CPython diff evidence for Counter reentrant update"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_helper_function_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter helper function"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_multiset_operations_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter multiset operations"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_multiset_operations_matrix_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter multiset matrix"
    );
    assert!(
        row.diff_evidence.contains(
            "cpython_collections_counter_multiset_operations_equivalent_to_set_operations_diff_subset"
        ),
        "collections sandbox manifest must cite CPython diff evidence for Counter set-equivalence matrix"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_symmetric_difference_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter symmetric difference"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_inplace_operations_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter inplace operations"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_counter_inplace_operations_matrix_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Counter inplace matrix"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_core_runtime_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for collections.abc core runtime"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_iterable_iterator_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Iterable/Iterator"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_iterable_sample_matrix_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Iterable sample matrix"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_iterator_sample_matrix_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Iterator sample matrix"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_sequence_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Sequence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_sequence_mixins_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Sequence mixins"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_mapping_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Mapping"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_mapping_view_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for mapping views"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_mutable_sequence_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for MutableSequence"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_mapping_mixins_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Mapping mixins"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_mapping_mixin_views_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Mapping mixin views"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_userdict_view_snapshot_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for UserDict view snapshots"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_set_mutable_set_mixins_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Set/MutableSet mixins"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_set_from_iterable_operator_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Set._from_iterable operator dispatch"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_set_real_set_interoperability_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Set real-set interoperability"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_set_hash_matches_frozenset_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Set._hash()"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_issue26915_identity_first_object_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for identity-first container membership"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_set_noncomparable_comparison_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Set comparison fallback"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_reversible_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Reversible"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_reversible_direct_subclass_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Reversible direct subclassing"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_collection_direct_subclass_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Collection direct subclassing"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_async_runtime_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for async ABC runtime"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_async_iterator_mixin_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for AsyncIterator mixin"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_async_generator_core_mixin_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for AsyncGenerator core mixin"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_async_generator_throw_close_mixin_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for AsyncGenerator throw/close mixin"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_generator_mixin_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Generator mixin behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_generator_sample_matrix_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Generator sample matrix"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_generator_runtime_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Generator runtime behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_types_coroutine_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for collections.abc types.coroutine behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_coroutine_mixin_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Coroutine mixin behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_abstract_methods_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for ABC abstract methods"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_validate_isinstance_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for validate_isinstance"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_direct_subclassing_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for ABC direct subclassing"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_hashable_direct_subclass_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for Hashable direct subclassing"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_registration_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for ABC registration"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_bytestring_buffer_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for ByteString/Buffer"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_bytestring_deprecation_warnings_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for ByteString deprecation warnings"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_collections_abc_composite_abstract_methods_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for composite ABC abstract methods"
    );
}

#[test]
fn collections_abc_generator_coroutine_diff_covers_runtime_subsets() {
    for (subset, diff) in [
        (
            "cpython_collections_abc_async_generator_core_mixin_subset",
            "cpython_collections_abc_async_generator_core_mixin_diff_subset",
        ),
        (
            "cpython_collections_abc_async_generator_throw_close_mixin_subset",
            "cpython_collections_abc_async_generator_throw_close_mixin_diff_subset",
        ),
        (
            "cpython_collections_abc_generator_mixin_subset",
            "cpython_collections_abc_generator_mixin_diff_subset",
        ),
        (
            "cpython_collections_abc_generator_sample_matrix_subset",
            "cpython_collections_abc_generator_sample_matrix_diff_subset",
        ),
        (
            "cpython_collections_abc_generator_runtime_subset",
            "cpython_collections_abc_generator_runtime_diff_subset",
        ),
        (
            "cpython_collections_abc_types_coroutine_subset",
            "cpython_collections_abc_types_coroutine_diff_subset",
        ),
        (
            "cpython_collections_abc_coroutine_mixin_subset",
            "cpython_collections_abc_coroutine_mixin_diff_subset",
        ),
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "collections.abc generator/coroutine subset evidence `{subset}` must exist"
        );
        assert!(
            CPYTHON_DIFF.contains(&format!("fn {diff}(")),
            "collections.abc generator/coroutine CPython diff evidence `{diff}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(subset) && CPYTHON_COVERAGE.contains(diff),
            "coverage document must link collections.abc generator/coroutine evidence `{subset}` / `{diff}`"
        );
        assert!(
            CPYTHON_MIGRATION.contains(subset),
            "migration document must describe collections.abc generator/coroutine subset `{subset}`"
        );
    }

    let async_core_diff = CPYTHON_DIFF
        .split("fn cpython_collections_abc_async_generator_core_mixin_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_collections_abc_async_generator_throw_close_mixin_diff_subset()")
                .next()
        })
        .expect("collections.abc AsyncGenerator core diff evidence must be extractable");
    for required in [
        "class MinimalAGen(AsyncGenerator):",
        "isinstance(mgen, AsyncIterator)",
        "mgen.__aiter__() is mgen",
        "await mgen.__anext__()",
        "await mgen.asend(2)",
    ] {
        assert!(
            async_core_diff.contains(required),
            "collections.abc AsyncGenerator core diff evidence must cover `{required}`"
        );
    }

    let async_throw_close_diff = CPYTHON_DIFF
        .split("fn cpython_collections_abc_async_generator_throw_close_mixin_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_collections_abc_generator_mixin_diff_subset()")
                .next()
        })
        .expect("collections.abc AsyncGenerator throw/close diff evidence must be extractable");
    for required in [
        "AsyncGenerator.athrow(mgen, ValueError)",
        "AsyncGenerator.aclose(mgen)",
        "closed.send(None)",
        "athrow traceback object",
        "IgnoreGeneratorExit().aclose()",
    ] {
        assert!(
            async_throw_close_diff.contains(required),
            "collections.abc AsyncGenerator throw/close diff evidence must cover `{required}`"
        );
    }

    let generator_mixin_diff = CPYTHON_DIFF
        .split("fn cpython_collections_abc_generator_mixin_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_collections_abc_generator_sample_matrix_diff_subset()")
                .next()
        })
        .expect("collections.abc Generator mixin diff evidence must be extractable");
    for required in [
        "class MinimalGen(Generator):",
        "iter(mgen) is mgen",
        "mgen.send(None) is next(mgen)",
        "FailOnClose().close()",
        "IgnoreGeneratorExit().close()",
    ] {
        assert!(
            generator_mixin_diff.contains(required),
            "collections.abc Generator mixin diff evidence must cover `{required}`"
        );
    }

    let generator_runtime_diff = CPYTHON_DIFF
        .split("fn cpython_collections_abc_generator_runtime_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_collections_abc_types_coroutine_diff_subset()")
                .next()
        })
        .expect("collections.abc Generator runtime diff evidence must be extractable");
    for required in [
        "issubclass(Generator, Iterator)",
        "class GenBlocked(GenLike):",
        "async def agen():",
        "issubclass(AsyncGenerator, AsyncIterator)",
        "class AGenBlocked(AGenLike):",
    ] {
        assert!(
            generator_runtime_diff.contains(required),
            "collections.abc Generator runtime diff evidence must cover `{required}`"
        );
    }

    let types_coroutine_diff = CPYTHON_DIFF
        .split("fn cpython_collections_abc_types_coroutine_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_collections_abc_coroutine_mixin_diff_subset()")
                .next()
        })
        .expect("collections.abc types.coroutine diff evidence must be extractable");
    for required in [
        "@types.coroutine",
        "isinstance(wrapped, Awaitable)",
        "isinstance(wrapped, Coroutine)",
        "result = await iterable_coro()",
        "Coroutine.register(CoroLike)",
    ] {
        assert!(
            types_coroutine_diff.contains(required),
            "collections.abc types.coroutine diff evidence must cover `{required}`"
        );
    }

    let coroutine_mixin_diff = CPYTHON_DIFF
        .split("fn cpython_collections_abc_coroutine_mixin_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_collections_abc_abstract_methods_diff_subset()")
                .next()
        })
        .expect("collections.abc Coroutine mixin diff evidence must be extractable");
    for required in [
        "class DefaultCoro(Coroutine):",
        "super().send(value)",
        "super().throw(typ, val, tb)",
        "IgnoreExit().close()",
        "FailClose().close()",
    ] {
        assert!(
            coroutine_mixin_diff.contains(required),
            "collections.abc Coroutine mixin diff evidence must cover `{required}`"
        );
    }
}

#[test]
fn collections_abc_newer_oracle_diff_evidence_stays_capability_gated() {
    for (function, required) in [
        (
            "fn cpython_collections_abc_mutable_sequence_diff_subset()",
            &[
                "issubclass(array.array, MutableSequence)",
                "skipping MutableSequence diff",
            ][..],
        ),
        (
            "fn cpython_collections_abc_bytestring_buffer_diff_subset()",
            &[
                "hasattr(collections.abc, 'Buffer')",
                "skipping collections.abc ByteString/Buffer diff",
            ],
        ),
        (
            "fn cpython_collections_abc_bytestring_deprecation_warnings_diff_subset()",
            &[
                "warnings.catch_warnings(record=True)",
                "skipping collections.abc ByteString deprecation diff",
            ],
        ),
        (
            "fn cpython_collections_abc_composite_abstract_methods_diff_subset()",
            &[
                "hasattr(collections.abc, 'Buffer')",
                "skipping collections.abc composite abstract-method diff",
            ],
        ),
    ] {
        let start = CPYTHON_DIFF.find(function).unwrap_or_else(|| {
            panic!("collections.abc gated diff evidence `{function}` must exist")
        });
        let body = &CPYTHON_DIFF[start..];
        let end = body.find("\n#[test]").unwrap_or(body.len());
        let body = &body[..end];

        for text in required {
            assert!(
                body.contains(text),
                "collections.abc gated diff evidence `{function}` must contain `{text}`"
            );
        }
    }
}

#[test]
fn collections_public_diff_evidence_stays_capability_gated() {
    for (function, required) in [
        (
            "fn cpython_collections_counter_comparison_diff_subset()",
            &[
                "hasattr(Counter(), 'total')",
                "skipping Counter comparison diff",
            ][..],
        ),
        (
            "fn cpython_collections_counter_multiset_operations_equivalent_to_set_operations_diff_subset()",
            &[
                "hasattr(Counter, '__xor__')",
                "skipping Counter set-equivalence diff",
            ],
        ),
        (
            "fn cpython_collections_counter_symmetric_difference_diff_subset()",
            &[
                "hasattr(Counter, '__xor__')",
                "skipping Counter symmetric-difference diff",
            ],
        ),
        (
            "fn cpython_collections_namedtuple_match_args_diff_subset()",
            &[
                "getattr(Point, '__match_args__', None)",
                "skipping namedtuple __match_args__ diff",
            ],
        ),
        (
            "fn cpython_collections_namedtuple_factory_instance_diff_subset()",
            &[
                "getattr(Point, '__match_args__', None)",
                "skipping namedtuple factory/instance diff",
            ],
        ),
        (
            "fn cpython_collections_namedtuple_new_builtins_issue_43102_diff_subset()",
            &[
                "hasattr(obj.__new__, '__builtins__')",
                "skipping namedtuple new builtins issue diff",
            ],
        ),
    ] {
        let start = CPYTHON_DIFF
            .find(function)
            .unwrap_or_else(|| panic!("collections gated diff evidence `{function}` must exist"));
        let body = &CPYTHON_DIFF[start..];
        let end = body.find("\n#[test]").unwrap_or(body.len());
        let body = &body[..end];

        for text in required {
            assert!(
                body.contains(text),
                "collections gated diff evidence `{function}` must contain `{text}`"
            );
        }
    }
}

#[test]
fn copy_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "copy",
        &["cpython_copy_public_subset"],
        &["pickle protocol"],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "copy")
        .expect("sandbox stdlib manifest must include copy");
    for evidence in [
        "cpython_copy_public_diff_subset",
        "cpython_array_one_byte_public_copy_byteswap_compare_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "copy sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn copy_public_diff_covers_pure_memory_subset() {
    assert!(
        CPYTHON_SUBSET.contains("fn cpython_copy_public_subset("),
        "copy runtime subset evidence must exist"
    );
    assert!(
        CPYTHON_DIFF.contains("fn cpython_copy_public_diff_subset("),
        "copy CPython diff evidence must exist"
    );
    assert!(
        CPYTHON_COVERAGE.contains("cpython_copy_public_subset")
            && CPYTHON_COVERAGE.contains("cpython_copy_public_diff_subset"),
        "coverage document must link copy runtime and diff evidence"
    );
    assert!(
        CPYTHON_MIGRATION.contains("cpython_copy_public_subset")
            && CPYTHON_MIGRATION.contains("cpython_copy_public_diff_subset"),
        "migration document must link copy runtime and diff evidence"
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "copy")
        .expect("sandbox stdlib manifest must include copy");
    assert!(
        row.excluded_surface.contains("pickle protocol"),
        "copy sandbox manifest must keep pickle protocol outside the supported surface"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_copy_public_diff_subset"),
        "copy sandbox manifest must cite copy public CPython diff evidence"
    );

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_copy_public_diff_subset");
    for required in [
        "copy.Error is copy.error",
        "copy.dispatch_table",
        "copy.copy(nested)",
        "copy.deepcopy(nested)",
        "list-alias",
        "dict-alias",
        "tuple-alias",
        "list-cycle",
        "instance-alias",
        "instance-cycle",
        "userlist-alias",
        "userlist-cycle",
        "userdict-alias",
        "deque-alias",
    ] {
        assert!(
            body.contains(required),
            "copy CPython diff evidence must cover pure-memory behavior `{required}`"
        );
    }
}

#[test]
fn io_bytesio_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "io.BytesIO",
        &[
            "cpython_io_bytesio_public_subset",
            "cpython_memoryview_bytesio_readinto_subset",
        ],
        &["Real files", "file descriptors"],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "io.BytesIO")
        .expect("sandbox stdlib manifest must include io.BytesIO");
    assert!(
        row.diff_evidence
            .contains("cpython_io_bytesio_public_diff_subset"),
        "io.BytesIO sandbox manifest must cite CPython diff evidence for public BytesIO behavior"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_memoryview_bytesio_readinto_diff_subset"),
        "io.BytesIO sandbox manifest must cite CPython diff evidence for readinto(memoryview)"
    );
    assert!(
        row.diff_evidence
            .contains("cpython_array_one_byte_public_file_methods_diff_subset"),
        "io.BytesIO sandbox manifest must cite CPython diff evidence for array tofile/fromfile BytesIO behavior"
    );
}

#[test]
fn io_bytesio_cross_module_diff_stays_pure_memory_only() {
    for (subset, diff) in [
        (
            "cpython_io_bytesio_public_subset",
            "cpython_io_bytesio_public_diff_subset",
        ),
        (
            "cpython_memoryview_bytesio_readinto_subset",
            "cpython_memoryview_bytesio_readinto_diff_subset",
        ),
        (
            "cpython_array_one_byte_public_file_methods_subset",
            "cpython_array_one_byte_public_file_methods_diff_subset",
        ),
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "io.BytesIO pure-memory subset evidence `{subset}` must exist"
        );
        assert!(
            CPYTHON_DIFF.contains(&format!("fn {diff}(")),
            "io.BytesIO pure-memory CPython diff evidence `{diff}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(subset) && CPYTHON_COVERAGE.contains(diff),
            "coverage document must link io.BytesIO evidence `{subset}` / `{diff}`"
        );
        assert!(
            CPYTHON_MIGRATION.contains(subset),
            "migration document must describe io.BytesIO subset `{subset}`"
        );
    }

    let bytesio_diff = CPYTHON_DIFF
        .split("fn cpython_io_bytesio_public_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_functools_public_helpers_diff_subset()")
                .next()
        })
        .expect("io.BytesIO public diff evidence must be extractable");
    for required in [
        "io.BytesIO(b'abc')",
        "bio.getbuffer()",
        "io.UnsupportedOperation",
        "bio.fileno",
        "bio.detach",
        "with io.BytesIO(b'xy') as inside:",
        "bio.readinto(target)",
    ] {
        assert!(
            bytesio_diff.contains(required),
            "io.BytesIO public diff evidence must cover `{required}`"
        );
    }

    let memoryview_diff = CPYTHON_DIFF
        .split("fn cpython_memoryview_bytesio_readinto_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_memoryview_weakref_live_diff_subset()")
                .next()
        })
        .expect("memoryview BytesIO readinto diff evidence must be extractable");
    for required in [
        "memoryview(bytearray(b'abc'))",
        "memoryview(b'abc')",
        "bio.readinto(target)",
        "initial_bytes=b'ab'",
    ] {
        assert!(
            memoryview_diff.contains(required),
            "memoryview BytesIO diff evidence must cover `{required}`"
        );
    }

    let array_file_diff = CPYTHON_DIFF
        .split("fn cpython_array_one_byte_public_file_methods_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_array_one_byte_public_clear_diff_subset()")
                .next()
        })
        .expect("array BytesIO file-method diff evidence must be extractable");
    for required in [
        "target = io.BytesIO()",
        "a.tofile(target)",
        "array.array(tc).fromfile(io.BytesIO(), 1, 2)",
        "TextRead",
        "ByteArrayRead",
    ] {
        assert!(
            array_file_diff.contains(required),
            "array BytesIO file-method diff evidence must cover `{required}`"
        );
    }

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "io.BytesIO")
        .expect("sandbox stdlib manifest must include io.BytesIO");
    for excluded in ["Real files", "file descriptors"] {
        assert!(
            row.excluded_surface.contains(excluded),
            "io.BytesIO sandbox manifest must keep `{excluded}` outside the supported surface"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        for required in ["in-memory", "io.BytesIO", "file descriptors"] {
            assert!(
                document.contains(required),
                "io.BytesIO docs must keep pure-memory boundary `{required}` documented"
            );
        }
        assert!(
            document.contains("Real files")
                || document.contains("host file")
                || document.contains("buffering layers"),
            "io.BytesIO docs must keep host file APIs outside the sandbox subset"
        );
    }
}

#[test]
fn math_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "math / math.integer",
        &[
            "cpython_math_core_subset",
            "cpython_math_constants_and_classification_subset",
            "cpython_math_integer_subset",
            "cpython_math_isclose_subset",
            "cpython_math_hypot_dist_subset",
            "cpython_math_copysign_subset",
            "cpython_math_signbit_subset",
            "cpython_math_trunc_subset",
            "cpython_math_ceil_subset",
            "cpython_math_floor_subset",
            "cpython_math_degrees_radians_subset",
            "cpython_math_cbrt_subset",
            "cpython_math_erf_erfc_subset",
            "cpython_math_gamma_lgamma_subset",
            "cpython_math_exp_exp2_subset",
            "cpython_math_expm1_subset",
            "cpython_math_log_family_subset",
            "cpython_math_trig_subset",
            "cpython_math_hyperbolic_subset",
            "cpython_math_fabs_subset",
            "cpython_math_fma_subset",
            "cpython_math_fmax_fmin_subset",
            "cpython_math_fmod_remainder_subset",
            "cpython_math_frexp_ldexp_modf_subset",
            "cpython_math_fsum_subset",
            "cpython_math_sumprod_subset",
            "cpython_math_nextafter_ulp_subset",
            "cpython_math_pow_subset",
            "cpython_math_sqrt_subset",
            "cpython_math_gcd_subset",
            "cpython_math_lcm_subset",
            "cpython_math_prod_subset",
        ],
        &["Platform/libm", "locale-sensitive"],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "math / math.integer")
        .expect("sandbox stdlib manifest must include math / math.integer");
    for evidence in [
        "cpython_math_core_diff_subset",
        "cpython_math_constants_and_classification_diff_subset",
        "cpython_math_isclose_diff_subset",
        "cpython_math_hypot_dist_diff_subset",
        "cpython_math_gcd_diff_subset",
        "cpython_math_lcm_diff_subset",
        "cpython_math_prod_diff_subset",
        "cpython_math_integer_diff_subset",
        "cpython_math_sqrt_diff_subset",
        "cpython_math_fabs_diff_subset",
        "cpython_math_copysign_diff_subset",
        "cpython_math_signbit_diff_subset",
        "cpython_math_trunc_diff_subset",
        "cpython_math_ceil_diff_subset",
        "cpython_math_floor_diff_subset",
        "cpython_math_degrees_radians_diff_subset",
        "cpython_math_cbrt_diff_subset",
        "cpython_math_erf_erfc_diff_subset",
        "cpython_math_gamma_lgamma_diff_subset",
        "cpython_math_fma_diff_subset",
        "cpython_math_fmax_fmin_diff_subset",
        "cpython_math_exp_exp2_diff_subset",
        "cpython_math_expm1_diff_subset",
        "cpython_math_log_family_diff_subset",
        "cpython_math_trig_diff_subset",
        "cpython_math_hyperbolic_diff_subset",
        "cpython_math_fmod_remainder_diff_subset",
        "cpython_math_frexp_ldexp_modf_diff_subset",
        "cpython_math_fsum_diff_subset",
        "cpython_math_sumprod_diff_subset",
        "cpython_math_nextafter_ulp_diff_subset",
        "cpython_math_pow_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "math sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn math_newer_oracle_diff_evidence_stays_capability_gated() {
    for (function, probe, skip) in [
        (
            "cpython_math_constants_and_classification_diff_subset",
            "hasattr(math, 'isnormal')",
            "skipping math constants/classification diff",
        ),
        (
            "cpython_math_signbit_diff_subset",
            "hasattr(math, 'signbit')",
            "skipping math.signbit diff",
        ),
        (
            "cpython_math_cbrt_diff_subset",
            "hasattr(math, 'cbrt')",
            "skipping math.cbrt diff",
        ),
        (
            "cpython_math_fma_diff_subset",
            "hasattr(math, 'fma')",
            "skipping math.fma diff",
        ),
        (
            "cpython_math_fmax_fmin_diff_subset",
            "hasattr(math, 'fmax')",
            "skipping math.fmax/fmin diff",
        ),
        (
            "cpython_math_exp_exp2_diff_subset",
            "hasattr(math, 'exp2')",
            "skipping math.exp/exp2 diff",
        ),
        (
            "cpython_math_sumprod_diff_subset",
            "hasattr(math, 'sumprod')",
            "skipping math.sumprod diff",
        ),
        (
            "cpython_math_nextafter_ulp_diff_subset",
            "math.nextafter(1.0, 2.0, steps=0)",
            "skipping math.nextafter/ulp diff",
        ),
    ] {
        let start = CPYTHON_DIFF
            .find(&format!("fn {function}()"))
            .unwrap_or_else(|| panic!("math gated diff evidence `{function}` must exist"));
        let body = &CPYTHON_DIFF[start..];
        let end = body.find("\n#[test]").unwrap_or(body.len());
        let body = &body[..end];
        assert!(
            body.contains(probe) && body.contains(skip),
            "math gated diff evidence `{function}` must keep probe `{probe}` and skip text `{skip}`"
        );
    }
}

#[test]
fn sys_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "sys",
        &[
            "cpython_float_hash_and_sys_info_subset",
            "cpython_builtin_negation_sys_maxsize_subset",
            "cpython_int_max_str_digits_runtime_subset",
            "cpython_attribute_introspection_builtins_subset",
            "cpython_builtin_breakpoint_custom_hook_subset",
            "cpython_builtin_breakpoint_default_stub_subset",
            "cpython_builtin_breakpoint_passthru_error_subset",
            "cpython_types_frame_locals_proxy_type_subset",
        ],
        &[
            "Real argv/process state",
            "real stdin/stdout/stderr",
            "refcount/GC/debug APIs",
        ],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "sys")
        .expect("sandbox stdlib manifest must include sys");
    for evidence in [
        "cpython_globals_locals_builtin_diff_subset",
        "cpython_attribute_introspection_builtins_diff_subset",
        "cpython_builtin_negation_sys_maxsize_diff_subset",
        "cpython_int_max_str_digits_runtime_diff_subset",
        "cpython_builtin_breakpoint_custom_hook_diff_subset",
        "cpython_builtin_breakpoint_passthru_error_diff_subset",
        "cpython_float_hash_and_sys_info_diff_subset",
        "cpython_types_frame_locals_proxy_type_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "sys sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn sys_process_stdio_and_debug_api_stop_line_stays_sandbox_classified() {
    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "sys")
        .expect("sandbox stdlib manifest must include sys");

    for excluded in [
        "Real argv/process state",
        "real stdin/stdout/stderr",
        "refcount/GC/debug APIs",
    ] {
        assert!(
            row.excluded_surface.contains(excluded),
            "sys sandbox manifest must keep `{excluded}` outside the supported surface"
        );
    }

    for supported in [
        "cpython_float_hash_and_sys_info_subset",
        "cpython_int_max_str_digits_runtime_subset",
        "cpython_attribute_introspection_builtins_subset",
        "cpython_types_frame_locals_proxy_type_subset",
        "cpython_builtin_breakpoint_default_stub_subset",
    ] {
        assert!(
            row.supported_surface.contains(supported),
            "sys sandbox manifest must list supported in-memory evidence `{supported}`"
        );
    }

    let stdio_start = CPYTHON_SUBSET
        .find("fn cpython_attribute_introspection_builtins_subset()")
        .expect("attribute introspection subset evidence must be extractable");
    let stdio_end = CPYTHON_SUBSET[stdio_start..]
        .find("\n#[test]")
        .map(|offset| stdio_start + offset)
        .unwrap_or(CPYTHON_SUBSET.len());
    let stdio_source = &CPYTHON_SUBSET[stdio_start..stdio_end];
    for required in [
        "hasattr(sys, 'stdout')",
        "getattr(sys, 'stdout') is sys.stdout",
        "from sys import stdin, stderr, stdout",
        "stdin is sys.stdin",
        "stderr is sys.stderr",
        "stdout is sys.stdout",
    ] {
        assert!(
            stdio_source.contains(required),
            "sys stdio placeholder subset evidence must cover `{required}`"
        );
    }

    let frame_start = CPYTHON_SUBSET
        .find("fn cpython_types_frame_locals_proxy_type_subset()")
        .expect("types.FrameType/sys frame subset evidence must be extractable");
    let frame_end = CPYTHON_SUBSET[frame_start..]
        .find("\n#[test]")
        .map(|offset| frame_start + offset)
        .unwrap_or(CPYTHON_SUBSET.len());
    let frame_source = &CPYTHON_SUBSET[frame_start..frame_end];
    for required in [
        "inspect.currentframe()",
        "types.FrameLocalsProxyType",
        "frame.f_locals",
    ] {
        assert!(
            frame_source.contains(required),
            "sys frame subset evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        for required in ["stdin", "stdout", "stderr"] {
            assert!(
                document.contains(required),
                "sys docs must describe stdio placeholder term `{required}`"
            );
        }
    }

    for required in [
        "Real argv/process state",
        "real stdin/stdout/stderr streams",
        "implementation refcount/GC/debug APIs",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(required),
            "migration document must keep sys stop-line term `{required}`"
        );
    }
}

#[test]
fn builtins_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "builtins",
        &[
            "cpython_eval_builtin_subset",
            "cpython_exec_builtin_subset",
            "cpython_exec_closure_subset",
            "cpython_eval_exec_builtins_mapping_subset",
            "cpython_compile_builtin_code_object_subset",
            "cpython_globals_locals_builtin_subset",
            "cpython_vars_dir_builtin_subset",
            "cpython_isinstance_builtin_subset",
            "cpython_issubclass_builtin_subset",
            "cpython_attribute_introspection_builtins_subset",
            "cpython_all_any_builtin_subset",
            "cpython_len_builtin_subset",
            "cpython_min_max_sum_builtin_subset",
            "cpython_iter_next_builtin_subset",
            "cpython_aiter_anext_builtin_subset",
            "cpython_stop_iteration_value_subset",
            "cpython_enumerate_zip_sorted_builtin_subset",
            "cpython_builtin_sorted_exact_subset",
            "cpython_zip_strict_builtin_subset",
            "cpython_map_filter_builtin_subset",
            "cpython_map_strict_builtin_subset",
            "cpython_abs_builtin_subset",
            "cpython_builtin_print_keyword_subset",
            "cpython_divmod_builtin_subset",
            "cpython_round_builtin_subset",
            "cpython_pow_builtin_subset",
            "cpython_chr_ord_builtin_subset",
            "cpython_format_builtin_and_custom_dunder_format_subset",
            "cpython_ascii_builtin_subset",
            "cpython_builtin_cmp_absent_subset",
            "cpython_builtin_none_ne_direct_subset",
            "cpython_builtin_exception_hierarchy_subset",
            "cpython_runtime_exception_capture_subset",
            "cpython_base_exception_args_subset",
            "cpython_base_exception_with_traceback_subset",
            "cpython_system_exit_oserror_attributes_subset",
            "cpython_syntax_error_attributes_subset",
            "cpython_unicode_error_attributes_subset",
            "cpython_attribute_error_keyword_attributes_subset",
            "cpython_object_repr_str_direct_subset",
            "cpython_str_builtin_custom_dunder_subset",
            "cpython_builtin_bool_notimplemented_subset",
            "cpython_builtin_construct_singletons_subset",
            "cpython_builtin_singleton_attribute_access_subset",
            "cpython_hash_builtin_subset",
            "cpython_id_builtin_subset",
            "cpython_builtin_breakpoint_custom_hook_subset",
            "cpython_builtin_breakpoint_default_stub_subset",
            "cpython_builtin_breakpoint_passthru_error_subset",
        ],
        &[
            "`open()`",
            "`input()`",
            "host TTY behavior",
            "default pdb-backed breakpoint behavior",
            "process/environment side effects",
        ],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "builtins")
        .expect("sandbox stdlib manifest must include builtins");
    for evidence in [
        "cpython_globals_locals_builtin_diff_subset",
        "cpython_vars_dir_builtin_diff_subset",
        "cpython_eval_builtin_diff_subset",
        "cpython_exec_builtin_diff_subset",
        "cpython_eval_exec_builtins_mapping_diff_subset",
        "cpython_compile_builtin_code_object_diff_subset",
        "cpython_isinstance_builtin_diff_subset",
        "cpython_issubclass_builtin_diff_subset",
        "cpython_attribute_introspection_builtins_diff_subset",
        "cpython_ascii_builtin_diff_subset",
        "cpython_chr_ord_builtin_diff_subset",
        "cpython_builtin_cmp_absent_diff_subset",
        "cpython_builtin_none_ne_direct_diff_subset",
        "cpython_builtin_exception_hierarchy_diff_subset",
        "cpython_runtime_exception_capture_diff_subset",
        "cpython_base_exception_args_diff_subset",
        "cpython_base_exception_with_traceback_diff_subset",
        "cpython_system_exit_oserror_attributes_diff_subset",
        "cpython_syntax_error_attributes_diff_subset",
        "cpython_unicode_error_attributes_diff_subset",
        "cpython_object_repr_str_direct_diff_subset",
        "cpython_str_builtin_custom_dunder_diff_subset",
        "cpython_builtin_bool_notimplemented_diff_subset",
        "cpython_builtin_singleton_construction_and_attributes_diff_subset",
        "cpython_all_any_builtin_diff_subset",
        "cpython_len_builtin_diff_subset",
        "cpython_min_max_sum_builtin_diff_subset",
        "cpython_iter_next_builtin_diff_subset",
        "cpython_aiter_anext_builtin_diff_subset",
        "cpython_stop_iteration_value_diff_subset",
        "cpython_map_filter_builtin_diff_subset",
        "cpython_map_strict_builtin_diff_subset",
        "cpython_enumerate_zip_sorted_builtin_diff_subset",
        "cpython_builtin_sorted_exact_diff_subset",
        "cpython_zip_strict_builtin_diff_subset",
        "cpython_divmod_builtin_diff_subset",
        "cpython_pow_builtin_diff_subset",
        "cpython_abs_builtin_diff_subset",
        "cpython_builtin_print_keyword_diff_subset",
        "cpython_round_builtin_diff_subset",
        "cpython_format_builtin_and_custom_dunder_format_diff_subset",
        "cpython_hash_id_builtins_diff_subset",
        "cpython_builtin_breakpoint_custom_hook_diff_subset",
        "cpython_builtin_breakpoint_passthru_error_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "builtins sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn runtime_exception_capture_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_runtime_exception_capture_subset(",
        "[][10]",
        "{}[\\\"key\\\"]",
        "1[0]",
        "for item in 1",
        "1(2)",
        "raise NotImplementedError(\\\"todo\\\")",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused runtime exception capture subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(
        CPYTHON_DIFF,
        "cpython_runtime_exception_capture_diff_subset",
    );
    for required in [
        "Lib/test/test_exceptions.py runtime exception object capture subset",
        "[][10]",
        "{}[\"key\"]",
        "1[0]",
        "isinstance(error, TypeError)",
        "for item in 1",
        "1(2)",
        "raise NotImplementedError(\"todo\")",
    ] {
        assert!(
            body.contains(required),
            "focused runtime exception capture CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_runtime_exception_capture_subset")
                && document.contains("cpython_runtime_exception_capture_diff_subset"),
            "focused runtime exception capture evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn builtin_singleton_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_builtin_construct_singletons_subset(",
        "for const in [None, Ellipsis, NotImplemented]",
        "tp = type(const)",
        "tp() is const",
        "lambda: tp(1, 2)",
        "lambda: tp(a=1, b=2)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused singleton construction subset evidence must cover `{required}`"
        );
    }

    for required in [
        "fn cpython_builtin_singleton_attribute_access_subset(",
        "for singleton in [NotImplemented, Ellipsis]",
        "type(singleton) is singleton.__class__",
        "type(singleton).__class__ is type",
        "setattr(singleton, 'prop', 1)",
        "setattr(type(singleton), 'prop', 1)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused singleton attribute subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(
        CPYTHON_DIFF,
        "cpython_builtin_singleton_construction_and_attributes_diff_subset",
    );
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_construct_singletons / ::test_singleton_attribute_access",
        "for const in [None, Ellipsis, NotImplemented]",
        "tp() is const",
        "lambda: tp(1, 2)",
        "lambda: tp(a=1, b=2)",
        "for singleton in [NotImplemented, Ellipsis]",
        "type(singleton) is singleton.__class__",
        "setattr(singleton, 'prop', 1)",
        "setattr(type(singleton), 'prop', 1)",
    ] {
        assert!(
            body.contains(required),
            "focused singleton CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_builtin_construct_singletons_subset")
                && document.contains("cpython_builtin_singleton_attribute_access_subset")
                && document
                    .contains("cpython_builtin_singleton_construction_and_attributes_diff_subset"),
            "focused singleton evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn hash_id_builtins_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_hash_builtin_subset(",
        "type(hash(None)).__name__",
        "hash(1) == hash(1.0)",
        "hash(True) == hash(1)",
        "hash('spam') == hash(b'spam')",
        "hash((0, 1, 2, 3))",
        "lambda: hash([])",
        "lambda: hash({})",
        "lambda: hash(([1],))",
        "lambda: hash(Bad())",
        "lambda: hash(NoHash())",
        "hash(value) == 42",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused hash builtin subset evidence must cover `{required}`"
        );
    }

    for required in [
        "fn cpython_id_builtin_subset(",
        "type(id(None)).__name__",
        "type(id(1)).__name__",
        "type(id('spam')).__name__",
        "id(items) == id(alias)",
        "id(items) == id(other)",
        "id(d) == id(d)",
        "lambda: id()",
        "lambda: id(1, 2)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused id builtin subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_hash_id_builtins_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_hash / ::test_invalid_hash_typeerror / ::test_id",
        "type(hash(None)).__name__",
        "hash(1) == hash(1.0)",
        "hash(True) == hash(1)",
        "hash('spam') == hash(b'spam')",
        "hash((0, 1, 2, 3))",
        "lambda: hash([])",
        "lambda: hash({})",
        "lambda: hash(([1],))",
        "lambda: hash(Bad())",
        "lambda: hash(NoHash())",
        "hash(value) == 42",
        "type(id(None)).__name__",
        "type(id(1)).__name__",
        "type(id('spam')).__name__",
        "id(items) == id(alias)",
        "id(items) == id(other)",
        "id(d) == id(d)",
        "lambda: id()",
        "lambda: id(1, 2)",
    ] {
        assert!(
            body.contains(required),
            "focused hash/id CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_hash_builtin_subset")
                && document.contains("cpython_id_builtin_subset")
                && document.contains("cpython_hash_id_builtins_diff_subset"),
            "focused hash/id builtins evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn len_builtin_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_len_builtin_subset(",
        "len('123')",
        "len((1, 2, 3, 4))",
        "len([1, 2, 3, 4])",
        "len({'a': 1, 'b': 2})",
        "class BadSeq",
        "raise ValueError",
        "class InvalidLen",
        "return None",
        "class FloatLen",
        "return 4.5",
        "class NegativeLen",
        "return -10",
        "class HugeLen",
        "sys.maxsize + 1",
        "class HugeNegativeLen",
        "-sys.maxsize - 10",
        "class NoLenMethod",
        "lambda: len()",
        "lambda: len([], [])",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused len builtin subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_len_builtin_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_len",
        "len('123')",
        "len((1, 2, 3, 4))",
        "len([1, 2, 3, 4])",
        "len({'a': 1, 'b': 2})",
        "class BadSeq",
        "raise ValueError",
        "class InvalidLen",
        "return None",
        "class FloatLen",
        "return 4.5",
        "class NegativeLen",
        "return -10",
        "class HugeLen",
        "sys.maxsize + 1",
        "class HugeNegativeLen",
        "-sys.maxsize - 10",
        "class NoLenMethod",
        "lambda: len()",
        "lambda: len([], [])",
    ] {
        assert!(
            body.contains(required),
            "focused len builtin CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_len_builtin_subset")
                && document.contains("cpython_len_builtin_diff_subset")
                && document.contains("BuiltinTest::test_len"),
            "focused len builtin evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn min_max_sum_builtins_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_min_max_sum_builtin_subset(",
        "max('123123')",
        "min('123123')",
        "max((1, 2, 3, 1, 2, 3))",
        "min((1, 2, 3, 1, 2, 3))",
        "def neg(x):",
        "key=neg",
        "default=None",
        "class BadSeq",
        "raise ValueError('badseq')",
        "lambda: max()",
        "lambda: max(42)",
        "lambda: max(())",
        "lambda: min()",
        "lambda: min(42)",
        "lambda: min(())",
        "sum([])",
        "sum(list(range(2, 8)))",
        "sum([[1], [2], [3]], [])",
        "sum(range(10), start=1000)",
        "sum([], False) is False",
        "repr(sum([-0.0]))",
        "math.isinf(sum([float('inf'), float('inf')]))",
        "lambda: sum(['a', 'b', 'c'])",
        "lambda: sum([b'a', b'c'], b'')",
        "lambda: sum(values, bytearray(b''))",
        "lambda: sum([1.0, 10**1000])",
        "complex(1, -0.0)",
        "\"(2-0j)\"",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused min/max/sum builtin subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_min_max_sum_builtin_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_max / ::test_min / ::test_sum",
        "max('123123')",
        "min('123123')",
        "max((1, 2, 3, 1, 2, 3))",
        "min((1, 2, 3, 1, 2, 3))",
        "def neg(x):",
        "key=neg",
        "default=None",
        "class BadSeq",
        "raise ValueError('badseq')",
        "lambda: max()",
        "lambda: max(42)",
        "lambda: max(())",
        "lambda: min()",
        "lambda: min(42)",
        "lambda: min(())",
        "sum([])",
        "sum(list(range(2, 8)))",
        "sum([[1], [2], [3]], [])",
        "sum(range(10), start=1000)",
        "sum([], False) is False",
        "lambda: sum(['a', 'b', 'c'])",
        "lambda: sum([b'a', b'c'], b'')",
    ] {
        assert!(
            body.contains(required),
            "focused min/max/sum CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_min_max_sum_builtin_subset")
                && document.contains("cpython_min_max_sum_builtin_diff_subset"),
            "focused min/max/sum evidence must be documented in coverage and migration notes"
        );
    }
    assert!(
        CPYTHON_COVERAGE.contains("BuiltinTest::test_max")
            && CPYTHON_COVERAGE.contains("test_min")
            && CPYTHON_COVERAGE.contains("test_sum")
            && CPYTHON_COVERAGE.contains("test_sum_accuracy"),
        "focused min/max/sum coverage notes must name the CPython source cases and excluded accuracy test"
    );
    assert!(
        CPYTHON_MIGRATION.contains("aggregate methods")
            && CPYTHON_MIGRATION.contains("BuiltinTest::test_sum")
            && CPYTHON_MIGRATION.contains("test_sum_accuracy")
            && CPYTHON_MIGRATION.contains("implementation-internal"),
        "focused min/max/sum migration notes must classify the aggregate audit and excluded CPython implementation detail"
    );
}

#[test]
fn all_any_builtins_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_all_any_builtin_subset(",
        "class TestFailingBool",
        "raise RuntimeError('bool fail')",
        "class TestFailingIter",
        "raise RuntimeError('iter fail')",
        "all([2, 4, 6])",
        "all([2, None, 6])",
        "all([])",
        "all([0, TestFailingBool()])",
        "any([None, None, None])",
        "any([None, 4, None])",
        "any([])",
        "any([1, TestFailingBool()])",
        "all(x > 42 for x in [50, 60])",
        "all(x > 42 for x in [50, 40, 60, TestFailingBool()])",
        "any(x > 42 for x in [40, 60, 30, TestFailingBool()])",
        "lambda: all([2, TestFailingBool(), 6])",
        "lambda: all(TestFailingIter())",
        "lambda: any([None, TestFailingBool(), 6])",
        "lambda: any(TestFailingIter())",
        "lambda: all(10)",
        "lambda: all()",
        "lambda: all([2, 4, 6], [])",
        "lambda: any(10)",
        "lambda: any()",
        "lambda: any([2, 4, 6], [])",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused all/any builtin subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_all_any_builtin_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_all / ::test_any",
        "class TestFailingBool",
        "raise RuntimeError('bool fail')",
        "class TestFailingIter",
        "raise RuntimeError('iter fail')",
        "all([2, 4, 6])",
        "all([2, None, 6])",
        "all([])",
        "all([0, TestFailingBool()])",
        "any([None, None, None])",
        "any([None, 4, None])",
        "any([])",
        "any([1, TestFailingBool()])",
        "all(x > 42 for x in [50, 60])",
        "any(x > 42 for x in [40, 60, 30])",
        "lambda: all([2, TestFailingBool(), 6])",
        "lambda: all(TestFailingIter())",
        "lambda: any([None, TestFailingBool(), 6])",
        "lambda: any(TestFailingIter())",
        "lambda: all(10)",
        "lambda: all()",
        "lambda: all([2, 4, 6], [])",
        "lambda: any(10)",
        "lambda: any()",
        "lambda: any([2, 4, 6], [])",
    ] {
        assert!(
            body.contains(required),
            "focused all/any CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_all_any_builtin_subset")
                && document.contains("cpython_all_any_builtin_diff_subset")
                && document.contains("BuiltinTest::test_all")
                && document.contains("test_any"),
            "focused all/any evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn iter_next_builtins_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_iter_next_builtin_subset(",
        "Lib/test/test_builtin.py::BuiltinTest::test_iter",
        "::test_next",
        "selected Lib/test/test_iter.py iterator exhaustion cases",
        "for value in [('1', '2'), ['1', '2'], '12']",
        "iterator = iter(value)",
        "next(iterator)",
        "except StopIteration",
        "lambda: iter()",
        "lambda: iter(42, 42)",
        "iterator = iter(range(2))",
        "next(iterator, 42)",
        "class Iter",
        "def __next__(self):",
        "def gen():",
        "HAS_MORE = 1",
        "NO_MORE = 2",
        "def exhaust(iterator):",
        "state[1] = iter(spam, NO_MORE)",
        "stop_by_exception",
        "list(iter(stop_by_exception, 99))",
        "show('list', iter(items)",
        "show('tuple', iter((0, 1, 2, 3, 4)))",
        "show('string', iter('abcde'))",
        "class Sequence",
        "show('sequence', iter(sequence)",
        "show('callable', iter(spam, 5))",
        "show('range', iter(range(5)))",
        "show('yield', gen())",
        "show('enumerate', enumerate(range(5)))",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused iter/next builtin subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_iter_next_builtin_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_iter / ::test_next",
        "for value in [('1', '2'), ['1', '2'], '12']",
        "iterator = iter(value)",
        "next(iterator)",
        "except StopIteration",
        "iterator = iter(range(2))",
        "next(iterator, 42)",
        "class Iter",
        "def __next__(self):",
        "def gen():",
        "lambda: iter()",
        "lambda: iter(42, 42)",
        "lambda: next()",
        "lambda: next(42)",
    ] {
        assert!(
            body.contains(required),
            "focused iter/next CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_iter_next_builtin_subset")
                && document.contains("cpython_iter_next_builtin_diff_subset")
                && document.contains("callable-sentinel")
                && document.contains("sink-state"),
            "focused iter/next evidence must be documented in coverage and migration notes"
        );
    }
    assert!(
        CPYTHON_MIGRATION.contains("BuiltinTest::test_iter")
            && CPYTHON_MIGRATION.contains("::test_next")
            && CPYTHON_MIGRATION.contains("test_iter_function_stop")
            && CPYTHON_MIGRATION.contains("test_iter_function_concealing_reentrant_exhaustion"),
        "focused iter/next migration notes must name the CPython builtin and iterator exhaustion sources"
    );
}

#[test]
fn aiter_anext_builtins_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_aiter_anext_builtin_subset(",
        "CPython builtin async-iterator entrypoint public behavior",
        "`aiter()` protocol validation separate from async-for lowering tests",
        "class AI",
        "def __aiter__(self):",
        "async def __anext__(self):",
        "raise StopAsyncIteration",
        "class AiterRaises",
        "raise ValueError(\"bad\")",
        "class BadAiter",
        "return 42",
        "hasattr(__import__(\"builtins\"), \"aiter\")",
        "callable(aiter)",
        "aiter(ai) is ai",
        "(\"missing\", lambda: aiter(()))",
        "(\"raises\", lambda: aiter(AiterRaises()))",
        "(\"bad\", lambda: aiter(BadAiter()))",
        "(\"arity0\", lambda: aiter())",
        "(\"arity2\", lambda: aiter(ai, ai))",
        "\"missing TypeError 'tuple' object is not an async iterable\"",
        "\"raises ValueError bad\"",
        "\"bad TypeError aiter() returned not an async iterator of type 'int'\"",
        "\"arity0 TypeError aiter() takes exactly one argument (0 given)\"",
        "\"arity2 TypeError aiter() takes exactly one argument (2 given)\"",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused aiter/anext builtin subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_aiter_anext_builtin_diff_subset");
    for required in [
        "Lib/test/test_builtin.py aiter()/anext() public async-iterator subset",
        "import builtins",
        "class AI",
        "def __aiter__(self):",
        "async def __anext__(self):",
        "raise StopAsyncIteration",
        "class AiterRaises",
        "raise ValueError(\"bad\")",
        "class BadAiter",
        "return 42",
        "hasattr(builtins, \"aiter\")",
        "callable(aiter)",
        "aiter(ai) is ai",
        "(\"missing\", lambda: aiter(()))",
        "(\"raises\", lambda: aiter(AiterRaises()))",
        "(\"bad\", lambda: aiter(BadAiter()))",
        "(\"arity0\", lambda: aiter())",
        "(\"arity2\", lambda: aiter(ai, ai))",
    ] {
        assert!(
            body.contains(required),
            "focused aiter/anext CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_aiter_anext_builtin_subset")
                && document.contains("cpython_aiter_anext_builtin_diff_subset")
                && document.contains("aiter()")
                && document.contains("__aiter__")
                && document.contains("__anext__"),
            "focused aiter/anext evidence must be documented in coverage and migration notes"
        );
    }
    assert!(
        CPYTHON_COVERAGE.contains("async-iterator return validation")
            && CPYTHON_COVERAGE.contains("missing-protocol `TypeError`s")
            && CPYTHON_COVERAGE.contains("one-argument arity"),
        "focused aiter/anext coverage notes must describe arity and protocol validation"
    );
    assert!(
        CPYTHON_MIGRATION.contains("one-argument arity")
            && CPYTHON_MIGRATION.contains("validation through `__anext__`")
            && CPYTHON_MIGRATION.contains("CPython-compatible propagation of")
            && CPYTHON_MIGRATION.contains("protocol errors"),
        "focused aiter/anext migration notes must describe protocol validation and error propagation"
    );
}

#[test]
fn stop_iteration_value_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_stop_iteration_value_subset(",
        "public StopIteration.value attribute",
        "without expanding into CPython's implementation-internal tests",
        "def g(value):",
        "if False:",
        "yield None",
        "return value",
        "for args in [(), (42,), (1, 2)]",
        "error = StopIteration(*args)",
        "error.args, error.value",
        "for value in [None, 99, (1, 2)]",
        "gen = g(value)",
        "except StopIteration as error",
        "class MyStop(StopIteration)",
        "custom = MyStop('x', 'y')",
        "\"ctor () () None\"",
        "\"ctor (42,) (42,) 42\"",
        "\"ctor (1, 2) (1, 2) 1\"",
        "\"gen None () None\"",
        "\"gen 99 (99,) 99\"",
        "\"gen (1, 2) ((1, 2),) (1, 2)\"",
        "\"sub ('x', 'y') x\"",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused StopIteration.value subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_stop_iteration_value_diff_subset");
    for required in [
        "Lib/test/test_generator.py public StopIteration.value behavior",
        "def g(value):",
        "if False:",
        "yield None",
        "return value",
        "for args in [(), (42,), (1, 2)]",
        "error = StopIteration(*args)",
        "error.args, error.value",
        "for value in [None, 99, (1, 2)]",
        "gen = g(value)",
        "except StopIteration as error",
        "class MyStop(StopIteration)",
        "custom = MyStop('x', 'y')",
    ] {
        assert!(
            body.contains(required),
            "focused StopIteration.value CPython diff evidence must cover `{required}`"
        );
    }

    assert!(
        CPYTHON_COVERAGE.contains("cpython_stop_iteration_value_diff_subset")
            && CPYTHON_COVERAGE.contains("cpython_stop_iteration_value_subset")
            && CPYTHON_COVERAGE.contains("public `StopIteration.value`")
            && CPYTHON_COVERAGE.contains("direct exception construction")
            && CPYTHON_COVERAGE.contains("generator return values")
            && CPYTHON_COVERAGE.contains("`StopIteration` subclasses"),
        "focused StopIteration.value coverage notes must document the public supported behavior"
    );
    assert!(
        CPYTHON_MIGRATION.contains("cpython_stop_iteration_value_subset")
            && CPYTHON_MIGRATION.contains("cpython_stop_iteration_value_diff_subset"),
        "focused StopIteration.value evidence must be listed in the migration manifest"
    );
}

#[test]
fn enumerate_zip_sorted_builtins_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_enumerate_zip_sorted_builtin_subset(",
        "Lib/test/test_enumerate.py::EnumerateTestCase",
        "Lib/test/test_builtin.py::BuiltinTest::test_zip / ::test_sorted",
        "class G",
        "class I",
        "class Ig",
        "e = enumerate(seq)",
        "iter(e) is e",
        "list(enumerate(seq))",
        "list(enumerate(G(seq)))",
        "list(enumerate(I(seq)))",
        "list(enumerate(Ig(seq)))",
        "next(enumerate(empty))",
        "list(enumerate(iterable=Ig(seq)))",
        "list(enumerate(iterable=Ig(seq), start=0))",
        "list(enumerate(start=0, iterable=Ig(seq)))",
        "lambda: enumerate()",
        "lambda: enumerate(1)",
        "lambda: enumerate('abc', 'a')",
        "lambda: enumerate(iterable=[], x=3)",
        "lambda: enumerate(X('abc'))",
        "lambda: enumerate(N('abc'))",
        "list(enumerate(E('abc')))",
        "ZeroDivisionError",
        "list(zip(a, b))",
        "list(zip(a, [4, 5, 6]))",
        "list(zip(a, (4, 5, 6, 7)))",
        "list(zip(a, I()))",
        "list(zip())",
        "list(zip(*[]))",
        "list(zip(range(5), range(10)))",
        "sorted([3, 1, 2])",
        "sorted([1, 2, 3], key=lambda x: -x)",
        "sorted([3, 1, 2], reverse=True)",
        "lambda: zip(None)",
        "lambda: zip(a, G())",
        "lambda: sorted()",
        "lambda: sorted([], bad=True)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused enumerate/zip/sorted subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(
        CPYTHON_DIFF,
        "cpython_enumerate_zip_sorted_builtin_diff_subset",
    );
    for required in [
        "Lib/test/test_enumerate.py::EnumerateTestCase and Lib/test/test_builtin.py::BuiltinTest::test_zip / ::test_sorted",
        "class G",
        "class I",
        "class Ig",
        "e = enumerate(seq)",
        "iter(e) is e",
        "list(enumerate(seq))",
        "list(enumerate(G(seq)))",
        "list(enumerate(I(seq)))",
        "list(enumerate(Ig(seq)))",
        "next(enumerate(empty))",
        "list(enumerate(iterable=Ig(seq)))",
        "list(enumerate(iterable=Ig(seq), start=0))",
        "list(enumerate(start=0, iterable=Ig(seq)))",
        "lambda: enumerate()",
        "lambda: enumerate(1)",
        "lambda: enumerate('abc', 'a')",
        "lambda: enumerate(iterable=[], x=3)",
        "list(zip(a, b))",
        "list(zip(a, [4, 5, 6]))",
        "list(zip(a, (4, 5, 6, 7)))",
        "list(zip(a, Z()))",
        "list(zip())",
        "list(zip(*[]))",
        "list(zip(range(5), range(10)))",
        "sorted([3, 1, 2])",
        "sorted([1, 2, 3], key=lambda x: -x)",
        "sorted([3, 1, 2], reverse=True)",
        "lambda: zip(None)",
        "lambda: zip(a, Bad())",
        "lambda: sorted()",
        "lambda: sorted([], bad=True)",
    ] {
        assert!(
            body.contains(required),
            "focused enumerate/zip/sorted CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_enumerate_zip_sorted_builtin_subset")
                && document.contains("cpython_enumerate_zip_sorted_builtin_diff_subset"),
            "focused enumerate/zip/sorted evidence must be documented in coverage and migration notes"
        );
    }
    assert!(
        CPYTHON_MIGRATION.contains("Lib/test/test_enumerate.py::EnumerateTestCase")
            && CPYTHON_MIGRATION.contains("BuiltinTest::test_zip")
            && CPYTHON_MIGRATION.contains("::test_sorted"),
        "focused enumerate/zip/sorted migration notes must name the CPython source tests"
    );
}

#[test]
fn sorted_exact_builtin_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_builtin_sorted_exact_subset(",
        "Lib/test/test_builtin.py::TestSorted",
        "deterministic shuffled input",
        "all four methods",
        "copy = [7, 0, 3, 1, 8, 2, 9, 4, 6, 5]",
        "sorted(copy)",
        "print(copy)",
        "sorted(copy, key=lambda x: -x)",
        "sorted(copy, reverse=True)",
        "sorted([], key=None)",
        "letters = sorted('abracadabra')",
        "len(letters), letters[0], letters[1], letters[-1]",
        "for T in [list, tuple, str, set, frozenset, dict.fromkeys]",
        "sorted(T(unique)) == ['a', 'b', 'c', 'd', 'r']",
        "lambda: sorted(iterable=[])",
        "lambda: sorted([], None)",
        "lambda: sorted('The quick Brown fox'.split(), None, lambda x, y: 0)",
        "\"[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]\"",
        "\"[7, 0, 3, 1, 8, 2, 9, 4, 6, 5]\"",
        "\"[9, 8, 7, 6, 5, 4, 3, 2, 1, 0]\"",
        "\"11 a a r\"",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused sorted exact subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_builtin_sorted_exact_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::TestSorted public sorted() subset",
        "copy = [7, 0, 3, 1, 8, 2, 9, 4, 6, 5]",
        "sorted(copy)",
        "print(copy)",
        "sorted(copy, key=lambda x: -x)",
        "sorted(copy, reverse=True)",
        "sorted([], key=None)",
        "letters = sorted('abracadabra')",
        "len(letters), letters[0], letters[1], letters[-1]",
        "for T in [list, tuple, str, set, frozenset, dict.fromkeys]",
        "sorted(T(unique)) == ['a', 'b', 'c', 'd', 'r']",
        "lambda: sorted(iterable=[])",
        "lambda: sorted([], None)",
        "lambda: sorted('The quick Brown fox'.split(), None, lambda x, y: 0)",
    ] {
        assert!(
            body.contains(required),
            "focused sorted exact CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_builtin_sorted_exact_subset")
                && document.contains("cpython_builtin_sorted_exact_diff_subset")
                && document.contains("Lib/test/test_builtin.py::TestSorted"),
            "focused sorted exact evidence must be documented in coverage and migration notes"
        );
    }
    assert!(
        CPYTHON_COVERAGE.contains("source-list preservation")
            && CPYTHON_COVERAGE.contains("key=")
            && CPYTHON_COVERAGE.contains("reverse=")
            && CPYTHON_COVERAGE.contains("iterable input type")
            && CPYTHON_COVERAGE.contains("positional/keyword argument rejection"),
        "focused sorted exact coverage notes must describe supported TestSorted behavior and argument rejection"
    );
    assert!(
        CPYTHON_MIGRATION.contains("all four current")
            && CPYTHON_MIGRATION.contains("without mutating the input list")
            && CPYTHON_MIGRATION.contains("`key=None`")
            && CPYTHON_MIGRATION.contains("accepted list/tuple/str/set/frozenset/dict-key")
            && CPYTHON_MIGRATION.contains("positional-only `iterable` rejection")
            && CPYTHON_MIGRATION.contains("legacy third positional")
            && CPYTHON_MIGRATION.contains("comparison-function")
            && CPYTHON_MIGRATION.contains("rejection"),
        "focused sorted exact migration notes must describe direct TestSorted behavior and argument rejection"
    );
}

#[test]
fn attribute_error_keyword_attributes_subset_is_source_migration_classified() {
    for required in [
        "fn cpython_attribute_error_keyword_attributes_subset(",
        "AttributeError('foo', name='name', obj='obj')",
        "error.args, error.name, error.obj",
        "AttributeError('foo', invalid='value')",
        "unexpected keyword argument 'invalid'",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused AttributeError keyword-attribute subset evidence must cover `{required}`"
        );
    }

    assert!(
        !CPYTHON_DIFF.contains("fn cpython_attribute_error_keyword_attributes_diff_subset("),
        "AttributeError keyword-attribute subset must not claim default CPython diff parity while the local oracle rejects name=/obj="
    );

    for required in [
        "cpython_attribute_error_keyword_attributes_subset",
        "local `python3` oracle predates this CPython behavior",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(required),
            "AttributeError keyword-attribute subset-only classification must document `{required}`"
        );
    }

    assert!(
        CPYTHON_COVERAGE.contains("cpython_attribute_error_keyword_attributes_subset"),
        "AttributeError keyword-attribute subset must remain in coverage notes"
    );
}

#[test]
fn unicode_error_attributes_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_unicode_error_attributes_subset(",
        "UnicodeError()",
        "UnicodeEncodeError('ascii', 'a', 0, 1, 'ordinal not in range')",
        "UnicodeDecodeError('ascii', bytearray(b'\\\\xff'), 0, 1, 'ordinal not in range')",
        "UnicodeTranslateError('\\\\u3042', 0, 1, 'ouch')",
        "error.encoding",
        "error.reason",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused UnicodeError attributes subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_unicode_error_attributes_diff_subset");
    for required in [
        "Lib/test/test_exceptions.py::testAttributes UnicodeError subset",
        "UnicodeError()",
        "UnicodeEncodeError('ascii', 'a', 0, 1, 'ordinal not in range')",
        "UnicodeDecodeError('ascii', bytearray(b'\\xff'), 0, 1, 'ordinal not in range')",
        "UnicodeTranslateError('\\u3042', 0, 1, 'ouch')",
        "error.encoding",
        "error.reason",
    ] {
        assert!(
            body.contains(required),
            "focused UnicodeError attributes CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_unicode_error_attributes_subset")
                && document.contains("cpython_unicode_error_attributes_diff_subset"),
            "focused UnicodeError attributes evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn syntax_error_attributes_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_syntax_error_attributes_subset(",
        "SyntaxError()",
        "SyntaxError('msgStr')",
        "('filenameStr', 'linenoStr', 'offsetStr', 'textStr', 'endLinenoStr', 'endOffsetStr')",
        "error.msg",
        "error.end_lineno",
        "error.print_file_and_line",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused SyntaxError attributes subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_syntax_error_attributes_diff_subset");
    for required in [
        "Lib/test/test_exceptions.py::testAttributes SyntaxError stable subset",
        "SyntaxError()",
        "SyntaxError('msgStr')",
        "('filenameStr', 'linenoStr', 'offsetStr', 'textStr')",
        "error.msg",
        "getattr(error, 'end_lineno', None)",
        "error.print_file_and_line",
    ] {
        assert!(
            body.contains(required),
            "focused SyntaxError attributes CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_syntax_error_attributes_subset")
                && document.contains("cpython_syntax_error_attributes_diff_subset"),
            "focused SyntaxError attributes evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn system_exit_oserror_attributes_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_system_exit_oserror_attributes_subset(",
        "SystemExit('foo')",
        "system.args, system.code",
        "OSError('foo', 'bar', 'baz')",
        "OSError('foo', 'bar', 'baz', None, 'quux')",
        "error.errno",
        "error.filename2",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused SystemExit/OSError subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(
        CPYTHON_DIFF,
        "cpython_system_exit_oserror_attributes_diff_subset",
    );
    for required in [
        "Lib/test/test_exceptions.py::testAttributes SystemExit/OSError subset",
        "SystemExit('foo')",
        "system.args, system.code",
        "OSError('foo', 'bar', 'baz')",
        "OSError('foo', 'bar', 'baz', None, 'quux')",
        "error.errno",
        "error.filename2",
    ] {
        assert!(
            body.contains(required),
            "focused SystemExit/OSError CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_system_exit_oserror_attributes_subset")
                && document.contains("cpython_system_exit_oserror_attributes_diff_subset"),
            "focused SystemExit/OSError evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn base_exception_with_traceback_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_base_exception_with_traceback_subset(",
        "raise IndexError(4)",
        "error.__traceback__",
        "error.with_traceback(tb)",
        "error.with_traceback(None)",
        "Exception().with_traceback(5)",
        "Exception().__traceback__ = 5",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused BaseException with_traceback subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(
        CPYTHON_DIFF,
        "cpython_base_exception_with_traceback_diff_subset",
    );
    for required in [
        "Lib/test/test_exceptions.py::testWithTraceback / ::testInvalidTraceback public subset",
        "raise IndexError(4)",
        "error.__traceback__",
        "error.with_traceback(tb)",
        "error.with_traceback(None)",
        "Exception().with_traceback(5)",
        "Exception().__traceback__ = 5",
    ] {
        assert!(
            body.contains(required),
            "focused BaseException with_traceback CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_base_exception_with_traceback_subset")
                && document.contains("cpython_base_exception_with_traceback_diff_subset"),
            "focused BaseException with_traceback evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn base_exception_args_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_base_exception_args_subset(",
        "Exception()",
        "Exception('foo')",
        "Exception('foo', 1)",
        "ValueError(3)",
        "raise Exception('caught', 7)",
        "error.args",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused BaseException args/display subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_base_exception_args_diff_subset");
    for required in [
        "Lib/test/test_exceptions.py::testAttributes BaseException args/display subset",
        "Exception()",
        "Exception('foo')",
        "Exception('foo', 1)",
        "ValueError(3)",
        "raise Exception('caught', 7)",
        "error.args",
    ] {
        assert!(
            body.contains(required),
            "focused BaseException args/display CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_base_exception_args_subset")
                && document.contains("cpython_base_exception_args_diff_subset"),
            "focused BaseException args/display evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn builtin_exception_hierarchy_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_builtin_exception_hierarchy_subset(",
        "raise OverflowError('big')",
        "except ArithmeticError as error",
        "except LookupError as error",
        "OverflowError.__bases__[0].__name__",
        "GeneratorExit('stop')",
        "except BaseException as error",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused builtin exception hierarchy subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(
        CPYTHON_DIFF,
        "cpython_builtin_exception_hierarchy_diff_subset",
    );
    for required in [
        "Lib/test/test_exceptions.py builtin exception hierarchy public subset",
        "raise OverflowError('big')",
        "except ArithmeticError as error",
        "except LookupError as error",
        "OverflowError.__bases__[0].__name__",
        "GeneratorExit('stop')",
        "except BaseException as error",
    ] {
        assert!(
            body.contains(required),
            "focused builtin exception hierarchy CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains("cpython_builtin_exception_hierarchy_subset")
                && document.contains("cpython_builtin_exception_hierarchy_diff_subset"),
            "focused builtin exception hierarchy evidence must be documented in coverage and migration notes"
        );
    }
}

#[test]
fn builtin_getattr_public_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_builtin_getattr_public_subset(",
        "getattr(sys, 'spam')",
        "getattr(sys, 'missing', 'fallback')",
        "getattr(box, 'value')",
        "getattr(box, 'label')",
        "getattr(Box, 'label')",
        "getattr(sys, 'stdout') is sys.stdout",
        "chr(0x10ffff)",
        "lambda: getattr(1, 2)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused getattr subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_builtin_getattr_public_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_getattr public supported subset",
        "getattr(sys, 'spam')",
        "getattr(sys, 'missing', 'fallback')",
        "getattr(box, 'value')",
        "getattr(box, 'label')",
        "getattr(Box, 'label')",
        "getattr(sys, 'stdout') is sys.stdout",
        "chr(0x10ffff)",
        "lambda: getattr(1, 2)",
    ] {
        assert!(
            body.contains(required),
            "focused getattr CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, MANIFEST] {
        assert!(
            document.contains("cpython_builtin_getattr_public_subset")
                && document.contains("cpython_builtin_getattr_public_diff_subset"),
            "focused getattr evidence must be documented in coverage and CPython test manifest"
        );
    }
}

#[test]
fn builtin_none_ne_direct_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_builtin_none_ne_direct_subset(",
        "None.__ne__(None)",
        "None.__ne__(0) is NotImplemented",
        "object.__eq__(left, right) is NotImplemented",
        "object.__ne__(left, right) is NotImplemented",
        "lambda: None.__ne__()",
        "lambda: object.__ne__(None, 0, 1)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused None.__ne__ subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_builtin_none_ne_direct_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test___ne__",
        "None.__ne__(None)",
        "None.__ne__(0) is NotImplemented",
        "object.__eq__(left, right) is NotImplemented",
        "object.__ne__(left, right) is NotImplemented",
        "lambda: None.__ne__()",
        "lambda: object.__ne__(None, 0, 1)",
    ] {
        assert!(
            body.contains(required),
            "focused None.__ne__ CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, MANIFEST] {
        assert!(
            document.contains("cpython_builtin_none_ne_direct_subset")
                && document.contains("cpython_builtin_none_ne_direct_diff_subset"),
            "focused None.__ne__ evidence must be documented in coverage and CPython test manifest"
        );
    }
}

#[test]
fn object_repr_str_direct_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_object_repr_str_direct_subset(",
        "object.__repr__(custom)",
        "object.__str__(custom)",
        "object.__str__(BadRepr())",
        "object.__str__(L([1]))",
        "object.__repr__(object=plain)",
        "object.__str__(object=plain)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused object repr/str subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_object_repr_str_direct_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_repr public object descriptor subset",
        "object.__repr__(custom)",
        "object.__str__(custom)",
        "object.__str__(BadRepr())",
        "object.__str__(L([1]))",
        "object.__repr__(object=plain)",
        "object.__str__(object=plain)",
    ] {
        assert!(
            body.contains(required),
            "focused object repr/str CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, MANIFEST] {
        assert!(
            document.contains("cpython_object_repr_str_direct_subset")
                && document.contains("cpython_object_repr_str_direct_diff_subset"),
            "focused object repr/str evidence must be documented in coverage and CPython test manifest"
        );
    }
}

#[test]
fn str_builtin_custom_dunder_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_str_builtin_custom_dunder_subset(",
        "str(custom)",
        "f'{custom}'",
        "'%s' % custom",
        "object.__format__(custom, '')",
        "str(Bad())",
        "str(Raises())",
        "instance_only.__str__",
        "format-priority",
        "str-sub",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused str custom-dunder subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(
        CPYTHON_DIFF,
        "cpython_str_builtin_custom_dunder_diff_subset",
    );
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_repr / ::test_format public str dispatch subset",
        "str(custom)",
        "f'{custom}'",
        "'%s' % custom",
        "object.__format__(custom, '')",
        "str(Bad())",
        "str(Raises())",
        "instance_only.__str__",
        "format-priority",
        "str-sub",
    ] {
        assert!(
            body.contains(required),
            "focused str custom-dunder CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, MANIFEST] {
        assert!(
            document.contains("cpython_str_builtin_custom_dunder_subset")
                && document.contains("cpython_str_builtin_custom_dunder_diff_subset"),
            "focused str custom-dunder evidence must be documented in coverage and CPython test manifest"
        );
    }
}

#[test]
fn builtin_setattr_delattr_public_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_builtin_setattr_delattr_public_subset(",
        "setattr(sys, 'eggs', 7)",
        "delattr(sys, 'eggs')",
        "setattr(box, 'value', 3)",
        "setattr(Box, 'label', 'box')",
        "delattr(box, 'value')",
        "delattr(Box, 'label')",
        "lambda: setattr(1, 2, 3)",
        "lambda: delattr(1, 2)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused setattr/delattr subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(
        CPYTHON_DIFF,
        "cpython_builtin_setattr_delattr_public_diff_subset",
    );
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_setattr / ::test_delattr public supported subset",
        "setattr(sys, 'eggs', 7)",
        "delattr(sys, 'eggs')",
        "setattr(box, 'value', 3)",
        "setattr(Box, 'label', 'box')",
        "delattr(box, 'value')",
        "delattr(Box, 'label')",
        "lambda: setattr(1, 2, 3)",
        "lambda: delattr(1, 2)",
    ] {
        assert!(
            body.contains(required),
            "focused setattr/delattr CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, MANIFEST] {
        assert!(
            document.contains("cpython_builtin_setattr_delattr_public_subset")
                && document.contains("cpython_builtin_setattr_delattr_public_diff_subset"),
            "focused setattr/delattr evidence must be documented in coverage and CPython test manifest"
        );
    }
}

#[test]
fn builtin_hasattr_public_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_builtin_hasattr_public_subset(",
        "hasattr(sys, 'probe')",
        "hasattr(sys, 'missing')",
        "hasattr(box, 'value')",
        "hasattr(box, 'label')",
        "hasattr(Box, 'label')",
        "chr(0x10ffff)",
        "raise AttributeError(name)",
        "raise SystemExit('exit')",
        "raise ValueError('bad')",
        "lambda: hasattr(1, 2)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused hasattr subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_builtin_hasattr_public_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_hasattr public supported subset",
        "hasattr(sys, 'probe')",
        "hasattr(sys, 'missing')",
        "hasattr(box, 'value')",
        "hasattr(box, 'label')",
        "hasattr(Box, 'label')",
        "chr(0x10ffff)",
        "raise AttributeError(name)",
        "raise SystemExit('exit')",
        "raise ValueError('bad')",
        "lambda: hasattr(1, 2)",
    ] {
        assert!(
            body.contains(required),
            "focused hasattr CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, MANIFEST] {
        assert!(
            document.contains("cpython_builtin_hasattr_public_subset")
                && document.contains("cpython_builtin_hasattr_public_diff_subset"),
            "focused hasattr evidence must be documented in coverage and CPython test manifest"
        );
    }
}

#[test]
fn builtin_callable_public_subset_has_focused_diff_evidence() {
    for required in [
        "fn cpython_builtin_callable_public_subset(",
        "callable(len)",
        "callable('a')",
        "callable(callable)",
        "callable(f)",
        "callable(Plain)",
        "callable(plain)",
        "callable(wm.meth)",
        "plain.__call__ = lambda: 1",
        "def __call__(self, value):",
        "callable(cc)",
        "callable(child)",
        "lambda: callable(1, 2)",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(required),
            "focused callable subset evidence must cover `{required}`"
        );
    }

    let body = extract_rust_test_body(CPYTHON_DIFF, "cpython_builtin_callable_public_diff_subset");
    for required in [
        "Lib/test/test_builtin.py::BuiltinTest::test_callable public supported subset",
        "callable(len)",
        "callable('a')",
        "callable(callable)",
        "callable(f)",
        "callable(Plain)",
        "callable(plain)",
        "callable(wm.meth)",
        "plain.__call__ = lambda: 1",
        "def __call__(self, value):",
        "callable(cc)",
        "callable(child)",
        "lambda: callable(1, 2)",
    ] {
        assert!(
            body.contains(required),
            "focused callable CPython diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, MANIFEST] {
        assert!(
            document.contains("cpython_builtin_callable_public_subset")
                && document.contains("cpython_builtin_callable_public_diff_subset"),
            "focused callable evidence must be documented in coverage and CPython test manifest"
        );
    }
}

#[test]
fn exec_closure_subset_stays_documented_and_version_gated() {
    let subset_name = "cpython_exec_closure_subset";
    let diff_name = "cpython_exec_closure_diff_subset";

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "exec closure runtime subset evidence must exist"
    );
    assert!(
        !CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "exec closure subset must not be mistaken for same-named direct CPython parity"
    );

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        for required in [subset_name, "CellType", "__closure__", "closure="] {
            assert!(
                document.contains(required),
                "exec closure docs must mention `{required}`"
            );
        }
    }
    assert!(
        MANIFEST.contains(
            "Host CPython version differences keep this method out of differential parity",
        ),
        "test manifest must document why exec closure stays outside direct CPython diff parity"
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "builtins")
        .expect("sandbox stdlib manifest must include builtins");
    assert!(
        row.supported_surface.contains(subset_name),
        "builtins sandbox manifest must list exec closure subset evidence"
    );
    assert!(
        !row.diff_evidence.contains(diff_name),
        "builtins sandbox manifest must not cite same-named direct CPython diff evidence for version-gated exec closure"
    );
}

#[test]
fn breakpoint_default_stub_stays_sandbox_only() {
    let subset_name = "cpython_builtin_breakpoint_default_stub_subset";
    let diff_name = "cpython_builtin_breakpoint_default_stub_diff_subset";

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "sandbox breakpoint default-stub subset evidence must exist"
    );
    assert!(
        !CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "sandbox breakpoint default-stub behavior must not be represented as direct CPython parity"
    );

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION, MANIFEST] {
        assert!(
            document.contains(subset_name)
                && document.contains("sandbox no-op")
                && document.contains("pdb-backed")
                && document.contains("PYTHONBREAKPOINT"),
            "breakpoint sandbox docs must keep the default-stub subset and CPython pdb/PYTHONBREAKPOINT stop line together"
        );
    }

    for module in ["builtins", "sys"] {
        let row = sandbox_stdlib_rows()
            .into_iter()
            .find(|row| row.module == module)
            .unwrap_or_else(|| panic!("sandbox stdlib manifest must include {module}"));
        assert!(
            row.supported_surface.contains(subset_name),
            "{module} sandbox manifest must list the breakpoint default-stub subset"
        );
        assert!(
            !row.diff_evidence.contains(diff_name),
            "{module} sandbox manifest must not cite direct CPython diff evidence for the sandbox-only default stub"
        );
    }
}

#[test]
fn builtins_host_io_and_default_debugger_stop_line_stays_out_of_scope() {
    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "builtins")
        .expect("sandbox stdlib manifest must include builtins");

    for excluded in [
        "`open()`",
        "`input()`",
        "host TTY behavior",
        "default pdb-backed breakpoint behavior",
        "process/environment side effects",
    ] {
        assert!(
            row.excluded_surface.contains(excluded),
            "builtins sandbox manifest must keep `{excluded}` outside the supported surface"
        );
    }

    assert!(
        row.supported_surface
            .contains("cpython_builtin_breakpoint_default_stub_subset"),
        "builtins sandbox manifest must list the default breakpoint stub only as local subset evidence"
    );
    assert!(
        !row.diff_evidence
            .contains("cpython_builtin_breakpoint_default_stub_diff_subset"),
        "builtins sandbox manifest must not cite CPython diff parity for the default breakpoint stub"
    );

    let stub_start = CPYTHON_SUBSET
        .find("fn cpython_builtin_breakpoint_default_stub_subset()")
        .expect("breakpoint default-stub subset evidence must be extractable");
    let stub_end = CPYTHON_SUBSET[stub_start..]
        .find("\n#[test]")
        .map(|offset| stub_start + offset)
        .unwrap_or(CPYTHON_SUBSET.len());
    let stub_source = &CPYTHON_SUBSET[stub_start..stub_end];
    for required in [
        "breakpoint None",
        "hook None",
        "dunder None",
        "sys.__breakpointhook__(1, key=2)",
    ] {
        assert!(
            stub_source.contains(required),
            "breakpoint default-stub subset evidence must cover `{required}`"
        );
    }

    for required in ["pdb", "breakpoint", "PYTHONBREAKPOINT"] {
        assert!(
            CPYTHON_COVERAGE.contains(required) && CPYTHON_MIGRATION.contains(required),
            "builtins sandbox docs must keep debugger stop-line term `{required}` documented"
        );
    }

    for required in [
        "Host I/O integration",
        "real `open()`",
        "TTY behavior",
        "`input()`",
        "pty",
        "Default `pdb` integration",
        "breakpoint()",
        "environment-variable",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(required),
            "migration document must keep host/debugger out-of-scope term `{required}`"
        );
    }
}

#[test]
fn types_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "types",
        &[
            "cpython_types_names_public_surface_subset",
            "cpython_types_singleton_type_aliases_subset",
            "cpython_types_module_type_subset",
            "cpython_types_runtime_type_aliases_subset",
            "cpython_types_generic_alias_union_type_subset",
            "cpython_types_union_public_operator_and_classinfo_subset",
            "cpython_types_union_forward_ref_subset",
            "cpython_types_union_typevar_parameter_subset",
            "cpython_types_union_parameter_substitution_subset",
            "cpython_types_union_bad_classinfo_checks_subset",
            "cpython_types_union_newtype_subset",
            "cpython_types_mappingproxy_exact_dict_subset",
            "cpython_types_mappingproxy_method_surface_subset",
            "cpython_types_mappingproxy_custom_mapping_subset",
            "cpython_types_mappingproxy_union_subset",
            "cpython_types_mappingproxy_hash_subset",
            "cpython_types_mappingproxy_richcompare_subset",
            "cpython_types_mappingproxy_contains_subset",
            "cpython_types_mappingproxy_views_subset",
            "cpython_types_mappingproxy_len_subset",
            "cpython_types_mappingproxy_iterators_subset",
            "cpython_types_mappingproxy_reversed_subset",
            "cpython_types_mappingproxy_copy_subset",
            "cpython_types_simple_namespace_basic_subset",
            "cpython_types_simple_namespace_recursive_and_replace_subset",
            "cpython_types_simple_namespace_new_and_invalid_replace_subset",
            "cpython_types_simple_namespace_remaining_public_subset",
            "cpython_types_class_creation_new_class_resolve_bases_subset",
            "cpython_types_class_creation_new_class_meta_helper_subset",
            "cpython_types_class_creation_new_class_metaclass_keywords_subset",
            "cpython_types_class_creation_prepare_resolve_bases_subset",
            "cpython_types_class_creation_mro_entries_core_subset",
            "cpython_types_class_creation_mro_entries_multiple_subset",
            "cpython_types_class_creation_prepare_and_metaclass_callable_subset",
            "cpython_types_class_creation_metaclass_override_function_subset",
            "cpython_types_class_creation_non_type_metaclass_derivation_subset",
            "cpython_types_class_creation_metaclass_derivation_subset",
            "cpython_types_class_creation_one_argument_type_subset",
            "cpython_types_coroutine_public_subset",
            "cpython_types_coroutine_async_def_subset",
            "cpython_types_coroutine_generator_wrapper_subset",
            "cpython_types_coroutine_generator_frame_subset",
            "cpython_types_coroutine_generator_yieldfrom_subset",
            "cpython_types_coroutine_duck_generator_wrapper_subset",
            "cpython_types_coroutine_duck_generator_await_subset",
            "cpython_types_coroutine_duck_generator_proxy_subset",
            "cpython_types_function_type_subset",
            "cpython_types_code_traceback_type_aliases_subset",
            "cpython_types_frame_type_alias_subset",
            "cpython_types_slot_and_method_wrapper_types_subset",
            "cpython_types_frame_locals_proxy_type_subset",
        ],
        &[
            "CPython object-layout internals",
            "exact C descriptor types",
            "pickle identity matrices",
            "interpreter lifecycle behavior",
        ],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "types")
        .expect("sandbox stdlib manifest must include types");
    for evidence in [
        "cpython_types_names_public_surface_diff_subset",
        "cpython_types_singleton_type_aliases_diff_subset",
        "cpython_types_module_type_diff_subset",
        "cpython_types_generic_alias_union_type_diff_subset",
        "cpython_types_union_public_operator_and_classinfo_diff_subset",
        "cpython_types_union_forward_ref_diff_subset",
        "cpython_types_union_forward_get_type_hints_diff_subset",
        "cpython_types_union_typevar_parameter_diff_subset",
        "cpython_types_union_parameter_substitution_diff_subset",
        "cpython_types_union_copy_pickle_diff_subset",
        "cpython_types_union_bad_module_guard_diff_subset",
        "cpython_types_union_genericalias_subclass_bad_eq_diff_subset",
        "cpython_types_union_bad_classinfo_checks_diff_subset",
        "cpython_types_union_unhashable_metaclass_diff_subset",
        "cpython_types_union_dynamic_hashability_diff_subset",
        "cpython_types_union_newtype_diff_subset",
        "cpython_types_union_io_diff_subset",
        "cpython_types_union_typed_dict_diff_subset",
        "cpython_types_union_protocol_diff_subset",
        "cpython_types_union_special_form_diff_subset",
        "cpython_types_union_literal_diff_subset",
        "cpython_types_class_creation_new_class_meta_helper_diff_subset",
        "cpython_types_class_creation_one_argument_type_diff_subset",
        "cpython_types_class_creation_get_original_bases_diff_subset",
        "cpython_types_class_creation_metaclass_new_error_diff_subset",
        "cpython_types_class_creation_subclass_inherited_slot_update_diff_subset",
        "cpython_types_class_creation_mro_entries_core_diff_subset",
        "cpython_types_class_creation_mro_entries_multiple_diff_subset",
        "cpython_types_class_creation_prepare_resolve_bases_diff_subset",
        "cpython_types_class_creation_prepare_and_metaclass_callable_diff_subset",
        "cpython_types_class_creation_metaclass_override_function_diff_subset",
        "cpython_types_class_creation_non_type_metaclass_derivation_diff_subset",
        "cpython_types_class_creation_metaclass_derivation_diff_subset",
        "cpython_types_class_creation_new_class_resolve_bases_diff_subset",
        "cpython_types_coroutine_public_diff_subset",
        "cpython_types_coroutine_async_def_diff_subset",
        "cpython_types_coroutine_generator_wrapper_diff_subset",
        "cpython_types_coroutine_generator_frame_diff_subset",
        "cpython_types_coroutine_generator_yieldfrom_diff_subset",
        "cpython_types_coroutine_duck_generator_wrapper_diff_subset",
        "cpython_types_coroutine_duck_generator_await_diff_subset",
        "cpython_types_coroutine_duck_generator_proxy_diff_subset",
        "cpython_types_function_type_diff_subset",
        "cpython_types_code_traceback_type_aliases_diff_subset",
        "cpython_types_frame_type_alias_diff_subset",
        "cpython_types_runtime_type_aliases_diff_subset",
        "cpython_types_float_constructor_edges_diff_subset",
        "cpython_types_float_to_string_diff_subset",
        "cpython_types_normal_integers_diff_subset",
        "cpython_types_format_spec_errors_diff_subset",
        "cpython_types_mappingproxy_exact_dict_diff_subset",
        "cpython_types_mappingproxy_method_surface_diff_subset",
        "cpython_types_mappingproxy_union_diff_subset",
        "cpython_types_mappingproxy_hash_diff_subset",
        "cpython_types_mappingproxy_contains_diff_subset",
        "cpython_types_mappingproxy_views_diff_subset",
        "cpython_types_mappingproxy_missing_diff_subset",
        "cpython_types_mappingproxy_len_diff_subset",
        "cpython_types_mappingproxy_iterators_diff_subset",
        "cpython_types_mappingproxy_reversed_diff_subset",
        "cpython_types_mappingproxy_copy_diff_subset",
        "cpython_types_mappingproxy_richcompare_diff_subset",
        "cpython_types_mappingproxy_custom_mapping_diff_subset",
        "cpython_types_mappingproxy_chainmap_diff_subset",
        "cpython_types_simple_namespace_basic_diff_subset",
        "cpython_types_simple_namespace_recursive_diff_subset",
        "cpython_types_simple_namespace_new_and_invalid_replace_diff_subset",
        "cpython_types_simple_namespace_remaining_public_diff_subset",
        "cpython_types_simple_namespace_state_order_diff_subset",
        "cpython_types_simple_namespace_fake_comparison_diff_subset",
        "cpython_types_method_descriptor_types_diff_subset",
        "cpython_types_slot_and_method_wrapper_types_diff_subset",
        "cpython_types_frame_locals_proxy_type_diff_subset",
        "cpython_types_int_format_diff_subset",
        "cpython_types_float_format_diff_subset",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "types sandbox manifest must cite CPython diff evidence `{evidence}`"
        );
    }
}

#[test]
fn types_coroutine_diff_covers_generator_async_runtime_subsets() {
    for (subset, diff) in [
        (
            "cpython_types_coroutine_public_subset",
            "cpython_types_coroutine_public_diff_subset",
        ),
        (
            "cpython_types_coroutine_async_def_subset",
            "cpython_types_coroutine_async_def_diff_subset",
        ),
        (
            "cpython_types_coroutine_generator_wrapper_subset",
            "cpython_types_coroutine_generator_wrapper_diff_subset",
        ),
        (
            "cpython_types_coroutine_generator_frame_subset",
            "cpython_types_coroutine_generator_frame_diff_subset",
        ),
        (
            "cpython_types_coroutine_generator_yieldfrom_subset",
            "cpython_types_coroutine_generator_yieldfrom_diff_subset",
        ),
        (
            "cpython_types_coroutine_duck_generator_wrapper_subset",
            "cpython_types_coroutine_duck_generator_wrapper_diff_subset",
        ),
        (
            "cpython_types_coroutine_duck_generator_await_subset",
            "cpython_types_coroutine_duck_generator_await_diff_subset",
        ),
        (
            "cpython_types_coroutine_duck_generator_proxy_subset",
            "cpython_types_coroutine_duck_generator_proxy_diff_subset",
        ),
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "types.coroutine runtime subset evidence `{subset}` must exist"
        );
        assert!(
            CPYTHON_DIFF.contains(&format!("fn {diff}(")),
            "types.coroutine CPython diff evidence `{diff}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(subset) && CPYTHON_COVERAGE.contains(diff),
            "coverage document must link types.coroutine evidence `{subset}` / `{diff}`"
        );
        assert!(
            CPYTHON_MIGRATION.contains(subset),
            "migration document must describe types.coroutine runtime subset `{subset}`"
        );
    }

    let public_diff = CPYTHON_DIFF
        .split("fn cpython_types_coroutine_public_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_types_coroutine_async_def_diff_subset()")
                .next()
        })
        .expect("types.coroutine public diff evidence must be extractable");
    for required in [
        "inspect.CO_ITERABLE_COROUTINE",
        "types.CoroutineType",
        "returns_itercoro() is gencoro",
        "types.coroutine(types.coroutine(gen)) is gen",
    ] {
        assert!(
            public_diff.contains(required),
            "types.coroutine public diff evidence must cover `{required}`"
        );
    }

    let wrapper_diff = CPYTHON_DIFF
        .split("fn cpython_types_coroutine_generator_wrapper_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_types_coroutine_generator_frame_diff_subset()")
                .next()
        })
        .expect("types.coroutine wrapper diff evidence must be extractable");
    for required in [
        "isinstance(wrapper, types._GeneratorWrapper)",
        "isinstance(wrapper, collections.abc.Coroutine)",
        "wrapper.cr_code is exact_gen.gi_code",
        "wrapper.__await__() is exact_gen",
        "wrapper.throw(Exception('ham'))",
    ] {
        assert!(
            wrapper_diff.contains(required),
            "types.coroutine wrapper diff evidence must cover `{required}`"
        );
    }

    let frame_diff = CPYTHON_DIFF
        .split("fn cpython_types_coroutine_generator_frame_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_types_coroutine_generator_yieldfrom_diff_subset()")
                .next()
        })
        .expect("types.coroutine frame diff evidence must be extractable");
    for required in [
        "wrapper.cr_frame is gen.gi_frame",
        "wrapper.cr_code is gen.gi_code",
        "gen.gi_frame.f_code is gen.gi_code",
        "wrapper.cr_frame is None",
    ] {
        assert!(
            frame_diff.contains(required),
            "types.coroutine frame diff evidence must cover `{required}`"
        );
    }

    let yieldfrom_diff = CPYTHON_DIFF
        .split("fn cpython_types_coroutine_generator_yieldfrom_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_types_coroutine_duck_generator_wrapper_diff_subset()")
                .next()
        })
        .expect("types.coroutine yield-from diff evidence must be extractable");
    for required in [
        "result = yield from inner()",
        "wrapper.gi_yieldfrom is gen.gi_yieldfrom",
        "wrapper.cr_await is gen.gi_yieldfrom",
        "wrapper.gi_yieldfrom is None",
    ] {
        assert!(
            yieldfrom_diff.contains(required),
            "types.coroutine yield-from diff evidence must cover `{required}`"
        );
    }

    let duck_await_diff = CPYTHON_DIFF
        .split("fn cpython_types_coroutine_duck_generator_await_diff_subset()")
        .nth(1)
        .and_then(|tail| {
            tail.split("fn cpython_types_coroutine_duck_generator_proxy_diff_subset()")
                .next()
        })
        .expect("types.coroutine duck-generator await diff evidence must be extractable");
    for required in [
        "return await foo() + 100",
        "coro.send(None)",
        "coro.send(20)",
        "ex.args[0]",
    ] {
        assert!(
            duck_await_diff.contains(required),
            "types.coroutine duck-generator await diff evidence must cover `{required}`"
        );
    }
}

#[test]
fn types_new_class_metaclass_keywords_diff_covers_runtime_subset() {
    let subset_name = "cpython_types_class_creation_new_class_metaclass_keywords_subset";
    let diff_name = "cpython_types_class_creation_new_class_meta_helper_diff_subset";

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "types.new_class metaclass-keyword runtime subset evidence must exist"
    );
    assert!(
        CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
        "types.new_class meta-helper CPython diff evidence must exist"
    );

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(subset_name) && document.contains(diff_name),
            "types docs must link `{subset_name}` to `{diff_name}`"
        );
    }

    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("types.new_class meta-helper diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for required in [
        "def meta_func(name, bases, ns, **kw):",
        "types.new_class('X', (int, object), dict(metaclass=meta_func, x=0))",
        "res[3] == {'x': 0}",
    ] {
        assert!(
            diff_source.contains(required),
            "types.new_class metaclass keyword diff evidence must cover `{required}`"
        );
    }
}

#[test]
fn types_singleton_alias_diff_evidence_stays_capability_gated() {
    let start = CPYTHON_DIFF
        .find("fn cpython_types_singleton_type_aliases_diff_subset()")
        .expect("types singleton alias diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    assert!(
        body.contains("hasattr(types, 'NoneType')")
            && body.contains("hasattr(types, 'NotImplementedType')")
            && body.contains("hasattr(types, 'EllipsisType')")
            && body.contains("skipping types singleton aliases diff"),
        "types singleton alias diff evidence must stay gated for older CPython oracles"
    );
}

#[test]
fn types_mappingproxy_hash_diff_evidence_stays_capability_gated() {
    let start = CPYTHON_DIFF
        .find("fn cpython_types_mappingproxy_hash_diff_subset()")
        .expect("types mappingproxy hash diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    assert!(
        body.contains("hash(MappingProxyType(M()))")
            && body.contains("skipping types mappingproxy hash diff"),
        "types mappingproxy hash diff evidence must stay gated for older CPython oracles"
    );
}

#[test]
fn zip_strict_diff_evidence_stays_capability_gated() {
    let start = CPYTHON_DIFF
        .find("fn cpython_zip_strict_builtin_diff_subset()")
        .expect("zip strict diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    assert!(
        body.contains("zip([1], [2], strict=True)") && body.contains("skipping zip(strict) diff"),
        "zip strict diff evidence must stay gated for older CPython oracles"
    );
}

#[test]
fn map_strict_diff_evidence_stays_capability_gated() {
    let start = CPYTHON_DIFF
        .find("fn cpython_map_strict_builtin_diff_subset()")
        .expect("map strict diff evidence must exist");
    let body = &CPYTHON_DIFF[start..];
    let end = body.find("\n#[test]").unwrap_or(body.len());
    let body = &body[..end];

    assert!(
        body.contains("map(pack, [1], [2], strict=True)")
            && body.contains("skipping map(strict) diff"),
        "map strict diff evidence must stay gated for older CPython oracles"
    );
}

#[test]
fn cpython_migration_documents_sandbox_stdlib_diff_and_runtime_subset_evidence() {
    for required in [
        "`cpython_diff` oracle evidence",
        "and either local `cpython_subset`",
        "local `cpython_subset`",
        "runtime guard evidence",
        "matching runtime subset evidence",
        "local runtime evidence",
        "direct CPython diff evidence plus local subset/runtime evidence",
        "not that the full CPython module has been cloned",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(required),
            "migration document must mention sandbox stdlib evidence rule `{required}`"
        );
    }
}

#[test]
fn cpython_migration_documents_default_oracle_only_bytearray_subset_boundaries() {
    for required in [
        "default CPython oracle used by `cpython_diff` in this workspace",
        "does not expose `bytearray.take_bytes()`",
        "CPython-version-gated direct diff evidence",
        "`cpython_bytearray_resize_diff_subset`",
        "`cpython_bytearray_resize_forbidden_diff_subset`",
        "`cpython_bytearray_search_reentrancy_buffererror_diff_subset`",
        "local subset evidence rather than direct `cpython_diff` evidence",
        "does not expose",
        "current public `__buffer__`",
        "protocol behavior",
        "historical corrupted-bytearray",
        "current",
        "CPython's regression test prevents",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(required),
            "migration document must mention default-oracle bytearray subset boundary `{required}`"
        );
    }

    for required in [
        "default CPython",
        "oracle used by `cpython_diff` in this workspace",
        "CPython-version-gated",
        "`cpython_bytearray_resize_diff_subset`",
        "`cpython_bytearray_resize_forbidden_diff_subset`",
        "`cpython_bytearray_search_reentrancy_buffererror_diff_subset`",
        "does not expose",
        "`bytearray.take_bytes()`",
        "remains local subset evidence",
        "current public `__buffer__`",
        "protocol behavior",
        "historical corrupted-bytearray",
        "current",
        "CPython's regression test prevents",
    ] {
        assert!(
            CPYTHON_COVERAGE.contains(required),
            "coverage document must mention default-oracle bytearray subset boundary `{required}`"
        );
    }
}

#[test]
fn cpython_migration_documents_sandbox_stdlib_allow_list_semantics() {
    for required in [
        "Sandbox import policy is allow-list based",
        "package entries cover",
        "their child modules",
        "SandboxPolicy::deny_stdlib()",
        "must be explicitly allowed",
        "sandbox_policy_denies_stdlib_imports",
        "sandbox_policy_denies_required_sandbox_stdlib_surface",
        "sandbox_policy_allows_required_sandbox_stdlib_surface",
        "sandbox_policy_required_stdlib_allow_list_excludes_compatibility_shims",
        "sandbox_policy_requires_explicit_allow_for_extra_stdlib_shims",
        "stdlib_create_module_registry_is_classified_by_scope",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(required),
            "migration document must mention sandbox allow-list semantic `{required}`"
        );
    }
}

#[test]
fn cpython_migration_documents_cpython_as_behavior_oracle_not_stdlib_source_drop() {
    for required in [
        "not a full CPython clone",
        "Do not wholesale port CPython `Lib/`",
        "Use CPython as an oracle",
        "public behavior and tests",
        "supported sandbox behavior",
        "MiniPython's Rust runtime",
        "standard-library",
        "accepted only when",
        "supported surface",
        "excluded surface",
        "concrete `cpython_diff` evidence",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(required),
            "migration document must mention CPython migration boundary `{required}`"
        );
    }

    for required in [
        "CPython remains the behavior oracle",
        "not an implementation source to copy",
        "must not wholesale port CPython `Lib/`",
        "direct differential evidence",
    ] {
        assert!(
            CPYTHON_COVERAGE.contains(required),
            "coverage document must mention CPython migration boundary `{required}`"
        );
    }
}

#[test]
fn readmes_document_cpython_oracle_not_implementation_source() {
    for required in [
        "sandbox-focused Rust Python",
        "rather than a full CPython",
        "CPython is the behavior oracle",
        "not an implementation source",
        "wholesale port CPython `Lib/`",
        "CPython public behavior migration",
        "executable differential tests",
        "Every bundled stdlib module must have a matching `cpython_diff` case",
    ] {
        assert!(
            README.contains(required),
            "README must document CPython oracle boundary `{required}`"
        );
    }

    for required in [
        "面向 sandbox 的 Rust Python",
        "而不是完整复制",
        "CPython 是行为 oracle",
        "不是实现来源",
        "wholesale 搬 CPython",
        "可执行 differential tests",
        "每个 bundled stdlib 模块必须有对应的 `cpython_diff` case",
    ] {
        assert!(
            README_CN.contains(required),
            "README_CN must document CPython oracle boundary `{required}`"
        );
    }
}

#[test]
fn sandbox_policy_guard_names_reference_real_runtime_tests() {
    for guard in [
        "sandbox_policy_denies_stdlib_imports",
        "sandbox_policy_denies_required_sandbox_stdlib_surface",
        "sandbox_policy_allows_required_sandbox_stdlib_surface",
        "sandbox_policy_required_stdlib_allow_list_excludes_compatibility_shims",
        "sandbox_policy_requires_explicit_allow_for_extra_stdlib_shims",
        "out_of_scope_host_io_network_and_process_surfaces_stay_unavailable",
    ] {
        let test_signature = format!("fn {guard}()");
        assert!(
            LANGUAGE_TESTS.contains(&test_signature),
            "documented sandbox policy guard `{guard}` must exist in tests/language.rs"
        );
        assert!(
            CPYTHON_MIGRATION.contains(guard) || CPYTHON_COVERAGE.contains(guard),
            "sandbox policy guard `{guard}` must be referenced by migration or coverage docs"
        );
    }
}

#[test]
fn json_sandbox_hook_stop_line_is_documented_and_guarded() {
    for term in [
        "object_hook",
        "object_pairs_hook",
        "parse_float",
        "parse_int",
        "parse_constant",
        "default",
        "cls",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(term),
            "migration document must mention json hook stop-line term `{term}`"
        );
        assert!(
            CPYTHON_COVERAGE.contains(term),
            "coverage document must mention json hook stop-line term `{term}`"
        );
        assert!(
            LANGUAGE_TESTS.contains(term),
            "tests/language.rs must guard json hook stop-line term `{term}`"
        );
    }
}

#[test]
fn required_sandbox_stdlib_runtime_guard_matches_manifest_modules() {
    let guard_modules = required_stdlib_runtime_guard_modules();
    let manifest_modules = sandbox_stdlib_module_names();

    assert_eq!(
        guard_modules, manifest_modules,
        "required sandbox stdlib runtime guard allow-list drifted from manifest modules"
    );
}

#[test]
fn required_sandbox_stdlib_scope_matches_defined_surface() {
    let expected = [
        "array",
        "builtins",
        "collections",
        "collections.abc",
        "copy",
        "functools",
        "io",
        "itertools",
        "json",
        "math",
        "math.integer",
        "operator",
        "sys",
        "types",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<BTreeSet<_>>();

    assert_eq!(
        sandbox_stdlib_module_names(),
        expected,
        "sandbox stdlib manifest drifted from the explicitly scoped required module surface"
    );
    assert_eq!(
        required_stdlib_runtime_guard_modules(),
        expected,
        "runtime required stdlib allow-list drifted from the explicitly scoped module surface"
    );

    for excluded in [
        "pickle", "typing", "weakref", "time", "os", "os.path", "re", "string", "unittest",
    ] {
        assert!(
            !expected.contains(excluded),
            "compatibility/test-support module `{excluded}` must stay out of required sandbox stdlib"
        );
    }
}

#[test]
fn cpython_migration_documents_out_of_scope_runtime_stop_line_guard() {
    for required in [
        "out_of_scope_host_io_network_and_process_surfaces_stay_unavailable",
        "open()",
        "input()",
        "asyncio",
        "http",
        "ssl",
        "socket",
        "subprocess",
        "signal",
        "threading",
        "pty",
        "urllib",
        "multiprocessing",
        "_ssl",
        "_socket",
        "_ctypes",
        "_testcapi",
        "C ABI",
        "CPython-internal",
        "co_stacksize",
        "locale-sensitive",
        "pdb",
        "breakpoint",
    ] {
        assert!(
            CPYTHON_MIGRATION.contains(required),
            "migration document must mention out-of-scope runtime stop-line term `{required}`"
        );
    }
}

#[test]
fn cpython_coverage_documents_out_of_scope_runtime_stop_line_guard() {
    for required in [
        "out_of_scope_host_io_network_and_process_surfaces_stay_unavailable",
        "open()",
        "input()",
        "asyncio",
        "http",
        "ssl",
        "socket",
        "subprocess",
        "signal",
        "threading",
        "pty",
        "urllib",
        "multiprocessing",
        "_ssl",
        "_socket",
        "_ctypes",
        "_testcapi",
        "C ABI",
        "CPython-internal",
        "co_stacksize",
        "locale-sensitive",
        "pdb",
        "breakpoint",
    ] {
        assert!(
            CPYTHON_COVERAGE.contains(required),
            "coverage document must mention out-of-scope runtime stop-line term `{required}`"
        );
    }
}

#[test]
fn stdlib_create_module_registry_is_explicitly_tracked() {
    let actual = stdlib_create_module_names();
    let expected = [
        "_types",
        "_weakref",
        "annotationlib",
        "array",
        "ast",
        "builtins",
        "collections",
        "collections.abc",
        "copy",
        "decimal",
        "dis",
        "enum",
        "fractions",
        "functools",
        "inspect",
        "io",
        "itertools",
        "json",
        "math",
        "math.integer",
        "operator",
        "os",
        "os.path",
        "pickle",
        "re",
        "string",
        "string.templatelib",
        "sys",
        "test",
        "test.typinganndata",
        "test.typinganndata.ann_module",
        "test.typinganndata.ann_module2",
        "test.typinganndata.ann_module3",
        "time",
        "types",
        "typing",
        "unittest",
        "unittest.mock",
        "warnings",
        "weakref",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "stdlib create_module registry drifted; classify new modules before exposing them"
    );
}

#[test]
fn stdlib_create_module_registry_keeps_stop_line_modules_unavailable() {
    let actual = stdlib_create_module_names();
    for forbidden in [
        "_ctypes",
        "_socket",
        "_ssl",
        "_testcapi",
        "asyncio",
        "http",
        "locale",
        "multiprocessing",
        "pdb",
        "pty",
        "signal",
        "ssl",
        "socket",
        "subprocess",
        "threading",
        "urllib",
    ] {
        assert!(
            !actual.contains(forbidden),
            "out-of-scope module `{forbidden}` must not be exposed by default"
        );
    }
}

#[test]
fn stdlib_create_module_registry_is_classified_by_scope() {
    let actual = stdlib_create_module_names();
    let sandbox_modules = sandbox_stdlib_module_names();
    let compatibility_modules = compatibility_module_registry_names();
    let classified = sandbox_modules
        .union(&compatibility_modules)
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual = actual
        .into_iter()
        .map(str::to_string)
        .collect::<BTreeSet<_>>();

    assert!(
        sandbox_modules.is_disjoint(&compatibility_modules),
        "sandbox stdlib modules must not also be compatibility-only modules"
    );
    assert_eq!(
        actual, classified,
        "every create_module() entry must be classified as required sandbox stdlib or compatibility/test support"
    );
}

#[test]
fn pickle_stays_compatibility_only_not_required_sandbox_stdlib() {
    let actual = stdlib_create_module_names();
    let sandbox_modules = sandbox_stdlib_module_names();
    let compatibility_modules = compatibility_module_registry_names();

    assert!(
        actual.contains("pickle"),
        "pickle registry entry should remain visible for migrated pure-memory tests"
    );
    assert!(
        compatibility_modules.contains("pickle"),
        "pickle must be classified as compatibility/test support"
    );
    assert!(
        !sandbox_modules.contains("pickle"),
        "pickle must not become required sandbox stdlib without an explicit scope change"
    );
    assert!(
        CPYTHON_MIGRATION.contains("`pickle` | pure-memory test serialization support"),
        "migration registry must document pickle as pure-memory test serialization support"
    );

    let copy = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "copy")
        .expect("sandbox stdlib manifest must include copy");
    assert!(
        copy.excluded_surface.contains("pickle protocol"),
        "copy sandbox manifest must keep pickle protocol outside the default surface"
    );

    let operator = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "operator")
        .expect("sandbox stdlib manifest must include operator");
    assert!(
        operator.excluded_surface.contains("Full pickle metadata"),
        "operator sandbox manifest must keep full pickle metadata outside the default surface"
    );
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
fn cpython_tokenizer_operator_diff_evidence_matches_runtime_subsets() {
    for (subset, diff) in [
        (
            "cpython_tokenize_multiplicative_operators_subset",
            "cpython_tokenize_multiplicative_operators_diff_subset",
        ),
        (
            "cpython_tokenize_unary_operators_subset",
            "cpython_tokenize_unary_operators_diff_subset",
        ),
        (
            "cpython_tokenize_exact_type_subset",
            "cpython_tokenize_exact_type_diff_subset",
        ),
        (
            "cpython_tokenize_matrix_multiply_and_ellipsis_subset",
            "cpython_tokenize_matrix_multiply_and_ellipsis_diff_subset",
        ),
        (
            "cpython_tokenize_selector_and_method_subset",
            "cpython_tokenize_selector_and_method_diff_subset",
        ),
        (
            "cpython_tokenize_async_await_subset",
            "cpython_tokenize_async_await_diff_subset",
        ),
        (
            "cpython_tokenize_comments_subset",
            "cpython_tokenize_comments_diff_subset",
        ),
        (
            "cpython_tokenize_indentation_blank_line_subset",
            "cpython_tokenize_indentation_blank_line_diff_subset",
        ),
        (
            "cpython_tokenize_nested_indentation_subset",
            "cpython_tokenize_nested_indentation_diff_subset",
        ),
        (
            "cpython_tokenize_formfeed_whitespace_subset",
            "cpython_tokenize_formfeed_whitespace_diff_subset",
        ),
        (
            "cpython_tokenize_unmatched_indentation_subset",
            "cpython_tokenize_unmatched_indentation_diff_subset",
        ),
        (
            "cpython_tokenize_implicit_line_joining_subset",
            "cpython_tokenize_implicit_line_joining_diff_subset",
        ),
        (
            "cpython_tokenize_explicit_line_joining_subset",
            "cpython_tokenize_explicit_line_joining_diff_subset",
        ),
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "tokenizer runtime subset `{subset}` must exist"
        );
        assert!(
            CPYTHON_DIFF.contains(&format!("fn {diff}(")),
            "tokenizer CPython diff evidence `{diff}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(diff),
            "coverage document must mention tokenizer diff evidence `{diff}`"
        );
        assert!(
            CPYTHON_MIGRATION.contains(diff),
            "migration document must mention tokenizer diff evidence `{diff}`"
        );
    }
}

#[test]
fn tokenizer_interpolated_string_split_subsets_stay_documented_as_partial() {
    for subset in [
        "cpython_tokenize_f_string_span_subset",
        "cpython_tokenize_f_string_split_token_subset",
        "cpython_tokenize_t_string_span_subset",
        "cpython_tokenize_t_string_split_token_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "interpolated-string tokenizer subset `{subset}` must exist"
        );
        assert!(
            CPYTHON_COVERAGE.contains(subset) && CPYTHON_MIGRATION.contains(subset),
            "interpolated-string tokenizer subset `{subset}` must stay documented"
        );

        let diff_name = subset.replace("_subset", "_diff_subset");
        assert!(
            !CPYTHON_DIFF.contains(&format!("fn {diff_name}(")),
            "tokenizer API subset `{subset}` must not be mistaken for same-named CPython output diff `{diff_name}`"
        );
    }

    for required in [
        "parser still consumes collapsed",
        "tokenize_cpython_with_spans()",
        "split tokens",
        "CPython tokenizer surface",
    ] {
        assert!(
            CPYTHON_COVERAGE.contains(required) || CPYTHON_MIGRATION.contains(required),
            "tokenizer docs must keep interpolated-string partial surface note `{required}`"
        );
    }
}

#[test]
fn cpython_operator_precedence_smoke_diff_covers_grammar_operator_subsets() {
    let diff_name = "cpython_program_output_parity_smoke_diff_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("program output parity smoke diff must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for subset in [
        "cpython_grammar_additive_ops_subset",
        "cpython_grammar_multiplicative_ops_subset",
        "cpython_grammar_unary_ops_subset",
        "cpython_grammar_bitwise_and_shift_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "grammar operator runtime subset `{subset}` must exist"
        );
    }

    for required in [
        "operator-precedence-and-associativity",
        "1 & 1",
        "1 ^ 1",
        "1 | 1",
        "1 << 1",
        "8 >> 1",
        "1 - 1 - 1",
        "1 / 1 * 1 % 1",
        "~1",
        "---1",
    ] {
        assert!(
            diff_source.contains(required),
            "program output parity smoke diff must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("bitwise")
                && document.contains("shift")
                && document.contains("additive")
                && document.contains("multiplicative")
                && document.contains("unary"),
            "operator docs must link `{diff_name}` to bitwise, shift, additive, multiplicative, and unary operator coverage"
        );
    }
}

#[test]
fn cpython_control_flow_smoke_diff_covers_grammar_runtime_subsets() {
    let diff_name = "cpython_program_output_parity_smoke_diff_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("program output parity smoke diff must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for subset in [
        "cpython_grammar_equal_comparison_subset",
        "cpython_grammar_ordering_comparison_subset",
        "cpython_grammar_membership_comparison_subset",
        "cpython_grammar_identity_comparison_subset",
        "cpython_grammar_chained_comparison_subset",
        "cpython_grammar_boolean_operations_subset",
        "cpython_grammar_if_else_subset",
        "cpython_grammar_elif_subset",
        "cpython_grammar_while_subset",
        "cpython_grammar_for_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "grammar runtime subset `{subset}` must exist"
        );
    }

    for required in [
        "boolean-expression-short-circuit-identity",
        "while-else",
        "for-else-continue",
        "for-break-skips-else",
        "conditional-expression-precedence",
        "custom-bool-and-len-truth-protocol",
        "1 < 2",
        "True and False",
        "while",
        "else",
        "for",
        "break",
        "continue",
    ] {
        assert!(
            diff_source.contains(required),
            "program output parity smoke diff must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("boolean")
                && document.contains("comparison")
                && document.contains("while")
                && document.contains("for")
                && document.contains("control flow"),
            "control-flow docs must link `{diff_name}` to boolean, comparison, while, for, and control-flow coverage"
        );
    }
}

#[test]
fn cpython_comparison_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_comparison_helper_rules_diff_subset";
    let subset_name = "cpython_comparison_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("comparison helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "comparison helper runtime subset evidence must exist"
    );
    for required in [
        "1 == 1 | 0",
        "1 != 2 | 0",
        "1 <= 2 | 1",
        "1 < 2 | 1",
        "3 >= 2 | 1",
        "4 > 1 | 2",
        "1 in [0 | 1]",
        "2 not in [1 | 0]",
        "None is None",
        "None is not 1 | 0",
        "1 < 1 > 1 == 1 >= 1 <= 1 != 1 in 1 not in x is x is not x",
    ] {
        assert!(
            diff_source.contains(required),
            "comparison helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "comparison helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_stmt_diff_covers_match_runtime_subset() {
    let diff_name = "cpython_grammar_match_stmt_diff_subset";
    let subset_name = "cpython_grammar_match_stmt_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match statement CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match statement runtime subset evidence must exist"
    );
    for required in [
        "match subject",
        "case 0",
        "case [command, direction]",
        "case {\"x\": value, **rest}",
        "case Holder.token",
        "case Point(1, y=value) as point",
    ] {
        assert!(
            diff_source.contains(required),
            "match statement diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match statement docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_numeric_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_match_numeric_literal_helper_rules_diff_subset";
    let subset_name = "cpython_match_numeric_literal_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match numeric helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match numeric helper runtime subset evidence must exist"
    );
    for required in [
        "case 1",
        "case -2",
        "case 1.5",
        "case 2j",
        "case 1 + 2j",
        "case -1.5 - 2.5j",
        "case {-1.5 - 2.5j: item}",
    ] {
        assert!(
            diff_source.contains(required),
            "match numeric helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match numeric helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_capture_wildcard_group_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_match_capture_wildcard_group_helper_rules_diff_subset";
    let subset_name = "cpython_match_capture_wildcard_group_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match capture/wildcard/group helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match capture/wildcard/group helper runtime subset evidence must exist"
    );
    for required in [
        "case captured",
        "case _",
        "case ([1, value])",
        "case (_)",
        "case (captured)",
    ] {
        assert!(
            diff_source.contains(required),
            "match capture/wildcard/group helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match capture/wildcard/group helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_capture_target_and_star_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_match_capture_target_and_star_pattern_helper_rules_diff_subset";
    let subset_name = "cpython_match_capture_target_and_star_pattern_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match capture-target/star helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match capture-target/star helper runtime subset evidence must exist"
    );
    for required in [
        "case [first, *middle]",
        "case [first, *_, last]",
        "case 1 as captured",
        "case {'x': value, **rest}",
        "case Box(value=captured)",
    ] {
        assert!(
            diff_source.contains(required),
            "match capture-target/star helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match capture-target/star helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_value_attr_name_or_attr_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_match_value_attr_name_or_attr_helper_rules_diff_subset";
    let subset_name = "cpython_match_value_attr_name_or_attr_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match value/attr/name_or_attr helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match value/attr/name_or_attr helper runtime subset evidence must exist"
    );
    for required in [
        "case A.B",
        "case Nested.Inner.C",
        "case {Keys.Names.key: value}",
        "case Box()",
        "case Outer.Inner()",
    ] {
        assert!(
            diff_source.contains(required),
            "match value/attr/name_or_attr helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match value/attr/name_or_attr helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_pattern_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_match_pattern_helper_rules_diff_subset";
    let subset_name = "cpython_match_pattern_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match pattern helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match pattern helper runtime subset evidence must exist"
    );
    for required in [
        "case left, right",
        "case value if value > 2",
        "case [label, 1] | [label, 2]",
        "match item := ['go', 'n']",
        "case ([1, value])",
        "case [first, *middle, last]",
        "case Holder.token",
        "case {'x': value, **rest}",
        "case Point(1, y=value) as point",
    ] {
        assert!(
            diff_source.contains(required),
            "match pattern helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match pattern helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_sequence_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_match_sequence_helper_rules_diff_subset";
    let subset_name = "cpython_match_sequence_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match sequence helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match sequence helper runtime subset evidence must exist"
    );
    for required in [
        "case []",
        "case ()",
        "case [value,]",
        "case (value,)",
        "case [first, *middle, last,]",
        "case first, *_, last",
        "case (left, right)",
    ] {
        assert!(
            diff_source.contains(required),
            "match sequence helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match sequence helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_mapping_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_match_mapping_helper_rules_diff_subset";
    let subset_name = "cpython_match_mapping_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match mapping helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match mapping helper runtime subset evidence must exist"
    );
    for required in [
        "case {}",
        "case {**rest}",
        "case {'x': first, 'y': second, **rest,}",
        "case {Keys.label: [first, second],}",
        "case {DynamicKeys.KEY: y, 'a': z}",
        "except ValueError as error",
    ] {
        assert!(
            diff_source.contains(required),
            "match mapping helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match mapping helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_match_class_helper_diff_covers_runtime_subset() {
    let diff_name = "cpython_match_class_helper_rules_diff_subset";
    let subset_name = "cpython_match_class_helper_rules_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("match class helper CPython diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    assert!(
        CPYTHON_SUBSET.contains(&format!("fn {subset_name}(")),
        "match class helper runtime subset evidence must exist"
    );
    for required in [
        "case Empty()",
        "case Point(1, value,)",
        "case Point(x=1, y=[first, second],)",
        "case Point(1, y=value,)",
        "case Outer.Inner(value)",
        "case int(value,)",
        "case range()",
        "case range(10)",
        "case max(0, 1)",
        "__match_args__",
    ] {
        assert!(
            diff_source.contains(required),
            "match class helper diff evidence must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name) && document.contains(subset_name),
            "match class helper docs must link `{diff_name}` to `{subset_name}`"
        );
    }
}

#[test]
fn cpython_ast_dump_public_diff_covers_exact_subsets() {
    let diff_name = "cpython_ast_dump_public_diff_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("ast.dump public diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for subset in [
        "cpython_ast_dump_plain_first_pass_subset",
        "cpython_ast_dump_indent_first_pass_subset",
        "cpython_ast_dump_incomplete_first_pass_subset",
        "cpython_ast_dump_exact_subset",
        "cpython_ast_dump_indent_exact_subset",
        "cpython_ast_dump_incomplete_exact_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "ast.dump runtime subset `{subset}` must exist"
        );
    }

    for required in [
        "ast-dump-public",
        "ASTHelpers_Test::test_dump",
        "::test_dump_indent",
        "::test_dump_incomplete",
        "ast.dump(node, annotate_fields=False)",
        "ast.dump(node, include_attributes=True)",
        "ast.dump(node, indent=3)",
        "legacy default-field rendering",
    ] {
        assert!(
            diff_source.contains(required),
            "ast.dump public diff must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("ast.dump")
                && document.contains("default-field"),
            "ast.dump docs must link `{diff_name}` to current CPython default-field coverage"
        );
    }
}

#[test]
fn cpython_ast_parse_public_diff_covers_core_subset() {
    let diff_name = "cpython_ast_parse_public_diff_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("ast.parse public diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for subset in [
        "cpython_ast_module_parse_dump_first_pass_subset",
        "cpython_ast_parse_null_bytes_subset",
        "cpython_ast_parse_invalid_ast_subset",
        "cpython_ast_parse_optimize_debug_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "ast.parse runtime subset `{subset}` must exist"
        );
    }

    for required in [
        "ast-parse-public",
        "Lib/ast.py::parse public wrapper",
        "ast.parse('x = 1')",
        "mode='eval'",
        "mode='single'",
        "mode='func_type'",
        "ast.parse(node) is node",
        "ast.PyCF_ONLY_AST",
        "legacy ast.dump default-field rendering",
    ] {
        assert!(
            diff_source.contains(required),
            "ast.parse public diff must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("ast.parse")
                && document.contains("exec")
                && document.contains("eval")
                && document.contains("func_type"),
            "ast.parse docs must link `{diff_name}` to exec/eval/single/func_type coverage"
        );
    }
}

#[test]
fn cpython_compile_source_positions_diff_covers_public_invariants() {
    let diff_name = "cpython_compile_source_positions_public_invariants_diff_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("compile source-position diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for subset in [
        "cpython_compile_source_positions_code_positions_first_pass_subset",
        "cpython_compile_source_positions_lambda_return_position_subset",
        "cpython_compile_source_positions_weird_attribute_position_regressions_subset",
        "cpython_compile_source_positions_multistatement_code_lines_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "compile source-position subset `{subset}` must exist"
        );
    }

    for required in [
        "compile-source-positions-public-invariants",
        "TestSourcePositions public co_positions invariants",
        "code.co_positions",
        "co.co_lines()",
        "f = lambda: x",
        "f = lambda: 1 + 2",
        "z = 3",
        "CPython oracle lacks code.co_positions",
    ] {
        assert!(
            diff_source.contains(required),
            "compile source-position diff must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("co_positions()")
                && document.contains("public")
                && document.contains("opcode"),
            "compile source-position docs must link `{diff_name}` to public non-opcode invariants"
        );
    }
}

#[test]
fn cpython_compile_specifics_lineno_diff_covers_public_invariants() {
    let diff_name = "cpython_compile_specifics_lineno_public_invariants_diff_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("compile lineno diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for subset in [
        "cpython_compile_specifics_lineno_procedure_call_subset",
        "cpython_compile_specifics_lineno_attribute_subset",
        "cpython_compile_specifics_lineno_after_no_code_first_pass_subset",
        "cpython_compile_specifics_lineno_after_implicit_return_subset",
        "cpython_compile_specifics_lineno_of_backward_jump_conditional_in_loop_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "compile lineno subset `{subset}` must exist"
        );
    }

    for required in [
        "compile-specifics-lineno-public-invariants",
        "TestSpecifics public line-number invariants",
        "code.co_lines",
        "def call()",
        "def no_code1()",
        "def load_attr()",
        "def load_method()",
        "def if1(x)",
        "def loop_conditional()",
        "CPython oracle lacks code.co_lines",
    ] {
        assert!(
            diff_source.contains(required),
            "compile lineno diff must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("co_lines()")
                && document.contains("public")
                && document.contains("opcode"),
            "compile lineno docs must link `{diff_name}` to public non-opcode invariants"
        );
    }
}

#[test]
fn cpython_ast_literal_eval_public_diff_covers_exact_subsets() {
    let diff_name = "cpython_ast_literal_eval_public_diff_subset";
    let diff_start = CPYTHON_DIFF
        .find(&format!("fn {diff_name}("))
        .expect("ast.literal_eval public diff evidence must exist");
    let diff_end = CPYTHON_DIFF[diff_start..]
        .find("\n#[test]")
        .map(|offset| diff_start + offset)
        .unwrap_or(CPYTHON_DIFF.len());
    let diff_source = &CPYTHON_DIFF[diff_start..diff_end];

    for subset in [
        "cpython_ast_literal_eval_first_pass_subset",
        "cpython_ast_literal_eval_exact_subset",
        "cpython_ast_literal_eval_complex_full_subset",
        "cpython_ast_literal_eval_complex_exact_subset",
    ] {
        assert!(
            CPYTHON_SUBSET.contains(&format!("fn {subset}(")),
            "ast.literal_eval runtime subset `{subset}` must exist"
        );
    }

    for required in [
        "ast-literal-eval-public",
        "ASTHelpers_Test::test_literal_eval",
        "::test_literal_eval_complex",
        "ast.literal_eval('[1, 2, 3]')",
        "ast.literal_eval('set()')",
        "ast.parse('[1, 2]', mode='eval').body",
        "'3.25+6.75j'",
        "'3+(0+6j)'",
    ] {
        assert!(
            diff_source.contains(required),
            "ast.literal_eval public diff must cover `{required}`"
        );
    }

    for document in [CPYTHON_COVERAGE, CPYTHON_MIGRATION] {
        assert!(
            document.contains(diff_name)
                && document.contains("ast.literal_eval")
                && document.contains("complex"),
            "ast.literal_eval docs must link `{diff_name}` to literal and complex coverage"
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

#[test]
fn cpython_test_manifest_collections_group_counts_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COLLECTIONS_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let groups = manifest_groups();

    assert_manifest_group_count(
        &groups,
        "Lib/test/test_collections.py",
        "module-level `test_*` functions",
        module_level_test_function_count(&source),
    );

    for group in [
        "TestUserObjects",
        "TestChainMap",
        "TestNamedTuple",
        "ABCTestCase",
        "TestOneTrickPonyABCs",
        "WithSet",
        "TestCollectionABCs",
        "CounterSubclassWithSetItem",
        "CounterSubclassWithGet",
        "TestCounter",
    ] {
        let expected = class_counts.get(group).copied().unwrap_or_else(|| {
            panic!("missing class `{group}` in {CPYTHON_TEST_COLLECTIONS_SOURCE}")
        });
        assert_manifest_group_count(&groups, "Lib/test/test_collections.py", group, expected);
    }
}

#[test]
fn cpython_test_manifest_types_group_counts_match_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPES_SOURCE);
    let class_counts = python_test_class_method_counts(&source);
    let groups = manifest_groups();

    assert_manifest_group_count(
        &groups,
        "Lib/test/test_types.py",
        "module-level `test_*` functions",
        module_level_test_function_count(&source),
    );

    for group in [
        "TypesTests",
        "UnionTests",
        "MappingProxyTests",
        "ClassCreationTests",
        "SimpleNamespaceTests",
        "CoroutineTests",
        "FunctionTests",
        "SubinterpreterTests",
    ] {
        let expected = class_counts
            .get(group)
            .copied()
            .unwrap_or_else(|| panic!("missing class `{group}` in {CPYTHON_TEST_TYPES_SOURCE}"));
        assert_manifest_group_count(&groups, "Lib/test/test_types.py", group, expected);
    }
}

#[test]
fn cpython_test_manifest_types_tests_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPES_SOURCE);
    let expected = python_test_class_method_names(&source, "TypesTests");
    let methods = method_audit_methods("## `Lib/test/test_types.py::TypesTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TypesTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| {
            matches!(
                method.status,
                "ported" | "partial" | "blocked_by_runtime" | "blocked_by_cpython_internal"
            )
        }),
        "TypesTests method audit contains an unknown status"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        23,
        "ported TypesTests method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial TypesTests method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_runtime")
            .count(),
        4,
        "blocked-by-runtime TypesTests method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_cpython_internal")
            .count(),
        3,
        "blocked-by-CPython-internal TypesTests method count drifted"
    );
    assert!(
        methods
            .iter()
            .any(|method| method.method == "test_names" && method.status == "ported"),
        "TypesTests test_names public alias surface should remain ported"
    );
    assert!(
        methods.iter().any(|method| {
            method.method == "test_internal_sizes" && method.status == "blocked_by_cpython_internal"
        }),
        "TypesTests internal size checks should stay classified as CPython-internal"
    );
    assert!(
        methods
            .iter()
            .any(|method| method.method == "test_none_type" && method.status == "ported"),
        "TypesTests singleton alias coverage should remain ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "TypesTests method audit drifted");
}

#[test]
fn cpython_test_manifest_builtin_sorted_method_audit_is_complete() {
    let methods = method_audit_methods("## `Lib/test/test_builtin.py::TestSorted` Method Audit");

    assert_eq!(
        methods.len(),
        4,
        "TestSorted method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "all TestSorted methods should be ported"
    );

    for expected in [
        "test_basic",
        "test_bad_arguments",
        "test_inputtypes",
        "test_baddecorator",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing TestSorted method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_types_union_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPES_SOURCE);
    let expected = python_test_class_method_names(&source, "UnionTests");
    let methods = method_audit_methods("## `Lib/test/test_types.py::UnionTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "UnionTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| {
            matches!(
                method.status,
                "ported"
                    | "partial"
                    | "blocked_by_runtime"
                    | "blocked_by_cpython_internal"
                    | "not_started"
            )
        }),
        "UnionTests method audit contains an unknown status"
    );
    assert!(
        methods
            .iter()
            .any(|method| method.method == "test_hash" && method.status == "ported"),
        "UnionTests hash method should remain fully ported"
    );
    assert!(
        methods.iter().any(|method| {
            method.method == "test_or_type_operator_reference_cycle"
                && method.status == "blocked_by_cpython_internal"
        }),
        "UnionTests reference-cycle method should remain classified as CPython-internal"
    );

    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "UnionTests method audit drifted");
}

#[test]
fn cpython_test_manifest_types_mappingproxy_method_audit_is_complete() {
    let methods =
        method_audit_methods("## `Lib/test/test_types.py::MappingProxyTests` Method Audit");

    assert_eq!(
        methods.len(),
        15,
        "MappingProxyTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "all MappingProxyTests methods should be ported"
    );

    for expected in [
        "test_constructor",
        "test_methods",
        "test_get",
        "test_missing",
        "test_customdict",
        "test_chainmap",
        "test_contains",
        "test_views",
        "test_len",
        "test_iterators",
        "test_reversed",
        "test_copy",
        "test_union",
        "test_hash",
        "test_richcompare",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing MappingProxyTests method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_types_class_creation_method_audit_is_complete() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPES_SOURCE);
    let expected = python_test_class_method_names(&source, "ClassCreationTests");
    let methods =
        method_audit_methods("## `Lib/test/test_types.py::ClassCreationTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "ClassCreationTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "all ClassCreationTests methods should be ported"
    );

    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "ClassCreationTests method audit drifted");
}

#[test]
fn cpython_test_manifest_types_coroutine_method_audit_is_tracked() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_TYPES_SOURCE);
    let expected = python_test_class_method_names(&source, "CoroutineTests");
    let methods = method_audit_methods("## `Lib/test/test_types.py::CoroutineTests` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "CoroutineTests method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "partial" | "blocked_by_runtime")),
        "CoroutineTests method statuses should be ported, partial, or blocked_by_runtime"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        11,
        "ported CoroutineTests method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial CoroutineTests method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_runtime")
            .count(),
        0,
        "blocked CoroutineTests method count drifted"
    );

    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "CoroutineTests method audit drifted");
}

#[test]
fn cpython_test_manifest_types_function_method_audit_is_complete() {
    let methods = method_audit_methods("## `Lib/test/test_types.py::FunctionTests` Method Audit");

    assert_eq!(
        methods.len(),
        2,
        "FunctionTests method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == "ported"),
        "all FunctionTests methods should be ported"
    );

    for expected in [
        "test_function_type_defaults",
        "test_function_type_wrong_defaults",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing FunctionTests method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_collections_chainmap_method_audit_is_complete() {
    let methods =
        method_audit_methods("## `Lib/test/test_collections.py::TestChainMap` Method Audit");

    assert_eq!(
        methods.len(),
        10,
        "TestChainMap method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "partial" | "not_started")),
        "TestChainMap method statuses should be ported, partial, or not_started"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        10,
        "ported TestChainMap method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial TestChainMap method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "not_started")
            .count(),
        0,
        "not_started TestChainMap method count drifted"
    );

    for expected in [
        "test_basics",
        "test_ordering",
        "test_constructor",
        "test_bool",
        "test_missing",
        "test_order_preservation",
        "test_iter_not_calling_getitem_on_maps",
        "test_dict_coercion",
        "test_new_child",
        "test_union_operators",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing TestChainMap method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_collections_namedtuple_method_audit_is_complete() {
    let methods =
        method_audit_methods("## `Lib/test/test_collections.py::TestNamedTuple` Method Audit");

    assert_eq!(
        methods.len(),
        23,
        "TestNamedTuple method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| matches!(
            method.status,
            "ported" | "partial" | "not_started" | "blocked_by_cpython_internal"
        )),
        "TestNamedTuple method statuses should be ported, partial, not_started, or blocked_by_cpython_internal"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        20,
        "ported TestNamedTuple method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial TestNamedTuple method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "not_started")
            .count(),
        0,
        "not_started TestNamedTuple method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_cpython_internal")
            .count(),
        3,
        "blocked_by_cpython_internal TestNamedTuple method count drifted"
    );

    for expected in [
        "test_factory",
        "test_defaults",
        "test_readonly",
        "test_factory_doc_attr",
        "test_field_doc",
        "test_field_doc_reuse",
        "test_field_repr",
        "test_name_fixer",
        "test_module_parameter",
        "test_instance",
        "test_tupleness",
        "test_odd_sizes",
        "test_large_size",
        "test_pickle",
        "test_copy",
        "test_name_conflicts",
        "test_repr",
        "test_keyword_only_arguments",
        "test_namedtuple_subclass_issue_24931",
        "test_field_descriptor",
        "test_new_builtins_issue_43102",
        "test_match_args",
        "test_non_generic_subscript",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing TestNamedTuple method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_collections_user_objects_method_audit_is_complete() {
    let methods =
        method_audit_methods("## `Lib/test/test_collections.py::TestUserObjects` Method Audit");

    assert_eq!(
        methods.len(),
        6,
        "TestUserObjects method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "not_started")),
        "TestUserObjects method statuses should be ported or not_started"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        6,
        "ported TestUserObjects method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "not_started")
            .count(),
        0,
        "not_started TestUserObjects method count drifted"
    );

    for expected in [
        "test_str_protocol",
        "test_list_protocol",
        "test_dict_protocol",
        "test_list_copy",
        "test_dict_copy",
        "test_dict_missing",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing TestUserObjects method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_collections_one_trick_pony_method_audit_is_complete() {
    let methods = method_audit_methods(
        "## `Lib/test/test_collections.py::TestOneTrickPonyABCs` Method Audit",
    );

    assert_eq!(
        methods.len(),
        16,
        "TestOneTrickPonyABCs method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported")),
        "TestOneTrickPonyABCs method statuses should all be ported"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial TestOneTrickPonyABCs method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        16,
        "ported TestOneTrickPonyABCs method count drifted"
    );

    for expected in [
        "test_Awaitable",
        "test_Coroutine",
        "test_Hashable",
        "test_AsyncIterable",
        "test_AsyncIterator",
        "test_Iterable",
        "test_Reversible",
        "test_Collection",
        "test_Iterator",
        "test_Generator",
        "test_AsyncGenerator",
        "test_Sized",
        "test_Container",
        "test_Callable",
        "test_direct_subclassing",
        "test_registration",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing TestOneTrickPonyABCs method audit row for `{expected}`"
        );
    }
}

#[test]
fn cpython_test_manifest_collections_collection_abcs_method_audit_matches_current_source() {
    let source = cpython_source_or_skip!(CPYTHON_TEST_COLLECTIONS_SOURCE);
    let expected = python_test_class_method_names(&source, "TestCollectionABCs");
    let methods =
        method_audit_methods("## `Lib/test/test_collections.py::TestCollectionABCs` Method Audit");

    assert_eq!(
        methods.len(),
        expected.len(),
        "TestCollectionABCs method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| matches!(
            method.status,
            "ported" | "partial" | "not_started" | "blocked_by_cpython_internal"
        )),
        "TestCollectionABCs method statuses should be ported, partial, not_started, or blocked_by_cpython_internal"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        24,
        "ported TestCollectionABCs method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial TestCollectionABCs method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "not_started")
            .count(),
        0,
        "not_started TestCollectionABCs method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "blocked_by_cpython_internal")
            .count(),
        1,
        "blocked_by_cpython_internal TestCollectionABCs method count drifted"
    );

    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).collect::<Vec<_>>();
    let extra = actual.difference(&expected).collect::<Vec<_>>();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "TestCollectionABCs method audit drifted; missing={missing:?}; extra={extra:?}"
    );
}

#[test]
fn cpython_test_manifest_collections_counter_method_audit_is_complete() {
    let methods =
        method_audit_methods("## `Lib/test/test_collections.py::TestCounter` Method Audit");

    assert_eq!(
        methods.len(),
        23,
        "TestCounter method audit row count drifted"
    );
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.status, "ported" | "partial" | "not_started")),
        "TestCounter method statuses should be ported, partial, or not_started"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "ported")
            .count(),
        23,
        "ported TestCounter method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        0,
        "partial TestCounter method count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "not_started")
            .count(),
        0,
        "not_started TestCounter method count drifted"
    );

    for expected in [
        "test_basics",
        "test_update_reentrant_add_clears_counter",
        "test_init",
        "test_total",
        "test_order_preservation",
        "test_update",
        "test_copying",
        "test_copy_subclass",
        "test_conversions",
        "test_invariant_for_the_in_operator",
        "test_multiset_operations",
        "test_inplace_operations",
        "test_subtract",
        "test_unary",
        "test_repr_nonsortable",
        "test_helper_function",
        "test_multiset_operations_equivalent_to_set_operations",
        "test_eq",
        "test_le",
        "test_lt",
        "test_ge",
        "test_gt",
        "test_symmetric_difference",
    ] {
        assert!(
            methods.iter().any(|method| method.method == expected),
            "missing TestCounter method audit row for `{expected}`"
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

fn assert_manifest_group_status(
    groups: &[ManifestGroup<'_>],
    source: &str,
    group: &str,
    expected: &str,
) {
    let actual = groups
        .iter()
        .find(|entry| entry.source == source && entry.group == group)
        .unwrap_or_else(|| panic!("missing manifest group `{source}` / `{group}`"))
        .status;
    assert_eq!(
        actual, expected,
        "manifest status drifted for `{source}` / `{group}`"
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

fn sandbox_stdlib_rows() -> Vec<SandboxStdlibRow<'static>> {
    let mut in_section = false;
    let mut rows = Vec::new();

    for line in CPYTHON_MIGRATION.lines() {
        if line == "## Sandbox Stdlib Manifest" {
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
        let module = normalize_markdown_code_cell(cells[0]);
        if module == "Module" || module.chars().all(|ch| ch == '-') {
            continue;
        }
        rows.push(SandboxStdlibRow {
            module,
            supported_surface: cells[1],
            diff_evidence: cells[2],
            excluded_surface: cells[3],
        });
    }

    rows
}

fn extract_rust_test_body<'a>(source: &'a str, name: &str) -> &'a str {
    let needle = format!("fn {name}(");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("missing Rust test `{name}`"));
    let tail = &source[start..];
    let end = tail.find("\n#[test]").unwrap_or(tail.len());
    &tail[..end]
}

fn stdlib_create_module_names() -> BTreeSet<&'static str> {
    let start = STDLIB_SOURCE
        .find("pub(crate) fn create_module(")
        .expect("stdlib.rs must define create_module()");
    let end = start
        + STDLIB_SOURCE[start..]
            .find("\n        _ => Err(")
            .expect("create_module() must end with a ModuleNotFoundError fallback");

    STDLIB_SOURCE[start..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line.strip_prefix('"')?;
            let (name, rest) = rest.split_once('"')?;
            if rest.trim_start().starts_with("=>") {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

fn sandbox_stdlib_module_names() -> BTreeSet<String> {
    let mut modules = BTreeSet::new();
    for row in sandbox_stdlib_rows() {
        match row.module.as_str() {
            "collections / collections.abc" => {
                modules.insert("collections".to_string());
                modules.insert("collections.abc".to_string());
            }
            "math / math.integer" => {
                modules.insert("math".to_string());
                modules.insert("math.integer".to_string());
            }
            "io.BytesIO" => {
                modules.insert("io".to_string());
            }
            module => {
                modules.insert(module.to_string());
            }
        }
    }
    modules
}

fn compatibility_module_registry_names() -> BTreeSet<String> {
    let mut in_section = false;
    let mut modules = BTreeSet::new();

    for line in CPYTHON_MIGRATION.lines() {
        if line == "## Runtime Compatibility Module Registry" {
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
        if cells.len() != 2 {
            continue;
        }
        let module = strip_backticks(cells[0]).unwrap_or(cells[0]);
        if module == "Module" || module.chars().all(|ch| ch == '-') {
            continue;
        }
        modules.insert(module.to_string());
    }

    modules
}

fn required_stdlib_runtime_guard_modules() -> BTreeSet<String> {
    let constant = "const REQUIRED_SANDBOX_STDLIB_MODULES: &[&str] = &[";
    let list_start = LANGUAGE_TESTS
        .find(constant)
        .map(|start| start + constant.len())
        .expect("language.rs must define REQUIRED_SANDBOX_STDLIB_MODULES");
    let list_end = LANGUAGE_TESTS[list_start..]
        .find("];")
        .map(|offset| list_start + offset)
        .expect("REQUIRED_SANDBOX_STDLIB_MODULES must close with ];");
    quoted_strings(&LANGUAGE_TESTS[list_start..list_end])
        .into_iter()
        .collect()
}

fn quoted_strings(source: &str) -> Vec<String> {
    let mut strings = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0;

    while let Some(&byte) = bytes.get(index) {
        if byte != b'"' {
            index += 1;
            continue;
        }
        index += 1;
        let literal_start = index;
        while let Some(&inner) = bytes.get(index) {
            if inner == b'\\' {
                index += 2;
                continue;
            }
            if inner == b'"' {
                strings.push(source[literal_start..index].to_string());
                index += 1;
                break;
            }
            index += 1;
        }
    }

    strings
}

fn backtick_tokens(text: &str) -> Vec<&str> {
    text.split('`')
        .enumerate()
        .filter_map(|(index, part)| (index % 2 == 1).then_some(part))
        .collect()
}

fn normalize_markdown_code_cell(cell: &'static str) -> String {
    cell.replace('`', "")
}

fn token_tests_methods() -> Vec<ManifestMethod<'static>> {
    method_audit_methods("## `Lib/test/test_grammar.py::TokenTests` Method Audit")
}

fn assert_builtin_method_audit_status_matches_current_source(
    class_name: &str,
    section_heading: &str,
    status: &str,
) {
    let source = cpython_source_or_skip!(CPYTHON_TEST_BUILTIN_SOURCE);
    let expected = python_test_class_method_names(&source, class_name);
    let methods = method_audit_methods(section_heading);

    assert_eq!(
        methods.len(),
        expected.len(),
        "{class_name} method audit row count drifted"
    );
    assert!(
        methods.iter().all(|method| method.status == status),
        "{class_name} methods should stay classified as `{status}`"
    );

    let actual = methods
        .iter()
        .map(|method| method.method.to_string())
        .collect::<BTreeSet<_>>();
    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{class_name} method audit drifted");
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
                .split(['(', '[', ':'])
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

fn python_test_class_method_names(source: &str, class_name: &str) -> Vec<String> {
    let lines = source.lines().collect::<Vec<_>>();
    let class_start = lines
        .iter()
        .position(|line| {
            line.strip_prefix("class ")
                .and_then(|rest| rest.split(['(', '[', ':']).next())
                == Some(class_name)
        })
        .unwrap_or_else(|| panic!("missing class `{class_name}`"));
    let class_end = lines[class_start + 1..]
        .iter()
        .position(|line| line.starts_with("class "))
        .map(|offset| class_start + 1 + offset)
        .unwrap_or(lines.len());

    lines[class_start + 1..class_end]
        .iter()
        .filter_map(|line| {
            let rest = line.strip_prefix("    def ")?;
            if !rest.starts_with("test_") {
                return None;
            }
            Some(
                rest.split('(')
                    .next()
                    .expect("split always yields a first item")
                    .to_string(),
            )
        })
        .collect()
}

fn python_test_method_source(source: &str, class_name: &str, method_name: &str) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    let class_start = lines
        .iter()
        .position(|line| {
            line.strip_prefix("class ")
                .and_then(|rest| rest.split(['(', '[', ':']).next())
                == Some(class_name)
        })
        .unwrap_or_else(|| panic!("missing class `{class_name}`"));
    let class_end = lines[class_start + 1..]
        .iter()
        .position(|line| line.starts_with("class "))
        .map(|offset| class_start + 1 + offset)
        .unwrap_or(lines.len());
    let method_signature = format!("    def {method_name}(");
    let method_start = lines[class_start + 1..class_end]
        .iter()
        .position(|line| line.starts_with(&method_signature))
        .map(|offset| class_start + 1 + offset)
        .unwrap_or_else(|| panic!("missing method `{class_name}::{method_name}`"));
    let method_end = lines[method_start + 1..class_end]
        .iter()
        .position(|line| line.starts_with("    def test_"))
        .map(|offset| method_start + 1 + offset)
        .unwrap_or(class_end);

    lines[method_start..method_end].join("\n")
}

fn python_call_string_arguments(source: &str, call_name: &str) -> Vec<String> {
    let pattern = format!("{call_name}(");
    let mut arguments = Vec::new();
    let mut offset = 0;

    while let Some(position) = source[offset..].find(&pattern) {
        let mut index = offset + position + pattern.len();
        let bytes = source.as_bytes();
        while matches!(bytes.get(index), Some(b' ' | b'\n' | b'\t')) {
            index += 1;
        }

        let Some(&quote) = bytes.get(index) else {
            break;
        };
        if quote != b'\'' && quote != b'"' {
            offset = index.saturating_add(1);
            continue;
        }

        let literal_start = index + 1;
        index = literal_start;
        while let Some(&byte) = bytes.get(index) {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if byte == quote {
                arguments.push(source[literal_start..index].to_string());
                index += 1;
                break;
            }
            index += 1;
        }
        offset = index;
    }

    arguments
}

fn python_reference_string_arguments(source: &str, function_name: &str) -> Vec<String> {
    let pattern = format!("{function_name},");
    let mut arguments = Vec::new();
    let mut offset = 0;

    while let Some(position) = source[offset..].find(&pattern) {
        let mut index = offset + position + pattern.len();
        let bytes = source.as_bytes();
        while matches!(bytes.get(index), Some(b' ' | b'\n' | b'\t')) {
            index += 1;
        }

        let Some(&quote) = bytes.get(index) else {
            break;
        };
        if quote != b'\'' && quote != b'"' {
            offset = index.saturating_add(1);
            continue;
        }

        let literal_start = index + 1;
        index = literal_start;
        while let Some(&byte) = bytes.get(index) {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if byte == quote {
                arguments.push(source[literal_start..index].to_string());
                index += 1;
                break;
            }
            index += 1;
        }
        offset = index;
    }

    arguments
}

fn python_string_literal_has_rust_evidence(literal_inner: &str, evidence: &str) -> bool {
    let mut candidates = BTreeSet::new();
    candidates.insert(literal_inner.to_string());
    candidates.insert(literal_inner.replace('\\', "\\\\"));
    candidates.insert(literal_inner.replace('\t', "\\t"));
    candidates.insert(literal_inner.replace('\t', "\\\\t"));
    candidates.insert(literal_inner.replace('\n', "\\n"));
    candidates.insert(literal_inner.replace('\n', "\\\\n"));

    candidates
        .iter()
        .any(|candidate| evidence.contains(candidate))
}

fn rust_test_names(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("fn ")?;
            let name = rest.split_once('(')?.0;
            name.starts_with("cpython_").then(|| name.to_string())
        })
        .collect()
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
