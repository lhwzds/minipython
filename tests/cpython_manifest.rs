use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const MANIFEST: &str = include_str!("cpython_test_manifest.md");
const CPYTHON_TEST_AST_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_ast/test_ast.py";
const CPYTHON_TEST_BUILTIN_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_builtin.py";
const CPYTHON_TEST_COLLECTIONS_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_collections.py";
const CPYTHON_TEST_COMPILE_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_compile.py";
const CPYTHON_TEST_TYPE_COMMENTS_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_type_comments.py";
const CPYTHON_TEST_TYPE_PARAMS_SOURCE: &str =
    "/Volumes/samsung/GitHub/cpython/Lib/test/test_type_params.py";

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
    assert_source_total(&groups, "Lib/test/test_builtin.py", 133);
    assert_source_total(&groups, "Lib/test/test_collections.py", 103);
    assert_source_total(&groups, "Lib/test/test_type_comments.py", 17);
    assert_source_total(&groups, "Lib/test/test_type_params.py", 107);
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
fn cpython_test_manifest_compile_specifics_method_audit_matches_current_source() {
    let source = fs::read_to_string(CPYTHON_TEST_COMPILE_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_COMPILE_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_COMPILE_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_COMPILE_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_COMPILE_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_COMPILE_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_COMPILE_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_COMPILE_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_COMPILE_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_COMPILE_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_COMPILE_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_COMPILE_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_COMPILE_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_COMPILE_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_TYPE_COMMENTS_SOURCE).unwrap_or_else(|error| {
        panic!("failed to read {CPYTHON_TEST_TYPE_COMMENTS_SOURCE}: {error}")
    });
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
    let source = fs::read_to_string(CPYTHON_TEST_TYPE_COMMENTS_SOURCE).unwrap_or_else(|error| {
        panic!("failed to read {CPYTHON_TEST_TYPE_COMMENTS_SOURCE}: {error}")
    });
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
    let source = fs::read_to_string(CPYTHON_TEST_TYPE_PARAMS_SOURCE).unwrap_or_else(|error| {
        panic!("failed to read {CPYTHON_TEST_TYPE_PARAMS_SOURCE}: {error}")
    });
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
    let source = fs::read_to_string(CPYTHON_TEST_TYPE_PARAMS_SOURCE).unwrap_or_else(|error| {
        panic!("failed to read {CPYTHON_TEST_TYPE_PARAMS_SOURCE}: {error}")
    });
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
    let source = fs::read_to_string(CPYTHON_TEST_TYPE_PARAMS_SOURCE).unwrap_or_else(|error| {
        panic!("failed to read {CPYTHON_TEST_TYPE_PARAMS_SOURCE}: {error}")
    });
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
    let source = fs::read_to_string(CPYTHON_TEST_TYPE_PARAMS_SOURCE).unwrap_or_else(|error| {
        panic!("failed to read {CPYTHON_TEST_TYPE_PARAMS_SOURCE}: {error}")
    });
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
fn cpython_test_manifest_builtin_group_counts_match_current_source() {
    let source = fs::read_to_string(CPYTHON_TEST_BUILTIN_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_BUILTIN_SOURCE}: {error}"));
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
fn cpython_test_manifest_builtin_eval_exec_method_audit_is_tracked() {
    let source = fs::read_to_string(CPYTHON_TEST_BUILTIN_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_BUILTIN_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_BUILTIN_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_BUILTIN_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_BUILTIN_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_BUILTIN_SOURCE}: {error}"));
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
        7,
        "ported BuiltinTest attribute/introspection audit count drifted"
    );
    assert_eq!(
        methods
            .iter()
            .filter(|method| method.status == "partial")
            .count(),
        2,
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
        ("test_dir", "partial"),
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
fn cpython_test_manifest_builtin_aggregate_method_audit_is_tracked() {
    let source = fs::read_to_string(CPYTHON_TEST_BUILTIN_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_BUILTIN_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_BUILTIN_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_BUILTIN_SOURCE}: {error}"));
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
fn cpython_test_manifest_builtin_test_breakpoint_method_audit_is_classified() {
    assert_builtin_method_audit_status_matches_current_source(
        "TestBreakpoint",
        "## `Lib/test/test_builtin.py::TestBreakpoint` Method Audit",
        "blocked_by_runtime",
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
    let source = fs::read_to_string(CPYTHON_TEST_BUILTIN_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_BUILTIN_SOURCE}: {error}"));
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
fn cpython_test_manifest_ast_tests_method_audit_matches_current_source() {
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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
    let source = fs::read_to_string(CPYTHON_TEST_AST_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_AST_SOURCE}: {error}"));
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

#[test]
fn cpython_test_manifest_collections_group_counts_match_current_source() {
    let source = fs::read_to_string(CPYTHON_TEST_COLLECTIONS_SOURCE).unwrap_or_else(|error| {
        panic!("failed to read {CPYTHON_TEST_COLLECTIONS_SOURCE}: {error}")
    });
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
    let source = fs::read_to_string(CPYTHON_TEST_COLLECTIONS_SOURCE).unwrap_or_else(|error| {
        panic!("failed to read {CPYTHON_TEST_COLLECTIONS_SOURCE}: {error}")
    });
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

fn assert_builtin_method_audit_status_matches_current_source(
    class_name: &str,
    section_heading: &str,
    status: &str,
) {
    let source = fs::read_to_string(CPYTHON_TEST_BUILTIN_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_TEST_BUILTIN_SOURCE}: {error}"));
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
