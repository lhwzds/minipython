# CPython Test Migration Manifest

This manifest tracks CPython test modules that directly pressure Python syntax,
AST shape, parser diagnostics, and parser-coupled runtime behavior.

The counts below come from the local CPython checkout at
`/Volumes/samsung/GitHub/cpython`:

- `Lib/test/test_grammar.py`
- `Lib/test/test_syntax.py`
- `Lib/test/test_compile.py`
- `Lib/test/test_ast/test_ast.py`
- `Lib/test/test_ast/snippets.py`

This file is deliberately stricter than `tests/cpython_grammar_inventory.md`.
The grammar inventory can say every grammar production has some Rust coverage;
this manifest asks whether the actual CPython test methods have equivalent
Rust coverage.

## Status Vocabulary

- `ported`: every CPython test method in the row has equivalent Rust coverage.
- `partial`: some source shapes or assertions from the row have been migrated,
  but full method-level parity has not been proven.
- `blocked_by_runtime`: the test is syntax-adjacent, but migration needs
  runtime or standard-library behavior that MiniPython does not yet implement.
- `blocked_by_ast_module`: the test validates CPython's public `ast` module or
  AST-object APIs rather than only MiniPython's parser.
- `blocked_by_cpython_internal`: the test validates CPython-only internal
  implementation surfaces rather than Python source behavior.
- `not_started`: no meaningful Rust equivalent has been started.
- `source_data`: the CPython file provides shared test data rather than unittest
  methods.

## Summary

| Status | Groups | Test methods |
| --- | ---: | ---: |
| `ported` | 16 | 306 |
| `partial` | 5 | 207 |
| `blocked_by_runtime` | 0 | 0 |
| `blocked_by_ast_module` | 2 | 16 |
| `blocked_by_cpython_internal` | 1 | 3 |
| `not_started` | 0 | 0 |
| `source_data` | 5 | 0 |
| **Total** | 29 | 532 |

## Source Groups

