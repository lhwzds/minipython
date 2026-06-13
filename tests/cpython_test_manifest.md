# CPython Test Migration Manifest

This manifest tracks CPython test modules that directly pressure Python syntax,
AST shape, parser diagnostics, parser-coupled runtime behavior, and runtime
behavior already used by the supported MiniPython subset.

The counts below come from the local CPython checkout at
`/Volumes/samsung/GitHub/cpython`:

- `Lib/test/test_grammar.py`
- `Lib/test/test_syntax.py`
- `Lib/test/test_compile.py`
- `Lib/test/test_builtin.py`
- `Lib/test/test_complex.py`
- `Lib/test/test_float.py`
- `Lib/test/test_collections.py`
- `Lib/test/test_types.py`
- `Lib/test/test_type_comments.py`
- `Lib/test/test_type_params.py`
- `Lib/test/test_memoryview.py`
- `Lib/test/test_bytes.py`
- `Lib/test/test_ast/test_ast.py`
- `Lib/test/test_ast/snippets.py`

This file is deliberately stricter than `tests/cpython_grammar_inventory.md`.
The grammar inventory can say every grammar production has some Rust coverage;
this manifest asks whether the actual CPython test methods have equivalent
Rust coverage.

## Status Vocabulary

- `ported`: every CPython test method in the row has equivalent Rust coverage.
- `ported_public`: every portable public CPython method in the row has
  equivalent Rust coverage, and all remaining methods are explicitly classified
  as `blocked_by_cpython_internal`.
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
| `ported` | 55 | 656 |
| `ported_public` | 4 | 110 |
| `partial` | 11 | 492 |
| `blocked_by_runtime` | 5 | 15 |
| `blocked_by_ast_module` | 2 | 16 |
| `blocked_by_cpython_internal` | 5 | 10 |
| `not_started` | 0 | 0 |
| `source_data` | 18 | 0 |
| **Total** | 100 | 1299 |

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
| `Lib/test/test_compile.py` | `TestSpecifics` | 98 | `partial` | Current evidence covers selected syntax-adjacent cases including argument handling, literal leading zeroes, invalid named expressions, subscript behavior, dead-code compile regressions, type aliases, warning filtering, newline/indentation compile boundaries and leading-newline `co_firstlineno` / `co_lines()` behavior through `cpython_compile_specifics_newline_and_indentation_subset`, source string/bytes encoding boundaries through `cpython_compile_specifics_encoding_subset`, runtime `compile()` warning capture through `cpython_compile_specifics_runtime_warning_capture_subset`, finally-body compile warnings through `cpython_compile_specifics_warning_in_finally_subset`, module-filtered warning capture through `cpython_compile_specifics_filter_syntax_warnings_by_module_subset`, PEP 765 finally-control-flow warnings through `cpython_compile_specifics_pep_765_warning_subset`, `__debug__` assignment and builtins-module mutation behavior through `cpython_compile_specifics_debug_assignment_subset`, optimize-level docstring behavior through `cpython_compile_specifics_docstring_optimize_subset`, syntax-error compile/eval/exec boundaries through `cpython_compile_specifics_syntax_error_boundaries_subset`, `None` target rejection through `cpython_compile_specifics_none_assignment_subset`, import grammar through `cpython_compile_specifics_import_syntax_subset`, selected compile-stability regressions through `cpython_compile_specifics_compile_stability_subset`, invalid public-AST compile diagnostics through `cpython_compile_specifics_invalid_public_ast_subset`, public AST compile behavior through `cpython_compile_specifics_compile_ast_public_subset` and `cpython_compile_specifics_compile_ast_cpython_file_subset`, lambda code-object metadata through `cpython_compile_specifics_lambda_code_metadata_subset`, type-sensitive code-object constant non-merging through `cpython_compile_specifics_dont_merge_constants_public_subset`, private-name code-object metadata through `cpython_compile_specifics_name_mangling_code_varnames_subset`, integer boundary constants through `cpython_compile_specifics_integer_constant_edges_subset`, compile-time integer digit-limit behavior through `cpython_compile_specifics_int_literals_too_long_subset`, public compile/runtime regression shapes through `cpython_compile_specifics_public_regression_shapes_subset`, interactive single-statement compile boundaries through `cpython_compile_specifics_single_statement_subset`, dict display evaluation order through `cpython_compile_specifics_dict_evaluation_order_subset`, large dict literal preservation through `cpython_compile_specifics_big_dict_literal_subset`, compile filename/path-like behavior through `cpython_compile_specifics_compile_filename_subset`, compile argument conversion errors through `cpython_compile_specifics_compile_argument_conversion_subset`, memoryview source NUL handling through `cpython_compile_specifics_null_terminated_memoryview_subset`, explicit general mapping locals behavior for `exec()` through `cpython_compile_specifics_exec_general_mapping_locals_subset`, function line-table attribute forms through `cpython_compile_specifics_lineno_attribute_subset`, async-for implicit-return function line tables through `cpython_compile_specifics_line_number_implicit_return_after_async_for_subset`, implicit-return frame line behavior through `cpython_compile_specifics_lineno_after_implicit_return_subset`, implicit-return `if` function line tables through `cpython_compile_specifics_if_implicit_return_code_lines_subset`, conditional loop-backedge line tables through `cpython_compile_specifics_lineno_of_backward_jump_conditional_in_loop_subset`, synthetic jump try/loop line tables through `cpython_compile_specifics_synthetic_jump_line_tables_subset`, empty-block line propagation through `cpython_compile_specifics_lineno_propagation_empty_blocks_subset`, and nested generator-expression code-object line tables through `cpython_compile_specifics_line_number_genexp_subset`. All public-compatible current methods have method-level Rust evidence; the row remains `partial` because temp-file/child-process/resource-limit cases require future host-runtime policy and CPython bytecode/optimizer/instruction-position methods are intentionally classified as implementation-internal. |
| `Lib/test/test_compile.py` | `TestBooleanExpression` | 4 | `ported` | The method audit below covers all 4 current methods. `cpython_compile_boolean_expression_exact_subset` and `boolean-expression-short-circuit-identity` in the differential harness cover `and` / `or` short-circuit operand identity, exact `__bool__` call counts in mixed expressions, and exception propagation from truthiness. |
| `Lib/test/test_compile.py` | `TestSourcePositions` | 33 | `ported_public` | The method audit below classifies all 33 current methods. Parser and public-AST source-position evidence covers many related statement and expression spans, `cpython_compile_source_positions_multiline_assert_rewrite_subset` ports the public-AST compile stability method for a rewritten multiline assert, `cpython_compile_source_positions_code_positions_first_pass_subset` ports the public simple-assignment `co_positions()` AST-offset invariant, `cpython_compile_source_positions_lambda_return_position_subset` ports the public lambda-body `co_positions()` bounds from `test_lambda_return_position`, `cpython_compile_source_positions_weird_attribute_position_regressions_subset` ports the public non-None ordered-position invariant for unusual multiline attribute chains, `cpython_compile_source_positions_multistatement_code_lines_subset` extends first-pass runtime code-object line coverage across multiple statement-leading source lines, and `cpython_compile_specifics_lineno_after_no_code_first_pass_subset` starts function `__code__.co_firstlineno` / `co_lines()` / `co_positions()` coverage for no-code function bodies using source-token function-definition lines. The remaining methods are CPython opcode/debug-range column-model checks and are classified as `blocked_by_cpython_internal`. |
| `Lib/test/test_compile.py` | `TestStaticAttributes` | 4 | `ported` | The method audit below covers all 4 current methods. `cpython_compile_static_attributes_exact_subset` covers tuple-valued class `__static_attributes__`, sorted/deduplicated `self.<attr>` Store targets, nested function collection into the nearest class, nested class isolation, and subclass-specific tuples. |
| `Lib/test/test_compile.py` | `TestExpressionStackSize` | 17 | `ported` | The method audit below covers all 17 current methods. `cpython_compile_expression_stack_size_shapes_subset` ports them as compile-shape checks for long `and` / `or` / mixed boolean chains, chained comparisons, conditional expressions, binary expressions, list/tuple/set/dict displays, function and method positional/keyword calls, repeated function-body boolean expressions, 3050-target unpack assignment, and 3050-argument annotated signatures. MiniPython does not expose CPython `co_stacksize`, so this checks the equivalent register-compiler stability surface. |
| `Lib/test/test_compile.py` | `TestStackSizeStability` | 27 | `ported` | The method audit below covers all 27 current methods. `cpython_compile_stack_size_stability_control_flow_subset` ports them as repeated sync/async function compile-shape checks; MiniPython does not expose CPython `co_stacksize`, so the equivalent evidence is clean compilation of the repeated control-flow snippets plus existing output/differential control-flow tests. |
| `Lib/test/test_compile.py` | `TestInstructionSequence` | 3 | `blocked_by_cpython_internal` | The method audit below classifies all 3 current methods as CPython `_testinternalcapi` instruction-sequence object and opcode metadata coverage. |
| `Lib/test/test_builtin.py` | module-level `test_*` functions | 0 | `source_data` | The current local CPython source has no module-level `test_*` functions in this file; executable tests live under the unittest classes below. |
| `Lib/test/test_builtin.py` | `BuiltinTest` | 96 | `partial` | Current evidence covers a broad public builtin subset, including `BuiltinTest::test_import` ordinary builtin import behavior and error classification, `abs()`, direct `test_all` / `test_any` semantics plus the public dynamic-name-lookup behavior of `test_all_any_tuple_list_set_optimization`, `ascii()`, `callable()` class-level and inherited `__call__` lookup with instance-level `__call__` ignored, attribute helpers including `sys.stdout` lookup and `hasattr()` exception propagation, `cmp` absence from `builtins`, direct `None.__ne__` / inherited `object.__eq__` identity behavior, `chr()` / `ord()` including Unicode scalar boundaries and large out-of-range `chr()` errors, `compile()` including public AST/code-object, `ast.PyCF_OPTIMIZED_AST`, non-coroutine and coroutine-producing `ast.PyCF_ALLOW_TOP_LEVEL_AWAIT`, nested top-level-await invalid cases, and async-generator flag-preservation behavior, `dir()` / `vars()`, `divmod()`, `eval()` / `exec()` including dict and general-mapping globals/locals slices, exec compile-time SyntaxWarning module filtering, builtins-mapping `__build_class__` behavior including empty dict-subclass builtins, read-only globals writeback, and redirected-stdout NameError behavior, `filter()` including internal-payload iterator pickle round trips, `hash()`, `id()`, integer negation around `sys.maxsize`, `iter()` / `next()` including bad-iterable exception identity, `isinstance()` / `issubclass()`, `len()`, `map()` including `strict`, bad-iterable exception identity, internal-payload iterator pickle round trips, and strict-failure pickle preservation, min/max/sum aggregate behavior including `key=`, `default=`, `sum(start=...)`, and aggregate error paths, numeric base formatting helpers, `pow()`, `repr()`, `round()`, `type()`, `zip()` including `strict`, bad-iterable exception identity, internal-payload iterator pickle round trips, and strict-failure pickle preservation, `format()`, bytearray `translate()` and `extend()` public error propagation, bytearray `join()` custom-iterator and re-entrant resize `BufferError` behavior, singleton type construction through `test_construct_singletons`, singleton attribute access through `test_singleton_attribute_access`, `NotImplemented` boolean-context rejection, and selected bytearray regression cases. CPython-only `test_sum_accuracy` compensated-floating behavior is treated as implementation-specific. Full method-level parity remains open across broader import/open/input/file-system behavior, process/environment interaction, remaining pickle/deallocation details, broader warning matrices, CPython code-object optimization internals, and remaining error-message surfaces. |
| `Lib/test/test_builtin.py` | `TestBreakpoint` | 12 | `partial` | `cpython_builtin_breakpoint_custom_hook_subset`, `cpython_builtin_breakpoint_default_stub_subset`, `cpython_builtin_breakpoint_passthru_error_subset`, and the differential `builtin-breakpoint-custom-hook` / `builtin-breakpoint-passthru-error` cases cover the portable sandbox subset: `breakpoint` visibility, mutable `sys.breakpointhook`, custom hook dispatch, positional/keyword passthrough, hook return values, sandbox no-op default hook behavior, custom-hook TypeError propagation, reset identity, and lost-hook `RuntimeError`. CPython's default pdb-backed hook, `PYTHONBREAKPOINT`, warning paths, and debugger import behavior remain blocked by runtime/host integration. |
| `Lib/test/test_builtin.py` | `PtyTests` | 7 | `blocked_by_runtime` | The method audit below classifies all 7 current methods as host-IO integration coverage. They require interactive TTY behavior, `pty`, `signal`, file descriptors, stdin/stdout encodings, and child-process orchestration. |
| `Lib/test/test_builtin.py` | `TestSorted` | 4 | `ported` | `cpython_builtin_sorted_exact_subset` ports all current methods in the audit below, covering deterministic basic ordering without mutating the source list, `key=`, `reverse=`, keyword/positional argument rejection, list/tuple/str/set/frozenset/dict-key input types, and the legacy bad-decorator positional-argument rejection. |
| `Lib/test/test_builtin.py` | `ShutdownTest` | 1 | `blocked_by_cpython_internal` | The method audit below classifies the current method as CPython interpreter shutdown and module/builtins lifetime behavior through a child CPython process. This is not a MiniPython language-surface requirement. |
| `Lib/test/test_builtin.py` | `ImmortalTests` | 3 | `blocked_by_cpython_internal` | The method audit below classifies all 3 current methods as CPython immortal-object refcount implementation details with `sys.getrefcount()`. MiniPython should not copy that internal memory-management contract. |
| `Lib/test/test_builtin.py` | `TestType` | 10 | `partial` | The method audit below covers all 10 current methods. Current evidence covers dynamic `type()` construction including public `int` subclass layout, type name/qualname/doc/first-line metadata slices, generic alias/type-parameter metadata, the public `test_bad_args` matrix including extra positional arguments and mappingproxy namespaces, keyword rejection through `cpython_type_nokwargs_subset`, class `__type_params__` assignment/delete behavior through `cpython_type_typeparams_subset`, the public `test_bad_slots` invalid-slot error-class matrix through `cpython_type_bad_slots_subset`, and ordered dynamic-class namespace preservation through `cpython_type_namespace_order_subset`. Full method-level parity remains open for surrogate-code-point `UnicodeEncodeError` branches. |
| `Lib/test/test_complex.py` | `ComplexTest` | 37 | `ported` | The method audit below now tracks all 37 current CPython complex-number methods. Current evidence covers complex construction, exact complex object identity, numeric protocols, constructor deprecation warnings, subclass behavior, formatting, arithmetic, comparison, hashing, truthiness, representation, signed-zero/NaN/inf handling, underscore parsing, and CPython differential parity for non-finite division/multiplication and complex power edge rows through the `cpython_complex_*` subset and diff tests. |
| `Lib/test/test_float.py` | module-level `test_*` functions | 0 | `source_data` | The current local CPython source has no module-level `test_*` functions in this file; executable tests live under the unittest classes below. |
| `Lib/test/test_float.py` | `GeneralFloatCases` | 22 | `partial` | The method audit below classifies all 22 current methods. Current evidence covers core `float()` construction, Unicode and bytes-like input parsing, underscore validation, numeric conversion protocols, `float.from_number()`, `is_integer()` / `as_integer_ratio()`, NaN containment identity behavior, floor/ceil/modulo, the active IEEE power matrix, hashing, subclass keyword construction, and float/int comparison boundaries. The row remains partial because locale-sensitive comma behavior is not fully proven. |
| `Lib/test/test_float.py` | `FormatFunctionsTestCase` | 1 | `ported` | `cpython_float_getformat_subset` covers the public `float.__getformat__()` method, argument errors, descriptor access, instance access, and subclass inheritance. |
| `Lib/test/test_float.py` | `IEEEFormatTestCase` | 3 | `blocked_by_runtime` | These methods require `struct` binary float packing/unpacking and `_testcapi` float constants; MiniPython does not yet expose that standard-library/runtime surface. |
| `Lib/test/test_float.py` | `FormatTestCase` | 4 | `ported` | All 4 current methods have method-level Rust evidence. Current evidence covers the public `format()` float behavior in `test_format`, explicit-precision empty presentation, issue5864, issue35560 zero-width formatting, precision-side fractional grouping, and the complete local CPython `mathdata/formatfloat_testcases.txt` dataset through old-style `%` and `format()` checks. |
| `Lib/test/test_float.py` | `ReprTestCase` | 2 | `ported` | Both current methods have method-level Rust evidence. Current evidence covers CPython short repr behavior and the complete local CPython `mathdata/floating_points.txt` dataset for `repr(float)` round trips. |
| `Lib/test/test_float.py` | `RoundTestCase` | 10 | `ported` | All 10 current methods have method-level Rust evidence. Current evidence covers public round behavior for infinities, NaNs, `ndigits` validation, complete extreme positive and negative `ndigits` grids, overflow rows, previous round-half-even bugs, `None` handling, direct `float.__round__()`, format-special rows, and the CPython `round()` / fixed-format consistency sweep with deterministic random-like replacement rows. |
| `Lib/test/test_float.py` | `InfNanTest` | 6 | `ported` | `cpython_float_inf_nan_string_subset` covers public infinity and NaN string parsing, malformed spellings, `repr()` / `str()` spellings, and sign handling. |
| `Lib/test/test_float.py` | `HexFloatTestCase` | 6 | `ported` | All 6 current methods have method-level Rust evidence. Current evidence covers public `float.hex()` / `float.fromhex()` endpoints, invalid input categories, whitespace handling, the complete deterministic `test_from_hex` input matrix, bpo-44954 behavior, a 10,000-row deterministic `test_roundtrip` sweep, and subclass dispatch. |
| `Lib/test/test_types.py` | module-level `test_*` functions | 0 | `source_data` | The current local CPython source has no module-level `test_*` functions in this file; executable tests live under the unittest classes below. |
| `Lib/test/test_types.py` | `TypesTests` | 30 | `partial` | The method audit below now classifies all 30 current CPython methods. Current evidence covers many portable core runtime behaviors through existing builtin, numeric, formatting, singleton, traceback, frame, and type-object tests; `cpython_types_names_public_surface_subset` covers the public `types.__all__` name set, visible aliases, and `_types` accelerator alias identity, `cpython_types_float_constructor_edges_subset` covers the `float()` constructor edge strings from `test_float_constructor`, `cpython_types_float_to_string_subset` covers the full exponent string matrix and direct numeric `__format__` surface from `test_float_to_string`, `cpython_types_normal_integers_subset` covers the public arithmetic, comparison, `sys.maxsize` boundary, and negative-shift rows from `test_normal_integers`, `cpython_types_int_format_subset` covers the complete non-locale public integer `__format__` matrix from `test_int__format__`, `cpython_types_float_format_subset` covers the complete non-locale public float `__format__` matrix from `test_float__format__`, `cpython_types_format_spec_errors_subset` covers the shared mini-language error parser rows from `test_format_spec_errors`, `cpython_types_slot_and_method_wrapper_types_subset` covers public wrapper-descriptor and method-wrapper identities plus direct `object` / `int` slot calls, `cpython_types_method_descriptor_types_subset` covers the public method/classmethod descriptor alias matrix from `test_method_descriptor_types`, `cpython_types_singleton_type_aliases_subset` covers public singleton type aliases, `cpython_types_module_type_subset` covers `types.ModuleType`, `cpython_types_code_traceback_type_aliases_subset` covers the public `CodeType` and `TracebackType` aliases backed by MiniPython runtime objects, `cpython_types_frame_type_alias_subset` covers the public `FrameType` alias backed by real `sys._getframe()` frame objects, `cpython_types_frame_locals_proxy_type_subset` covers the public `FrameLocalsProxyType` metadata and `inspect.currentframe().f_locals` proxy behavior, and `cpython_types_runtime_type_aliases_subset` covers public runtime aliases for functions, lambdas, generators, coroutines, async generators, builtin functions/methods, and bound methods. The row remains `partial` because locale behavior, descriptor `__get__` signature introspection, CPython C-extension descriptor crash coverage, and CPython-internal surfaces are still partial or explicitly blocked in the audit. |
| `Lib/test/test_types.py` | `UnionTests` | 29 | `ported_public` | The method audit below now tracks all 29 current CPython methods. Current evidence covers every portable public PEP 604 union method through `cpython_types_union_public_operator_and_classinfo_subset`, `cpython_types_union_forward_ref_subset`, `cpython_types_union_forward_get_type_hints_subset`, `cpython_types_union_genericalias_subclass_bad_eq_subset`, `cpython_types_union_typevar_parameter_subset`, `cpython_types_union_parameter_substitution_subset`, `cpython_types_union_copy_pickle_subset`, `cpython_types_union_bad_classinfo_checks_subset`, `cpython_types_union_unhashable_metaclass_subset`, `cpython_types_union_dynamic_hashability_subset`, `cpython_types_union_newtype_subset`, `cpython_types_union_io_subset`, `cpython_types_union_typed_dict_subset`, `cpython_types_union_protocol_subset`, `cpython_types_union_special_form_subset`, `cpython_types_union_literal_subset`, `cpython_types_union_bad_module_guard_subset`, `cpython_types_generic_alias_union_type_subset`, `cpython_typing_get_origin_args_subset`, namedtuple `GenericAlias` behavior, `types.UnionType[...]` subscription, and type-parameter tests. The only remaining method is the CPython reference-cycle regression classified as implementation-internal below. |
| `Lib/test/test_types.py` | `MappingProxyTests` | 15 | `ported` | All 15 current CPython methods now have method-level Rust evidence in the audit below, covering constructor inputs, method surface, lookup/get/missing behavior, custom mapping forwarding, ChainMap sources, membership, live views, length, iteration, reversed iteration, copy independence, union operators, hash forwarding, and rich comparisons. |
| `Lib/test/test_types.py` | `ClassCreationTests` | 25 | `ported` | All 25 current CPython methods now have method-level Rust evidence in the audit below, covering public `types.new_class()`, `types.prepare_class()`, `types.resolve_bases()`, `__mro_entries__`, `__orig_bases__`, metaclass derivation and override behavior, one-argument `type` restrictions, metaclass-constructor error propagation, dynamic slot update behavior for dict subclasses, and tuple-subclass bases. |
| `Lib/test/test_types.py` | `SimpleNamespaceTests` | 18 | `ported` | Current evidence covers construction from mappings and pair iterables, insertion order, live and surviving `__dict__` mappings, attribute get/set/delete, repr/equality, nested and recursive namespace graphs, unsupported mapping and rich-ordering operations, subclass construction, MiniPython internal-payload pickle round trips, `copy.replace()`, public `SimpleNamespace.__new__` / `.__replace__`, invalid subtype replacement rejection, and fake-namespace comparison safety through `cpython_types_simple_namespace_basic_subset`, `cpython_types_simple_namespace_recursive_and_replace_subset`, `cpython_types_simple_namespace_new_and_invalid_replace_subset`, and `cpython_types_simple_namespace_remaining_public_subset`. |
| `Lib/test/test_types.py` | `CoroutineTests` | 11 | `ported` | The method audit below classifies all 11 current methods as ported. `cpython_types_coroutine_public_subset`, `cpython_types_coroutine_async_def_subset`, `cpython_types_coroutine_generator_wrapper_subset`, `cpython_types_coroutine_generator_frame_subset`, `cpython_types_coroutine_generator_yieldfrom_subset`, `cpython_types_coroutine_duck_generator_wrapper_subset`, `cpython_types_coroutine_duck_generator_await_subset`, and `cpython_types_coroutine_duck_generator_proxy_subset` now cover public `types.coroutine()` behavior for wrong arguments, non-generator return values, native coroutine pass-through and `cr_code`, coroutine-like object pass-through, iterable-coroutine pass-through, generator-function decorator identity and flags, `_GeneratorWrapper` type/ABC relationships, wrapper repr/dir, native-generator forwarding, native generator `__name__` / `__qualname__` / `gi_code` / `gi_frame` / `gi_yieldfrom` and wrapper `cr_code` / `cr_frame` / `cr_await` stable identity, direct duck-generator forwarding, duck-generator await execution, duck-generator attribute pass-through and aliasing, `unittest.mock.MagicMock` proxy verification for forwarded `close()` / `send()` / `next()` / `throw()` calls, catchable wrapper argument TypeErrors without forwarding side effects, propagated throw `side_effect` exceptions, double-wrap avoidance, and `weakref.ref(wrapper)` alive-reference identity. |
| `Lib/test/test_types.py` | `FunctionTests` | 2 | `ported` | Both current CPython methods now have method-level Rust evidence in the audit below. `cpython_types_function_type_subset` covers public `types.FunctionType` construction over MiniPython code objects, globals dictionaries, explicit and code-derived names, positional defaults, keyword-only defaults, callable execution, public function type identity, and wrong-default TypeErrors. |
| `Lib/test/test_types.py` | `SubinterpreterTests` | 1 | `blocked_by_cpython_internal` | The current method validates CPython static-type slot behavior across subinterpreters. MiniPython should not copy CPython's subinterpreter implementation contract. |
| `Lib/test/test_collections.py` | module-level `test_*` functions | 0 | `source_data` | The current local CPython source has no module-level `test_*` functions in this file; executable tests live under the unittest classes below. |
| `Lib/test/test_collections.py` | `TestUserObjects` | 6 | `ported` | All 6 current methods now have method-level Rust evidence: `cpython_collections_userdict_public_methods_subset` ports `test_dict_protocol` and `test_dict_copy`; `cpython_collections_userlist_public_methods_subset` ports `test_list_protocol` and `test_list_copy`; `cpython_collections_userstring_protocol_and_userdict_missing_subset` ports `test_str_protocol` and `test_dict_missing`, including `UserString` protocol visibility and `UserDict` subclass `__missing__` dispatch while preserving `get()` ignoring `__missing__`. |
| `Lib/test/test_collections.py` | `TestChainMap` | 10 | `ported` | All 10 current CPython methods now have method-level Rust evidence: `cpython_collections_chainmap_public_methods_subset`, `cpython_collections_chainmap_copy_pickle_eval_identity_subset`, `cpython_collections_chainmap_missing_and_first_map_mutation_subset`, `cpython_collections_chainmap_iter_does_not_call_getitem_subset`, `cpython_collections_chainmap_new_child_custom_mapping_subset`, `cpython_collections_chainmap_order_preservation_subset`, `cpython_collections_chainmap_union_operators_subset`, and the mappingproxy ChainMap slice cover constructor, bool, first-map mutation, `maps`, `parents`, `new_child()`, custom child mapping protocol lookup, subclass `__missing__`, `pop()`, `popitem()`, `clear()`, ordering, dict coercion, iteration, views, containment, lookup, `get()`, shallow/deep copy identity expectations, pickle/eval(repr(...)) round trips, OrderedDict order-preservation matrices, ChainMap/mapping union, in-place union, iterable-pair behavior, and CPython subclass result-type rules including `super().__ror__`. |
| `Lib/test/test_collections.py` | `TestNamedTuple` | 23 | `ported_public` | All 20 current public methods have method-level Rust evidence. `cpython_collections_namedtuple_factory_instance_subset` ports `test_factory`, `test_instance`, `test_tupleness`, and `test_odd_sizes`, covering factory construction, generated type metadata including inherited `tuple.__getitem__`, invalid-name errors, tuple-like instances including tuple `isinstance`, hashing parity with tuple values, slicing, `count()`, `index()`, `_make()`, `_replace()`, `_asdict()`, empty and one-field tuples, comma/sequence field inputs, constructor positional/keyword binding, weakref exclusion, arity/name errors, and `__match_args__`. `cpython_collections_namedtuple_defaults_rename_readonly_subset` adds defaults, `_field_defaults`, generated `__new__.__defaults__`, `rename=True`, `module=`, class `__doc__` assignment, and readonly field/item rejection coverage. The three non-public descriptor-internal methods are classified as `blocked_by_cpython_internal` in the method audit. |
| `Lib/test/test_collections.py` | `ABCTestCase` | 0 | `source_data` | Shared assertion helpers for the ABC test classes; it has no direct `test_*` methods. |
| `Lib/test/test_collections.py` | `TestOneTrickPonyABCs` | 16 | `ported` | All 16 current CPython methods now have method-level Rust evidence for public `collections.abc` runtime behavior across `Awaitable`, `Coroutine`, `Hashable`, `AsyncIterable`, `AsyncIterator`, `Iterable`, `Reversible`, `Collection`, `Iterator`, `Generator`, `AsyncGenerator`, `Sized`, `Container`, and `Callable`; exact public abstract-method instantiation errors from `ABCTestCase.validate_abstract_methods`; public structural helper behavior from `ABCTestCase.validate_isinstance`; public direct-subclassing behavior for the supported one-trick pony ABCs; public `ABC.register()` behavior for those ABCs; public coroutine-object type parity for the supported `AsyncGenerator` ABC async mixins including traceback argument handling; and the public `types.coroutine()` iterable-coroutine distinction. |
| `Lib/test/test_collections.py` | `WithSet` | 0 | `source_data` | Helper mutable-set class used by `TestCollectionABCs`; it has no direct `test_*` methods. |
| `Lib/test/test_collections.py` | `TestCollectionABCs` | 25 | `ported_public` | Current evidence covers all 24 public methods across `Set`, `MutableSet`, `Mapping`, `MutableMapping`, `MappingView`, `Sequence`, `ByteString`, `Buffer`, and `MutableSequence` ABC behavior. `cpython_collections_abc_composite_abstract_methods_subset` ports the composite-ABC abstract-method rejection matrix for supported public ABCs; `cpython_collections_abc_set_from_iterable_operator_subset` ports `test_Set_from_iterable`; `cpython_collections_abc_set_real_set_interoperability_subset` ports the public operator/comparison/equality matrix from `test_Set_interoperability_with_real_sets`; `cpython_collections_abc_set_hash_matches_frozenset_subset` ports `test_Set_hash_matches_frozenset`, including the CPython `sys.maxsize` range stress sample; `cpython_collections_abc_set_noncomparable_comparison_subset` ports `test_issue16373` for non-comparable `Set` subclass comparison fallback; `cpython_collections_abc_issue26915_identity_first_object_subset` ports the identity-first container regression for both `support.NEVER_EQ`-style objects and distinct `float('nan')` objects; `cpython_collections_abc_bytestring_deprecation_warnings_subset` ports the public `ByteString` deprecation-warning surface for import, attribute access, `isinstance()`, class-statement subclass creation, and dynamic `type(..., (ByteString,), ...)` subclass creation used by CPython's abstract-method helper; `cpython_collections_abc_mutable_sequence_subset` ports `test_MutableSequence` including `deque` and `array.array` ABC registrations; `cpython_collections_abc_userdict_view_snapshot_subset` ports `test_MutableMapping_subclass` for `UserDict` view ABC relationships and eager set-operation snapshots. `test_illegal_patma_flags` is classified as CPython-internal ABC flag coverage in the method audit. |
| `Lib/test/test_collections.py` | `CounterSubclassWithSetItem` | 0 | `source_data` | Helper class for `TestCounter`; it has no direct `test_*` methods. |
| `Lib/test/test_collections.py` | `CounterSubclassWithGet` | 0 | `source_data` | Helper class for `TestCounter`; it has no direct `test_*` methods. |
| `Lib/test/test_collections.py` | `TestCounter` | 23 | `ported` | All 23 current CPython methods now have method-level Rust evidence. `cpython_collections_counter_basics_subset` ports `test_basics`, including construction, missing-key zero lookup, dict equality, views, iteration, repr, `most_common()`, `elements()`, in-place item mutation, `pop`, `popitem`, `clear`, `fromkeys` rejection, unhashability, `update()`, additive `__init__()`, and `setdefault()`; the remaining methods are covered by the dedicated Counter tests in the method audit below, including the deterministic 1000-pair multiset and in-place matrices plus the full symmetric-difference and zero/one-count set-equivalence matrices. |
| `Lib/test/test_type_comments.py` | `TypeCommentTests` | 17 | `ported` | All 17 current CPython methods now have method-level Rust evidence in the audit below, covering statement, function, argument, and type-ignore public AST metadata; `func_type` mode; misplaced comment rejection; modern syntax smoke checks; duplicate function type comments; non-ASCII comment text; and default `async` / `await` assignment rejection. |
| `Lib/test/test_type_params.py` | `TypeParamsInvalidTest` | 13 | `ported` | All 13 current CPython methods now have method-level Rust evidence in the audit below, covering duplicate names, non-colliding ordinary bindings, disallowed type-scope expressions, and the explicit-object generic-class MRO rejection. |
| `Lib/test/test_type_params.py` | `TypeParamsNonlocalTest` | 4 | `ported` | All 4 current CPython methods have method-level Rust evidence in the audit below through `cpython_type_params_nonlocal_scope_subset`. |
| `Lib/test/test_type_params.py` | `TypeParamsAccessTest` | 32 | `ported` | `cpython_type_params_access_core_subset` now gives method-level evidence for all current access methods, including exact type-parameter object identity in annotations and alias lambdas, default/decorator out-of-scope errors, nested generic closures, no-leak checks, class-scope annotation lookup, implicit `Generic` bases plus `types.get_original_bases()`, generic metaclass access through `metaclass=MyMeta[A, B]` and exact `type(cls) is meta`, class-local alias dereference, class/generic-alias lambda and comprehension capture, class-base lambdas via `typing.get_args()`, nested-class base comprehension scope splits, generic-method generator-expression annotations, type-parameter bound/value comprehension annotation scopes, nonlocal shadowing, previous-type-parameter bounds, generic-method `super()`, and builtin `type`/`object` empty `__type_params__`. |
| `Lib/test/test_type_params.py` | `GlobalGenericClass` | 0 | `source_data` | Helper class used by runtime type-parameter tests; it has no direct `test_*` methods. |
| `Lib/test/test_type_params.py` | `TypeParamsLazyEvaluationTest` | 3 | `ported` | `cpython_type_params_lazy_evaluation_qualname_subset` and `cpython_type_params_lazy_evaluation_bounds_subset` cover all current methods, including generic class/function `__qualname__`, recursive class bound/constraints lookup, deferred missing-name errors with later re-evaluation, tuple-constraint behavior, and CPython-style `typing.NoDefault` for missing defaults. |
| `Lib/test/test_type_params.py` | `TypeParamsClassScopeTest` | 10 | `ported` | `cpython_type_params_class_scope_first_pass_subset` and `cpython_type_params_class_scope_lazy_subset` cover all current public class-scope methods: aliases and generic method bounds can read prior class locals, names without a class binding use enclosing nonlocals, future class bindings force global lookup, explicit `global` and `nonlocal` class-body assignments are honored by lazy alias reads, later class-attribute mutation is visible to lazy bounds/aliases, and nested free-variable lookup keeps class bases and class-body names distinct. |
| `Lib/test/test_type_params.py` | `DynamicClassTest` | 2 | `blocked_by_runtime` | Requires fuller dynamic generic class construction through `types.new_class()` callbacks than MiniPython's current first-pass class-creation helper surface exposes. |
| `Lib/test/test_type_params.py` | `TypeParamsManglingTest` | 7 | `ported` | `cpython_type_params_mangling_subset` ports all current methods, covering public type-parameter names, class-private references inside class bodies, methods, aliases, bases, nested lambdas/comprehensions, and non-leaky mangling across module/function/class boundaries. |
| `Lib/test/test_type_params.py` | `TypeParamsComplexCallsTest` | 3 | `ported` | `cpython_type_params_complex_calls_subset` ports all current methods, covering generic functions with both positional defaults and keyword-only defaults, annotations preserving exact type-parameter identity, class-header `**kwargs` propagation through `__init_subclass__`, implicit `Generic` bases after positional and unpacked bases, and starargs base forms including empty unpacking. |
| `Lib/test/test_type_params.py` | `TypeParamsTraditionalTypeVarsTest` | 3 | `ported` | `cpython_type_params_traditional_typevars_subset` ports all current methods, covering explicit `Generic[T]` rejection in PEP 695 class headers, traditional `typing.TypeVar` rejection when undeclared in generic bases, and ordinary annotations that combine PEP 695 type parameters with traditional TypeVars. |
| `Lib/test/test_type_params.py` | `TypeParamsTypeVarTest` | 3 | `ported` | `cpython_type_params_typevar_runtime_subset` ports all current methods, covering PEP 695 type parameters as `typing.TypeVar` objects with lazy bounds, tuple constraints, variance metadata, and generator/coroutine nested-scope capture. |
| `Lib/test/test_type_params.py` | `TypeParamsTypeVarTupleTest` | 2 | `ported` | `cpython_invalid_type_param_subset` covers invalid TypeVarTuple bounds/constraints, and `cpython_type_params_typevartuple_paramspec_runtime_subset` covers runtime `typing.TypeVarTuple` compatibility and traditional constructor metadata. |
| `Lib/test/test_type_params.py` | `TypeParamsTypeVarParamSpecTest` | 2 | `ported` | `cpython_invalid_type_param_subset` covers invalid ParamSpec bounds/constraints, and `cpython_type_params_typevartuple_paramspec_runtime_subset` covers runtime `typing.ParamSpec` compatibility, variance metadata, and traditional constructor metadata. |
| `Lib/test/test_type_params.py` | `TypeParamsTypeParamsDunder` | 6 | `ported` | All 6 current CPython methods have method-level Rust evidence in the audit below through `cpython_type_params_dunder_subset`. |
| `Lib/test/test_type_params.py` | `Class1` | 0 | `source_data` | Helper class for pickling tests; it has no direct `test_*` methods. |
| `Lib/test/test_type_params.py` | `Class2` | 0 | `source_data` | Helper class for pickling tests; it has no direct `test_*` methods. |
| `Lib/test/test_type_params.py` | `Class3` | 0 | `source_data` | Helper class for pickling tests; it has no direct `test_*` methods. |
| `Lib/test/test_type_params.py` | `Class4` | 0 | `source_data` | Helper class for pickling tests; it has no direct `test_*` methods. |
| `Lib/test/test_type_params.py` | `TypeParamsPickleTest` | 2 | `blocked_by_runtime` | Requires pickle compatibility for functions and classes with type parameters. MiniPython has not committed to CPython pickle byte compatibility. |
| `Lib/test/test_type_params.py` | `TypeParamsWeakRefTest` | 1 | `ported` | `cpython_type_params_weakrefs_subset` ports the current CPython method by proving `weakref.ref()` can be constructed for traditional `TypeVar`, `ParamSpec`, `ParamSpec.args`, `ParamSpec.kwargs`, `TypeVarTuple`, old-style and PEP 695 generic classes, their generic aliases, their instances, and `Generic[T]`. |
| `Lib/test/test_type_params.py` | `TypeParamsRuntimeTest` | 2 | `ported` | `cpython_type_params_runtime_name_error_subset` ports `test_name_error`, proving missing names in nested generic class bases/bounds surface as catchable runtime `NameError`; `cpython_type_params_runtime_class_namespace_subset` ports `test_broken_class_namespace`, proving metaclass `__prepare__` custom class namespace lookup can surface dict-subclass `__missing__` exceptions during nested generic class base evaluation. |
| `Lib/test/test_type_params.py` | `DefaultsTest` | 9 | `ported` | `cpython_type_param_defaults_subset` ports defaults on functions, classes, type aliases, and exact starred TypeVarTuple defaults by preserving `Unpack[...]` default values and `GenericAlias.__iter__` parity; `cpython_type_param_starred_invalid_subset` ports `test_starred_invalid`; `cpython_type_param_defaults_lazy_and_symtable_subset` ports lazy default evaluation/caching plus both symtable-key regressions; `cpython_type_param_nondefault_after_default_subset` ports `test_nondefault_after_default`. |
| `Lib/test/test_type_params.py` | `TestEvaluateFunctions` | 3 | `ported` | `cpython_type_params_evaluate_functions_subset` ports all current methods, covering `evaluate_value`, `evaluate_bound`, `evaluate_default`, `evaluate_constraints`, `annotationlib.call_evaluate_function()` with VALUE/FORWARDREF/STRING formats, traditional `typing.TypeAliasType` and type-parameter constructors, and the `_typing._ConstEvaluator` construction/immutability regression. |
| `Lib/test/test_memoryview.py` | direct test method definitions | 42 | `partial` | `cpython_memoryview_minimal_runtime_subset`, `cpython_memoryview_getbuf_fail_subset`, `cpython_memoryview_basic_methods_and_release_subset`, `cpython_memoryview_writable_setitem_subset`, `cpython_memoryview_slice_reference_subset`, `cpython_memoryview_public_buffer_attributes_subset`, `cpython_memoryview_array_b_buffer_subset`, `cpython_memoryview_array_signed_byte_buffer_subset`, `cpython_memoryview_array_non_byte_public_read_subset`, `cpython_memoryview_array_non_byte_writeback_subset`, `cpython_memoryview_cast_one_byte_format_subset`, `cpython_memoryview_getitem_index_count_compare_subset`, `cpython_memoryview_hex_separator_subset`, `cpython_memoryview_hex_reentrant_release_subset`, `cpython_memoryview_copy_rejection_subset`, `cpython_memoryview_pickle_rejection_subset`, `cpython_memoryview_hash_release_cache_subset`, `cpython_memoryview_release_during_index_subset`, `cpython_memoryview_bytesio_readinto_subset`, and `cpython_memoryview_weakref_live_subset` port the first one-dimensional bytes-like public behavior slices, including constructor argument binding across bytes/bytearray/memoryview exporters, non-buffer constructor rejection, CPython-style public constructor `TypeError` diagnostics, iteration, equality, read-only hashing and cached hash availability after release, writable/released hash errors, supported attributes and methods, method-level `tobytes()` / `tolist()` evidence, `toreadonly()`, `release()`, context-manager lifecycle, released-state errors, released `str()` / `repr()`, same-object `with ... as` identity, reversed iteration, bytearray-backed writable item assignment, same-size slice assignment, overlapping self-copy, read-only assignment errors, deletion errors, bounds checks, tuple-key scalar get/set, tuple-key `NotImplementedError` for unsupported subviews and multidimensional slicing, mixed tuple-key `TypeError`, no-resize assignment checks, shared bytearray object storage, true bytearray-backed subview sharing, slice-of-slice sharing, negative-stride subview writeback, readonly preservation through slicing, exporter identity through `obj`, positive/negative/empty-slice `strides`, one-dimensional contiguity attributes, unsigned and signed array-backed one-byte memoryviews, one-dimensional non-byte numeric array-backed read attributes/getitem/tolist/tobytes/slicing/byte-cast behavior plus same-format scalar and slice writeback for `h`, `H`, `i`, `I`, `f`, and `d`, one-byte `B` / `b` / `c` casts, one-dimensional cast `shape`, `c`-format bytes elements and writable assignment, integer getitem, `index()` start/stop behavior, `count()` over logical view contents, equality with buffer objects, ordered-comparison `TypeError`, logical-byte `hex()` separator grouping for reversed non-contiguous views, released-view `hex()` errors, re-entrant separator conversion resize safety, public `copy.copy()` / `pickle.dumps()` rejection, one-dimensional release-during-`__index__` safety for scalar getitem, slice getitem, item/slice assignment, RHS byte conversion, bound get/set methods, in-memory `io.BytesIO.readinto()` writable-target fills and read-only target rejection, plus live `weakref.ref()` construction/call/`ReferenceType` behavior for memoryviews. Full parity remains open for multidimensional casts, broader slicing/refcount matrices, GC weakref clearing/callback invocation, ctypes, and full buffer protocol behavior. |
| `Lib/test/test_bytes.py` | `BaseBytesTest` | 71 | `partial` | Current evidence includes `cpython_bytes_basics_and_ord_subset`, `cpython_bytes_literal_subset`, `cpython_string_bytes_codec_subset`, `cpython_bytes_hex_fromhex_subset`, `cpython_bytes_hex_descriptor_error_messages_subset`, `cpython_bytes_iterable_constructor_subset`, `cpython_bytes_buffer_constructor_subset`, `cpython_bytes_array_array_buffer_subset`, `cpython_bytes_mutating_list_constructor_subset`, `cpython_bytes_constructor_exception_subset`, `cpython_bytes_dunder_bytes_and_blocking_subset`, `cpython_bytes_bytearray_index_error_and_hash_subset`, `cpython_bytes_length_constructor_boundary_subset`, `cpython_bytes_constructor_overflow_guard_subset`, `cpython_bytes_constructor_concat_repeat_contains_subset`, `cpython_bytes_compare_slice_reversed_subset`, `cpython_bytes_search_methods_subset`, `cpython_bytes_search_bounds_index_subset`, `cpython_bytes_prefix_suffix_methods_subset`, `cpython_bytes_prefix_suffix_typeerror_messages_subset`, `cpython_bytes_split_rsplit_methods_subset`, `cpython_bytes_splitlines_methods_subset`, `cpython_bytes_ascii_case_predicate_methods_subset`, `cpython_bytes_expandtabs_zfill_methods_subset`, `cpython_bytes_strip_methods_subset`, `cpython_bytes_alignment_methods_subset`, `cpython_bytes_method_typeerror_messages_subset`, `cpython_bytes_more_method_typeerror_messages_subset`, `cpython_bytes_core_method_typeerror_messages_subset`, `cpython_bytes_join_translate_maketrans_typeerror_messages_subset`, `cpython_bytes_maketrans_translate_subset`, `cpython_bytes_remove_affix_methods_subset`, `cpython_bytes_join_subset`, `cpython_bytes_replace_partition_methods_subset`, `cpython_bytearray_mutation_methods_subset`, `cpython_bytearray_extended_slice_assignment_subset`, `cpython_bytes_copy_module_subset`, `cpython_bytes_pickle_roundtrip_subset`, and `cpython_bytes_iterator_pickle_roundtrip_subset`, covering exact empty bytes/bytearray construction type and `__class__` identity, one-byte slice `ord()` values, first-pass bytes/bytearray literals, basic constructors, `copy.copy()` / `copy.deepcopy()` type and equality preservation for bytes and bytearray with independent bytearray copy buffers, string encoding constructors, decode/encode slices, `fromhex()` inputs and exact odd-digit/invalid-position diagnostics, `hex()` separator grouping and exact descriptor diagnostics, construction from supported integer iterables, `__getitem__` sequences, live mutating lists, `__index__` elements, bytes-like buffer constructors, first-pass `array.array('B')` buffer inputs, and direct `cpython_bytes_buffer_constructor_diff_subset` oracle evidence, constructor exception propagation from `__index__` / `__iter__`, `bytes()` `__bytes__` dispatch, bytes-subclass result preservation, non-bytes result rejection, `__bytes__` precedence over `__index__`, `__bytes__ = None` fallback blocking, invalid-index TypeError messages for bytes and bytearray, bytearray unhashability, integer-length construction with direct CPython diff evidence for signed-size boundaries, sandbox allocation guards for address-space-sized lengths, mixed bytes/bytearray concatenation result types, repetition and repeat TypeErrors, membership over integer and bytes-like needles, lexicographic comparisons, all CPython byte-order bytes/bytearray comparison-against-`str` rows, reversed iteration, ordinary and extended slicing, `count()`, `find()`, `rfind()`, `index()`, and `rindex()` over bytes-like and integer byte needles with start/stop bounds, exact search/prefix/suffix over-arity TypeError diagnostics, Python-level `__index__` conversion and exception propagation for search and prefix/suffix `start` / `stop` bounds, `startswith()` / `endswith()` over bytes-like and tuple prefixes/suffixes with `None` bounds, exact tuple-candidate TypeError diagnostics, `split()` / `rsplit()` over ASCII whitespace and bytes-like separators with `maxsplit`, `splitlines()` over CR/LF/CRLF with `keepends`, ASCII `lower()` / `upper()` / `capitalize()` / `title()` / `swapcase()` and `is*` predicate methods inherited through `BytesAsStringTest` / `ByteArrayAsStringTest`, `expandtabs()` byte-level tab expansion with `tabsize` keyword behavior, `zfill()` sign-aware zero fill, and builtin type `dir()` visibility inherited through those same classes, `strip()` / `lstrip()` / `rstrip()` over ASCII whitespace and bytes-like strip sets, `center()` / `ljust()` / `rjust()` alignment over default and custom single-byte fills, exact TypeError diagnostics for bytes/bytearray split/partition/strip and alignment fill argument validation, exact ASCII case/predicate, splitlines, expandtabs, zfill, removeprefix, and removesuffix TypeError diagnostics, exact core bytes method arity and integer-conversion TypeError diagnostics, exact join/translate/maketrans TypeError diagnostics for unbound, missing, over-arity, non-iterable, and no-argument calls, `maketrans()` / `translate()` 256-byte table construction, `None` identity translation tables, optional deletion bytes including `delete=`, bytes-like table/delete arguments, class and instance `maketrans()` lookup, receiver-driven translate result types, `removeprefix()` / `removesuffix()` over bytes-like affixes, `join()` receiver-driven result types over iterable bytes-like items, plus `replace()`, `partition()`, and `rpartition()` result-type behavior, bytes-like arguments, replacement count handling, empty-needle replacement, empty separators, bytearray-specific `append()`, `extend()`, `insert()`, `pop()`, `remove()`, `reverse()`, `clear()`, and `copy()` mutation behavior, bytearray extended slice assignment/deletion, integer-iterable RHS conversion, self-slice assignment, special method dispatch, saturated large slice bounds, supported bytes/bytearray pickle value/type round trips and iterator pickle round trips, and representative TypeError/ValueError/IndexError paths. Full parity remains open for the broader bytes/bytearray method matrix, CPython binary pickle-byte compatibility, non-`B` buffer formats and full buffer-protocol matrices, large-allocation/overflow stress, and remaining exact error-message matrices outside the covered bytes method diagnostics. |
| `Lib/test/test_bytes.py` | `BytesTest` | 7 | `partial` | Existing evidence ports the public behavior of `test__bytes__`, `test_getitem_error`, `test_custom`, `test_bytes_blocking`, and `test_repeat_id_preserving`; `cpython_memoryview_bytesio_readinto_subset` covers the public read-only target rejection from `test_buffer_is_readonly` through in-memory `io.BytesIO.readinto()`. Remaining acceptance: the exact `test_buffer_is_readonly` fixture still needs host raw file I/O via `open(fd, "rb", buffering=0)`, and `test_from_format` is CPython C API coverage for `PyBytes_FromFormat()` through `ctypes` / `_testcapi`. |
| `Lib/test/test_bytes.py` | `ByteArrayTest` | 43 | `partial` | Existing evidence ports public slices of `test_getitem_error`, `test_setitem_error`, `test_nohash`, `test_mod_concurrent_mutation`, `test_reverse`, `test_clear`, `test_copy`, `test_regexps`, `test_resize`, `test_take_bytes`, `test_setitem`, `test_delitem`, `test_setslice`, `test_setslice_extend`, `test_extended_set_del_slice`, `test_setslice_trap`, `test_iconcat`, `test_irepeat`, `test_irepeat_1char`, `test_alloc`, `test_init_alloc`, `test_extend`, `test_remove`, `test_pop`, `test_nosort`, `test_append`, `test_insert`, `test_copied`, `test_partition_bytearray_doesnt_share_nullstring`, `test_resize_forbidden`, `test_iterator_pickling2`, `test_iterator_length_hint`, `test_exhausted_iterator`, `test_repeat_after_setslice`, `test_mutating_index`, `test_mutating_index_inbounds`, `test_search_methods_reentrancy_raises_buffererror`, `test_extend_empty_buffer_overflow`, and `test_hex_use_after_free` with gated direct `cpython_bytearray_hex_reentrant_separator_buffererror_diff_subset` evidence. Remaining acceptance: `test_bytearray_api` needs host file I/O and `readinto()` / binary-write policy, `test_take_bytes_optimization`, `test_fifo_overrun`, and `test_del_expand` assert CPython allocator/layout details, `test_obsolete_write_lock` is `_testcapi` C API coverage, and the `_testlimitedcapi` branches inside `test_setitem`, `test_delitem`, `test_mutating_index`, and `test_mutating_index_inbounds` stay outside MiniPython's public runtime contract. |
| `Lib/test/test_bytes.py` | `AssortedBytesTest` | 13 | `ported` | Existing evidence ports `test_bytes_repr`, `test_bytearray_repr`, `test_bytes_str`, `test_bytearray_str`, `test_format`, `test_compare_bytes_to_bytearray`, `test_doc`, `test_from_bytearray`, `test_literal`, `test_split_bytearray`, `test_rsplit_bytearray`, and `test_return_self` public behavior through bytes literal/runtime, repr/str, format, docs, bytes-like split, and bytearray copy tests; `cpython_bytes_warning_compare_subset` with direct `cpython_bytes_warning_compare_diff_subset` evidence ports `test_compare` with `sys.flags.bytes_warning`, `BytesWarning` capture, and `-bb` warning-as-error behavior. |
| `Lib/test/test_bytes.py` | `BytearrayPEP3137Test` | 1 | `ported` | `cpython_bytearray_pep3137_returns_new_copy_subset` ports `test_returns_new_copy`, covering value-equal but distinct mutable bytearray results for no-op string-like methods and one-item joins. |
| `Lib/test/test_bytes.py` | `SubclassTest` | 5 | `ported` | `cpython_bytes_bytearray_subclass_ops_and_join_subset`, `cpython_bytes_bytearray_subclass_fromhex_subset`, `cpython_bytes_bytearray_subclass_copy_subset`, and `cpython_bytes_bytearray_subclass_pickle_subset` port all current methods for supported bytes and bytearray subclasses. CPython binary pickle byte compatibility remains outside this public subset. |
| `Lib/test/test_bytes.py` | `ByteArraySubclassTest` | 1 | `ported` | The inherited `SubclassTest` methods are tracked through the shared `SubclassTest` row; `cpython_bytearray_subclass_init_override_subset` ports direct `test_init_override`, covering bytearray subclass `__init__` override behavior with positional and keyword `source` construction. |
| `Lib/test/test_bytes.py` | `FreeThreadingTest` | 2 | `blocked_by_cpython_internal` | `test_free_threading_bytearray` and `test_free_threading_bytearrayiter` are CPython free-threading stress tests gated on `support.Py_GIL_DISABLED`; they validate C-level thread-safety/refcount races rather than MiniPython language semantics. |
| `Lib/test/test_ast/test_ast.py` | module-level `test_*` functions | 0 | `source_data` | The current local CPython source has no module-level `test_*` functions in this file; executable tests live under the unittest classes below. |
| `Lib/test/test_ast/test_ast.py` | `LazyImportTest` | 1 | `blocked_by_runtime` | The current CPython method is `@support.cpython_only` and calls `ensure_lazy_imports("ast", ...)`, which runs a child CPython process and asserts importing `ast` does not populate selected modules in `sys.modules`. MiniPython tracks lazy-import syntax and public AST `is_lazy` fields elsewhere; this method is a host import-runtime side-effect check. |
| `Lib/test/test_ast/test_ast.py` | `AST_Tests` | 61 | `partial` | All portable public methods now have method-level Rust evidence, including public AST constructor/base-object behavior, generated ASDL class hierarchy/inventory/signatures, `_field_types` / `__annotations__`, `test_ast_validation` parser-produced public AST validation over the snippet matrix, compare modes, `test_snippets` public `to_tuple()` and `_assertTrueorder` slices, full `test_repr` snapshot parity from CPython's current `exec_tests + eval_tests`, feature-version cases, null-byte handling, import/alias/slice field checks, default end-position compile-from-AST cases, parser warning capture, and t-string structure. The row remains `partial` because `test_AST_garbage_collection` is blocked on public weakref/cyclic-GC runtime support and the remaining CPython-only methods are classified as implementation-internal. |
| `Lib/test/test_ast/test_ast.py` | `CopyTests` | 14 | `ported` | The method audit below now covers all 14 current methods. Direct method-level Rust evidence covers pickling, parent-link deepcopy, replace interface/native loops, native class fields/attributes, custom class fields/attributes, extra/missing field rejection, defaulted missing fields, and non-string unpacked keywords. Binary pickle byte compatibility remains outside this AST-only slice. |
| `Lib/test/test_ast/test_ast.py` | `ASTHelpers_Test` | 29 | `ported` | All 29 current CPython methods now have direct method-level Rust evidence, covering parse and parse-in-error behavior, dump variants, iterator helpers, literal evaluation and diagnostics, recursion detection, location helpers, docstring helpers, source-segment/end-position helpers, import-from validation, lazy import AST fields, and compile-from-public-AST helper coverage. |
| `Lib/test/test_ast/test_ast.py` | `ASTValidatorTests` | 40 | `ported` | All 40 current CPython methods now have method-level Rust evidence, covering public-AST root modes, statement and expression context validation, function/class/try/try-star validation, argument validation, comprehensions, match-pattern validation, `test_stdlib_validates` file-backed compile seeds, and recursive stdlib compile seeds. |
| `Lib/test/test_ast/test_ast.py` | `ConstantTests` | 8 | `ported` | All 8 current CPython methods now have method-level Rust evidence in the audit below. `cpython_ast_constant_compile_first_pass_subset` ports invalid Constant value validation, singleton identity preservation, scalar/tuple/frozenset value preservation, illegal assignment targets, docstring retrieval from Constant module docstrings, CPython-style `LOAD_CONST` observation through the supported `dis.hasconst` / `dis.get_instructions()` subset including tuple constants, `literal_eval()` operand replacement, and string-prefix `kind` metadata. |
| `Lib/test/test_ast/test_ast.py` | `EndPositionTests` | 28 | `ported` | The method audit below covers all 28 current CPython methods. Coverage includes parser source extraction for calls, definitions, literals, suites, f-strings, imports, slices, binary/boolean operations, tuple/list/set/dict displays, redundant parentheses, comprehensions, yield/await, newline variants, padded extraction, missing location attributes, and UTF-8 byte-column offsets. |
| `Lib/test/test_ast/test_ast.py` | `NodeTransformerTests` | 5 | `ported` | All 5 current CPython methods now have method-level Rust evidence in the audit below. `cpython_ast_node_transformer_first_pass_subset` covers removing a single AST field, removing a node from a list field, returning a list of replacement nodes, mutating a node in place, and replacing a node. It also covers the supporting `NodeVisitor` dispatch path used by `NodeTransformer`. |
| `Lib/test/test_ast/test_ast.py` | `ASTConstructorTests` | 11 | `ported` | All 11 current CPython methods now have direct method-level Rust evidence in the audit below, covering `FunctionDef`, expression-context defaults, fieldless custom subclasses, `_fields`, `_field_types`, `_attributes`, missing required fields, incomplete/malformed field metadata, implicit list defaults, and non-string unpacked constructor keywords. |
| `Lib/test/test_ast/test_ast.py` | `ModuleStateTests` | 3 | `blocked_by_ast_module` | The method audit below classifies all 3 current methods as CPython `ast` / `_ast` module lifecycle coverage: reload safety, `sys.modules` import hooks, and subinterpreter unload behavior. |
| `Lib/test/test_ast/test_ast.py` | `CommandLineTests` | 13 | `blocked_by_ast_module` | The method audit below classifies all 13 current methods as CPython `python -m ast` / `ast.main()` command-line surface coverage. |
| `Lib/test/test_ast/test_ast.py` | `ASTOptimizationTests` | 3 | `ported` | All 3 current CPython methods now have method-level Rust evidence in the audit below. `cpython_ast_optimization_format_folding_subset` ports `test_folding_format` by checking that `ast.parse(..., optimize=-1)` preserves the old-style `%s` `BinOp` while `optimize=1` folds it to `JoinedStr` / `FormattedValue`. `cpython_ast_optimization_match_case_folding_subset` ports `test_folding_match_case_allowed_expressions` and `test_match_case_not_folded_in_unoptimized_ast`, covering optimize-driven folding of signed real/imaginary match literals in `MatchValue`, `MatchMapping`, and nested `MatchSequence` patterns while preserving the unoptimized `BinOp` shape at `optimize=0`. |
| `Lib/test/test_ast/snippets.py` | snippet source data | 0 | `source_data` | Shared AST parse snippets are migrated through `cpython_ast_snippets_parse_inventory_subset`, sampled by `cpython_ast_snippets_structural_dump_subset`, and now have public-AST `to_tuple()` evidence in `cpython_ast_snippets_public_to_tuple_first_pass_subset` plus focused match, annotation, assignment/operator, assignment-target/block, with/raise/assert, try/try-star, import/control, decorator/named-expression, positional-only/default-parameter, type-parameter/type-alias, start-mode, eval-expression, display/comprehension, call/slice, and interpolated-string slices; this file has no unittest methods. |

## `Lib/test/test_ast/test_ast.py::LazyImportTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_lazy_import` | `blocked_by_runtime` | The method body only calls CPython's `ensure_lazy_imports("ast", ...)`, which spawns a `python -S -c ...` child process and checks `sys.modules` side effects after importing `ast`. MiniPython's portable lazy-import syntax and AST `is_lazy` surface are covered separately by `cpython_lazy_import_syntax_subset` and `cpython_ast_lazy_import_fields_subset`. | Requires a committed host-process/import-runtime side-effect contract for the `ast` module, not just language syntax or public AST nodes. |

## `Lib/test/test_ast/test_ast.py::AST_Tests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_AST_objects` | `ported` | `cpython_ast_base_object_and_missing_fields_subset` covers base `ast.AST()` fields, custom attributes, `__dict__`, missing-attribute lookup, and positional-argument rejection. | None for this method. |
| `test_AST_fields_NULL_check` | `ported` | `cpython_ast_base_object_and_missing_fields_subset` deletes class `_fields` and checks the public `AttributeError` constructor path rather than falling back to generated metadata. | None for this method. |
| `test_AST_garbage_collection` | `blocked_by_runtime` | None. | Requires public `weakref` plus cyclic GC behavior for AST objects. |
| `test_snippets` | `ported` | `cpython_ast_snippets_public_order_subset` covers the full current 219-case `exec` / `single` / `eval` snippet matrix for `_assertTrueorder`, `__match_args__`, and compile-from-public-AST round trips, while the focused `to_tuple()` slices cover the public tuple snapshots. | None for this method. |
| `test_ast_validation` | `ported` | `cpython_ast_snippets_public_order_subset` parses each current snippet with `optimize=False` and compiles the resulting public AST. | None for this method. |
| `test_parse_invalid_ast` | `ported` | `cpython_ast_parse_invalid_ast_subset` rejects non-root public AST nodes passed to `ast.parse()` for `optimize=-1/0/1/2`. | None for this method. |
| `test_optimization_levels__debug__` | `ported` | `cpython_ast_parse_optimize_debug_subset` covers string-source and public-AST `__debug__` parsing at all four optimize levels. | None for this method. |
| `test_invalid_position_information` | `ported` | `cpython_ast_invalid_position_information_subset` covers the CPython invalid line and column range matrices. | None for this method. |
| `test_compilation_of_ast_nodes_with_default_end_position_values` | `ported` | `cpython_ast_import_alias_slice_fields_subset` compiles hand-built import nodes whose end-position attributes are left at constructor defaults. | None for this method. |
| `test_negative_locations_for_compile` | `ported` | `cpython_ast_negative_locations_compile_subset` covers accepted negative-location compile and `ast.parse(..., optimize=2)` cases. | None for this method. |
| `test_docstring_optimization_single_node` | `ported` | `cpython_ast_docstring_optimization_single_node_subset` covers optimize-driven docstring removal for module, class, function, and async-function single-node bodies. | None for this method. |
| `test_docstring_optimization_multiple_nodes` | `ported` | `cpython_ast_docstring_optimization_multiple_nodes_subset` covers optimize-driven docstring removal while preserving following statement nodes. | None for this method. |
| `test_slice` | `ported` | `cpython_ast_import_alias_slice_fields_subset` checks public slice defaults for `x[::]`. | None for this method. |
| `test_from_import` | `ported` | `cpython_ast_import_alias_slice_fields_subset` checks relative from-import `module is None`. | None for this method. |
| `test_non_interned_future_from_ast` | `ported` | `cpython_ast_import_alias_slice_fields_subset` mutates a parsed future import module string before compile-from-public-AST. | None for this method. |
| `test_alias` | `ported` | `cpython_ast_import_alias_slice_fields_subset` covers alias `name`, `asname`, and source-position fields for ordinary, star, renamed, and import-alias forms. | None for this method. |
| `test_base_classes` | `ported` | `cpython_ast_base_classes_exact_subset` checks representative concrete and abstract AST inheritance with `issubclass()`. | None for this method. |
| `test_field_attr_existence` | `ported` | `cpython_ast_field_attr_existence_subset` walks `ast.__dict__`, constructs AST classes from generated annotations, and verifies tuple-valued `_fields`. | None for this method. |
| `test_arguments` | `ported` | `cpython_ast_arguments_annotations_subset` covers `ast.arguments` `_fields`, `_field_types`, `__annotations__`, default list/None fields, and positional construction. | None for this method. |
| `test_field_attr_writable` | `ported` | `cpython_ast_node_class_metadata_subset` covers writable instance `_fields`. | None for this method. |
| `test_classattrs` | `ported` | `cpython_ast_node_class_metadata_subset` covers `ast.Constant` constructor defaults, missing fields, location kwargs, arbitrary kwargs warnings, duplicate-value errors, and supported constant payloads. | None for this method. |
| `test_constant_subclasses` | `ported` | `cpython_ast_node_class_metadata_subset` covers native AST subclass initialization through `super().__init__`, custom attributes, exact type checks, and `isinstance()` behavior. | None for this method. |
| `test_module` | `ported` | `cpython_ast_node_class_metadata_subset` checks hand-built `ast.Module(body, [])` preserves the body list object. | None for this method. |
| `test_nodeclasses` | `ported` | `cpython_ast_node_class_metadata_subset` covers zero-argument deprecation behavior, random attributes, positional and keyword `BinOp` fields, too-many-argument errors, and arbitrary kwargs warnings. | None for this method. |
| `test_no_fields` | `ported` | `cpython_ast_node_class_metadata_subset` checks fieldless operator nodes such as `ast.Sub()` expose `_fields == ()`. | None for this method. |
| `test_invalid_sum` | `ported` | `cpython_ast_validator_basic_errors_subset` rejects abstract sum nodes where concrete public AST nodes are required. | None for this method. |
| `test_invalid_identifier` | `ported` | `cpython_ast_constant_name_validation_subset` rejects non-string `Name.id` during compile-from-public-AST. | None for this method. |
| `test_invalid_constant` | `ported` | `cpython_ast_validator_basic_errors_subset` rejects invalid `Constant` payloads containing type objects. | None for this method. |
| `test_empty_yield_from` | `ported` | `cpython_ast_validator_basic_errors_subset` rejects `YieldFrom.value = None`. | None for this method. |
| `test_issue31592` | `blocked_by_cpython_internal` | None. | This CPython-only crash regression monkeypatches `unicodedata.normalize`; MiniPython should not copy CPython's C assertion boundary. |
| `test_issue18374_binop_col_offset` | `ported` | `cpython_ast_binop_and_dotted_decorator_locations_subset` checks nested binop line and column spans. | None for this method. |
| `test_issue39579_dotted_name_end_col_offset` | `ported` | `cpython_ast_binop_and_dotted_decorator_locations_subset` checks dotted decorator attribute end columns. | None for this method. |
| `test_ast_asdl_signature` | `ported` | `cpython_ast_asdl_signature_doc_subset` checks generated ASDL `__doc__` signatures for representative concrete and sum classes. | None for this method. |
| `test_compare_basics` | `ported` | `cpython_ast_compare_first_pass_subset` covers basic equal and unequal parsed-tree comparisons. | None for this method. |
| `test_compare_modified_ast` | `ported` | `cpython_ast_compare_first_pass_subset` covers mutable `_fields` and `_attributes` comparison behavior. | None for this method. |
| `test_compare_literals` | `ported` | `cpython_ast_compare_literals_exact_subset` covers CPython's full literal matrix including signed integers, float infinities, non-ASCII strings, tuples, frozensets, and same-looking int/float/bool/complex values that must compare unequal as AST constants. | None for this method. |
| `test_compare_fieldless` | `ported` | `cpython_ast_compare_first_pass_subset` covers fieldless operator nodes and missing runtime field handling. | None for this method. |
| `test_compare_modes` | `ported` | `cpython_ast_compare_modes_snippets_subset` compares fresh public ASTs for the current CPython `exec_tests`, `eval_tests`, and `single_tests` snippet sets. | None for this method. |
| `test_compare_attributes_option` | `ported` | `cpython_ast_compare_first_pass_subset` covers `compare_attributes=False` versus `True` on location-different parsed trees. | None for this method. |
| `test_compare_attributes_option_missing_attribute` | `ported` | `cpython_ast_compare_first_pass_subset` covers missing runtime location attributes with `compare_attributes=True`. | None for this method. |
| `test_positional_only_feature_version` | `ported` | `cpython_ast_feature_version_gates_subset` accepts and rejects the CPython function and lambda positional-only examples at `(3, 8)` and `(3, 7)`. | None for this method. |
| `test_assignment_expression_feature_version` | `ported` | `cpython_ast_feature_version_gates_subset` gates walrus expressions at `(3, 8)` versus `(3, 7)`. | None for this method. |
| `test_pep750_tstring` | `ported` | `cpython_ast_feature_version_gates_subset` gates t-string parsing at `(3, 14)` versus `(3, 13)`. | None for this method. |
| `test_pep758_except_without_parens` | `ported` | `cpython_ast_pep758_feature_version_subset` gates comma-separated non-star exception handlers at `(3, 14)` versus `(3, 13)`. | None for this method. |
| `test_pep758_except_with_single_expr` | `ported` | `cpython_ast_pep758_feature_version_subset` covers every CPython single-expression, tuple-expression, parenthesized-expression, and `as exc` combination for ordinary `except` and `except*`, accepted at both `(3, 14)` and `(3, 13)`. | None for this method. |
| `test_pep758_except_star_without_parens` | `ported` | `cpython_ast_pep758_feature_version_subset` gates comma-separated `except*` handlers at `(3, 14)` versus `(3, 13)`. | None for this method. |
| `test_conditional_context_managers_parse_with_low_feature_version` | `ported` | `cpython_ast_feature_version_gates_subset` accepts a conditional expression context manager at feature version `(3, 8)`. | None for this method. |
| `test_exception_groups_feature_version` | `ported` | `cpython_ast_feature_version_gates_subset` gates `except*` at `(3, 11)` versus `(3, 10)`. | None for this method. |
| `test_type_params_feature_version` | `ported` | `cpython_ast_feature_version_gates_subset` gates type aliases, generic classes, and generic functions at `(3, 12)` versus `(3, 11)`. | None for this method. |
| `test_type_params_default_feature_version` | `ported` | `cpython_ast_feature_version_gates_subset` gates defaulted `TypeVar`, `TypeVarTuple`, and `ParamSpec` syntax at `(3, 13)` versus `(3, 12)`. | None for this method. |
| `test_invalid_major_feature_version` | `ported` | `cpython_ast_feature_version_gates_subset` rejects `(2, 7)` and `(4, 0)` feature versions. | None for this method. |
| `test_constant_as_name` | `ported` | `cpython_ast_constant_name_validation_subset` rejects `True`, `False`, and `None` represented as `ast.Name` during compile-from-public-AST. | None for this method. |
| `test_constant_as_unicode_name` | `ported` | `cpython_ast_constant_name_validation_subset` rejects Unicode-normalized constant names parsed from bytes source. | None for this method. |
| `test_precedence_enum` | `blocked_by_cpython_internal` | None. | Validates private `_ast_unparse._Precedence` enum layout with `enum._test_simple_enum`; MiniPython should test unparse precedence through public output instead. |
| `test_ast_recursion_limit` | `blocked_by_cpython_internal` | `cpython_static_nesting_and_complexity_limit_subset` and public-AST recursion tests cover MiniPython stack-safety behavior. | CPython's C recursion remaining-depth and platform crash-depth matrix is not a MiniPython public contract. |
| `test_null_bytes` | `ported` | `cpython_ast_parse_null_bytes_subset` checks the public `SyntaxError` message for NUL bytes in source strings. | None for this method. |
| `test_none_checks` | `ported` | `cpython_ast_none_required_fields_subset` mutates parser-built `alias`, `arg`, `comprehension`, `keyword`, `match_case`, and `withitem` required fields to `None` and checks exact `ValueError` diagnostics. | None for this method. |
| `test_repr` | `ported` | `cpython_ast_repr_full_snapshot_from_cpython_source_subset` loads CPython's current `snippets.py::exec_tests + eval_tests` list and compares MiniPython `repr(ast.parse(..., optimize=False))` output against every `data/ast_repr.txt` snapshot. | None for this method. |
| `test_repr_large_input_crash` | `ported` | `cpython_ast_repr_large_input_crash_subset` propagates the oversized integer decimal-conversion `ValueError` through `repr(ast.Constant(...))`. | None for this method. |
| `test_tstring` | `ported` | `cpython_ast_tstring_structure_subset` checks parser-generated `TemplateStr`, literal `Constant`, and `Interpolation` nodes. | None for this method. |
| `test_filter_syntax_warnings_by_module` | `ported` | `cpython_ast_filter_syntax_warnings_by_module_subset` captures tokenizer-originated `SyntaxWarning` records through `ast.parse()` with default and explicit filenames. | None for this method. |

## `Lib/test/test_ast/test_ast.py::CopyTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_pickling` | `ported` | `cpython_ast_copy_pickling_exact_subset` round-trips representative public AST trees through every MiniPython pickle protocol and compares them with `ast.compare(..., compare_attributes=True)`. | None for this method's observable AST round-trip behavior; CPython binary pickle byte compatibility is outside this AST-only slice. |
| `test_copy_with_parents` | `ported` | `cpython_ast_copy_with_parents_exact_subset` deep-copies the CPython parent-link tree shape, removes original parent links, and checks copied child parent structures compare to the copied parent nodes. | None for this method; CPython's stack-limit helper is not part of the public AST behavior. |
| `test_replace_interface` | `ported` | `cpython_ast_copy_replace_interface_exact_subset` walks every native AST class, checks `__replace__` availability, and rejects positional `copy.replace()` / `node.__replace__()` calls. | None for this method. |
| `test_replace_native` | `ported` | `cpython_ast_copy_replace_native_exact_subset` walks every native AST class and checks shallow replacement, field replacement, attribute replacement, and no side effects on the original node. | None for this method. |
| `test_replace_accept_known_class_fields` | `ported` | `cpython_ast_copy_replace_accept_known_class_fields_exact_subset` checks `ast.Name` field replacement preserves the original `ctx` object and installs the new `id` object. | None for this method. |
| `test_replace_accept_known_class_attributes` | `ported` | `cpython_ast_copy_replace_accept_known_class_attributes_exact_subset` checks native AST location-attribute replacement and `__reduce__()` state for original and replacement nodes. | None for this method. |
| `test_replace_accept_known_custom_class_fields` | `ported` | `cpython_ast_copy_replace_accept_known_custom_class_fields_exact_subset` covers shallow custom-field copy and explicit custom-field replacement while preserving both string-field and object-field identity. | None for this method. |
| `test_replace_accept_known_custom_class_attributes` | `ported` | `cpython_ast_copy_replace_accept_known_custom_class_attributes_exact_subset` checks custom `_attributes` defaults and replacement of a known custom attribute. | None for this method. |
| `test_replace_ignore_known_custom_instance_fields` | `ported` | `cpython_ast_copy_replace_ignore_known_custom_instance_fields_exact_subset` checks instance-only extra fields are dropped by shallow replacement and by known native-field replacement without mutating the original. | None for this method's supported value behavior. |
| `test_replace_reject_missing_field` | `ported` | `cpython_ast_copy_replace_reject_missing_field_exact_subset` checks missing required native fields raise the exact `TypeError` unless the replacement call provides the field. | None for this method. |
| `test_replace_accept_missing_field_with_default` | `ported` | `cpython_ast_copy_replace_accept_missing_field_with_default_exact_subset` checks defaulted `FunctionDef` fields survive replacement. | None for this method. |
| `test_replace_reject_known_custom_instance_fields_commits` | `ported` | `cpython_ast_copy_replace_reject_known_custom_instance_fields_commits_exact_subset` rejects explicit replacement of instance-only extra fields and preserves the original node. | None for this method's supported value behavior. |
| `test_replace_reject_unknown_instance_fields` | `ported` | `cpython_ast_copy_replace_reject_unknown_instance_fields_exact_subset` rejects unknown replacement keywords and preserves the original node. | None for this method's supported value behavior. |
| `test_replace_non_str_kwarg` | `ported` | `cpython_ast_copy_replace_non_str_kwarg_exact_subset` rejects non-string unpacked replacement keywords with the expected public `TypeError` shape. | None for this method. |

## `Lib/test/test_ast/test_ast.py::NodeTransformerTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_node_remove_single` | `ported` | `cpython_ast_node_transformer_first_pass_subset` removes the `SomeType` annotation from a `FunctionDef.returns` single-value AST field by returning `None` from `visit_Name()`. | None for this method. |
| `test_node_remove_from_list` | `ported` | `cpython_ast_node_transformer_first_pass_subset` removes a `yield` expression statement from a function-body list field by returning `None` from `visit_Expr()`. | None for this method. |
| `test_node_return_list` | `ported` | `cpython_ast_node_transformer_first_pass_subset` returns multiple `keyword` nodes from `visit_keyword()` and expands a class keyword list in place. | None for this method. |
| `test_node_mutate` | `ported` | `cpython_ast_node_transformer_first_pass_subset` mutates an existing call node in place by changing `print(...)` to `log(...)`. | None for this method. |
| `test_node_replace` | `ported` | `cpython_ast_node_transformer_first_pass_subset` replaces a call node with a new `logger.log(..., debug=True)` AST subtree. | None for this method. |

## `Lib/test/test_ast/test_ast.py::ConstantTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_validation` | `ported` | `cpython_ast_constant_compile_first_pass_subset` rejects `ast.Constant(value=[1, 2, 3])` during compile-from-public-AST with the CPython-facing invalid Constant type error. | None for this method. |
| `test_singletons` | `ported` | `cpython_ast_constant_compile_first_pass_subset` compiles public `ast.Constant` nodes for `None`, `False`, `True`, `Ellipsis`, and `b""` and checks singleton identity for the supported runtime values. | None for this method. |
| `test_values` | `ported` | `cpython_ast_constant_compile_first_pass_subset` compiles scalar, string, bytes, tuple, frozenset, nested tuple, and nested frozenset constants and checks value preservation. | None for this method. |
| `test_assign_to_constant` | `ported` | `cpython_ast_constant_compile_first_pass_subset` replaces an assignment target with `ast.Constant(value=1)` and checks the public Store-context `ValueError`. | None for this method. |
| `test_get_docstring` | `ported` | `cpython_ast_constant_compile_first_pass_subset` parses a module docstring with `optimize=False` and verifies `ast.get_docstring()` returns it. | None for this method. |
| `test_load_const` | `ported` | `cpython_ast_constant_compile_first_pass_subset` observes supported `LOAD_CONST` values through the public `dis.hasconst` / `dis.get_instructions()` subset before and after replacing expression nodes with `ast.Constant` nodes. | None for MiniPython's supported public `dis` slice; CPython bytecode layout beyond constant observation remains implementation-specific. |
| `test_literal_eval` | `ported` | `cpython_ast_constant_compile_first_pass_subset` replaces both sides of a parsed `BinOp` with `ast.Constant` nodes and checks `ast.literal_eval()` returns `10 + 20j`. | None for this method. |
| `test_string_kind` | `ported` | `cpython_ast_constant_compile_first_pass_subset` checks parser-generated `Constant.kind` for plain, `u`, raw, and bytes string prefixes. | None for this method. |

## `Lib/test/test_ast/test_ast.py::EndPositionTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_lambda` | `ported` | `cpython_ast_lambda_end_positions_exact_subset` checks lambda body and argument end-position/source-segment behavior. | None for this method. |
| `test_func_def` | `ported` | `cpython_ast_func_def_end_positions_exact_subset` checks function definition, argument annotation, return annotation, and return-statement spans. | None for this method. |
| `test_call` | `ported` | `cpython_ast_call_end_positions_exact_subset` checks call argument, keyword, and unpacked keyword value source segments. | None for this method. |
| `test_call_noargs` | `ported` | `cpython_ast_call_noargs_end_positions_exact_subset` checks no-argument call end positions. | None for this method. |
| `test_class_def` | `ported` | `cpython_ast_class_def_end_positions_exact_subset` checks class definition, base, and method-body source spans. | None for this method. |
| `test_class_kw` | `ported` | `cpython_ast_class_kw_end_positions_exact_subset` checks class keyword argument spans. | None for this method. |
| `test_multi_line_str` | `ported` | `cpython_ast_multi_line_str_end_positions_exact_subset` checks triple-quoted string literal spans. | None for this method. |
| `test_continued_str` | `ported` | `cpython_ast_continued_str_end_positions_exact_subset` checks adjacent continued string literal spans. | None for this method. |
| `test_suites` | `ported` | `cpython_ast_suites_end_positions_exact_subset` checks suite and control-flow statement source spans. | None for this method. |
| `test_fstring` | `ported` | `cpython_ast_fstring_end_positions_exact_subset` checks f-string replacement-expression spans. | None for this method. |
| `test_fstring_multi_line` | `ported` | `cpython_ast_fstring_multi_line_end_positions_exact_subset` checks multi-line f-string replacement-expression spans. | None for this method. |
| `test_import_from_multi_line` | `ported` | `cpython_ast_import_from_multiline_end_positions_exact_subset` checks parenthesized multi-line import-from spans. | None for this method. |
| `test_slices` | `ported` | `cpython_ast_slices_end_positions_exact_subset` checks subscript and nested slice source spans. | None for this method. |
| `test_binop` | `ported` | `cpython_ast_binop_end_positions_exact_subset` checks binary-operation end positions and child operand spans. | None for this method. |
| `test_boolop` | `ported` | `cpython_ast_boolop_end_positions_exact_subset` checks boolean-operation end positions and child operand spans. | None for this method. |
| `test_tuples` | `ported` | `cpython_ast_tuples_end_positions_exact_subset` checks tuple display source spans. | None for this method. |
| `test_attribute_spaces` | `ported` | `cpython_ast_attribute_spaces_end_positions_exact_subset` checks spaced attribute trailer spans. | None for this method. |
| `test_redundant_parenthesis` | `ported` | `cpython_ast_redundant_parenthesis_end_positions_exact_subset` checks redundant-parenthesis source-segment behavior. | None for this method. |
| `test_trailers_with_redundant_parenthesis` | `ported` | `cpython_ast_trailers_with_redundant_parenthesis_end_positions_exact_subset` checks call, subscript, and attribute trailer spans with redundant parentheses. | None for this method. |
| `test_displays` | `ported` | `cpython_ast_displays_end_positions_exact_subset` checks list, set, and dict display source spans. | None for this method. |
| `test_comprehensions` | `ported` | `cpython_ast_comprehensions_end_positions_exact_subset` checks comprehension target, iterable, filter, and outer-expression spans. | None for this method. |
| `test_yield_await` | `ported` | `cpython_ast_yield_await_end_positions_exact_subset` checks yield, yield-from, and await expression spans. | None for this method. |
| `test_source_segment_multi` | `ported` | `cpython_ast_source_segment_multi_exact_subset` checks source extraction for a multi-line tuple inside a binary operation. | None for this method. |
| `test_source_segment_padded` | `ported` | `cpython_ast_source_segment_padded_exact_subset` checks padded source extraction and UTF-8 byte-column end offsets. | None for this method. |
| `test_source_segment_endings` | `ported` | `cpython_ast_source_segment_endings_exact_subset` checks source extraction across CR, LF, and CRLF endings. | None for this method. |
| `test_source_segment_tabs` | `ported` | `cpython_ast_source_segment_tabs_exact_subset` checks padded source extraction with tab and form-feed indentation. | None for this method. |
| `test_source_segment_newlines` | `ported` | `cpython_ast_source_segment_newlines_exact_subset` checks source extraction across mixed newline function bodies. | None for this method. |
| `test_source_segment_missing_info` | `ported` | `cpython_ast_source_segment_missing_info_exact_subset` checks `ast.get_source_segment()` returns `None` when required location attributes are missing. | None for this method. |

## `Lib/test/test_ast/test_ast.py::ASTConstructorTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_FunctionDef` | `ported` | `cpython_ast_constructor_functiondef_exact_subset` checks default `ast.arguments()` lists, missing `FunctionDef.name` warning behavior, and default `decorator_list` preservation. | None for this method. |
| `test_expr_context` | `ported` | `cpython_ast_constructor_expr_context_exact_subset` checks `Name` constructor defaults to `Load`, accepts positional `Store`, accepts keyword `Del`, and warns when required `id` is missing. | None for this method. |
| `test_custom_subclass_with_no_fields` | `ported` | `cpython_ast_constructor_custom_subclass_with_no_fields_exact_subset` checks a fieldless custom `ast.AST` subclass constructs with an empty `__dict__`. | None for this method. |
| `test_fields_but_no_field_types` | `ported` | `cpython_ast_constructor_fields_but_no_field_types_exact_subset` checks a custom subclass with `_fields` but no `_field_types` leaves missing fields absent and accepts explicit keyword fields. | None for this method. |
| `test_fields_and_types` | `ported` | `cpython_ast_constructor_fields_and_types_exact_subset` checks `_field_types` plus annotation defaults produce `None` for missing optional fields and preserve explicit values. | None for this method. |
| `test_custom_attributes` | `ported` | `cpython_ast_constructor_custom_attributes_exact_subset` checks custom `_attributes` keyword assignment and unexpected keyword deprecation warnings. | None for this method. |
| `test_fields_and_types_no_default` | `ported` | `cpython_ast_constructor_fields_and_types_no_default_exact_subset` checks required custom fields without defaults warn when missing, remain absent, and preserve explicit values. | None for this method. |
| `test_incomplete_field_types` | `ported` | `cpython_ast_constructor_incomplete_field_types_exact_subset` checks missing `_field_types` entries warn while class annotation defaults still initialize both fields. | None for this method. |
| `test_malformed_fields_with_bytes` | `ported` | `cpython_ast_constructor_malformed_fields_with_bytes_exact_subset` checks malformed byte-valued `_fields` entries warn without crashing. | None for this method. |
| `test_complete_field_types` | `ported` | `cpython_ast_constructor_complete_field_types_exact_subset` checks complete custom `_field_types` provide explicit `None` defaults and implicit empty-list defaults. | None for this method. |
| `test_non_str_kwarg` | `ported` | `cpython_ast_constructor_non_str_kwarg_exact_subset` checks non-string unpacked constructor keywords raise `TypeError` and string-equal key objects collide with positional arguments. | None for this method. |

## `Lib/test/test_ast/test_ast.py::ModuleStateTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_reload_module` | `blocked_by_ast_module` | The method is `@support.cpython_only` through its class and asserts `_ast` module reload/unload safety via `sys.modules`, CPython `compile(..., PyCF_ONLY_AST)`, `types.CodeType`, and `support.gc_collect()`. | Requires a CPython-compatible `_ast` extension-module lifecycle and host module reload model. |
| `test_sys_modules` | `blocked_by_ast_module` | The method is `@support.cpython_only` through its class and asserts CPython's internal `PyAST_Check()` path does not import `_ast` through a monkeypatched `builtins.__import__`. | Requires CPython `_ast` import-state and `sys.modules` interaction semantics. |
| `test_subinterpreter` | `blocked_by_ast_module` | The method is `@support.cpython_only` through its class and runs `_ast` / `ast` compile/unload behavior in a CPython subinterpreter. | Requires CPython subinterpreter support and `_ast` module teardown semantics. |

## `Lib/test/test_ast/test_ast.py::CommandLineTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_invocation` | `blocked_by_ast_module` | The method exhaustively combines `ast.main()` command-line flags over a temporary file. | Requires the CPython `python -m ast` / `ast.main()` CLI surface and host tempfile policy. |
| `test_help_message` | `blocked_by_ast_module` | The method asserts `ast.main()` help and unknown-option usage output. | Requires the CPython `ast.main()` argument parser and stderr formatting. |
| `test_exec_mode_flag` | `blocked_by_ast_module` | The method asserts formatted `ast.main()` output for `-m/--mode exec`. | Requires the CPython `python -m ast` CLI output contract. |
| `test_single_mode_flag` | `blocked_by_ast_module` | The method asserts formatted `ast.main()` output for `-m/--mode single`. | Requires the CPython `python -m ast` CLI output contract. |
| `test_eval_mode_flag` | `blocked_by_ast_module` | The method asserts formatted `ast.main()` output for `-m/--mode eval`. | Requires the CPython `python -m ast` CLI output contract. |
| `test_func_type_mode_flag` | `blocked_by_ast_module` | The method asserts formatted `ast.main()` output for `-m/--mode func_type`. | Requires the CPython `python -m ast` CLI output contract. |
| `test_no_type_comments_flag` | `blocked_by_ast_module` | The method asserts `ast.main()` output with `--no-type-comments`. | Requires the CPython `python -m ast` CLI output contract. |
| `test_include_attributes_flag` | `blocked_by_ast_module` | The method asserts `ast.main()` output with `-a/--include-attributes`. | Requires the CPython `python -m ast` CLI output contract. |
| `test_indent_flag` | `blocked_by_ast_module` | The method asserts `ast.main()` output with `-i/--indent`. | Requires the CPython `python -m ast` CLI output contract. |
| `test_feature_version_flag` | `blocked_by_ast_module` | The method asserts `ast.main()` feature-version parsing and CLI-raised `SyntaxError`. | Requires the CPython `python -m ast` CLI output and error contract. |
| `test_no_optimize_flag` | `blocked_by_ast_module` | The method asserts unoptimized `ast.main()` output for match-case expressions. | Requires the CPython `python -m ast` CLI optimize-flag surface. |
| `test_optimize_flag` | `blocked_by_ast_module` | The method asserts optimized `ast.main()` output for match-case expressions. | Requires the CPython `python -m ast` CLI optimize-flag surface. |
| `test_show_empty_flag` | `blocked_by_ast_module` | The method asserts `ast.main()` output with `--show-empty`. | Requires the CPython `python -m ast` CLI output contract. |

## `Lib/test/test_ast/test_ast.py::ASTOptimizationTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_folding_format` | `ported` | `cpython_ast_optimization_format_folding_subset` checks `'%s' % (a,)` remains a `BinOp` at `optimize=-1` and folds to `JoinedStr` / `FormattedValue(conversion=115)` at `optimize=1`. | None for this method. |
| `test_folding_match_case_allowed_expressions` | `ported` | `cpython_ast_optimization_match_case_folding_subset` checks optimize-driven folding of signed numeric and real-plus-imaginary pattern literals in `MatchValue`, `MatchMapping`, and nested `MatchSequence` patterns. | None for this method. |
| `test_match_case_not_folded_in_unoptimized_ast` | `ported` | `cpython_ast_optimization_match_case_folding_subset` checks `case 1+2j` remains a `BinOp` pattern at `optimize=0` but folds to a `Constant` at `optimize=1/2`. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsInvalidTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_name_collisions` | `ported` | `cpython_type_params_duplicate_name_subset` rejects duplicate plain, TypeVarTuple, and ParamSpec names across function and class type-parameter lists. | None for this method. |
| `test_name_non_collision_02` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves a function type parameter and an ordinary positional parameter with the same name remain distinct. | None for this method. |
| `test_name_non_collision_03` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves a function type parameter and an ordinary vararg parameter with the same name remain distinct. | None for this method. |
| `test_name_non_collision_04` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves unmangled private type-parameter names do not collide with ordinary method parameters. | None for this method. |
| `test_name_non_collision_05` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves explicitly mangled-looking type-parameter names do not collide with ordinary private method parameters. | None for this method. |
| `test_name_non_collision_06` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves a generic class type parameter and an ordinary method parameter with the same name remain distinct. | None for this method. |
| `test_name_non_collision_07` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves a generic class type parameter and an ordinary method local with the same name remain distinct. | None for this method. |
| `test_name_non_collision_08` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves a generic class type parameter and a comprehension target with the same name remain distinct. | None for this method. |
| `test_name_non_collision_9` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves nested generic class and method type parameters with the same name remain separate objects. | None for this method. |
| `test_name_non_collision_10` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves a class annotation target with the same name as a type parameter remains an ordinary annotation binding. | None for this method. |
| `test_name_non_collision_13` | `ported` | `cpython_type_params_invalid_non_collision_subset` proves a nested generic function can still declare and write an ordinary global with the same name as its type parameter. | None for this method. |
| `test_disallowed_expressions` | `ported` | `cpython_type_params_invalid_disallowed_expression_subset` rejects named expressions, yield, yield-from, and await in generic definition/type-alias type scopes before runtime evaluation. | None for this method. |
| `test_incorrect_mro_explicit_object` | `ported` | `cpython_type_params_invalid_explicit_object_mro_subset` rejects an explicit `object` base on a generic class with a CPython-style MRO `TypeError`. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsAccessTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_class_access_01` | `ported` | `cpython_type_params_access_core_subset` proves generic class bases preserve both normalized `__bases__` and original `dict[A, B]` / `Generic[A, B]` metadata through `types.get_original_bases()`. | None for this method. |
| `test_class_access_02` | `ported` | `cpython_type_params_access_core_subset` proves a generic metaclass expression `MyMeta[A, B]` uses the class type-parameter scope while preserving distinct metaclass/class type-parameter objects and exact `type(cls) is meta` / `cls.__class__ is meta` behavior. | None for this method. |
| `test_class_access_03` | `ported` | `cpython_type_params_access_core_subset` rejects class decorators that try to read class type parameters before their type scope exists. | None for this method. |
| `test_function_access_01` | `ported` | `cpython_type_params_access_core_subset` proves function annotations reuse the exact `__type_params__` objects inside `dict[A, B]`. | None for this method. |
| `test_function_access_02` | `ported` | `cpython_type_params_access_core_subset` proves default values are evaluated outside the function type-parameter scope and raise `NameError` for `A`. | None for this method. |
| `test_function_access_03` | `ported` | `cpython_type_params_access_core_subset` proves function decorators are evaluated outside the function type-parameter scope and raise `NameError` for `A`. | None for this method. |
| `test_method_access_01` | `ported` | `cpython_type_params_access_core_subset` proves method annotations can see a class-local annotation name and the exact method type-parameter object. | None for this method. |
| `test_nested_access_01` | `ported` | `cpython_type_params_access_core_subset` proves nested generic classes/functions preserve the exact outer and inner type-parameter objects through a lambda return. | None for this method. |
| `test_out_of_scope_01` | `ported` | `cpython_type_params_access_core_subset` proves class type parameters do not leak to the following module statement. | None for this method. |
| `test_out_of_scope_02` | `ported` | `cpython_type_params_access_core_subset` proves method type parameters do not leak to the surrounding class body. | None for this method. |
| `test_class_scope_interaction_01` | `ported` | `cpython_type_params_access_core_subset` proves a generic method annotation can read an earlier class local. | None for this method. |
| `test_class_scope_interaction_02` | `ported` | `cpython_type_params_access_core_subset` proves nested generic classes append implicit `Generic` in `__bases__` and preserve `Base` / `Generic[T]` in `types.get_original_bases()`. | None for this method. |
| `test_class_deref` | `ported` | `cpython_type_params_access_core_subset` proves a class-local binding wins when a non-generic type alias in a generic class reads `T`. | None for this method. |
| `test_shadowing_nonlocal` | `ported` | `cpython_type_params_access_core_subset` proves ordinary local shadowing of a type parameter remains capturable and mutable through `nonlocal`. | None for this method. |
| `test_reference_previous_typevar` | `ported` | `cpython_type_params_access_core_subset` proves later type-parameter bounds can reference earlier type parameters through `Sequence[S]`. | None for this method. |
| `test_super` | `ported` | `cpython_type_params_access_core_subset` proves a generic method with annotations still supports zero-argument `super()`. | None for this method. |
| `test_type_alias_containing_lambda` | `ported` | `cpython_type_params_access_core_subset` proves a lambda stored in a generic type alias returns the exact alias type-parameter object. | None for this method. |
| `test_class_base_containing_lambda` | `ported` | `cpython_type_params_access_core_subset` proves a lambda created inside a generic class base captures an outer local plus the class type parameter and remains observable through `types.get_original_bases()` and `typing.get_args()`. | None for this method. |
| `test_comprehension_01` | `ported` | `cpython_type_params_access_core_subset` proves alias values and type-parameter constraints evaluate comprehensions in annotation scope, including target shadowing and self type-parameter references exposed through `__constraints__`. | None for this method. |
| `test_comprehension_02` | `ported` | `cpython_type_params_access_core_subset` proves lambdas nested inside alias-value and bound comprehensions capture the correct comprehension target values. | None for this method. |
| `test_comprehension_03` | `ported` | `cpython_type_params_access_core_subset` proves the same nested-lambda comprehension capture behavior for generic function type-parameter bounds and returned values. | None for this method. |
| `test_gen_exp_in_nested_class` | `ported` | `cpython_type_params_access_core_subset` proves a generator expression in a nested class base captures the outer class type parameter while a sibling base expression still sees the class-local binding. | None for this method. |
| `test_gen_exp_in_nested_generic_class` | `ported` | `cpython_type_params_access_core_subset` proves the same generator-expression base lookup split when the nested class is itself generic and has an implicit `Generic` base. | None for this method. |
| `test_listcomp_in_nested_class` | `ported` | `cpython_type_params_access_core_subset` proves a list comprehension in a nested class base captures the outer class type parameter while a sibling base expression still sees the class-local binding. | None for this method. |
| `test_listcomp_in_nested_generic_class` | `ported` | `cpython_type_params_access_core_subset` proves the same list-comprehension base lookup split when the nested class is itself generic and has an implicit `Generic` base. | None for this method. |
| `test_gen_exp_in_generic_method` | `ported` | `cpython_type_params_access_core_subset` proves generic-method generator-expression annotations capture the enclosing class type parameter while sibling annotations preserve class-local lookup. | None for this method. |
| `test_nested_scope_in_generic_alias` | `ported` | `cpython_type_params_access_core_subset` proves generic alias generator/list comprehensions see the alias type parameter, while non-generic alias generator/list comprehensions in class scope resolve sibling names through globals rather than class locals. | None for this method. |
| `test_lambda_in_alias_in_class` | `ported` | `cpython_type_params_access_core_subset` proves non-generic alias lambdas in class scope use global lookup instead of class-local bindings. | None for this method. |
| `test_lambda_in_alias_in_generic_class` | `ported` | `cpython_type_params_access_core_subset` proves a lambda inside a non-generic alias in a generic class sees the class type parameter with exact identity. | None for this method. |
| `test_lambda_in_generic_alias_in_class` | `ported` | `cpython_type_params_access_core_subset` proves a lambda inside a generic class-local alias sees the alias type parameter while sibling names still use global lookup. | None for this method. |
| `test_lambda_in_generic_alias_in_generic_class` | `ported` | `cpython_type_params_access_core_subset` proves nested annotation-scope capture where alias `T` shadows class `T` and class `U` remains visible with exact identity. | None for this method. |
| `test_type_special_case` | `ported` | `cpython_type_params_access_core_subset` proves builtin `type` and `object` expose empty `__type_params__`. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsLazyEvaluationTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_qualname` | `ported` | `cpython_type_params_lazy_evaluation_qualname_subset` proves module-level and function-local generic classes/functions expose CPython-style `__qualname__`, including `<locals>` for nested definitions. | None for this method. |
| `test_recursive_class` | `ported` | `cpython_type_params_lazy_evaluation_bounds_subset` proves recursive class references resolve lazily through `__bound__` / `__constraints__`, including tuple constraints, class-object identity, and `typing.NoDefault` for missing defaults. | None for this method. |
| `test_evaluation_error` | `ported` | `cpython_type_params_lazy_evaluation_bounds_subset` proves undefined names no longer fail class creation, `__bound__` / tuple `__constraints__` raise on public access, non-tuple `__constraints__` / tuple `__bound__` avoid premature evaluation, later binding re-evaluates successfully, and missing defaults expose `typing.NoDefault`. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsTraditionalTypeVarsTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_traditional_01` | `ported` | `cpython_type_params_traditional_typevars_subset` proves a PEP 695 generic class header rejects explicit `Generic[T]` inheritance instead of creating a duplicate Generic base. | None for this method. |
| `test_traditional_02` | `ported` | `cpython_type_params_traditional_typevars_subset` proves `typing.TypeVar("S")` creates a public TypeVar object and generic class bases reject undeclared traditional TypeVars such as `dict[T, S]`. | None for this method. |
| `test_traditional_03` | `ported` | `cpython_type_params_traditional_typevars_subset` proves ordinary function annotations can combine a PEP 695 type parameter and a traditional `typing.TypeVar`, including union annotations and exact annotation object identity. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsTypeVarTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_typevar_01` | `ported` | `cpython_type_params_typevar_runtime_subset` proves PEP 695 plain type parameters are `typing.TypeVar` instances with lazy bound, union-bound, tuple-constraint, `__infer_variance__`, `__covariant__`, and `__contravariant__` metadata. | None for this method. |
| `test_typevar_generator` | `ported` | `cpython_type_params_typevar_runtime_subset` proves nested generator functions preserve outer, inner, and nested-inner type-parameter objects as `typing.TypeVar` instances. | None for this method. |
| `test_typevar_coroutine` | `ported` | `cpython_type_params_typevar_runtime_subset` proves nested async functions preserve outer and inner type-parameter objects as `typing.TypeVar` instances through coroutine completion. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsTypeVarTupleTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_typevartuple_01` | `ported` | `cpython_invalid_type_param_subset` rejects bounds and constraints on `*A` TypeVarTuple parameters for functions, classes, and type aliases. | None for this method. |
| `test_typevartuple_02` | `ported` | `cpython_type_params_typevartuple_paramspec_runtime_subset` proves PEP 695 variadic type parameters are `typing.TypeVarTuple` instances and traditional `TypeVarTuple("Ts")` exposes public name/default metadata. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsTypeVarParamSpecTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_paramspec_01` | `ported` | `cpython_invalid_type_param_subset` rejects bounds and constraints on `**A` ParamSpec parameters for functions, classes, and type aliases. | None for this method. |
| `test_paramspec_02` | `ported` | `cpython_type_params_typevartuple_paramspec_runtime_subset` proves PEP 695 ParamSpec objects are `typing.ParamSpec` instances with variance metadata, and traditional `ParamSpec("P")` exposes public name/default metadata. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsRuntimeTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_name_error` | `ported` | `cpython_type_params_runtime_name_error_subset` proves both nested generic class-header regressions raise catchable `NameError` at runtime: an undefined base and an undefined base combined with a type-parameter bound referencing the current parameter. | None for this method. |
| `test_broken_class_namespace` | `ported` | `cpython_type_params_runtime_class_namespace_subset` proves a metaclass `__prepare__` dict-subclass namespace participates in nested generic class base lookup and that `__missing__("T")` can raise catchable `RuntimeError`; the same Rust test also verifies dict-subclass namespace assignments become final class attributes. | None for this method. |

## `Lib/test/test_type_params.py::DefaultsTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_defaults_on_func` | `ported` | `cpython_type_param_defaults_subset` proves generic functions expose plain TypeVar, ParamSpec, and TypeVarTuple `__default__` metadata for `int`, `float`, and `None`. | None for this method. |
| `test_defaults_on_class` | `ported` | `cpython_type_param_defaults_subset` proves generic classes expose plain TypeVar, ParamSpec, and TypeVarTuple `__default__` metadata for `int`, `float`, and `None`. | None for this method. |
| `test_defaults_on_type_alias` | `ported` | `cpython_type_param_defaults_subset` proves type aliases expose plain TypeVar, ParamSpec, and TypeVarTuple `__default__` metadata for `int`, `float`, and `None`. | None for this method. |
| `test_starred_invalid` | `ported` | `cpython_type_param_starred_invalid_subset` rejects `type Alias[T = *int] = int` and `type Alias[**P = *int] = int`. | None for this method. |
| `test_starred_typevartuple` | `ported` | `cpython_type_param_defaults_subset` proves a starred TypeVarTuple default equals the one-shot `Unpack[...]` object returned by `next(iter(tuple[int, str]))`, keeps the CPython-style repr, and still exposes generic-alias origin/args metadata. | None for this method. |
| `test_nondefault_after_default` | `ported` | `cpython_type_param_nondefault_after_default_subset` rejects function, class, and type-alias parameter lists where a non-default type parameter follows a defaulted one. | None for this method. |
| `test_lazy_evaluation` | `ported` | `cpython_type_param_defaults_lazy_and_symtable_subset` proves undefined defaults raise `NameError`, later bindings evaluate successfully, and successful default reads are cached. | None for this method. |
| `test_symtable_key_regression_default` | `ported` | `cpython_type_param_defaults_lazy_and_symtable_subset` proves a default expression containing `[T for T in [T]]` resolves the correct type-parameter object. | None for this method. |
| `test_symtable_key_regression_name` | `ported` | `cpython_type_param_defaults_lazy_and_symtable_subset` proves separate aliases using defaults `A` and `B` resolve their own later module bindings. | None for this method. |

## `Lib/test/test_type_params.py::TestEvaluateFunctions` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_general` | `ported` | `cpython_type_params_evaluate_functions_subset` proves `evaluate_value`, `evaluate_bound`, and `evaluate_default` callables return `int` directly and through `annotationlib.call_evaluate_function()` for VALUE/FORWARDREF, and return `int` for STRING across PEP 695 aliases/parameters plus traditional `typing.TypeAliasType`, `TypeVar`, `ParamSpec`, and `TypeVarTuple` constructors. | None for this method. |
| `test_constraints` | `ported` | `cpython_type_params_evaluate_functions_subset` proves PEP 695 and traditional `TypeVar` constraint evaluators return `(int, str)` for direct calls plus VALUE/FORWARDREF, and return `(int, str)` for STRING. | None for this method. |
| `test_const_evaluator` | `ported` | `cpython_type_params_evaluate_functions_subset` checks the public const-evaluator repr for an `int` bound and the `_typing._ConstEvaluator` TypeError paths for direct construction and class-attribute assignment. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsManglingTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_mangling` | `ported` | `cpython_type_params_mangling_subset` proves public `__name__` values stay unmangled while class-private references inside the class body, method annotations/body, and type-alias value resolve to the same type-parameter objects. | None for this method. |
| `test_no_leaky_mangling_in_module` | `ported` | `cpython_type_params_mangling_subset` proves module-level private-looking names before and after a generic class definition remain ordinary module names. | None for this method. |
| `test_no_leaky_mangling_in_function` | `ported` | `cpython_type_params_mangling_subset` proves a generic class inside a function does not cause later function-local `__foo` bindings to be class-mangled. | None for this method. |
| `test_no_leaky_mangling_in_class` | `ported` | `cpython_type_params_mangling_subset` proves an inner generic class restores the outer class mangling context and preserves the inner class's own private-name mangling. | None for this method. |
| `test_no_mangling_in_bases` | `ported` | `cpython_type_params_mangling_subset` proves class-header bases and keyword names are evaluated without class-private mangling while still adding the implicit `Generic` base. | None for this method. |
| `test_no_mangling_in_nested_scopes` | `ported` | `cpython_type_params_mangling_subset` proves non-type-parameter private-looking globals used in bounds, lambdas, list comprehensions, and generator expressions in class headers are not class-mangled. | None for this method. |
| `test_type_params_are_mangled` | `ported` | `cpython_type_params_mangling_subset` proves private-looking type parameters are available through class-private references in bounds, class-header base expressions, header lambdas, and class-body assignments. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsTypeParamsDunder` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_typeparams_dunder_class_01` | `ported` | `cpython_type_params_dunder_subset` proves nested generic class type parameters are visible to a nested static method, match `Outer.__type_params__` / `Inner.__type_params__`, and are exposed through `__parameters__`. | None for this method. |
| `test_typeparams_dunder_class_02` | `ported` | `cpython_type_params_dunder_subset` proves a non-generic class exposes empty `__type_params__`. | None for this method. |
| `test_typeparams_dunder_class_03` | `ported` | `cpython_type_params_dunder_subset` proves assigning `ClassA.__type_params__ = ()` overrides the visible dunder value. | None for this method. |
| `test_typeparams_dunder_function_01` | `ported` | `cpython_type_params_dunder_subset` proves nested generic function type parameters are visible in the nested function body and match `outer.__type_params__` / `inner.__type_params__`. | None for this method. |
| `test_typeparams_dunder_function_02` | `ported` | `cpython_type_params_dunder_subset` proves a non-generic function exposes empty `__type_params__`. | None for this method. |
| `test_typeparams_dunder_function_03` | `ported` | `cpython_type_params_dunder_subset` proves assigning `func.__type_params__ = ()` overrides the visible dunder value. | None for this method. |

## `Lib/test/test_type_params.py::TypeParamsWeakRefTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_weakrefs` | `ported` | `cpython_type_params_weakrefs_subset` covers `weakref.ref()` construction for traditional `TypeVar`, `ParamSpec`, `ParamSpec.args`, `ParamSpec.kwargs`, `TypeVarTuple`, old-style and PEP 695 generic classes, their generic aliases, their instances, and `Generic[T]`; it also covers first-pass `ParamSpecArgs` / `ParamSpecKwargs` `__origin__` identity. | None for this method; collection-time weakref clearing remains outside this CPython method. |

## `Lib/test/test_type_params.py::TypeParamsNonlocalTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_nonlocal_disallowed_01` | `ported` | `cpython_type_params_nonlocal_scope_subset` rejects `nonlocal X` inside a nested generic function where `X` is the type parameter. | None for this method. |
| `test_nonlocal_disallowed_02` | `ported` | `cpython_type_params_nonlocal_scope_subset` rejects a nested ordinary function declaring the outer generic function's type parameter as `nonlocal`. | None for this method. |
| `test_nonlocal_disallowed_03` | `ported` | `cpython_type_params_nonlocal_scope_subset` rejects `nonlocal T` directly inside a generic class body. | None for this method. |
| `test_nonlocal_allowed` | `ported` | `cpython_type_params_nonlocal_scope_subset` preserves ordinary local shadowing of a type parameter and allows a nested closure to capture that local binding. | None for this method. |

## `Lib/test/test_type_comments.py::TypeCommentTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_funcdef` | `ported` | `cpython_type_comment_public_ast_metadata_subset` and `cpython_func_type_comment_helper_rules_subset` cover own-line and inline function type comments plus ordinary `ast.parse()` returning `None`. | None for this method. |
| `test_asyncdef` | `ported` | `cpython_type_comment_public_ast_metadata_subset` and `cpython_func_type_comment_helper_rules_subset` cover own-line and inline async-function type comments plus ordinary `ast.parse()` returning `None`. | None for this method. |
| `test_asyncvar` | `ported` | `cpython_type_comment_modern_syntax_and_ignores_subset` checks default public `ast.parse()` rejects assigning to `async` and `await`. | None for this method. |
| `test_asynccomp` | `ported` | `cpython_type_comment_modern_syntax_and_ignores_subset` parses the async-comprehension source with `type_comments=True`. | None for this method. |
| `test_matmul` | `ported` | `cpython_type_comment_modern_syntax_and_ignores_subset` parses the matrix-multiply source with `type_comments=True`. | None for this method. |
| `test_fstring` | `ported` | `cpython_type_comment_modern_syntax_and_ignores_subset` parses the f-string source with `type_comments=True`. | None for this method. |
| `test_underscorednumber` | `ported` | `cpython_type_comment_modern_syntax_and_ignores_subset` parses the underscored-number source with `type_comments=True`. | None for this method. |
| `test_redundantdef` | `ported` | `cpython_func_type_comment_helper_rules_subset` rejects duplicate function and async-function type comments. | None for this method. |
| `test_nonasciidef` | `ported` | `cpython_type_comment_modern_syntax_and_ignores_subset` preserves the non-ASCII function type-comment text. | None for this method. |
| `test_forstmt` | `ported` | `cpython_type_comment_public_ast_metadata_subset` preserves `For.type_comment` with `type_comments=True` and returns `None` through ordinary `ast.parse()`. | None for this method. |
| `test_withstmt` | `ported` | `cpython_type_comment_public_ast_metadata_subset` preserves `With.type_comment` with `type_comments=True` and returns `None` through ordinary `ast.parse()`. | None for this method. |
| `test_parenthesized_withstmt` | `ported` | `cpython_type_comment_public_ast_metadata_subset` and `cpython_type_comment_modern_syntax_and_ignores_subset` preserve type comments on parenthesized `with` statements and hide them without `type_comments=True`. | None for this method. |
| `test_vardecl` | `ported` | `cpython_type_comment_public_ast_metadata_subset` preserves `Assign.type_comment` with `type_comments=True` and returns `None` through ordinary `ast.parse()`. | None for this method. |
| `test_ignores` | `ported` | `cpython_type_comment_modern_syntax_and_ignores_subset` preserves the exact `TypeIgnore.lineno` and `TypeIgnore.tag` values and returns an empty list without `type_comments=True`. | None for this method. |
| `test_longargs` | `ported` | `cpython_type_comment_argument_ast_metadata_subset` covers positional-only, ordinary, vararg, keyword-only, and kwarg argument type comments plus ordinary `ast.parse()` returning `None`. | None for this method. |
| `test_inappropriate_type_comments` | `ported` | `cpython_inappropriate_type_comments_subset` checks ordinary parsing ignores misplaced type comments while `ast.parse(..., type_comments=True)` raises `SyntaxError`. | None for this method. |
| `test_func_type_input` | `ported` | `cpython_func_type_input_subset` and `cpython_type_expression_helper_rules_subset` cover public `ast.parse(..., mode="func_type")` shapes and marker ordering. | None for this method. |

## `Lib/test/test_compile.py::TestSpecifics` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_no_ending_newline` | `ported` | `cpython_compile_specifics_newline_and_indentation_subset` accepts non-newline-terminated source and old-Mac final carriage-return source through public `compile()`. | None for this method. |
| `test_empty` | `ported` | `cpython_compile_specifics_newline_and_indentation_subset` accepts empty string source through public `compile()`. | None for this method. |
| `test_other_newlines` | `ported` | `cpython_compile_specifics_newline_and_indentation_subset` accepts CRLF, lone-CR, and mixed-newline source containing function definitions. | None for this method. |
| `test_debug_assignment` | `ported` | `cpython_compile_specifics_debug_assignment_subset` rejects assignment to `__debug__` through public `compile()`, imports the public `builtins` module, mutates `builtins.__debug__` with `setattr()`, and proves expression-level `__debug__` remains the previous constant value. | None for this method. |
| `test_argument_handling` | `ported` | `cpython_compile_specifics_syntax_error_boundaries_subset` rejects duplicate lambda parameters, duplicate function parameters, and local/global parameter conflicts. | None for this method. |
| `test_syntax_error` | `ported` | `cpython_compile_specifics_syntax_error_boundaries_subset` rejects the invalid `1+*3` compile boundary. | None for this method. |
| `test_none_keyword_arg` | `ported` | `cpython_compile_specifics_syntax_error_boundaries_subset` rejects `f(None=1)`. | None for this method. |
| `test_duplicate_global_local` | `ported` | `cpython_compile_specifics_syntax_error_boundaries_subset` rejects `def f(a): global a; a = 1`. | None for this method. |
| `test_exec_with_general_mapping_for_locals` | `ported` | `cpython_compile_specifics_exec_general_mapping_locals_subset` covers mapping locals for `exec()`, including `__getitem__`, `__setitem__`, `keys()`, `globals()`, `locals()`, missing-key handling, dict subclasses, and non-mapping locals rejection. | None for this method. |
| `test_extended_arg` | `ported` | `cpython_compile_specifics_compile_stability_subset` executes the CPython long-expression plus decrementing `while` loop source shape and checks it returns `0`. | None for the public source behavior; CPython `EXTENDED_ARG` bytecode shape is not a MiniPython register-VM contract. |
| `test_argument_order` | `ported` | `cpython_compile_specifics_syntax_error_boundaries_subset` rejects default-before-nondefault parameter ordering. | None for this method. |
| `test_float_literals` | `ported` | `cpython_compile_specifics_syntax_error_boundaries_subset` rejects the bad float literal spellings from the CPython method. | None for this method. |
| `test_indentation` | `ported` | `cpython_compile_specifics_newline_and_indentation_subset` accepts nested indented blocks without a trailing newline. | None for this method. |
| `test_leading_newlines` | `ported` | `cpython_compile_specifics_newline_and_indentation_subset` checks `compile("\\n" * 256 + "spam", "fn", "exec").co_firstlineno == 1` and `[line for _, _, line in co.co_lines()] == [0, 257]`. | None for this method. |
| `test_literals_with_leading_zeroes` | `ported` | `NUMBER` coverage includes CPython invalid leading-zero integer and prefixed forms plus valid leading-zero float, exponent, and imaginary literals. | None for this method. |
| `test_int_literals_too_long` | `ported` | `cpython_compile_specifics_int_literals_too_long_subset` directly ports the compile-time decimal integer source limit matrix, including `SyntaxError.lineno` for the offending line and unlimited hexadecimal literal parsing. | None for this method. |
| `test_unary_minus` | `ported` | `cpython_compile_specifics_integer_constant_edges_subset` covers large hexadecimal integers, unary minus, signed minimum-boundary literals, and large integer `co_consts` exposure as Python `int`. | None for MiniPython's public integer semantics; exact CPython constant-folding table shape is not required. |
| `test_sequence_unpacking_error` | `ported` | `cpython_compile_specifics_compile_stability_subset` executes the CPython sequence-unpacking regression source and checks the resulting values. | None for this method. |
| `test_none_assignment` | `ported` | `cpython_compile_specifics_none_assignment_subset` rejects binding `None` across assignments, definitions, loops, parameters, and imports in both `single` and `exec` modes. | None for this method. |
| `test_import` | `ported` | `cpython_compile_specifics_import_syntax_subset` covers accepted import forms, future imports, aliases, parenthesized from-imports, and invalid malformed import shapes. | None for this method. |
| `test_for_distinct_code_objects` | `ported` | `cpython_compile_specifics_lambda_code_metadata_subset` checks distinct lambda code-object identities. | None for this method. |
| `test_lambda_doc` | `ported` | `cpython_compile_specifics_lambda_code_metadata_subset` checks lambda `__doc__ is None`. | None for this method. |
| `test_lambda_consts` | `ported` | `cpython_compile_specifics_lambda_code_metadata_subset` checks lambda/function `__code__.co_consts` for the supported constant surface. | None for this method. |
| `test_encoding` | `ported` | `cpython_compile_specifics_encoding_subset` covers comment and coding-cookie lines for string source plus bytes-source decoding and bad-cookie rejection. | None for this method. |
| `test_subscripts` | `ported` | `cpython_user_defined_subscript_protocol_subset` covers user-defined subscript protocol behavior for `__getitem__`, `__setitem__`, and `__delitem__` shapes exercised by the CPython method. | None for this method. |
| `test_annotation_limit` | `ported` | `cpython_compile_specifics_compile_stability_subset` compiles the large annotated-signature source. | None for this method. |
| `test_mangling` | `ported` | `cpython_compile_specifics_name_mangling_code_varnames_subset` covers class-private name mangling in function `co_varnames` for assignment, deletion, annotation, and import bindings. | None for this method. |
| `test_condition_expression_with_dead_blocks_compiles` | `ported` | `cpython_compile_specifics_compile_stability_subset` compiles the conditional-expression dead-block source. | None for this method. |
| `test_condition_expression_with_redundant_comparisons_compiles` | `ported` | `cpython_compile_specifics_compile_stability_subset` compiles the redundant-comparison conditional sources. | None for this method. |
| `test_dead_code_with_except_handler_compiles` | `ported` | `cpython_compile_specifics_compile_stability_subset` compiles the dead-code except-handler source. | None for this method. |
| `test_try_except_in_while_with_chained_condition_compiles` | `ported` | `cpython_compile_specifics_compile_stability_subset` compiles the chained-condition while/try/except/finally source. | None for this method. |
| `test_compile_invalid_namedexpr` | `ported` | `cpython_compile_specifics_invalid_public_ast_subset` rejects invalid public-AST `NamedExpr.target` with CPython-style `TypeError`. | None for this method. |
| `test_compile_redundant_jumps_and_nops_after_moving_cold_blocks` | `blocked_by_cpython_internal` | The method checks CPython opcode order and NOP/jump elimination through `dis`. | MiniPython should not copy CPython bytecode optimizer layout. |
| `test_compile_redundant_jump_after_convert_pseudo_ops` | `blocked_by_cpython_internal` | The method checks CPython opcode layout after pseudo-op conversion. | MiniPython register bytecode has different control-flow lowering. |
| `test_compile_ast` | `ported` | `cpython_compile_specifics_compile_ast_public_subset` directly covers the CPython method's small source -> AST -> code sample matrix, code-object equality independent of filename, second-compile `co_filename`, mode/start-node `TypeError` boundaries, and invalid-child `TypeError`; `cpython_compile_specifics_compile_ast_cpython_file_subset` covers the method's full `Lib/test/test_compile.py` self-compile sample. | None for this method. |
| `test_compile_invalid_typealias` | `ported` | `cpython_compile_specifics_invalid_public_ast_subset` rejects invalid public-AST `TypeAlias.name` with CPython-style `TypeError`. | None for this method. |
| `test_dict_evaluation_order` | `ported` | `cpython_compile_specifics_dict_evaluation_order_subset` pins left-to-right key-before-value dict display evaluation. | None for this method. |
| `test_compile_filename` | `ported` | `cpython_compile_specifics_compile_filename_subset` covers string and bytes filenames, `co_filename`, and bytearray, memoryview, and list filename rejection. | None for this method. |
| `test_compile_filename_refleak` | `ported` | `cpython_compile_specifics_compile_argument_conversion_subset` covers the method's public argument conversion errors for `mode`, `optimize`, and `dont_inherit`. | None for the public behavior; the original reference-leak harness intent is CPython internal and intentionally not reproduced. |
| `test_same_filename_used` | `blocked_by_cpython_internal` | The method checks identity sharing of nested code-object `co_filename` strings. | MiniPython should preserve public filename values, not CPython object interning. |
| `test_single_statement` | `ported` | `cpython_compile_specifics_single_statement_subset` accepts the CPython `single`-mode one-statement shapes, including simple, compound, class, import, comments, and multiline string cases. | None for this method. |
| `test_bad_single_statement` | `ported` | `cpython_compile_specifics_single_statement_subset` rejects multi-physical-statement `single`-mode inputs and unterminated inline compound statements. | None for this method. |
| `test_particularly_evil_undecodable` | `blocked_by_runtime` | Null-byte source rejection is covered by source and bytes compile tests. | The CPython method uses temp files and child-process script execution, which belongs to a future sandbox host-IO policy. |
| `test_yet_more_evil_still_undecodable` | `blocked_by_runtime` | Null-byte source rejection is covered by source and bytes compile tests. | The CPython method uses temp files and child-process script execution, which belongs to a future sandbox host-IO policy. |
| `test_compiler_recursion_limit` | `blocked_by_cpython_internal` | `cpython_static_nesting_and_complexity_limit_subset` covers MiniPython's parser/compiler complexity guard against stack exhaustion. | The CPython method is `@support.cpython_only`; exact compiler-frame recursion depths and platform-specific crash-depth matrices are not a MiniPython contract. |
| `test_null_terminated` | `ported` | `cpython_compile_specifics_null_terminated_memoryview_subset` covers memoryview source objects, sliced memoryviews, eval/exec compile steps, and embedded-NUL rejection. | None for this method. |
| `test_merge_constants` | `blocked_by_cpython_internal` | The method is marked CPython-only and asserts constant-object sharing and peephole optimizer behavior. | MiniPython should preserve public values without copying CPython constant-merging internals. |
| `test_merge_code_attrs` | `blocked_by_cpython_internal` | The method is marked CPython-only and asserts `co_linetable` object sharing. | MiniPython has no CPython `co_linetable` object contract. |
| `test_remove_unused_consts` | `blocked_by_cpython_internal` | The method is marked CPython-only and asserts exact optimized `co_consts` tuple shape. | MiniPython should not be forced to match CPython constant pruning layout. |
| `test_remove_unused_consts_no_docstring` | `blocked_by_cpython_internal` | The method is marked CPython-only and asserts exact optimized `co_consts` tuple shape. | MiniPython should not be forced to match CPython constant pruning layout. |
| `test_remove_unused_consts_extended_args` | `blocked_by_cpython_internal` | The method is marked CPython-only and mixes exact `co_consts` layout with extended-argument stress. | MiniPython register bytecode has no CPython `EXTENDED_ARG` contract. |
| `test_strip_unused_None` | `blocked_by_cpython_internal` | The method is marked CPython-only and asserts exact absence/presence of `None` in `co_consts`. | MiniPython should not copy CPython constant-table pruning. |
| `test_peephole_opt_unreachable_code_array_access_in_bounds` | `blocked_by_cpython_internal` | The method is marked CPython-only and validates a CPython peephole optimizer memory-safety regression through `dis`. | Not a MiniPython language-surface requirement. |
| `test_docstring` | `ported` | `cpython_compile_specifics_docstring_optimize_subset` covers source and public-AST `compile(..., optimize=0/1/2)` behavior for function, class, and module `__doc__`, including f-strings and constant expressions that must not become docstrings. | None for this method's public behavior; CPython opcode/constant-table assertions remain classified separately. |
| `test_docstring_interactive_mode` | `ported` | `cpython_compile_specifics_docstring_optimize_subset` covers `single`-mode `compile(..., optimize=0/1/2)` behavior for function and class `__doc__`. | None for this method. |
| `test_docstring_omitted` | `blocked_by_cpython_internal` | The method is marked CPython-only and asserts `dis` output does not include `NOP`. | MiniPython should not reproduce CPython opcode/NOP layout. |
| `test_dont_merge_constants` | `ported` | `cpython_compile_specifics_dont_merge_constants_public_subset` checks distinct code-object identity, code-object inequality, return `repr()` parity, and type-sensitive `co_consts` distinctions for signed zero floats, int-vs-float tuple constants, str-vs-bytes constants, signed-zero complex constants, and set-membership constants. | None for the public behavior; CPython's peephole conversion of set membership constants to exact `frozenset` entries is not a MiniPython register-VM layout contract. |
| `test_path_like_objects` | `ported` | `cpython_compile_specifics_compile_filename_subset` covers filename objects whose `__fspath__()` returns `str` or `bytes`, rejects non-string path results, and propagates `__fspath__()` exceptions. | None for this method. |
| `test_stack_overflow` | `blocked_by_runtime` | MiniPython has static complexity guards for safe parser/compiler execution. | The CPython resource-heavy 100000-plus statement stress test belongs to a future sandbox resource-limit policy. |
| `test_dead_blocks_do_not_generate_bytecode` | `blocked_by_cpython_internal` | The method is marked CPython-only and asserts opcode count and final opcode shape. | MiniPython register bytecode optimization shape is separate. |
| `test_false_while_loop` | `blocked_by_cpython_internal` | The method asserts exact CPython disassembly length for unreachable loop bodies. | MiniPython should preserve behavior, not opcode count. |
| `test_consts_in_conditionals` | `blocked_by_cpython_internal` | The method asserts CPython optimizer removal through disassembly. | MiniPython can optimize differently while preserving expression semantics. |
| `test_imported_load_method` | `blocked_by_cpython_internal` | The method asserts `LOAD_ATTR` rather than `LOAD_METHOD` opcode selection. | MiniPython register bytecode has no CPython opcode contract. |
| `test_folding_type_param` | `blocked_by_cpython_internal` | The method asserts CPython `LOAD_SMALL_INT` and oparg folding inside type-parameter code objects. | MiniPython should track public type-parameter semantics separately from CPython opcode choices. |
| `test_lineno_procedure_call` | `ported` | `cpython_compile_specifics_lineno_procedure_call_subset` covers the public invariant that a multiline parenthesized procedure call does not report the opening-paren-only physical line through function `co_lines()`. | None for this method. |
| `test_lineno_after_implicit_return` | `ported` | `cpython_compile_specifics_lineno_after_implicit_return_subset` covers CPython's public `sys._getframe()` frame-line behavior after implicit returns from executed and skipped `if` bodies. | None for this method. |
| `test_lineno_after_no_code` | `ported` | `cpython_compile_specifics_lineno_after_no_code_first_pass_subset` covers the public invariant that no-code function bodies expose a single `__code__.co_lines()` span whose line equals `co_firstlineno`, source-token-derived `co_firstlineno` for later function definitions, plus matching `co_positions()` line/None-column shape. | None for MiniPython's public line-table surface; CPython's exact `end == len(code.co_code)` bytecode-bytes assertion is implementation-specific. |
| `test_lineno_attribute` | `ported` | `cpython_compile_specifics_lineno_attribute_subset` ports public `co_lines()` parity for multiline attribute load, method call, store, and augmented store forms. | None for this method. |
| `test_line_number_genexp` | `ported` | `cpython_compile_specifics_line_number_genexp_subset` ports the public nested generator-expression code-object `co_lines()` sequence exposed through the outer function's `co_consts`. | None for this method. |
| `test_line_number_implicit_return_after_async_for` | `ported` | `cpython_compile_specifics_line_number_implicit_return_after_async_for_subset` ports the public async-function `co_lines()` sequence for an implicit return after `async for`. | None for this method. |
| `test_line_number_synthetic_jump_multiple_predecessors` | `ported` | `cpython_compile_specifics_synthetic_jump_line_tables_subset` covers the public function `co_lines()` sequence for the try/loop/yield cold-block source shape. | None for MiniPython's public line-table surface; CPython `dis.Bytecode(...).positions` opcode-specific assertions are implementation-specific. |
| `test_line_number_synthetic_jump_multiple_predecessors_nested` | `ported` | `cpython_compile_specifics_synthetic_jump_line_tables_subset` covers the public function `co_lines()` sequence for the nested try/except cold-block source shape. | None for MiniPython's public line-table surface; CPython `dis.Bytecode(...).positions` opcode-specific assertions are implementation-specific. |
| `test_line_number_synthetic_jump_multiple_predecessors_more_nested` | `ported` | `cpython_compile_specifics_synthetic_jump_line_tables_subset` covers the public function `co_lines()` sequence for the deeper nested try/except cold-block source shape. | None for MiniPython's public line-table surface; CPython `dis.Bytecode(...).positions` opcode-specific assertions are implementation-specific. |
| `test_lineno_of_backward_jump_conditional_in_loop` | `ported` | `cpython_compile_specifics_lineno_of_backward_jump_conditional_in_loop_subset` covers the public function `co_lines()` loop-backedge line for a conditional inside a loop. | None for MiniPython's public line-table surface; CPython `dis.Bytecode(...).positions` opcode-specific assertions are implementation-specific. |
| `test_big_dict_literal` | `ported` | `cpython_compile_specifics_big_dict_literal_subset` evaluates the CPython 0xFFFF+1-entry dict display and preserves every key at runtime. | None for the public source behavior; the original compiler flushing boundary is covered through observable dict length rather than CPython bytecode internals. |
| `test_redundant_jump_in_if_else_break` | `blocked_by_cpython_internal` | The method asserts absence of next-instruction CPython jumps in disassembly. | MiniPython should not mirror CPython jump encoding. |
| `test_no_wraparound_jump` | `blocked_by_cpython_internal` | The method asserts absence of `EXTENDED_ARG` in CPython bytecode. | MiniPython register bytecode does not use CPython `EXTENDED_ARG`. |
| `test_uses_slice_instructions` | `blocked_by_cpython_internal` | The method is marked CPython-only and checks `BINARY_SLICE`, `STORE_SLICE`, `BUILD_SLICE`, and constant-table opcode use. | MiniPython slice semantics should be tested at the language level. |
| `test_compare_positions` | `blocked_by_cpython_internal` | Comparison grammar and AST/source-span behavior is covered elsewhere. | The CPython method asserts exact `dis.get_instructions()` comparison opcode positions; MiniPython's register bytecode has no CPython comparison-opcode position contract. |
| `test_if_expression_expression_empty_block` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` compiles the three empty-block conditional-expression assertion/function sources. | None for this method. |
| `test_multi_line_lambda_as_argument` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` compiles a multiline keyword argument whose value is a multiline lambda body. | None for this method. |
| `test_apply_static_swaps` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` checks duplicate target assignment `a, a = x, y` returns the rightmost stored value. | None for this method. |
| `test_apply_static_swaps_2` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` checks duplicate target assignment `a, b, a = x, y, z` returns the later `a` value. | None for this method. |
| `test_apply_static_swaps_3` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` checks duplicate target assignment `a, a, b = x, y, z` preserves the second `a` value. | None for this method. |
| `test_variable_dependent` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` checks dependent stores preserve `(54, 96)` for `a = 42; b = a + 54; a = 54`. | None for this method. |
| `test_duplicated_small_exit_block` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` compiles the while/try/except return control-flow regression. | None for this method. |
| `test_cold_block_moved_to_end` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` compiles the while/try/except/else cold-block regression. | None for this method. |
| `test_remove_empty_basic_block_with_jump_target_label` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` compiles the while-loop conditional-expression empty-block regression. | None for this method. |
| `test_remove_redundant_nop_edge_case` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` compiles the nested conditional-expression NOP regression source without asserting CPython NOP layout. | None for the public compile-stability surface. |
| `test_lineno_propagation_empty_blocks` | `ported` | `cpython_compile_specifics_lineno_propagation_empty_blocks_subset` covers the public function `co_lines()` sequence for the while/try/except/else empty-block smoke-test shape. | None for MiniPython's public line-table surface; CPython opcode/debug-position metadata remains outside MiniPython's register-bytecode contract. |
| `test_global_declaration_in_except_used_in_else` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` executes the combined `try` plus `except` and `except*` global declaration used from `else` shape. | None for this method. |
| `test_regression_gh_120225` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` compiles the async function containing `match`, f-string guard, async dict comprehension, and nested list expression. | None for this method. |
| `test_globals_dict_subclass` | `ported` | `cpython_compile_specifics_public_regression_shapes_subset` checks `exec()` with a dict subclass globals object leaves function global lookup behavior catchably missing. | None for this method. |
| `test_compile_warnings` | `ported` | `cpython_compile_specifics_runtime_warning_capture_subset` checks repeated runtime `compile()` warning emission, category identity, filename, and line numbers. | None for this method. |
| `test_compile_warning_in_finally` | `ported` | `cpython_compile_specifics_warning_in_finally_subset` checks warning de-duplication and line numbers for ordinary and `except*` finally paths. | None for this method. |
| `test_filter_syntax_warnings_by_module` | `ported` | `cpython_compile_specifics_filter_syntax_warnings_by_module_subset` checks CPython's `syntax_warnings.py` source shape through runtime `compile()`, including tokenizer, codegen/static, and finally-control-flow `SyntaxWarning` line numbers plus the public `module=` keyword. | None for this method. |
| `test_pep_765_warnings` | `ported` | `cpython_compile_specifics_pep_765_warning_subset` covers source and public-AST `compile()` warnings for return, break, and continue escaping `finally`. | None for this method. |
| `test_pep_765_no_warnings` | `ported` | `cpython_compile_specifics_pep_765_warning_subset` covers nested definition and nested loop cases inside `finally` that should not warn. | None for this method. |

Additional container runtime evidence:
`cpython_tuple_subclass_core_sequence_subset` covers the supported public
`tuple` subclass sequence protocol outside CPython's internal object layout:
construction from iterables, `isinstance`, truthiness, `len()`, iteration,
indexing, slicing, reversed iteration, default display through `repr()` /
`str()` / f-strings, empty instances, and constructor error paths.
`cpython_dict_subclass_core_mapping_subset` covers the supported public
`dict` subclass mapping protocol outside CPython's internal dict layout:
construction from mappings and pair iterables, item assignment/replacement and
deletion through subscript syntax, inherited `update()` / `get()`, key
iteration, membership, default display through `repr()` / `str()` / f-strings,
recursive storage display, and missing-key deletion errors.
`cpython_set_and_frozenset_subclass_subset` covers the supported public
`set` and `frozenset` subclass protocols outside CPython's internal object
layout: construction, membership, mutation and method result types, default
display through `repr()` / `str()` / f-strings for empty and non-empty
instances, direct `object.__format__` fallback display, non-empty format spec
rejection, custom `__format__` priority, frozenset hashing/copy identity,
custom `__new__`, slots, and ABC registration.

Additional `Lib/test/test_memoryview.py` evidence:
`cpython_memoryview_getbuf_fail_subset` ports
`AbstractMemoryTests::test_getbuf_fail`, covering public `TypeError`
rejection for non-buffer objects passed to `memoryview()`. Direct CPython diff
evidence is in `cpython_memoryview_getbuf_fail_diff_subset`.
`cpython_memoryview_hex_reentrant_release_subset` ports
`AbstractMemoryTests::test_hex_use_after_free`, covering released-view
`hex()` `ValueError` and bytearray exporter resize-locking while separator
conversion invokes Python code. Direct `cpython_memoryview_hex_released_view_diff_subset`
evidence covers released-view `hex()`, and gated direct
`cpython_memoryview_hex_reentrant_release_diff_subset` evidence covers the
re-entrant BufferError path for CPython oracles with the current fix.
`cpython_memoryview_weakref_live_subset` ports the live-reference portion of
`AbstractMemoryTests::test_weakref`, covering `weakref.ref(memoryview(...))`
construction, callback argument acceptance, `callback=None`, direct ref calls,
and `weakref.ReferenceType` classification.
`cpython_memoryview_bytesio_readinto_subset` ports the in-memory portion of
`AbstractMemoryTests::test_writable_readonly`, covering
`io.BytesIO.readinto()` writes to `bytearray` and bytearray-backed
`memoryview`, read-only bytes-backed target rejection, stream-position
advancement, and `initial_bytes=` construction.
`cpython_memoryview_array_b_buffer_subset` ports the public one-byte
`array.array('B')` exporter slice from CPython memoryview coverage, covering
writable `B`-format attributes, `obj` identity, `tolist()` / `tobytes()`,
scalar and same-size slice writeback into the original array, subview stride
preservation, and `toreadonly()` retaining the array exporter.
`cpython_memoryview_array_signed_byte_buffer_subset` ports the public signed
one-byte `array.array('b')` exporter slice from CPython memoryview coverage,
covering raw bytes/bytearray initialization, signed iterable initialization and
iteration, writable `b`-format attributes, signed `tolist()` / scalar getitem,
scalar write range/type errors, same-format memoryview slice assignment, and
structure mismatch errors for bytes and unsigned-byte views.
`cpython_memoryview_array_non_byte_public_read_subset` ports the public
one-dimensional non-byte numeric `array.array` exporter read slice from CPython
memoryview coverage, covering element-sized attributes, scalar getitem,
`tolist()`, logical `tobytes()`, slicing, contiguity, and byte casts for `h`,
`H`, `i`, `I`, `f`, and `d` formats.
`cpython_memoryview_array_non_byte_writeback_subset` ports the public
one-dimensional same-format non-byte numeric `array.array` exporter writeback
slice from CPython memoryview coverage, covering scalar item assignment,
contiguous and extended slice assignment, backing-array visibility, `__index__`
scalar conversion, and structure-mismatch rejection for bytes and differently
formatted memoryviews.
`cpython_array_module_and_constructor_public_surface_subset` adds direct public
`array` module and constructor evidence: `array.typecodes`, legacy typecode
construction, str-subclass typecode arguments, invalid typecode `ValueError`,
non-string and wrong-arity constructor `TypeError`s, keyword rejection, and
zero-length array self-slice / concat / repeat behavior.
`cpython_array_subclass_public_construction_subset` adds public `array.array`
subclass evidence: ordinary subclasses, custom `__init__`, custom `__new__`
via `array.array.__new__`, inherited storage-backed methods,
subclass-specific `repr()`, `isinstance()` / `issubclass()` relationships,
direct `array.array.__new__` allocation, and base-array `copy.copy()` results.
`cpython_array_one_byte_public_sequence_subset` adds direct public
`array.array('B')` / `array.array('b')` sequence evidence for the same
first-pass one-byte storage: `typecode`, `itemsize`, `len()`, truthiness,
`tolist()`, `tobytes()`, scalar indexing, slicing, `reversed()`, and direct
dunder method calls.
`cpython_array_short_public_sequence_and_mutation_subset` extends that public
array evidence to native-endian signed and unsigned short `h` / `H` storage,
covering two-byte `itemsize`, element-count sequence behavior, raw bytes
round-trips, mutation methods, `byteswap()`, concat/repeat, `__index__`
conversion, overflow errors, `fromfile()` short reads, and array-source
constructor conversion by public elements instead of raw bytes.
`cpython_array_int_public_sequence_and_mutation_subset` extends the same
public array evidence to fixed-width native-endian signed and unsigned int
`i` / `I` storage, covering four-byte `itemsize`, element-count sequence
behavior, raw bytes round-trips, mutation methods, `byteswap()`,
concat/repeat, `__index__` conversion, signed/unsigned overflow errors,
`fromfile()` short reads, and array-source constructor conversion by public
elements instead of raw bytes.
`cpython_array_long_long_public_sequence_and_mutation_subset` extends that
public array evidence to fixed-width native-endian signed and unsigned long
long `q` / `Q` storage, covering eight-byte `itemsize`, element-count sequence
behavior, raw bytes round-trips, mutation methods, `byteswap()`,
concat/repeat, `__index__` conversion, signed/unsigned overflow errors,
BigInt-backed unsigned values above `i64::MAX`, `fromfile()` short reads, and
array-source constructor conversion by public elements instead of raw bytes.
`cpython_array_native_long_public_sequence_and_mutation_subset` extends the
same evidence to platform-native C long signed and unsigned `l` / `L` storage,
covering native `itemsize`, element-count sequence behavior, raw bytes
round-trips, mutation methods, `byteswap()`, concat/repeat, `__index__`
conversion, signed/unsigned overflow errors, BigInt-backed unsigned values
above `i64::MAX` on 64-bit C long platforms, `fromfile()` short reads, and
array-source constructor conversion by public elements instead of raw bytes.
`cpython_array_float_public_sequence_and_mutation_subset` extends the same
evidence to native-endian float and double `f` / `d` storage, covering four-
and eight-byte `itemsize`, element-count sequence behavior, raw bytes
round-trips, mutation methods, `byteswap()`, concat/repeat, `fromfile()` short
reads, `__float__` conversion before `__index__` fallback, conversion error
propagation, and array-source constructor conversion by public elements instead
of raw bytes.
`cpython_array_unicode_public_sequence_and_mutation_subset` extends the same
evidence to native-endian Unicode `u` / `w` storage, covering four-byte
`itemsize`, string constructor initialization, `tolist()`, `tobytes()`,
`repr()`, `tounicode()`, scalar and slice access, mutation methods,
`fromunicode()`, raw byte round-trips, `byteswap()` invalid-code-point errors,
concat/repeat, `fromfile()` short reads, invalid item type errors, and
array-source constructor conversion by public elements instead of raw bytes.
`cpython_array_one_byte_public_mutation_methods_subset` adds the corresponding
mutable-method evidence for `append()`, `insert()`, `extend()`, `pop()`,
`reverse()`, `count()`, `index()`, `remove()`, `fromlist()`, `frombytes()`,
and `clear()`, including signed/unsigned byte overflow errors, same-kind array
`extend()` rejection, input type validation, and empty-array `repr()` parity.
`cpython_array_one_byte_public_subscript_mutation_subset` adds the matching
mutable-subscript evidence for scalar assignment, direct `__setitem__()`,
contiguous and extended slice assignment, same-kind array RHS validation,
extended-slice length errors, direct `__delitem__()`, contiguous and extended
slice deletion, and CPython's array assignment index error shape for supported
one-byte `B` / `b` arrays.
`cpython_array_one_byte_public_copy_byteswap_compare_subset` adds copy,
byteswap, and comparison evidence for supported one-byte arrays: `__copy__()`,
`__deepcopy__()`, `copy.copy()`, `copy.deepcopy()`, independent copied storage,
one-byte `byteswap()` no-op behavior, direct comparison dunders, numeric
element equality/order across `B` / `b`, and CPython's non-array comparison
`NotImplemented` / TypeError split.
`cpython_array_one_byte_public_concat_repeat_subset` adds concatenation and
repeat evidence for supported one-byte arrays: same-kind `+` / `__add__()`,
cross-kind and non-array rejection, `*` / reflected `*` / direct repeat
dunders, `__index__` repeat counts, zero/negative repeats, operator versus
direct-dunder non-integer diagnostics, and identity-preserving `+=` / `*=`.
`cpython_array_one_byte_public_buffer_info_subset` adds public `buffer_info()`
evidence for supported one-byte arrays, covering method visibility, tuple
shape, integer nonzero address surrogate, current element count after
mutation, bound-method dispatch, and extra-argument rejection without asserting
CPython's exact process-local buffer address.
`cpython_array_one_byte_public_unicode_method_rejection_subset` adds public
`fromunicode()` / `tounicode()` rejection evidence for supported one-byte
arrays, covering method visibility, non-unicode-array `ValueError`s, arity and
non-string argument TypeErrors, and receiver preservation after rejected calls.
`cpython_array_one_byte_public_file_methods_subset` adds public `tofile()` /
`fromfile()` evidence for supported one-byte arrays through `io.BytesIO`,
covering raw-byte writes, exact and short reads, partial append before
`EOFError`, zero-count reads, invalid counts, non-bytes `read()` results, and
the `BytesIO.read()` / `write()` / `getvalue()` methods needed for the public
file protocol.
The remaining currently unported direct methods are classified by dependency:
`test_getbuffer`, `test_refs`, `test_gc`, `test_buffer_reference_loop`,
`test_picklebuffer_reference_loop`, and `test_racing_getbuf_and_releasebuf`
depend on CPython refcount, cyclic-GC, `__buffer__`, `pickle.PickleBuffer`, or
host threading/shared-memory behavior; remaining `test_writable_readonly`
coverage beyond bytearray and memoryview targets depends on broader
`readinto()` buffer-protocol interop; `test_hash_use_after_free`,
`test_issue22668`, `test_array_assign`, and `test_half_float` require
array-backed behavior beyond the current `array('B')` / `array('b')` one-byte
writable slices or non-byte-format memoryviews.
`BaseArrayMemoryTests.test_getbuffer` and skipped
`BaseArrayMemoryTests.test_tolist` remain part of that broader array-backed
buffer gap.

Additional `Lib/test/test_weakref.py` evidence:
`cpython_weakref_ref_supported_target_matrix_subset` covers first-pass public
`weakref.ref()` construction behavior: unsupported built-in instances are
rejected, supported functions/classes/instances/sets/memoryviews/builtin type
objects are accepted, `__slots__ = ()` instances are rejected, `__weakref__`
slot instances are accepted, and keyword arguments to `weakref.ref()` are
rejected. `cpython_weakref_ref_callback_attribute_subset` covers public
`weakref.ref.__callback__` metadata for omitted callback, function callback,
and `callback=None`, plus readonly assignment behavior.
`cpython_weakref_ref_type_identity_subset` covers canonical
`weakref.ReferenceType` identity through `type(ref)`, `ref.__class__`,
`object.__getattribute__(ref, "__class__")`, and `isinstance(ref, type(ref))`.
`cpython_weakref_proxy_type_aliases_subset` covers public `weakref.ProxyType`
and `weakref.CallableProxyType` aliases, their `weakref` module/name/qualname
metadata, `weakref.ProxyTypes`, and matching `_weakref` aliases without
`_weakref.ProxyTypes`.
`cpython_weakref_proxy_live_forwarding_subset` covers first-pass live
`weakref.proxy()` behavior: construction through `weakref` / `_weakref`,
`ProxyType` versus `CallableProxyType` classification, `weakref.ProxyTypes`
membership, target `__class__` forwarding, attribute read/write/delete and
bound-method forwarding, subscript get/set/delete forwarding through
`__getitem__` / `__setitem__` / `__delitem__`, `operator.index()` /
`__index__` forwarding, `__bytes__` / `__dir__` forwarding, floor-division and
matrix-multiply special method forwarding including in-place variants,
`__iter__`, `__reversed__`, and `__bool__` forwarding, callable proxy
positional/keyword calls, built-in `list` subclass target truthiness, `len()`,
iteration, membership, method forwarding, item and slice mutation, and reversed
iteration, positional `callback=None`, keyword-argument rejection, and
unhashable proxy behavior. `cpython_list_subclass_core_sequence_subset` pins
the same supported list-subclass protocol outside weakref proxy forwarding,
including `repr()` / `str()` / f-string display and recursive storage.
`cpython_weakref_ref_live_repr_subset` covers public live `weakref.ref`
`repr()` / `str()` shape for ordinary instances, class objects, functions,
sets, and frozensets without asserting exact memory addresses.
`cpython_weakref_ref_dunder_methods_subset` covers direct public access to live
`weakref.ref` `__repr__`, `__str__`, `__hash__`, `__call__`, `__eq__`, and
`__ne__` methods without depending on CPython's internal method-wrapper type;
direct equality methods return `NotImplemented` for non-weakref operands like
CPython.
`cpython_weakref_ref_live_compare_hash_subset` covers live `weakref.ref`
equality/inequality through referent equality, callback-agnostic equality,
referent hash reuse, and same-live-target set/dict key behavior. Full
`test_weakref.py` parity remains future work for proxies, weakref registries,
dead-reference clearing, callback invocation, and GC interactions.

Additional `Lib/test/test_bytes.py` evidence:
`cpython_bytes_basics_and_ord_subset` ports `BaseBytesTest::test_basics` and
`test_ord`, covering exact builtin bytes/bytearray type identity through
`type()`, ordinary `__class__`, and `object.__getattribute__`, plus `ord()` over
one-byte bytes/bytearray slices for `[0, 65, 127, 128, 255]`.
`cpython_string_bytes_codec_subset` ports `BaseBytesTest::test_encoding` and
`test_decode`, covering bytes/bytearray construction from text under `utf-8`
and `utf-16`, Latin-1 strict encode errors and `ignore`, UTF-8 decode errors
and `ignore` through positional and keyword arguments, direct
`bytearray.decode()` success/error paths, and default UTF-8 decode.
`cpython_bytes_empty_sequence_index_subset` ports the public behavior of
`BaseBytesTest::test_empty_sequence`, covering empty bytes/bytearray length and
`IndexError` normalization for ordinary, `sys.maxsize`-sized, and arbitrary
large positive/negative subscript indices.
`cpython_bytes_length_constructor_boundary_subset` ports the public behavior of
`BaseBytesTest::test_from_int` and `test_from_ssize`, covering zero-filled
integer-length construction, string/buffer source construction, `__index__`
length conversion, and catchable `ValueError` / `OverflowError` classes at the
signed index-size boundaries. Direct CPython diff evidence is in
`cpython_bytes_length_constructor_boundary_diff_subset`.
`cpython_bytes_constructor_overflow_guard_subset` ports the public safety
contract of `BaseBytesTest::test_constructor_overflow`, covering
address-space-sized bytes/bytearray constructor lengths that must raise
catchable `OverflowError` or `MemoryError` instead of crashing or escaping
Python exception handling.
`cpython_bytes_check_encoding_errors_devmode_subset` ports the CPython `-X dev`
public behavior from `BaseBytesTest::test_check_encoding_errors`, covering
eager `LookupError` for invalid bytes/bytearray constructor and decode
`encoding` / `errors` names. CPython default-mode lazy validation of invalid
`errors` names remains a documented mode difference rather than being claimed as
normal-mode parity.
`cpython_bytes_iterable_constructor_subset` ports the public behavior of
`BaseBytesTest::test_from_iterable`, `test_from_tuple`, `test_from_list`, and
`test_from_index`, covering bytes/bytearray construction from `range`, range
iterators, set inputs, generators without `__length_hint__`, list/tuple inputs,
`__getitem__` sequences, valid/erroring `__index__` item conversion, and the
public `test_constructor_type_errors` / `test_constructor_value_errors` error
class matrix for source, encoding/errors, and out-of-byte-range iterable item
cases.
`cpython_bytes_hex_fromhex_subset`,
`cpython_bytes_hex_separator_boundaries_subset`, and
`cpython_bytes_hex_descriptor_error_messages_subset` port the public behavior of
`BaseBytesTest::test_fromhex`, `test_hex`, `test_hex_separator_basics`,
`test_hex_separator_five_bytes`, `test_hex_separator_six_bytes`, and current
CPython main `test_hex_simd_boundaries` / `test_hex_nibble_boundaries`, covering
bytes/bytearray `fromhex()` from string and bytes-like inputs including
`memoryview` and `array.array('B')`, CPython ASCII-whitespace skipping including
vertical tab, rejection of non-ASCII whitespace/input bytes, exact odd-hex-digit
and invalid-position diagnostics, `hex()` separator grouping, separator-byte
boundaries, `bytes_per_sep` `__index__` conversion,
catchable C-int overflow errors, and public `hex()` output correctness across
length and nibble boundary samples, plus exact unbound and invalid-receiver
`hex()` descriptor diagnostics.
Remaining non-portable `BaseBytesTest` rows are classified explicitly:
`test_memory_leak_gh_140939` depends on `_testcapi.PY_SSIZE_T_MAX` and CPython
allocation-leak accounting for percent-format width handling;
`test_free_after_iterating` delegates to `test.support.check_free_after_iterating`
and validates CPython object deallocation behavior; `test_sq_item` calls
`_testlimitedcapi.sequence_getitem`, so it is C API coverage rather than a
MiniPython language/runtime contract.
`cpython_bytes_percent_format_subset` and
`cpython_bytes_percent_format_dunder_bytes_errors_subset` port the public
behavior of `BaseBytesTest::test_mod` and `test_imod`, covering bytes/bytearray
old-style `%` formatting for `%b`, `%s`, `%d`, `%i`, `%u`, `%x`, `%X`, `%o`,
`%c`, `%f`, `%F`, `%e`, `%E`, `%g`, `%G`, literal percent escapes,
NUL-containing format strings, bytes mapping keys including keys with
parentheses, dynamic width/precision, receiver-driven result types, `%d` /
`%i` / `%u` `__int__` / `__index__` conversion, `%x` / `%X` / `%o` and `%c`
`__index__` conversion, float conversions through direct numbers and
successful `__float__` / `__index__` protocols, user `__getitem__` mapping
objects with bytes mapping keys and propagated lookup exceptions, memoryview
input for `%b` / `%s` with CPython's public memoryview rejection for `%c`, `%=`
rebinding, representative catchable error classes, `__bytes__` dispatch and
bad-result rejection, propagated `__bytes__` exceptions, non-ASCII `%r` / `%a`
repr escaping, and CPython public error ordering for mapping/dynamic-width
mixes, including missing bytes mapping keys preserving `KeyError.args`. The C
memory-leak stress case and full generic non-formatting `KeyError.args` parity
remain outside this public subset.
`cpython_bytes_percent_dunder_and_reentrant_bytearray_subset` extends that
coverage to direct `__mod__` descriptor calls, bytes/bytearray subclass
receivers returning base result types, method visibility, and
`ByteArrayTest::test_mod_concurrent_mutation` safety behavior where re-entrant
resizing during `%a` formatting raises `BufferError`.
`cpython_bytes_rmod_subset` ports the public behavior of
`BaseBytesTest::test_rmod`, covering catchable `TypeError` for unsupported
left operands in reflected modulo dispatch and `NotImplemented` return values
from direct bytes/bytearray `__rmod__` calls.
`cpython_bytes_search_methods_subset` also ports
the search/count side of `BaseBytesTest::test_none_arguments` and
`test_integer_arguments_out_of_byte_range`, covering `None` start/stop bounds
for `count()`, `find()`, `index()`, `rfind()`, and `rindex()`, plus
out-of-byte-range integer needles on both bytes and bytearray.
`cpython_bytes_prefix_suffix_typeerror_messages_subset` ports public
`BaseBytesTest` startswith/endswith TypeError diagnostics, covering the
CPython distinction between invalid top-level prefixes/suffixes and invalid
tuple candidates for both bytes and bytearray.
`cpython_bytes_search_prefix_suffix_error_messages_subset` ports
`BaseBytesTest::test_find_etc_raise_correct_error_messages`, covering the
public requirement that over-arity `TypeError` diagnostics match CPython's
exact search or prefix/suffix method text for both bytes and bytearray.
`cpython_bytes_replace_partition_methods_subset` ports
`BaseBytesTest::test_replace`, current CPython main
`test_replace_count_keyword`, `test_replace_int_error`, `test_partition`,
`test_rpartition`, `test_partition_string_error`, and
`test_partition_int_error`, covering bytes/bytearray replace result types,
positional and keyword `count`, bytes-like arguments, partition tuple shapes,
empty-separator `ValueError`, and representative public `TypeError` paths.
`cpython_bytes_method_typeerror_messages_subset` ports public `BaseBytesTest`
error-message rows for bytes/bytearray `split()` / `rsplit()`,
`partition()` / `rpartition()`, `strip()` / `lstrip()` / `rstrip()`, and
`center()` / `ljust()` / `rjust()` fill argument validation, pinning CPython's
exact `TypeError.args[0]` text for this method slice.
`cpython_bytes_more_method_typeerror_messages_subset` ports additional public
`BaseBytesTest` diagnostics for bytes/bytearray ASCII case and predicate
methods, `splitlines()`, `expandtabs()`, `zfill()`, `removeprefix()`, and
`removesuffix()`, pinning exact `TypeError.args[0]` text for unbound methods,
bound arity errors, and non-integer tabsize/width conversion.
`cpython_bytes_core_method_typeerror_messages_subset` ports additional public
`BaseBytesTest` diagnostics for bytes/bytearray `split()` / `rsplit()`, search,
prefix/suffix, strip, alignment, partition, and replace methods, pinning exact
`TypeError.args[0]` text for unbound methods, bound arity errors, slice-bound
conversion, maxsplit/width/count conversion, and partition arity.
`cpython_bytes_join_translate_maketrans_typeerror_messages_subset` ports public
`BaseBytesTest` diagnostics for bytes/bytearray `join()`, `translate()`, and
`maketrans()`, pinning exact `TypeError.args[0]` text for unbound descriptor
calls, bound and descriptor missing-argument calls, join over-arity, join
non-iterables, and no-argument `maketrans()` calls.
`cpython_bytearray_extend_subset` ports the public behavior of
`ByteArrayTest::test_extend`, covering self-extension, map and generator
inputs, all-or-nothing invalid item handling, `__index__` item conversion, and
bytearray-specific `TypeError` messages.
`cpython_bytes_mutating_list_constructor_subset` ports
`BaseBytesTest::test_from_mutating_list`, covering bytes and bytearray
construction from live lists whose items clear or append to the source list
during `__index__` conversion.
`cpython_bytes_buffer_constructor_subset` and
`cpython_bytes_array_array_buffer_subset` port the portable public part of
`BaseBytesTest::test_from_buffer`, covering bytes, bytearray, memoryview,
first-pass `array.array('B')`, and bytes-subclass constructor sources for both
bytes and bytearray, including fallback to bytes-like construction when a bytes
subclass `__index__` raises `TypeError`. The array-backed buffer slice also
covers `fromhex()`, search/replace, concat, membership, bytearray in-place
concat and slice assignment, while preserving CPython's distinction that
`bytes == array.array('B', ...)` is false and ordered comparison is unsupported.
Non-`B` array formats and full buffer-protocol matrices remain in the
source-group buffer protocol gap.
`cpython_bytearray_resize_subset` ports the public behavior of current CPython
`ByteArrayTest::test_resize`, covering truncation, zero-filled growth,
`__index__` length conversion, catchable public error classes, method
visibility, and sandbox-safe `MemoryError` behavior for impractically large
sizes.
`cpython_bytearray_resize_forbidden_subset` ports the public behavior of
current CPython `ByteArrayTest::test_resize_forbidden`, covering active
memoryview exports blocking all resizing bytearray operations before mutation.
`cpython_bytearray_alloc_and_subclass_mutation_subset` ports the public behavior
of `ByteArrayTest::test_alloc` and `test_init_alloc`, covering `__alloc__()`
visibility, empty allocation value, allocation-greater-than-length semantics
including generator-driven `bytearray.__init__()` intermediate mutation, plus
the public subclass branch of `test_resize` and inherited bytearray mutation
methods. Exact CPython allocator growth policy remains outside scope.
`cpython_bytearray_take_bytes_subset` ports the public behavior of current
CPython `ByteArrayTest::test_take_bytes`, covering whole-buffer and prefix
take-and-delete behavior, negative stop normalization, `None` stop,
`__index__` conversion, active memoryview exporter `BufferError`, public error
classes, inherited method dispatch for `bytearray` subclasses, and method
visibility. The
remaining `take_bytes` gaps are exact allocation-size accounting,
`sys.getsizeof()` parity, and CPython-only optimization checks.
`cpython_bytearray_iterator_length_hint_and_repeat_diff_subset` and
`cpython_bytearray_iterator_length_hint_and_repeat_regressions_subset` port the
public behavior of current CPython `ByteArrayTest::test_iterator_length_hint`
and `test_repeat_after_setslice`, covering bytearray iterator exhaustion after
clearing the original bytearray plus repetition after resizing slice
assignment. `cpython_bytearray_exhausted_iterator_diff_subset` and
`cpython_bytearray_exhausted_iterator_subset` port
`ByteArrayTest::test_exhausted_iterator`, covering the distinction between an
already exhausted bytearray iterator and a sibling iterator that can still
observe appended bytes, plus the no-crash exhausted-iterator regression.
`cpython_bytearray_mutating_index_conversion_diff_subset` and
`cpython_bytearray_mutating_index_safety_subset` port the Python-level public
behavior of current CPython `ByteArrayTest::test_mutating_index` and
`test_mutating_index_inbounds`, covering `__index__` conversion and reentrant
mutation safety during bytearray item/slice assignment and byte-valued mutation
methods. The reentrant safety rows stay as local runtime evidence because
default system CPython versions disagree on those crash-regression branches.
`_testlimitedcapi.sequence_setitem` branches remain classified as C API coverage
outside MiniPython's runtime surface.
`cpython_bytearray_search_reentrancy_buffererror_subset` ports the public
behavior of current CPython
`ByteArrayTest::test_search_methods_reentrancy_raises_buffererror`, covering
bytearray search methods, membership, `split()`, and `rsplit()` when `__buffer__`
argument conversion attempts to resize the locked receiver bytearray. This
current-CPython `__buffer__` behavior is kept in the subset suite rather than
the default system-CPython oracle.
`cpython_bytearray_extend_empty_buffer_overflow_subset` ports the public behavior
of current CPython `ByteArrayTest::test_extend_empty_buffer_overflow`, covering
`bytearray.extend()` over zero-length-hint iterators and catchable
`float(bytearray())` `ValueError` parsing failures while classifying the original
C allocation/NUL-termination regression as CPython-internal implementation
coverage. Capability-gated direct CPython diff evidence is in
`cpython_bytearray_extend_empty_buffer_overflow_diff_subset` when the selected
oracle has the fixed public behavior.
`cpython_bytearray_regexps_subset` ports the public behavior of
`ByteArrayTest::test_regexps`, covering the supported `re.findall()` bytes
pattern subset where ASCII `\w+` over bytes-like subjects returns ordinary
bytes matches.
`cpython_bytes_bytearray_subclass_basics_subset` ports the first public
bytes/bytearray subclass behavior from `BaseBytesTest::test_custom`,
`AssortedBytesTest`, and the module-level `BytesSubclass` /
`ByteArraySubclass` definitions, covering bytes-like construction,
`isinstance()` / `issubclass()`, `bytes()` conversion, length, and truthiness.
`cpython_bytes_bytearray_subclass_ops_and_join_subset` ports the public
`SubclassTest::test_basic` and `::test_join` behavior for bytes and bytearray
subclasses, covering comparison, concatenation, repetition, inherited method
visibility, and base-type `join()` result objects for single-item joins.
`cpython_bytes_bytearray_subclass_fromhex_subset` ports
`SubclassTest::test_fromhex`, covering subclass-preserving `fromhex()`
classmethods for bytes and bytearray subclasses, custom `__new__` / `__init__`
construction hooks, `bytes.__new__`, `bytearray.__new__`, and
`bytearray.__init__`.
`cpython_bytearray_subclass_init_override_subset` ports
`ByteArraySubclassTest::test_init_override`, covering custom bytearray subclass
`__init__` argument binding for positional and keyword `source` construction,
including an overridden initializer that leaves the bytearray storage empty.
`cpython_bytes_bytearray_subclass_copy_subset` ports
`SubclassTest::test_copy`, covering `copy.copy()` and `copy.deepcopy()` for
bytes and bytearray subclasses, with distinct top-level objects, preserved
concrete subclass types, copied user attributes, and nested subclass attribute
values.
`cpython_bytes_bytearray_subclass_pickle_subset` ports
`SubclassTest::test_pickle`, covering pickle round trips for bytes and
bytearray subclasses across supported protocols, preserving concrete subclass
types, value equality, user attributes, nested subclass attribute types, and
distinct restored objects through MiniPython's internal pickle payload.
`cpython_bytes_dunder_bytes_and_blocking_subset` ports
`BytesTest::test_bytes_blocking` plus related `BaseBytesTest::test_custom`
behavior, covering `bytes()` dispatch to `__bytes__`, bytes-subclass return
preservation, non-bytes return rejection, `__bytes__` precedence over
`__index__`, CPython's str-subclass and bytes-subclass `__bytes__`
regression cases from `BytesTest::test_custom`, explicit-encoding handling for
str subclasses, `str.__new__` returning concrete str subclasses, and
`__bytes__ = None` blocking for otherwise convertible objects.
`cpython_bytes_dunder_bytes_method_subset` ports `BytesTest::test__bytes__`,
covering direct `bytes.__bytes__()` calls on exact bytes and bytes subclasses,
exact bytes result type, value equality, exact bytes self identity,
bytes-subclass copy-out identity, inherited class descriptor calls, and `dir()`
visibility.
`cpython_bytes_repeat_id_preserving_subset` ports
`BytesTest::test_repeat_id_preserving`, covering exact bytes object identity
for repeat-by-one in both operand orders, empty bytes singleton identity, and
distinct exact bytes results for zero/negative/two repeats and bytes-subclass
repeat operations.
`cpython_bytes_bytearray_index_error_and_hash_subset` ports
`BytesTest::test_getitem_error` plus `ByteArrayTest::test_getitem_error`,
`test_setitem_error`, and `test_nohash`, covering public invalid-index
TypeError messages and bytearray's unhashable `TypeError`.
`cpython_bytes_bytearray_subclass_repr_and_compare_subset` extends that slice to
bytes subclass `repr()` / `str()`, bytearray subclass class-name repr, bytes-like
equality against builtin `bytes`, `bytearray`, and `memoryview`, plus bytewise
ordering for supported bytes-like values.
`cpython_bytes_bytearray_assorted_public_subset` ports
`AssortedBytesTest::test_from_bytearray` and
`test_compare_bytes_to_bytearray`, covering bytearray construction from a
memoryview-backed bytes object plus both operand orders for bytes/bytearray rich
comparison.
`cpython_bytes_warning_compare_subset` with direct
`cpython_bytes_warning_compare_diff_subset` evidence ports
`AssortedBytesTest::test_compare`,
covering `sys.flags.bytes_warning`, `BytesWarning` capture for bytes/string,
bytearray/string, and bytes/int equality comparisons, and `-bb` default
warning-as-error behavior.
`cpython_bytes_format_method_subset` ports the public behavior of
`AssortedBytesTest::test_format`, covering `format()` and direct `__format__()`
on bytes/bytearray for omitted and empty format specs, non-empty spec
`TypeError` rejection, and explicit `!s` f-string conversion continuing to use
ordinary string formatting.
`cpython_bytes_bytearray_type_doc_subset` ports
`AssortedBytesTest::test_doc`, covering public bytes and bytearray type
docstrings, constructor-signature prefixes, and `dir()` visibility.
`cpython_bytearray_hex_reentrant_separator_buffererror_subset` with gated direct
`cpython_bytearray_hex_reentrant_separator_buffererror_diff_subset` evidence
ports the public behavior of current CPython
`ByteArrayTest::test_hex_use_after_free`, covering bytearray `hex()`
resize-locking while a bytes-subclass separator runs re-entrant `__len__` code.
Older CPython oracles that still expose the accepted-and-cleared behavior are
explicitly skipped by the direct diff.
`cpython_bytearray_inplace_concat_repeat_subset` ports the public behavior of
`ByteArrayTest::test_iconcat`, `test_irepeat`, and `test_irepeat_1char`,
covering bytearray `+=`, `*=`, `__iadd__`, and `__imul__`
alias-preserving in-place mutation, bytes-like concat operands, repeat counts,
representative catchable `TypeError` paths, and same-object return behavior for
bytearray subclasses through inherited `__iadd__` / `__imul__`.
`cpython_bytearray_nonmutating_methods_copy_buffers_subset` ports the public
behavior of `ByteArrayTest::test_copied` and
`test_partition_bytearray_doesnt_share_nullstring`, covering independent
bytearray objects returned by non-mutating operations and absent-separator
partition/rpartition empty results.
`cpython_bytearray_pep3137_returns_new_copy_subset` ports
`BytearrayPEP3137Test::test_returns_new_copy` and the public behavior of
`AssortedBytesTest::test_return_self`, covering value-equal but distinct
bytearray results for no-op `zfill()`, `rjust()`, `ljust()`, `center()`,
`split()`, `rsplit()`, absent-separator `partition()` / `rpartition()`,
`splitlines()`, `replace(b'', b'')`, and one-item `join()`.
`cpython_bytes_pickle_roundtrip_subset` with direct
`cpython_bytes_pickle_roundtrip_diff_subset` evidence ports the public
value/type round-trip assertions from `BaseBytesTest::test_pickling` for
supported bytes and bytearray payloads. The remaining pickle gap in the
source-group row refers to subclass pickle behavior and CPython's real binary
pickle stream.
`cpython_bytes_iterator_pickle_roundtrip_subset` ports
`BaseBytesTest::test_iterator_pickling` for supported bytes and bytearray
iterators, covering initial and already-advanced iterator state across every
exposed pickle protocol.
`cpython_bytearray_iterator_pickle_shared_exporter_subset` with direct
`cpython_bytearray_iterator_pickle_shared_exporter_diff_subset` evidence ports
`ByteArrayTest::test_iterator_pickling2`, covering the relationship between a
pickled bytearray iterator and the copied mutable bytearray object for initial,
running, empty, and exhausted iterator states.

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
`cpython_ast_copy_replace_reject_unknown_instance_fields_exact_subset`,
`cpython_ast_copy_replace_accept_known_custom_class_fields_exact_subset`, and
`cpython_ast_copy_replace_non_str_kwarg_exact_subset` split all current CPython
copy methods out of broader first-pass coverage into direct method-level Rust
tests.

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
`cpython_ast_repr_full_snapshot_from_cpython_source_subset` port all current
`test_repr` snapshots by comparing against CPython's `data/ast_repr.txt`;
`cpython_ast_repr_large_input_crash_subset` ports the large-input repr
regression. Together these cover CPython-style structural `repr()` output for parsed
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
`test_pep758_except_star_without_parens`, and the full single-expression matrix
from `test_pep758_except_with_single_expr`.
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
mutated runtime `_fields` / `_attributes`, fieldless operator nodes, missing
runtime fields, and `compare_attributes=True`.
`cpython_ast_compare_literals_exact_subset` ports `test_compare_literals`,
covering CPython's full literal exact-type matrix for public `Constant` values.
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
`cpython_ast_snippets_exec_to_tuple_match_subset` extends that CPython
`snippets.py::exec_tests` public-AST evidence to the two match-statement
snapshots, pinning `Match`, `match_case`, `MatchValue`, `Constant`, `Pass`,
and wildcard `MatchAs` source locations plus compile-from-public-AST
round-trips.
`cpython_ast_snippets_exec_to_tuple_annotations_subset` extends the same
CPython `AST_Tests.test_snippets` evidence to module/class docstrings,
varargs, kwargs, unpacked vararg annotations, starred return annotations, and
all-parameter-kind function signatures.
`cpython_ast_snippets_exec_to_tuple_assignment_ops_subset` extends the
`snippets.py::exec_tests` public-AST evidence to annotated assignments with
starred generic annotations and all augmented-assignment operator singleton
nodes from `Add` through `FloorDiv`.
`cpython_ast_snippets_exec_to_tuple_assignment_targets_and_blocks_subset`
extends the same public-AST evidence to tuple/list/subscript assignment targets,
`for` / `while` `else` blocks, and CPython's nested-`If` representation for
`elif` chains.
`cpython_ast_snippets_exec_to_tuple_with_raise_assert_subset` extends the
`snippets.py::exec_tests` public-AST evidence to `withitem` variants,
parenthesized with-items, `Raise` exception/cause shapes, and assert messages.
`cpython_ast_snippets_exec_to_tuple_try_handlers_subset` extends it to `Try`,
`TryStar`, `ExceptHandler` names, `else` bodies, and `finally` bodies.
`cpython_ast_snippets_exec_to_tuple_positional_only_params_subset` extends it to
positional-only parameters, positional defaults, keyword-only default slots,
and `**kwargs` argument nodes. `cpython_ast_snippets_exec_to_tuple_type_params_subset`
extends it to PEP 695 `TypeAlias`, generic class definitions, generic function
definitions, and `TypeVar` / `TypeVarTuple` / `ParamSpec` metadata.
`cpython_ast_start_modes_public_to_tuple_subset` pins public root-node
`to_tuple()` shapes for `Expression`, `Interactive`, and `FunctionType`
across `eval`, `single`, and `func_type` parsing modes.
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
`cpython_ast_snippets_public_order_subset` ports the `_assertTrueorder`
invariant that CPython applies inside `AST_Tests.test_snippets`, checking
recursive source-position ordering, decorator-list ordering, public
`__match_args__` exposure on parser-built AST instances, and
compile-from-public-AST round-trips for the full current 219-case `exec`,
`single`, and `eval` snippet matrix. The same full matrix also ports
`AST_Tests.test_ast_validation` by running `ast.parse(..., optimize=False)` in
default `exec` mode for each snippet and compiling the resulting public AST.
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

## `Lib/test/test_compile.py::TestSourcePositions` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_simple_assignment` | `ported` | `cpython_compile_source_positions_code_positions_first_pass_subset` covers CPython's public AST-offset invariant for `x = 1`: the artificial module-start position may remain line 0, and the real assignment `co_positions()` tuple reports line 1 with columns inside the assignment statement span. | None for this method's public invariant; MiniPython still exposes fewer code-position tuples than CPython's opcode-level debug ranges. |
| `test_compiles_to_extended_op_arg` | `blocked_by_cpython_internal` | The method forces CPython `EXTENDED_ARG` pressure and asserts exact `BINARY_OP` source positions through `dis.Bytecode()`. | MiniPython's register bytecode has no CPython `EXTENDED_ARG` or `BINARY_OP` opcode-position contract. |
| `test_multiline_expression` | `blocked_by_cpython_internal` | The method asserts the exact CPython `CALL` opcode position for a multiline call expression. | Requires CPython opcode-level debug ranges. |
| `test_multiline_boolean_expression` | `blocked_by_cpython_internal` | The method is specialization-gated and asserts exact `POP_JUMP_IF_TRUE`, `POP_JUMP_IF_FALSE`, and `COMPARE_OP` opcode positions. | Requires CPython specialized opcode source ranges. |
| `test_multiline_assert` | `blocked_by_cpython_internal` | The method asserts exact `LOAD_COMMON_CONSTANT`, `LOAD_CONST`, `CALL`, and `RAISE_VARARGS` positions for a multiline assert. | Requires CPython opcode/debug-range metadata. |
| `test_multiline_generator_expression` | `blocked_by_cpython_internal` | The method asserts exact nested-code `YIELD_VALUE`, `JUMP_BACKWARD`, and `RETURN_VALUE` positions. | Requires CPython generator-code opcode source ranges. |
| `test_multiline_async_generator_expression` | `blocked_by_cpython_internal` | The method asserts exact async-generator nested-code `YIELD_VALUE` and `RETURN_VALUE` positions. | Requires CPython async-generator opcode source ranges. |
| `test_multiline_list_comprehension` | `blocked_by_cpython_internal` | The method asserts exact `LIST_APPEND` and `JUMP_BACKWARD` positions. | Requires CPython comprehension opcode source ranges. |
| `test_multiline_async_list_comprehension` | `blocked_by_cpython_internal` | The method executes an async function and asserts exact `LIST_APPEND`, `JUMP_BACKWARD`, and `RETURN_VALUE` positions. | Requires CPython async-comprehension opcode source ranges. |
| `test_multiline_set_comprehension` | `blocked_by_cpython_internal` | The method asserts exact `SET_ADD` and `JUMP_BACKWARD` positions. | Requires CPython set-comprehension opcode source ranges. |
| `test_multiline_async_set_comprehension` | `blocked_by_cpython_internal` | The method executes an async function and asserts exact `SET_ADD`, `JUMP_BACKWARD`, and `RETURN_VALUE` positions. | Requires CPython async set-comprehension opcode source ranges. |
| `test_multiline_dict_comprehension` | `blocked_by_cpython_internal` | The method asserts exact `MAP_ADD` and `JUMP_BACKWARD` positions. | Requires CPython dict-comprehension opcode source ranges. |
| `test_multiline_async_dict_comprehension` | `blocked_by_cpython_internal` | The method executes an async function and asserts exact `MAP_ADD`, `JUMP_BACKWARD`, and `RETURN_VALUE` positions. | Requires CPython async dict-comprehension opcode source ranges. |
| `test_matchcase_sequence` | `blocked_by_cpython_internal` | The method asserts exact `MATCH_SEQUENCE`, `UNPACK_SEQUENCE`, and `STORE_NAME` positions. | Requires CPython pattern-matching opcode source ranges. |
| `test_matchcase_sequence_wildcard` | `blocked_by_cpython_internal` | The method asserts exact `MATCH_SEQUENCE`, `UNPACK_EX`, and multiple `STORE_NAME` positions. | Requires CPython pattern-matching opcode source ranges. |
| `test_matchcase_mapping` | `blocked_by_cpython_internal` | The method asserts exact `MATCH_MAPPING`, `MATCH_KEYS`, and `STORE_NAME` positions. | Requires CPython pattern-matching opcode source ranges. |
| `test_matchcase_mapping_wildcard` | `blocked_by_cpython_internal` | The method asserts exact `MATCH_MAPPING`, `MATCH_KEYS`, and `STORE_NAME` positions for a `**rest` mapping pattern. | Requires CPython pattern-matching opcode source ranges. |
| `test_matchcase_class` | `blocked_by_cpython_internal` | The method asserts exact `MATCH_CLASS`, `UNPACK_SEQUENCE`, and `STORE_NAME` positions. | Requires CPython pattern-matching opcode source ranges. |
| `test_matchcase_or` | `blocked_by_cpython_internal` | The method asserts exact repeated `MATCH_CLASS` positions for an or-pattern. | Requires CPython pattern-matching opcode source ranges. |
| `test_very_long_line_end_offset` | `blocked_by_cpython_internal` | The method asserts the exact end-column offset on a CPython `CALL` instruction for a very long line. | Requires CPython opcode column metadata. |
| `test_complex_single_line_expression` | `blocked_by_cpython_internal` | The method asserts exact repeated `BINARY_OP` positions for a complex single-line expression. | Requires CPython opcode occurrence and source-range metadata. |
| `test_multiline_assert_rewritten_as_method_call` | `ported` | `cpython_compile_source_positions_multiline_assert_rewrite_subset` ports the public AST rewrite path: copy an assert location to a generated method call, fix missing locations, and compile the tree. | None for this method's public AST compile behavior. |
| `test_push_null_load_global_positions` | `blocked_by_cpython_internal` | The method asserts exact `LOAD_GLOBAL` positions for CPython push-null call shapes. | Requires CPython call-lowering opcode source ranges. |
| `test_attribute_augassign` | `blocked_by_cpython_internal` | The method asserts exact CPython `LOAD_ATTR` and `STORE_ATTR` source-position tuples for a multiline attribute augmented assignment. The portable public function `co_lines()` behavior for the same source family is covered by `cpython_compile_specifics_lineno_attribute_subset`. | Requires CPython opcode/debug-range metadata. |
| `test_attribute_del` | `blocked_by_cpython_internal` | The method asserts the exact CPython `DELETE_ATTR` source-position tuple for a multiline attribute deletion. The portable public function `co_lines()` behavior for the same source family is covered by `cpython_compile_specifics_lineno_attribute_subset`. | Requires CPython opcode/debug-range metadata. |
| `test_attribute_load` | `blocked_by_cpython_internal` | The method asserts the exact CPython `LOAD_ATTR` source-position tuple for a multiline attribute load. The portable public function `co_lines()` behavior for the same source family is covered by `cpython_compile_specifics_lineno_attribute_subset`. | Requires CPython opcode/debug-range metadata. |
| `test_attribute_store` | `blocked_by_cpython_internal` | The method asserts the exact CPython `STORE_ATTR` source-position tuple for a multiline attribute store. The portable public function `co_lines()` behavior for the same source family is covered by `cpython_compile_specifics_lineno_attribute_subset`. | Requires CPython opcode/debug-range metadata. |
| `test_method_call` | `blocked_by_cpython_internal` | The method asserts exact CPython `LOAD_ATTR` and `CALL` source-position tuples for a multiline method call. The portable public function `co_lines()` behavior for the same source family is covered by `cpython_compile_specifics_lineno_attribute_subset`. | Requires CPython opcode/debug-range metadata. |
| `test_weird_attribute_position_regressions` | `ported` | `cpython_compile_source_positions_weird_attribute_position_regressions_subset` covers the public safety invariant that every exposed function `co_positions()` tuple for unusual multiline attribute chains has non-`None` bounds and ordered start/end source coordinates. | None for this method's public invariant; MiniPython still exposes fewer function-body position tuples than CPython's opcode-level debug ranges. |
| `test_column_offset_deduplication` | `blocked_by_cpython_internal` | The method is `@support.cpython_only` and asserts distinct CPython nested code objects and exact `co_positions()` lists for identical source text at different columns. | Requires CPython code-object constant layout and debug-position deduplication behavior. |
| `test_load_super_attr` | `blocked_by_cpython_internal` | The method searches CPython nested code objects and asserts the exact `LOAD_GLOBAL` position for `super()`. | Requires CPython nested-code layout and opcode source ranges. |
| `test_lambda_return_position` | `ported` | `cpython_compile_source_positions_lambda_return_position_subset` covers CPython's public lambda snippets and asserts every exposed lambda `__code__.co_positions()` tuple stays on line 1 with columns inside the lambda body expression range. | None for this method's public invariant; MiniPython still exposes fewer function-body position tuples than CPython's opcode-level debug ranges. |
| `test_return_in_with_positions` | `blocked_by_cpython_internal` | The method asserts exact CPython `dis.get_instructions()` counts and positions for `LOAD_CONST None` and `RETURN_VALUE` around a `with` return. | Requires CPython with-statement opcode lowering and debug-position metadata. |

## `Lib/test/test_compile.py::TestBooleanExpression` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_short_circuit_and` | `ported` | `cpython_compile_boolean_expression_exact_subset` and differential case `boolean-expression-short-circuit-identity` check that `and` returns the short-circuit operand object and calls `__bool__` exactly once on evaluated operands. | None for this method. |
| `test_short_circuit_or` | `ported` | `cpython_compile_boolean_expression_exact_subset` and differential case `boolean-expression-short-circuit-identity` check that `or` returns the short-circuit operand object and calls `__bool__` exactly once on evaluated operands. | None for this method. |
| `test_compound` | `ported` | `cpython_compile_boolean_expression_exact_subset` and differential case `boolean-expression-short-circuit-identity` check mixed `and` / `or` chains preserve CPython operand identity and avoid redundant truthiness calls. | None for this method. |
| `test_exception` | `ported` | `cpython_compile_boolean_expression_exact_subset` and differential case `boolean-expression-short-circuit-identity` check truthiness exceptions propagate through direct `bool()` and boolean short-circuit evaluation. | None for this method. |

## `Lib/test/test_compile.py::TestStaticAttributes` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_basic` | `ported` | `cpython_compile_static_attributes_exact_subset` checks class `__static_attributes__` is a tuple containing sorted/deduplicated `self.a` and `self.b` Store targets while excluding reads. | None for this method. |
| `test_nested_function` | `ported` | `cpython_compile_static_attributes_exact_subset` checks nested functions contribute `self.<attr>` Store targets to the nearest enclosing class and ignore non-self stores. | None for this method. |
| `test_nested_class` | `ported` | `cpython_compile_static_attributes_exact_subset` checks nested classes collect their own `__static_attributes__` independently from the outer class. | None for this method. |
| `test_subclass` | `ported` | `cpython_compile_static_attributes_exact_subset` checks subclasses get their own `__static_attributes__` tuple rather than inheriting parent-collected attributes. | None for this method. |

## `Lib/test/test_compile.py::TestExpressionStackSize` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_and` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long repeated `and` expression source shape. | None for MiniPython's register-compiler stability surface. |
| `test_or` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long repeated `or` expression source shape. | None for MiniPython's register-compiler stability surface. |
| `test_and_or` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long mixed `and` / `or` expression source shape. | None for MiniPython's register-compiler stability surface. |
| `test_chained_comparison` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long chained-comparison source shape. | None for MiniPython's register-compiler stability surface. |
| `test_if_else` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long conditional-expression source shape. | None for MiniPython's register-compiler stability surface. |
| `test_binop` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long binary-operation source shape. | None for MiniPython's register-compiler stability surface. |
| `test_list` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long list-display source shape. | None for MiniPython's register-compiler stability surface. |
| `test_tuple` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long tuple-display source shape. | None for MiniPython's register-compiler stability surface. |
| `test_set` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long set-display source shape. | None for MiniPython's register-compiler stability surface. |
| `test_dict` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long dict-display source shape. | None for MiniPython's register-compiler stability surface. |
| `test_func_args` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long positional function-call source shape. | None for MiniPython's register-compiler stability surface. |
| `test_func_kwargs` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long keyword function-call source shape. | None for MiniPython's register-compiler stability surface. |
| `test_meth_args` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long positional method-call source shape. | None for MiniPython's register-compiler stability surface. |
| `test_meth_kwargs` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the long keyword method-call source shape. | None for MiniPython's register-compiler stability surface. |
| `test_func_and` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles a function body with repeated boolean expressions. | None for MiniPython's register-compiler stability surface. |
| `test_stack_3050` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the 3050-target unpack-assignment regression source. | None for MiniPython's register-compiler stability surface. |
| `test_stack_3050_2` | `ported` | `cpython_compile_expression_stack_size_shapes_subset` compiles the 3050-argument annotated-signature regression source. | None for MiniPython's register-compiler stability surface. |

## `Lib/test/test_compile.py::TestStackSizeStability` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_if` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `if` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_if_else` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `if` / `elif` / `else` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_try_except_bare` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated bare `try` / `except` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_try_except_qualified` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated qualified `except` plus `else` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_try_except_as` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `except ... as` plus `else` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_try_except_star_qualified` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated qualified `except*` plus `else` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_try_except_star_as` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `except* ... as` plus `else` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_try_except_star_finally` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `except*` plus `finally` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_try_finally` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `try` / `finally` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_with` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `with` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_while_else` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `while` / `else` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_for` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `for` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_for_else` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `for` / `else` snippets inside a generated function body. | None for MiniPython's register-compiler stability surface. |
| `test_for_break_continue` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated `for` snippets with `break`, `continue`, `elif`, and `else`. | None for MiniPython's register-compiler stability surface. |
| `test_for_break_continue_inside_try_finally_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated loop-control snippets nested inside `try` / `finally`. | None for MiniPython's register-compiler stability surface. |
| `test_for_break_continue_inside_finally_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated loop-control snippets inside a `finally` block. | None for MiniPython's register-compiler stability surface. |
| `test_for_break_continue_inside_except_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated loop-control snippets inside an `except` block. | None for MiniPython's register-compiler stability surface. |
| `test_for_break_continue_inside_with_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated loop-control snippets inside a `with` block. | None for MiniPython's register-compiler stability surface. |
| `test_return_inside_try_finally_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated return snippets nested inside `try` / `finally`. | None for MiniPython's register-compiler stability surface. |
| `test_return_inside_finally_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated return snippets inside a `finally` block. | None for MiniPython's register-compiler stability surface. |
| `test_return_inside_except_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated return snippets inside an `except` block. | None for MiniPython's register-compiler stability surface. |
| `test_return_inside_with_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated return snippets inside a `with` block. | None for MiniPython's register-compiler stability surface. |
| `test_async_with` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated async-function snippets containing `async with`. | None for MiniPython's register-compiler stability surface. |
| `test_async_for` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated async-function snippets containing `async for`. | None for MiniPython's register-compiler stability surface. |
| `test_async_for_else` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated async-function snippets containing `async for` / `else`. | None for MiniPython's register-compiler stability surface. |
| `test_for_break_continue_inside_async_with_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated async-function snippets with loop control inside `async with`. | None for MiniPython's register-compiler stability surface. |
| `test_return_inside_async_with_block` | `ported` | `cpython_compile_stack_size_stability_control_flow_subset` compiles repeated async-function snippets with `return` inside `async with`. | None for MiniPython's register-compiler stability surface. |

## `Lib/test/test_compile.py::TestInstructionSequence` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_basics` | `blocked_by_cpython_internal` | The class is `@support.cpython_only` and skipped without `_testinternalcapi`; this method constructs `_testinternalcapi.new_instruction_sequence()` objects, labels, CPython opcode numbers, and expected jump targets. | Requires CPython's internal instruction-sequence builder and opcode metadata, not MiniPython language semantics. |
| `test_nested` | `blocked_by_cpython_internal` | The class is `@support.cpython_only` and skipped without `_testinternalcapi`; this method asserts nested CPython instruction-sequence storage through `add_nested()` / `get_nested()`. | Requires CPython's internal instruction-sequence object model. |
| `test_static_attributes_are_sorted` | `blocked_by_cpython_internal` | The class is `@support.cpython_only` and skipped without `_testinternalcapi`; this method observes class `__static_attributes__` ordering through CPython's internal instruction-sequence test class. MiniPython's public `__static_attributes__` behavior is already covered by `TestStaticAttributes`. | Requires CPython's internal test harness shape; the portable public class behavior is tracked separately. |

## `Lib/test/test_builtin.py::BuiltinTest Eval/Exec Method Audit`

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_eval` | `ported` | `cpython_eval_builtin_subset` covers expression evaluation, explicit globals/locals, UTF-8 BOM bytes, non-ASCII strings, argument errors, syntax errors, and locals mapping exception propagation. | None for this method. |
| `test_eval_kwargs` | `ported` | `cpython_eval_builtin_subset` covers `source=`, `globals=`, and `locals=` keyword binding, including the CPython behavior where `locals=` alone keeps the current globals mapping. | None for this method. |
| `test_general_eval` | `ported` | `cpython_eval_builtin_subset` covers general mapping locals, `__getitem__`, `keys()` / `dir()`, `globals()`, `locals()`, dict subclasses, nested spreadsheet-style lookup, and invalid mapping shapes. | None for this method. |
| `test_exec` | `ported` | `cpython_exec_builtin_subset` covers string and code-object execution in current scope, dict-backed globals/locals, global declarations split from locals, byte sources, and error paths. | None for this method. |
| `test_exec_kwargs` | `ported` | `cpython_exec_builtin_subset` covers `source=`, `globals=`, and `locals=` keyword binding, including global assignment behavior when only `locals=` is supplied. | None for this method. |
| `test_exec_globals` | `ported` | `cpython_eval_exec_builtins_mapping_subset` covers empty `__builtins__` missing `print` as catchable `NameError` and non-mapping `__builtins__` as `TypeError`. | None for this method. |
| `test_exec_globals_frozen` | `ported` | `cpython_eval_exec_builtins_mapping_subset` covers read-only builtins writes, missing `__build_class__`, custom read-only builtins providing `__build_class__`, empty read-only builtins, and read-only globals writeback. | None for this method. |
| `test_exec_globals_error_on_get` | `ported` | `cpython_eval_exec_builtins_mapping_subset` covers dict-subclass globals and builtins whose `__getitem__` raises a custom exception. | None for this method. |
| `test_exec_globals_dict_subclass` | `ported` | `cpython_eval_exec_builtins_mapping_subset` and differential case `exec-eval-builtins-mapping` cover dict-subclass builtins lookup success and missing-name `NameError`. | None for this method. |
| `test_eval_builtins_mapping` | `ported` | `cpython_eval_exec_builtins_mapping_subset` covers exact-dict `MappingProxyType` builtins mappings for eval success and missing-name `NameError`. | None for this method. |
| `test_exec_builtins_mapping_import` | `ported` | `cpython_eval_exec_builtins_mapping_subset` and differential case `exec-eval-builtins-mapping` cover missing `__import__` under mappingproxy builtins and custom `__import__` binding for import statements. | None for this method. |
| `test_eval_builtins_mapping_reduce` | `ported` | `cpython_eval_exec_builtins_mapping_subset` covers list/tuple iterator `__reduce__()` through mappingproxy builtins, including the `(iter, (sequence,), index)` result and empty builtins mapping `AttributeError`. | Host CPython version differences remain isolated from differential parity for this method. |
| `test_exec_redirected` | `ported` | `cpython_eval_exec_builtins_mapping_subset` covers `sys.stdout = None` while `exec('a')` still raises a catchable `NameError` instead of an internal error. | None for this method. |
| `test_exec_closure` | `ported` | `cpython_exec_closure_subset` covers executable function `__code__`, `co_freevars`, function `__closure__`, manual `types.CellType` cells, and TypeError paths for invalid closure use. | Host CPython version differences keep this method out of differential parity. |
| `test_exec_filter_syntax_warnings_by_module` | `ported` | `cpython_exec_filter_syntax_warnings_by_module_subset` executes the CPython `syntax_warnings.py` source shape through `exec()`, captures the six `SyntaxWarning` records under the default `<string>` module, and proves explicit globals `__name__` drives warning module filtering while `wm.filename` remains `<string>`. | None for this method. |

## `Lib/test/test_builtin.py::BuiltinTest Core Runtime Method Audit`

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_abs` | `ported` | `cpython_abs_builtin_subset` covers int, bool, float, complex, custom `__abs__`, and TypeError paths. | None for this method. |
| `test_all` | `ported` | `cpython_all_any_builtin_subset` covers truthy/falsy iterables, empty iterables, short-circuiting, generator input, RuntimeError propagation, non-iterable rejection, and arity errors. | None for this method. |
| `test_any` | `ported` | `cpython_all_any_builtin_subset` covers falsy/truthy iterables, empty iterables, short-circuiting, generator input, RuntimeError propagation, non-iterable rejection, and arity errors. | None for this method. |
| `test_all_any_tuple_list_set_optimization` | `ported` | `cpython_builtin_generator_dynamic_lookup_subset` covers the portable public behavior: `all`, `any`, `tuple`, `list`, and `set` are resolved dynamically when globals and the builtins module are overwritten. | None for the public builtin lookup semantics; CPython's generator `co_consts` de-duplication assertion is optimizer/code-object-internal and remains outside MiniPython's public runtime model. |
| `test_ascii` | `partial` | `cpython_ascii_builtin_subset` covers empty/basic values, recursive list/dict rendering, scalar Unicode escaping, f-string `!a`, and non-ASCII printable escaping. | Lone surrogate storage/rendering remains future Unicode-runtime work because MiniPython strings currently use Rust scalar values. |
| `test_neg` | `ported` | `cpython_builtin_negation_sys_maxsize_subset` covers the `-sys.maxsize - 1` integer boundary, `isinstance(..., int)`, and negation back to `sys.maxsize + 1`. | None for this method. |
| `test_callable` | `ported` | `cpython_attribute_introspection_builtins_subset` covers ordinary callables, classes, bound methods, class-level `__call__`, inherited `__call__`, and ignored instance-level `__call__`. | None for this method. |
| `test_chr` | `ported` | `cpython_chr_ord_builtin_subset` covers ordinary characters, Unicode scalar boundaries through `0x10ffff`, bool/int conversion behavior, and TypeError/ValueError paths. | None for this method. |
| `test_cmp` | `ported` | `cpython_builtin_cmp_absent_subset` and `cpython_builtin_cmp_absent_diff_subset` prove `builtins.cmp` is absent, attribute access raises `AttributeError`, and bare `cmp(1, 2)` raises `NameError`. | None for this method. |
| `test___ne__` | `ported` | `cpython_builtin_none_ne_direct_subset` covers direct `None.__ne__` behavior and `NotImplemented` fallback for unrelated objects. | None for this method. |
| `test_divmod` | `ported` | `cpython_divmod_builtin_subset` covers signed integer division, the `-sys.maxsize-1` boundary, float quotient/remainder signs, bool operands, mixed int/float operands, arity errors, TypeError, and zero-division paths. | None for this method. |
| `test_hash` | `ported` | `cpython_hash_builtin_subset` and `cpython_hash_id_builtins_diff_subset` cover portable hash invariants for `None`, numbers, strings/bytes, tuples, functions, unhashable containers, oversized integer `__hash__`, non-integer `__hash__` rejection, and the int-subclass `__hash__` self-return branch. | None for this method. |
| `test_invalid_hash_typeerror` | `ported` | `cpython_hash_builtin_subset` and `cpython_hash_id_builtins_diff_subset` cover the regression shape where `__hash__()` returns a non-integer and `hash()` raises `TypeError`. | None for this method. |
| `test_hex` | `ported` | `cpython_integer_base_builtins_subset` covers positive/negative `hex()` rendering, bool input, custom `__index__`, and TypeError paths. | None for this method. |
| `test_id` | `ported` | `cpython_id_builtin_subset` and `cpython_hash_id_builtins_diff_subset` cover stable object identity relationships and process-specific integer return typing for singleton, scalar, tuple, list, and dict objects. | None for this method. |
| `test_len` | `ported` | `cpython_len_builtin_subset` covers strings, tuples, lists, dicts, custom `__len__`, propagated exceptions, non-integer/float/negative/overflow returns, missing `__len__`, and arity errors. | None for this method. |
| `test_next` | `ported` | `cpython_iter_next_builtin_subset` covers range and custom iterators, repeated exhaustion, default values, generator exhaustion, and selected iterator re-entrancy/exhaustion cases. | None for this method. |
| `test_oct` | `ported` | `cpython_integer_base_builtins_subset` covers positive/negative `oct()` rendering, bool input, custom `__index__`, and TypeError paths. | None for this method. |
| `test_ord` | `ported` | `cpython_chr_ord_builtin_subset` covers one-character strings, one-byte bytes/bytearray objects, Unicode scalar boundaries through `0x10ffff`, and TypeError paths. | None for this method. |
| `test_pow` | `ported` | `cpython_pow_builtin_subset` covers integer and float powers, negative integer exponents, negative-real fractional powers returning complex results, the shared `**` behavior, three-argument modular pow, keyword arguments, `mod=None`, `functools.partial(pow, ...)` keyword-shape assertions, zero division, invalid modulus, and TypeError/ValueError paths. | None for this method. |
| `test_repr` | `ported` | `cpython_repr_builtin_subset` covers primitive/container repr, recursive list/dict placeholders, custom `__repr__`, blocked `__repr__`, and non-string `__repr__` rejection. `cpython_object_repr_str_direct_subset` covers direct `object.__repr__` / `object.__str__` descriptor lookup and calls, inherited bindings, generic object display, `object.__str__` delegation to `__repr__`, raw direct-call non-string return behavior, container subclass display, and arity/keyword errors. `cpython_str_builtin_custom_dunder_subset` covers ordinary `str()` / `print()` / f-string `!s` / default f-string / string `%s` dispatch through class-level `__str__`, non-string result rejection, propagated exceptions, instance-level `__str__` bypass, and `str` subclass empty-format behavior. | None for this method. |
| `test_repr_blocked` | `ported` | `cpython_repr_builtin_subset` covers `__repr__ = None` causing `repr(instance)` to raise `TypeError`. | None for this method. |
| `test_round` | `ported` | `cpython_round_builtin_subset` covers float and int rounding, half-even behavior, negative `ndigits`, keyword arguments, custom `__round__`, missing/instance-level `__round__` rejection, and TypeError paths. | None for this method. |
| `test_round_large` | `ported` | `cpython_round_builtin_subset` covers integral floats around `5e15` remaining unchanged. | None for this method. |
| `test_bug_27936` | `ported` | `cpython_round_builtin_subset` covers `round(x, None)` matching no-argument `round(x)` with matching result types for int, float, `decimal.Decimal`, and `fractions.Fraction`. | None for this method. |
| `test_format` | `ported` | `cpython_format_builtin_and_custom_dunder_format_subset` covers basic `format()`, empty format specs for the builtin values used by CPython's method, inherited custom class `__format__`, class-level lookup over instance attributes, derived-`str` format-spec objects passed to builtin and user `__format__` paths, empty object format, wrong-result TypeError, non-string spec TypeError, `object.__format__` argument rejection, and non-empty object format rejection. `cpython_str_builtin_custom_dunder_subset` adds empty `object.__format__(..., "")` dispatch through class-level `__str__` plus custom `__format__` precedence over `__str__`. | None for this method. |
| `test_bin` | `ported` | `cpython_integer_base_builtins_subset` covers zero/positive/negative `bin()` rendering, large integer boundaries, bool input, custom `__index__`, and TypeError paths. | None for this method. |

## `Lib/test/test_builtin.py::BuiltinTest Attribute/Introspection Method Audit`

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_delattr` | `ported` | `cpython_attribute_introspection_builtins_subset` covers module attribute deletion, builtin arity errors, non-string attribute-name rejection, and instance deletion through the shared attribute helper surface. | None for this method. |
| `test_dir` | `ported` | `cpython_vars_dir_builtin_subset` covers local-scope names, module names, `types.ModuleType` subclass instances with invalid visible `__dict__`, type and instance names, custom `__dir__` returning list/tuple/set values, invalid `__dir__` results, `object.__dir__()` parity for lists, slot-only instances, the CPython `__slots__ = ["__class__", "__dict__"]` shadowing behavior where `object.__dir__()` only merges class names when the visible `__class__` value is itself a class, and traceback `dir()` exposing exactly `tb_frame`, `tb_lasti`, `tb_lineno`, and `tb_next`. | None for this method. |
| `test_getattr` | `partial` | `cpython_attribute_introspection_builtins_subset` covers `sys.stdout` lookup, missing attributes, default values, builtin arity errors, non-string attribute-name rejection, and the maximum valid Unicode scalar attribute name. | Lone-surrogate attribute names remain future Unicode-runtime work because MiniPython strings currently use Rust scalar values. |
| `test_hasattr` | `ported` | `cpython_attribute_introspection_builtins_subset` covers present/missing module attributes, maximum-valid-scalar attribute names, non-string attribute-name rejection, and propagation of non-`AttributeError` exceptions from `__getattr__`. | None for this method. |
| `test_isinstance` | `ported` | `cpython_isinstance_builtin_subset` covers direct and inherited user-class instances, tuple classinfo, builtin scalar hierarchy checks, exception hierarchy checks, and TypeError paths for invalid classinfo. | None for this method. |
| `test_issubclass` | `ported` | `cpython_issubclass_builtin_subset` covers direct and inherited user classes, tuple classinfo, builtin scalar hierarchy checks, exception hierarchy checks, and TypeError paths for invalid arguments. | None for this method. |
| `test_setattr` | `ported` | `cpython_attribute_introspection_builtins_subset` covers module and instance attribute assignment, class attribute assignment through instance lookup, builtin arity errors, non-string attribute-name rejection, and attribute-write failure on immutable scalar values. | None for this method. |
| `test_type` | `ported` | `cpython_type_builtin_subset` covers one-argument `type()` over strings, tuples, scalars, containers, functions, classes, and instances, plus supported three-argument dynamic class construction and public error paths. | None for this method. |
| `test_vars` | `ported` | `cpython_vars_dir_builtin_subset` covers local `vars()`, module `vars()`, live module dictionaries, class and instance dictionaries, wrong-arity errors, missing-`__dict__` rejection, and property-backed `__dict__` access. | None for this method. |

## `Lib/test/test_builtin.py::BuiltinTest Aggregate Builtins Method Audit`

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_max` | `ported` | `cpython_min_max_sum_builtin_subset` covers string/list/tuple/positional inputs, mixed int/float comparisons, empty iterable errors, bad sequence propagation, `key=`, `key=None`, `default=`, and representative keyword/type errors. | None for this method. |
| `test_min` | `ported` | `cpython_min_max_sum_builtin_subset` covers string/list/tuple/positional inputs, mixed int/float comparisons, empty iterable errors, bad sequence propagation, `key=`, `key=None`, `default=`, and representative keyword/type errors. | None for this method. |
| `test_sum` | `ported` | `cpython_min_max_sum_builtin_subset` covers empty and integer sums, iterator and sequence-protocol inputs, list concatenation with an explicit start value, large integer starts, bool sums, float sums, negative-zero float rendering, infinity results checked through `math.isinf()`, `OverflowError` for huge integers mixed with float/complex starts, complex-constructor summation, complex signed-zero preservation, rejected string/bytes/bytearray/dict/list starts, keyword errors, and bad sequence propagation. | None for this method. |
| `test_sum_accuracy` | `blocked_by_cpython_internal` | None. | CPython marks this `@support.cpython_only` and validates a compensated summation algorithm whose exact rounding behavior other implementations may choose differently. MiniPython should cover public `sum()` type/error behavior and its own numeric accuracy policy, not CPython's internal algorithm choice. |

## `Lib/test/test_builtin.py::BuiltinTest Iterator Builtins Method Audit`

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_filter` | `ported` | `cpython_map_filter_builtin_subset` covers `filter()` over strings, lists, tuples, sequence-protocol objects, truth filtering with `None`, callable predicate filtering, arity errors, non-callable predicates, non-iterable inputs, predicate arity errors, and source iterator exception propagation. | None for this method. |
| `test_filter_pickle` | `ported` | `cpython_builtin_iterator_pickle_subset` covers filter iterator round trips, resumed already-advanced filter pickles, type preservation, and public value preservation across every exposed MiniPython pickle protocol. | None for this method's public iterator semantics; MiniPython does not claim CPython binary pickle byte compatibility. |
| `test_filter_dealloc` | `blocked_by_cpython_internal` | None. | CPython marks this as a resource-heavy recursive deallocation / thrashcan regression for deeply nested filter objects and `gc.collect()`. MiniPython should cover stack-safe iterator cleanup through its own runtime model rather than copying CPython's deallocator shape. |
| `test_iter` | `ported` | `cpython_iter_next_builtin_subset` covers `iter()` over tuples, lists, strings, sequence-protocol objects, callable-sentinel iterators, generator and enumerate exhaustion, arity errors, and sink-state behavior after exhaustion. | None for this method. |
| `test_map` | `ported` | `cpython_map_filter_builtin_subset` covers `map()` over one, two, and three iterables, nested map calls, sequence-protocol objects, shortest-input truncation, arity errors, non-iterable inputs, non-callable functions, and mapped-function exception propagation. | None for this method. |
| `test_map_pickle` | `ported` | `cpython_builtin_iterator_pickle_subset` covers map iterator round trips, resumed already-advanced map pickles, type preservation, and public value preservation across every exposed MiniPython pickle protocol. | None for this method's public iterator semantics; MiniPython does not claim CPython binary pickle byte compatibility. |
| `test_map_pickle_strict` | `ported` | `cpython_builtin_iterator_pickle_subset` covers strict map iterator round trips for equal-length inputs across every exposed MiniPython pickle protocol. | None for this method's public iterator semantics. |
| `test_map_pickle_strict_fail` | `ported` | `cpython_builtin_iterator_pickle_subset` covers strict map failure preservation before and after pickle restore, including the already-yielded prefix before `ValueError`. | None for this method's public iterator semantics. |
| `test_map_strict` | `ported` | `cpython_map_strict_builtin_subset` covers strict map equal-length output, shorter/longer argument `ValueError` cases, multi-argument diagnostics, keyword rejection, and strict-mode mismatch handling with ordinary objects. | None for this method. |
| `test_map_strict_iterators` | `ported` | `cpython_map_strict_builtin_subset` covers strict map iterator consumption side effects after a length mismatch, including the surviving positions of the longer iterators. | None for this method. |
| `test_map_strict_error_handling` | `ported` | `cpython_map_strict_builtin_subset` covers strict map error ordering for iterators that raise custom exceptions versus length-mismatch `ValueError`, preserving the yielded prefix. | None for this method. |
| `test_map_strict_error_handling_stopiteration` | `ported` | `cpython_map_strict_builtin_subset` covers strict map conversion of early `StopIteration` from participating iterators into length-mismatch `ValueError`, preserving the yielded prefix. | None for this method. |
| `test_zip` | `ported` | `cpython_enumerate_zip_sorted_builtin_subset` covers `zip()` over tuples, lists, sequence-protocol objects, zero inputs, star-unpacked empty inputs, truncation to shortest input, non-iterable errors, constructor-time iterator exception propagation, and avoiding length preallocation from unsized sequences. | None for this method. |
| `test_zip_pickle` | `ported` | `cpython_builtin_iterator_pickle_subset` covers zip iterator round trips, resumed already-advanced zip pickles, type preservation, and public value preservation across every exposed MiniPython pickle protocol. | None for this method's public iterator semantics; MiniPython does not claim CPython binary pickle byte compatibility. |
| `test_zip_pickle_strict` | `ported` | `cpython_builtin_iterator_pickle_subset` covers strict zip iterator round trips for equal-length inputs across every exposed MiniPython pickle protocol. | None for this method's public iterator semantics. |
| `test_zip_pickle_strict_fail` | `ported` | `cpython_builtin_iterator_pickle_subset` covers strict zip failure preservation before and after pickle restore, including the already-yielded prefix before `ValueError`. | None for this method's public iterator semantics. |
| `test_zip_bad_iterable` | `ported` | `cpython_bad_iterable_exception_identity_subset` covers preserving the exact exception object raised by a failing `__iter__` through `zip()`. | None for this method. |
| `test_zip_strict` | `ported` | `cpython_zip_strict_builtin_subset` covers strict zip equal-length output, shorter/longer argument `ValueError` cases, multi-argument diagnostics, and keyword rejection. | None for this method. |
| `test_zip_strict_iterators` | `ported` | `cpython_zip_strict_builtin_subset` covers strict zip iterator consumption side effects after a length mismatch, including the surviving positions of the longer iterators. | None for this method. |
| `test_zip_strict_error_handling` | `ported` | `cpython_map_strict_builtin_subset` and `cpython_zip_strict_builtin_subset` cover the same strict length/error ordering model for iterator exceptions versus length-mismatch `ValueError`, with direct zip strict length diagnostics and iterator consumption evidence. | None for this method's public behavior. |
| `test_zip_strict_error_handling_stopiteration` | `ported` | `cpython_zip_strict_builtin_subset` covers strict zip conversion of early `StopIteration` into length-mismatch `ValueError`; the map strict matrix keeps the yielded-prefix error-ordering cases aligned with CPython's shared strict iterator semantics. | None for this method's public behavior. |
| `test_zip_result_gc` | `blocked_by_cpython_internal` | None. | CPython marks this `@support.cpython_only`; it validates tuple reuse and `gc.is_tracked()` interaction for CPython's zip speed optimization. MiniPython should test public `zip()` values and iterator lifetime safety, not CPython GC tracking internals. |

## `Lib/test/test_builtin.py::BuiltinTest Compile/I/O/Regression Method Audit`

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_compile` | `ported` | `cpython_compile_builtin_code_object_subset` covers public `compile()` string, bytes-with-BOM, keyword arguments, `eval` / `exec` / `single` modes, `dont_inherit`, executable code objects, and representative TypeError / ValueError / SyntaxError paths. | None for this method's portable public behavior; broader optimize and public-AST compile matrices are tracked in `TestSpecifics` and AST audits. |
| `test_compile_top_level_await_no_coro` | `ported` | `cpython_builtin_compile_top_level_await_no_coro_subset` covers public `ast.PyCF_ALLOW_TOP_LEVEL_AWAIT` flag handling for `single` and `exec` compilation of ordinary functions and list/set/generator/dict comprehensions, proving non-awaiting top-level code objects do not carry `inspect.CO_COROUTINE`. | None for this method. |
| `test_compile_top_level_await` | `ported` | `cpython_builtin_compile_top_level_await_subset` covers the current CPython sample matrix across `single` and `exec` modes, optimize levels `-1`, `0`, `1`, and `2`, public `ast.PyCF_ALLOW_TOP_LEVEL_AWAIT` rejection/acceptance, `inspect.CO_COROUTINE` code-object metadata, and execution through both `types.FunctionType(co, globals)` and `eval(co, globals)` with module-code globals writeback. | None for this method. |
| `test_compile_top_level_await_invalid_cases` | `ported` | `cpython_builtin_compile_top_level_await_invalid_cases_subset` covers both ordinary and `ast.PyCF_ALLOW_TOP_LEVEL_AWAIT` compilation in `single` and `exec` modes rejecting nested ordinary functions that use `await`, async comprehensions, `async for`, or `async with` as `SyntaxError`. | None for this method. |
| `test_compile_async_generator` | `ported` | `cpython_builtin_compile_async_generator_flag_subset` covers `compile(..., flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT)`, `exec()` of the resulting module code object, and `types.AsyncGeneratorType` identity for the created async generator function. | None for this method. |
| `test_compile_ast` | `ported` | Public source-to-AST compile coverage exists through `cpython_compile_specifics_compile_ast_public_subset`, `cpython_compile_specifics_compile_ast_cpython_file_subset`, AST optimization tests over `ast.parse(..., optimize=...)`, and `cpython_builtin_compile_optimized_ast_subset`, which covers `ast.PyCF_OPTIMIZED_AST` through builtin `compile()` for source and public-AST inputs plus the exact `__debug__` optimized-AST matrix. | None for this method. |
| `test_open` | `blocked_by_runtime` | None. | Requires an `open()` builtin, file object text I/O, embedded-NUL path validation, and host filesystem policy. |
| `test_open_default_encoding` | `blocked_by_runtime` | None. | Requires locale/environment-sensitive default text encoding policy and `EncodingWarning` interaction. |
| `test_open_non_inheritable` | `blocked_by_runtime` | None. | Requires real file descriptors and OS-level inheritable-flag behavior. |
| `test_input` | `blocked_by_runtime` | None. | Requires an `input()` builtin wired to mutable `sys.stdin` / `sys.stdout`, prompt writing, closed-stdin errors, EOF handling, and missing stdio attribute errors. |
| `test_input_gh130163` | `blocked_by_runtime` | None. | Requires `input()` resilience against stdio object replacement during attribute lookup plus CPython GC/lifetime behavior. |
| `test_bytearray_translate` | `ported` | `cpython_builtin_bytearray_translate_extend_errors_subset` covers short translation-table `ValueError` and invalid delete-argument `TypeError` behavior. | None for this method's public error-class behavior. |
| `test_bytearray_extend_error` | `ported` | `cpython_builtin_bytearray_translate_extend_errors_subset` covers `bytearray.extend(map(int, "X"))` raising `ValueError` while preserving the receiver as empty. | None for this method. |
| `test_bytearray_join_with_misbehaving_iterator` | `ported` | `cpython_bytearray_join_reentrant_resize_subset` covers re-entrant separator resizing during `bytearray.join()` raising a catchable `BufferError`; the case stays as local runtime evidence because default system CPython versions disagree on this regression. | None for this method's public behavior. |
| `test_bytearray_join_with_custom_iterator` | `ported` | `cpython_bytearray_join_custom_iterator_diff_subset` and `cpython_bytearray_join_reentrant_resize_subset` cover `bytearray.join()` accepting a custom iterator and producing the expected bytearray result. | None for this method. |
| `test_construct_singletons` | `ported` | `cpython_builtin_construct_singletons_subset` and `cpython_builtin_singleton_construction_and_attributes_diff_subset` cover zero-argument construction of `NoneType`, `ellipsis`, and `NotImplementedType` returning the existing singleton plus positional and keyword TypeError paths. | None for this method. |
| `test_bool_notimplemented` | `ported` | `cpython_builtin_bool_notimplemented_subset` covers `bool(NotImplemented)`, `if NotImplemented`, and `not NotImplemented` rejecting the sentinel in boolean contexts. Capability-gated direct CPython rejection evidence is in `cpython_builtin_bool_notimplemented_diff_subset` for oracles with the current TypeError behavior. | None for this method. |
| `test_singleton_attribute_access` | `ported` | `cpython_builtin_singleton_attribute_access_subset` and `cpython_builtin_singleton_construction_and_attributes_diff_subset` cover `NotImplemented` and `Ellipsis` `__class__` identity plus missing instance/class attribute access and assignment errors. | None for this method. |

## `Lib/test/test_builtin.py::TestSorted` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_basic` | `ported` | `cpython_builtin_sorted_exact_subset` covers a deterministic shuffled list, proves `sorted()` returns ascending order without mutating the source list, and covers `key=` plus `reverse=True` ordering. | None for this method. |
| `test_bad_arguments` | `ported` | `cpython_builtin_sorted_exact_subset` covers positional-only rejection for `iterable=`, rejection of a second positional argument, and the accepted `key=None` shape. | None for this method. |
| `test_inputtypes` | `ported` | `cpython_builtin_sorted_exact_subset` covers `list`, `tuple`, `str`, `set`, `frozenset`, and `dict.fromkeys` iterable inputs with matching sorted key/value order. | None for this method. |
| `test_baddecorator` | `ported` | `cpython_builtin_sorted_exact_subset` covers the legacy third positional argument rejection that CPython keeps for the removed comparison-function slot. | None for this method. |

## `Lib/test/test_builtin.py::TestBreakpoint` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_breakpoint` | `partial` | `cpython_builtin_breakpoint_custom_hook_subset` covers builtin visibility and custom hook dispatch; `cpython_builtin_breakpoint_default_stub_subset` covers MiniPython's sandbox no-op default hook; the diff case `builtin-breakpoint-custom-hook` compares the custom-hook public behavior against CPython. | CPython's default `sys.__breakpointhook__` pdb/debugger integration remains blocked by runtime policy. |
| `test_breakpoint_with_breakpointhook_set` | `ported_public` | `cpython_builtin_breakpoint_custom_hook_subset` covers assigning `sys.breakpointhook` to a Python function and dispatching through `breakpoint()`. | None for the custom-hook public subset. |
| `test_breakpoint_with_breakpointhook_reset` | `partial` | `cpython_builtin_breakpoint_custom_hook_subset` covers saving and restoring `sys.breakpointhook` and identity with `sys.__breakpointhook__`; `cpython_builtin_breakpoint_default_stub_subset` covers calling the restored sandbox default hook. | CPython's restored default hook still routes to pdb/debugger integration, which is outside the sandbox runtime policy. |
| `test_breakpoint_with_args_and_keywords` | `ported_public` | `cpython_builtin_breakpoint_custom_hook_subset` covers positional and keyword argument passthrough to the active custom hook. | None for the custom-hook public subset. |
| `test_breakpoint_with_passthru_error` | `ported_public` | `cpython_builtin_breakpoint_passthru_error_subset` and the differential `builtin-breakpoint-passthru-error` case cover TypeError propagation from the configured custom hook. | None for the custom-hook public subset. |
| `test_envar_good_path_builtin` | `blocked_by_runtime` | None. | Requires `PYTHONBREAKPOINT` lookup, environment-variable policy, and dynamic builtin-hook import. |
| `test_envar_good_path_other` | `blocked_by_runtime` | None. | Requires `PYTHONBREAKPOINT` dotted import resolution for non-builtin hooks. |
| `test_envar_good_path_noop_0` | `blocked_by_runtime` | None. | Requires the `PYTHONBREAKPOINT=0` no-op convention. |
| `test_envar_good_path_empty_string` | `blocked_by_runtime` | None. | Requires the empty-string environment-variable fallback to the default debugger hook. |
| `test_envar_unimportable` | `blocked_by_runtime` | None. | Requires import failure warnings and CPython-compatible `RuntimeWarning` text for invalid `PYTHONBREAKPOINT` values. |
| `test_envar_ignored_when_hook_is_set` | `blocked_by_runtime` | None. | Requires precedence rules between an explicitly assigned `sys.breakpointhook` and `PYTHONBREAKPOINT`. |
| `test_runtime_error_when_hook_is_lost` | `ported_public` | `cpython_builtin_breakpoint_custom_hook_subset` covers deleting `sys.breakpointhook` and receiving `RuntimeError: lost sys.breakpointhook`. | None for the public lost-hook behavior. |

## `Lib/test/test_builtin.py::PtyTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_input_tty` | `blocked_by_runtime` | None. | Requires `input()` behavior when stdin/stdout are real TTYs backed by `pty.fork()`. |
| `test_input_tty_non_ascii` | `blocked_by_runtime` | None. | Requires TTY input/output encoding behavior through wrapped stdio streams. |
| `test_input_tty_non_ascii_unicode_errors` | `blocked_by_runtime` | None. | Requires TTY readline error-handler behavior for non-ASCII terminal bytes. |
| `test_input_tty_null_in_prompt` | `blocked_by_runtime` | None. | Requires TTY-backed `input()` prompt validation and null-character error behavior. |
| `test_input_tty_nonencodable_prompt` | `blocked_by_runtime` | None. | Requires strict stdout encoding failure behavior for terminal prompts. |
| `test_input_tty_nondecodable_input` | `blocked_by_runtime` | None. | Requires strict stdin decoding failure behavior for terminal input. |
| `test_input_no_stdout_fileno` | `blocked_by_runtime` | None. | Requires CPython's fallback path when stdin is a terminal but stdout lacks `fileno()`. |

## `Lib/test/test_builtin.py::ShutdownTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_cleanup` | `blocked_by_cpython_internal` | None. | Validates CPython child-process interpreter shutdown, object finalization, module lifetime, builtins availability during teardown, GC cycles, and shutdown-time stdout encoding behavior. |

## `Lib/test/test_builtin.py::ImmortalTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_immortals` | `blocked_by_cpython_internal` | None. | Validates CPython's immortal-object refcount threshold for `None`, booleans, `Ellipsis`, `NotImplemented`, and small integers through `sys.getrefcount()`. |
| `test_list_repeat_respect_immortality` | `blocked_by_cpython_internal` | None. | Validates that CPython list repetition preserves immortal-object refcount invariants. |
| `test_tuple_repeat_respect_immortality` | `blocked_by_cpython_internal` | None. | Validates that CPython tuple repetition preserves immortal-object refcount invariants. |

## `Lib/test/test_builtin.py::TestType` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_new_type` | `ported` | `cpython_type_dynamic_class_subset` covers ordinary dynamic class construction, metadata, base selection for MiniPython classes, instance `type()` / `__class__`, inherited methods, class-dict method installation, and `type('C', (B, int), ...)` public int-subclass layout including inherited int behavior, equality to `42`, and `int.to_bytes()` on the dynamic subclass instance. | None for this method. |
| `test_type_nokwargs` | `ported` | `cpython_type_nokwargs_subset` rejects extra keyword arguments for three-argument `type()`, including `x=` and `dict=`. | None for this method. |
| `test_type_name` | `partial` | `cpython_type_name_qualname_subset` and `cpython_type_name_qualname_diff_subset` cover valid dynamic-class names, writable `__name__`, bytes-name rejection, and NUL-containing name rejection while preserving existing metadata. | Full parity still needs surrogate-code-point `UnicodeEncodeError` branches for construction and assignment; MiniPython currently rejects lone-surrogate string escapes during lexing because runtime strings use Rust scalar values. |
| `test_type_qualname` | `ported` | `cpython_type_name_qualname_subset` and `cpython_type_name_qualname_diff_subset` cover `__qualname__` supplied in the namespace, writable class `__qualname__`, and non-string rejection. | None for this method. |
| `test_type_firstlineno` | `ported` | `cpython_type_doc_and_firstlineno_subset` covers dynamic-class `__firstlineno__`, deletion after `__module__` assignment, and writable replacement. | None for this method. |
| `test_type_typeparams` | `ported` | `cpython_type_typeparams_subset` covers generic class `__type_params__`, `typing.TypeVar` identity, user assignment override, delete rejection, and preserving the override. | None for this method. |
| `test_type_doc` | `partial` | `cpython_type_doc_and_firstlineno_subset` and `cpython_type_doc_and_firstlineno_diff_subset` cover dynamic-class `__doc__` construction and assignment for strings, non-ASCII strings, NUL-containing strings, bytes, integers, and `None`. | Full parity still needs the surrogate-code-point `UnicodeEncodeError` branch during dynamic-class construction; MiniPython currently rejects lone-surrogate string escapes during lexing because runtime strings use Rust scalar values. |
| `test_bad_args` | `ported` | `cpython_type_builtin_subset` and `cpython_type_dynamic_class_subset` cover wrong arity, extra positional/keyword arguments, non-string names, invalid NUL names, non-tuple bases, non-mapping namespaces, mappingproxy namespaces, invalid bases, and incompatible builtin bases. | None for this method's public error-class behavior. |
| `test_bad_slots` | `ported` | `cpython_type_bad_slots_subset` covers invalid `__slots__` bytes values, unsupported nonempty slots on `int` subclasses, invalid identifiers, NUL-containing names, class-variable conflicts, duplicate `__dict__` / `__weakref__`, and inherited `__dict__` / `__weakref__` slot rejection. | None for this method's public error-class behavior; internal slot layout remains outside scope. |
| `test_namespace_order` | `ported` | `cpython_type_namespace_order_subset` covers ordered mapping namespaces, `OrderedDict.move_to_end()`, and preserving insertion order in the created class dictionary. | None for this method. |

## `Lib/test/test_types.py::TypesTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_names` | `ported` | `cpython_types_names_public_surface_subset` covers the current public `types.__all__` name set, visible module attributes, function/lambda and builtin-function/builtin-method alias identity, `_types` accelerator alias identity, and basic type-object shape for descriptor/C-only aliases. `cpython_types_runtime_type_aliases_subset` covers public aliases backed by MiniPython runtime values. | None for the public module-name and accelerator-alias behavior; CPython's forced pure-Python `types` fallback path is not a MiniPython runtime contract. |
| `test_truth_values` | `ported` | Language truthiness tests cover false `None`, zero numbers, and empty strings plus true nonzero numbers, nonempty strings, dicts, functions, classes, modules, and instances. | None for this method's public behavior. |
| `test_boolean_ops` | `ported` | `runs_boolean_operators`, `short_circuits_boolean_operators`, and `cpython_compile_boolean_expression_exact_subset` cover `or`, `and`, `not`, operand identity, and truthiness call counts. | None for this method. |
| `test_comparisons` | `ported` | `compares_numbers`, `compares_strings`, `runs_chained_comparisons`, and `cpython_operator_comparison_predicate_subset` cover chained numeric/string comparisons and identity checks. | None for this method. |
| `test_float_constructor` | `ported` | `cpython_types_float_constructor_edges_subset` covers `float('')`, `float('5\\0')`, and `float('5_5\\0')` raising catchable `ValueError`. | None for this method. |
| `test_zero_division` | `ported` | `reports_division_by_zero`, `cpython_divmod_builtin_subset`, and numeric operator tests cover float and integer true-division, floor-division, and modulo zero-division errors. | None for this method. |
| `test_numeric_types` | `ported` | `runs_scalar_builtin_constructors`, `runs_float_literals`, integer conversion tests, and numeric comparison tests cover int/float equality, zero-argument constructors, int truncation, and float construction from integers. | None for this method's public behavior. |
| `test_float_to_string` | `ported` | `cpython_types_float_to_string_subset` covers the full -99-through-99 exponent matrix, three-digit exponent rows, `%g` / `%#g`, direct `float.__format__('e')`, direct `int` / `bool` `__format__`, class-level numeric `__format__`, and descriptor/type-error boundaries. | None for this method. |
| `test_normal_integers` | `ported` | `cpython_types_normal_integers_subset` covers the public addition, comparison, multiplication-commutativity regression, floor-division/multiplication `sys.maxsize` boundary rows, unified `int` result type checks, `isinstance()` boundary checks, and negative-shift `ValueError` rows. CPython's small-integer object-sharing assertion is classified as implementation-specific rather than a MiniPython language contract. | None for this method's public behavior. |
| `test_floats` | `ported` | Float literal, arithmetic, comparison, and operator tests cover the addition and ordering rows in the CPython method. | None for this method. |
| `test_strings` | `ported` | String length, concatenation, repetition, min/max, membership, percent-formatting, and extended-slice tests cover this method's public string behavior. | None for this method. |
| `test_type_function` | `ported` | `cpython_type_builtin_subset` covers `type()` rejecting two and four positional arguments. | None for this method. |
| `test_int__format__` | `ported` | `cpython_types_int_format_subset` covers the CPython method's direct `int.__format__()` matrix for decimal/character/binary/octal/hex formatting, sign and alignment interaction, alternate prefixes, zero fill, comma grouping, huge integers, disallowed precision, invalid `c` combinations, non-string format specs, invalid type-code sweep, float presentation types routed through float formatting, and custom fill/alignment regression rows. | None for this method's non-locale public behavior; locale-aware `n` behavior remains tracked separately by the locale rows below. |
| `test_float__format__locale` | `blocked_by_runtime` | None. | Requires host locale configuration and locale-aware `format(..., 'n')` parity. |
| `test_int__format__locale` | `blocked_by_runtime` | None. | Requires host locale configuration and locale-aware integer `format(..., 'n')` parity. |
| `test_float__format__` | `ported` | `cpython_types_float_format_subset` covers the CPython method's direct `float.__format__()` and `format()` matrix for default, fixed, scientific, general, percent, sign, no-explicit-type, zero-padding, comma grouping, alternate-form, huge fixed-output, invalid integer-presentation, non-string format spec, and custom fill/alignment rows. | None for this method's non-locale public behavior; locale-aware `n` behavior remains tracked separately by the locale rows above. |
| `test_format_spec_errors` | `ported` | `cpython_types_format_spec_errors_subset` covers the CPython method's huge width, huge precision, combined huge width/precision `ValueError` rows plus comma-disallowed type-code rows for `x`, `X`, `o`, `b`, `n`, and `s`. | None for this method's public error-class behavior. |
| `test_internal_sizes` | `blocked_by_cpython_internal` | None. | `object.__basicsize__` and `tuple.__itemsize__` expose CPython object layout, not a MiniPython contract. |
| `test_slot_wrapper_types` | `ported` | `cpython_types_slot_and_method_wrapper_types_subset` covers `object.__init__`, `object.__str__`, `object.__lt__`, and `int.__lt__` as `types.WrapperDescriptorType` instances, including exact `type(...) is types.WrapperDescriptorType` checks and callable direct slot behavior. | None for this method's public behavior. |
| `test_dunder_get_signature` | `blocked_by_runtime` | None. | Requires descriptor `__get__` signature introspection through `inspect.signature()` and CPython builtin docstring metadata. |
| `test_method_wrapper_types` | `ported` | `cpython_types_slot_and_method_wrapper_types_subset` covers `object().__init__`, `object().__str__`, `object().__lt__`, and `(42).__lt__` as `types.MethodWrapperType` instances, including exact `type(...) is types.MethodWrapperType` checks and callable bound slot behavior. | None for this method's public behavior. |
| `test_method_descriptor_types` | `ported` | `cpython_types_method_descriptor_types_subset` covers `str.join` and `list.append` as `types.MethodDescriptorType`, bound `''.join` and `[].append` as `types.BuiltinMethodType`, `int.__dict__['from_bytes']` as `types.ClassMethodDescriptorType`, `int.from_bytes` and `int.__new__` as `types.BuiltinMethodType`, plus executable unbound descriptor calls for `list.append`, `int.from_bytes`, `int.__dict__['from_bytes']`, and `int.__new__`. | None for this method's public behavior. |
| `test_method_descriptor_crash` | `blocked_by_runtime` | None. | Requires `_io`, `io`, and `_queue` method-descriptor binding behavior; the original regression is tied to CPython descriptor internals. |
| `test_ellipsis_type` | `ported` | `cpython_types_singleton_type_aliases_subset` covers `isinstance(Ellipsis, types.EllipsisType)` and singleton type metadata. | None for this method. |
| `test_notimplemented_type` | `ported` | `cpython_types_singleton_type_aliases_subset` covers `isinstance(NotImplemented, types.NotImplementedType)` and singleton type metadata. | None for this method. |
| `test_none_type` | `ported` | `cpython_types_singleton_type_aliases_subset` covers `isinstance(None, types.NoneType)` and singleton type metadata. | None for this method. |
| `test_traceback_and_frame_types` | `ported` | `cpython_types_code_traceback_type_aliases_subset` and `cpython_types_frame_type_alias_subset` cover traceback and frame objects against `types.TracebackType` and `types.FrameType`. | None for the public alias behavior. |
| `test_capsule_type` | `blocked_by_cpython_internal` | `cpython_types_names_public_surface_subset` exposes `CapsuleType` in the module surface. | `_datetime.datetime_CAPI` is a CPython C-extension capsule surface, not a MiniPython language contract. |
| `test_call_unbound_crash` | `blocked_by_cpython_internal` | List method behavior is covered elsewhere. | The source is a CPython specialized-instruction crash regression run in a child process; MiniPython should preserve list-call semantics without reproducing CPython specialization machinery. |
| `test_frame_locals_proxy_type` | `ported` | `cpython_types_frame_locals_proxy_type_subset` covers `types.FrameLocalsProxyType` as a builtin type object with public `__doc__`, `__module__`, and `__name__` metadata, plus `inspect.currentframe()` returning a frame whose `f_locals` is a `FrameLocalsProxy` mapping over live locals. | None for this method's public behavior. |

## `Lib/test/test_types.py::UnionTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_or_types_operator` | `ported` | `cpython_types_union_public_operator_and_classinfo_subset` covers order-insensitive equality for builtin, legacy `typing.List` / `Tuple`, parameterized legacy typing alias, and `None` unions; `NoneType` normalization; nested flattening; deduplication; single-member simplification; invalid operands; equality against unrelated objects; ordering-operation TypeErrors; and the long builtin-type operand chain. | None for this method. |
| `test_hash` | `ported` | `cpython_types_union_public_operator_and_classinfo_subset` covers order-insensitive hash equality and parity with `typing.Union[int, str]`. | None for this method. |
| `test_union_of_unhashable` | `ported` | `cpython_types_union_unhashable_metaclass_subset` covers custom metaclass `__hash__ = None` preserving union `__args__` order and raising catchable `TypeError` with the metaclass name for all-unhashable, leading-builtin, and trailing-builtin PEP 604 unions. | None for this method. |
| `test_unhashable_becomes_hashable` | `ported` | `cpython_types_union_dynamic_hashability_subset` covers CPython's construction-time tracking for union members that were unhashable when the union was created, including the later `union contains 2 unhashable elements` error after those classes become hashable, while fresh unions become hashable immediately. | None for this method. |
| `test_instancecheck_and_subclasscheck` | `ported` | `cpython_types_union_public_operator_and_classinfo_subset` covers `isinstance()` / `issubclass()` for int-or-str unions, `typing.Union[int, str]`, int-or-None unions, `typing.Union[int, None]`, and `collections.abc.Mapping` unions. | None for this method. |
| `test_instancecheck_and_subclasscheck_order` | `ported` | `cpython_types_union_typevar_parameter_subset` covers TypeVar-containing union classinfo order semantics: leading concrete operands can resolve before an invalid TypeVar, leading TypeVars raise `TypeError`, and unresolved object/object-subclass checks raise `TypeError` for both direct PEP 604 and `typing.Union` forms. | None for this method. |
| `test_bad_instancecheck` | `ported` | `cpython_types_union_bad_classinfo_checks_subset` covers PEP 604 union classinfo short-circuiting on the leading concrete type and propagation of a custom metaclass `__instancecheck__` `ZeroDivisionError` for the later union member. | None for this method. |
| `test_bad_subclasscheck` | `ported` | `cpython_types_union_bad_classinfo_checks_subset` covers PEP 604 union classinfo short-circuiting on the leading concrete type and propagation of a custom metaclass `__subclasscheck__` `ZeroDivisionError` for the later union member. | None for this method. |
| `test_or_type_operator_with_TypeVar` | `ported` | `cpython_types_union_typevar_parameter_subset` covers both TypeVar operand orders and one-parameter union substitution simplifying back to the surviving type. | None for this method. |
| `test_union_args` | `ported` | `cpython_types_union_public_operator_and_classinfo_subset` covers `__args__` ordering for builtin, nested, flattened, deduplicated, `typing.Union`, and `NoneType` unions, plus the CPython loop over builtin generic aliases, legacy `typing.List` / `typing.Tuple` / `typing.Callable` aliases, and `typing.Hashable`. `cpython_types_union_typevar_parameter_subset` adds TypeVar-or-None ordering for both operand directions. | None for this method. |
| `test_union_parameter_chaining` | `ported` | `cpython_types_union_typevar_parameter_subset` covers `__parameters__` tracking and substitution through unions containing generic aliases, nested union arguments inside generic aliases, multi-TypeVar substitution, and deduplication after both parameters resolve to the same generic alias. | None for this method. |
| `test_union_parameter_substitution` | `ported` | `cpython_types_union_parameter_substitution_subset` covers substitutions through builtin types, builtin generic aliases, legacy `typing.List` / `typing.Tuple` / `typing.Hashable` / `typing.Callable`, `typing.Literal`, `typing.NewType`, `collections.abc.Hashable` / `Callable`, nested PEP 604 and `typing.Union` operands, duplicate-removal after substitution, and substituting a fresh TypeVar while preserving `__parameters__`. | None for this method. |
| `test_union_pickle` | `ported` | `cpython_types_union_copy_pickle_subset` covers pickle round trips for all exposed pickle protocols on a TypeVar-containing PEP 604 union alias, preserving equality, `__args__`, `__parameters__`, and public union type name. | None for this method. |
| `test_union_copy` | `ported` | `cpython_types_union_copy_pickle_subset` covers `copy.copy()` and `copy.deepcopy()` on a TypeVar-containing PEP 604 union alias, preserving equality, `__args__`, `__parameters__`, non-identity, and public union type name. | None for this method. |
| `test_union_parameter_substitution_errors` | `ported` | `cpython_types_union_parameter_substitution_subset` covers the CPython public arity error path for substituting two arguments into a one-TypeVar union alias. | None for this method. |
| `test_or_type_operator_with_forward` | `ported` | `cpython_types_union_forward_ref_subset` covers both TypeVar/string forward-reference operand orders, preserving forward-reference operand order. `cpython_types_union_forward_get_type_hints_subset` covers `typing.get_type_hints()` resolving those forward references through function globals while preserving `typing.get_args()` order. | None for this method. |
| `test_or_type_operator_with_Protocol` | `ported` | `cpython_types_union_protocol_subset` covers class-based `typing.Protocol` subclass creation and PEP 604 equality with `typing.Union[Proto, str]`. | None for this method. |
| `test_or_type_operator_with_Alias` | `ported` | `cpython_types_union_public_operator_and_classinfo_subset` covers both `list`-or-`str` and legacy `typing.List`-or-`str` equality with `typing.Union`. | None for this method. |
| `test_or_type_operator_with_NamedTuple` | `ported` | `cpython_types_union_public_operator_and_classinfo_subset` covers `namedtuple(...)` type objects unioned with `str`, including equality with `typing.Union` and preserved `__args__` type names. | None for this method. |
| `test_or_type_operator_with_TypedDict` | `ported` | `cpython_types_union_typed_dict_subset` covers class-based `typing.TypedDict` creation, call construction as a dict, and PEP 604 equality with `typing.Union[Point2D, str]`. | None for this method. |
| `test_or_type_operator_with_NewType` | `ported` | `cpython_types_union_newtype_subset` covers public `typing.NewType` metadata, call pass-through behavior, and PEP 604 union equality with `typing.Union[NewType, str]`. | None for this method. |
| `test_or_type_operator_with_IO` | `ported` | `cpython_types_union_io_subset` covers public `typing.IO` metadata, PEP 604 equality with `typing.Union[typing.IO, str]`, `typing.IO[str]` generic alias union behavior, and bare `typing.TextIO` / `typing.BinaryIO` union operands. | None for this method. |
| `test_or_type_operator_with_SpecialForm` | `ported` | `cpython_types_union_special_form_subset` covers the CPython public assertions for `typing.Any`, `typing.NoReturn`, `typing.Optional[int]`, flattened optional unions, and extending an existing `typing.Union` with another type. | None for this method. |
| `test_or_type_operator_with_Literal` | `ported` | `cpython_types_union_literal_subset` covers non-enum `typing.Literal[...]` union args, literal deduplication, CPython's distinct `Literal[0]` vs `Literal[False]` / `Literal[1]` vs `Literal[True]` behavior, and `enum.IntEnum` literal identity/equality parity including alias members and distinct enum-member-vs-int/bool literal unions. | None for this method. |
| `test_or_type_repr` | `ported` | `cpython_types_union_public_operator_and_classinfo_subset` covers direct int-or-str repr, flattened union repr, `None` spelling, `str()` parity, and `typing.GenericAlias(list, int)` union repr. | None for this method. |
| `test_or_type_operator_with_genericalias` | `ported` | `cpython_types_union_public_operator_and_classinfo_subset` covers builtin generic alias union flattening, `__args__`, order-insensitive equality, deduplication, repr for larger mixed generic-alias unions, and invalid `isinstance()` / `issubclass()` classinfo checks for generic-alias unions; `cpython_types_generic_alias_union_type_subset` covers `types.GenericAlias` identity; `cpython_types_union_genericalias_subclass_bad_eq_subset` covers `types.GenericAlias` subclasses, subclass payload equality/hash/repr, union equality/deduplication/order behavior, invalid GenericAlias-subclass union classinfo checks, and `ZeroDivisionError` propagation from bad type equality. | None for this method. |
| `test_or_type_operator_with_bad_module` | `ported` | `cpython_types_union_bad_module_guard_subset` covers the public bad-`__module__` metaclass regression path for `TypeVar` / `_SpecialForm` shaped objects by requiring catchable `TypeError` or `ZeroDivisionError` rather than a runtime crash. | None for this method. |
| `test_or_type_operator_reference_cycle` | `blocked_by_cpython_internal` | No Rust evidence by design. | CPython `sys.gettotalrefcount()` leak checks are implementation-internal. |
| `test_instantiation` | `ported` | `cpython_types_generic_alias_union_type_subset` covers `types.UnionType` construction rejection through zero-arg instantiation, public type identity, and `types.UnionType[...]` subscription for single, duplicate, and multi-member type operands. `cpython_types_union_forward_ref_subset` covers `typing.ForwardRef`, `typing.Union[int, "str"]`, and `types.UnionType[int, "str"]` forward-reference instantiation parity. | None for this method. |

## `Lib/test/test_types.py::MappingProxyTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_constructor` | `ported` | `cpython_types_mappingproxy_exact_dict_subset`, `cpython_types_mappingproxy_missing_subset`, and `cpython_types_mappingproxy_chainmap_subset` cover exact dict, dict-subclass missing behavior, ChainMap sources, equality, and invalid constructor inputs. | None for this method's public behavior. |
| `test_methods` | `ported` | `cpython_types_mappingproxy_method_surface_subset` covers the public method names and behavior for containment, item lookup, iteration, length, reverse iteration, union helpers, `copy`, `get`, `items`, `keys`, `values`, and `__class_getitem__`. | None for this method. |
| `test_get` | `ported` | `cpython_types_mappingproxy_exact_dict_subset` covers item lookup, missing-key `KeyError`, `get()` default `None`, and explicit default values. | None for this method. |
| `test_missing` | `ported` | `cpython_types_mappingproxy_missing_subset` covers dict-subclass `__missing__` dispatch for subscript lookup while `get()` and membership do not invoke missing. | None for this method. |
| `test_customdict` | `ported` | `cpython_types_mappingproxy_custom_mapping_subset` covers public forwarding of `__contains__`, `__getitem__`, `__iter__`, `__len__`, `copy`, `get`, `items`, `keys`, and `values` through a custom mapping object. | None for this method's observable mapping-protocol behavior. |
| `test_chainmap` | `ported` | `cpython_types_mappingproxy_chainmap_subset` covers ChainMap membership, lookup, missing-key `KeyError`, iteration, length, copy, `get`, and view methods through `MappingProxyType`. | None for this method. |
| `test_contains` | `ported` | `cpython_types_mappingproxy_contains_subset` covers exact-dict key membership and missing-key rejection. | None for this method. |
| `test_views` | `ported` | `cpython_types_mappingproxy_views_subset` covers live keys/items/values views before and after underlying dict mutation. | None for this method. |
| `test_len` | `ported` | `cpython_types_mappingproxy_len_subset` covers `len()` for exact-dict proxies over sizes 0 through 5. | None for this method. |
| `test_iterators` | `ported` | `cpython_types_mappingproxy_iterators_subset` covers iteration over keys, keys view, values view, and items view. | None for this method. |
| `test_reversed` | `ported` | `cpython_types_mappingproxy_reversed_subset` covers reverse iteration after underlying dict mutation and iterator exhaustion. | None for this method. |
| `test_copy` | `ported` | `cpython_types_mappingproxy_copy_subset` covers exact-dict copy type/equality and independence after later underlying dict mutation. | None for this method. |
| `test_union` | `ported` | `cpython_types_mappingproxy_union_subset` covers exact dict and mappingproxy operands for normal, reflected, and in-place union operations, plus invalid iterable-pair operands. | None for this method. |
| `test_hash` | `ported` | `cpython_types_mappingproxy_hash_subset` covers unhashable exact-dict proxies and hash forwarding for proxies over hashable custom mappings. | None for this method's public hash behavior. |
| `test_richcompare` | `ported` | `cpython_types_mappingproxy_richcompare_subset` covers equality, inequality, and catchable CPython-style `TypeError` ordering between mappingproxy objects. | None for this method. |

## `Lib/test/test_types.py::ClassCreationTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_new_class_basics` | `ported` | `cpython_types_class_creation_new_class_meta_helper_subset` and `cpython_types_class_creation_new_class_resolve_bases_subset` cover default `types.new_class("C")` construction, `__name__`, and default `object` bases. | None for this method. |
| `test_new_class_subclass` | `ported` | `cpython_types_class_creation_new_class_meta_helper_subset` covers `types.new_class()` with an explicit `int` base and `issubclass()` behavior. | None for this method. |
| `test_new_class_meta` | `ported` | `cpython_types_class_creation_new_class_meta_helper_subset` covers repeated construction through a custom metaclass, inherited `__prepare__` namespace values, and caller keyword-dict preservation. | None for this method. |
| `test_new_class_exec_body` | `ported` | `cpython_types_class_creation_new_class_meta_helper_subset` covers `exec_body` namespace mutation combined with custom metaclass-prepared namespace values. | None for this method. |
| `test_new_class_metaclass_keywords` | `ported` | `cpython_types_class_creation_new_class_metaclass_keywords_subset` covers callable metaclass dispatch with name, original bases, empty namespace, and non-metaclass keyword forwarding. | None for this method. |
| `test_new_class_defaults` | `ported` | `cpython_types_class_creation_new_class_meta_helper_subset` covers `types.new_class("E", (), {}, None)`, preserving the class name and default `object` base. | None for this method. |
| `test_new_class_meta_with_base` | `ported` | `cpython_types_class_creation_new_class_meta_helper_subset` covers keyword-form `types.new_class()` with an explicit base, custom metaclass, `exec_body`, and prepared namespace values. | None for this method. |
| `test_new_class_with_mro_entry` | `ported` | `cpython_types_class_creation_mro_entries_core_subset` covers single-provider `__mro_entries__` replacement, `__orig_bases__`, and public `__mro__` shape. | None for this method. |
| `test_new_class_with_mro_entry_genericalias` | `ported` | `cpython_types_class_creation_mro_entries_core_subset` covers `typing.List[int]` and `list[int]` base resolution with original bases preserved. | None for this method. |
| `test_new_class_with_mro_entry_none` | `ported` | `cpython_types_class_creation_mro_entries_core_subset` covers empty-tuple `__mro_entries__` results removing a base while preserving original bases and MRO order. | None for this method. |
| `test_new_class_with_mro_entry_error` | `ported` | `cpython_types_class_creation_mro_entries_core_subset` covers catchable `TypeError` for non-tuple `__mro_entries__` results. | None for this method's public error-class behavior. |
| `test_new_class_with_mro_entry_multiple` | `ported` | `cpython_types_class_creation_mro_entries_multiple_subset` covers multiple providers expanding left-to-right into dynamic class bases. | None for this method. |
| `test_new_class_with_mro_entry_multiple_2` | `ported` | `cpython_types_class_creation_mro_entries_multiple_subset` covers multiple providers around an ordinary class base, original-bases preservation, and resulting MRO order. | None for this method. |
| `test_get_original_bases` | `ported` | `cpython_types_class_creation_get_original_bases_subset` covers ordinary classes, generic user classes, builtin classes, `typing.NamedTuple`, `typing.TypedDict`, and non-type rejection. | None for this method. |
| `test_prepare_class` | `ported` | `cpython_types_class_creation_prepare_resolve_bases_subset` covers derived metaclass selection, custom `__prepare__` namespace identity, and empty remaining keyword dictionaries. | None for this method. |
| `test_bad___prepare__` | `ported` | `cpython_types_class_creation_prepare_and_metaclass_callable_subset` covers catchable class-statement `TypeError` for type and non-type metaclass `__prepare__` methods returning non-mappings. | None for this method's public error-class behavior. |
| `test_resolve_bases` | `ported` | `cpython_types_class_creation_prepare_resolve_bases_subset` covers empty inputs, class-only tuple identity preservation, instance `__mro_entries__` replacement, and base removal. | None for this method. |
| `test_resolve_bases_with_mro_entry` | `ported` | `cpython_types_class_creation_prepare_resolve_bases_subset` covers `typing.List[int]` and `list[int]` replacement through `types.resolve_bases()`. | None for this method. |
| `test_metaclass_derivation` | `ported` | `cpython_types_class_creation_metaclass_derivation_subset` covers most-specific metaclass selection, `__new__` call order through `super()`, winner `__prepare__`, base-order independence, and compatible explicit metaclass overrides. | None for this method. |
| `test_metaclass_override_function` | `ported` | `cpython_types_class_creation_metaclass_override_function_subset` covers function metaclasses returning arbitrary objects without winner-metaclass calculation over empty, `object`, and custom-metaclass bases. | None for this method. |
| `test_metaclass_override_callable` | `ported` | `cpython_types_class_creation_non_type_metaclass_derivation_subset` covers non-`type` metaclass products used as bases, winner selection from `type(base)`, object bases, and incompatible-base conflicts. | None for this method. |
| `test_one_argument_type` | `ported` | `cpython_types_class_creation_one_argument_type_subset` covers builtin `type(obj)` inspection, one-argument `type` subclass rejection, and three-argument construction through a type subclass. | None for this method. |
| `test_metaclass_new_error` | `ported` | `cpython_types_class_creation_metaclass_new_error_subset` covers winner-metaclass selection for three-argument `type()` and exception propagation from the metaclass constructor. | None for this method. |
| `test_subclass_inherited_slot_update` | `ported` | `cpython_types_class_creation_subclass_inherited_slot_update_subset` covers dict-subclass subscript behavior after dynamic `__getitem__` replacement and delegation back to `dict.__getitem__`. | None for this method's public behavior; CPython C-slot mechanics remain outside scope. |
| `test_tuple_subclass_as_bases` | `ported` | `cpython_types_class_creation_new_class_resolve_bases_subset` covers tuple-subclass bases passed to `type()` and also checks the same public preservation through `types.new_class()`. | None for this method. |

## `Lib/test/test_types.py::CoroutineTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_wrong_args` | `ported` | `cpython_types_coroutine_public_subset` covers `types.coroutine()` rejecting `None`, integers, and ordinary objects with catchable `TypeError` containing the public callable-expectation text. | None for this method. |
| `test_non_gen_values` | `ported` | `cpython_types_coroutine_public_subset` covers non-generator function return pass-through for strings and awaitable objects, including idempotent second decoration. | None for this method. |
| `test_async_def` | `ported` | `cpython_types_coroutine_async_def_subset` covers `types.coroutine(async_fn) is async_fn`, stable function `__code__` identity and flags, pass-through of native coroutine objects returned by a wrapped regular function, native coroutine `cr_code.co_flags`, and idempotent close behavior; `cpython_grammar_async_await_method_subset` covers async-function `__code__.co_flags & inspect.CO_COROUTINE`. | None for this method. |
| `test_duck_coro` | `ported` | `cpython_types_coroutine_public_subset` covers pass-through of a coroutine-like object exposing `send`, `throw`, `close`, and `__await__`, with `__await__()` returning the original object. | None for this method. |
| `test_duck_corogen` | `ported` | `cpython_types_coroutine_public_subset` covers pass-through of a coroutine/generator-like object exposing `send`, `throw`, `close`, `__await__`, `__iter__`, and `__next__`. | None for this method. |
| `test_duck_gen` | `ported` | `cpython_types_coroutine_duck_generator_wrapper_subset` covers `_GeneratorWrapper` around a returned generator-like object, `__await__()` / `iter()` returning the wrapper, and direct `send()` forwarding with `StopIteration.value` preservation. `cpython_types_coroutine_duck_generator_proxy_subset` covers the CPython `unittest.mock.MagicMock(GenLike)` matrix for generator ABC recognition, wrapper type/ABC checks, `__name__` / `__qualname__` identity pass-through, missing `gi_*` / `cr_*` AttributeErrors, later attribute pass-through and `cr_*` aliases, forwarded `close()` / `send()` / `next()` / `throw()` call assertions, propagated `side_effect` exceptions, catchable invalid-argument TypeErrors without forwarding side effects, double-wrap avoidance, and `weakref.ref(wrapper)` returning the live wrapper. | None for this method. |
| `test_duck_functional_gen` | `ported` | `cpython_types_coroutine_duck_generator_wrapper_subset` covers the functional duck-generator wrapper and direct `send()` completion value forwarding. `cpython_types_coroutine_duck_generator_await_subset` covers the exact CPython await path through a returned duck `_GeneratorWrapper`, including the first yielded value and final `StopIteration.value`; `cpython_collections_abc_types_coroutine_subset` covers await execution for generator functions directly decorated with `types.coroutine()`. | None for this method. |
| `test_gen` | `ported` | `cpython_types_coroutine_generator_wrapper_subset` covers `_GeneratorWrapper` around returned native generators, `__await__` / `iter()` returning the wrapped generator, exact CPython attribute identity forwarding for `__name__`, `__qualname__`, `gi_code`, `gi_running`, `gi_frame`, `gi_suspended`, and `cr_code`, `next()` / `send()` / `throw()` forwarding, StopIteration value propagation, and double-decoration behavior. `cpython_types_coroutine_generator_frame_subset` covers native generator `gi_frame` creation, stable frame identity, `f_code` identity, wrapper `gi_frame` / `cr_frame` aliasing, suspension, and closed-frame clearing. `cpython_types_coroutine_generator_yieldfrom_subset` covers native generator `gi_yieldfrom` delegate exposure and wrapper `cr_await` aliasing while suspended in `yield from`, plus clearing after delegate completion and generator close. | None for this method. |
| `test_returning_itercoro` | `ported` | `cpython_types_coroutine_public_subset` covers a function returning an already iterable-coroutine generator and idempotent second decoration preserving the original generator object. | None for this method. |
| `test_genfunc` | `ported` | `cpython_types_coroutine_public_subset` covers generator-function identity, idempotent second decoration, `inspect.CO_ITERABLE_COROUTINE`, `inspect.CO_COROUTINE` exclusion, and generator `gi_code.co_flags` exposure for iterable-coroutine functions. | None for this method. |
| `test_wrapper_object` | `ported` | `cpython_types_coroutine_generator_wrapper_subset` covers `_GeneratorWrapper` repr/str, `dir()` surface for iterator methods and coroutine/generator aliases, `__await__` / `__iter__` / `__next__`, and wrapper method dispatch. | None for this method. |

## `Lib/test/test_types.py::FunctionTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_function_type_defaults` | `ported` | `cpython_types_function_type_subset` constructs functions from `ex.__code__` with globals dictionaries, explicit and code-derived names, positional defaults, keyword-only defaults, and then executes those functions while checking public `__defaults__`, `__kwdefaults__`, `__name__`, `__qualname__`, `__module__`, and function type identity. | None for this method's portable public behavior. |
| `test_function_type_wrong_defaults` | `ported` | `cpython_types_function_type_subset` covers catchable `TypeError` for non-code arg 1, non-dict globals arg 2, non-string/non-None name arg 3, non-tuple/non-None defaults arg 4, invalid closure for a no-freevar code object, and non-dict/non-None kwdefaults arg 6. | None for this method. |

## `Lib/test/test_collections.py::TestUserObjects` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_str_protocol` | `ported` | `cpython_collections_userstring_protocol_and_userdict_missing_subset` checks `set(dir(UserString)) >= set(dir(str))`. | None for this method. |
| `test_list_protocol` | `ported` | `cpython_collections_userlist_public_methods_subset` checks `set(dir(UserList)) >= set(dir(list))`. | None for this method. |
| `test_dict_protocol` | `ported` | `cpython_collections_userdict_public_methods_subset` checks `set(dir(UserDict)) >= set(dir(dict))`. | None for this method. |
| `test_list_copy` | `ported` | `cpython_collections_userlist_public_methods_subset` covers `UserList` construction, list mutation methods, `.data`, `.copy()` data independence/equality, and `copy.copy()` data independence plus shallow instance-attribute copying. | None for this method. |
| `test_dict_copy` | `ported` | `cpython_collections_userdict_public_methods_subset` covers `UserDict` item assignment/deletion, `.data`, `.copy()` data independence/equality, and `copy.copy()` data independence plus shallow instance-attribute copying. | None for this method. |
| `test_dict_missing` | `ported` | `cpython_collections_userstring_protocol_and_userdict_missing_subset` covers `UserDict` subclass `__missing__` dispatch through subscript and direct `__getitem__` lookup while preserving `get()` as a non-`__missing__` lookup. | None for this method. |

## `Lib/test/test_collections.py::TestChainMap` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_basics` | `ported` | `cpython_collections_chainmap_public_methods_subset` covers empty construction, first-map assignment/deletion, `new_child()`, `maps`, `parents`, `items()`, iteration, `len()`, containment, lookup, `get()`, dict coercion, and shallow copies; `cpython_collections_chainmap_copy_pickle_eval_identity_subset` covers exact repr alternatives, shallow-copy first-map copying and parent-map sharing, pickle round trips across every exposed protocol, `copy.deepcopy()`, `eval(repr(...))`, and CPython-style object identity expectations; `cpython_types_mappingproxy_chainmap_subset` covers mappingproxy forwarding over ChainMap. | None for this method. |
| `test_ordering` | `ported` | `cpython_collections_chainmap_public_methods_subset` ports the CPython `baseline.copy(); combined.update(adjustments)` ordering comparison against `list(cm.items())`. | Exact unittest harness is not mirrored. |
| `test_constructor` | `ported` | `cpython_collections_chainmap_public_methods_subset` checks `ChainMap().maps == [{}]` and one-argument `ChainMap({1: 2}).maps == [{1: 2}]`. | Exact unittest harness is not mirrored. |
| `test_bool` | `ported` | `cpython_collections_chainmap_public_methods_subset` checks empty maps, two empty maps, and first/parent non-empty truthiness. | Exact unittest harness is not mirrored. |
| `test_missing` | `ported` | `cpython_collections_chainmap_missing_and_first_map_mutation_subset` covers a ChainMap subclass with `__missing__`, `__getitem__` fallback to missing, `get()` and membership not invoking missing, first-map `pop()` defaults, `popitem()`, and empty-first-map `KeyError`. | Exact unittest harness is not mirrored. |
| `test_order_preservation` | `ported` | `cpython_collections_chainmap_order_preservation_subset` ports the CPython OrderedDict multi-map matrix, checking `''.join(d)` and the exact `list(d.items())` combined order. | None for this method; `OrderedDict` itself is only a minimal constructor alias over insertion-ordered dict storage. |
| `test_iter_not_calling_getitem_on_maps` | `ported` | `cpython_collections_chainmap_iter_does_not_call_getitem_subset` ports the CPython `UserDict` subclass side-effect check, proving `set(ChainMap(d))` iterates mapping keys without invoking the map's overridden `__getitem__`. | None for this method. |
| `test_dict_coercion` | `ported` | `cpython_collections_chainmap_public_methods_subset` checks both `dict(cm)` and `dict(cm.items())` for combined ChainMap contents. | Exact unittest harness is not mirrored. |
| `test_new_child` | `ported` | `cpython_collections_chainmap_public_methods_subset` covers explicit child maps, child-map identity through `maps[0]`, keyword-created child maps, parent lookup, and `parents`; `cpython_collections_chainmap_new_child_custom_mapping_subset` ports the lowerdict child-map protocol checks for containment, `get()`, and subscript lookup. | None for this method. |
| `test_union_operators` | `ported` | `cpython_collections_chainmap_union_operators_subset` covers ChainMap-to-ChainMap union, ChainMap in-place union, ChainMap-to-dict union, dict-to-ChainMap union, iterable-pair rejection for plain union, iterable-pair acceptance for in-place union, and CPython subclass result-type rules including `SubclassRor.__ror__ -> super().__ror__`. | None for this method. |

## `Lib/test/test_collections.py::TestNamedTuple` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_factory` | `ported` | `cpython_collections_namedtuple_factory_instance_subset` ports CPython's factory assertions for generated `__name__`, `__slots__`, `__module__`, inherited `tuple.__getitem__`, `_fields`, invalid type/field name `ValueError` cases, digit-containing valid names, leading-underscore type names, unicode field reprs, and `_make()` arity errors. `cpython_collections_namedtuple_defaults_rename_readonly_subset` adds `rename=True`, `module=`, and generated `__new__.__defaults__` metadata coverage. `cpython_collections_namedtuple_copy_keyword_generic_alias_subset` adds namedtuple generic-alias subscription behavior. | None for this method. |
| `test_defaults` | `ported` | `cpython_collections_namedtuple_defaults_rename_readonly_subset` ports the CPython defaults matrix for two, one, zero, `None`, list, and iterator defaults; `_field_defaults`; constructor default filling; bad default counts/types; and generated `__new__.__defaults__`. | Complete for the current MiniPython namedtuple public surface. |
| `test_readonly` | `ported` | `cpython_collections_namedtuple_defaults_rename_readonly_subset` ports CPython's readonly checks for field assignment/deletion, tuple item assignment/deletion, and value preservation after failed mutation attempts. | Complete for the current MiniPython namedtuple public surface. |
| `test_factory_doc_attr` | `ported` | `cpython_collections_namedtuple_defaults_rename_readonly_subset` ports generated class `__doc__` text and class docstring assignment. | Complete for the current MiniPython namedtuple public surface. |
| `test_field_doc` | `ported` | `cpython_collections_namedtuple_field_doc_subset` ports default field descriptor docstrings and per-field `__doc__` mutation. | None for this method. |
| `test_field_doc_reuse` | `blocked_by_cpython_internal` | None. | Validates CPython descriptor reuse/reference behavior rather than ordinary Python source semantics. |
| `test_field_repr` | `blocked_by_cpython_internal` | None. | Validates CPython descriptor repr details; MiniPython should expose public field behavior without copying exact internal descriptor identity. |
| `test_name_fixer` | `ported` | `cpython_collections_namedtuple_defaults_rename_readonly_subset` ports the CPython `rename=True` matrix for invalid identifiers, keywords, leading underscores, duplicates, and empty field names. | Complete for the current MiniPython namedtuple public surface. |
| `test_module_parameter` | `ported` | `cpython_collections_namedtuple_defaults_rename_readonly_subset` ports `module=` storage and equality for a non-string module object. | Complete for the current MiniPython namedtuple public surface. |
| `test_instance` | `ported` | `cpython_collections_namedtuple_factory_instance_subset` ports CPython's instance assertions for positional, keyword, mixed, starred, and `**` construction; wrong arity/keyword errors; repr; `__weakref__` exclusion; `_make()`, `_fields`, `_replace()`, `_asdict()`; `_replace()` unexpected-keyword errors; comma field strings; and non-string field-name sequences. `cpython_collections_namedtuple_defaults_rename_readonly_subset` adds readonly field/item behavior. | None for this method. |
| `test_tupleness` | `ported` | `cpython_collections_namedtuple_factory_instance_subset` covers tuple `isinstance`, tuple equality, tuple/list conversion, unpacking, iteration through `max(p)`, star expansion through `max(*p)`, numeric and negative indexing, out-of-range `IndexError`, hash parity with the equivalent tuple, field attributes, missing-field `AttributeError`, slicing, and `count()` / `index()` tuple-method behavior. | None for this method. |
| `test_odd_sizes` | `ported` | `cpython_collections_namedtuple_factory_instance_subset` ports CPython's zero-field and one-field namedtuple construction, `_make()`, repr, `_asdict()`, `_replace()`, field access, and `_fields` assertions. | None for this method. |
| `test_large_size` | `ported` | `cpython_collections_namedtuple_large_size_subset` ports CPython's large-field namedtuple construction, `_make()`, field access by generated names, repr smoke, `_asdict()`, `_replace()`, and `_fields` assertions with deterministic field names. | None for this method. |
| `test_pickle` | `ported` | `cpython_collections_namedtuple_pickle_subset` ports generated namedtuple pickle round trips over `-1` and all exposed protocols, preserving value equality, `_fields`, generated type identity, and absence of the `OrderedDict` bytes marker through MiniPython's internal pickle payload; it also checks mutable field values are independently copied after round-trip. | None for this method. |
| `test_copy` | `ported` | `cpython_collections_namedtuple_copy_keyword_generic_alias_subset` ports `copy.copy()` and `copy.deepcopy()` for namedtuple instances, preserving value equality and `_fields`. | None for this method. |
| `test_name_conflicts` | `ported` | `cpython_collections_namedtuple_name_conflicts_subset` ports CPython's conflict-prone field-name matrix, including `itemgetter`, `property`, `self`, `cls`, `tuple`, and the full broader `words` set through construction, keyword construction, `_make()`, `repr()`, `_asdict()`, `_replace()`, `_fields`, and `__getnewargs__()`. | None for this method. |
| `test_repr` | `ported` | `cpython_collections_namedtuple_repr_subset` ports CPython's generated namedtuple repr and subclass-name repr assertions. | None for this method. |
| `test_keyword_only_arguments` | `ported` | `cpython_collections_namedtuple_copy_keyword_generic_alias_subset` ports positional rejection for keyword-only factory options and accepted `rename=True` keyword behavior. | None for this method. |
| `test_namedtuple_subclass_issue_24931` | `ported` | `cpython_collections_namedtuple_subclass_issue_24931_subset` ports subclassing a generated namedtuple type, constructing the subclass, preserving `_asdict()` ordered key/value behavior, and supporting writable subclass instance `__dict__` attributes. | None for this method. |
| `test_field_descriptor` | `blocked_by_cpython_internal` | None. | Primarily validates generated descriptor internals rather than ordinary namedtuple value semantics. |
| `test_new_builtins_issue_43102` | `ported` | `cpython_collections_namedtuple_new_builtins_issue_43102_subset` ports CPython's generated namedtuple `__new__.__globals__['__builtins__'] == {}` and `__new__.__builtins__ == {}` assertions. | None for this method. |
| `test_match_args` | `ported` | `cpython_collections_namedtuple_match_args_subset` ports CPython's generated `__match_args__ == ('x', 'y')` assertion and adds executable class-pattern coverage proving namedtuple positional subpatterns use generated field metadata, including too-many-positional and duplicate positional/keyword field errors. | None for this method. |
| `test_non_generic_subscript` | `ported` | `cpython_collections_namedtuple_copy_keyword_generic_alias_subset` ports namedtuple type subscription returning a `GenericAlias`, including `__origin__`, empty `__parameters__`, `__args__`, alias construction, original namedtuple type identity, and tuple-like instance equality. | None for this method. |

## `Lib/test/test_collections.py::TestOneTrickPonyABCs` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_Awaitable` | `ported` | `cpython_collections_abc_async_runtime_subset` covers native coroutine objects, structural `__await__`, and non-samples; `cpython_collections_abc_types_coroutine_subset` covers the `types.coroutine()` iterable-coroutine non-`Awaitable` distinction plus `Coroutine.register()` propagation through `Awaitable`; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__await__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text. | None for this method. |
| `test_Coroutine` | `ported` | `cpython_collections_abc_async_runtime_subset` covers native coroutine objects, structural `__await__` / `send` / `throw` / `close`, and missing-method non-samples; `cpython_collections_abc_types_coroutine_subset` covers the `types.coroutine()` iterable-coroutine non-`Coroutine` distinction; `cpython_collections_abc_coroutine_mixin_subset` covers direct-subclass `send`, `throw`, and `close` mixin behavior, including `StopIteration`, exception propagation, swallowed `GeneratorExit`, ignored-exit `RuntimeError`, and close-time error propagation; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__await__` / `send` / `throw` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text. | None for this method. |
| `test_Hashable` | `ported` | `cpython_collections_abc_core_runtime_subset` covers builtin hashable/non-hashable samples plus structural `__hash__` and `__hash__ = None`; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__hash__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_validate_isinstance_subset` covers CPython's `validate_isinstance(Hashable, '__hash__')` structural helper behavior, including dynamic `setattr()` and `__hash__ = None` blocking through an explicit `object` base; `cpython_collections_abc_hashable_direct_subclass_subset` covers the direct `Hashable` subclass `super().__hash__()` fallback result and `issubclass(int, H)` rejection. | None for this method. |
| `test_AsyncIterable` | `ported` | `cpython_collections_abc_async_runtime_subset` covers structural `__aiter__`, non-samples, and `__aiter__ = None` blocking; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__aiter__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_validate_isinstance_subset` covers CPython's `validate_isinstance(AsyncIterable, '__aiter__')` structural helper behavior. | None for this method. |
| `test_AsyncIterator` | `ported` | `cpython_collections_abc_async_runtime_subset` covers structural `__aiter__` + async `__anext__`, `__anext__`-only rejection, the exact CPython `None` / `object` / `list` non-sample `isinstance` and `issubclass(type(...))` matrix, and `__anext__ = None` blocking; `cpython_collections_abc_async_iterator_mixin_subset` covers direct-subclass inherited `AsyncIterator.__aiter__()` returning `self`; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__anext__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text. | None for this method. |
| `test_Iterable` | `ported` | `cpython_collections_abc_iterable_iterator_subset` covers builtin iterable samples, built-in iterators, structural user classes, direct subclassing, inheritance, and non-samples; `cpython_collections_abc_iterable_sample_matrix_subset` covers CPython's public `Iterable` non-sample and sample matrix, including dict views, native generators, generator expressions, direct-subclass `super().__iter__()` mixin behavior, and `__iter__ = None` blocking; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__iter__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_validate_isinstance_subset` covers CPython's `validate_isinstance(Iterable, '__iter__')` structural helper behavior. | None for this method. |
| `test_Reversible` | `ported` | `cpython_collections_abc_reversible_subset` covers builtin reversible samples, non-reversible samples, `Sequence` inheritance, structural user classes, direct subclassing, and `None` blocking; `cpython_collections_counter_basics_subset` adds the OrderedDict/Counter Reversible samples; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__reversed__` / `__iter__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_reversible_direct_subclass_subset` covers the direct subclass `list(reversed(R())) == []` behavior and `issubclass(float, R)` rejection. | None for this method. |
| `test_Collection` | `ported` | `cpython_collections_abc_core_runtime_subset` covers builtin collections, non-collections, structural user classes, inheritance links, direct subclassing, and missing-method non-samples; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__len__` / `__iter__` / `__contains__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_collection_direct_subclass_subset` covers direct subclass iteration, derived subclass iteration, generated-subclass rejection for `list` / `set` / `float`, missing-method non-samples, direct `None` blocking, and inherited `__contains__ = None` blocking. | None for this method. |
| `test_Iterator` | `ported` | `cpython_collections_abc_iterable_iterator_subset` covers builtin iterator samples, iterable inheritance, structural iterator classes, and `__next__`-only rejection; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__next__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_iterator_sample_matrix_subset` covers CPython's public non-sample and sample matrix for `Iterator`, including bytes / str / tuple / list / dict / set / frozenset / dict-view iterators, native generators, generator expressions, and the Issue 10565 `__next__`-only rejection. | None for this method. |
| `test_Generator` | `ported` | `cpython_collections_abc_generator_runtime_subset` covers native generators, structural generator protocol classes, missing-method non-samples, direct subclassing, and inheritance through `Iterator`; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `send` / `throw` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_generator_sample_matrix_subset` covers CPython's public `Generator` non-sample and sample matrix, including native generators, lambda-yield generators, structural `Gen`, and direct `Generator` subclasses; `cpython_collections_abc_generator_mixin_subset` covers direct-subclass `__iter__`, `__next__`, `send`, default `throw`, `close`, `FailOnClose`, and `IgnoreGeneratorExit` mixin behavior. | None for this method. |
| `test_AsyncGenerator` | `ported` | `cpython_collections_abc_generator_runtime_subset` covers native async generators, structural async-generator protocol classes, CPython's `NonAGen1` / `NonAGen2` / `NonAGen3` missing-protocol non-samples, missing-method non-samples, direct subclassing, and inheritance through `AsyncIterator`; `cpython_collections_abc_async_generator_core_mixin_subset` covers direct-subclass `__aiter__` and `__anext__` mixin behavior through `asend(None)`; `cpython_collections_abc_async_generator_throw_close_mixin_subset` covers default `athrow()` and `aclose()` mixin behavior, including ABC mixin calls producing coroutine-typed `Awaitable` / `Coroutine` objects, `.send(None)` drive-to-`StopIteration`, `.close()`, coroutine reuse errors, `athrow(typ)`, explicit exception instances with `tb=None`, real traceback object argument acceptance with CPython's raised traceback replacement behavior, invalid non-traceback `tb` rejection, `GeneratorExit` / `StopAsyncIteration` swallowing, close-time error propagation, and ignored-`GeneratorExit` `RuntimeError`; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `asend` / `athrow` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text. | None for this method. |
| `test_Sized` | `ported` | `cpython_collections_abc_core_runtime_subset` covers builtin sized samples, non-samples, structural `__len__`, and `__len__ = None` blocking; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__len__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_validate_isinstance_subset` covers CPython's `validate_isinstance(Sized, '__len__')` structural helper behavior. | None for this method. |
| `test_Container` | `ported` | `cpython_collections_abc_core_runtime_subset` covers builtin container samples, non-samples, structural `__contains__`, and `__contains__ = None` blocking; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__contains__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_validate_isinstance_subset` covers CPython's `validate_isinstance(Container, '__contains__')` structural helper behavior. | None for this method. |
| `test_Callable` | `ported` | `cpython_collections_abc_core_runtime_subset` covers function, builtin, type, bound-method, structural `__call__`, and non-callable samples; `cpython_collections_abc_abstract_methods_subset` covers complete direct subclass instantiation, missing `__call__` rejection, and direct ABC constructor rejection with exact CPython `TypeError` text; `cpython_collections_abc_validate_isinstance_subset` covers CPython's `validate_isinstance(Callable, '__call__')` structural helper behavior. | None for this method. |
| `test_direct_subclassing` | `ported` | `cpython_collections_abc_direct_subclassing_subset` ports the CPython loop over `Hashable`, `Iterable`, `Iterator`, `Reversible`, `Sized`, `Container`, and `Callable`, covering direct subclass relationships and confirming `int` is not treated as a subclass of each generated subclass. | None for this method. |
| `test_registration` | `ported` | `cpython_collections_abc_registration_subset` ports the public CPython registration loop for `Hashable`, `Iterable`, `Iterator`, `Reversible`, `Sized`, `Container`, and `Callable`, including pre-registration rejection, `register()` returning the class, `issubclass()`, `isinstance()`, and subclass propagation. | None for this method. |

## `Lib/test/test_collections.py::TestCollectionABCs` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_Set` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` covers registered `set` / `frozenset` `Set` relationships, `Set` inheritance through `Collection`, `Sized`, `Iterable`, and `Container`, and explicit `Set` subclass comparison behavior; `cpython_collections_abc_composite_abstract_methods_subset` covers direct `Set()` rejection and incomplete `Set` subclass rejection for `__contains__`, `__iter__`, and `__len__`. | None for this method. |
| `test_hash_Set` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` covers explicit `Set` subclass `_hash()` behavior for equal set contents; `cpython_collections_abc_set_hash_matches_frozenset_subset` broadens `_hash()` parity against `frozenset`. | None for this method. |
| `test_isdisjoint_Set` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` covers `Set.isdisjoint()` returning true for disjoint custom sets and false for overlapping custom sets. | None for this method. |
| `test_equality_Set` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` covers `Set` ordering, equality, inequality, and `NotImplemented` fallback against non-set operands for explicit `Set` subclasses. | None for this method. |
| `test_arithmetic_Set` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` covers `Set.__and__()` returning a custom `Set` result with the expected intersection contents. | None for this method. |
| `test_MutableSet` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` covers registered `set` / `frozenset` `MutableSet` relationships, `MutableSet` inheritance, and explicit mutable-set mixins; `cpython_collections_abc_composite_abstract_methods_subset` covers direct `MutableSet()` rejection and incomplete `MutableSet` subclass rejection for inherited set methods plus `add` / `discard`. | None for this method. |
| `test_issue_5647` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` ports the in-place intersection regression by applying `__iand__()` across two `WithSet` instances without mutating during iteration. | None for this method. |
| `test_issue_4920` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` covers `MutableSet.pop()` removing and returning one existing element while shortening the set. | None for this method. |
| `test_issue8750` | `ported` | `cpython_collections_abc_set_mutable_set_mixins_subset` covers self-subtraction and self-symmetric-difference clearing while preserving self-intersection and self-union. | None for this method. |
| `test_issue16373` | `ported` | `cpython_collections_abc_set_noncomparable_comparison_subset` ports the comparable/non-comparable `Set` comparison fallback matrix. | None for this method. |
| `test_issue26915` | `ported` | `cpython_collections_abc_issue26915_identity_first_object_subset` covers identity-first membership for a `support.NEVER_EQ`-style object and for distinct `float('nan')` objects across explicit `Sequence`, `ItemsView`, `KeysView`, and `ValuesView`, plus `Sequence.index()` / `count()`. | None for this method. |
| `test_Set_from_iterable` | `ported` | `cpython_collections_abc_set_from_iterable_operator_subset` ports normal and in-place `MutableSet` operator dispatch through an instance `_from_iterable()` override. | None for this method. |
| `test_Set_interoperability_with_real_sets` | `ported` | `cpython_collections_abc_set_real_set_interoperability_subset` ports custom `Set` interoperability with real `set` and list operands across binary operators, ordering, equality, inequality, and non-Set ordering `TypeError` paths. | None for this method. |
| `test_Set_hash_matches_frozenset` | `ported` | `cpython_collections_abc_set_hash_matches_frozenset_subset` covers `_hash()` parity for hashable samples including scalars, object identities, NaN, nested frozensets, large integers, range-derived frozensets, and CPython's `sys.maxsize - 10 .. sys.maxsize + 10` range stress sample. | None for this method. |
| `test_Mapping` | `ported` | `cpython_collections_abc_mapping_subset` covers registered `dict` relationships, `Mapping` inheritance, explicit `Mapping` subclassing, and non-structural mapping behavior; `cpython_collections_abc_mapping_mixins_subset` covers comparison and `reversed()` rejection for explicit mapping subclasses; `cpython_collections_abc_composite_abstract_methods_subset` covers direct `Mapping()` rejection and incomplete `Mapping` subclass rejection. | None for this method. |
| `test_MutableMapping` | `ported` | `cpython_collections_abc_mapping_subset` covers registered `dict` `MutableMapping` relationships, inheritance, explicit `MutableMapping` subclassing, and non-structural mutable-mapping behavior; `cpython_collections_abc_mapping_mixins_subset` covers public mutable-mapping mixins; `cpython_collections_abc_composite_abstract_methods_subset` covers direct `MutableMapping()` rejection and incomplete `MutableMapping` subclass rejection. | None for this method. |
| `test_MutableMapping_subclass` | `ported` | `cpython_collections_abc_userdict_view_snapshot_subset` ports `UserDict` keys/items/values view ABC relationships and eager set-operation snapshots that are not affected by later `UserDict` mutation. | None for this method. |
| `test_Sequence` | `ported` | `cpython_collections_abc_sequence_subset` covers supported built-in sequence registrations, `memoryview`, explicit `Sequence` subclassing, non-structural behavior, and inheritance through `Reversible`, `Collection`, `Sized`, `Iterable`, and `Container`; `cpython_collections_abc_composite_abstract_methods_subset` covers direct `Sequence()` rejection and incomplete `Sequence` subclass rejection. | None for this method. |
| `test_Sequence_mixins` | `ported` | `cpython_collections_abc_sequence_mixins_subset` ports `Sequence.index()` parity against native list/str start/stop behavior and covers `count`, `__contains__`, `__iter__`, `__reversed__`, membership fallback, and keyword calls. | None for this method. |
| `test_ByteString` | `ported` | `cpython_collections_abc_bytestring_buffer_subset` covers supported `ByteString` relationships for bytes/bytearray, non-relationships for str/list/tuple/memoryview, direct subclassing, and no metaclass conflict with `Awaitable`; `cpython_collections_abc_composite_abstract_methods_subset` covers direct `ByteString()` rejection and complete/incomplete `ByteString` subclass behavior; `cpython_collections_abc_bytestring_deprecation_warnings_subset` covers public deprecation warnings for import, `isinstance()`, class-statement subclass creation, and dynamic `type(..., (ByteString,), ...)` subclass creation used by CPython's `validate_abstract_methods()` helper. | None for this method. |
| `test_ByteString_attribute_access` | `ported` | `cpython_collections_abc_bytestring_deprecation_warnings_subset` covers fresh `collections.abc.ByteString` attribute access under `warnings.catch_warnings(record=True)`, asserting `DeprecationWarning` and a `ByteString` message. | None for this method. |
| `test_Buffer` | `ported` | `cpython_collections_abc_bytestring_buffer_subset` covers `Buffer` relationships for bytes/bytearray/memoryview, non-relationships for ordinary text/containers, structural `__buffer__` behavior, and `__buffer__ = None` blocking; `cpython_collections_abc_composite_abstract_methods_subset` covers direct `Buffer()` rejection and incomplete `Buffer` subclass rejection. | None for this method. |
| `test_MutableSequence` | `ported` | `cpython_collections_abc_mutable_sequence_subset` covers list, bytearray, `collections.deque`, and `array.array` registrations, non-mutable tuple/str/bytes relationships, inheritance through `Sequence`, `Reversible`, `Collection`, `Sized`, `Iterable`, and `Container`, non-structural protocol behavior, and explicit mutable-sequence mixins; `cpython_collections_abc_composite_abstract_methods_subset` covers direct `MutableSequence()` rejection and incomplete `MutableSequence` subclass rejection. | None for this method. |
| `test_MutableSequence_mixins` | `ported` | `cpython_collections_abc_mutable_sequence_subset` ports explicit `MutableSequence` subclass mixins for `append`, `extend`, `reverse`, `pop`, `remove`, `+=`, `clear`, and self-extension. | None for this method. |
| `test_illegal_patma_flags` | `blocked_by_cpython_internal` | None. | Validates CPython private ABC type flags via `__abc_tpflags__` and `__flags__`; MiniPython should not copy this internal flag representation. |

## `Lib/test/test_collections.py::TestCounter` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_basics` | `ported` | `cpython_collections_counter_basics_subset` ports all current CPython assertions for Counter construction from iterables, mappings, and keywords; dict/Mapping instance and subclass checks; `len`, `values()`, `keys()`, iteration, `items()`, missing-key zero lookup, membership, `get`, equality with dicts and Counters, exact `repr`, `most_common()`, `elements()`, item mutation/deletion, `pop`, `popitem`, `clear`, `fromkeys` rejection, unhashability, `update()`, additive `__init__()`, and `setdefault()`. | None for this method. |
| `test_update_reentrant_add_clears_counter` | `ported` | `cpython_collections_counter_update_reentrant_add_clears_counter_subset` ports CPython's reentrant update case where an int-subclass count clears the Counter from `__add__` before the replacement count is written. | None for this method. |
| `test_init` | `ported` | `cpython_collections_counter_init_update_subset` ports all current CPython `Counter(...)` assertions for `self=` and `iterable=` keyword keys, `iterable=None`, bad positional sources, too many constructor arguments, and unbound `Counter.__init__()` TypeError behavior. | None for this method. |
| `test_total` | `ported` | `cpython_collections_counter_comparison_subset` ports the current CPython assertion that `Counter(a=10, b=5, c=0).total() == 15`. | None for this method. |
| `test_order_preservation` | `ported` | `cpython_collections_counter_order_preservation_subset` ports CPython's insertion-order assertions for Counter construction, tied counts, `elements()`, unary plus/minus, binary multiset operations, in-place multiset operations, `update()`, and `subtract()`. | None for this method. |
| `test_update` | `ported` | `cpython_collections_counter_init_update_subset` ports all current CPython `update()` assertions for `self=` and `iterable=` keyword keys, `iterable=None`, bad positional sources, too many arguments, and unbound `Counter.update()` TypeError behavior. | None for this method. |
| `test_copying` | `ported` | `cpython_collections_counter_copying_subset` ports CPython's `copy()`, `copy.copy()`, `copy.deepcopy()`, pickle round-trip, `eval(repr(...))`, `update(words)`, and `Counter(words)` assertions, plus copy-independence checks after mutation. | None for this method. |
| `test_copy_subclass` | `ported` | `cpython_collections_counter_copy_subclass_subset` ports CPython's Counter subclass construction and `copy()` result-type assertions, and adds an independence check that mutating the copy does not change the original. | None for this method. |
| `test_conversions` | `ported` | `cpython_collections_counter_conversions_subset` ports all current CPython assertions for `sorted(Counter(s).elements())`, sorted Counter iteration, `dict(Counter(s))` versus `dict(Counter(s).items())`, and `set(Counter(s))`. | None for this method. |
| `test_invariant_for_the_in_operator` | `ported` | `cpython_collections_counter_comparison_subset` ports the CPython loop over `Counter(a=10, b=-2, c=0)`, proving every iterated key remains a member even when its count is zero or negative. | None for this method. |
| `test_multiset_operations` | `ported` | `cpython_collections_counter_multiset_operations_subset` covers zero/negative stripping, representative direct dunder dispatch, formulas, and positive-count filtering; `cpython_collections_counter_multiset_operations_matrix_subset` ports the CPython 1000-pair randomized formula matrix with deterministic samples; `cpython_collections_counter_multiset_operations_equivalent_to_set_operations_subset` ports the zero/one-count set-equivalence matrix. | None for this method. |
| `test_inplace_operations` | `ported` | `cpython_collections_counter_inplace_operations_subset` covers deterministic `Counter.__iadd__`, `__isub__`, `__ior__`, `__iand__`, and `__ixor__` behavior, equality with the corresponding binary operation, receiver mutation, and identity preservation via `id()`; `cpython_collections_counter_inplace_operations_matrix_subset` ports the CPython 1000-pair randomized in-place matrix with deterministic samples. | None for this method. |
| `test_subtract` | `ported` | `cpython_collections_counter_subtract_unary_subset` ports all current CPython `subtract()` assertions over keyword counts, Counter sources, iterable sources, negative and zero counts, `self=` / `iterable=` keyword keys, and representative bad-argument TypeErrors. | None for this method. |
| `test_unary` | `ported` | `cpython_collections_counter_subtract_unary_subset` ports CPython unary `+Counter` and `-Counter` count-filtering semantics for positive, zero, and negative counts. | None for this method. |
| `test_repr_nonsortable` | `ported` | `cpython_collections_counter_repr_nonsortable_subset` ports the current CPython assertion that `repr(Counter(a=2, b=None))` includes both `"'a': 2"` and `"'b': None"` instead of failing on non-comparable count values. | None for this method. |
| `test_helper_function` | `ported` | `cpython_collections_counter_helper_function_subset` ports CPython's `_count_elements()` helper behavior for exact dicts, OrderedDict insertion order, and Counter subclasses overriding `__setitem__` or `get`. | None for this method. |
| `test_multiset_operations_equivalent_to_set_operations` | `ported` | `cpython_collections_counter_multiset_operations_equivalent_to_set_operations_subset` ports CPython's full 64-by-64 zero/one-count Counter matrix and checks add, subtract, union, intersection, symmetric difference, and rich comparisons against the equivalent set operations. | None for this method. |
| `test_eq` | `ported` | `cpython_collections_counter_comparison_subset` ports the current CPython equality and inequality assertions, including zero-count equivalence against a missing key. | None for this method. |
| `test_le` | `ported` | `cpython_collections_counter_comparison_subset` ports all current CPython `Counter <= Counter` assertions, including empty-vs-positive and empty-vs-negative count cases. | None for this method. |
| `test_lt` | `ported` | `cpython_collections_counter_comparison_subset` ports both current CPython strict-subset assertions for Counter counts. | None for this method. |
| `test_ge` | `ported` | `cpython_collections_counter_comparison_subset` ports all current CPython `Counter >= Counter` assertions, including empty-vs-negative and empty-vs-positive count cases. | None for this method. |
| `test_gt` | `ported` | `cpython_collections_counter_comparison_subset` ports both current CPython strict-superset assertions for Counter counts. | None for this method. |
| `test_symmetric_difference` | `ported` | `cpython_collections_counter_symmetric_difference_subset` ports CPython's full 9^4 population matrix for Counter symmetric difference, including elementwise absolute-difference invariants, subtract-then-union equivalence, non-negative union-minus-intersection equivalence, positive filtering, input-order preservation, and in-place symmetric-difference parity. | None for this method. |

## `Lib/test/test_grammar.py::TokenTests` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_backslash` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-backslash-line-continuation`, comparing the line-continuation result and comment-backslash behavior directly against CPython. | None for this method. |
| `test_plain_integers` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-plain-integers-method`, covering exact type equality for zero literals, prefixed integer equality, invalid `eval("0x")`, 64-bit `sys.maxsize`, signed min-int equality, positive oversized prefixed literals, and no-overflow `eval()` of huge integer strings. | None for this method. |
| `test_long_integers` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-long-integers-method`, covering every exact large integer literal spelling from the CPython method plus representative equality, ordering, and subtraction checks against CPython. | None for this method. |
| `test_floats` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-floats-method`, covering every exact float literal spelling from the CPython method plus repr/type/equality checks against CPython. | None for this method. |
| `test_float_exponent_tokenization` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-float-exponent-tokenization-method` for both accepted lowercase `else` boundary spellings, and `cpython_rejection_parity_smoke_diff_subset` includes `grammar-token-float-exponent-tokenization-uppercase-else` for the rejected non-keyword `Else` spelling. | None for this method. |
| `test_underscore_literals` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-underscore-literals-method`, proving the full CPython `VALID_UNDERSCORE_LITERALS` table evaluates like its underscore-free spelling, the full `INVALID_UNDERSCORE_LITERALS` table raises `SyntaxError`, and `_0` raises `NameError`. | None for this method. |
| `test_bad_numerical_literals` | `ported` | `cpython_bad_numerical_literals_subset` rejects the source forms, and `cpython_syntax_error_message_parity_diff_subset` now includes all 18 CPython `check(...)` cases from this method with matching CPython/MiniPython error-message parity. | None for this method. |
| `test_end_of_numerical_literals` | `ported` | `cpython_end_of_numerical_literals_subset` now covers every CPython source generated by this method: accepted numeric literals, warning cases for keyword/soft-keyword boundaries, direct error cases without warnings, warning-as-error behavior for `is`, non-ASCII fraction-slash rejection, and the three hexadecimal list-comprehension boundary forms. | None for this method. |
| `test_string_literals` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-string-literals-method`, covering the exact CPython assertions for empty strings, quote escaping, `ord()` values, double/single quote equivalence, triple-quoted strings, and explicit backslash line joining. | None for this method. |
| `test_string_prefixes` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-string-prefixes-method`, mirroring CPython's `eval()`-based checks for `u`, `r`, `rf`, and `fr` string prefixes and proving each parses to non-empty `str` values with CPython-matching repr output. | None for this method. |
| `test_bytes_prefixes` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-bytes-prefixes-method`, mirroring CPython's `eval()`-based checks for `b`, `br`, and `rb` bytes prefixes and proving each parses to non-empty `bytes` values with CPython-matching repr output. | None for this method. |
| `test_ellipsis` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-ellipsis-method`, proving `... is Ellipsis` and `eval(".. .")` raises `SyntaxError` like CPython. | None for this method. |
| `test_eof_error` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes all three CPython samples and proves MiniPython reports `was never closed` for each unterminated function-header parenthesis. | None for this method. |
| `test_max_level` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `grammar-token-max-level-method`, proving 200 nested parentheses evaluate to `()` and 201 nested parentheses raise `SyntaxError` with `too many nested parentheses`. | None for this method. |

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
| `test_expression_with_assignment` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `expression-with-assignment-message`, proving the CPython message for assignment inside a call argument, and `cpython_invalid_call_argument_helper_rules_subset` pins the diagnostic span to the same argument start offset. | None for this method. |
| `test_curly_brace_after_primary_raises_immediately` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes `syntax-error-curly-brace-after-primary-single-mode`, proving `compile("f{}", "<testcase>", "single")` raises `SyntaxError` with `invalid syntax`; `cpython_interactive_input_subset` pins the MiniPython interactive entry point to the same rejection. | None for this method. |
| `test_assign_call` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `assign-call-message`, proving `f() = 1` raises a SyntaxError whose text contains `assign`, matching CPython's method-level assertion. | None for this method. |
| `test_assign_del` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes all 27 invalid-delete cases from this CPython method, covering invalid empty targets, literals, `None`, starred targets, function-call targets, conditional/named expressions, ordinary expressions, and `del a += b`. | None for this method. |
| `test_global_param_err_first` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `global-param-error-first-message`, proving the first reported error contains `parameter and global`; `cpython_scope_declaration_error_subset` pins the MiniPython compile diagnostic to line 3 at the `global` keyword. | None for this method. |
| `test_nonlocal_param_err_first` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `nonlocal-param-error-first-message`, proving the first reported error contains `parameter and nonlocal`; `cpython_scope_declaration_error_subset` pins the MiniPython compile diagnostic to line 3 at the `nonlocal` keyword. | None for this method. |
| `test_raise_from_error_message` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `raise-from-following-invalid-call-message`, proving the valid `raise AssertionError() from None` line does not mask the next-line call syntax error; `cpython_invalid_call_argument_helper_rules_subset` pins the invalid-call diagnostic to line 3 at the second comma. | None for this method. |
| `test_yield_outside_function` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes all 9 `yield`-outside-function cases from this method across `if`, `else`, `while`, class body, and loop `else` contexts; `cpython_invalid_control_flow_context_subset` pins MiniPython compile errors for each shape. | None for this method. |
| `test_return_outside_function` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes all 9 `return`-outside-function cases from this method across `if`, `else`, `while`, class body, and loop `else` contexts; `cpython_invalid_control_flow_context_subset` pins MiniPython compile errors for each shape. | None for this method. |
| `test_break_outside_loop` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes all 7 `break`-outside-loop cases from this method across module, `if`, class body, and `with` contexts; `cpython_invalid_control_flow_context_subset` pins MiniPython compile errors and line-specific diagnostics for each shape. | None for this method. |
| `test_continue_outside_loop` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes all 6 `continue`-outside-loop cases from this method across `if`, class body, and `with` contexts; `cpython_invalid_control_flow_context_subset` pins MiniPython's CPython-style `not properly in loop` compile errors and line-specific diagnostics for each shape. | None for this method. |
| `test_unexpected_indent` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the CPython `foo()` then indented `bar()` shape and proves both CPython and MiniPython report `unexpected indent`; `cpython_invalid_control_flow_context_subset` keeps a local rejection case for the same parse path. | None for this method. |
| `test_no_indent` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `if 1:\nfoo()` and `cpython_invalid_block_subset` covers the same no-suite semantic across supported compound statements with `expected an indented block`. | None for this method. |
| `test_bad_outdent` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `if 1:\n  foo()\n bar()` and `cpython_tokenize_unmatched_indentation_subset` covers matching-dedent rejection with CPython-style `unindent does not match` wording. | None for this method. |
| `test_kwargs_last` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `int(base=10, '2')`, and `cpython_invalid_call_argument_helper_rules_subset` keeps the same positional-after-keyword parse rejection. | None for this method. |
| `test_kwargs_last2` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `int(**{'base': 10}, '2')`, and `cpython_invalid_call_argument_helper_rules_subset` keeps the same positional-after-keyword-unpacking parse rejection. | None for this method. |
| `test_kwargs_last3` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes `int(**{'base': 10}, *['2'])`, and `cpython_invalid_call_argument_helper_rules_subset` keeps the same iterable-unpacking-after-keyword-unpacking parse rejection. | None for this method. |
| `test_generator_in_function_call` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the exact CPython source shape, and `cpython_invalid_call_argument_helper_rules_subset` pins MiniPython's diagnostic to line 1 columns 11-53 for the unparenthesized generator expression. | None for this method. |
| `test_except_then_except_star` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the exact CPython mixed-handler source, and `cpython_invalid_control_flow_syntax_subset` pins MiniPython's diagnostic to line 3 columns 1-8 for `except*`. | None for this method. |
| `test_except_star_then_except` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the exact CPython mixed-handler source, and `cpython_invalid_control_flow_syntax_subset` pins MiniPython's diagnostic to line 3 columns 1-7 for `except`. | None for this method. |
| `test_empty_line_after_linecont` | `ported` | `cpython_program_output_parity_smoke_diff_subset` includes the empty physical line after a continuation character; `cpython_tokenize_explicit_line_joining_subset` keeps local executable coverage for both CPython source shapes, including the split-indented-function shape that local Python 3.9 rejects but the checked-out CPython source expects to compile. | None for this method. |
| `test_continuation_bad_indentation` | `ported` | `cpython_tokenize_explicit_line_joining_subset` includes the exact CPython source shape and rejects it with MiniPython's CPython-style bad-outdent diagnostic. It is intentionally not in the system-Python differential suite because the local Python 3.9 accepts this shape while the checked-out CPython test expects rejection. | None for this method. |
| `test_disallowed_type_param_names` | `ported` | `cpython_disallowed_type_param_names_subset` rejects `__classdict__` for class, function, and type-alias type parameters while preserving parse acceptance for nested `__class__`, `__classcell__`, and `__classdictcell__` type parameters. It is intentionally subset-only because local Python 3.9 does not parse PEP 695 type parameters. | None for this method. |
| `test_barry_as_flufl_with_syntax_errors` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the exact regression source and targets current CPython's `expected ':'` parser message, while accepting local Python 3.9's legacy `invalid syntax`; `cpython_invalid_control_flow_syntax_subset` keeps local parser rejection coverage. | None for this method. |
| `test_invalid_line_continuation_error_position` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes all three CPython invalid-continuation source shapes, and `cpython_tokenize_explicit_line_joining_subset` pins the MiniPython lexer diagnostics to the CPython line/column positions. | None for this method. |
| `test_invalid_line_continuation_left_recursive` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the invalid trailing-space left-recursive continuation shape; `cpython_tokenize_explicit_line_joining_subset` pins MiniPython lexer diagnostics for both the invalid trailing space and continuation-at-EOF cases. The EOF shape is subset-only because local Python 3.9 executes it as a `NameError` while the checked-out CPython source expects a compile-time EOF error. | None for this method. |
| `test_error_parenthesis` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes unclosed `(`, `[`, `{`, unmatched `)`, `]`, `}`, and the nested mismatched opening-parenthesis example; `cpython_syntax_error_parenthesis_subset` covers the full CPython method shape including unclosed delimiters before a following assignment-like line, the `match` class-pattern EOF case, and the latin-cookie bytes source. `cpython_bytes_source_rejection_parity_diff_subset` keeps the bytes-source rejection aligned with CPython. | None for this method. |
| `test_error_string_literal` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes all six CPython source shapes for ordinary, escaped-end-quote, raw escaped-end-quote, and triple-quoted unterminated strings; `cpython_invalid_string_literal_subset` pins MiniPython lexer diagnostics and spans for the same shapes plus existing tokenizer string-error cases. | None for this method. |
| `test_invisible_characters` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the string-source `print\x17("Hello")` invalid non-printable character diagnostic, `cpython_tokenize_error_token_subset` pins both string and bytes-source lexer spans from the CPython method, and `cpython_bytes_source_rejection_parity_diff_subset` keeps the bytes-source rejection aligned with CPython. | None for this method. |
| `test_match_call_does_not_raise_syntax_error` | `ported` | `cpython_program_output_parity_smoke_diff_subset` runs the exact CPython compile-only source through CPython and MiniPython, and `cpython_soft_keyword_call_acceptance_subset` keeps local soft-keyword call coverage. | None for this method. |
| `test_case_call_does_not_raise_syntax_error` | `ported` | `cpython_program_output_parity_smoke_diff_subset` runs the exact CPython compile-only source through CPython and MiniPython, and `cpython_soft_keyword_call_acceptance_subset` keeps local soft-keyword call coverage. | None for this method. |
| `test_multiline_compiler_error_points_to_the_end` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the exact multiline duplicate-keyword call source with CPython's `keyword argument repeated` wording, and `cpython_invalid_call_argument_helper_rules_subset` pins MiniPython's diagnostic to line 3 at the repeated keyword. | None for this method. |
| `test_multiline_string_concat_missing_comma_points_to_last_string` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the exact multiline string-concatenation missing-comma source and targets CPython's `Perhaps you forgot a comma` wording, while accepting local Python 3.9's legacy `invalid syntax`; `cpython_multiline_string_concat_missing_comma_subset` pins MiniPython's diagnostic to line 4 at the final adjacent string token. | None for this method. |
| `test_except_stmt_invalid_as_expr` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the exact `except ValueError as obj.attr` source and expects CPython's `cannot use except statement with attribute` wording, while `cpython_invalid_control_flow_syntax_subset` pins MiniPython's diagnostic span to the full `obj.attr` target. | None for this method. |
| `test_match_stmt_invalid_as_expr` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the exact `case x as obj.attr` source and expects CPython's `cannot use attribute as pattern target` wording, while `cpython_invalid_match_pattern_subset` pins MiniPython's diagnostic span to the full `obj.attr` target. | None for this method. |
| `test_ifexp_else_stmt` | `ported` | `cpython_invalid_expression_rules_subset` rejects every CPython statement keyword after conditional-expression `else`, and `cpython_syntax_error_message_parity_diff_subset` covers the current CPython message for statement tokens in this area. | None for this method. |
| `test_ifexp_body_stmt_else_expression` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes the CPython `pass`, `break`, and `continue` statement-body shapes, and `cpython_invalid_expression_rules_subset` pins MiniPython's `expected expression before 'if', but statement is given` diagnostic. | None for this method. |
| `test_ifexp_body_stmt_else_stmt` | `ported` | `cpython_syntax_error_message_parity_diff_subset` includes a statement body with a statement `else` branch, and `cpython_invalid_expression_rules_subset` covers all CPython body/else statement pairs. | None for this method. |
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

## `Lib/test/test_complex.py::ComplexTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_truediv` | `ported` | `cpython_complex_division_subset`, `cpython_complex_truediv_nonfinite_diff_subset`, and `cpython_complex_truediv_extreme_inverse_diff_subset` cover small-grid inverse checks, huge/tiny inverse checks, direct `complex.__truediv__()`, mixed float/complex division, NaN denominators, non-finite quotient recovery, signed-zero results, and TypeError paths for unsupported operands. | None for the public deterministic method surface. |
| `test_truediv_zero_division` | `ported` | `cpython_complex_division_subset` covers the CPython zero-denominator operand matrix as catchable `ZeroDivisionError`. | None for this method. |
| `test_floordiv` | `ported` | `cpython_complex_division_subset` covers complex floor-division rejection across complex/real and real/complex operand pairs. | None for this method. |
| `test_floordiv_zero_division` | `ported` | `cpython_complex_division_subset` and `cpython_complex_division_unsupported_zero_diff_subset` cover the zero-denominator operand matrix as catchable `TypeError` because floor division is unsupported for complex operands. | None for this method. |
| `test_richcompare` | `ported` | `cpython_complex_richcompare_subset` covers equality and inequality dunders, ordering dunders returning `NotImplemented`, operator fallback `TypeError`, float equality for real-valued complex numbers, and unrelated equality operands. | None for this method. |
| `test_richcompare_boundaries` | `ported` | `cpython_complex_richcompare_boundaries_subset` covers the large-integer boundary matrix where real-valued complex equality must avoid lossy float rounding. | None for this method. |
| `test_add` | `ported` | `cpython_complex_add_sub_mul_subset` covers integer operands, signed-zero preservation, huge-integer `OverflowError`, and unsupported-operand `TypeError` paths. | None for this method. |
| `test_sub` | `ported` | `cpython_complex_add_sub_mul_subset` covers integer operands, complex/complex subtraction, signed-zero preservation, huge-integer `OverflowError`, and unsupported-operand `TypeError` paths. | None for this method. |
| `test_mul` | `ported` | `cpython_complex_add_sub_mul_subset` and `cpython_complex_mul_nonfinite_diff_subset` cover integer operands, signed-zero and non-finite multiplication behavior, huge-integer `OverflowError`, unsupported-operand `TypeError`, and CPython's non-finite complex-by-complex recovery matrix. | None for this method. |
| `test_mod` | `ported` | `cpython_complex_division_subset` covers complex modulo rejection across complex/real and real/complex operand pairs. | None for this method. |
| `test_mod_zero_division` | `ported` | `cpython_complex_division_subset` and `cpython_complex_division_unsupported_zero_diff_subset` cover the zero-denominator operand matrix as catchable `TypeError` because modulo is unsupported for complex operands. | None for this method. |
| `test_divmod` | `ported` | `cpython_complex_division_subset` covers `divmod()` rejection across complex/real and real/complex operand pairs. | None for this method. |
| `test_divmod_zero_division` | `ported` | `cpython_complex_division_subset` and `cpython_complex_division_unsupported_zero_diff_subset` cover the zero-denominator operand matrix as catchable `TypeError` because `divmod()` is unsupported for complex operands. | None for this method. |
| `test_pow` | `ported` | `cpython_complex_pow_subset`, `cpython_complex_pow_zero_and_stress_diff_subset`, and `cpython_complex_pow_overflow_boundary_diff_subset` cover zero exponent and zero-base behavior, complex/integer exponent paths, direct `complex.__pow__()`, three-argument modulo rejection, zero-to-negative-or-complex `ZeroDivisionError`, TypeError paths, self-comparison stress rows, overflow rows, and the boundary no-crash matrix. | None for this method's public behavior. |
| `test_pow_with_small_integer_exponents` | `ported` | `cpython_complex_pow_small_integer_exponents_subset` covers string-result parity across int, integral-float, and zero-imaginary complex exponent forms for the CPython finite, infinite, negative, zero, and overflow cases. | None for this method. |
| `test_boolcontext` | `ported` | `cpython_complex_bool_and_special_numbers_subset` covers truthiness for non-zero complex values, zero complex values, `1j`, and direct `__bool__()` calls. | None for this method's public deterministic behavior. |
| `test_conjugate` | `ported` | `cpython_complex_method_subset` covers bound `conjugate()`, method visibility through `dir()`, and signed-zero conjugation behavior. | None for this method. |
| `test_constructor` | `ported` | `cpython_complex_constructor_basic_subset`, `cpython_complex_constructor_protocol_subset`, `cpython_complex_subclass_constructor_and_from_number_subset`, and `cpython_complex_bool_and_special_numbers_subset` cover exact built-in construction, exact-object identity for `complex(c)`, `complex.__new__(complex, c)`, `c.__complex__()`, and unary plus, keyword forms, signed zero, numeric protocol conversion, CPython's two-argument and keyword-real `DeprecationWarning` compatibility paths, strict-complex-subclass `__complex__` result warnings, subclass construction, custom `__new__`, special numbers, public TypeError/OverflowError paths, and custom exception propagation. | None for this method's public behavior. |
| `test___complex__` | `ported` | `cpython_complex_method_subset` and `cpython_complex_subclass_constructor_and_from_number_subset` cover bound `__complex__()` for exact complex values and subclasses returning exact complex results. | None for this method. |
| `test_constructor_special_numbers` | `ported` | `cpython_complex_bool_and_special_numbers_subset` covers exact complex and complex subclass construction across signed zero, infinities, and NaNs while preserving component signs and subclass identity. | None for this method. |
| `test_constructor_from_string` | `ported` | `cpython_complex_constructor_from_string_subset` covers real-only, imaginary-only, signed unit imaginary, real-plus-imaginary, parenthesized, Unicode-whitespace-wrapped, long, signed-underflow, overflow, and malformed string cases. | None for this method. |
| `test_constructor_negative_nans_from_string` | `ported` | `cpython_complex_bool_and_special_numbers_subset` covers negative NaN signs in real and imaginary string components. | None for this method. |
| `test_underscores` | `ported` | `cpython_complex_string_underscore_and_literal_subset` covers the full CPython shared `VALID_UNDERSCORE_LITERALS` and `INVALID_UNDERSCORE_LITERALS` matrix after applying the same `ComplexTest` filters for non-base literal spellings and skipped old-octal forms. | None for this method. |
| `test_from_number` | `ported` | `cpython_complex_from_number_subset` covers exact built-in `complex.from_number()` over int, float, exact complex with CPython object identity preservation, complex subclasses, `__complex__`, `__float__`, `__index__`, NaN self-inequality, and TypeError rejection paths. | None for this method. |
| `test_from_number_subclass` | `ported` | `cpython_complex_subclass_constructor_and_from_number_subset` covers complex subclass `from_number()` preserving subclass identity and custom subclass `__new__` behavior. | None for this method's public subclass behavior. |
| `test_hash` | `ported` | `cpython_complex_hash_subset` covers real-valued complex hash invariants against integer and floating values plus the `2000005 - 1j` non-`-1` invariant. | None for this method. |
| `test_abs` | `ported` | `cpython_complex_abs_subset` covers finite magnitude checks, bound and unbound `__abs__()`, and finite-component magnitude overflow raising `OverflowError`. | None for this method. |
| `test_repr_str` | `ported` | `cpython_complex_repr_str_subset` covers ordinary, infinity, NaN, negative-NaN normalization, pure imaginary non-finite spellings, direct `__repr__()` / `__str__()`, and repr-to-complex round trips. | None for this method. |
| `test_negative_zero_repr_str` | `ported` | `cpython_complex_repr_str_subset` covers CPython's signed-zero real and imaginary `repr()` / `str()` spellings. | None for this method. |
| `test_pos` | `ported` | `cpython_complex_unary_subset` and `cpython_complex_subclass_constructor_and_from_number_subset` cover unary plus on exact complex and complex subclasses returning exact complex results. | None for this method. |
| `test_neg` | `ported` | `cpython_complex_unary_subset` covers unary negation and direct `__neg__()` lookup. | None for this method. |
| `test_getnewargs` | `ported` | `cpython_complex_getnewargs_subset` covers `__getnewargs__()` returning real and imaginary floats while preserving signed zero and infinity values. | None for this method. |
| `test_plus_minus_0j` | `ported` | `cpython_complex_string_underscore_and_literal_subset` covers `0j` and `-0j` preserving distinct imaginary zero signs. | None for this method. |
| `test_negated_imaginary_literal` | `ported` | `cpython_complex_string_underscore_and_literal_subset` covers negated imaginary literals preserving `-0.0` real components, finite imaginary values, and overflowing negative imaginary literals. | None for this method. |
| `test_overflow` | `ported` | `cpython_complex_overflow_subset` covers string-constructor overflow producing infinities for real-only, imaginary-only, and real-plus-imaginary forms. | None for this method. |
| `test_repr_roundtrip` | `ported` | `cpython_complex_string_underscore_and_literal_subset` covers `complex(repr(z))` and `eval(repr(z))` round trips across the signed-zero, subnormal, finite, infinity, and NaN matrix. | None for this method. |
| `test_format` | `ported` | `cpython_complex_format_subset` covers the complete CPython deterministic complex formatting matrix for empty and omitted presentation types, sign handling, precision, alignment, alternate form, comma grouping, large finite values, NaN/Inf casing, direct `__format__()`, `str.format()`, integer presentation rejection, zero-padding rejection, and `=` alignment rejection. | None for this method. |

## `Lib/test/test_float.py::GeneralFloatCases` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_float` | `ported` | `cpython_float_constructor_core_subset` covers ordinary numeric/string construction, whitespace, Unicode digits, bad hexadecimal-like spellings, malformed signs, bad NaN/Inf dot forms, invalid strings, embedded NULs, and bytes parsing failures. | None for the covered public constructor behavior. |
| `test_noargs` | `ported` | `cpython_float_constructor_core_subset` covers `float()` returning `0.0`. | None for this method. |
| `test_underscores` | `ported` | `cpython_float_string_underscore_subset` covers the CPython valid and invalid underscore literal matrix after applying the same float-specific filters, plus NaN/Inf underscore rejection. | None for this method. |
| `test_non_numeric_input_types` | `ported` | `cpython_float_bytes_like_input_types_subset` covers `bytes`, `bytearray`, string/bytes/bytearray subclasses, `memoryview`, `array.array('B')`, and invalid bytes-like parsing error classes. | None for this method's public behavior. |
| `test_float_memoryview` | `ported` | `cpython_float_bytes_like_input_types_subset` covers sliced `memoryview` parsing with trailing NUL, spaces, letters, and extra source bytes outside the slice. | None for this method. |
| `test_error_message` | `ported` | `cpython_float_constructor_core_subset`, `cpython_float_bytes_like_input_types_subset`, and `cpython_runtime_error_message_parity_diff_subset` cover representative invalid string, bytes, all-whitespace, NUL, and non-UTF-8 `ValueError` message surfaces. | None for the portable message shapes currently covered. |
| `test_float_with_comma` | `partial` | `cpython_float_constructor_core_subset` covers comma-containing strings staying invalid and ordinary dot-decimal syntax staying accepted. | Full parity needs locale-controlled `LC_NUMERIC` runs for comma decimal-point locales. |
| `test_floatconversion` | `ported` | `cpython_float_conversion_protocol_subset` covers `__float__()` dispatch, exact and subclass float results, subclass constructor behavior, `str` subclass `__float__`, non-float return rejection, and keyword-argument rejection. | None for this method's public conversion behavior. |
| `test_keyword_args` | `ported` | `cpython_rejection_parity_smoke_diff_subset` includes `floatconversion` keyword-argument rejection for `float(x='3.14')`. | None for this method. |
| `test_keywords_in_subclass` | `ported` | `cpython_float_keywords_in_subclass_subset` covers float subclass construction, keyword rejection when the subclass does not accept it, `__init__` keyword handling, and subclass `__new__` keyword handling. | None for this method. |
| `test_from_number` | `ported` | `cpython_float_from_number_subset` covers exact float results, float subclasses, `__float__`, `__index__`, NaN handling, method visibility, instance calls, invalid argument classes, arity/keyword errors, and huge-index overflow. | None for CPython oracles that expose `float.from_number()`. |
| `test_from_number_subclass` | `ported` | `cpython_float_from_number_subset` covers subclass `from_number()` returning the subclass type for float and index inputs. | None for CPython oracles that expose `float.from_number()`. |
| `test_is_integer` | `ported` | `cpython_float_ratio_and_component_methods_subset` covers finite integer/non-integer values plus NaN and infinity returning `False`. | None for this method. |
| `test_floatasratio` | `ported` | `cpython_float_ratio_and_component_methods_subset` covers exact public ratio rows for finite values, integer-valued floats, NaN `ValueError`, and infinity `OverflowError`. | The original random stress loop is represented by deterministic public rows. |
| `test_float_containment` | `ported` | `cpython_float_containment_subset` covers list, tuple, set, dict, repeated tuple, equal-but-nonidentical containers, and identical NaN object membership/count behavior. | None for this method. |
| `test_float_floor` | `ported` | `cpython_float_floor_ceil_subset` covers bound and unbound `__floor__()`, ordinary finite rows, huge finite rows, NaN `ValueError`, infinity `OverflowError`, and `math.floor()` parity. | None for this method. |
| `test_float_ceil` | `ported` | `cpython_float_floor_ceil_subset` covers bound and unbound `__ceil__()`, ordinary finite rows, huge finite rows, NaN `ValueError`, infinity `OverflowError`, and `math.ceil()` parity. | None for this method. |
| `test_float_mod` | `ported` | `cpython_float_mod_signed_zero_subset` covers `%` and `operator.mod()` signed-zero preservation, tiny finite remainders, and modulo-by-zero rejection. | None for this method's public deterministic rows. |
| `test_float_pow` | `ported` | `cpython_float_pow_special_cases_subset` covers the active CPython C99 F.9.4.4 matrix for NaN identities, infinities, signed zero, zero-to-finite-negative `ZeroDivisionError`, negative-infinity fractional powers staying real, finite negative-base fractional powers producing complex results, large `+/-1` exponents, and underflow sign rows across `**`, `pow()`, and `operator.pow()`. | None for this method's public active assertions. |
| `test_hash` | `ported` | `cpython_float_hash_and_sys_info_subset` covers integer/floating hash equality for small ints, `sys.float_info.max`, infinities, and `sys.hash_info` constants. | None for this method. |
| `test_hash_nan` | `ported` | `cpython_float_hash_and_sys_info_subset` covers NaN hash integer behavior and float-subclass NaN hashing ignoring a mixin `__hash__`. | None for this method. |
| `test_issue_gh143006` | `ported` | `cpython_float_int_comparison_boundaries_subset` covers the `EvilInt.__neg__` regression and additional exact float/int comparison boundaries around large integers and NaN ordering. | None for this method. |

## `Lib/test/test_float.py::FormatFunctionsTestCase` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_getformat` | `ported` | `cpython_float_getformat_subset` covers `double` and `float` format strings, invalid name and type errors, `dir()` visibility, class/instance access, and subclass inheritance. | None for this method. |

## `Lib/test/test_float.py::IEEEFormatTestCase` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_double_specials_do_unpack` | `blocked_by_runtime` | None. | Requires `struct.unpack()` IEEE double parsing for binary special values. |
| `test_float_specials_do_unpack` | `blocked_by_runtime` | None. | Requires `struct.unpack()` IEEE single-float parsing for binary special values. |
| `test_serialized_float_rounding` | `blocked_by_cpython_internal` | None. | Requires CPython `_testcapi.FLT_MAX` and `struct.pack()` single-float rounding details. |

## `Lib/test/test_float.py::FormatTestCase` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_format` | `ported` | `cpython_float_default_precision_format_subset`, `cpython_float_fractional_grouping_format_subset`, `cpython_float_format_specials_subset`, and `cpython_types_float_format_subset` cover empty/default/general presentation, invalid float presentation codes, NaN/Inf casing, integer and fractional grouping, width/alignment, sign, zero padding, scientific notation, and invalid grouping combinations. | None for this method's public deterministic assertions. |
| `test_format_testfile` | `ported` | `cpython_float_format_testfile_full_subset` and `cpython_float_format_testfile_full_diff_subset` cover the complete local CPython `mathdata/formatfloat_testcases.txt` dataset across old-style `%` and `format()` for `f`, `e`, `g`, `%r`, alternate form, half-even rounding, precision-zero decimal preservation, and negative-value variants. | None for this method against the local CPython data file. |
| `test_issue5864` | `ported` | `cpython_float_default_precision_format_subset` covers explicit precision with empty presentation type and CPython-style general-format switching. | None for this method. |
| `test_issue35560` | `ported` | `cpython_float_zero_width_format_subset` covers zero-width float formatting across empty, fixed, exponent, and general presentation types with positive and negative values. | None for this method. |

## `Lib/test/test_float.py::ReprTestCase` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_repr` | `ported` | `cpython_float_repr_roundtrip_full_subset` and `cpython_float_repr_roundtrip_full_diff_subset` cover the complete local CPython `mathdata/floating_points.txt` dataset for `repr(float)` round trips, including signed zero, subnormal-scale rows, large finite rows, and historical crash-regression values. | None for this method against the local CPython data file. |
| `test_short_repr` | `ported` | `cpython_float_short_repr_subset` covers CPython's finite short-repr matrix, `str()`/`repr()` parity, round trips, negative variants, and `sys.float_repr_style == 'short'`. | None for this method. |

## `Lib/test/test_float.py::RoundTestCase` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_inf_nan` | `ported` | `cpython_float_round_specials_subset` covers no-`ndigits` rounding of infinities and NaNs raising the expected catchable errors. | None for this method. |
| `test_inf_nan_ndigits` | `ported` | `cpython_float_round_specials_subset` covers `ndigits=0` preserving infinities and NaNs as floats plus bad `ndigits` type errors. | None for this method. |
| `test_large_n` | `ported` | `cpython_float_round_specials_subset` covers the complete CPython large-positive-`ndigits` grid: 28 looped finite rows plus 5 explicit boundary rows, including ordinary values, very large finite values, subnormal values, and the `1.4e-315` to `1e-315` row. | None for this method. |
| `test_small_n` | `ported` | `cpython_float_round_specials_subset` covers the complete CPython large-negative-`ndigits` grid: 28 looped rows across positive, negative, very large, and subnormal values, preserving `-0.0` for negative inputs. | None for this method. |
| `test_overflow` | `ported` | `cpython_float_round_specials_subset` covers positive and negative overflow rows for large negative `ndigits`. | None for this method. |
| `test_previous_round_bugs` | `ported` | `cpython_float_round_specials_subset` covers the historical 562949953421312.5 / 56294995342131.5 rows and half-even tens cases. | None for this method. |
| `test_matches_float_format` | `ported` | `cpython_float_round_matches_format_subset` covers the CPython thousandths grid, half-cent grid, and 500 deterministic pseudo-random replacement values, checking `round(x, n)` against `float(format(x, ".nf"))` for `n = 0..3` across 6000 total comparison points. The matching `cpython_float_round_matches_format_diff_subset` compares the same deterministic sweep against the local CPython oracle. | None for this method's public consistency invariant. |
| `test_format_specials` | `ported` | `cpython_float_format_specials_subset` covers percent-format and `format()` spellings for NaN/Inf under ordinary, plus-sign, space-sign, alternate, fixed, exponent, and general forms. | None for this method. |
| `test_None_ndigits` | `ported` | `cpython_float_round_specials_subset` covers `round(x, None)` and keyword `ndigits=None`. | None for this method. |
| `test_round_with_none_arg_direct_call` | `ported` | `cpython_float_round_dunder_none_subset` covers bound, descriptor, subclass, bad receiver, bad type, and keyword paths for `float.__round__()`. | None for this method. |

## `Lib/test/test_float.py::InfNanTest` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_inf_from_str` | `ported` | `cpython_float_inf_nan_string_subset` covers case-insensitive `inf` / `infinity` parsing with optional signs and malformed infinity spellings. | None for this method. |
| `test_inf_as_str` | `ported` | `cpython_float_inf_nan_string_subset` covers `repr()` and `str()` for positive and negative infinity. | None for this method. |
| `test_nan_from_str` | `ported` | `cpython_float_inf_nan_string_subset` covers case-insensitive `nan` parsing with optional signs and malformed NaN spellings. | None for this method. |
| `test_nan_as_str` | `ported` | `cpython_float_inf_nan_string_subset` covers `repr()` and `str()` for NaN and negative NaN. | None for this method. |
| `test_inf_signs` | `ported` | `cpython_float_inf_nan_string_subset` covers sign preservation for parsed infinities and arithmetic infinity rows through `math.copysign()`. | None for this method. |
| `test_nan_signs` | `ported` | `cpython_float_inf_nan_string_subset` covers sign preservation for parsed NaNs and arithmetic NaN rows through `math.copysign()`. | None for this method. |

## `Lib/test/test_float.py::HexFloatTestCase` Method Audit

| CPython method | Status | Rust evidence | Remaining acceptance |
| --- | --- | --- | --- |
| `test_ends` | `ported` | `cpython_float_hex_fromhex_first_pass_subset` covers MIN, TINY, EPS, and MAX endpoint construction and `hex()` spelling. | None for this method. |
| `test_invalid_inputs` | `ported` | `cpython_float_hex_fromhex_invalid_inputs_subset` covers malformed infinity/NaN spellings, bad hexadecimal structure, bad underscores, bad exponents, embedded NULs, non-string receiver errors, and non-float class argument errors. | None for the invalid categories currently in the CPython method. |
| `test_whitespace` | `ported` | `cpython_float_hex_fromhex_ends_whitespace_subset` covers accepted leading/trailing whitespace and rejected embedded whitespace around representative finite and special values. | None for this method. |
| `test_from_hex` | `ported` | `cpython_float_fromhex_accepted_variants_subset`, `cpython_float_fromhex_overflow_zero_underflow_subset`, `cpython_float_fromhex_rounding_boundaries_subset`, and `cpython_float_fromhex_bpo44954_regression_subset` cover the complete deterministic CPython method matrix: accepted spellings, infinities/NaNs, overflow, zeros, underflow, half-even rounding boundaries, and the bpo-44954 rows. `cpython_test_manifest_float_fromhex_matrix_inputs_have_runtime_evidence` guards that all 262 current local CPython `fromHex(...)` inputs have runtime evidence in the Rust subset/diff tests. | None for the deterministic `test_from_hex` matrix. |
| `test_roundtrip` | `ported` | `cpython_float_hex_fromhex_roundtrip_matrix_subset` covers endpoint, signed-zero, infinity, NaN, max/min/subnormal boundaries, and a deterministic 10,000-row exponent/mantissa/sign sweep with CPython-style overflow skipping. The matching `float-hex-fromhex-roundtrip-matrix` differential case compares the same sweep against the local CPython oracle. | None for this method's public round-trip invariant. |
| `test_subclass` | `ported` | `cpython_float_hex_fromhex_subclass_subset` covers float subclass `fromhex()` dispatch, `__new__`, `__init__`, and subclass return type behavior. | None for this method. |

## Immediate Method-Level Audit Order

1. Continue public behavior migration in partial runtime rows that already have
   a working foundation, especially `Lib/test/test_bytes.py::BaseBytesTest` and
   `Lib/test/test_memoryview.py` direct methods.
2. Revisit `AST_Tests` only after public weakref/cyclic-GC support exists; its
   remaining unported rows are blocked runtime or CPython-internal
   implementation checks.

The acceptance bar for moving a row to `ported` is deliberately high: every
method in the row needs a named Rust test or documented differential parity
case, and the relevant command must pass in `cargo test`. A row can move to
`ported_public` only when every remaining unported method is explicitly
classified as CPython-internal implementation coverage.
