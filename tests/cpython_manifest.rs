use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const MANIFEST: &str = include_str!("cpython_test_manifest.md");
const CPYTHON_COVERAGE: &str = include_str!("cpython_coverage.md");
const CPYTHON_MIGRATION: &str = include_str!("cpython_migration.md");
const CPYTHON_DIFF: &str = include_str!("cpython_diff.rs");
const CPYTHON_SUBSET: &str = include_str!("cpython_subset.rs");
const LANGUAGE_TESTS: &str = include_str!("language.rs");
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
        let has_direct_diff_evidence = evidence_names.iter().any(|evidence| {
            evidence.starts_with("cpython_") || sandbox_stdlib_legacy_direct_evidence(evidence)
        });
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
    if sandbox_stdlib_legacy_runtime_evidence(evidence) {
        return true;
    }

    let candidates = sandbox_stdlib_runtime_subset_candidates(evidence);
    candidates
        .iter()
        .any(|candidate| CPYTHON_SUBSET.contains(candidate) || LANGUAGE_TESTS.contains(candidate))
}

fn sandbox_stdlib_runtime_subset_candidates(evidence: &str) -> Vec<String> {
    if sandbox_stdlib_legacy_runtime_evidence(evidence) {
        return vec![evidence.to_string()];
    }

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
    if evidence == "cpython_json_loads_dumps_diff_subset" {
        candidates.push("cpython_json_loads_dumps_basic_subset".to_string());
    }
    if evidence == "cpython_array_one_byte_public_clear_diff_subset" {
        candidates.push("cpython_array_one_byte_public_mutation_methods_subset".to_string());
    }

    candidates
}

fn sandbox_stdlib_legacy_direct_evidence(evidence: &str) -> bool {
    matches!(
        evidence,
        "globals-locals-builtins"
            | "exec-builtin"
            | "compile-code-object-builtin"
            | "builtin-breakpoint-custom-hook"
            | "builtin-breakpoint-passthru-error"
            | "iter-next-builtins"
            | "map-filter-builtins"
            | "float-hash-and-sys-info"
            | "types-frame-locals-proxy-currentframe"
            | "types-method-descriptor-types"
            | "types-int-dunder-format-matrix"
            | "types-float-dunder-format-matrix"
    )
}