| Source | Group | Test methods | Status | Migration evidence / next action |
| --- | --- | ---: | --- | --- |
| `Lib/test/test_grammar.py` | module-level `test_*` functions | 0 | `source_data` | The current local CPython source has no module-level `test_*` functions in this file; its executable tests live under `TokenTests` and `GrammarTests`. |
| `Lib/test/test_grammar.py` | `TokenTests` | 14 | `ported` | All 14 CPython methods now have method-level Rust parity evidence in the audit below. |
| `Lib/test/test_grammar.py` | `GrammarTests` | 61 | `ported` | All 61 current CPython methods now have method-level Rust parity evidence in the audit below, covering variable annotations, function/lambda forms, simple/compound statements, control flow, imports, exceptions, suites, expressions, comparisons, operators, selectors, atoms, class definitions, comprehensions, with/async forms, parenthesized evaluation, matrix multiplication, and the complex lambda/f-string regression. |
| `Lib/test/test_syntax.py` | module-level `test_*` functions | 0 | `source_data` | The current local CPython source has no module-level `test_*` functions in this file; doctest loading is tracked separately from method-level syntax migration. |
| `Lib/test/test_syntax.py` | `SyntaxWarningTest` | 2 | `ported` | Both current CPython methods have method-level Rust evidence in `cpython_finally_control_flow_warning_subset`, covering all return/break/continue-in-finally warning shapes. |
| `Lib/test/test_syntax.py` | `SyntaxErrorTestCase` | 44 | `ported` | Every current method has method-level Rust evidence in the audit below, including CPython-only static-nesting and parser-complexity cases. |
| `Lib/test/test_syntax.py` | `LazyImportRestrictionTestCase` | 9 | `ported` | All 9 current lazy-import restriction methods are covered by `cpython_lazy_import_syntax_subset`, including try/try-star, except blocks, functions, async functions, classes, nested scopes, star-import rejection, and valid module-level compile-only cases. |
| `Lib/test/test_syntax.py` | legacy nested lazy-import helper group | 0 | `source_data` | Older manifest snapshots tracked two nested lazy-import helper cases separately; the current local CPython source folds those into `LazyImportRestrictionTestCase::test_lazy_import_nested_scopes`. |
| `Lib/test/test_compile.py` | `TestSpecifics` | 98 | `partial` | Current evidence covers selected syntax-adjacent cases including argument handling, literal leading zeroes, invalid named expressions, subscript behavior, dead-code compile regressions, type aliases, warning filtering, newline/indentation compile boundaries through `cpython_compile_specifics_newline_and_indentation_subset`, syntax-error compile/eval/exec boundaries through `cpython_compile_specifics_syntax_error_boundaries_subset`, `None` target rejection through `cpython_compile_specifics_none_assignment_subset`, import grammar through `cpython_compile_specifics_import_syntax_subset`, selected compile-stability regressions through `cpython_compile_specifics_compile_stability_subset`, dict display evaluation order through `cpython_compile_specifics_dict_evaluation_order_subset`, and first-pass compile filename behavior through `cpython_compile_specifics_compile_filename_subset`. Full method-level parity remains open across broader code-object metadata, optimization details, line-number behavior, memoryview/path-like filename handling, constants, and warning matrices. |
| `Lib/test/test_compile.py` | `TestBooleanExpression` | 4 | `ported` | `cpython_compile_boolean_expression_exact_subset` and `boolean-expression-short-circuit-identity` in the differential harness port all current methods, covering `and` / `or` short-circuit operand identity, exact `__bool__` call counts in mixed expressions, and exception propagation from truthiness. |
| `Lib/test/test_compile.py` | `TestSourcePositions` | 33 | `partial` | Parser and public-AST source-position evidence covers many related statement and expression spans, and `cpython_compile_source_positions_multiline_assert_rewrite_subset` ports the public-AST compile stability method for a rewritten multiline assert. Full parity remains open because CPython's code-object `co_positions()` / opcode debug-range checks are not fully exposed by MiniPython yet. |
| `Lib/test/test_compile.py` | `TestStaticAttributes` | 4 | `ported` | `cpython_compile_static_attributes_exact_subset` ports all current methods, covering tuple-valued class `__static_attributes__`, sorted/deduplicated `self.<attr>` Store targets, nested function collection into the nearest class, nested class isolation, and subclass-specific tuples. |
| `Lib/test/test_compile.py` | `TestExpressionStackSize` | 17 | `ported` | `cpython_compile_expression_stack_size_shapes_subset` ports all current methods as compile-shape checks for long `and` / `or` / mixed boolean chains, chained comparisons, conditional expressions, binary expressions, list/tuple/set/dict displays, function and method positional/keyword calls, repeated function-body boolean expressions, 3050-target unpack assignment, and 3050-argument annotated signatures. MiniPython does not expose CPython `co_stacksize`, so this checks the equivalent register-compiler stability surface. |
| `Lib/test/test_compile.py` | `TestStackSizeStability` | 27 | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` ports all current methods as repeated sync/async function compile-shape checks; MiniPython does not expose CPython `co_stacksize`, so the equivalent evidence is clean compilation of the repeated control-flow snippets plus existing output/differential control-flow tests. |
| `Lib/test/test_compile.py` | `TestInstructionSequence` | 3 | `blocked_by_cpython_internal` | Requires CPython's `_testinternalcapi` instruction-sequence object and opcode metadata. |
| `Lib/test/test_ast/test_ast.py` | module-level `test_*` functions | 0 | `source_data` | The current local CPython source has no module-level `test_*` functions in this file; executable tests live under the unittest classes below. |
| `Lib/test/test_ast/test_ast.py` | `LazyImportTest` | 1 | `partial` | `cpython_ast_lazy_import_fields_subset` covers the syntax-adjacent public AST `is_lazy` fields for parsed ordinary/lazy imports, constructors, `ast.dump()`, and compile-from-public-AST. The CPython-only runtime side-effect assertion in `ensure_lazy_imports("ast", ...)` still needs a broader import-system/runtime model. |
| `Lib/test/test_ast/test_ast.py` | `AST_Tests` | 61 | `partial` | Current coverage includes public AST constructor/base-object behavior, generated ASDL class hierarchy/inventory/signatures, `_field_types` / `__annotations__`, selected parser and compile validation, compare modes, feature-version cases, null-byte handling, import/alias/slice field checks, default end-position compile-from-AST cases, parser warning capture, and selected optimization behavior. Full method-level parity remains open across the current 61 CPython methods, including garbage-collection, complete source-location validation, full feature-version matrices, repr snapshots, and t-string details. |
| `Lib/test/test_ast/test_ast.py` | `CopyTests` | 14 | `partial` | Direct method-level Rust evidence now covers pickling, parent-link deepcopy, replace interface/native loops, native class fields/attributes, custom class attributes, extra/missing field rejection, defaulted missing fields, and non-string unpacked keywords. `cpython_ast_copy_replace_accept_known_custom_class_fields_first_pass_subset` still adapts CPython's string identity assertion to value equality because MiniPython strings are not yet identity-preserving objects. Full parity remains blocked on the broader string/object identity model, with binary pickle byte compatibility still outside this AST-only slice. |
| `Lib/test/test_ast/test_ast.py` | `ASTHelpers_Test` | 29 | `ported` | All 29 current CPython methods now have direct method-level Rust evidence, covering parse and parse-in-error behavior, dump variants, iterator helpers, literal evaluation and diagnostics, recursion detection, location helpers, docstring helpers, source-segment/end-position helpers, import-from validation, lazy import AST fields, and compile-from-public-AST helper coverage. |
| `Lib/test/test_ast/test_ast.py` | `ASTValidatorTests` | 40 | `ported` | All 40 current CPython methods now have method-level Rust evidence, covering public-AST root modes, statement and expression context validation, function/class/try/try-star validation, argument validation, comprehensions, match-pattern validation, `test_stdlib_validates` file-backed compile seeds, and recursive stdlib compile seeds. |
| `Lib/test/test_ast/test_ast.py` | `ConstantTests` | 8 | `ported` | `cpython_ast_constant_compile_first_pass_subset` ports all current CPython methods: invalid Constant value validation, singleton identity preservation, scalar/tuple/frozenset value preservation, illegal assignment targets, docstring retrieval from Constant module docstrings, CPython-style `LOAD_CONST` observation through the supported `dis.hasconst` / `dis.get_instructions()` subset including tuple constants, `literal_eval()` operand replacement, and string-prefix `kind` metadata. |
| `Lib/test/test_ast/test_ast.py` | `EndPositionTests` | 28 | `ported` | All 28 current CPython methods now have method-level Rust evidence. Coverage includes parser source extraction for calls, definitions, literals, suites, f-strings, imports, slices, binary/boolean operations, tuple/list/set/dict displays, redundant parentheses, comprehensions, yield/await, newline variants, padded extraction, missing location attributes, and UTF-8 byte-column offsets. |
| `Lib/test/test_ast/test_ast.py` | `NodeTransformerTests` | 5 | `ported` | `cpython_ast_node_transformer_first_pass_subset` covers the current CPython node-transformer scenarios: removing a single AST field, removing a node from a list field, returning a list of replacement nodes, mutating a node in place, and replacing a node. It also covers the supporting `NodeVisitor` dispatch path used by `NodeTransformer`. |
| `Lib/test/test_ast/test_ast.py` | `ASTConstructorTests` | 11 | `ported` | All 11 current CPython methods now have direct method-level Rust evidence, covering `FunctionDef`, expression-context defaults, fieldless custom subclasses, `_fields`, `_field_types`, `_attributes`, missing required fields, incomplete/malformed field metadata, implicit list defaults, and non-string unpacked constructor keywords. |
| `Lib/test/test_ast/test_ast.py` | `ModuleStateTests` | 3 | `blocked_by_ast_module` | Requires CPython `ast` module reload/subinterpreter behavior. |
| `Lib/test/test_ast/test_ast.py` | `CommandLineTests` | 13 | `blocked_by_ast_module` | Requires the CPython `python -m ast` command-line surface. |
| `Lib/test/test_ast/test_ast.py` | `ASTOptimizationTests` | 3 | `ported` | `cpython_ast_optimization_format_folding_subset` ports `test_folding_format` by checking that `ast.parse(..., optimize=-1)` preserves the old-style `%s` `BinOp` while `optimize=1` folds it to `JoinedStr` / `FormattedValue`. `cpython_ast_optimization_match_case_folding_subset` ports `test_folding_match_case_allowed_expressions` and `test_match_case_not_folded_in_unoptimized_ast`, covering optimize-driven folding of signed real/imaginary match literals in `MatchValue`, `MatchMapping`, and nested `MatchSequence` patterns while preserving the unoptimized `BinOp` shape at `optimize=0`. |
| `Lib/test/test_ast/snippets.py` | snippet source data | 0 | `source_data` | Shared AST parse snippets are migrated through `cpython_ast_snippets_parse_inventory_subset`, sampled by `cpython_ast_snippets_structural_dump_subset`, and now have first public-AST `to_tuple()` evidence in `cpython_ast_snippets_public_to_tuple_first_pass_subset`; this file has no unittest methods. |

Additional `ASTConstructorTests` evidence:
`cpython_ast_constructor_functiondef_exact_subset`,
`cpython_ast_constructor_expr_context_exact_subset`,
`cpython_ast_constructor_custom_subclass_with_no_fields_exact_subset`,
`cpython_ast_constructor_fields_but_no_field_types_exact_subset`,
`cpython_ast_constructor_fields_and_types_exact_subset`,
`cpython_ast_constructor_custom_attributes_exact_subset`,
`cpython_ast_constructor_fields_and_types_no_default_exact_subset`,
`cpython_ast_constructor_incomplete_field_types_exact_subset`,
`cpython_ast_constructor_malformed_fields_with_bytes_exact_subset`,
`cpython_ast_constructor_complete_field_types_exact_subset`, and
`cpython_ast_constructor_non_str_kwarg_exact_subset` split all current CPython
constructor methods out of broader first-pass coverage into direct
method-level Rust tests.

Additional `CopyTests` evidence:
`cpython_ast_copy_pickling_exact_subset`,
`cpython_ast_copy_with_parents_exact_subset`,
`cpython_ast_copy_replace_interface_exact_subset`,
`cpython_ast_copy_replace_native_exact_subset`,
`cpython_ast_copy_replace_accept_known_class_fields_exact_subset`,
`cpython_ast_copy_replace_accept_known_class_attributes_exact_subset`,
`cpython_ast_copy_replace_accept_known_custom_class_attributes_exact_subset`,
`cpython_ast_copy_replace_ignore_known_custom_instance_fields_exact_subset`,
`cpython_ast_copy_replace_reject_missing_field_exact_subset`,
`cpython_ast_copy_replace_accept_missing_field_with_default_exact_subset`,
`cpython_ast_copy_replace_reject_known_custom_instance_fields_commits_exact_subset`,
`cpython_ast_copy_replace_reject_unknown_instance_fields_exact_subset`, and
`cpython_ast_copy_replace_non_str_kwarg_exact_subset` split most current
CPython copy methods out of broader first-pass coverage into direct
method-level Rust tests. `cpython_ast_copy_replace_accept_known_custom_class_fields_first_pass_subset`
keeps the remaining custom-field method covered except for CPython string
object identity.

Additional `EndPositionTests` evidence: `cpython_ast_func_def_end_positions_exact_subset`,
`cpython_ast_class_def_end_positions_exact_subset`,
`cpython_ast_tuples_end_positions_exact_subset`,
`cpython_ast_displays_end_positions_exact_subset`,
`cpython_ast_source_segment_endings_exact_subset`,
`cpython_ast_source_segment_tabs_exact_subset`, and
`cpython_ast_source_segment_newlines_exact_subset` split the corresponding
CPython methods out of broader first-pass coverage into direct method-level
Rust tests.

Additional `EndPositionTests` evidence:
`cpython_ast_suites_end_positions_exact_subset`,
`cpython_ast_fstring_end_positions_exact_subset`,
`cpython_ast_fstring_multi_line_end_positions_exact_subset`,
`cpython_ast_import_from_multiline_end_positions_exact_subset`,
`cpython_ast_comprehensions_end_positions_exact_subset`, and
`cpython_ast_yield_await_end_positions_exact_subset` split the corresponding
CPython methods out of broader first-pass coverage into direct method-level
Rust tests.

Additional `EndPositionTests` evidence:
`cpython_ast_call_end_positions_exact_subset`,
`cpython_ast_multi_line_str_end_positions_exact_subset`,
`cpython_ast_continued_str_end_positions_exact_subset`,
`cpython_ast_slices_end_positions_exact_subset`,
`cpython_ast_binop_end_positions_exact_subset`,
`cpython_ast_boolop_end_positions_exact_subset`,
`cpython_ast_redundant_parenthesis_end_positions_exact_subset`,
`cpython_ast_trailers_with_redundant_parenthesis_end_positions_exact_subset`,
and `cpython_ast_source_segment_multi_exact_subset` split the remaining
CPython methods out of broader first-pass coverage into direct method-level
Rust tests.

Additional `AST_Tests` evidence: `cpython_ast_parse_invalid_ast_subset`
ports CPython `test_parse_invalid_ast`, including the `optimize=-1/0/1/2`
matrix for rejecting non-root public AST nodes as `ast.parse()` input.
`cpython_ast_base_classes_exact_subset` ports CPython `test_base_classes`,
checking representative concrete and abstract public AST class inheritance via
`issubclass()`.
`cpython_ast_parse_null_bytes_subset` ports `test_null_bytes`, preserving the
public `SyntaxError` message for NUL bytes in source strings passed to
`ast.parse()`.
`cpython_ast_parse_optimize_debug_subset` ports
`test_optimization_levels__debug__`, proving that `ast.parse()` preserves
`__debug__` as a `Name` at `optimize=-1/0` and folds it to
`Constant(False)` at `optimize=1/2` for both source strings and public AST
input. `cpython_ast_invalid_position_information_subset` ports
`test_invalid_position_information`, covering invalid line and column ranges on
compiled public AST nodes. `cpython_ast_negative_locations_compile_subset`
ports `test_negative_locations_for_compile`, preserving the accepted
negative-location cases that must not crash or raise during `compile()` and
`ast.parse(..., optimize=2)`.
`cpython_ast_binop_and_dotted_decorator_locations_subset` ports
`test_issue18374_binop_col_offset` and
`test_issue39579_dotted_name_end_col_offset`, covering nested binary-operation
end positions with explicit line joining and end positions on dotted
decorator attributes.
`cpython_ast_tstring_structure_subset` ports `test_tstring`, covering
parser-generated `TemplateStr`, literal `Constant`, and `Interpolation`
public-AST structure for t-strings.
`cpython_ast_filter_syntax_warnings_by_module_subset` ports
`test_filter_syntax_warnings_by_module` for tokenizer-originated
`SyntaxWarning` capture through `ast.parse()`, including default
`<unknown>` filenames and explicit filename/module parse calls.
`cpython_ast_repr_first_pass_subset`,
`cpython_ast_repr_eval_expression_snapshot_subset`, and
`cpython_ast_repr_large_input_crash_subset` port the first `test_repr`
snapshots plus the large-input repr regression, covering CPython-style
structural `repr()` output for parsed
modules, functions, classes, docstrings, returns, deletes, assignments,
assignment targets, annotated assignments, augmented assignments, for/while/if,
with, raise, try/except/finally, assert, import/from-import/lazy-import,
global, expr/pass/break/continue, comprehensions, async statements, unpacking,
yield/yield-from, decorators, named expressions, positional-only arguments,
type aliases, generic classes/functions, match statements, expression forms
from `snippets.py::eval_tests`, long-list compression, and `ValueError`
propagation for oversized integer decimal conversion inside AST repr.
`cpython_ast_pep758_feature_version_subset`
ports `test_pep758_except_without_parens`,
`test_pep758_except_star_without_parens`, and the single-expression acceptance
part of `test_pep758_except_with_single_expr`.
`cpython_ast_feature_version_gates_subset` ports
`test_positional_only_feature_version`,
`test_assignment_expression_feature_version`, `test_pep750_tstring`,
`test_exception_groups_feature_version`, `test_type_params_feature_version`,
`test_type_params_default_feature_version`,
`test_invalid_major_feature_version`, and
`test_conditional_context_managers_parse_with_low_feature_version`.
`cpython_ast_docstring_optimization_single_node_subset`
and `cpython_ast_docstring_optimization_multiple_nodes_subset` port
`test_docstring_optimization_single_node` and
`test_docstring_optimization_multiple_nodes` for class, function, and
async-function bodies. `cpython_ast_constant_name_validation_subset` ports
`test_invalid_identifier`, `test_constant_as_name`, and
`test_constant_as_unicode_name`. `cpython_ast_compare_first_pass_subset` ports
the first `test_compare_*` surface, including basic structural comparisons,
mutated runtime `_fields` / `_attributes`, literal type exactness, fieldless
operator nodes, missing runtime fields, and `compare_attributes=True`.
`cpython_ast_compare_modes_snippets_subset` ports `test_compare_modes` over
the current CPython `Lib/test/test_ast/snippets.py` exec/eval/single sample
sets through MiniPython's public `ast.parse()` and `ast.compare()` APIs.
`cpython_ast_snippets_public_to_tuple_first_pass_subset` ports the first
public-AST `to_tuple()` shapes from CPython `AST_Tests.test_snippets` for
functions, classes, return/delete statements, `for`/`while`/`if`/`with`
control flow, `try`/`try*`, `raise`/`assert`, ordinary and lazy imports,
`global`, `pass`/`break`/`continue`, `for` unpacking targets, comprehension
source spans, async functions/loops/context managers, unpacking displays, and
`yield` / `yield from`, decorated definitions including generator-argument
decorators, named expressions, positional-only parameters and defaults, type
aliases, and generic class/function/type-alias type parameters, plus `eval` /
`single` mode expression trees, including source positions and
compile-from-public-AST round-trips.
`cpython_ast_snippets_eval_to_tuple_core_expr_subset` extends that CPython
`snippets.py::eval_tests` public-AST evidence to constants, boolean operators,
binary operators, unary operators, lambda, dict, and set expression nodes.
`cpython_ast_snippets_eval_to_tuple_display_comp_subset` extends it to
multi-line displays plus list, set, dict, and generator comprehensions with
tuple/list targets. `cpython_ast_snippets_eval_to_tuple_compare_call_slice_subset`
adds comparison chains, call forms with interleaved keywords and `*` / `**`
unpacking, generator arguments, attributes, subscripts, omitted-bound slices,
tuple/list displays, and conditional expressions, including
compile-from-public-AST round-trips.
`cpython_ast_snippets_eval_to_tuple_interpolated_string_subset` completes the
remaining `snippets.py::eval_tests` public-AST interpolated-string batch for
f-string `JoinedStr` / `FormattedValue` and t-string `TemplateStr` /
`Interpolation` nodes, including format-spec source spans and
compile-from-public-AST round-trips.
`cpython_ast_validator_basic_errors_subset`
ports `test_invalid_sum`, `test_invalid_constant`, and
`test_empty_yield_from`.

Additional `ASTHelpers_Test` evidence: `cpython_ast_increment_lineno_on_module_type_ignores_subset`
ports CPython `test_increment_lineno_on_module`, including parsed `TypeIgnore`
nodes, tags, and line-number increments through `Module.type_ignores`.
`cpython_ast_fix_missing_locations_module_append_subset` ports the exact
`test_fix_missing_locations` case that appends a hand-built `Expr` to a parsed
module before filling missing locations.
`cpython_ast_increment_lineno_exact_subset` ports the exact
`test_increment_lineno` root-vs-child line-number increment snapshots and the
`end_lineno is None` preservation case.
`cpython_ast_copy_location_call_none_attrs_subset` ports the remaining exact
`test_copy_location` call-node `None` location-attribute case.
`cpython_ast_iter_helpers_exact_subset` ports the exact `test_iter_fields` and
`test_iter_child_nodes` call-node assertions for field dictionaries, child
count, child order, and keyword-node dump output.
`cpython_ast_get_docstring_exact_subset` ports CPython `test_get_docstring`
for module/class/function/async-function docstrings, `clean=False`, and
unsupported-node `TypeError`.
`cpython_ast_get_docstring_none_exact_subset` ports CPython
`test_get_docstring_none` across empty modules, module-level non-docstring
assignments, classes, functions, and async functions.
`cpython_ast_importfrom_level_none_validation_subset` ports CPython
`test_bad_integer` and `test_level_as_none` behavior for public
`ast.ImportFrom` nodes, including explicit `lineno=None` / `col_offset=None`
validation and `level=None` execution as an absolute import.
`cpython_ast_bad_integer_exact_subset` and
`cpython_ast_level_as_none_exact_subset` split those two CPython methods into
direct method-level checks.
`cpython_ast_literal_eval_complex_full_subset` and
`cpython_ast_literal_eval_complex_exact_subset` port CPython
`test_literal_eval_complex`, including all accepted signed real/imaginary
forms and the rejected non-literal complex expression shapes.
`cpython_ast_literal_eval_str_int_limit_exact_subset` ports CPython
`test_literal_eval_str_int_limit` as direct method-level evidence for decimal
integer digit limits and unlimited hexadecimal literal parsing.
`cpython_ast_literal_eval_malformed_dict_nodes_exact_subset`,
`cpython_ast_literal_eval_trailing_ws_exact_subset`,
`cpython_ast_literal_eval_malformed_lineno_exact_subset`, and
`cpython_ast_literal_eval_syntax_errors_exact_subset` port the corresponding
CPython `ASTHelpers_Test` methods as direct method-level evidence.
`cpython_ast_recursion_direct_exact_subset` and
`cpython_ast_recursion_indirect_exact_subset` port CPython
`test_recursion_direct` and `test_recursion_indirect` as direct method-level
checks for compile-from-public-AST `RecursionError`.
`cpython_ast_dump_exact_subset`, `cpython_ast_dump_indent_exact_subset`,
`cpython_ast_dump_incomplete_exact_subset`, and
`cpython_ast_dump_show_empty_exact_subset` split CPython `test_dump`,
`test_dump_indent`, `test_dump_incomplete`, and `test_dump_show_empty` into
direct method-level evidence.
`cpython_ast_parse_exact_subset`, `cpython_ast_parse_in_error_exact_subset`, and
`cpython_ast_literal_eval_exact_subset` split CPython `test_parse`,
`test_parse_in_error`, and `test_literal_eval` into direct method-level
evidence.
`cpython_ast_copy_location_exact_subset`,
`cpython_ast_fix_missing_locations_exact_subset`, and
`cpython_ast_increment_lineno_on_module_exact_subset` split CPython
`test_copy_location`, `test_fix_missing_locations`, and
`test_increment_lineno_on_module` into direct method-level evidence.
`cpython_ast_multiline_docstring_location_exact_subset`,
`cpython_ast_elif_stmt_start_position_exact_subset`,
`cpython_ast_elif_stmt_start_position_with_else_exact_subset`, and
`cpython_ast_starred_expr_end_position_within_call_exact_subset` split CPython
`test_multi_line_docstring_col_offset_and_lineno_issue16806`,
`test_elif_stmt_start_position`, `test_elif_stmt_start_position_with_else`,
and `test_starred_expr_end_position_within_call` into direct method-level
evidence.

Additional `ASTValidatorTests` evidence:
`cpython_ast_validator_module_exact_subset` splits CPython `test_module` into
direct method-level evidence.
`cpython_ast_validator_delete_exact_subset`,
`cpython_ast_validator_assign_exact_subset`, and
`cpython_ast_validator_augassign_exact_subset` split CPython `test_delete`,
`test_assign`, and `test_augassign` into direct method-level evidence.
`cpython_ast_validator_core_expr_exact_subset` splits CPython `test_expr`,
`test_boolop`, `test_unaryop`, `test_yield`, and `test_compare` into direct
method-level evidence.
`cpython_ast_validator_lambda_exact_subset`,
`cpython_ast_validator_ifexp_exact_subset`,
`cpython_ast_validator_dict_exact_subset`,
`cpython_ast_validator_set_exact_subset`,
`cpython_ast_validator_call_exact_subset`,
`cpython_ast_validator_attribute_exact_subset`,
`cpython_ast_validator_subscript_exact_subset`,
`cpython_ast_validator_starred_exact_subset`,
`cpython_ast_validator_list_exact_subset`, and
`cpython_ast_validator_tuple_exact_subset` split CPython `test_lambda`,
`test_ifexp`, `test_dict`, `test_set`, `test_call`, `test_attribute`,
`test_subscript`, `test_starred`, `test_list`, and `test_tuple` into direct
method-level evidence.
`cpython_ast_validator_listcomp_exact_subset`,
`cpython_ast_validator_setcomp_exact_subset`,
`cpython_ast_validator_generatorexp_exact_subset`, and
`cpython_ast_validator_dictcomp_exact_subset` split CPython `test_listcomp`,
`test_setcomp`, `test_generatorexp`, and `test_dictcomp` into direct
method-level evidence.
`cpython_ast_validator_funcdef_exact_subset`,
`cpython_ast_validator_classdef_exact_subset`,
`cpython_ast_validator_try_exact_subset`, and
`cpython_ast_validator_try_star_exact_subset` split CPython `test_funcdef`,
`test_classdef`, `test_try`, and `test_try_star` into direct method-level
evidence.
`cpython_ast_validator_for_exact_subset`,
`cpython_ast_validator_while_exact_subset`,
`cpython_ast_validator_if_exact_subset`,
`cpython_ast_validator_with_exact_subset`,
`cpython_ast_validator_raise_exact_subset`,
`cpython_ast_validator_assert_exact_subset`,
`cpython_ast_validator_import_exact_subset`,
`cpython_ast_validator_importfrom_exact_subset`,
`cpython_ast_validator_global_exact_subset`, and
`cpython_ast_validator_nonlocal_exact_subset` split CPython `test_for`,
`test_while`, `test_if`, `test_with`, `test_raise`, `test_assert`,
`test_import`, `test_importfrom`, `test_global`, and `test_nonlocal` into
direct method-level evidence.

## `Lib/test/test_grammar.py::TokenTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_backslash` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-backslash-line-continuation`, comparing the line-continuation result and comment-backslash behavior directly against CPython. | None for this method. |
| `test_plain_integers` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-plain-integers-method`, covering exact type equality for zero literals, prefixed integer equality, invalid `eval("0x")`, 64-bit `sys.maxsize`, signed min-int equality, positive oversized prefixed literals, and no-overflow `eval()` of huge integer strings. | None for this method. |
| `test_long_integers` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-long-integers-method`, covering every exact large integer literal spelling from the CPython method plus representative equality, ordering, and subtraction checks against CPython. | None for this method. |
| `test_floats` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-floats-method`, covering every exact float literal spelling from the CPython method plus repr/type/equality checks against CPython. | None for this method. |
| `test_float_exponent_tokenization` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-float-exponent-tokenization-method` for both accepted lowercase `else` boundary spellings, and `cpython_rejection_parity_smoke_subset` includes `grammar-token-float-exponent-tokenization-uppercase-else` for the rejected non-keyword `Else` spelling. | None for this method. |
| `test_underscore_literals` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-underscore-literals-method`, proving the full CPython `VALID_UNDERSCORE_LITERALS` table evaluates like its underscore-free spelling, the full `INVALID_UNDERSCORE_LITERALS` table raises `SyntaxError`, and `_0` raises `NameError`. | None for this method. |
| `test_bad_numerical_literals` | `ported` | `cpython_bad_numerical_literals_subset` rejects the source forms, and `cpython_syntax_error_message_parity_subset` now includes all 18 CPython `check(...)` cases from this method with matching CPython/MiniPython error-message parity. | None for this method. |
| `test_end_of_numerical_literals` | `ported` | `cpython_end_of_numerical_literals_subset` now covers every CPython source generated by this method: accepted numeric literals, warning cases for keyword/soft-keyword boundaries, direct error cases without warnings, warning-as-error behavior for `is`, non-ASCII fraction-slash rejection, and the three hexadecimal list-comprehension boundary forms. | None for this method. |
| `test_string_literals` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-string-literals-method`, covering the exact CPython assertions for empty strings, quote escaping, `ord()` values, double/single quote equivalence, triple-quoted strings, and explicit backslash line joining. | None for this method. |
| `test_string_prefixes` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-string-prefixes-method`, mirroring CPython's `eval()`-based checks for `u`, `r`, `rf`, and `fr` string prefixes and proving each parses to non-empty `str` values with CPython-matching repr output. | None for this method. |
| `test_bytes_prefixes` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-bytes-prefixes-method`, mirroring CPython's `eval()`-based checks for `b`, `br`, and `rb` bytes prefixes and proving each parses to non-empty `bytes` values with CPython-matching repr output. | None for this method. |
| `test_ellipsis` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-ellipsis-method`, proving `... is Ellipsis` and `eval(".. .")` raises `SyntaxError` like CPython. | None for this method. |
| `test_eof_error` | `ported` | `cpython_syntax_error_message_parity_subset` includes all three CPython samples and proves MiniPython reports `was never closed` for each unterminated function-header parenthesis. | None for this method. |
| `test_max_level` | `ported` | `cpython_program_output_parity_smoke_subset` includes `grammar-token-max-level-method`, proving 200 nested parentheses evaluate to `()` and 201 nested parentheses raise `SyntaxError` with `too many nested parentheses`. | None for this method. |

## `Lib/test/test_grammar.py::GrammarTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_eval_input` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` includes the exact CPython `eval('1, 0 or 1')` shape and asserts the tuple result. | None for this method. |
| `test_var_annot_basics` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` covers annotated names with and without values, annotated builtin attributes, annotated subscript expressions, annotation storage, and the final list mutation assertion from CPython. | None for this method. |
| `test_var_annot_syntax_errors` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` rejects all 15 parser, AST-pass, and symtable-pass source strings from the CPython method. | None for this method. |
| `test_var_annot_basic_semantics` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` covers execution order, global declaration over a globally annotated name, function-local skipped annotation evaluation, empty function `__annotations__`, simple annotated local binding with `UnboundLocalError`, parenthesized name annotations as non-simple `NameError`, parenthesized annotated assignment with a value, exact class `__annotations__` for private and public names, class-body target failures, and catchable class-body `NameError`. | None for this method. |
| `test_annotations_inheritance` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` covers the full class hierarchy from CPython and proves classes without local annotations return their own empty `__annotations__`. | None for this method. |
| `test_var_annot_module_semantics` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` covers `test.__annotations__`, `test.typinganndata.ann_module.__annotations__`, `ann_module.M.__annotations__`, and `ann_module2.__annotations__` through the CPython import paths, including `typing.Tuple[int, int]` and PEP 604 union annotations. | None for this method. |
| `test_var_annot_in_module` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` imports the CPython fixture path `test.typinganndata.ann_module3` and proves `f_bad_ann()`, `g_bad_ann()`, and `D_bad_ann(5)` raise catchable `NameError`s with CPython-style messages. | None for this method. |
| `test_var_annot_simple_exec` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` covers CPython's separate `exec(source, gns, lns)` scope shape: `__annotate__` stays out of globals, locals keep the assigned value, and `lns["__annotate__"](annotationlib.Format.VALUE)` returns `{'x': int}`. | None for this method. |
| `test_var_annot_rhs` | `ported` | `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` covers annotated RHS tuple assignment, `yield` RHS inside a function created by `exec`, and starred tuple RHS through `from typing import Tuple` with `Tuple[int, ...]`. | None for this method. |
| `test_funcdef` | `ported` | `cpython_grammar_tests_funcdef_first_pass_subset` ports the current CPython method across function `__code__.co_varnames`, ordinary/vararg/defaulted calls, keyword-only parameters, rejected bad parameter lists and bad call-unpack syntax, keyword-after-star and `**kwargs` calls, evaluated annotations including positional-only and private-name mangling, PEP 614 decorator expressions, closure capture shapes, and trailing comma parameter lists. | None for this method. |
| `test_lambdef` | `ported` | `cpython_grammar_lambda_subset` covers the current CPython method-level lambda body shapes, nested/default lambdas, ordinary/defaulted/keyword-only calls, invalid assignment and tuple-parameter syntax, trailing-comma parameter lists, and the uncalled `lambda: a[d]` expression boundary. | None for this method. |
| `test_simple_stmt` | `ported` | `cpython_simple_stmts_subset` ports the current CPython method-level top-level and function-body `x = 1; pass; del x` shapes, including semicolon-separated simple statements and the optional trailing semicolon. | None for this method. |
| `test_expr_stmt` | `ported` | `cpython_grammar_tests_expr_stmt_subset` ports the current CPython method-level expression statements, tuple-valued assignments, chained assignments, unpacking targets, the mixed chained/unpacking assignment, and both invalid assignment-target cases. | None for this method. |
| `test_former_statements_refer_to_builtins` | `ported` | `cpython_grammar_tests_former_statements_refer_to_builtins_subset` ports the current CPython method-level `print foo` / `exec foo` diagnostics at top level, inline-suite, and indented-block positions, and verifies malformed parenthesized variants stay generic syntax errors. | None for this method. |
| `test_del_stmt` | `ported` | `cpython_grammar_tests_del_stmt_subset` ports the current CPython method-level delete sequence, including name targets, nested tuple/list delete targets, empty tuple delete, slice delete, and compile-only complex delete targets; focused delete helper tests cover the observable runtime effects and diagnostics. | None for this method. |
| `test_pass_stmt` | `ported` | `cpython_grammar_pass_statement_subset` ports the current CPython method-level bare `pass` statement and additional no-op pass contexts. | None for this method. |
| `test_break_stmt` | `ported` | `cpython_grammar_break_continue_subset` ports the current CPython method-level `while 1: break` shape and broader observable loop-break behavior. | None for this method. |
| `test_continue_stmt` | `ported` | `cpython_grammar_break_continue_subset` ports the current CPython method-level inline `while i: i = 0; continue` shape plus the try/except and try/finally continue-in-loop cases. | None for this method. |
| `test_break_continue_loop` | `ported` | `cpython_grammar_break_continue_subset` ports the current CPython nested continue-then-break try/except loop regression shape through an observable `test_inner()` result. | None for this method. |
| `test_return` | `ported` | `cpython_ast_return_stmt_subset` ports the current CPython method-level bare return, value return, unparenthesized starred tuple return, and rejected class-body return shape. | None for this method. |
| `test_control_flow_in_finally` | `ported` | `cpython_control_flow_in_finally_override_subset` ports every current CPython method-level case: six break-in-finally overrides, six continue-in-finally overrides, three return-in-finally overrides, and the four issue #37830 return-with-break/continue-in-finally cases. | None for this method. |
| `test_yield` | `ported` | `cpython_grammar_yield_stmt_subset` ports the current CPython method-level standalone yield/yield-from definitions, yield RHS definitions, implicit tuple yield forms, parenthesized subexpression and call-argument yield forms, unparenthesized syntax rejections, top-level/class-scope rejections, and the annotation-yield rejection; focused generator tests cover send/throw/close/yield-from runtime behavior. | None for this method. |
| `test_yield_in_comprehensions` | `ported` | `cpython_grammar_yield_stmt_subset` and `cpython_invalid_comprehension_subset` port the current CPython method-level yield-in-comprehension cases: allowed yield/yield-from in the outer iterable, and rejected yield/yield-from in list/set/dict/generator comprehension element, filter, inner iterable, target, module-level, and class-body positions. | None for this method. |
| `test_raise` | `ported` | `cpython_grammar_raise_and_try_except_subset` ports the current CPython method-level `raise RuntimeError('just testing')` and `raise KeyboardInterrupt` try/except shapes, plus broader raise/cause/context/except matching behavior. | None for this method. |
| `test_import` | `ported` | `cpython_grammar_import_stmt_subset` ports the current CPython method-level ordinary import, multi-import, `from time import time`, parenthesized from-import, `from sys import path, argv`, and parenthesized from-import with and without a trailing comma. | None for this method. |
| `test_global` | `ported` | `cpython_grammar_global_stmt_subset` ports the current CPython method-level `global a`, `global a, b`, and ten-name global declaration shapes, plus executable global write behavior. | None for this method. |
| `test_nonlocal` | `ported` | `cpython_scope_closure_and_nonlocal_subset` ports the current CPython method-level nested `nonlocal x` and `nonlocal x, y` declarations, plus executable nonlocal read/write behavior. | None for this method. |
| `test_assert` | `ported` | `cpython_grammar_assert_stmt_subset` ports the current CPython method-level truthy assert forms, message expression forms, lambda assert forms, and `assert True` / `assert True, msg` non-failure behavior. | None for this method. |
| `test_assert_failures` | `ported` | `cpython_grammar_assert_stmt_subset` ports the current CPython method-level assertion failure object behavior by catching `AssertionError as e`, reading `e.args[0]` for `assert 0, "msg"`, and proving bare `assert False` leaves `e.args` empty. | None for this method. |
| `test_assert_syntax_warnings` | `ported` | `cpython_grammar_assert_stmt_subset` ports the current CPython non-empty tuple-condition warning cases for `assert(x, "msg")`, `assert(False, "msg")`, and `assert(False,)`, while preserving no-warning behavior for ordinary assert-message syntax. | None for this method. |
| `test_assert_warning_promotes_to_syntax_error` | `ported` | `cpython_grammar_assert_stmt_subset` ports the current CPython warning-as-error behavior through MiniPython's static warning-as-error path: ordinary `assert x, "msg"` has no promoted warning, while tuple-condition assert forms are promoted to errors. | None for this method. |
| `test_if` | `ported` | `cpython_grammar_if_else_subset` ports the current CPython method-level inline `if 1: pass`, `if/else`, `if/elif`, and multi-`elif`/`else` source shapes, while focused branch tests cover observable true/false/elif execution. | None for this method. |
| `test_while` | `ported` | `cpython_grammar_while_subset` ports the current CPython method-level `while 0: pass`, `while 0: pass else: pass`, and Issue1920 `while 0` optimized-away-else-preserved shape, with broader loop/else behavior covered by focused runtime tests. | None for this method. |
| `test_for` | `ported` | `cpython_grammar_for_subset` ports the current CPython method-level inline tuple-iterable loop, empty iterable `for ... else`, growing sequence-protocol iteration through `__getitem__`, single-target tuple unpacking, and starred iterable list concatenation shapes. | None for this method. |
| `test_try` | `ported` | `cpython_grammar_raise_and_try_except_subset` ports the current CPython method-level typed, bare, tuple, comma-list, and tuple-`as` `except` forms, `try/finally`, `else`, and invalid `except Exception as a.b` / `a[b]` targets, with broader raise/cause/context behavior covered by focused runtime tests. `catches_dotted_exception_handler_type` and `catches_dynamic_exception_handler_type_expression` add runtime coverage for dotted and dynamic exception handler types. | None for this method. |
| `test_try_star` | `ported` | `cpython_grammar_try_star_subset` ports the current CPython method-level typed, tuple, comma-list, tuple-`as`, and invalid bare/attribute/subscript `except*` forms plus `try/finally`, with broader ExceptionGroup split and except-star restrictions covered by focused runtime tests. | None for this method. |
| `test_suite` | `ported` | `cpython_grammar_suite_and_dedent_subset` and `cpython_grammar_suite_comments_and_pass_subset` port the current CPython method-level inline suite, indented pass suite, and comment-only-line/pass sequence inside an indented suite. | None for this method. |
| `test_test` | `ported` | `cpython_grammar_boolean_operations_subset` ports the current CPython method-level `not`, `and`, `or`, nested `not`, and mixed boolean-chain `if ...: pass` source shapes, with observable truthiness and operand-return semantics covered by focused runtime tests. | None for this method. |
| `test_comparison` | `ported` | `cpython_grammar_chained_comparison_subset`, `cpython_grammar_identity_comparison_subset`, `cpython_grammar_membership_comparison_subset`, and `cpython_comparison_helper_rules_subset` port the current CPython method-level truthy condition, equality/ordering operators, identity, membership, and long mixed chained-comparison source shapes. | None for this method. |
| `test_comparison_is_literal` | `ported` | `cpython_grammar_identity_literal_warning_subset` ports every current CPython warning source for `is` / `is not` against ordinary int, str, and tuple literals, including chained comparisons, plus the no-warning singleton identity checks for `None`, `False`, `True`, and `...` under warning-as-error mode. | None for this method. |
| `test_warn_missed_comma` | `ported` | `cpython_grammar_warn_missed_comma_subset` ports the current CPython method-level callable, subscriptable, and invalid-index `SyntaxWarning` shapes, including the no-warning lambda-call, name/int/bool/slice-index, and dict-key tuple cases. | None for this method. |
| `test_binary_mask_ops` | `ported` | `cpython_grammar_bitwise_and_shift_subset` ports the current CPython method-level bitwise-and, bitwise-xor, and bitwise-or assignment shapes and checks their executable values. | None for this method. |
| `test_shift_ops` | `ported` | `cpython_grammar_bitwise_and_shift_subset` ports the current CPython method-level left-shift, right-shift, and chained-shift assignment shapes and checks their executable values. | None for this method. |
| `test_additive_ops` | `ported` | `cpython_grammar_additive_ops_subset` ports the current CPython method-level plain, additive, subtractive, and mixed `+` / `-` assignment shapes and checks left-associative executable values. | None for this method. |
| `test_multiplicative_ops` | `ported` | `cpython_grammar_multiplicative_ops_subset` ports the current CPython method-level `*`, `/`, `%`, and mixed multiplicative assignment shapes and checks executable values. | None for this method. |
| `test_unary_ops` | `ported` | `cpython_grammar_unary_ops_subset` ports the current CPython method-level unary plus, unary minus, invert, mixed unary/bitwise, and chained unary/arithmetic assignment shapes and checks executable values. | None for this method. |
| `test_selectors` | `ported` | `cpython_grammar_selectors_subset` ports the current CPython method-level import/module attribute call chain, `sys.path[0]`, `sys.modules['time'].time()`, string index/slice shapes, dict tuple-key selector assignments, and deterministic sorted key-list assertion. | None for this method. |
| `test_atoms` | `ported` | `cpython_grammar_atoms_subset` ports the current CPython method-level grouped-expression, tuple-display, list-display, empty dict, dict literal, boolean-expression dict key/value, set literal, name, string, and number atom shapes, with focused tests still covering comprehensions, ellipsis, singletons, and generator displays. | None for this method. |
| `test_classdef` | `ported` | `cpython_grammar_classdef_method_subset` ports the current CPython method-level bare class, empty-parentheses class, single and multiple inheritance, class-body method definitions, simple class decorator, and all PEP 614 class decorator expression shapes including boolean, named-expression, lambda, subscript, decorator-call-chain, and `__call__.__call__` decorators. | None for this method. |
| `test_dictcomps` | `ported` | `cpython_grammar_dictcomps_method_subset` ports the current CPython method-level dict comprehension `{i:i+1 for i in nums}` and exact resulting dictionary. | None for this method. |
| `test_listcomps` | `ported` | `cpython_grammar_listcomps_method_subset` ports the current CPython method-level strip, arithmetic, filtered, nested-for, nested-listcomp, lambda/listcomp, function-local, nested-front, invalid listcomp syntax, and supplier/part join list-comprehension shapes. | None for this method. |
| `test_genexps` | `ported` | `cpython_grammar_genexps_method_subset` ports the current CPython method-level generator-of-list, StopIteration, non-iterable TypeError, string-product generator, nested-generator, sum, filtered-sum, nested-list/generator, false-filter, and parenthesized-generator syntax-error shapes. | None for this method. |
| `test_comprehension_specials` | `ported` | `cpython_grammar_comprehension_specials_method_subset` ports the current CPython method-level outermost iterable precomputation, inner expression lazy lookup, adjacent `if` filters in list comprehensions and generator expressions, and single-element tuple-unpack targets. | None for this method. |
| `test_with_statement` | `ported` | `cpython_grammar_with_statement_method_subset` ports the current CPython method-level ordinary and parenthesized `with` forms, including no target, simple target, tuple-unpack target, multiple managers, mixed `as`/bare managers, trailing commas, and three-manager parenthesized groups while checking target bindings and nested cleanup order. | None for this method. |
| `test_if_else_expr` | `ported` | `cpython_grammar_if_else_expr_method_subset` ports the current CPython method-level lambda/list-comprehension conditional-expression shapes, branch short-circuiting with `_checkeval`, boolean/arith/comparison precedence cases, and `not` interaction. | None for this method. |
| `test_paren_evaluation` | `ported` | `cpython_grammar_paren_evaluation_method_subset` ports the current CPython method-level floor-division grouping examples and identity-comparison cases where parentheses change comparison-chain grouping. | None for this method. |
| `test_matrix_mul` | `ported` | `cpython_grammar_matrix_mul_method_subset` ports the current CPython method-level `@` and `@=` examples with `__matmul__`, `__imatmul__`, and instance attribute assignment. | None for this method. |
| `test_async_await` | `ported` | `cpython_grammar_async_await_method_subset` ports the current CPython method-level async function body, function `__name__`, `__code__.co_flags & inspect.CO_COROUTINE`, decorator, and decorated async-function custom attribute cases. | None for this method. |
| `test_async_for` | `ported` | `cpython_grammar_async_for_method_subset` ports the current CPython method-level async iterator, empty async-for body, tuple-unpack target, async-for `else`, and final user exception propagation cases. | None for this method. |
| `test_async_with` | `ported` | `cpython_grammar_async_with_method_subset` ports the current CPython method-level async context manager, no-target, name-target, tuple-unpack-target, multi-manager, and mixed `as`/bare manager forms. | None for this method. |
| `test_complex_lambda` | `ported` | `cpython_grammar_complex_lambda_method_subset` ports the current CPython method-level multi-line f-string replacement expression containing lambda keyword arguments and verifies the empty string result. | None for this method. |

## `Lib/test/test_syntax.py::SyntaxWarningTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_return_in_finally` | `ported` | `cpython_finally_control_flow_warning_subset` covers all three CPython return-in-finally source shapes: direct `return`, nested `try` body `return`, and nested `except` body `return`. | None for this method. |
| `test_break_and_continue_in_finally` | `ported` | `cpython_finally_control_flow_warning_subset` covers all six CPython loop-finally source shapes: direct, nested-`try`, and nested-`except` forms for both `break` and `continue`. | None for this method. |

## `Lib/test/test_syntax.py::SyntaxErrorTestCase` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_expression_with_assignment` | `ported` | `cpython_syntax_error_message_parity_subset` includes `expression-with-assignment-message`, proving the CPython message for assignment inside a call argument, and `cpython_invalid_call_argument_helper_rules_subset` pins the diagnostic span to the same argument start offset. | None for this method. |
| `test_curly_brace_after_primary_raises_immediately` | `ported` | `cpython_program_output_parity_smoke_subset` includes `syntax-error-curly-brace-after-primary-single-mode`, proving `compile("f{}", "<testcase>", "single")` raises `SyntaxError` with `invalid syntax`; `cpython_interactive_input_subset` pins the MiniPython interactive entry point to the same rejection. | None for this method. |
| `test_assign_call` | `ported` | `cpython_syntax_error_message_parity_subset` includes `assign-call-message`, proving `f() = 1` raises a SyntaxError whose text contains `assign`, matching CPython's method-level assertion. | None for this method. |
| `test_assign_del` | `ported` | `cpython_syntax_error_message_parity_subset` includes all 27 invalid-delete cases from this CPython method, covering invalid empty targets, literals, `None`, starred targets, function-call targets, conditional/named expressions, ordinary expressions, and `del a += b`. | None for this method. |
| `test_global_param_err_first` | `ported` | `cpython_syntax_error_message_parity_subset` includes `global-param-error-first-message`, proving the first reported error contains `parameter and global`; `cpython_scope_declaration_error_subset` pins the MiniPython compile diagnostic to line 3 at the `global` keyword. | None for this method. |
| `test_nonlocal_param_err_first` | `ported` | `cpython_syntax_error_message_parity_subset` includes `nonlocal-param-error-first-message`, proving the first reported error contains `parameter and nonlocal`; `cpython_scope_declaration_error_subset` pins the MiniPython compile diagnostic to line 3 at the `nonlocal` keyword. | None for this method. |
| `test_raise_from_error_message` | `ported` | `cpython_syntax_error_message_parity_subset` includes `raise-from-following-invalid-call-message`, proving the valid `raise AssertionError() from None` line does not mask the next-line call syntax error; `cpython_invalid_call_argument_helper_rules_subset` pins the invalid-call diagnostic to line 3 at the second comma. | None for this method. |
| `test_yield_outside_function` | `ported` | `cpython_syntax_error_message_parity_subset` includes all 9 `yield`-outside-function cases from this method across `if`, `else`, `while`, class body, and loop `else` contexts; `cpython_invalid_control_flow_context_subset` pins MiniPython compile errors for each shape. | None for this method. |
| `test_return_outside_function` | `ported` | `cpython_syntax_error_message_parity_subset` includes all 9 `return`-outside-function cases from this method across `if`, `else`, `while`, class body, and loop `else` contexts; `cpython_invalid_control_flow_context_subset` pins MiniPython compile errors for each shape. | None for this method. |
| `test_break_outside_loop` | `ported` | `cpython_syntax_error_message_parity_subset` includes all 7 `break`-outside-loop cases from this method across module, `if`, class body, and `with` contexts; `cpython_invalid_control_flow_context_subset` pins MiniPython compile errors and line-specific diagnostics for each shape. | None for this method. |
| `test_continue_outside_loop` | `ported` | `cpython_syntax_error_message_parity_subset` includes all 6 `continue`-outside-loop cases from this method across `if`, class body, and `with` contexts; `cpython_invalid_control_flow_context_subset` pins MiniPython's CPython-style `not properly in loop` compile errors and line-specific diagnostics for each shape. | None for this method. |
| `test_unexpected_indent` | `ported` | `cpython_syntax_error_message_parity_subset` includes the CPython `foo()` then indented `bar()` shape and proves both CPython and MiniPython report `unexpected indent`; `cpython_invalid_control_flow_context_subset` keeps a local rejection case for the same parse path. | None for this method. |
| `test_no_indent` | `ported` | `cpython_syntax_error_message_parity_subset` includes `if 1:\nfoo()` and `cpython_invalid_block_subset` covers the same no-suite semantic across supported compound statements with `expected an indented block`. | None for this method. |
| `test_bad_outdent` | `ported` | `cpython_syntax_error_message_parity_subset` includes `if 1:\n  foo()\n bar()` and `cpython_tokenize_unmatched_indentation_subset` covers matching-dedent rejection with CPython-style `unindent does not match` wording. | None for this method. |
| `test_kwargs_last` | `ported` | `cpython_syntax_error_message_parity_subset` includes `int(base=10, '2')`, and `cpython_invalid_call_argument_helper_rules_subset` keeps the same positional-after-keyword parse rejection. | None for this method. |
| `test_kwargs_last2` | `ported` | `cpython_syntax_error_message_parity_subset` includes `int(**{'base': 10}, '2')`, and `cpython_invalid_call_argument_helper_rules_subset` keeps the same positional-after-keyword-unpacking parse rejection. | None for this method. |
| `test_kwargs_last3` | `ported` | `cpython_syntax_error_message_parity_subset` includes `int(**{'base': 10}, *['2'])`, and `cpython_invalid_call_argument_helper_rules_subset` keeps the same iterable-unpacking-after-keyword-unpacking parse rejection. | None for this method. |
| `test_generator_in_function_call` | `ported` | `cpython_syntax_error_message_parity_subset` includes the exact CPython source shape, and `cpython_invalid_call_argument_helper_rules_subset` pins MiniPython's diagnostic to line 1 columns 11-53 for the unparenthesized generator expression. | None for this method. |
| `test_except_then_except_star` | `ported` | `cpython_syntax_error_message_parity_subset` includes the exact CPython mixed-handler source, and `cpython_invalid_control_flow_syntax_subset` pins MiniPython's diagnostic to line 3 columns 1-8 for `except*`. | None for this method. |
| `test_except_star_then_except` | `ported` | `cpython_syntax_error_message_parity_subset` includes the exact CPython mixed-handler source, and `cpython_invalid_control_flow_syntax_subset` pins MiniPython's diagnostic to line 3 columns 1-7 for `except`. | None for this method. |
| `test_empty_line_after_linecont` | `ported` | `cpython_program_output_parity_smoke_subset` includes the empty physical line after a continuation character; `cpython_tokenize_explicit_line_joining_subset` keeps local executable coverage for both CPython source shapes, including the split-indented-function shape that local Python 3.9 rejects but the checked-out CPython source expects to compile. | None for this method. |
| `test_continuation_bad_indentation` | `ported` | `cpython_tokenize_explicit_line_joining_subset` includes the exact CPython source shape and rejects it with MiniPython's CPython-style bad-outdent diagnostic. It is intentionally not in the system-Python differential suite because the local Python 3.9 accepts this shape while the checked-out CPython test expects rejection. | None for this method. |
| `test_disallowed_type_param_names` | `ported` | `cpython_disallowed_type_param_names_subset` rejects `__classdict__` for class, function, and type-alias type parameters while preserving parse acceptance for nested `__class__`, `__classcell__`, and `__classdictcell__` type parameters. It is intentionally subset-only because local Python 3.9 does not parse PEP 695 type parameters. | None for this method. |
| `test_barry_as_flufl_with_syntax_errors` | `ported` | `cpython_syntax_error_message_parity_subset` includes the exact regression source and targets current CPython's `expected ':'` parser message, while accepting local Python 3.9's legacy `invalid syntax`; `cpython_invalid_control_flow_syntax_subset` keeps local parser rejection coverage. | None for this method. |
| `test_invalid_line_continuation_error_position` | `ported` | `cpython_syntax_error_message_parity_subset` includes all three CPython invalid-continuation source shapes, and `cpython_tokenize_explicit_line_joining_subset` pins the MiniPython lexer diagnostics to the CPython line/column positions. | None for this method. |
| `test_invalid_line_continuation_left_recursive` | `ported` | `cpython_syntax_error_message_parity_subset` includes the invalid trailing-space left-recursive continuation shape; `cpython_tokenize_explicit_line_joining_subset` pins MiniPython lexer diagnostics for both the invalid trailing space and continuation-at-EOF cases. The EOF shape is subset-only because local Python 3.9 executes it as a `NameError` while the checked-out CPython source expects a compile-time EOF error. | None for this method. |
| `test_error_parenthesis` | `ported` | `cpython_syntax_error_message_parity_subset` includes unclosed `(`, `[`, `{`, unmatched `)`, `]`, `}`, and the nested mismatched opening-parenthesis example; `cpython_syntax_error_parenthesis_subset` covers the full CPython method shape including unclosed delimiters before a following assignment-like line, the `match` class-pattern EOF case, and the latin-cookie bytes source. `cpython_bytes_source_rejection_parity_subset` keeps the bytes-source rejection aligned with CPython. | None for this method. |
| `test_error_string_literal` | `ported` | `cpython_syntax_error_message_parity_subset` includes all six CPython source shapes for ordinary, escaped-end-quote, raw escaped-end-quote, and triple-quoted unterminated strings; `cpython_invalid_string_literal_subset` pins MiniPython lexer diagnostics and spans for the same shapes plus existing tokenizer string-error cases. | None for this method. |
| `test_invisible_characters` | `ported` | `cpython_syntax_error_message_parity_subset` includes the string-source `print\x17("Hello")` invalid non-printable character diagnostic, `cpython_tokenize_error_token_subset` pins both string and bytes-source lexer spans from the CPython method, and `cpython_bytes_source_rejection_parity_subset` keeps the bytes-source rejection aligned with CPython. | None for this method. |
| `test_match_call_does_not_raise_syntax_error` | `ported` | `cpython_program_output_parity_smoke_subset` runs the exact CPython compile-only source through CPython and MiniPython, and `cpython_soft_keyword_call_acceptance_subset` keeps local soft-keyword call coverage. | None for this method. |
| `test_case_call_does_not_raise_syntax_error` | `ported` | `cpython_program_output_parity_smoke_subset` runs the exact CPython compile-only source through CPython and MiniPython, and `cpython_soft_keyword_call_acceptance_subset` keeps local soft-keyword call coverage. | None for this method. |
| `test_multiline_compiler_error_points_to_the_end` | `ported` | `cpython_syntax_error_message_parity_subset` includes the exact multiline duplicate-keyword call source with CPython's `keyword argument repeated` wording, and `cpython_invalid_call_argument_helper_rules_subset` pins MiniPython's diagnostic to line 3 at the repeated keyword. | None for this method. |
| `test_multiline_string_concat_missing_comma_points_to_last_string` | `ported` | `cpython_syntax_error_message_parity_subset` includes the exact multiline string-concatenation missing-comma source and targets CPython's `Perhaps you forgot a comma` wording, while accepting local Python 3.9's legacy `invalid syntax`; `cpython_multiline_string_concat_missing_comma_subset` pins MiniPython's diagnostic to line 4 at the final adjacent string token. | None for this method. |
| `test_except_stmt_invalid_as_expr` | `ported` | `cpython_syntax_error_message_parity_subset` includes the exact `except ValueError as obj.attr` source and expects CPython's `cannot use except statement with attribute` wording, while `cpython_invalid_control_flow_syntax_subset` pins MiniPython's diagnostic span to the full `obj.attr` target. | None for this method. |
| `test_match_stmt_invalid_as_expr` | `ported` | `cpython_syntax_error_message_parity_subset` includes the exact `case x as obj.attr` source and expects CPython's `cannot use attribute as pattern target` wording, while `cpython_invalid_match_pattern_subset` pins MiniPython's diagnostic span to the full `obj.attr` target. | None for this method. |
| `test_ifexp_else_stmt` | `ported` | `cpython_invalid_expression_rules_subset` rejects every CPython statement keyword after conditional-expression `else`, and `cpython_syntax_error_message_parity_subset` covers the current CPython message for statement tokens in this area. | None for this method. |
| `test_ifexp_body_stmt_else_expression` | `ported` | `cpython_syntax_error_message_parity_subset` includes the CPython `pass`, `break`, and `continue` statement-body shapes, and `cpython_invalid_expression_rules_subset` pins MiniPython's `expected expression before 'if', but statement is given` diagnostic. | None for this method. |
| `test_ifexp_body_stmt_else_stmt` | `ported` | `cpython_syntax_error_message_parity_subset` includes a statement body with a statement `else` branch, and `cpython_invalid_expression_rules_subset` covers all CPython body/else statement pairs. | None for this method. |
| `test_nested_named_except_blocks` | `ported` | `cpython_static_nesting_and_complexity_limit_subset` generates nested named `except Exception as e` blocks and proves MiniPython rejects the over-limit shape with `too many statically nested blocks`. | None for this method. |
| `test_with_statement_many_context_managers` | `ported` | `cpython_static_nesting_and_complexity_limit_subset` ports the CPython context-manager generator shape, accepting the in-range case and rejecting the over-limit case with `too many statically nested blocks`. | None for this method. |
| `test_async_with_statement_many_context_managers` | `ported` | `cpython_static_nesting_and_complexity_limit_subset` ports the async context-manager generator shape, accepting the in-range case and rejecting the over-limit case with `too many statically nested blocks`. | None for this method. |
| `test_syntax_error_on_deeply_nested_blocks` | `ported` | `cpython_static_nesting_and_complexity_limit_subset` accepts the maximum supported nested `while` depth and rejects the CPython over-limit depth with `too many statically nested blocks`. | None for this method. |
| `test_error_on_parser_stack_overflow` | `ported` | `cpython_static_nesting_and_complexity_limit_subset` runs the CPython-style 100000-prefix unary source through exec, eval, and interactive MiniPython entry points and requires `too complex` without stack overflow. | None for this method. |
| `test_deep_invalid_rule` | `ported` | `cpython_static_nesting_and_complexity_limit_subset` rejects the CPython deep invalid-rule source without hanging or backtracking exponentially. | None for this method. |