fn sandbox_stdlib_legacy_runtime_evidence(evidence: &str) -> bool {
    matches!(
        evidence,
        "globals-locals-builtins"
            | "exec-builtin"
            | "compile-code-object-builtin"
            | "builtin-breakpoint-custom-hook"
            | "builtin-breakpoint-passthru-error"
            | "iter-next-builtins"
            | "map-filter-builtins"
            | "float-hash-and-sys-info"
            | "types-frame-locals-proxy-currentframe"
            | "types-method-descriptor-types"
            | "types-int-dunder-format-matrix"
            | "types-float-dunder-format-matrix"
            | "pure-memory-stdlib-core"
            | "operator-precedence-and-associativity"
    )
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
fn itertools_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "itertools",
        &[
            "cpython_itertools_core_iterator_subset",
            "cpython_itertools_keyword_error_subset",
            "cpython_itertools_pairwise_subset",
        ],
        &[],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "itertools")
        .expect("sandbox stdlib manifest must include itertools");
    for evidence in [
        "cpython_itertools_core_diff_subset",
        "cpython_itertools_keyword_error_diff_subset",
        "cpython_itertools_pairwise_diff_subset",
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
    let pairwise_end = CPYTHON_SUBSET[pairwise_start..]
        .find("\n// Adapted from CPython Lib/test/test_list.py")
        .map(|offset| pairwise_start + offset)
        .expect("itertools pairwise subset must end before sequence constructor tests");

    let core_source = &CPYTHON_SUBSET[core_start..keyword_start];
    let keyword_source = &CPYTHON_SUBSET[keyword_start..pairwise_start];
    let pairwise_source = &CPYTHON_SUBSET[pairwise_start..pairwise_end];

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
    let pairwise_end = CPYTHON_DIFF[pairwise_start..]
        .find("\n// Differential smoke tests")
        .map(|offset| pairwise_start + offset)
        .expect("itertools pairwise diff subset must end before smoke tests");

    let core_source = &CPYTHON_DIFF[core_start..keyword_start];
    let keyword_source = &CPYTHON_DIFF[keyword_start..pairwise_start];
    let pairwise_source = &CPYTHON_DIFF[pairwise_start..pairwise_end];

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
            "cpython_json_dumps_allow_nan_subset",
            "cpython_json_dumps_check_circular_subset",
            "cpython_json_dumps_ensure_ascii_subset",
            "cpython_json_dumps_indent_subset",
            "cpython_json_dumps_skipkeys_subset",
            "cpython_json_dumps_sort_keys_subset",
            "cpython_json_dumps_separators_subset",
            "cpython_json_dumps_float_spelling_subset",
            "cpython_json_loads_number_and_whitespace_subset",
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
            "full `JSONDecodeError` compatibility",
        ],
    );

    let row = sandbox_stdlib_rows()
        .into_iter()
        .find(|row| row.module == "json")
        .expect("sandbox stdlib manifest must include json");
    for evidence in [
        "cpython_json_loads_dumps_diff_subset",
        "cpython_json_keyword_argument_binding_diff_subset",
        "cpython_json_loads_escape_and_duplicate_key_diff_subset",
        "cpython_json_loads_unicode_escape_roundtrip_diff_subset",
        "cpython_json_loads_strict_diff_subset",
        "cpython_json_dumps_string_escape_diff_subset",
        "cpython_json_dumps_key_coercion_diff_subset",
        "cpython_json_dumps_allow_nan_diff_subset",
        "cpython_json_dumps_check_circular_diff_subset",
        "cpython_json_dumps_ensure_ascii_diff_subset",
        "cpython_json_dumps_indent_diff_subset",
        "cpython_json_dumps_skipkeys_diff_subset",
        "cpython_json_dumps_sort_keys_diff_subset",
        "cpython_json_dumps_separators_diff_subset",
        "cpython_json_dumps_float_spelling_diff_subset",
        "cpython_json_loads_number_and_whitespace_diff_subset",
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
fn operator_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "operator",
        &[
            "cpython_operator_public_helpers_subset",
            "cpython_operator_length_hint_subset",
            "cpython_operator_comparison_predicate_subset",
            "cpython_operator_arithmetic_bitwise_subset",
            "cpython_operator_sequence_member_subset",
            "cpython_operator_callable_helper_subset",
            "cpython_operator_inplace_helper_subset",
            "cpython_operator_module_metadata_subset",
            "cpython_operator_signature_helper_subset",
            "cpython_operator_helper_repr_subset",
        ],
        &[],
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
fn collections_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "collections / collections.abc",
        &[
            "cpython_collections_counter_public_subset",
            "cpython_collections_chainmap_public_methods_subset",
            "cpython_collections_namedtuple_public_subset",
            "cpython_collections_userdict_userlist_public_subset",
            "cpython_collections_userdict_public_methods_subset",
            "cpython_collections_userlist_public_methods_subset",
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
            .contains("cpython_collections_chainmap_public_diff_subset"),
        "collections sandbox manifest must cite CPython diff evidence for ChainMap public methods"
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
            "cpython_math_exp_exp2_subset",
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
        "cpython_math_fma_diff_subset",
        "cpython_math_fmax_fmin_diff_subset",
        "cpython_math_exp_exp2_diff_subset",
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
fn sys_sandbox_manifest_lists_public_subset_evidence() {
    assert_sandbox_manifest_subset_evidence(
        "sys",
        &[
            "cpython_float_hash_and_sys_info_subset",
            "cpython_builtin_negation_sys_maxsize_subset",
            "cpython_attribute_introspection_builtins_subset",
            "cpython_builtin_breakpoint_custom_hook_subset",
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
        "globals-locals-builtins",
        "cpython_attribute_introspection_builtins_diff_subset",
        "builtin-breakpoint-custom-hook",
        "builtin-breakpoint-passthru-error",
        "float-hash-and-sys-info",
        "types-frame-locals-proxy-currentframe",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "sys sandbox manifest must cite CPython diff evidence `{evidence}`"
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
            "cpython_enumerate_zip_sorted_builtin_subset",
            "cpython_builtin_sorted_exact_subset",
            "cpython_zip_strict_builtin_subset",
            "cpython_map_filter_builtin_subset",
            "cpython_map_strict_builtin_subset",
            "cpython_abs_builtin_subset",
            "cpython_divmod_builtin_subset",
            "cpython_round_builtin_subset",
            "cpython_pow_builtin_subset",
            "cpython_chr_ord_builtin_subset",
            "cpython_format_builtin_and_custom_dunder_format_subset",
            "cpython_ascii_builtin_subset",
            "cpython_builtin_breakpoint_custom_hook_subset",
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
        "globals-locals-builtins",
        "exec-builtin",
        "compile-code-object-builtin",
        "cpython_attribute_introspection_builtins_diff_subset",
        "cpython_ascii_builtin_diff_subset",
        "cpython_chr_ord_builtin_diff_subset",
        "builtin-breakpoint-custom-hook",
        "builtin-breakpoint-passthru-error",
        "iter-next-builtins",
        "map-filter-builtins",
    ] {
        assert!(
            row.diff_evidence.contains(evidence),
            "builtins sandbox manifest must cite CPython diff evidence `{evidence}`"
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
            "cpython_types_class_creation_prepare_resolve_bases_subset",
            "cpython_types_class_creation_mro_entries_core_subset",
            "cpython_types_class_creation_metaclass_derivation_subset",
            "cpython_types_class_creation_one_argument_type_subset",
            "cpython_types_coroutine_public_subset",
            "cpython_types_function_type_subset",
            "cpython_types_code_traceback_type_aliases_subset",
            "cpython_types_frame_type_alias_subset",
            "cpython_types_frame_locals_proxy_type_subset",
        ],
        &[
            "CPython object-layout internals",
            "exact C descriptor types",
            "pickle identity matrices",
            "interpreter lifecycle behavior",
        ],
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
        "does not expose `bytearray.resize()`",
        "does not expose `bytearray.take_bytes()`",
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
        "does not expose `bytearray.resize()`",
        "does not expose",
        "`bytearray.take_bytes()`",
        "remains local subset evidence",
        "does not expose the",
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
fn cpython_migration_documents_out_of_scope_runtime_stop_line_guard() {
    for required in [
        "out_of_scope_host_io_network_and_process_surfaces_stay_unavailable",
        "open()",
        "input()",
        "socket",
        "subprocess",
        "signal",
        "pty",
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
        "locale",
        "multiprocessing",
        "pdb",
        "pty",
        "signal",
        "socket",
        "subprocess",
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