## `Lib/test/test_syntax.py::LazyImportRestrictionTestCase` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_lazy_import_in_try_block` | `ported` | `cpython_lazy_import_syntax_subset` rejects lazy `import` and lazy `from ... import ...` in `try` bodies with CPython-style try/except-block messages. | None for this method. |
| `test_lazy_import_in_trystar_block` | `ported` | `cpython_lazy_import_syntax_subset` rejects lazy imports in `try` bodies that use `except*` handlers. | None for this method. |
| `test_lazy_import_in_except_block` | `ported` | `cpython_lazy_import_syntax_subset` rejects lazy imports inside an `except*` handler body. | None for this method. |
| `test_lazy_import_in_function` | `ported` | `cpython_lazy_import_syntax_subset` rejects lazy `import` and lazy `from ... import ...` in ordinary function bodies. | None for this method. |
| `test_lazy_import_in_async_function` | `ported` | `cpython_lazy_import_syntax_subset` rejects lazy `import` and lazy `from ... import ...` in async function bodies. | None for this method. |
| `test_lazy_import_in_class` | `ported` | `cpython_lazy_import_syntax_subset` rejects lazy `import` and lazy `from ... import ...` in class bodies. | None for this method. |
| `test_lazy_import_star_forbidden` | `ported` | `cpython_lazy_import_syntax_subset` rejects module-level `lazy from ... import *` and preserves function-context error priority for the same star form inside a function. | None for this method. |
| `test_lazy_import_nested_scopes` | `ported` | `cpython_lazy_import_syntax_subset` rejects lazy imports inside class-method, function-local class, and nested function scopes with the matching function/class diagnostic. | None for this method. |
| `test_lazy_import_valid_cases` | `ported` | `cpython_lazy_import_syntax_subset` uses `compile_source` for CPython's module-level compile-only valid lazy import forms, including aliases and `from ... import ... as ...`. | None for this method. |

## Immediate Method-Level Audit Order

1. Continue `Lib/test/test_ast/test_ast.py::AST_Tests.test_snippets` beyond
   the decorator, walrus, positional-only, and PEP 695 `to_tuple()` samples.
2. Continue partial `test_ast.py` classes method-by-method, especially
   `AST_Tests`, `ASTHelpers_Test`, `ASTValidatorTests`, and `EndPositionTests`.

The acceptance bar for moving a row to `ported` is deliberately high: every
method in the row needs a named Rust test or documented differential parity
case, and the relevant command must pass in `cargo test`.
