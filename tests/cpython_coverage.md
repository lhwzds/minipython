# CPython Grammar Coverage

This matrix tracks MiniPython coverage against the local CPython checkout:

- Grammar: `/Volumes/samsung/GitHub/cpython/Grammar/python.gram`
- Tokens: `/Volumes/samsung/GitHub/cpython/Grammar/Tokens`
- AST shape: `/Volumes/samsung/GitHub/cpython/Parser/Python.asdl`
- Full grammar inventory: `tests/cpython_grammar_inventory.md`
- Migration batches: `tests/cpython_migration.md`

Status values:

- `supported`: implemented and covered by Rust tests.
- `partial`: some syntax is implemented, but CPython's full rule is not.
- `planned`: next implementation slice.
- `missing`: not implemented.
- `out_of_scope_runtime`: outside the syntax/AST/compile/tokenize migration target.

Recent runtime migration notes:

- `NUMBER` also includes CPython `test_compile.py::test_literals_with_leading_zeroes`
  coverage for invalid leading-zero integer/prefixed forms and valid
  leading-zero float, exponent, and imaginary literals.
- `ENCODING` also includes the CPython `TestDetectEncoding` latin-1 and UTF-8
  normalization matrices, short BOM-prefixed source with first-line code,
  second-line non-UTF-8 cookie takeover after non-UTF-8 shebang bytes, and
  executable `latin-1-unix` / `utf_8_mac` cookie spellings.
- `ERRORTOKEN` also includes CPython's invalid left-recursive line-continuation
  cases after attribute-style prefixes, including final explicit line
  continuation at EOF.
- `STRING` / `STRING_RUNTIME` also include contextual CPython Unicode casing
  parity for Greek final sigma in `str.lower()`, `str.capitalize()`, and
  `str.title()` via `cpython_string_capitalize_title_swapcase_casefold_subset`.
- `STRING_RUNTIME` also includes CPython `test_format.py` parity for old-style
  `%` formatting of arbitrary-precision decimal, hexadecimal, and octal
  integers with width, precision, alignment, zero padding, sign flags,
  alternate prefixes, and width-driven zero padding when precision is present;
  small-int `%d` / `%x` / `%X` / `%o` alternate-prefix and zero-value behavior,
  including `%d` truncation of float inputs; malformed old-style format
  strings, mapping-key shapes, star width/precision argument consumption, and
  numeric conversion type errors reject with CPython-style parity checks;
  non-ASCII fill characters in
  `format()` alignment and `g` / `G` general
  floating-point formatting through `format()` and f-strings, plus float and
  complex precision formatting via `format()` for `f` / `F`, `e` / `E`, and
  `g` / `G` format codes, and CPython-style invalid format-specifier
  `ValueError` messages for `format()`, f-strings, and `str.format()`. It also
  covers CPython's `z` negative-zero coercion option for float and complex
  f-string formatting, including `%` percentage presentation, fill-character
  ordering, tiny negative values that round to zero, post-rounding sign
  preservation for values such as `-.09`, and invalid `z` specifier
  positions/types.
- `cpython_ast_snippets_parse_inventory_subset` ports the parse-only statement,
  interactive, and expression smoke inventory from
  `Lib/test/test_ast/snippets.py`, covering one-or-more CPython AST samples for
  supported statement classes, expression classes, type-parameter syntax,
  f-strings, and t-strings without requiring every sampled runtime path to
  execute. `cpython_ast_snippets_structural_dump_subset` adds the first
  structural assertions against MiniPython's internal AST dump for
  representative snippet shapes.
  `cpython_ast_snippets_public_to_tuple_first_pass_subset` adds the first
  CPython `to_tuple()` public-AST snippet checks for functions, classes,
  return/delete statements, `for`/`while`/`if`/`with` control flow,
  `try`/`try*`, `raise`/`assert`, ordinary and lazy imports, `global`,
  `pass`/`break`/`continue`, `for` unpacking targets, comprehension source
  spans, async functions/loops/context managers, unpacking displays, and
  `yield` / `yield from`, decorated definitions including generator-argument
  decorators, named expressions, positional-only parameters and defaults,
  type aliases, and generic class/function/type-alias type parameters, plus
  `eval` / `single` mode expression trees, including source positions and
  compile-from-public-AST round-trips.
  `cpython_ast_snippets_eval_to_tuple_core_expr_subset` adds the next
  `snippets.py::eval_tests` public-AST `to_tuple()` batch for constants,
  boolean operators, binary operators, unary operators, lambda, dict, and set
  expression nodes. `cpython_ast_snippets_eval_to_tuple_display_comp_subset`
  extends that to multi-line displays plus list, set, dict, and generator
  comprehensions with tuple/list targets.
  `cpython_ast_snippets_eval_to_tuple_compare_call_slice_subset` adds
  comparison chains, call forms with interleaved keywords and `*` / `**`
  unpacking, generator arguments, attributes, subscripts, omitted-bound slices,
  tuple/list displays, and conditional expressions, including
  compile-from-public-AST round-trips.
  `cpython_ast_snippets_eval_to_tuple_interpolated_string_subset` completes the
  remaining `snippets.py::eval_tests` public-AST interpolated-string batch for
  f-string `JoinedStr` / `FormattedValue` and t-string `TemplateStr` /
  `Interpolation` nodes, including format-spec source spans and
  compile-from-public-AST round-trips.
  `cpython_ast_module_parse_dump_first_pass_subset`
  exposes Python-visible `ast.parse()` / `ast.dump()` across `exec`, `eval`,
  `single`, and `func_type` modes, plus first-pass public node fields such as
  `Module.body`, `Assign.targets`, `Name.id`, `_fields`, and
  `isinstance(..., ast.AST)`. `cpython_ast_parse_null_bytes_subset` ports
  CPython `AST_Tests::test_null_bytes`, requiring `ast.parse()` over source
  strings containing NUL bytes to raise `SyntaxError` with CPython's public
  `source code string cannot contain null bytes` message.
  `cpython_ast_parse_invalid_ast_subset` ports CPython
  `AST_Tests::test_parse_invalid_ast` by rejecting non-root public AST
  nodes such as `ast.Constant(42)` as `ast.parse()` input while preserving full
  AST root-node input. `cpython_ast_parse_optimize_debug_subset` ports CPython
  `AST_Tests::test_optimization_levels__debug__`, including string-source and
  public-AST input for `optimize=-1/0/1/2`.
  `cpython_ast_import_alias_slice_fields_subset` adds CPython `AST_Tests`
  coverage for `test_slice`, `test_from_import`, `test_alias`,
  `test_non_interned_future_from_ast`, and
  `test_compilation_of_ast_nodes_with_default_end_position_values`, checking
  public AST slice defaults, relative import `module=None`, alias source
  spans, future-import module mutation, and compile-from-AST with default end
  positions.
	  `cpython_ast_optimization_format_folding_subset` ports CPython
	  `ASTOptimizationTests::test_folding_format` by folding `'%s' % (a,)` into
	  a `JoinedStr` / `FormattedValue` public AST only when `optimize > 0`.
	  `cpython_ast_optimization_match_case_folding_subset` ports the remaining
	  CPython `ASTOptimizationTests` match-case folding methods by folding signed
	  numeric and real-plus-imaginary pattern literals in `MatchValue`,
	  `MatchMapping`, and nested `MatchSequence` only when `optimize > 0`.
  `cpython_ast_docstring_optimization_single_node_subset` and
  `cpython_ast_docstring_optimization_multiple_nodes_subset` port CPython's
  `optimize=2` docstring removal for class, function, and async-function
  bodies, including `Pass` replacement for single-docstring bodies.
  `cpython_ast_invalid_position_information_subset` and
  `cpython_ast_negative_locations_compile_subset` port CPython's public-AST
  location validation for invalid line/column ranges while preserving accepted
  negative-location compile cases. `cpython_ast_pep758_feature_version_subset`
  ports CPython's PEP 758 `feature_version` gate for unparenthesized multiple
  `except` / `except*` exception types. `cpython_ast_feature_version_gates_subset`
  ports additional CPython `feature_version` gates for positional-only
  parameters, assignment expressions, t-strings, exception groups, type
  parameters, type-parameter defaults, and invalid major versions.
  `cpython_ast_compile_only_ast_first_pass_subset`
  adds first-pass `compile(..., ast.PyCF_ONLY_AST)` parity for `exec`, `eval`,
  and `single` modes. `cpython_ast_parse_exact_subset` splits CPython
  `ASTHelpers_Test::test_parse` into direct method-level coverage.
  `cpython_ast_constructor_first_pass_subset`
  covers first-pass public constructors for base `AST`, `arguments`, `Name`
  context defaults, `FunctionDef`, and hand-built `Module` / `Assign` /
  `Constant` trees. `cpython_ast_constructor_subclass_fields_first_pass_subset`
  extends CPython `ASTConstructorTests` coverage to custom `ast.AST`
  subclasses with `_fields`, `_field_types`, `_attributes`, implicit list
  defaults, `ast.expr_context` / `Load()` defaults, missing field behavior,
  and duplicate positional/keyword field rejection.
  `cpython_ast_constructor_non_str_kwarg_first_pass_subset` adds
  the TypeError side of CPython's non-string `**kwargs` constructor edge,
  including a key object whose Python-level equality matches a real AST field
  name. `cpython_ast_constructor_deprecation_warnings_subset` adds the first
  runtime `warnings.catch_warnings(record=True)` coverage for constructor
  `DeprecationWarning` messages, including missing required builtin fields,
  unexpected custom AST subclass attributes, missing `_field_types` entries,
  malformed non-string `_fields`, and non-string unpacked keyword warnings.
  The `cpython_ast_constructor_*_exact_subset` tests split all 11 current
  CPython `ASTConstructorTests` methods into direct method-level evidence for
  `FunctionDef`, expression contexts, custom subclasses, field metadata,
  attributes, incomplete/malformed fields, implicit defaults, and non-string
  unpacked keyword handling.
  `cpython_ast_copy_replace_first_pass_subset` starts CPython `CopyTests`
  coverage for shallow `copy.replace()` / `__replace__()` over native public
  AST nodes and custom `ast.AST` subclasses, including field replacement,
  location-attribute replacement, missing-field errors, dropping unknown
  instance attributes, and non-string unpacked keyword rejection.
  `cpython_ast_replace_native_class_iteration_first_pass_subset` extends that
  to recursive native AST class traversal through `ast.AST.__subclasses__()`,
  class-level `_fields` / `_attributes` / `__replace__` exposure, AST builtin
  class inheritance checks, and shallow replacement of every exposed native AST
  class field and location attribute.
  `cpython_ast_deepcopy_parent_links_first_pass_subset` adds the first
  `copy.deepcopy()` AST coverage for cyclic parent links and the abstract
  `ast.boolop` / `ast.operator` / `ast.unaryop` / `ast.cmpop` type checks used
  by CPython's `test_copy_with_parents`.
  `cpython_ast_pickle_roundtrip_first_pass_subset` adds the first CPython
  `CopyTests.test_pickling` migration by exposing a minimal `pickle` module
  with `HIGHEST_PROTOCOL`, `dumps()`, and `loads()` and checking public AST
  tree snapshot round-trips across representative statement forms.
  The `cpython_ast_copy_*_exact_subset` tests now split most current
  `CopyTests` methods into direct method-level evidence, including AST
  `__reduce__()` state snapshots for copied location attributes. The custom
  class field method remains first-pass because MiniPython's `Value::String`
  does not yet preserve object identity for `is` checks.
  `cpython_ast_native_abstract_class_hierarchy_subset` aligns the native AST
  class hierarchy with CPython's generated ASDL sum classes for `mod`, `stmt`,
  `expr`, `excepthandler`, `pattern`, `type_ignore`, and `type_param`,
  including direct `__subclasses__()`, `__bases__`, abstract-class
  `_attributes`, and the CPython field split where only `TypeVar` has
  `bound`. `cpython_ast_base_classes_exact_subset` ports CPython
  `AST_Tests.test_base_classes` as a direct method-level check for representative
  concrete and abstract AST class inheritance. `cpython_ast_asdl_inventory_exact_subset` turns that into an exact
  public `ast` module inventory check over all generated AST class names,
  direct subclass edges, `_fields`, and `_attributes` from the local
  `Parser/Python.asdl` snapshot. `cpython_ast_asdl_signature_doc_subset` adds
  CPython's generated ASDL `__doc__` signature surface for concrete nodes,
  enum-like sum nodes, and multi-line expression sum docs.
  `cpython_ast_arguments_annotations_subset` adds the generated
  `_field_types` / `__annotations__` surface for `ast.arguments` plus
  representative ASDL type mappings for list, optional-union, `str`, `object`,
  `type_ignore`, and `int | None` fields.
  `cpython_ast_node_class_metadata_subset` adds public AST node/class metadata
  coverage for writable `_fields`, missing public fields and attributes,
  native constructor arity, fieldless operator nodes, native-AST subclass
  initialization through `super().__init__`, and hand-built `Module.body`
  identity.
  `cpython_ast_base_object_and_missing_fields_subset` adds CPython
  `test_AST_objects` and `test_AST_fields_NULL_check` coverage for base
  `ast.AST()` instances, custom instance attributes, `__dict__`, positional
  constructor rejection, and the crash-regression path where deleting the
  class-level `_fields` attribute makes `ast.AST()` construction raise
  `AttributeError` instead of falling back to generated metadata.
  `cpython_ast_none_required_fields_subset` ports CPython `test_none_checks`
  for required public-AST fields by mutating parser-built `alias`, `arg`,
  `comprehension`, `keyword`, `match_case`, and `withitem` nodes to `None`
  and checking that compile-from-AST raises exact required-field
  `ValueError` diagnostics.
  `cpython_ast_filter_syntax_warnings_by_module_subset` ports CPython
  `AST_Tests.test_filter_syntax_warnings_by_module` for the parser-facing
  warning path: `ast.parse()` now routes tokenizer-originated `SyntaxWarning`
  records into `warnings.catch_warnings(record=True)` with line numbers,
  categories, and default or explicit filenames.
  `cpython_ast_field_attr_existence_subset` ports CPython's public
  `test_field_attr_existence` walk over `ast.__dict__`, constructing AST
  classes from generated `__annotations__` and checking tuple-valued `_fields`
  on every constructed AST node. This also keeps `type` checks precise enough
  to distinguish ast helper functions from actual class objects.
  `cpython_ast_compare_first_pass_subset` adds first-pass public
  `ast.compare()` coverage for structural equality, mutated `_fields` /
  `_attributes`, exact primitive type checks, fieldless operator nodes, and
  missing runtime fields/attributes.
  `cpython_ast_compare_modes_snippets_subset` extends that to CPython's
  current `AST_Tests.test_compare_modes` loop over the `exec_tests`,
  `eval_tests`, and `single_tests` snippets.
  `cpython_ast_helper_iteration_first_pass_subset` adds the
  first public helper coverage for `ast.iter_fields()`,
  `ast.iter_child_nodes()`, and `ast.walk()`.
  `cpython_ast_iter_helpers_exact_subset` ports the exact CPython
  `ASTHelpers_Test::test_iter_fields` and
  `ASTHelpers_Test::test_iter_child_nodes` call-node assertions for field
  dictionaries, child count, child order, and keyword dumps.
  `cpython_ast_node_transformer_first_pass_subset` adds first-pass
  `ast.NodeVisitor` / `ast.NodeTransformer` coverage for visitor dispatch,
  generic traversal, single-field removal, list-field removal, list-return
  replacement, in-place node mutation, and node replacement.
  `cpython_ast_constant_compile_first_pass_subset` ports the current
  `ConstantTests` methods for compiling public `ast.Constant` nodes holding
  supported singleton/value constants, rejecting invalid list constants,
  rejecting `Constant` assignment targets, module docstring lookup, replacing
  `BinOp` operands for `literal_eval()`, preserving supported string-prefix
  `kind` metadata, and observing supported `LOAD_CONST` values through a
  minimal `dis` module subset.
  `cpython_ast_literal_eval_first_pass_subset` adds first-pass
  `ast.literal_eval()` coverage for safe literal containers, bytes, sets,
  numeric signs, complex literals, AST-node input, and malformed expression
  rejection. `cpython_ast_literal_eval_exact_subset` splits CPython
  `ASTHelpers_Test::test_literal_eval` into direct method-level coverage.
  `cpython_ast_literal_eval_complex_full_subset` ports CPython
  `ASTHelpers_Test::test_literal_eval_complex`, including signed real-plus-
  imaginary forms, parenthesized complex literals, and CPython's rejected
  complex-expression shapes. `cpython_ast_literal_eval_complex_exact_subset`
  splits that CPython method into direct method-level coverage.
  `cpython_ast_literal_eval_str_int_limit_subset` adds CPython
  `ASTHelpers_Test::test_literal_eval_str_int_limit` coverage for
  `sys.set_int_max_str_digits()`-controlled decimal integer literal limits in
  `ast.literal_eval()`, while preserving unlimited hexadecimal literal parsing.
  `cpython_ast_literal_eval_str_int_limit_exact_subset` splits that CPython
  method into direct method-level coverage.
  `cpython_ast_recursion_detection_subset` ports CPython
  `ASTHelpers_Test::test_recursion_direct` and
  `ASTHelpers_Test::test_recursion_indirect` for cyclic public-AST compile
  detection. `cpython_ast_recursion_direct_exact_subset` and
  `cpython_ast_recursion_indirect_exact_subset` split those into direct
  CPython method-level checks.
  `cpython_ast_literal_eval_diagnostics_first_pass_subset` adds
  first-pass CPython diagnostic behavior for malformed dict nodes,
  string-leading-space handling, newline-driven `IndentationError`, and
  malformed-node line-number messages, including
  `ASTHelpers_Test::test_literal_eval_syntax_errors`.
  `cpython_ast_literal_eval_malformed_dict_nodes_exact_subset`,
  `cpython_ast_literal_eval_trailing_ws_exact_subset`,
  `cpython_ast_literal_eval_malformed_lineno_exact_subset`, and
  `cpython_ast_literal_eval_syntax_errors_exact_subset` split those behaviors
  into direct CPython method-level evidence.
  `cpython_ast_parse_in_error_first_pass_subset` adds first-pass CPython
  `ASTHelpers_Test::test_parse_in_error` behavior by preserving the active
  exception as `SyntaxError.__context__` when `ast.literal_eval()` parses
  malformed source inside an `except` block.
  `cpython_ast_parse_in_error_exact_subset` splits the same CPython method into
  direct method-level coverage.
  `cpython_ast_multiline_docstring_location_subset` ports CPython
  `ASTHelpers_Test::test_multi_line_docstring_col_offset_and_lineno_issue16806`
  coverage for module, function, nested-function, and trailing docstring
  expression `lineno` / `col_offset` values.
  `cpython_ast_multiline_docstring_location_exact_subset` splits the same
  CPython method into direct method-level coverage.
  `cpython_ast_compile_public_ast_first_pass_subset` adds first-pass
  compile-from-public-AST execution for representative `Module`,
  `Expression`, `Interactive`, and hand-built `Module` trees.
  `cpython_compile_source_positions_multiline_assert_rewrite_subset` adds
  direct CPython `TestSourcePositions` evidence that a generated method-call
  expression can inherit a multiline assert location, be fixed with
  `ast.fix_missing_locations()`, and compile from public AST.
  `cpython_ast_compile_public_ast_statement_second_pass_subset` and
  `cpython_ast_compile_public_ast_expression_second_pass_subset` extend that
  bridge through public AST forms for annotated, augmented, delete, import,
  global, nonlocal, assert, async function/for/with, try-star, named
  expressions, lambda, comprehensions, yield, yield-from, and await nodes.
  `cpython_ast_compile_public_ast_match_second_pass_subset` adds
  compile-from-public-AST execution for parser-generated and hand-built
  `Match`, `match_case`, and pattern nodes, including value, singleton,
  sequence/star, mapping/rest, class, as, wildcard, or-pattern, and guard
  forms.
  `cpython_ast_compile_public_ast_interpolated_string_second_pass_subset` adds
  compile-from-public-AST execution for parser-generated and hand-built
  `JoinedStr`, `FormattedValue`, `TemplateStr`, and `Interpolation` nodes,
  including conversion codes and nested `JoinedStr` format specs.
  `cpython_ast_constant_name_validation_subset` ports CPython validation for
  public `ast.Name` nodes whose `id` is not a string or is the reserved
  singleton spelling `True`, `False`, or `None`, including Unicode identifier
  normalization through `ast.parse(bytes, mode="eval")`.
  `cpython_ast_validator_basic_errors_subset` ports the first CPython
  public-AST validator errors for abstract `ast.expr()` nodes, invalid
  `ast.Constant` payloads containing type objects, and `YieldFrom.value=None`.
  `cpython_ast_validator_load_context_subset` ports CPython
  `ASTValidatorTests::test_module` plus the first load-context checks from
  `test_expr`, `test_boolop`, `test_unaryop`, and `test_yield`, requiring
  public `Name` nodes used as expressions to carry `Load` context.
  `cpython_ast_validator_module_exact_subset` splits CPython
  `ASTValidatorTests::test_module` into direct method-level coverage.
  `cpython_ast_validator_boolop_compare_shape_subset` adds the remaining
  CPython `test_boolop` structure checks for short value lists and `None`
  entries, plus the first `test_compare` checks for missing and mismatched
  comparators.
  `cpython_ast_validator_delete_exact_subset`,
  `cpython_ast_validator_assign_exact_subset`,
  `cpython_ast_validator_augassign_exact_subset`, and
  `cpython_ast_validator_core_expr_exact_subset` split the first validator
  statement/expression methods into direct method-level coverage for CPython
  `test_delete`, `test_assign`, `test_augassign`, `test_expr`,
  `test_boolop`, `test_unaryop`, `test_yield`, and `test_compare`.
  `cpython_ast_validator_expression_context_subset` ports the first validator
  checks for `Lambda`, `IfExp`, `Dict`, `Set`, `Call`, `Attribute`, and
  `Subscript`, including public `ast.Set` constructor exposure.
  `cpython_ast_validator_lambda_exact_subset`,
  `cpython_ast_validator_ifexp_exact_subset`,
  `cpython_ast_validator_dict_exact_subset`,
  `cpython_ast_validator_set_exact_subset`,
  `cpython_ast_validator_call_exact_subset`,
  `cpython_ast_validator_attribute_exact_subset`,
  `cpython_ast_validator_subscript_exact_subset`,
  `cpython_ast_validator_starred_exact_subset`,
  `cpython_ast_validator_list_exact_subset`, and
  `cpython_ast_validator_tuple_exact_subset` split CPython
  `ASTValidatorTests::test_lambda`, `test_ifexp`, `test_dict`, `test_set`,
  `test_call`, `test_attribute`, `test_subscript`, `test_starred`,
  `test_list`, and `test_tuple` into direct method-level coverage.
  `cpython_ast_validator_statement_context_subset` ports the next statement
  validator checks for `Delete`, `Assign`, `AugAssign`, `For`, `While`, `If`,
  `With`, `Raise`, `Assert`, `Import`, `ImportFrom`, `Global`, and
  `Nonlocal`, including Store/Del/Load target-context validation, empty
  target/body/item/name lists, negative import-from levels, and direct
  `None` entries in statement target lists.
  `cpython_ast_validator_for_exact_subset`,
  `cpython_ast_validator_while_exact_subset`,
  `cpython_ast_validator_if_exact_subset`,
  `cpython_ast_validator_with_exact_subset`,
  `cpython_ast_validator_raise_exact_subset`,
  `cpython_ast_validator_assert_exact_subset`,
  `cpython_ast_validator_import_exact_subset`,
  `cpython_ast_validator_importfrom_exact_subset`,
  `cpython_ast_validator_global_exact_subset`, and
  `cpython_ast_validator_nonlocal_exact_subset` split those CPython statement
  validator methods into direct method-level coverage.
  `cpython_ast_validator_definition_and_try_subset` ports the next
  `FunctionDef`, `ClassDef`, `Try`, and `TryStar` validator checks, including
  function argument annotation/default validation, empty definition/handler
  bodies, class base/decorator context validation, and try-statement handler /
  finalbody shape validation.
  `cpython_ast_validator_funcdef_exact_subset`,
  `cpython_ast_validator_classdef_exact_subset`,
  `cpython_ast_validator_try_exact_subset`, and
  `cpython_ast_validator_try_star_exact_subset` split CPython
  `ASTValidatorTests::test_funcdef`, `test_classdef`, `test_try`, and
  `test_try_star` into direct method-level coverage.
  `cpython_ast_validator_funcdef_pattern_matching_subset` ports CPython
  `ASTValidatorTests::test_funcdef_pattern_matching`, proving public
  `FunctionDef`, `arguments`, `arg`, `Pass`, `Name`, and `Load` AST nodes can
  be matched through class patterns using CPython-style `_fields` /
  `__match_args__` ordering.
  `cpython_ast_validator_comprehension_and_sequence_subset` ports the next
  `ListComp`, `SetComp`, `GeneratorExp`, `DictComp`, `Starred`, `List`, and
  `Tuple` validator checks, including non-empty comprehension generators,
  Store/Load validation for comprehension clauses, and sequence element
  validation.
  `cpython_ast_validator_listcomp_exact_subset`,
  `cpython_ast_validator_setcomp_exact_subset`,
  `cpython_ast_validator_generatorexp_exact_subset`, and
  `cpython_ast_validator_dictcomp_exact_subset` split CPython
  `ASTValidatorTests::test_listcomp`, `test_setcomp`, `test_generatorexp`,
  and `test_dictcomp` into direct method-level coverage.
  `cpython_ast_validator_match_pattern_subset` ports CPython's public-AST
  pattern validator cases for `MatchValue`, `MatchSingleton`,
  `MatchSequence`, `MatchMapping`, `MatchClass`, `MatchAs`, `MatchOr`, and
  `MatchStar`, requiring invalid match patterns to fail as `ValueError` during
  `compile(public_ast, ...)`.
  `cpython_ast_validator_stdlib_compile_seed_subset` starts the CPython
  `ASTValidatorTests::test_stdlib_validates` migration by compiling the current
  upstream `STDLIB_FILES` set through MiniPython's parser and compiler: all 150
  top-level `.py` files from the local CPython `Lib` checkout plus
  `test/test_grammar.py` and `test/test_unpack_ex.py`. That includes a `pty.py`
  regression for dotted exception handler type expressions such as
  `except tty.error`, a `compileall.py` regression for grouped
  `with (expr) as target` items, a `_py_warnings.py` regression for dynamic
  exception type expressions such as
  `except re.PatternError if message or module else ()`, a `_pydatetime.py`
  regression for blank lines between decorators and `def`, and a
  `dataclasses.py` regression for same-quote nested f-strings inside
  replacement expressions.
  `cpython_ast_validator_stdlib_recursive_compile_seed_subset` expands that
  file-backed seed to 255 recursive `.py` files from `__phello__`, `_pyrepl`,
  `asyncio`, `collections`, `compression`, `concurrent`, `ctypes`, `curses`,
  `dbm`, `email`, and `encodings`, including `_pyrepl/__main__.py`
  relative-import bytecode-level tracking and `_pyrepl/reader.py`
  `from ._threading_handler import ...` tokenization.
  `cpython_ast_location_helpers_first_pass_subset` adds first-pass
  `copy_location()`, `fix_missing_locations()`, `increment_lineno()`,
  `_attributes`, and `dump(..., include_attributes=True)` behavior for
  generated nodes. `cpython_ast_increment_lineno_on_module_type_ignores_subset`
  adds first-pass CPython `ASTHelpers_Test::test_increment_lineno_on_module`
  coverage for parsed `TypeIgnore` nodes and `Module.type_ignores` line-number
  updates. `cpython_ast_increment_lineno_on_module_exact_subset` splits the
  same CPython method into direct method-level coverage.
  `cpython_ast_fix_missing_locations_module_append_subset` adds
  CPython's exact parsed-module-plus-generated-expression
  `test_fix_missing_locations` snapshot.
  `cpython_ast_fix_missing_locations_exact_subset` splits the same CPython
  method into direct `exact_subset` coverage.
  `cpython_ast_copy_location_call_none_attrs_subset` adds the remaining
  exact CPython `test_copy_location` call-node case where `lineno` and
  `col_offset` survive a source node with `None` values while end-position
  attributes are cleared to `None`.
  `cpython_ast_copy_location_exact_subset` covers the full CPython
  `ASTHelpers_Test::test_copy_location` shape as direct method-level evidence.
  `cpython_ast_increment_lineno_exact_subset` adds CPython's exact
  `test_increment_lineno` snapshots for root-vs-child increments and
  `end_lineno is None` preservation. `cpython_ast_importfrom_level_none_validation_subset` adds
  CPython `ASTHelpers_Test::test_bad_integer` and `test_level_as_none`
  coverage for public `ast.ImportFrom` validation and `level=None`
  compile-from-AST behavior. `cpython_ast_bad_integer_exact_subset` and
  `cpython_ast_level_as_none_exact_subset` split those cases into direct
  method-level evidence. `cpython_ast_elif_and_starred_location_helpers_subset`
  adds CPython `ASTHelpers_Test::test_elif_stmt_start_position`,
  `test_elif_stmt_start_position_with_else`, and
  `test_starred_expr_end_position_within_call` coverage for `elif` statement
  start locations and starred call-argument end positions.
  `cpython_ast_elif_stmt_start_position_exact_subset`,
  `cpython_ast_elif_stmt_start_position_with_else_exact_subset`, and
  `cpython_ast_starred_expr_end_position_within_call_exact_subset` split those
  three CPython methods into direct method-level coverage.
  `cpython_ast_parse_source_locations_first_pass_subset` adds
  first-pass parser-generated source locations for calls, names, constants,
  binary operations, expression statements, and `copy_location()` over parsed
  nodes. `cpython_ast_binop_and_dotted_decorator_locations_subset` ports
  CPython's nested `BinOp` end-position regression for explicit line joining
  and dotted decorator attribute end-position regression.
  `cpython_ast_tstring_structure_subset` ports CPython's basic
  `AST_Tests.test_tstring` public-AST structure checks for `TemplateStr`,
  literal `Constant` parts, and `Interpolation` parts.
  `cpython_ast_repr_first_pass_subset`,
  `cpython_ast_repr_eval_expression_snapshot_subset`, and
  `cpython_ast_repr_large_input_crash_subset` port CPython
  `AST_Tests.test_repr` snapshots and the repr large-input regression for module, function, class, return,
  delete, assignment, annotated assignment, augmented assignment, for/while/if,
  with, raise, try/except/finally, assert, import/from-import/lazy-import,
  global, expr/pass/break/continue, tuple/list/subscript target, comprehension,
  async, unpacking, yield/yield-from, decorators, named expressions,
  positional-only arguments, type aliases, generic classes/functions,
  match statements, expression forms from `snippets.py::eval_tests`,
  docstring, long-list compressed AST node `repr()` output, and
  `ValueError` propagation when AST repr would convert an oversized integer
  to decimal text.
  `cpython_ast_get_docstring_first_pass_subset` adds first-pass
  `ast.get_docstring()` support for modules, classes, functions, async
  functions, `clean=False`, missing docstrings, and unsupported-node
  `TypeError`. `cpython_ast_get_docstring_exact_subset` ports CPython
  `ASTHelpers_Test::test_get_docstring` positive docstring extraction and
  unsupported-node `TypeError` checks as direct method evidence.
  `cpython_ast_get_docstring_none_exact_subset` ports CPython
  `ASTHelpers_Test::test_get_docstring_none`, pinning each module, class,
  function, and async-function no-docstring case as direct method evidence.
  `cpython_ast_get_source_segment_first_pass_subset` adds
  first-pass `ast.get_source_segment()` support for supported parsed nodes,
  padded multi-line extraction from explicit locations, missing location data,
  and non-AST objects. `cpython_ast_call_keyword_end_positions_subset` ports
  CPython `EndPositionTests.test_call` by pinning keyword and `**` keyword
  value source segments. `cpython_ast_call_end_positions_exact_subset` splits
  the same CPython method into direct `exact_subset` coverage.
  `cpython_ast_call_noargs_end_positions_exact_subset`
  ports CPython `EndPositionTests.test_call_noargs`;
  `cpython_ast_lambda_end_positions_exact_subset` ports
  `EndPositionTests.test_lambda`; and
  `cpython_ast_class_kw_end_positions_exact_subset` ports
  `EndPositionTests.test_class_kw` as direct method evidence.
  `cpython_ast_function_class_end_positions_first_pass_subset`
  adds first-pass function/class definition spans, argument annotation spans,
  return-statement spans, class base spans, class keyword attribute spans, and
  padded method source segments. `cpython_ast_func_def_end_positions_exact_subset`
  and `cpython_ast_class_def_end_positions_exact_subset` split CPython
  `EndPositionTests.test_func_def` and `test_class_def` into direct
  method-level coverage. `cpython_ast_string_literal_end_positions_subset`
  ports CPython `EndPositionTests.test_multi_line_str` and
  `test_continued_str`, covering parser-generated end positions for triple
  quoted and adjacent continued string constants.
  `cpython_ast_multi_line_str_end_positions_exact_subset` and
  `cpython_ast_continued_str_end_positions_exact_subset` split those two
  CPython methods into direct method-level coverage.
  `cpython_ast_lambda_slice_end_positions_first_pass_subset`
  adds lambda body/argument spans plus subscript/trailer source spans.
  `cpython_ast_multiline_slice_end_positions_subset` ports the multi-line
  tuple-slice part of CPython `EndPositionTests.test_slices`, including nested
  slice element bounds and the outer subscript end position.
  `cpython_ast_slices_end_positions_exact_subset` ports the full CPython
  `test_slices` method shape as direct method-level coverage.
  `cpython_ast_tuple_display_end_positions_first_pass_subset` adds tuple,
  list, set, and dict display spans, including empty displays, trailing commas,
  and spaced attribute trailers.
  `cpython_ast_tuples_end_positions_exact_subset` and
  `cpython_ast_displays_end_positions_exact_subset` split CPython
  `EndPositionTests.test_tuples` and `test_displays` into direct method-level
  coverage.
  `cpython_ast_attribute_spaces_end_positions_exact_subset` ports CPython
  `EndPositionTests.test_attribute_spaces` as direct method evidence.
  `cpython_ast_redundant_parentheses_source_segment_subset` ports CPython
  `EndPositionTests.test_redundant_parenthesis` and
  `test_trailers_with_redundant_parenthesis`, preserving the distinction where
  pure parenthesized `BinOp` spans exclude redundant parentheses, while
  parenthesized `Call`, `Subscript`, and `Attribute` trailer spans include the
  redundant parentheses around the primary.
  `cpython_ast_redundant_parenthesis_end_positions_exact_subset` and
  `cpython_ast_trailers_with_redundant_parenthesis_end_positions_exact_subset`
  split those CPython methods into direct method-level coverage.
  `cpython_ast_binop_boolop_end_positions_subset` ports the next CPython
  `EndPositionTests.test_binop` and `test_boolop` checks for binary-operation
  and boolean-operation end positions, including parenthesized child operands
  that widen the parent span without widening the child node's own source
  segment. `cpython_ast_binop_end_positions_exact_subset` and
  `cpython_ast_boolop_end_positions_exact_subset` split those two CPython
  methods into direct method-level coverage.
  `cpython_ast_source_segment_multi_tuple_subset` ports CPython
  `EndPositionTests.test_source_segment_multi` for a multi-line tuple that is
  the left side of a binary operation.
  `cpython_ast_source_segment_multi_exact_subset` splits the same CPython
  method into direct `exact_subset` coverage.
  `cpython_ast_source_segment_padded_exact_subset` ports CPython
  `EndPositionTests.test_source_segment_padded`, including UTF-8 byte-column
  end offsets for non-ASCII docstring text.
  `cpython_ast_yield_await_newline_segments_first_pass_subset` adds
  yield/await/yield-from expression spans plus CR/LF/CRLF source segment
  extraction. `cpython_ast_source_segment_tabs_and_mixed_newlines_subset` adds
  padded source extraction with tab/form-feed indentation and mixed
  `\n`/`\r`/`\r\n` function body source segments.
  `cpython_ast_yield_await_end_positions_exact_subset` splits CPython
  `EndPositionTests.test_yield_await` into direct method-level coverage.
  `cpython_ast_source_segment_endings_exact_subset`,
  `cpython_ast_source_segment_tabs_exact_subset`, and
  `cpython_ast_source_segment_newlines_exact_subset` split CPython
  `EndPositionTests.test_source_segment_endings`, `test_source_segment_tabs`,
  and `test_source_segment_newlines` into direct method-level coverage.
  `cpython_ast_source_segment_missing_info_exact_subset` ports CPython
  `EndPositionTests.test_source_segment_missing_info`, requiring
  `ast.get_source_segment()` to return `None` after deleting any required
  location attribute from parser-built statements.
  `cpython_ast_comprehension_end_positions_first_pass_subset` adds
  first-pass list/set comprehension source spans for targets, iterables,
  filters, and outer expression end positions.
  `cpython_ast_comprehensions_end_positions_exact_subset` splits CPython
  `EndPositionTests.test_comprehensions` into direct method-level coverage.
  `cpython_ast_suite_end_positions_first_pass_subset` adds first-pass
  suite/control-flow source spans for while, if/elif/else, for, try/except,
  pass, and selected nested child nodes.
  `cpython_ast_suites_end_positions_exact_subset` splits CPython
  `EndPositionTests.test_suites` into direct method-level coverage.
  `cpython_ast_import_end_positions_first_pass_subset` adds first-pass
  import/import-from statement spans plus alias spans and source extraction.
  `cpython_ast_import_from_multiline_end_positions_first_pass_subset` adds
  CPython's parenthesized multi-line import-from end-position behavior, and
  `cpython_ast_import_from_multiline_end_positions_exact_subset` ports the same
  CPython method as direct method-level evidence.
  `cpython_ast_fstring_end_positions_first_pass_subset` adds CPython f-string
  replacement-expression source spans, including multi-line replacement
  expressions. `cpython_ast_fstring_end_positions_exact_subset` and
  `cpython_ast_fstring_multi_line_end_positions_exact_subset` split CPython
  `EndPositionTests.test_fstring` and `test_fstring_multi_line` into direct
  method-level coverage.
  `cpython_ast_dump_plain_first_pass_subset` ports CPython
  `ASTHelpers_Test::test_dump` plain `ast.dump()` rendering for default,
  `annotate_fields=False`, and `include_attributes=True` forms.
  `cpython_ast_dump_exact_subset` splits the same CPython method into direct
  method-level `exact_subset` coverage.
  `cpython_ast_dump_indent_first_pass_subset` adds CPython-style
  `ast.dump(indent=...)` rendering for integer and string indents, including
  `include_attributes=True`. `cpython_ast_dump_indent_exact_subset` splits
  CPython `ASTHelpers_Test::test_dump_indent` into direct method-level
  coverage. `cpython_ast_dump_incomplete_first_pass_subset`
  adds first-pass CPython incomplete-node dump behavior for missing/default
  fields, positional omitted-field buffering, and attributes.
  `cpython_ast_dump_incomplete_exact_subset` splits CPython
  `ASTHelpers_Test::test_dump_incomplete` into direct method-level coverage.
  `cpython_ast_dump_show_empty_first_pass_subset` adds first-pass
  `show_empty=True` / `show_empty=False` behavior for supported public AST
  nodes. `cpython_ast_dump_show_empty_exact_subset` splits CPython
  `ASTHelpers_Test::test_dump_show_empty` into direct method-level coverage.
  `cpython_ast_lazy_import_fields_subset` adds CPython's public
  `Import.is_lazy` and `ImportFrom.is_lazy` fields to `_fields`, `ast.dump()`,
  parsed ordinary/lazy import nodes, AST constructors, and compile-from-public-AST
  execution.
  Exact CPython warning behavior,
  subclassing, field validation, full `to_tuple()` snippet coverage, parser
  source-location spans for remaining node families, remaining generated-node
  dump edge cases, deeper `literal_eval()` edge cases such as integer digit
  limits, and broader compile-from-public-AST parity remain open.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_mapping_mixins_subset`, covering explicit
  `Mapping` / `MutableMapping` subclass mixins for `get`, containment, key/item
  listing, equality, `pop`, `popitem`, `clear`, `update`, and `setdefault`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_mapping_mixin_views_subset`, covering live
  `KeysView` / `ItemsView` / `ValuesView` objects returned by explicit
  `Mapping` mixins, including membership, iteration after mutation, and
  set-like key/item view operators.
- `CONTAINER_RUNTIME` also includes `cpython_dict_view_richcompare_subset`,
  covering set-style rich comparisons for dict key/item views and propagation
  of Python-level `__eq__` errors during item-view comparisons.
- `CONTAINER_RUNTIME` also includes `cpython_dict_view_mappingproxy_subset`,
  covering dict-view `.mapping`, the read-only `mappingproxy` type object,
  live equality with the underlying dict, lookup, membership, and assignment
  rejection.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_mappingproxy_exact_dict_subset`, covering the exact-dict
  `types.MappingProxyType` constructor path plus `get`, live views, `copy`,
  iteration, reverse iteration, equality, ABC registration, and invalid
  constructor/write errors.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_mappingproxy_union_subset`, covering exact dict and
  mappingproxy operands for `mappingproxy | dict`, `dict | mappingproxy`, and
  `mappingproxy | mappingproxy`, while preserving the read-only `|=` error.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_mappingproxy_method_surface_subset` and
  `cpython_types_mappingproxy_custom_mapping_subset`, covering CPython's
  mappingproxy public method surface plus forwarding of lookup, containment,
  length, iteration, copy, get, keys, items, and values calls to user-defined
  mapping objects.
- `CONTAINER_RUNTIME` also includes `cpython_types_mappingproxy_hash_subset`,
  covering unhashable exact-dict proxies and hash forwarding for proxies over
  hashable user-defined mapping objects.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_mappingproxy_richcompare_subset`, covering equality,
  inequality, and catchable CPython-style `TypeError` ordering between
  `mappingproxy` objects.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_mappingproxy_contains_subset`,
  `cpython_types_mappingproxy_views_subset`,
  `cpython_types_mappingproxy_len_subset`,
  `cpython_types_mappingproxy_iterators_subset`,
  `cpython_types_mappingproxy_reversed_subset`, and
  `cpython_types_mappingproxy_copy_subset`, covering the corresponding CPython
  exact-dict `MappingProxyTests` methods for membership, live views, length,
  iterator conversion, reverse iteration exhaustion, and independent copies.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_mappingproxy_missing_subset` and
  `cpython_types_mappingproxy_chainmap_subset`, covering CPython
  `MappingProxyTests` behavior for dict subclasses with `__missing__` and
  `collections.ChainMap` mapping sources.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_simple_namespace_basic_subset`, covering the first CPython
  `types.SimpleNamespace` slice: dict, `collections.UserDict`, and
  pair-iterable construction, keyword override order, live `__dict__` /
  `vars()` access, attribute get/set/delete, repr/equality, unsupported mapping
  operations, constructor error paths, and subclass construction through the
  inherited builtin initializer.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_simple_namespace_recursive_and_replace_subset`, covering
  CPython `SimpleNamespace` recursive attribute identity, recursive repr,
  direct display recursion protection, `copy.replace()` shallow copies, keyword
  field replacement, and subclass-preserving replacement.
- `CONTAINER_RUNTIME` also includes
  `cpython_set_and_frozenset_subclass_subset`, covering first-pass CPython
  set/frozenset subclass construction, iteration, membership, `len`, conversion
  back to exact `set`, builtin method result types, in-place set mutation,
  `super().__init__`, custom `__new__` via `super().__new__`,
  frozenset-subclass hashing, frozenset subclass copy/constructor identity,
  empty frozenset subclass identity behavior, basic subclass `__slots__`, and
  Set/MutableSet/Hashable ABC registration.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_builtin_code_object_subset`, covering first-pass
  `compile(source, filename, mode)` for string and bytes sources in `exec`,
  `eval`, and `single` modes, plus feeding the resulting `code` objects through
  `eval()` and `exec()` with dict-backed globals/locals.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_newline_and_indentation_subset`, covering CPython
  `TestSpecifics` compile acceptance for empty string source, missing trailing
  newlines, CRLF and lone-CR source newlines, mixed newline source with nested
  definitions, and nested indented blocks.
- `RUNTIME_BUILTINS` also includes the
  `cpython_compile_specifics_*` TestSpecifics expansion for public
  `compile()`, `eval()`, and `exec()` boundaries: SyntaxError propagation for
  invalid assignments, duplicate parameters, invalid keyword targets, bad float
  literals, and invalid parameter ordering; `None` target rejection in
  `single` and `exec` modes; import grammar acceptance/rejection; and
  compile-stability shapes for large annotated signatures, conditional
  expressions, dead blocks, and try/except/finally control flow.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_compile_filename_subset`, covering first-pass
  CPython `compile()` filename behavior for string and bytes filenames plus the
  public `code.co_filename` attribute, while leaving memoryview/path-like
  filename cases to the future memoryview/path-like runtime slice.
- `RUNTIME_BUILTINS` also includes expanded `cpython_eval_builtin_subset` and
  `cpython_exec_builtin_subset` coverage for `source`, `globals`, and `locals`
  keyword binding, bytes-source execution through the same decoding path as
  `compile()`, CPython-style globals preparation before source compile/type
  errors, and exec writeback of assignments that happen before runtime
  exceptions. It also covers same-mapping globals/locals behavior for
  `eval(source, g, g)` named-expression writes and `exec(source, g, g)`
  ordinary/global assignment writes.
- `RUNTIME_BUILTINS` also includes
  `cpython_eval_exec_builtins_mapping_subset`, covering first-pass
  `globals['__builtins__']` lookup for restricted builtin dictionaries and
  exact-dict `mappingproxy` builtin mappings, default builtins injection into
  supplied eval/exec globals, dict-subclass builtin mappings, custom and default
  `__import__` lookup for import statements, and dict-subclass `__getitem__`
  exception propagation for globals and builtin mappings.

## Diagnostics Infrastructure

- `lex_with_spans()` exposes lexer token start/end locations, and
  `parse_with_diagnostic()` exposes the parser token index for failed parses.
  `source_parse_error_diagnostic()` uses both for CPython-style SyntaxError span
  tests, including repeated `found ...` token shapes and representative invalid
  assignment targets. It also covers representative parser errors without a
  `found ...` suffix, including empty inline suites, empty parenthesized `with`
  items, missing function defaults, missing call keyword values, and missing
  parameter separators.
- `tokenize_with_spans()` exposes the CPython-tokenize-style path for source
  forms that can produce tokens even when the stricter compile/parser path later
  rejects them. In that mode, MiniPython now also emits a synthetic final
  `NEWLINE` before `EOF` for non-newline-terminated token streams.
- `tokenize_cpython_with_spans()` builds on that path for CPython-tokenize
  compatibility details that should not feed the parser directly. It currently
  expands collapsed parser f-string and t-string tokens into first-pass
  `FSTRING_START` / `FSTRING_MIDDLE` / `FSTRING_END` and t-string equivalents.
- `detect_source_encoding()` exposes the byte-oriented PEP 263 detection step
  that precedes decoding file/bytes input, including coding cookies, UTF-8 BOM
  stripping, encoding-name normalization, consumed-line parity, and representative
  SyntaxError-style rejection cases.
- `tokenize_bytes_with_spans()` uses that detection step to decode bytes input
  and emit a leading CPython-style `ENCODING` token without changing the ordinary
  parser token stream. It also normalizes source CRLF/CR newlines before token
  production for supported bytes input.
- `run_source_bytes()` uses the same detection step before the ordinary
  parser/compiler/VM path, covering CPython-style execution of supported bytes
  source encodings and source newline normalization.
- The source-encoding subset also ports CPython `TestDetectEncoding` short-file,
  false-cookie, empty-first-line second-cookie, ignored-third-cookie,
  second-line BOM mismatch, and default UTF-8 decode-error cases, plus matching
  `test_source_encoding.py` byte-source execution errors beyond the first two
  source lines.
- Source decoding now uses `encoding_rs` for migrated non-latin codec families
  beyond the hand-written UTF-8/latin-1/latin-9 paths and falls back to
  `encoding_rs` label lookup for additional supported source encodings. Current
  CPython-derived coverage includes `cp1252`, `cp949`, `cp932`, and `cp1251`,
  including CPython-style undefined-byte rejection for source `cp1251` and
  `cp1252`; runtime `cp1251` and `cp1252` decode also preserve CPython's
  undefined-byte handling.
- `cpython_tokenize_exact_type_subset` ports CPython's
  `TokenInfo.exact_type` operator table to MiniPython's exact lexer token
  variants, and `cpython_tokenize_selector_and_method_subset` ports CPython's
  selector and decorator/method tokenizer span examples.
- `cpython_tokenize_async_await_subset` ports representative CPython
  `test_async` tokenizer source shapes while preserving MiniPython's
  parser-ready `async` / `await` keyword token variants.
- `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` starts
  method-oriented `Lib/test/test_grammar.py::GrammarTests` migration with
  eval input, variable annotation basics and syntax errors, annotation target
  execution-order behavior, and class annotation inheritance.

## Lexical Tokens

Note: `lexes_underscore_relative_import_module_after_dot` pins the lexical
boundary between invalid numeric-looking `._5` and valid relative import module
names like `from ._threading_handler import ...`.

| CPython token | Status | Rust evidence |
| --- | --- | --- |
| `ENDMARKER` | supported | `lexes_print_number`, `cpython_tokenize_spanned_tokens_subset` including EOF span |
| `NAME` | supported | `lexes_print_number`, `lexes_unicode_identifiers`, `lexes_underscore_relative_import_module_after_dot`, `cpython_ast_assignment_and_name_load_subset`, `cpython_unicode_identifier_subset`, `cpython_tokenize_spanned_tokens_subset` including keyword/name spans, `cpython_tokenize_formfeed_whitespace_subset` including formfeed-separated name/operator tokens |
| `NUMBER` | partial | `lexes_float_literals`, `lexes_imaginary_literals`, `lexes_number_separators`, `lexes_large_integer_literals`, `lexes_number_keyword_boundaries`, `rejects_invalid_number_separators`, `lexes_prefixed_integer_literals`, `rejects_invalid_prefixed_integer_literals`, `rejects_nonzero_leading_decimal_zeroes`, `cpython_tokenize_valid_number_token_stream_subset` covering the migrated CPython `test_int`, `test_long`, and `test_float` raw-token text/spans for integer/operator, large-integer, and float forms, including trailing-dot, uppercase exponent, and large exponent float spellings, `cpython_tokenize_underscore_number_token_stream_subset` covering CPython `test_underscore_literals` raw NUMBER text preservation/rejection behavior, `cpython_grammar_prefixed_integer_literals_subset`, `cpython_float_literal_forms_subset`, `cpython_float_exponent_tokenization_subset`, `cpython_end_of_numerical_literals_subset` including adjacent-name, hexadecimal adjacent-name, and fraction-slash spans, `cpython_tokenize_invalid_python_token_stream_subset` including tokenizer-only `2sin(x)` number/name split, `01234`, `0_7`, and `09_99` leading-zero NUMBER tokenization, invalid decimal underscore/exponent token splitting, and CPython `test_invalid_syntax` binary/octal/hex token splitting for invalid digits, invalid underscore suffixes, and missing prefixed-integer digits, `cpython_numeric_literal_warning_subset` including CPython `test_end_of_numerical_literals` keyword-boundary warnings, `test_tokenizer_fstring_warning_in_first_line` binary-boundary warning source `0b1and 2`, and warning-as-error spans for decimal, imaginary, binary, octal, and hexadecimal literals, `cpython_valid_underscore_number_literals_subset`, `cpython_large_integer_literals_subset` including CPython `test_long_integers` prefix-case and huge-literal forms, `cpython_integer_bit_methods_subset`, `cpython_integer_ratio_and_component_methods_subset`, `cpython_float_ratio_and_component_methods_subset`, `cpython_integer_base_builtins_subset`, `cpython_int_max_str_digits_runtime_subset` covering CPython `test_int.py::IntStrDigitLimitsTests` runtime digit-limit behavior for `int()`, `str()`, top-level/container `repr()`, sign/space padding, underscores, and unlimited power-of-two bases, `cpython_int_max_str_digits_formatting_subset` extending that digit-limit coverage to `format()`, f-strings, `str.format()`, and old-style `%s` / `%r` / `%a` / `%d` / `%i` / `%u` decimal formatting while preserving unlimited hexadecimal formatting, `cpython_divmod_builtin_subset`, `cpython_round_builtin_subset`, `cpython_pow_builtin_subset`, `cpython_bad_numerical_literals_subset` including representative bad-literal spans, `cpython_syntax_error_message_parity_subset`, `cpython_grammar_imaginary_literals_subset`, `runs_float_literals`, `runs_imaginary_literals`, `runs_prefixed_integer_literals` |
| `STRING` | partial | `lexes_string`, `lexes_string_line_continuations`, `lexes_string_octal_escapes`, `lexes_unicode_name_escapes`, `lexes_unicode_name_alias_escapes`, `lexes_bytes_literals`, `lexes_bytes_line_continuations`, `lexes_cpython_string_prefix_matrix`, `rejects_non_ascii_bytes_literals`, `rejects_cpython_unterminated_string_forms`, `rejects_cpython_invalid_string_escape_forms`, `rejects_cpython_unterminated_interpolated_string_forms`, `lexes_f_string_parts`, `lexes_f_string_escaped_brace_literals`, `lexes_f_string_backslash_before_doubled_braces`, `lexes_f_string_line_continuations`, `lexes_f_string_format_specs`, `lexes_raw_and_non_raw_f_string_format_spec_escapes`, `lexes_f_string_debug_expressions`, `lexes_raw_f_string_literals_and_empty_format_specs`, `cpython_tokenize_string_span_subset` including quote, embedded quote payloads, ordinary single- and double-quoted string expressions, raw-prefix matrix, `u`/`U` prefixes, `b`/`B` bytes prefixes, single- and double-quoted `br`/`rb` raw-bytes prefix matrix, split string/name/string tokenization, adjacent same-line string tokens before parser concatenation, multiline line-continuation, unicode-prefixed line-continuation, triple-quoted, unicode-prefixed triple-quoted, single-quoted raw bytes, triple-quoted raw bytes, escaped CRLF text inside a string token, and indented non-ASCII triple-quoted source spans, `cpython_string_literal_and_concat_subset`, `cpython_string_startswith_endswith_subset`, `cpython_string_find_index_subset`, `cpython_string_count_case_subset`, `cpython_string_capitalize_title_swapcase_casefold_subset`, `cpython_string_predicate_methods_subset` including the CPython `isascii()` alignment matrix, `cpython_string_identifier_printable_subset`, `cpython_string_expandtabs_subset`, `cpython_string_splitlines_subset`, `cpython_string_replace_subset`, `cpython_string_remove_affix_subset`, `cpython_string_split_rsplit_subset`, `cpython_string_strip_subset`, `cpython_string_alignment_and_zfill_subset`, `cpython_string_partition_rpartition_subset`, `cpython_string_join_subset`, `cpython_string_line_continuation_subset`, `cpython_string_octal_escape_subset`, `cpython_string_escape_warning_subset` including warning-as-error behavior, `cpython_string_invalid_escape_ascii_table_subset`, `cpython_string_escape_warning_location_subset`, `cpython_f_string_escape_warning_subset` including warning-as-error behavior, `cpython_unicode_name_escape_subset`, `cpython_bytes_literal_subset`, `cpython_string_prefix_matrix_subset`, `cpython_invalid_string_prefix_matrix_subset` adapted from CPython `test_invalid_string_prefixes`, `cpython_invalid_string_literal_subset` including CPython `test_invalid_syntax` unterminated ordinary, bytes, one-line triple, and multiline triple-quoted string spans plus non-ASCII bytes literal spans, `cpython_string_and_tstring_helper_rules_subset`, `cpython_f_string_basic_subset`, `cpython_f_string_triple_quoted_expression_subset`, `cpython_f_string_many_expressions_subset`, `cpython_f_string_format_specifier_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_raw_f_string_format_spec_subset`, `cpython_invalid_f_string_syntax_subset`, `cpython_invalid_t_string_syntax_subset`, `runs_python_string_literal_forms`, `runs_python_bytes_literal_forms`, `runs_f_strings`, `runs_f_string_expressions` |
| `STRING_RUNTIME` | partial | `cpython_ascii_builtin_subset` covers first-pass `ascii()` builtin behavior, CPython-style non-ASCII repr escaping, f-string `!a`, and recursive list/dict repr placeholders; `cpython_chr_ord_builtin_subset` covers first-pass `chr()` and `ord()` builtins for supported Unicode scalar values and one-character/one-byte inputs; `cpython_old_style_string_percent_format_subset` covers first-pass old-style `%` string formatting for `%s`, `%r`, `%a`, `%%`, `%d`, `%i`, `%u`, `%x`, `%X`, `%o`, `%c`, `%f`, `%F`, `%e`, `%E`, `%g`, `%G`, ignored `h` / `l` / `L` length modifiers, tuple argument consumption, `%(key)` mapping arguments, static and dynamic `*` flags/width/precision for text, integer, and float conversions, mapping-to-positional mixing errors, extra-argument errors, non-integer `*` errors, mapping-key `*` errors, non-real float-format errors, isolated/unsupported-format `ValueError` paths, and out-of-range `%c` errors; `cpython_string_format_and_format_map_subset` covers first-pass `str.format()` and `str.format_map()` literal rendering, escaped braces, positional/automatic/keyword/mapping fields, simple attribute and item lookup, conversions, and existing mini-format specs; `cpython_f_string_contextual_runtime_subset` covers f-string truthiness, empty format specs, f-string indexing versus `str.format()` field indexing, loop evaluation, and nested-quote dict subscripts; `cpython_f_string_format_error_subset` covers CPython-style f-string formatting TypeErrors and ValueErrors for unsupported object specs and unknown scalar format codes; `cpython_string_maketrans_translate_subset` covers `str.maketrans()` and `str.translate()` dictionary translation, deletion, integer/string replacements, non-ASCII replacements, and error paths; `cpython_string_bytes_codec_subset` covers first-pass `str.encode()`, `bytes.decode()`, `str(bytes, encoding)`, `bytes(str, encoding)`, `bytearray(str, encoding)`, codec constructor keyword behavior, `encoding_rs` label fallback with `cp1251` and `cp1252`, and CPython-style strict / `ignore` / `replace` behavior for undefined codec bytes; `cpython_bytes_hex_fromhex_subset` covers bytes/bytearray `fromhex()` and bytes/bytearray `hex()` with separator grouping; `cpython_templatelib_constructor_subset` covers the supported `string.templatelib.Template` and `Interpolation` constructors, Template `values`, builtin type metadata, and conversion error paths; `cpython_templatelib_final_type_and_iterator_subset` covers final templatelib type inheritance errors plus TemplateIter type metadata, identity iteration, yielded interpolation objects, and repeated exhaustion; `cpython_t_string_raw_concat_and_triple_subset` covers raw t-string literal preservation, Template + Template concatenation, Template/string concatenation TypeErrors, and triple-quoted t-string segments |
| `CONTAINER_RUNTIME` | partial | `cpython_sequence_constructor_builtins_subset` covers first-pass list, tuple, and set constructors over builtins, strings, generator expressions, existing tuple identity preservation, keyword rejection, non-iterable rejection, unhashable set elements, exact `set.__init__` reinitialization behavior including self-input clearing and partial mutation before an unhashable element error, plus exact `TestSet` constructor identity, literal equality, left-to-right literal insertion/evaluation order, unhashable set values, and `set.copy()` equality/type/identity; `cpython_set_mutation_methods_subset` covers first-pass `TestSet` mutation method behavior for `clear`, `add`, `remove`, `discard`, `pop`, and `update`, including duplicate-add no-op, unhashable argument errors, missing-key `KeyError`, nested set/frozenset lookup equivalence, pop-until-empty behavior, and update result/error paths; `cpython_set_direct_lookup_and_keyerror_payload_subset` covers direct set-key membership/discard/remove behavior plus preservation of the original missing key in `KeyError.args[0]`; `cpython_set_hash_exception_propagation_subset` covers propagation of exceptions raised by user-defined `__hash__` during set membership, `add`, and `discard`; `cpython_set_bad_comparison_errors_subset` covers hash-collision rich equality and propagation of exceptions raised by user-defined `__eq__` during set construction, membership, `add`, `discard`, and `remove`; `cpython_set_bad_comparison_algebra_errors_subset` covers the same rich-equality exception propagation across set/frozenset equality and ordering checks, relation methods, algebra methods, and `&`, `|`, `-`, and `^`; `cpython_set_iterator_mutation_subset` covers CPython set iterator size-change invalidation and the clear/refill-to-original-size no-crash regression; `cpython_set_reentrant_mutation_subset` covers set updates whose rich equality clears the source set plus hash-collision `set.add()` re-entering the same set from Python-level `__eq__`; `cpython_set_operations_mutating_subset` covers CPython `TestOperationsMutating` stable cases for set equality, ordering, algebra, relation methods, and update methods when element equality clears both participating sets; `cpython_set_rich_compare_reflection_subset` covers CPython set ordering fallback through `NotImplemented` into the right operand's reflected rich-comparison method; `cpython_set_inplace_algebra_methods_subset` covers `TestSet` iterable operand support for `update`, `intersection_update`, `difference_update`, and `symmetric_difference_update`, in-place set operator identity preservation, strict `TypeError` for unhashable iterable operands, and partial mutation before `set.update()` encounters an unhashable element; `cpython_set_only_sets_in_binary_ops_subset` covers CPython `TestOnlySetsInBinaryOps` equality, ordering, binary operator, in-place operator, and method-form iterable behavior for non-set operands including generators; `cpython_dict_constructor_update_fromkeys_subset` covers first-pass dict construction, update, and `fromkeys`; `cpython_dict_view_mappingproxy_subset` covers dict-view `.mapping` read-only `mappingproxy` type identity, live equality, lookup, membership, and item-assignment rejection; `cpython_iter_next_builtin_subset` covers first-pass iterator identity, `next(default)`, callable-sentinel iterator exhaustion, callable-raised `StopIteration`, reentrant callable-sentinel exhaustion, and supported iterator sink-state behavior after exhaustion; `cpython_map_strict_builtin_subset` covers strict `map()` length checks, iterator-consumption side effects, and propagated custom iterator exceptions versus strict-mode `StopIteration` conversion; `cpython_reversed_builtin_subset` covers first-pass reversed iteration over supported sequence, dict, and dict-view values |
| `COLLECTIONS_ABC_RUNTIME` | partial | `cpython_collections_abc_iterable_iterator_subset` covers the supported `collections.abc.Iterable` and `Iterator` module surface, `isinstance` checks for built-in containers, built-in iterators, `TemplateIter`, non-iterable scalar values, and structural user classes, plus `issubclass` checks for structural user classes and `Iterator` inheriting from `Iterable`; `cpython_collections_abc_core_runtime_subset` covers the supported `Hashable`, `Sized`, `Container`, `Callable`, and `Collection` ABC surface, including built-in container/type relationships, structural user classes, direct ABC subclassing, and CPython-style `None` blocking for special methods; `cpython_collections_abc_sequence_subset` covers `Sequence` for supported built-in sequence registrations, explicit Sequence subclassing, CPython's non-structural Sequence behavior, and Sequence inheritance through Reversible, Collection, Sized, Iterable, and Container; `cpython_collections_abc_sequence_mixins_subset` covers `Sequence` mixins for explicit subclasses, including index parity against native list/str start/stop behavior plus `count`, `__contains__`, `__iter__`, `__reversed__`, membership fallback, and keyword calls; `cpython_collections_abc_bytestring_buffer_subset` covers `ByteString` and `Buffer` for supported bytes/bytearray registrations, ByteString inheritance through Sequence, Buffer `__buffer__` structural subclasshook behavior, direct ABC subclassing, and CPython-style `None` blocking for `__buffer__`; `cpython_collections_abc_mutable_sequence_subset` covers `MutableSequence` for supported list/bytearray registrations, inheritance through Sequence/Reversible/Collection/Sized/Iterable/Container, CPython's non-structural protocol behavior, explicit subclass mixins, and self-extension; `cpython_collections_abc_mapping_subset` covers `Mapping` and `MutableMapping` for registered `dict`, ABC inheritance, direct subclassing, and CPython's non-structural mapping behavior; `cpython_collections_abc_mapping_view_subset` covers `MappingView`, `KeysView`, `ItemsView`, and `ValuesView` for built-in dict views, `KeysView`/`ItemsView` set behavior, `ValuesView` collection behavior, ABC inheritance, direct ABC subclassing, and CPython's non-structural view behavior; `cpython_collections_abc_set_mutable_set_mixins_subset` covers `Set` and `MutableSet` registrations for set/frozenset and supported set-like dict views, inheritance through Collection/Sized/Iterable/Container, explicit subclass mixins for comparison, binary set operations, `_hash`, `_from_iterable`, mutable update methods, and self-clearing regressions; `cpython_frozenset_basic_subset`, `cpython_set_frozenset_joint_ops_subset`, `cpython_set_frozenset_relationship_matrix_subset`, and `cpython_set_frozenset_algebra_matrix_subset` cover first-pass exact `frozenset` construction, empty singleton identity, no-op exact `frozenset.__init__`, immutable set algebra, equality with `set`, order-independent hashing for hashable elements, dict/set key behavior, shared set/frozenset joint operations from CPython `test_set.py`, the `isdisjoint` constructor matrix, set-of-frozensets uniqueness, non-mutating set algebra constructor matrices, multi-operand union/intersection/difference, and the Issue #6573 empty-set union regression; `cpython_collections_abc_reversible_subset` covers `Reversible` for supported built-in reversible containers/views, non-reversible scalar/container/iterator samples, `Sequence` inheritance, structural `__iter__` + `__reversed__` user classes, direct ABC subclassing, and `None` blocking; `cpython_collections_abc_async_runtime_subset` covers `Awaitable`, `Coroutine`, `AsyncIterable`, and `AsyncIterator` for native coroutine objects, structural user classes, non-samples, ABC inheritance, and `None` blocking; `cpython_collections_abc_generator_runtime_subset` covers `Generator` and `AsyncGenerator` for native generator objects, structural protocol classes, incomplete protocol non-samples, direct ABC subclassing, and `None` blocking |
| `NEWLINE` | supported | `lexes_newline`, `cpython_compile_crlf_newlines_subset`, `cpython_compile_specifics_newline_and_indentation_subset`, `cpython_tokenize_explicit_line_joining_subset` including continuation-only lines that do not emit statement newlines, token-kind/text parity with the no-continuation spelling, comment backslashes that do not continue, and bad-indentation continuation rejection, `cpython_tokenize_implicit_line_joining_subset` including the logical newline after a bracketed block containing comments and CPython's bracketed tuple/list/dict continuation semantics, `cpython_tokenize_spanned_tokens_subset` including newline span, `cpython_tokenize_trailing_space_without_newline_subset` covering tokenizer-mode preservation of a final whitespace-only physical line and final comment-only physical line, `cpython_tokenize_bytes_encoding_token_subset` covering a synthesized final newline after very long comment-only bytes source without a final newline, and `cpython_tokenize_invalid_python_token_stream_subset` including tokenizer-mode synthetic final newline |
| `INDENT` | supported | `lexes_if_block_indentation`, `lexes_tabs_in_indentation`, `cpython_tokenize_nested_indentation_subset`, `cpython_tokenize_max_indent_subset`, `cpython_tokenize_unmatched_indentation_subset` including CPython-style tab expansion and inconsistent tab/space indentation rejection, `cpython_tokenize_formfeed_whitespace_subset` including leading-formfeed indentation reset, `cpython_tokenize_explicit_line_joining_subset` including continuation-only lines that suppress unrelated indentation while still allowing a pending post-colon block indent, `cpython_tokenize_spanned_tokens_subset` including indent span |
| `DEDENT` | supported | `lexes_if_block_indentation`, `cpython_tokenize_nested_indentation_subset`, `cpython_tokenize_max_indent_subset`, `cpython_tokenize_unmatched_indentation_subset` including unmatched-dedent spans, `cpython_tokenize_spanned_tokens_subset` including dedent span |
| `LPAR` | supported | `lexes_print_number` |
| `RPAR` | supported | `lexes_print_number` |
| `LSQB` | supported | `lexes_list_brackets` |
| `RSQB` | supported | `lexes_list_brackets` |
| `COLON` | supported | `lexes_if_block_indentation` |
| `COMMA` | supported | `lexes_comma`, `prints_multiple_arguments` |
| `SEMI` | supported | `lexes_semicolon`, `cpython_grammar_semicolon_simple_statements_subset` |
| `PLUS` | supported | `lexes_plus`, `cpython_grammar_additive_ops_subset` |
| `MINUS` | supported | `lexes_arithmetic_operators`, `cpython_grammar_additive_ops_subset` |
| `STAR` | supported | `lexes_arithmetic_operators`, `cpython_grammar_multiplicative_ops_subset` |
| `SLASH` | supported | `lexes_arithmetic_operators`, `cpython_grammar_multiplicative_ops_subset` |
| `VBAR` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_bitwise_and_shift_subset` |
| `AMPER` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_bitwise_and_shift_subset` |
| `LESS` | supported | `lexes_comparison_operators`, `cpython_grammar_ordering_comparison_subset` |
| `GREATER` | supported | `lexes_comparison_operators`, `cpython_grammar_ordering_comparison_subset` |
| `EQUAL` | supported | `lexes_equal`, `assigns_and_reads_variable` |
| `DOT` | supported | `lexes_attribute_dot_after_parenthesized_number`, `reports_attribute_errors` |
| `PERCENT` | supported | `lexes_arithmetic_operators`, `cpython_grammar_multiplicative_ops_subset` |
| `LBRACE` | supported | `lexes_dict_braces`, `cpython_ast_dict_literal_subset`, `cpython_dict_display_unpacking_subset` |
| `RBRACE` | supported | `lexes_dict_braces`, `cpython_ast_dict_literal_subset`, `cpython_dict_display_unpacking_subset` |
| `EQEQUAL` | supported | `lexes_equal_equal`, `cpython_grammar_equal_comparison_subset` |
| `NOTEQUAL` | supported | `lexes_comparison_operators`, `cpython_grammar_equal_comparison_subset` |
| `LESSEQUAL` | supported | `lexes_comparison_operators`, `cpython_grammar_ordering_comparison_subset` |
| `GREATEREQUAL` | supported | `lexes_comparison_operators`, `cpython_grammar_ordering_comparison_subset` |
| `TILDE` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_unary_ops_subset` |
| `CIRCUMFLEX` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_bitwise_and_shift_subset` |
| `LEFTSHIFT` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_bitwise_and_shift_subset` |
| `RIGHTSHIFT` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_bitwise_and_shift_subset` |
| `DOUBLESTAR` | supported | `lexes_arithmetic_operators`, `cpython_grammar_power_and_paren_precedence_subset` |
| `PLUSEQUAL` | supported | `lexes_augmented_assignment_operators`, `cpython_ast_augmented_assignment_subset` |
| `MINEQUAL` | supported | `lexes_augmented_assignment_operators`, `cpython_ast_augmented_assignment_subset` |
| `STAREQUAL` | supported | `lexes_augmented_assignment_operators`, `cpython_ast_augmented_assignment_subset` |
| `SLASHEQUAL` | supported | `lexes_augmented_assignment_operators`, `cpython_ast_augmented_assignment_subset` |
| `PERCENTEQUAL` | supported | `lexes_augmented_assignment_operators`, `cpython_ast_augmented_assignment_subset` |
| `AMPEREQUAL` | supported | `lexes_bitwise_augmented_assignment_operators`, `cpython_ast_bitwise_augmented_assignment_subset` |
| `VBAREQUAL` | supported | `lexes_bitwise_augmented_assignment_operators`, `cpython_ast_bitwise_augmented_assignment_subset` |
| `CIRCUMFLEXEQUAL` | supported | `lexes_bitwise_augmented_assignment_operators`, `cpython_ast_bitwise_augmented_assignment_subset` |
| `LEFTSHIFTEQUAL` | supported | `lexes_bitwise_augmented_assignment_operators`, `cpython_ast_bitwise_augmented_assignment_subset` |
| `RIGHTSHIFTEQUAL` | supported | `lexes_bitwise_augmented_assignment_operators`, `cpython_ast_bitwise_augmented_assignment_subset` |
| `DOUBLESTAREQUAL` | supported | `lexes_augmented_assignment_operators`, `cpython_ast_augmented_assignment_subset` |
| `DOUBLESLASH` | supported | `lexes_arithmetic_operators`, `cpython_grammar_multiplicative_ops_subset` |
| `DOUBLESLASHEQUAL` | supported | `lexes_augmented_assignment_operators`, `cpython_ast_augmented_assignment_subset` |
| `AT` | supported | `lexes_decorator_at_sign`, `cpython_tokenize_exact_type_subset` including CPython pathological trailing whitespace, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, `runs_matrix_multiply_special_methods` |
| `ATEQUAL` | supported | `lexes_matrix_multiply_and_ellipsis_tokens`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, `runs_matrix_multiply_special_methods` |
| `RARROW` | supported | `lexes_function_return_arrow`, `cpython_grammar_annotations_subset` |
| `ELLIPSIS` | supported | `lexes_matrix_multiply_and_ellipsis_tokens`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset` |
| `COLONEQUAL` | supported | `lexes_colon_equal`, `cpython_assignment_expression_subset`, and named-expression runtime tests cover the walrus token |
| `EXCLAMATION` | supported | f-string and t-string conversion `!` is covered by `lexes_f_string_parts`, `lexes_f_string_debug_expressions`, `cpython_tokenize_f_string_split_token_subset`, `cpython_tokenize_t_string_split_token_subset`, `cpython_f_string_helper_rules_subset`, `cpython_f_string_basic_subset`, `cpython_f_string_debug_expression_subset`, `cpython_invalid_f_string_syntax_subset`, `runs_f_strings`, and `runs_f_string_expressions` |
| `OP` | out_of_scope_runtime | Aggregate tokenize.py token category rather than parser input token |
| `TYPE_IGNORE` | supported | `lexes_type_comments_and_type_ignores`, `cpython_type_comments_and_ignores_subset`, and `skips_type_comments_and_type_ignores` cover type-ignore trivia |
| `TYPE_COMMENT` | supported | `lexes_type_comments_and_type_ignores`, `cpython_type_comments_and_ignores_subset`, `cpython_func_type_comment_helper_rules_subset`, and `skips_type_comments_and_type_ignores` cover statement and function type comments |
| `SOFT_KEYWORD` | supported | `keeps_match_and_case_as_soft_keywords`, `cpython_grammar_match_stmt_subset`, `cpython_type_alias_statement_subset`, `cpython_type_params_metadata_subset`, and `cpython_lazy_import_syntax_subset` cover contextual soft-keyword behavior |
| `FSTRING_START` | partial | MiniPython's parser still consumes collapsed `Token::FString`, while `tokenize_cpython_with_spans()` now exposes first-pass split tokens; covered by `cpython_tokenize_f_string_split_token_subset` for CPython `FSTRING_START` spans across plain, raw-prefix, recursively nested, escaped-brace, debug-padding, multiline literal, non-ASCII/emoji, cross-line expression, and multiline triple-quoted f-strings, plus `lexes_f_string_parts`, `lexes_f_string_format_specs`, `lexes_f_string_debug_expressions`, `lexes_raw_f_string_literals_and_empty_format_specs`, `cpython_tokenize_f_string_span_subset` including CPython-derived start/end source spans, plain and raw-prefix f-strings, single/triple-quoted f-strings, multiline triple f-strings, multiline non-ASCII and emoji f-strings, replacement fields with lambda/non-ASCII/newline expression source, nested f-string expression source, ordinary and raw line continuations, `!r` conversions, debug expressions, and format specs, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_f_string_basic_subset`, `cpython_f_string_conversion_operator_edge_subset`, `cpython_f_string_contextual_runtime_subset`, `cpython_f_string_format_error_subset`, `cpython_f_string_many_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_invalid_f_string_syntax_subset`, `runs_f_strings`, `runs_f_string_expressions` |
| `FSTRING_MIDDLE` | partial | MiniPython's parser still stores f-string middles as collapsed `TokenFStringPart::Literal` values, while `tokenize_cpython_with_spans()` now exposes first-pass split middles; covered by `cpython_tokenize_f_string_split_token_subset` for CPython literal middles, escaped-brace middles around replacement fields, conversion-expression separators, debug-expression padding, multiline debug replacement fields, literal middles between multiple replacement fields, multiline expression newlines without synthetic indent/dedent tokens, physical `NL` tokens inside bracketed replacement expressions, multiline format-spec middles including unevenly indented post-colon literal middles, non-ASCII/emoji middles, nested f-string expression token boundaries, and nested format-spec replacement fields, plus `lexes_f_string_parts`, `lexes_f_string_escaped_brace_literals`, `lexes_f_string_backslash_before_doubled_braces`, `lexes_f_string_line_continuations`, `lexes_f_string_format_specs`, `lexes_f_string_debug_expressions`, `lexes_raw_f_string_literals_and_empty_format_specs`, `cpython_tokenize_f_string_span_subset` including literal/expression part checks, escaped-brace, backslash-before-doubled-brace, ordinary/raw line-continuation literals, multiline triple-quoted middles, non-ASCII and emoji middle text around replacement fields, lambda/non-ASCII/newline replacement expression source, conversion fields, format-spec middles, and debug-expression labels in `cpython_f_string_basic_subset` / `cpython_string_line_continuation_subset`, warning coverage in `cpython_f_string_escape_warning_subset`, `cpython_f_string_conversion_operator_edge_subset`, `cpython_f_string_contextual_runtime_subset`, `cpython_f_string_format_error_subset`, `cpython_f_string_many_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_invalid_f_string_syntax_subset`, `runs_f_strings`, `runs_f_string_expressions` |
| `FSTRING_END` | partial | MiniPython's parser still consumes collapsed `Token::FString`, while `tokenize_cpython_with_spans()` now exposes first-pass split end tokens; covered by `cpython_tokenize_f_string_split_token_subset` for CPython end-token spans across single-quoted, double-quoted, triple-quoted, multiline triple-quoted, non-ASCII/emoji, cross-line-expression, and recursively nested f-strings, plus `lexes_f_string_parts`, `lexes_f_string_format_specs`, `lexes_f_string_debug_expressions`, `lexes_raw_f_string_literals_and_empty_format_specs`, `cpython_tokenize_f_string_span_subset` including whole-literal source spans for single-line, triple-quoted, multiline, non-ASCII multiline, line-continuation, conversion, format-spec, debug-expression, lambda/newline-expression, and nested-expression f-strings, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_f_string_basic_subset`, `cpython_f_string_conversion_operator_edge_subset`, `cpython_f_string_contextual_runtime_subset`, `cpython_f_string_format_error_subset`, `cpython_f_string_many_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_invalid_f_string_syntax_subset`, `runs_f_strings`, `runs_f_string_expressions` |
| `TSTRING_START` | partial | MiniPython's parser still consumes collapsed `Token::TString`, while `tokenize_cpython_with_spans()` now exposes first-pass split tokens; covered by `cpython_tokenize_t_string_split_token_subset` for CPython-style t-string start spans across ordinary, raw-prefix, debug-padding, nested-format, multiline literal, non-ASCII/emoji, cross-line expression, and multiline triple-quoted t-strings, plus `lexes_t_string_parts`, `cpython_tokenize_t_string_span_subset` including CPython-derived whole-literal source spans, literal-only templates, ordinary and multiple interpolations, expression-source preservation, `rt`/`tr` raw prefixes, `!s`/`!r`/`!a` conversions, debug fields, format specs, nested format-spec replacement fields, and multiline triple-quoted t-strings, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_t_string_basic_subset`, `cpython_t_string_nested_template_and_runtime_error_subset`, `cpython_t_string_raw_concat_and_triple_subset`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_invalid_t_string_syntax_subset`, `runs_t_strings` |
| `TSTRING_MIDDLE` | partial | MiniPython's parser still stores t-string middles as collapsed `TokenFStringPart::Literal` values, while `tokenize_cpython_with_spans()` now exposes first-pass split middles; covered by `cpython_tokenize_t_string_split_token_subset` for literal middles, literal middles between multiple interpolation fields, raw-prefix literal tails after replacement fields, `!r` conversion tokens, debug-expression padding, multiline expression newlines without synthetic indent/dedent tokens, physical `NL` tokens inside bracketed replacement expressions, format-spec middles including unevenly indented post-colon literal middles, non-ASCII/emoji middles, and nested format-spec replacement fields, plus `lexes_t_string_parts`, `cpython_tokenize_t_string_span_subset` including literal/interpolation part checks, raw-prefix literal middles, conversion fields, debug labels, literal and nested-expression format specs, and triple-quoted multiline literal segments, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_t_string_basic_subset`, `cpython_t_string_nested_template_and_runtime_error_subset`, `cpython_t_string_raw_concat_and_triple_subset`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_invalid_t_string_syntax_subset`, `runs_t_strings` |
| `TSTRING_END` | partial | MiniPython's parser still consumes collapsed `Token::TString`, while `tokenize_cpython_with_spans()` now exposes first-pass split end tokens; covered by `cpython_tokenize_t_string_split_token_subset` for CPython-style t-string end spans across ordinary, raw-prefix, nested-format, multiline, non-ASCII/emoji, and cross-line-expression t-strings, plus `lexes_t_string_parts`, `cpython_tokenize_t_string_span_subset` including final source spans across single-line, conversion, debug, raw-prefix, nested-format, and multiline triple-quoted t-strings, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_t_string_basic_subset`, `cpython_t_string_nested_template_and_runtime_error_subset`, `cpython_t_string_raw_concat_and_triple_subset`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_invalid_t_string_syntax_subset`, `runs_t_strings` |
| `COMMENT` | supported | Comments and type-comment/ignore comments are covered by `cpython_tokenize_comments_subset`, `lexes_type_comments_and_type_ignores`, and `cpython_type_comments_and_ignores_subset` |
| `NL` | supported | Non-logical physical newlines inside blank/comment lines, implicit line joining including comments and physical newlines inside bracketed blocks, explicit continuation-only lines including blank physical lines after a backslash join, a final whitespace-only physical line without a trailing newline, and a final comment-only physical line without a trailing newline are covered by `cpython_tokenize_comments_subset`, `cpython_tokenize_implicit_line_joining_subset`, `cpython_tokenize_explicit_line_joining_subset`, and `cpython_tokenize_trailing_space_without_newline_subset` |
| `ERRORTOKEN` | partial | `rejects_unknown_character`, `rejects_invalid_non_printable_characters`, `rejects_null_bytes_with_cpython_message`, `rejects_unclosed_bracketed_statements`, `rejects_unmatched_closing_brackets`, `rejects_unterminated_string`, `rejects_cpython_unterminated_string_forms`, `rejects_cpython_invalid_string_escape_forms`, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_invalid_string_literal_subset`, `cpython_invalid_f_string_syntax_subset`, `cpython_invalid_t_string_syntax_subset`, `cpython_tokenize_explicit_line_joining_subset` including invalid line-continuation spans, `cpython_tokenize_error_token_subset` including invalid-character, non-breaking-space, and CPython `test_invisible_characters` non-printable-control spans, unmatched and mismatched bracket spans, cross-line mismatched closing bracket spans, EOF-in-multiline spans for bare and populated open brackets, unterminated triple-quoted string spans, single-quoted multiline f-string token errors, line-continuation EOF, null-byte spans, and too-deep bracket nesting |
| `ENCODING` | partial | `cpython_source_encoding_detection_subset` covers representative CPython `Lib/test/test_tokenize.py::TestDetectEncoding` and `Lib/test/test_source_encoding.py` behavior for default UTF-8 detection, first- and second-line coding cookies, latin-1 and UTF-8 normalization including CPython's 12-character `get_normal_name()` prefix behavior, ignored second-line cookies after real code, UTF-8 BOM stripping, BOM/cookie mismatch errors, null bytes in coding lines, unknown encodings, and ASCII/UTF-8 decode failures. `cpython_tokenize_bytes_encoding_token_subset` covers CPython-style leading `ENCODING` tokens from byte tokenization, BOM stripping before tokenization, latin-1 decoding, iso-8859-15 decoding, `cp1252`, `cp949`, `cp932`, and `cp1251` decoding, source CRLF normalization in triple-quoted strings, a `latin-1` long-comment source without a final newline, ASCII rejection of non-ASCII source bytes, and default-UTF-8 rejection of invalid bytes inside f-string middle text. `cpython_source_encoding_execution_subset` covers CPython-style execution of supported bytes source through decoding, parser, compiler, and VM, including UTF-8 default/BOM, first- and second-line iso-8859-15 cookies, double coding lines, long coding-cookie lines, long latin-1 coding-name normalization, long UTF-8 comment lines, non-UTF-8 coding-cookie comments, ignored third-line cookies, source CRLF/CR normalization in triple-quoted strings, non-UTF-8 shebangs with matching cookies, very long `latin-1` comment-only source without a final newline, `cp1252`, `cp949`, `cp932`, and `cp1251` source execution, partial UTF-8 BOM decode errors, representative BOM/cookie and ASCII decode errors, and invalid default-UTF-8 bytes inside f-string middle text. `cpython_bytes_source_output_parity_subset` adds CPython differential output parity for actual bytes files using default UTF-8 decoding, UTF-8 cookies, ISO-8859-15 first-line/second-line/empty-first-line cookies, ignored third-line cookies, double coding-line precedence, ISO-8859-15 cookie lines containing non-UTF-8 bytes, UTF-8 BOM default/comment/cookie handling, a UTF-8 BOM empty source line, CPython encoded-module `iso-8859-1` and `koi8-r` samples, CPython `tokenizedata/coding20731.py`, `cp949`, `cp932`, `cp1252`, and `cp1251`; `cpython_bytes_exec_source_output_parity_subset` covers current CPython `exec(bytes)` parity for long first- and second-line coding cookies, long coding-cookie lines, long normalized Latin-1 coding names, and long UTF-8 comment-only lines; `cpython_bytes_source_rejection_parity_subset` adds differential rejection parity for unknown cookies, BOM/cookie mismatches including second-line and fake-cookie BOM cases, partial UTF-8 and UTF-16-LE BOMs, ASCII-cookie body decode failures, default-UTF-8 second- and third-line decode failures, invalid f-string middle bytes, and CPython `tokenizedata/bad_coding*.py` samples. MiniPython still relies on the migrated manual decoders plus the `encoding_rs` label set rather than CPython's full codecs registry. |

## Starting Rules

| CPython rule | Status | Rust evidence |
| --- | --- | --- |
| `file` | supported | `cpython_ast_snippets_parse_inventory_subset`, `cpython_compile_crlf_newlines_subset`, `cpython_compile_specifics_newline_and_indentation_subset`, `cpython_compile_specifics_compile_stability_subset`, `runs_multiple_statements` |
| `interactive` | supported | `cpython_ast_snippets_parse_inventory_subset`, `cpython_interactive_input_subset`, `runs_interactive_input_mode`, `rejects_interactive_multiple_physical_statements` |
| `eval` | supported | `cpython_ast_snippets_parse_inventory_subset`, `cpython_eval_input_subset`, `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset`, `evaluates_eval_input_expression` |
| `func_type` | supported | `cpython_func_type_input_subset`, `cpython_type_expression_helper_rules_subset`, `parses_func_type_input_mode` |
| `statements` | supported | `cpython_ast_snippets_parse_inventory_subset`, `runs_multiple_statements`, `cpython_grammar_suite_and_dedent_subset` |
| `statement` | supported | `cpython_ast_snippets_parse_inventory_subset`, `runs_multiple_statements`, `runs_if_then_branch`, `runs_while_loop`, `runs_for_loop_over_range` |
| `single_compound_stmt` | supported | `cpython_statement_newline_subset`, `cpython_interactive_input_subset`, `runs_interactive_input_mode` |
| `statement_newline` | supported | `cpython_statement_newline_subset`, `cpython_interactive_input_subset`, `runs_interactive_input_mode`, `rejects_interactive_multiple_physical_statements` |
| `simple_stmts` | supported | `cpython_simple_stmts_subset`, `cpython_grammar_semicolon_simple_statements_subset`, `runs_semicolon_separated_simple_statements` |

## Simple Statements

| CPython rule | Status | Rust evidence |
| --- | --- | --- |
| `simple_stmt` | supported | `cpython_ast_snippets_parse_inventory_subset`, `runs_pass_statement`, `assigns_and_reads_variable`, `prints_number`, `cpython_simple_stmts_subset`, `cpython_grammar_semicolon_simple_statements_subset`, `cpython_grammar_import_stmt_subset`, `cpython_lazy_import_syntax_subset`, `cpython_type_alias_statement_subset`, `cpython_type_params_metadata_subset`, `cpython_invalid_assignment_target_subset`, `cpython_invalid_assignment_and_annotation_subset`, `cpython_invalid_simple_statement_subset` |
| `assignment` | supported | CPython's annotated-name, annotated attribute/subscript, chained `star_targets =`, augmented-assignment, and invalid-assignment alternatives are covered by `cpython_assignment_rule_alternatives_subset`, `assigns_and_reads_variable`, `runs_chained_assignment`, `reassigns_variable`, `cpython_assignment_target_helper_rules_subset`, `cpython_ast_tuple_unpacking_subset`, `unpacks_starred_assignments`, `compiles_starred_unpack_assignment_to_bytecode`, `cpython_ast_augmented_assignment_subset`, `cpython_ast_bitwise_augmented_assignment_subset`, `cpython_augassign_operator_subset`, `cpython_annotated_rhs_subset`, `cpython_invalid_assignment_target_subset`, `cpython_invalid_assignment_and_annotation_subset`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, `cpython_ast_subscript_assignment_subset`, `cpython_ast_slice_assignment_subset`, `cpython_user_defined_subscript_protocol_subset`, `cpython_grammar_annotations_subset`, and `runs_variable_annotations`; representative invalid assignment diagnostics also assert parser source spans |
| `annotated_rhs` | supported | `cpython_annotated_rhs_subset`, `runs_variable_annotations`, `runs_generator_send_values` |
| `augassign` | supported | `cpython_augassign_operator_subset`, `cpython_ast_augmented_assignment_subset`, `cpython_ast_bitwise_augmented_assignment_subset`, `cpython_invalid_assignment_and_annotation_subset`, `cpython_user_defined_subscript_protocol_subset`, `parses_matrix_augmented_assignment_statement`, `runs_augmented_assignments`, `runs_augmented_bitwise_assignments` |
| `invalid_assignment` | supported | Invalid annotated targets, tuple annotations, illegal annotation expressions, invalid chained assignment targets, yield assignment, illegal augmented-assignment expressions, and unparenthesized named expressions before slice colons are rejected by `cpython_invalid_assignment_and_annotation_subset` and `cpython_invalid_assignment_target_subset`; representative literal, keyword, function-call, operator, and walrus-target failures assert parser source spans |
| `invalid_ann_assign_target` | supported | Tuple, list, and parenthesized tuple/list annotated-assignment targets are rejected by `cpython_invalid_assignment_and_annotation_subset` |
| `star_targets` | supported | Single, comma, optional-trailing-comma, tuple/list, and one-star target forms are covered by `cpython_assignment_target_helper_rules_subset`, `cpython_ast_tuple_unpacking_subset`, and `unpacks_starred_assignments` |
| `star_target` | supported | Plain and starred assignment targets, including rejected bare starred targets and duplicate starred targets, are covered by `cpython_assignment_target_helper_rules_subset` and `cpython_ast_tuple_unpacking_subset` |
| `star_targets_list_seq` | supported | List target sequences with optional trailing commas and starred elements are covered by `cpython_assignment_target_helper_rules_subset` and `cpython_ast_tuple_unpacking_subset` |
| `star_targets_tuple_seq` | supported | Tuple target sequences with multiple elements, single trailing-comma elements, optional trailing commas, and starred elements are covered by `cpython_assignment_target_helper_rules_subset` and `cpython_ast_tuple_unpacking_subset` |
| `star_atom` | supported | Name, parenthesized, tuple, list, attribute, subscript, and slice assignment atoms are covered by `cpython_assignment_target_helper_rules_subset`, `cpython_ast_subscript_assignment_subset`, and `cpython_ast_slice_assignment_subset` |
| `target_with_star_atom` | supported | Attribute and subscript assignment targets over names, chained primaries, call results, generator-expression call results, named-expression subscript indexes, parenthesized named-expression slice bounds, and atom targets are covered by `cpython_assignment_target_helper_rules_subset` and `cpython_assignment_expression_subset` |
| `single_target` | supported | Name, parenthesized, attribute, subscript, and call-result single targets for annotated and augmented assignment are covered by `cpython_assignment_target_helper_rules_subset`, `cpython_ast_subscript_assignment_subset`, `cpython_ast_slice_assignment_subset`, and `runs_augmented_assignments` |
| `single_subscript_attribute_target` | supported | Attribute and subscript single targets, including call-result primaries and named-expression subscript indexes, are covered by `cpython_assignment_target_helper_rules_subset`, `cpython_assignment_expression_subset`, `cpython_user_defined_subscript_protocol_subset`, and `deletes_names_attributes_and_subscripts` |
| `t_primary` | supported | Recursive target primaries for attributes, subscripts, calls, generator-expression calls, and atoms are covered by `cpython_assignment_target_helper_rules_subset` |
| `t_lookahead` | supported | The `(`, `[`, and `.` target-primary continuations are covered by `cpython_assignment_target_helper_rules_subset` |
| `type_alias` | supported | `cpython_type_alias_statement_subset`, `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_scope_expression_subset`, `runs_function_class_and_alias_type_params`, `runs_generic_alias_type_subscripts` |
| `return_stmt` | supported | `cpython_ast_return_stmt_subset`, `cpython_finally_control_flow_warning_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset`, `cpython_invalid_control_flow_context_subset`, `returns_none_from_bare_return`, `reports_return_outside_function` |
| `raise_stmt` | supported | `cpython_grammar_raise_and_try_except_subset` including custom exception classes deriving from `Exception`, custom exception `__class__` identity, and subclass exceptions caught by a custom base handler, `cpython_base_exception_args_subset` including BaseException argument tuple/display/repr behavior, `cpython_base_exception_with_traceback_none_subset` including the `with_traceback(None)` path and non-traceback rejection, `cpython_system_exit_oserror_attributes_subset` including `SystemExit.code` and `OSError` attribute/args normalization plus CPython-style `OSError.__str__`, `cpython_syntax_error_attributes_subset` including `SyntaxError` construction attributes, `cpython_unicode_error_attributes_subset` including Unicode encode/decode/translate construction attributes, `cpython_attribute_error_keyword_attributes_subset` including `AttributeError(name=..., obj=...)`, `cpython_builtin_exception_hierarchy_subset` including standard builtin exception base-class catches, `cpython_invalid_simple_statement_subset`, `reports_unhandled_raise`, `preserves_explicit_exception_cause`, `preserves_implicit_exception_context`, `supports_raise_from_none`, `rejects_invalid_exception_cause` |
| `pass_stmt` | supported | `cpython_grammar_pass_statement_subset`, `runs_pass_statement` |
| `break_stmt` | supported | `cpython_grammar_break_continue_subset` covers plain break, `while False` break, nested try/except continue-then-break regression behavior, and if/else/break loop flow; `cpython_finally_control_flow_warning_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset`, `cpython_invalid_control_flow_context_subset`, `runs_break_inside_while_loop` |
| `continue_stmt` | supported | `cpython_grammar_break_continue_subset` covers plain continue, `while False` continue, continue through try/except and try/finally, and nested continue-then-break regression behavior; `cpython_finally_control_flow_warning_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset`, `cpython_invalid_control_flow_context_subset`, `runs_continue_inside_while_loop` |
| `global_stmt` | supported | `cpython_grammar_global_stmt_subset`, `cpython_global_binding_targets_subset`, `cpython_scope_declaration_error_subset`, `writes_global_name_from_function`, `augassigns_global_name_from_function` |
| `nonlocal_stmt` | supported | `cpython_scope_closure_and_nonlocal_subset`, `cpython_nonlocal_binding_targets_subset`, `cpython_scope_declaration_error_subset`, `writes_nonlocal_name_from_nested_function`, `nonlocal_writes_nearest_enclosing_scope` |
| `del_stmt` | supported | `cpython_delete_target_helper_rules_subset`, `cpython_grammar_del_stmt_subset`, `cpython_invalid_assignment_target_subset`, `cpython_invalid_simple_statement_subset`, `deletes_names_attributes_and_subscripts`, `deletes_list_slices`, `reports_delete_errors` |
| `del_targets` | supported | `cpython_delete_target_helper_rules_subset`, `cpython_grammar_del_stmt_subset`, `deletes_names_attributes_and_subscripts`, `deletes_list_slices` |
| `del_target` | supported | Name, attribute, subscript, tuple/list, and parenthesized delete targets are covered by `cpython_delete_target_helper_rules_subset`, `cpython_grammar_del_stmt_subset`, and `deletes_names_attributes_and_subscripts` |
| `del_t_atom` | supported | `cpython_delete_target_helper_rules_subset`, `cpython_grammar_del_stmt_subset`, `deletes_names_attributes_and_subscripts`, `deletes_list_slices` |
| `type_expressions` | supported | `cpython_func_type_input_subset`, `cpython_type_expression_helper_rules_subset`, `parses_func_type_input_mode` |
| `func_type_comment` | supported | Inline and own-line function type comments for `def` and `async def` are accepted by `cpython_func_type_comment_helper_rules_subset`; unrelated `# type: ignore` comments are skipped by `skips_type_comments_and_type_ignores` |
| `invalid_double_type_comments` | supported | Duplicate inline plus own-line function type comments are rejected for `def` and `async def` by `cpython_func_type_comment_helper_rules_subset` |
| `yield_stmt` | supported | `cpython_grammar_yield_stmt_subset`, `cpython_yield_expression_helper_rule_subset`, `runs_generator_yield_with_next`, `runs_for_loop_over_generator`, `runs_yield_from_expression`, `runs_generator_send_values`, `runs_generator_throw_values`, `runs_generator_close_values`, `catches_stop_iteration_from_next`; comprehension outer-iterable `yield` is accepted while comprehension-internal `yield` is rejected |
| `assert_stmt` | supported | `cpython_grammar_assert_stmt_subset`, `cpython_invalid_simple_statement_subset`, `runs_assert_statement_when_condition_is_truthy` |
| `invalid_raise_stmt` | supported | Missing raise value and missing raise cause forms are rejected by `cpython_invalid_simple_statement_subset` |
| `invalid_del_stmt` | supported | Literal, starred, function-call, conditional, operator, named-expression, and nested invalid delete targets are rejected by `cpython_invalid_simple_statement_subset` |
| `invalid_assert_stmt` | supported | Accidental assignment and unparenthesized named-expression assert forms are rejected by `cpython_invalid_simple_statement_subset` |
| `import_stmt` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, `cpython_lazy_import_syntax_subset`, `parses_relative_import_ellipsis_levels`, `cpython_ast_validator_stdlib_recursive_compile_seed_subset`, `rejects_invalid_import_forms_with_cpython_messages`, `runs_import_statement`, `runs_from_import_statement`, `runs_import_aliases_and_star_import`, `runs_lazy_import_syntax` |
| `import_name` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, `cpython_lazy_import_syntax_subset`, `runs_import_statement`, `runs_import_aliases_and_star_import`, `runs_lazy_import_syntax` |
| `import_from` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, `cpython_lazy_import_syntax_subset`, `parses_relative_import_ellipsis_levels`, `cpython_ast_validator_stdlib_recursive_compile_seed_subset`, `runs_from_import_statement`, `runs_import_aliases_and_star_import`, `runs_lazy_import_syntax` |
| `import_from_targets` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, `cpython_lazy_import_syntax_subset`, `cpython_ast_validator_stdlib_recursive_compile_seed_subset`, `rejects_invalid_import_forms_with_cpython_messages`, `runs_from_import_statement`, `runs_import_aliases_and_star_import` |
| `import_from_as_names` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `runs_from_import_statement`, `runs_import_aliases_and_star_import` |
| `import_from_as_name` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `runs_from_import_statement`, `runs_import_aliases_and_star_import` |
| `dotted_as_names` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `runs_import_statement`, `runs_import_aliases_and_star_import` |
| `dotted_as_name` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `runs_import_statement`, `runs_import_aliases_and_star_import` |
| `dotted_name` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `parses_relative_import_ellipsis_levels`, `runs_import_statement`, `runs_from_import_statement`, `runs_import_aliases_and_star_import` |
| `invalid_import` | supported | Missing import names, reversed `import ... from ...` order, and malformed parenthesized imports are rejected by `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, and `rejects_invalid_import_forms_with_cpython_messages` |
| `invalid_dotted_as_name` | supported | Invalid import aliases, including `__debug__`, attributes, calls, tuples, literals, capitalized `As`, and missing `as`, are rejected by `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, and `rejects_invalid_import_forms_with_cpython_messages` |
| `invalid_import_from_as_name` | supported | Invalid from-import aliases, including `__debug__`, attributes, calls, lists, tuples, subscripts, literals, capitalized `As`, and missing `as`, are rejected by `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, and `rejects_invalid_import_forms_with_cpython_messages` |
| `invalid_import_from_targets` | supported | Empty from-import targets, malformed parenthesized target lists, and non-parenthesized trailing commas are rejected by `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, and `rejects_invalid_import_forms_with_cpython_messages` |

## Compound Statements

| CPython rule | Status | Rust evidence |
| --- | --- | --- |
| `compound_stmt` | supported | `cpython_ast_snippets_parse_inventory_subset`, function, class, if, with, for, try, while, and match compound statements are covered by `cpython_compound_stmt_rule_alternatives_subset`, `cpython_function_def_decorated_rule_subset`, `cpython_class_def_decorated_rule_subset`, `runs_if_then_branch`, `runs_while_loop`, `runs_for_loop_over_range`, `cpython_invalid_block_subset`, `cpython_invalid_control_flow_syntax_subset`, `cpython_invalid_control_flow_context_subset`, `cpython_compile_control_flow_edge_subset`, `cpython_compile_stack_size_stability_control_flow_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset`, `cpython_grammar_with_stmt_subset`, `cpython_grammar_async_with_subset`, `cpython_grammar_try_star_subset`, and `cpython_grammar_match_stmt_subset` |
| `invalid_block` | supported | Missing indentation after compound-statement headers is rejected by `cpython_invalid_block_subset` including parser diagnostic spans and `cpython_invalid_control_flow_syntax_subset` |
| `block` | supported | Indented suites, inline simple-statement bodies, and missing-indent errors are covered by `cpython_grammar_suite_and_dedent_subset`, `cpython_simple_stmts_subset`, and `cpython_invalid_block_subset` including parser diagnostic spans |
| `if_stmt` | supported | `cpython_grammar_if_else_subset`, `cpython_grammar_elif_subset`, `cpython_invalid_control_flow_syntax_subset` |
| `elif_stmt` | supported | `cpython_grammar_elif_subset`, `runs_elif_branch`, `cpython_invalid_control_flow_syntax_subset` |
| `else_block` | supported | `cpython_grammar_if_else_subset`, `cpython_grammar_while_subset`, `cpython_grammar_for_subset`, `cpython_grammar_raise_and_try_except_subset`, `cpython_invalid_control_flow_syntax_subset`, `cpython_invalid_control_flow_context_subset` |
| `while_stmt` | supported | `cpython_grammar_while_subset`, `runs_while_else_after_condition_finishes_loop`, `cpython_invalid_control_flow_syntax_subset`, `cpython_compile_control_flow_edge_subset`, `cpython_control_flow_in_finally_override_subset` |
| `for_stmt` | supported | `cpython_grammar_for_subset`, `cpython_grammar_async_for_subset`, `cpython_builtin_range_for_iteration_subset`, `runs_for_loop_over_list`, `cpython_ast_tuple_unpacking_subset`, `cpython_invalid_assignment_target_subset`, `cpython_invalid_control_flow_syntax_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset` including CPython-derived break/continue through `try/finally`, inside `finally`, inside `with`, and inside `async with` |
| `async_for_stmt` | supported | `cpython_grammar_async_for_subset` including CPython-derived protocol errors for missing `__aiter__`, missing `__anext__`, non-awaitable `__anext__` results, `__anext__` results whose `__await__` raises while preserving `__cause__`, async-for/async-with nesting with `else`, `__aiter__` exception propagation before loop-body execution, and `StopAsyncIteration` raised while assigning async-for targets propagating instead of ending the loop, `runs_async_for_loop` |
| `invalid_if_stmt` | supported | Missing `if` colons and missing indented `if` blocks are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_elif_stmt` | supported | Top-level `elif`, missing `elif` colons, and missing indented `elif` blocks are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_else_stmt` | supported | Top-level `else`, missing indented `else` blocks, and `elif` following an `else` block are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_while_stmt` | supported | Missing `while` colons and missing indented `while` blocks are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_for_stmt` | supported | Missing ordinary and async `for` colons plus missing indented ordinary and async `for` blocks are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_for_target` | supported | Invalid ordinary and async `for` assignment targets are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `async_with_stmt` | supported | `cpython_grammar_async_with_subset` including CPython-derived non-awaitable `__aenter__` / `__aexit__` result errors, preserved `__context__`, `__aexit__` exception propagation, `__aenter__` exception propagation without calling `__aexit__`, and truthy async-exit suppression, `cpython_control_flow_inside_except_and_with_subset` including async target-binding failures that await `__aexit__` and async context manager protocol errors for missing `__aenter__` / `__aexit__` methods, `runs_async_with_statement`, `runs_parenthesized_async_with_items` |
| `with_stmt` | supported | `cpython_grammar_with_stmt_subset`, `cpython_invalid_control_flow_syntax_subset`, `cpython_invalid_assignment_target_subset`, `cpython_control_flow_inside_except_and_with_subset` including target-binding failures that call `__exit__`, sync and async context manager protocol errors for missing enter/exit methods plus sync/async manager mixup hints, multi-manager cleanup for later manager `__init__`, `__enter__`, and `__exit__` failures, `__exit__` result truthiness errors, complex sequence targets, generator `yield` inside `with`, and grouped context expressions followed by `as`, `runs_with_statement`, `runs_multiple_with_items_as_nested_managers`, `runs_parenthesized_with_items`, `runs_grouped_with_item_as_target`, `calls_with_exit_before_propagating_exception`, `with_exit_can_suppress_exception`, `with_exit_can_suppress_exceptions`, `with_exit_can_propagate_exceptions`, `with_exit_runs_before_return_break_and_continue` |
| `with_item` | supported | `cpython_grammar_with_stmt_subset`, `cpython_grammar_async_with_subset`, `cpython_invalid_control_flow_syntax_subset`, `runs_grouped_with_item_as_target` |
| `invalid_with_stmt` | supported | Missing ordinary and async `with` colons, including parenthesized forms and multiple items, are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_with_stmt_indent` | supported | Missing indented ordinary and async `with` blocks, including parenthesized forms, are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_with_item` | supported | Invalid ordinary, async, parenthesized, and comma-separated `with ... as` targets are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `try_stmt` | supported | `cpython_grammar_raise_and_try_except_subset` including custom exception subclass matching and custom exception `__class__` identity, `cpython_base_exception_args_subset` including caught exception argument preservation, `cpython_base_exception_with_traceback_none_subset` including catchable invalid-traceback `TypeError`, `cpython_system_exit_oserror_attributes_subset`, `cpython_syntax_error_attributes_subset`, `cpython_unicode_error_attributes_subset`, and `cpython_attribute_error_keyword_attributes_subset` including builtin exception attribute preservation, `cpython_builtin_exception_hierarchy_subset` including `ArithmeticError` and `LookupError` handler matching plus `GeneratorExit` / `Exception` separation, `cpython_invalid_control_flow_syntax_subset`, `cpython_invalid_control_flow_context_subset`, `cpython_runtime_exception_capture_subset`, `cpython_control_flow_in_finally_subset`, `cpython_control_flow_in_finally_override_subset` including issue #37830 pending-return override cases, `cpython_control_flow_inside_except_and_with_subset`, `cpython_grammar_try_star_subset`, `cpython_except_star_split_semantics_subset`, `cpython_except_star_rejects_exception_group_types_subset`, `catches_raised_exceptions`, `catches_tuple_exception_handlers`, `catches_dotted_exception_handler_type`, `catches_dynamic_exception_handler_type_expression`, `runs_except_star_handlers`, `splits_exception_groups_for_except_star_handlers`, `rejects_exception_group_types_in_except_star_handlers`, `runs_try_else_when_no_exception`, `runs_finally_without_exception`, `runs_finally_after_handled_exception`, `runs_finally_before_reraising_exception`, `runs_finally_before_returning_from_function`, `runs_finally_before_breaking_from_loop`, `runs_finally_before_continuing_loop` |
| `except_block` | supported | `cpython_grammar_raise_and_try_except_subset`, `cpython_invalid_control_flow_syntax_subset`, `cpython_runtime_exception_capture_subset`, `cpython_control_flow_inside_except_and_with_subset`, `catches_raised_exceptions`, `catches_tuple_exception_handlers`, `catches_dotted_exception_handler_type`, `catches_dynamic_exception_handler_type_expression` |
| `except_star_block` | supported | `cpython_grammar_try_star_subset`, `cpython_invalid_control_flow_syntax_subset`, `cpython_except_star_split_semantics_subset`, `cpython_except_star_rejects_exception_group_types_subset`, `runs_except_star_handlers`, `splits_exception_groups_for_except_star_handlers`, `rejects_exception_group_types_in_except_star_handlers`, `rejects_invalid_except_star_control_flow` |
| `finally_block` | supported | `cpython_finally_control_flow_warning_subset`, `cpython_control_flow_in_finally_subset`, `cpython_control_flow_in_finally_override_subset` including issue #37830 pending-return override cases, `cpython_invalid_control_flow_syntax_subset`, `runs_finally_without_exception`, `runs_finally_after_handled_exception`, `runs_finally_before_reraising_exception`, `runs_finally_before_returning_from_function`, `runs_finally_before_breaking_from_loop`, `runs_finally_before_continuing_loop` |
| `invalid_try_stmt` | supported | Missing try blocks, try statements without except/finally, and mixed except/except* handlers are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_except_stmt` | supported | Missing except colons, bare except without colon, unparenthesized multi-type `except ... as`, CPython `test_syntax.py` attribute targets after `except ... as`, other invalid `as` targets, and missing except blocks are rejected by `cpython_invalid_control_flow_syntax_subset` and `cpython_grammar_raise_and_try_except_subset` |
| `invalid_except_star_stmt` | supported | Missing except* colons, missing except* types, unparenthesized multi-type `except* ... as`, invalid `as` targets, and missing except* blocks are rejected by `cpython_invalid_control_flow_syntax_subset` and `cpython_grammar_try_star_subset` |
| `invalid_finally_stmt` | supported | Top-level `finally` and missing indented finally blocks are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_except_stmt_indent` | supported | Missing indented typed and bare except blocks are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `invalid_except_star_stmt_indent` | supported | Missing indented except* blocks are rejected by `cpython_invalid_control_flow_syntax_subset` |
| `decorators` | supported | `cpython_grammar_decorators_subset`, `parses_decorated_function_after_blank_line`, `runs_function_decorators`, `runs_expression_decorators`, `runs_class_decorators` |
| `function_def` | supported | Decorated and undecorated function definitions, including decorated async functions, are covered by `cpython_function_def_decorated_rule_subset`, `cpython_ast_function_def_subset`, `cpython_grammar_decorators_subset`, `cpython_grammar_annotations_subset`, `cpython_func_type_comment_helper_rules_subset`, `cpython_invalid_function_def_raw_subset`, `cpython_positional_only_arguments_subset`, `cpython_ast_function_defaults_and_keywords_subset`, `cpython_ast_starred_function_parameters_subset`, `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_param_subset`, `cpython_invalid_assignment_target_subset`, `cpython_function_globals_attribute_subset` for supported `function.__globals__` and `dir(function)` behavior, `cpython_function_builtins_attribute_subset` for supported `function.__builtins__` behavior, `defines_and_calls_function`, `runs_function_default_parameters`, `runs_function_keyword_arguments`, `runs_function_decorators`, `runs_function_annotations`, `runs_function_class_and_alias_type_params`, `runs_generic_alias_type_subscripts`, `runs_positional_only_parameters`, `runs_varargs_functions`, `runs_kwargs_functions`, `runs_keyword_only_parameters`, and `runs_recursive_function` |
| `function_def_raw` | supported | Ordinary `def` and `async def` headers with optional type params, params, return annotations, function type comments, and inline/indented bodies are covered by `cpython_function_def_raw_rule_subset`, `cpython_ast_function_def_subset`, `cpython_func_type_comment_helper_rules_subset`, `cpython_invalid_function_def_raw_subset`, `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `defines_and_calls_function`, and `runs_function_annotations` |
| `invalid_def_raw` | supported | Missing function-header parentheses, missing colons, and missing indented suites for ordinary, async, typed, and type-parameterized definitions are rejected by `cpython_invalid_function_def_raw_subset` including parser diagnostic spans |
| `async_funcdef` | supported | `cpython_async_funcdef_rule_subset`, `cpython_grammar_async_await_subset`, `cpython_grammar_generator_expression_subset`, `cpython_func_type_comment_helper_rules_subset`, `cpython_invalid_function_def_raw_subset`, `cpython_invalid_assignment_target_subset`, `runs_async_function_and_await_expression`, and `runs_coroutine_throw_and_close_methods` cover CPython's `async def` rule shape, empty coroutine bodies, type parameters, complex parameters, return annotations, function type comments, nested async functions, coroutine functions, CPython's conversion of unhandled coroutine `StopIteration` into `RuntimeError` with `__cause__`, CPython-style coroutine `__await__()` wrappers with iterator identity/reuse behavior, non-None send rejection on just-started coroutines, returned `StopIteration` objects staying ordinary return values, custom `__await__` iterators whose yielded values suspend through the outer coroutine and whose return values complete the await expression, CPython-style rejection of objects without `__await__` and of `__await__` returning `None`, a coroutine, or another non-iterator object, awaited expression composition, nested await, await in keyword arguments and tuple values, send/throw forwarding into suspended await expressions including CPython's custom-exception `test_await_14` shape, rejection of awaiting an already-suspended coroutine, returned exception values with cleared await context, async generators, async-generator `anext`/`asend`/`athrow`/`aclose`, `StopAsyncIteration` exhaustion, and CPython's rejection of `yield from` inside async functions and `return value` inside async generators |
| `class_def` | supported | Decorated and undecorated class definitions are covered by `cpython_class_def_decorated_rule_subset`, `cpython_grammar_class_def_subset`, `cpython_class_def_raw_helper_rules_subset`, `cpython_grammar_decorators_subset`, `cpython_grammar_annotations_subset`, `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_scope_expression_subset`, `cpython_invalid_assignment_target_subset`, `defines_and_instantiates_class`, `runs_class_attributes`, `cpython_compile_static_attributes_exact_subset` for CPython-style `__static_attributes__`, `runs_class_decorators`, `runs_class_annotations`, `runs_class_inheritance_and_header_arguments`, CPython parity for user-class `isinstance` over direct instances and base classes, `cpython_issubclass_builtin_subset` for supported class hierarchy checks, `cpython_type_builtin_subset` for one-argument `type()`, `cpython_type_dynamic_class_subset` for first-pass three-argument dynamic class construction, `cpython_type_name_qualname_subset` for mutable dynamic-class `__name__` and `__qualname__`, `cpython_type_doc_and_firstlineno_subset` for dynamic-class `__doc__` and `__firstlineno__`, `cpython_vars_dir_builtin_subset` for first-pass class/instance `vars()` and `dir()` introspection, `cpython_bound_method_metadata_subset` for bound-method metadata and class-body method aliasing, `cpython_bound_method_descriptor_and_repr_subset` for bound-method `__get__` and stable repr metadata, `cpython_bound_method_identity_subset` for stored bound-method identity and fresh attribute-access method objects, `cpython_unbound_super_descriptor_subset` for one-argument/unbound `super` descriptor rebinding and metadata, `runs_function_class_and_alias_type_params`, `runs_generic_alias_type_subscripts`, and `runs_instance_methods` |
| `class_def_raw` | supported | Raw class headers with optional type params, empty argument lists, positional bases, keyword/unpacked header arguments, and inline/indented bodies are covered by `cpython_class_def_raw_helper_rules_subset`, `cpython_grammar_class_def_subset`, `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_scope_expression_subset`, `runs_class_inheritance_and_header_arguments`, `runs_function_class_and_alias_type_params`, and `runs_generic_alias_type_subscripts` |
| `invalid_class_def_raw` | supported | Missing class-header colons and missing indented suites for plain, based, and type-parameterized class headers are rejected by `cpython_class_def_raw_helper_rules_subset` |
| `match_stmt` | supported | Valid match suites, multiple case blocks, inline and indented case bodies, invalid empty match suites, and the delegated invalid alternatives are covered by `cpython_grammar_match_stmt_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_invalid_match_pattern_subset`, `runs_match_literal_cases`, `runs_match_complex_literal_patterns`, `runs_match_class_patterns`, `runs_match_wildcard_case`, `runs_match_or_pattern`, `runs_match_sequence_patterns`, `runs_match_star_sequence_patterns`, `runs_match_as_patterns`, `runs_match_value_patterns`, `runs_match_mapping_patterns`, `runs_match_open_sequence_patterns`, `runs_match_capture_pattern`, `runs_match_guards`, and `keeps_match_and_case_as_soft_keywords` |
| `subject_expr` | supported | Named-expression subjects, tuple subjects, optional trailing commas, and starred tuple subjects are covered by `cpython_match_pattern_helper_rules_subset` and `cpython_grammar_match_stmt_subset` |
| `case_block` | supported | Multiple case blocks, wildcard cases, guarded cases, indented suites, and inline suites are covered by `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `cpython_invalid_match_pattern_subset` |
| `guard` | supported | Guarded cases, boolean guards, and named expressions inside guards are covered by `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_guards` |
| `patterns` | supported | Open sequence patterns and single patterns are covered by `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_open_sequence_patterns` |
| `pattern` | supported | `as_pattern` and `or_pattern` alternatives are covered by `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, `runs_match_as_patterns`, and `runs_match_or_pattern` |
| `as_pattern` | supported | As-pattern binding, nested as-patterns, wildcard captures, invalid targets, and parenthesized OR-as patterns are covered by `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, `cpython_invalid_match_pattern_subset`, and `runs_match_as_patterns` |
| `or_pattern` | supported | Literal/value OR-patterns, same-name capture alternatives, reordered capture alternatives, mapping alternatives, parenthesized `as` alternatives, non-final irrefutable alternatives, and different-name binding errors are covered by `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_or_pattern` |
| `closed_pattern` | supported | Literal, capture, wildcard, value, group, sequence, mapping, and class alternatives are covered by `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and the focused match helper tests |
| `literal_pattern` | supported | Numeric, signed, complex, adjacent-string, boolean, and None literal patterns are covered by `cpython_grammar_match_stmt_subset`; f-string and t-string match values are rejected by the parser like CPython |
| `literal_expr` | supported | Literal mapping keys, adjacent-string keys, complex keys, singleton keys, dotted value keys, duplicate literal-key rejection, and rejected f-string/t-string mapping keys are covered by `cpython_match_pattern_helper_rules_subset` and `cpython_grammar_match_stmt_subset` |
| `complex_number` | supported | Complex literal plus/minus alternatives in patterns and mapping keys are covered by `cpython_match_numeric_literal_helper_rules_subset` |
| `signed_number` | supported | Positive and negative numeric literal patterns are covered by `cpython_match_numeric_literal_helper_rules_subset` |
| `signed_real_number` | supported | Positive and negative real parts of complex literal patterns are covered by `cpython_match_numeric_literal_helper_rules_subset` |
| `real_number` | supported | Real numeric literal patterns are covered by `cpython_match_numeric_literal_helper_rules_subset` |
| `imaginary_number` | supported | Imaginary literal patterns and complex imaginary parts are covered by `cpython_match_numeric_literal_helper_rules_subset` |
| `capture_pattern` | supported | Capture patterns are covered by `cpython_match_capture_wildcard_group_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, and `runs_match_capture_pattern` |
| `pattern_capture_target` | supported | Capture targets and their `_`, `.`, `(`, and `=` lookahead exclusions are covered by `cpython_match_capture_target_and_star_pattern_helper_rules_subset` and `cpython_match_pattern_helper_rules_subset` |
| `wildcard_pattern` | supported | Wildcard cases and grouped wildcards are covered by `cpython_match_capture_wildcard_group_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_wildcard_case` |
| `value_pattern` | supported | Simple and recursive dotted value patterns, dotted mapping keys, and invalid equality/dangling-dot forms are covered by `cpython_match_value_attr_name_or_attr_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, and `runs_match_value_patterns` |
| `attr` | supported | Recursive dotted attributes in value patterns, mapping keys, and class names are covered by `cpython_match_value_attr_name_or_attr_helper_rules_subset` and `runs_match_value_patterns` |
| `name_or_attr` | supported | Bare class names and dotted class/value pattern prefixes are covered by `cpython_match_value_attr_name_or_attr_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `group_pattern` | supported | Parenthesized capture, wildcard, and sequence group patterns are covered by `cpython_match_capture_wildcard_group_helper_rules_subset` and `cpython_match_pattern_helper_rules_subset` |
| `sequence_pattern` | supported | Bracketed and parenthesized sequence alternatives, empty sequences, optional trailing commas, and star-containing sequences are covered by `cpython_match_sequence_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_sequence_patterns` |
| `open_sequence_pattern` | supported | Naked comma sequence patterns and parenthesized comma sequence patterns are covered by `cpython_match_sequence_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_open_sequence_patterns` |
| `maybe_sequence_pattern` | supported | One-or-more sequence subpatterns with optional trailing commas are covered by `cpython_match_sequence_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `maybe_star_pattern` | supported | Plain and starred sequence subpatterns, including wildcard star targets and duplicate-star rejection, are covered by `cpython_match_sequence_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_star_sequence_patterns` |
| `star_pattern` | supported | `*name` and `*_` sequence star patterns are covered by `cpython_match_capture_target_and_star_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_star_sequence_patterns` |
| `mapping_pattern` | supported | Empty mappings, pure rest mappings, item-only mappings, item-plus-rest mappings, trailing commas, invalid rest placement, static duplicate literal key rejection, and dynamic duplicate dotted-key `ValueError` behavior are covered by `cpython_match_mapping_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, `runs_match_mapping_patterns`, and `raises_value_error_for_dynamic_duplicate_match_mapping_keys` |
| `items_pattern` | supported | Single and multiple mapping pattern items are covered by `cpython_match_mapping_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `key_value_pattern` | supported | Literal keys, dotted value keys, nested value patterns, and dynamic duplicate key checks are covered by `cpython_match_mapping_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `raises_value_error_for_dynamic_duplicate_match_mapping_keys` |
| `double_star_pattern` | supported | Mapping rest patterns, optional trailing comma, and invalid `_` rest targets are covered by `cpython_match_mapping_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_mapping_patterns` |
| `class_pattern` | supported | Empty, positional-only, keyword-only, positional-plus-keyword, dotted-name, trailing-comma, builtin match-self classes, zero-positional builtin classes, non-class callees, and invalid keyword/positional ordering forms are covered by `cpython_match_class_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_class_patterns` |
| `positional_patterns` | supported | One-or-more positional class subpatterns with optional trailing commas, nested subpatterns, builtin positional-count errors, and no-binding-on-TypeError behavior are covered by `cpython_match_class_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `keyword_patterns` | supported | One-or-more class keyword subpatterns, keyword-only forms, and mixed positional-plus-keyword forms are covered by `cpython_match_class_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `keyword_pattern` | supported | Individual class keyword subpatterns with nested pattern values and duplicate-keyword rejection are covered by `cpython_match_class_helper_rules_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `invalid_match_stmt` | supported | Missing match colons, missing match indentation, and top-level case blocks are rejected by `cpython_invalid_match_pattern_subset` |
| `invalid_case_block` | supported | Missing case colons and missing case indentation with and without guards are rejected by `cpython_invalid_match_pattern_subset` |
| `invalid_as_pattern` | supported | `_`, literal expression, CPython `test_syntax.py` attribute targets after `case ... as`, and call-form as-pattern targets are rejected by `cpython_invalid_match_pattern_subset` |
| `invalid_class_pattern` | supported | Positional class patterns after keyword patterns are rejected by `cpython_invalid_match_pattern_subset` |
| `invalid_mapping_pattern` | supported | Mapping rest patterns before other mapping items, both with and without preceding items, and invalid f-string mapping keys are rejected by `cpython_invalid_match_pattern_subset` |
| `invalid_class_argument_pattern` | supported | Class pattern argument ordering errors with and without leading positional patterns are rejected by `cpython_invalid_match_pattern_subset` |

## Expressions

| CPython rule | Status | Rust evidence |
| --- | --- | --- |
| `expressions` | supported | Comma-separated expression tuples, single-expression trailing-comma tuples, and single-expression alternatives are covered by `cpython_ast_snippets_parse_inventory_subset`, `cpython_expressions_helper_rules_subset`, `prints_multiple_arguments`, `cpython_ast_print_multiple_arguments`, `cpython_eval_input_subset`, and `prints_naked_tuple_expression` |
| `expression` | supported | The `invalid_if_expression`, `invalid_expression`, `invalid_legacy_expression`, `if_expression`, `disjunction`, and `lambdef` alternatives are covered by `cpython_invalid_expression_rules_subset`, `cpython_grammar_conditional_expression_subset`, `cpython_expression_without_invalid_subset`, `cpython_grammar_boolean_operations_subset`, `cpython_grammar_lambda_subset`, `runs_boolean_operators`, `evaluates_eval_input_expression`, and `cpython_compile_expression_stack_size_shapes_subset` long-expression compile-shape checks |
| `star_expressions` | supported | Multi-element, single-trailing-comma, plain expression, and starred-expression alternatives are covered across assignment RHS, return values, for-loop iterables, annotated RHS values, yield values, displays, and comprehensions by `cpython_star_expressions_helper_rules_subset`, `cpython_grammar_starred_displays_subset`, `cpython_comprehension_unpacking_subset`, `runs_starred_sequence_displays`, and `runs_comprehension_unpacking` |
| `star_expression` | supported | Starred `*bitwise_or` and plain `expression` alternatives are covered by `cpython_star_expressions_helper_rules_subset`, `cpython_grammar_starred_displays_subset`, `cpython_comprehension_unpacking_subset`, `runs_starred_sequence_displays`, and `runs_comprehension_unpacking` |
| `star_named_expressions` | supported | Comma-separated named-expression/starred-expression lists with optional trailing commas are covered by `cpython_star_named_expression_helper_rules_subset`, `cpython_grammar_starred_displays_subset`, `cpython_comprehension_unpacking_subset`, `cpython_grammar_match_stmt_subset`, and `runs_starred_sequence_displays` |
| `star_named_expressions_sequence` | supported | Star-named expression sequences, optional trailing commas, and invalid starred-expression sequence rejection are covered by `cpython_star_named_expression_helper_rules_subset`, `cpython_invalid_starred_expression_subset`, `cpython_grammar_starred_displays_subset`, `cpython_comprehension_unpacking_subset`, and `runs_starred_sequence_displays` |
| `star_named_expression` | supported | Starred `*bitwise_or` and `named_expression` alternatives are covered by `cpython_star_named_expression_helper_rules_subset`, `cpython_grammar_starred_displays_subset`, `cpython_comprehension_unpacking_subset`, `cpython_grammar_match_stmt_subset`, and `runs_starred_sequence_displays` |
| `star_named_expression_sequence` | supported | Valid star-named expression elements and the invalid-starred-expression-unpacking sequence alternative are covered by `cpython_star_named_expression_helper_rules_subset`, `cpython_invalid_starred_expression_subset`, `cpython_grammar_starred_displays_subset`, `cpython_comprehension_unpacking_subset`, and `runs_starred_sequence_displays` |
| `invalid_starred_expression_unpacking` | supported | Unparenthesized conditional starred display expressions and starred call unpack assignment are rejected by `cpython_invalid_starred_expression_subset` |
| `invalid_starred_expression_unpacking_sequence` | supported | Dict unpacking in starred expression sequences and nested invalid starred unpacking are rejected by `cpython_invalid_starred_expression_subset` |
| `invalid_starred_expression` | supported | Empty starred expressions in display, call, and generic-alias contexts are rejected by `cpython_invalid_starred_expression_subset` |
| `await_expression` | supported | `cpython_await_primary_rule_subset`, `cpython_grammar_async_await_subset`, `cpython_grammar_generator_expression_subset`, `runs_async_function_and_await_expression`, and `reports_async_await_errors` cover plain awaits, awaited comprehension elements/filters/targets/iterables inside async functions, await-driven async generator expressions, awaited async-generator protocol objects, custom `__await__` iterators that yield through the coroutine and return an await result, CPython-style await/type errors for objects without `__await__` and invalid `__await__` return values, CPython's `await primary` precedence, awaited arithmetic composition, nested await, await in keyword arguments and tuple displays, send/throw forwarding through coroutine `__await__` wrappers, already-awaited coroutine rejection, and CPython's async-comprehension rejection outside async functions |
| `await_primary` | supported | `cpython_await_primary_rule_subset` covers CPython's `await primary` and plain `primary` alternatives, including awaited call, attribute, subscript, grouped primary, power-expression precedence as `(await primary) ** factor`, and invalid unary operands after `await`; async-generator protocol awaits and async-comprehension positions are also covered by `cpython_grammar_async_await_subset`, `cpython_grammar_generator_expression_subset`, `runs_async_function_and_await_expression`, and selector/call tests |
| `yield_expr` | supported | Bare yield, yield with `star_expressions`, yield from expression, yield inside f-string replacement fields, async-generator yield, comprehension outer-iterable `yield`, and comprehension-internal `yield` rejection are covered by `cpython_yield_expression_helper_rule_subset`, `cpython_grammar_yield_stmt_subset`, `cpython_grammar_generator_expression_subset`, `cpython_f_string_yield_expression_subset`, and `cpython_invalid_comprehension_subset` |
| `if_expression` | supported | `cpython_grammar_conditional_expression_subset`, `runs_conditional_expressions` |
| `expression_without_invalid` | supported | Conditional expressions, disjunctions, and lambdas are covered by `cpython_expression_without_invalid_subset` |
| `invalid_legacy_expression` | supported | Legacy `print` and `exec` statement expression syntax is rejected by `cpython_invalid_expression_rules_subset` |
| `invalid_expression` | supported | Missing commas, string-adjacent expressions, incomplete conditional expressions, CPython `test_syntax.py` conditional-expression branches that accidentally contain statements, statement-in-expression positions, and unparenthesized f-string/t-string lambda expressions are rejected by `cpython_invalid_expression_rules_subset` |
| `invalid_if_expression` | supported | Starred and double-starred else branches in conditional expressions are rejected by `cpython_invalid_expression_rules_subset` |
| `assignment_expression` | supported | The `NAME := expression` alternative is covered by `cpython_named_expression_helper_rules_subset`, `cpython_assignment_expression_subset`, `cpython_assignment_expression_comprehension_subset`, and named-expression runtime tests, including nested walrus bindings, condition/call/subscript positions, and comprehension scoping |
| `named_expression` | supported | The `assignment_expression`, `invalid_named_expression`, and plain `expression !':='` alternatives are covered by `cpython_named_expression_helper_rules_subset`, `cpython_invalid_named_expression_subset`, `cpython_assignment_expression_subset`, `cpython_assignment_expression_comprehension_subset`, `cpython_call_argument_helper_rules_subset`, `cpython_star_named_expression_helper_rules_subset`, `runs_named_expressions_in_allowed_expression_contexts`, and `runs_named_expressions_in_conditions_calls_and_subscripts` |
| `invalid_named_expression` | supported | Invalid walrus targets and accidental `=` after name, literal, operator, function-call, subscript, and attribute expressions in named-expression contexts are rejected by `cpython_invalid_named_expression_subset` |
| `annotation` | supported | The `':' expression` wrapper rule is covered by `cpython_annotation_helper_rule_subset`, plus function, return, module, class, positional-only, type-parameter, generic-alias, variable-annotation target side-effect, syntax-error, and class-inheritance annotation behavior in `cpython_grammar_annotations_subset`, `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset`, `cpython_positional_only_arguments_subset`, `cpython_type_params_generic_alias_subset`, `runs_function_annotations`, `runs_variable_annotations`, `runs_class_annotations`, and `runs_generic_alias_type_subscripts` |
| `disjunction` | supported | `cpython_grammar_boolean_operations_subset`, `returns_logical_operands_with_truthiness`, `cpython_compile_boolean_expression_exact_subset`, `cpython_compile_expression_stack_size_shapes_subset`, and the differential `boolean-expression-short-circuit-identity` case covering CPython `TestBooleanExpression` operand identity and exact `__bool__` call counts in mixed `and` / `or` chains |
| `conjunction` | supported | `cpython_grammar_boolean_operations_subset`, `short_circuits_boolean_operators`, `cpython_compile_boolean_expression_exact_subset`, `cpython_compile_expression_stack_size_shapes_subset`, and the differential `boolean-expression-short-circuit-identity` case covering CPython `TestBooleanExpression` operand identity and exact `__bool__` call counts in mixed `and` / `or` chains |
| `inversion` | supported | `cpython_grammar_boolean_operations_subset`, `runs_not_with_truthy_values` |
| `comparison` | supported | Plain `bitwise_or` comparisons, all comparison operators, and chained comparisons are covered by `cpython_grammar_*_comparison_subset`, `cpython_grammar_bitwise_and_shift_subset`, `cpython_comparison_helper_rules_subset`, and `cpython_compile_expression_stack_size_shapes_subset` long chained-comparison compile stability |
| `compare_op_bitwise_or_pair` | supported | All comparison operators consuming right-hand `bitwise_or` expressions are covered by `cpython_comparison_helper_rules_subset` |
| `eq_bitwise_or` | supported | Equality comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` |
| `noteq_bitwise_or` | supported | Inequality comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` |
| `lte_bitwise_or` | supported | Less-than-or-equal comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` |
| `lt_bitwise_or` | supported | Less-than comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` |
| `gte_bitwise_or` | supported | Greater-than-or-equal comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` |
| `gt_bitwise_or` | supported | Greater-than comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` |
| `notin_bitwise_or` | supported | `not in` comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` |
| `in_bitwise_or` | supported | `in` comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` |
| `isnot_bitwise_or` | supported | `is not` comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset`; singleton and mutable-container identity behavior is covered by `cpython_grammar_identity_comparison_subset` |
| `is_bitwise_or` | supported | `is` comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset`; singleton and mutable-container identity behavior is covered by `cpython_grammar_identity_comparison_subset` |
| `bitwise_or` | supported | `cpython_grammar_bitwise_and_shift_subset`; CPython differential parity for bitwise operator precedence is covered by `cpython_program_output_parity_smoke_subset` |
| `bitwise_xor` | supported | `cpython_grammar_bitwise_and_shift_subset`; CPython differential parity for bitwise operator precedence is covered by `cpython_program_output_parity_smoke_subset` |
| `bitwise_and` | supported | `cpython_grammar_bitwise_and_shift_subset`; CPython differential parity for bitwise operator precedence is covered by `cpython_program_output_parity_smoke_subset` |
| `shift_expr` | supported | `cpython_grammar_bitwise_and_shift_subset`; CPython differential parity for shift associativity is covered by `cpython_program_output_parity_smoke_subset` |
| `sum` | supported | `cpython_grammar_additive_ops_subset`, `runs_arithmetic_precedence`, and CPython differential parity for left-associative additive chains in `cpython_program_output_parity_smoke_subset` |
| `term` | supported | Multiplication, division, floor division, modulo, matrix multiplication, and left-associative term chains are covered by `cpython_grammar_multiplicative_ops_subset`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, `runs_matrix_multiply_special_methods`, `runs_division_modulo_and_power`, `runs_sequence_repetition_and_basic_len_list_builtins`, `reports_arithmetic_type_errors`, `reports_division_by_zero`, and CPython differential parity for multiplicative and matrix-multiply special-method behavior in `cpython_program_output_parity_smoke_subset` |
| `factor` | supported | `cpython_grammar_unary_ops_subset`, `runs_unary_arithmetic`, and CPython differential parity for unary precedence in `cpython_program_output_parity_smoke_subset` |
| `invalid_arithmetic` | supported | Unparenthesized `not` after `+`, `-`, `*`, `/`, `%`, `//`, and `@` is rejected by `cpython_invalid_arithmetic_and_factor_syntax_subset` |
| `invalid_factor` | supported | Unparenthesized `not` after unary `+`, `-`, and `~` is rejected by `cpython_invalid_arithmetic_and_factor_syntax_subset` |
| `power` | supported | `cpython_grammar_power_and_paren_precedence_subset`, `runs_division_modulo_and_power` |
| `primary` | supported | `cpython_primary_rule_subset` covers recursive attribute, call, generator-expression call, subscript, and atom alternatives; broader selector/call/subscript behavior is covered by `cpython_selector_helper_rules_subset`, `cpython_ast_redundant_parentheses_and_call_trailer_subset`, `cpython_ast_function_defaults_and_keywords_subset`, `cpython_ast_starred_call_arguments_subset`, `cpython_invalid_call_argument_syntax_subset`, `cpython_compile_expression_stack_size_shapes_subset` large function/method call compile-shape checks, `cpython_grammar_subscript_index_subset`, `cpython_grammar_slice_subset`, `cpython_grammar_class_def_subset`, `cpython_ast_subscript_assignment_subset`, `cpython_ast_slice_assignment_subset`, `cpython_user_defined_subscript_protocol_subset`, `cpython_type_params_generic_alias_subset`, and `runs_generic_alias_type_subscripts` |
| `slices` | supported | Single index, one-dimensional slice, tuple index, tuple of slices, starred subscript items, generic-alias unpack items, load/store/delete subscript paths, and first-pass `slice.indices()` normalization are covered by `cpython_selector_helper_rules_subset`, `cpython_grammar_subscript_index_subset`, `cpython_grammar_slice_subset`, `cpython_ast_subscript_assignment_subset`, `cpython_ast_slice_assignment_subset`, `cpython_user_defined_subscript_protocol_subset`, `runs_subscript_errors`, `slices_lists_and_strings`, `slices_ranges`, and `cpython_program_output_parity_smoke_subset` selector parity |
| `slice` | supported | The `a:b:c` slice alternative with omitted and present start/stop/step parts plus the `named_expression` index alternative and runtime `slice.indices()` behavior are covered by `cpython_selector_helper_rules_subset`, `cpython_grammar_subscript_index_subset`, `cpython_grammar_slice_subset`, `cpython_ast_slice_assignment_subset`, `cpython_assignment_expression_subset`, `cpython_user_defined_subscript_protocol_subset`, `slices_lists_and_strings`, and `slices_ranges` |
| `atom` | supported | `cpython_atom_rule_subset` covers names, singletons, strings, numbers, tuple/group/generator forms, list/list-comprehension forms, dict/set/comprehension forms, and ellipsis; broader literal/display behavior is covered by `cpython_display_helper_rules_subset`, `cpython_ast_constant_values_subset`, `cpython_grammar_prefixed_integer_literals_subset`, `cpython_grammar_imaginary_literals_subset`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, string/f-string/t-string helper tests, display/comprehension tests, runtime literal tests, and `cpython_program_output_parity_smoke_subset` atom display parity |
| `list` | supported | Empty lists, star-named expression sequences, named expressions, iterable unpacking, and optional trailing commas are covered by `cpython_sequence_display_helper_rules_subset`, `cpython_display_helper_rules_subset`, `cpython_ast_list_literal_subset`, `cpython_grammar_starred_displays_subset`, and `runs_starred_sequence_displays` |
| `tuple` | supported | Empty tuples, comma-disambiguated tuples, single-element tuples, star-named expression sequences, iterable unpacking, and optional trailing commas are covered by `cpython_sequence_display_helper_rules_subset`, `cpython_display_helper_rules_subset`, `cpython_ast_tuple_literal_subset`, `cpython_grammar_starred_displays_subset`, and `prints_tuple_literals` |
| `set` | supported | Star-named expression sequences, named expressions, iterable unpacking, duplicate collapsing, and optional trailing commas are covered by `cpython_sequence_display_helper_rules_subset`, `cpython_display_helper_rules_subset`, `cpython_grammar_set_comprehension_subset`, `cpython_grammar_starred_displays_subset`, and `runs_set_literals_and_comprehensions` |
| `dict` | supported | Empty dictionaries, key/value pairs, key-before-value and left-to-right display evaluation order, `**` unpack entries, mixed entries, optional trailing commas, and invalid dict display alternatives are covered by `cpython_dict_kvpair_helper_rules_subset`, `cpython_display_helper_rules_subset`, `cpython_ast_dict_literal_subset`, `cpython_compile_specifics_dict_evaluation_order_subset`, `cpython_dict_display_unpacking_subset`, `cpython_invalid_dict_display_syntax_subset`, and `runs_dict_display_unpacking` |
| `double_starred_kvpairs` | supported | Comma-separated key/value and unpack entries with optional trailing commas are covered by `cpython_dict_kvpair_helper_rules_subset`, `cpython_dict_display_unpacking_subset`, `cpython_invalid_dict_display_syntax_subset`, `cpython_comprehension_unpacking_subset`, `runs_dict_display_unpacking`, `runs_comprehension_unpacking`, `parses_dict_unpack_expression`, and `compiles_dict_unpack_expression_to_update_bytecode` |
| `double_starred_kvpair` | supported | Both `**bitwise_or` unpack entries and `kvpair` alternatives are covered by `cpython_dict_kvpair_helper_rules_subset`, `cpython_dict_display_unpacking_subset`, `cpython_invalid_dict_display_syntax_subset`, `cpython_comprehension_unpacking_subset`, `runs_dict_display_unpacking`, `runs_comprehension_unpacking`, `parses_dict_unpack_expression`, and `compiles_dict_unpack_expression_to_update_bytecode` |
| `kvpair` | supported | Expression keys and expression values separated by `:` are covered by `cpython_dict_kvpair_helper_rules_subset`, `cpython_display_helper_rules_subset`, `cpython_ast_dict_literal_subset`, `cpython_invalid_dict_display_syntax_subset`, and `runs_dict_display_unpacking` |
| `invalid_double_starred_kvpairs` | supported | Missing dictionary values, invalid starred/double-starred dictionary keys and values after plain or `**` entries, and unparenthesized conditional dict unpacking are rejected by `cpython_invalid_dict_display_syntax_subset` |
| `invalid_kvpair_unpacking` | supported | Invalid starred/double-starred dictionary keys and values, and unparenthesized conditional dict unpacking are rejected by `cpython_invalid_dict_display_syntax_subset` |
| `invalid_kvpair` | supported | Missing dictionary key colons, missing dictionary values, and invalid starred/double-starred dictionary values are rejected by `cpython_invalid_dict_display_syntax_subset` |
| `group` | supported | Parenthesized `yield_expr`, parenthesized `named_expression`, redundant parentheses, and invalid parenthesized starred expressions are covered by `cpython_group_helper_rule_subset`, `cpython_ast_redundant_parentheses_and_call_trailer_subset`, `cpython_grammar_starred_displays_subset`, and `reports_invalid_starred_group_expressions` |
| `invalid_group` | supported | `cpython_grammar_starred_displays_subset`, `reports_invalid_starred_group_expressions` cover parenthesized `*` and `**` group errors |
| `for_if_clauses` | supported | Synchronous and asynchronous comprehension clauses, awaited async-comprehension elements/filters, multiple clauses, multiple filters, missing `in`, invalid targets, iterable named-expression rejection, and target-expression named-expression conflicts are covered by `cpython_for_if_clause_helper_rules_subset`, `cpython_invalid_comprehension_subset`, and comprehension execution tests |
| `for_if_clause` | supported | Synchronous and asynchronous `for` clauses with `if` filters, awaited filter expressions, missing `in`, invalid targets, iterable named-expression rejection, and target-expression named-expression conflicts are covered by `cpython_for_if_clause_helper_rules_subset`, `cpython_invalid_comprehension_subset`, and comprehension execution tests |
| `fstring` | supported | Start/middle/end parsing, literal middles, replacement fields, whitespace-preserving debug fields, comment-stripped debug labels, debug comparisons from CPython `test_gh129093`, debug conversions and conversion formatting, ordinary escaped string, raw string, and bytes debug-expression source preservation, nested debug f-strings, nested f-string replacement expressions including same-quote prefixed strings inside replacement expressions, adjacent string/f-string concatenation, case-insensitive formatted/raw prefixes, VM formatting, scope/closure/name lookup from CPython f-string tests, and class-level `__format__` dispatch are covered by `cpython_f_string_helper_rules_subset`, `cpython_f_string_basic_subset`, `cpython_f_string_debug_expression_subset`, `cpython_f_string_scope_and_format_lookup_subset`, `cpython_format_builtin_and_custom_dunder_format_subset`, lexer/parser f-string tests including `lexes_prefixed_same_quote_string_inside_f_string_expression`, `runs_f_strings`, and `runs_f_string_expressions` |
| `fstring_middle` | supported | Literal middle tokens, replacement-field middles, escaped braces that remain literal text, raw quote round-trips, and backslash-before-doubled-brace literal text are covered by `cpython_f_string_helper_rules_subset`, `lexes_f_string_parts`, `lexes_f_string_escaped_brace_literals`, `lexes_f_string_backslash_before_doubled_braces`, `lexes_f_string_debug_expressions`, `parses_adjacent_f_strings_and_plain_strings`, `cpython_f_string_basic_subset`, `cpython_f_string_debug_expression_subset`, and `runs_f_strings` |
| `fstring_replacement_field` | supported | Replacement fields with ordinary expressions, parenthesized lambda expressions, multiline expression bodies including raw triple f-strings, nested f-string expressions, same-quote prefixed string literals inside replacement expressions, inner string escapes, raw string, bytes literals, triple-quoted string literals, adjacent string-literal concatenation, inline comments, comments immediately after the debug `=`, implicit newlines, `annotated_rhs` yield expressions, generator suspension/resume through yield expressions, parenthesized walrus expressions, `:=` format-spec disambiguation, whitespace-preserving debug `=`, comparison expressions before debug `=`, local/global/closure/name lookup, missing-name failures, conversions, debug conversion formatting, empty/full format specs, and nested format-spec fields are covered by `cpython_f_string_helper_rules_subset`, parser f-string tests including `lexes_prefixed_same_quote_string_inside_f_string_expression`, `cpython_f_string_basic_subset`, `cpython_f_string_triple_quoted_expression_subset`, `cpython_f_string_yield_expression_subset`, `cpython_f_string_debug_expression_subset`, `cpython_f_string_scope_and_format_lookup_subset`, and `runs_f_string_expressions` |
| `fstring_conversion` | supported | `!s`, `!r`, `!a`, debug-field conversions, and invalid conversions are covered by `cpython_f_string_helper_rules_subset`, `lexes_f_string_parts`, `lexes_f_string_debug_expressions`, `cpython_f_string_basic_subset`, `cpython_f_string_debug_expression_subset`, `runs_f_strings`, `runs_f_string_expressions`, and `cpython_invalid_f_string_syntax_subset` |
| `fstring_full_format_spec` | supported | Empty, literal, raw/non-raw escaped, right-brace greedy-matched, and nested-expression full format specs are covered by `cpython_f_string_helper_rules_subset`, `lexes_f_string_format_specs`, `lexes_raw_and_non_raw_f_string_format_spec_escapes`, `parses_f_string_format_spec`, `cpython_f_string_basic_subset`, `cpython_f_string_format_specifier_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_raw_f_string_format_spec_subset`, and `runs_f_string_expressions` |
| `fstring_format_spec` | supported | Literal format middle text and nested replacement fields, including raw/non-raw escapes, greedy `}` field termination, width, precision, alignment, alternate-form base prefixes, simple `s`/`d`/`b`/`o`/`x`/`X`/`c`/`f` format codes, zero-fill and `=` alignment, custom `__format__` format-spec delivery, inherited `object.__format__` rejection for non-empty specs, duplicate/mixed `,`/`_` grouping-option errors, and numeric grouping rendering for decimal integers, fixed-point floats, and underscore-grouped non-decimal integers, are covered by `cpython_f_string_helper_rules_subset`, `cpython_f_string_basic_subset`, `cpython_f_string_format_specifier_expressions_subset`, `cpython_format_builtin_and_custom_dunder_format_subset`, `cpython_format_grouping_option_errors_subset`, `cpython_format_grouping_rendering_subset`, `cpython_format_integer_codes_and_zero_alignment_subset`, `cpython_raw_f_string_format_spec_subset`, `runs_f_string_expressions`, and `formats_values_with_format_specs`, with CPython differential parity for nested format-spec expression output in `cpython_program_output_parity_smoke_subset` |
| `invalid_fstring_replacement_field` | supported | Empty f-string fields, CPython whitespace-only fields in ordinary and nested format-spec replacement fields, non-breaking-space non-printable character handling, unparenthesized lambda expression sources before format specs, invalid expression starts, line-continuation backslashes inside expression sources, invalid post-expression tokens, comments that hide the closing brace, missing expressions before `=`, `!`, `:`, and `}`, bad debug-field continuations, invalid debug conversions, and unterminated format specs are rejected by `cpython_invalid_f_string_syntax_subset` |
| `invalid_fstring_conversion_character` | supported | Missing and unsupported f-string conversion characters, including non-name conversion tokens, are rejected by `cpython_invalid_f_string_syntax_subset` |
| `tstring_format_spec_replacement_field` | supported | Nested replacement fields inside t-string format specs are covered by `cpython_string_and_tstring_helper_rules_subset`, `cpython_t_string_basic_subset`, and `runs_t_strings` |
| `tstring_format_spec` | supported | Literal and nested-expression t-string format specs are covered by `lexes_t_string_parts`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_t_string_basic_subset`, and `runs_t_strings` |
| `tstring_full_format_spec` | supported | Colon-prefixed t-string format specs, including nested replacement fields, are covered by `cpython_string_and_tstring_helper_rules_subset`, `cpython_t_string_basic_subset`, and `runs_t_strings` |
| `tstring` | supported | T-string start/middle/end parsing, literal-only templates, multiple interpolation execution, function-call, attribute/method-call, and dictionary-subscript interpolation values, whitespace-preserving debug fields, comments inside replacement fields, nested Template values, missing-variable runtime errors, adjacent t-string concatenation, and template concatenation are covered by `cpython_string_and_tstring_helper_rules_subset`, `cpython_t_string_basic_subset`, `cpython_t_string_nested_template_and_runtime_error_subset`, and `runs_t_strings` |
| `tstring_replacement_field` | supported | T-string replacement fields with ordinary and `annotated_rhs` yield expressions, conversions, whitespace-preserving debug expressions, comments immediately after the debug `=`, format specs, nested Template values, missing-name failures, function-call, attribute/method-call, and dictionary-subscript expressions are covered by `cpython_string_and_tstring_helper_rules_subset`, `cpython_t_string_basic_subset`, `cpython_t_string_nested_template_and_runtime_error_subset`, and `runs_t_strings` |
| `tstring_middle` | supported | Literal t-string middle parts and interpolation boundaries are covered by `lexes_t_string_parts`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_t_string_basic_subset`, and `runs_t_strings` |
| `string` | supported | Plain string literal forms, bytes literal forms, CPython string-prefix matrix forms, incompatible-prefix rejection, raw strings, triple quotes, escapes, and adjacent literal concatenation are covered by `lexes_string`, `lexes_bytes_literals`, `lexes_cpython_string_prefix_matrix`, `lexes_single_quoted_strings`, `lexes_raw_and_triple_quoted_strings`, `lexes_string_escapes`, `cpython_string_literal_and_concat_subset`, `cpython_bytes_literal_subset`, `cpython_string_prefix_matrix_subset`, `cpython_invalid_string_prefix_matrix_subset`, `cpython_string_and_tstring_helper_rules_subset`, `runs_python_string_literal_forms`, and `runs_python_bytes_literal_forms` |
| `strings` | supported | The `(fstring|string)+` and `tstring+` alternatives, adjacent plain/f-string/bytes concatenation, t-string concatenation, and invalid mixed concatenation are covered by `cpython_string_literal_and_concat_subset`, `cpython_bytes_literal_subset`, `cpython_f_string_helper_rules_subset`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_t_string_basic_subset`, and `cpython_invalid_t_string_syntax_subset` |
| `invalid_tstring_replacement_field` | supported | Empty t-string fields, CPython whitespace-only fields, non-breaking-space non-printable character handling, invalid expression starts, invalid post-expression tokens, comments that hide the closing brace, missing expressions before `=`, `!`, `:`, and `}`, bad debug-field continuations, invalid debug conversions, and unterminated format specs are rejected by `cpython_invalid_t_string_syntax_subset` |
| `invalid_tstring_conversion_character` | supported | Missing and unsupported t-string conversion characters, including non-name conversion tokens, are rejected by `cpython_invalid_t_string_syntax_subset` |
| `invalid_string_tstring_concat` | supported | T-string mixing with plain, unicode-prefixed, raw, f-string, raw f-string, bytes, and raw bytes adjacent literals in either order is rejected by `cpython_invalid_t_string_syntax_subset` |
| `invalid_comprehension` | supported | Dict unpacking in list/generator comprehensions, unparenthesized tuple comprehension targets for list/set comprehensions, named expressions in comprehension iterable expressions, named-expression rebinding of comprehension iteration variables, inner loops and target expressions that rebind earlier filter named-expression targets, direct named expressions in comprehension targets, comprehension-internal yield expressions, await-driven async comprehensions outside async functions including lambda-default boundaries, and class-body comprehension named expressions including lambda defaults are rejected by `cpython_invalid_comprehension_subset` |
| `invalid_for_if_clause` | supported | Synchronous and asynchronous comprehension clauses missing top-level `in` are rejected by `cpython_for_if_clause_helper_rules_subset` |
| `listcomp` | supported | Star-named list comprehension elements, ordinary clauses, async clauses, filters, invalid comprehension alternatives, unpacking elements, walrus scoping, and yield/await boundaries are covered by `cpython_comprehension_expression_rules_subset`, `cpython_grammar_list_comprehension_subset`, `cpython_invalid_comprehension_subset`, `cpython_comprehension_unpacking_subset`, `cpython_invalid_assignment_target_subset`, `cpython_assignment_expression_subset`, `cpython_yield_expression_helper_rule_subset`, `cpython_grammar_async_await_subset`, `runs_list_comprehensions`, and `runs_comprehension_unpacking` |
| `genexp` | supported | Plain expression, assignment-expression, starred-expression, awaited, and async-for generator elements plus clauses, invalid comprehension alternatives, unpacking elements, outer-iterable binding, walrus scoping, yield boundaries, async-generator execution, and async-generator protocol methods are covered by `cpython_comprehension_expression_rules_subset`, `cpython_grammar_generator_expression_subset`, `cpython_invalid_comprehension_subset`, `cpython_comprehension_unpacking_subset`, `cpython_assignment_expression_subset`, `cpython_yield_expression_helper_rule_subset`, `runs_generator_expressions`, `generator_expression_binds_outer_iterable_at_creation`, and `runs_comprehension_unpacking` |
| `dictcomp` | supported | Key/value dict comprehensions, `**expr` dict-unpack comprehensions, ordinary and async clauses, filters, unpacking elements, walrus scoping, and yield/await boundaries are covered by `cpython_comprehension_expression_rules_subset`, `cpython_grammar_dict_comprehension_subset`, `cpython_invalid_comprehension_subset`, `cpython_comprehension_unpacking_subset`, `cpython_assignment_expression_subset`, `cpython_yield_expression_helper_rule_subset`, `cpython_grammar_async_await_subset`, `runs_dict_comprehensions`, and `runs_comprehension_unpacking` |
| `setcomp` | supported | Star-named set comprehension elements, ordinary and async clauses, filters, invalid comprehension alternatives, unpacking elements, walrus scoping, and yield/await boundaries are covered by `cpython_comprehension_expression_rules_subset`, `cpython_grammar_set_comprehension_subset`, `cpython_invalid_comprehension_subset`, `cpython_comprehension_unpacking_subset`, `cpython_assignment_expression_subset`, `cpython_grammar_async_await_subset`, `runs_set_literals_and_comprehensions`, and `runs_comprehension_unpacking` |
| `lambdef` | supported | `cpython_grammar_lambda_subset` covers CPython `test_lambdef` alternatives, trailing-comma lambda parameter forms, nested lambda defaults, lambda/comprehension interaction, conditional-expression lambda bodies, invalid lambda bodies, and invalid parenthesized parameters; positional-only and closure behavior are also covered by `cpython_positional_only_arguments_subset`, `runs_lambda_expression`, `runs_positional_only_lambdas`, and `lambda_captures_closure` |
| `arguments` | supported | Positional, starred, keyword, double-starred, trailing-comma, and invalid call argument alternatives are covered by `cpython_call_argument_helper_rules_subset`, `cpython_ast_starred_call_arguments_subset`, `cpython_invalid_call_argument_syntax_subset`, `cpython_invalid_starred_expression_subset`, `runs_starred_call_arguments`, `runs_double_starred_call_arguments`, `runs_mixed_call_unpacking`, and `reports_call_unpacking_errors` |
| `args` | supported | Positional expression sequences, repeated starred unpacking, keyword tails, generator expression parenthesization, and invalid argument ordering are covered by `cpython_call_argument_helper_rules_subset`, `cpython_ast_starred_call_arguments_subset`, `cpython_invalid_call_argument_syntax_subset`, `cpython_invalid_starred_expression_subset`, `runs_starred_call_arguments`, and `reports_call_unpacking_errors` |
| `kwargs` | supported | Keyword/starred groups, keyword/double-starred groups, repeated `**` groups, and invalid keyword groups are covered by `cpython_call_argument_helper_rules_subset`, `cpython_ast_starred_call_arguments_subset`, `cpython_invalid_call_argument_syntax_subset`, `cpython_invalid_starred_expression_subset`, `runs_double_starred_call_arguments`, and `reports_call_unpacking_errors` |
| `starred_expression` | supported | Valid call `*expr` arguments and invalid starred call expressions are covered by `cpython_call_argument_helper_rules_subset`, `cpython_ast_starred_call_arguments_subset`, `cpython_invalid_call_argument_syntax_subset`, `cpython_invalid_starred_expression_subset`, `runs_starred_call_arguments`, and `reports_call_unpacking_errors` |
| `kwarg_or_starred` | supported | Named keyword arguments, starred arguments after keywords, duplicate keyword rejection, and invalid keyword targets are covered by `cpython_call_argument_helper_rules_subset`, `cpython_ast_starred_call_arguments_subset`, `cpython_invalid_call_argument_syntax_subset`, `cpython_invalid_starred_expression_subset`, `runs_starred_call_arguments`, and `reports_call_unpacking_errors` |
| `kwarg_or_double_starred` | supported | Named keyword arguments, `**expr` unpacking, repeated `**` unpacking, and invalid keyword-unpack assignment are covered by `cpython_call_argument_helper_rules_subset`, `cpython_ast_starred_call_arguments_subset`, `cpython_invalid_call_argument_syntax_subset`, `cpython_invalid_starred_expression_subset`, `runs_double_starred_call_arguments`, and `reports_call_unpacking_errors` |
| `invalid_arguments` | supported | Keyword-unpack followed by iterable-unpack, unparenthesized generator expressions in multi-argument calls, missing keyword values, positional-after-keyword calls, and repeated keyword syntax are rejected by `cpython_invalid_call_argument_syntax_subset` and `cpython_invalid_call_argument_helper_rules_subset` |
| `invalid_kwarg` | supported | Singleton keyword targets, `__debug__`, expression keyword targets, generator expressions after keyword assignment, and `**kwargs=...` forms are rejected by `cpython_invalid_call_argument_syntax_subset` and `cpython_invalid_call_argument_helper_rules_subset` |

## Function Parameters

| CPython rule | Status | Rust evidence |
| --- | --- | --- |
| `params` | supported | Valid function parameter alternatives and invalid function parameter alternatives are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_invalid_parameter_syntax_subset`, `cpython_invalid_parameters_subset`, and `cpython_ast_function_def_subset` |
| `parameters` | supported | Positional-only, positional-or-keyword, defaulted, starred, keyword-only, and `**kwargs` function parameter alternatives are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_ast_function_defaults_and_keywords_subset`, `cpython_ast_starred_function_parameters_subset`, and `cpython_positional_only_arguments_subset` |
| `slash_no_default` | supported | `cpython_positional_only_arguments_subset`, `runs_positional_only_parameters` |
| `slash_with_default` | supported | `cpython_positional_only_arguments_subset`, `runs_positional_only_parameters` |
| `star_etc` | supported | Varargs, starred-annotation varargs, keyword-only parameters, `**kwargs`, and invalid star alternatives are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_ast_starred_function_parameters_subset`, `cpython_invalid_parameter_syntax_subset`, `cpython_invalid_parameters_subset`, `runs_varargs_functions`, `runs_keyword_only_parameters`, and `runs_kwargs_functions` |
| `kwds` | supported | `**kwargs` with optional trailing comma, annotation, invalid defaults, and invalid followers are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_ast_starred_function_parameters_subset`, `cpython_invalid_parameter_syntax_subset`, `cpython_invalid_parameters_subset`, and `runs_kwargs_functions` |
| `param_no_default` | supported | Comma-terminated, close-paren-terminated, type-comment-bearing, and positional-only parameters without defaults are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_positional_only_arguments_subset`, and `cpython_ast_function_def_subset` |
| `param_no_default_star_annotation` | supported | Starred vararg annotations with and without a following `**kwargs` are covered by `cpython_function_parameter_helper_rules_subset` |
| `param_with_default` | supported | Comma-terminated, close-paren-terminated, type-comment-bearing, and positional-only parameters with defaults are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_ast_function_defaults_and_keywords_subset`, and `runs_function_default_parameters` |
| `param_maybe_default` | supported | Keyword-only parameters with and without defaults, including final and comma-terminated forms, are covered by `cpython_function_parameter_helper_rules_subset` and `cpython_ast_starred_function_parameters_subset` |
| `param` | supported | Parameter names with and without annotations across positional, keyword-only, starred, and `**kwargs` forms are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_grammar_annotations_subset`, and `runs_function_annotations` |
| `param_star_annotation` | supported | Starred parameter annotations are covered by `cpython_function_parameter_helper_rules_subset` and `cpython_star_annotation_helper_rule_subset` |
| `star_annotation` | supported | Starred parameter annotations, including annotations before defaults, are covered by `cpython_star_annotation_helper_rule_subset` |
| `default` | supported | Valid parameter default expressions and invalid missing default expressions are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_ast_function_defaults_and_keywords_subset`, `cpython_invalid_parameters_subset`, and `runs_function_default_parameters` |
| `invalid_parameters` | supported | Positional-only marker ordering, default ordering before and after `/`, parenthesized parameters, and slash/star ordering are rejected by `cpython_invalid_parameters_subset` |
| `invalid_default` | supported | Missing default values before `)`, `,`, and lambda `:` are rejected by `cpython_invalid_parameters_subset` and `cpython_invalid_lambda_parameters_subset` |
| `invalid_star_etc` | supported | Bare star, bare-star type-comment, `*, **kwargs`, vararg default, and repeated star forms are rejected by `cpython_invalid_parameters_subset` |
| `invalid_kwds` | supported | Kwarg defaults and parameters or `*`/`**`/`/` markers after `**kwargs` are rejected by `cpython_invalid_parameters_subset` |
| `invalid_parameters_helper` | supported | Default-before-non-default parameter ordering through both slash-with-default and ordinary default helpers is rejected by `cpython_invalid_parameters_subset` |
| `lambda_params` | supported | Valid lambda parameter alternatives and invalid lambda parameter alternatives are covered by `cpython_lambda_parameter_helper_rules_subset`, `cpython_grammar_lambda_subset`, `cpython_positional_only_arguments_subset`, `cpython_invalid_lambda_parameter_syntax_subset`, and `cpython_invalid_lambda_parameters_subset` |
| `lambda_parameters` | supported | Positional-only, positional-or-keyword, defaulted, starred, keyword-only, and `**kwargs` lambda parameter alternatives are covered by `cpython_lambda_parameter_helper_rules_subset`, `cpython_grammar_lambda_subset`, and `cpython_positional_only_arguments_subset` |
| `lambda_slash_no_default` | supported | `cpython_positional_only_arguments_subset`, `runs_positional_only_lambdas` |
| `lambda_slash_with_default` | supported | `cpython_positional_only_arguments_subset`, `runs_positional_only_lambdas` |
| `lambda_star_etc` | supported | Lambda varargs, keyword-only parameters, `**kwargs`, and invalid star alternatives are covered by `cpython_lambda_parameter_helper_rules_subset`, `cpython_grammar_lambda_subset`, `cpython_invalid_lambda_parameter_syntax_subset`, `cpython_invalid_lambda_parameters_subset`, and `runs_lambda_defaults_keywords_and_starred_parameters` |
| `lambda_kwds` | supported | Lambda `**kwargs`, invalid defaults, and invalid followers are covered by `cpython_lambda_parameter_helper_rules_subset`, `cpython_grammar_lambda_subset`, `cpython_invalid_lambda_parameter_syntax_subset`, `cpython_invalid_lambda_parameters_subset`, and `runs_lambda_defaults_keywords_and_starred_parameters` |
| `invalid_lambda_parameters` | supported | Positional-only marker ordering, default ordering before and after `/`, parenthesized parameters, and slash/star ordering are rejected by `cpython_invalid_lambda_parameters_subset` |
| `invalid_lambda_parameters_helper` | supported | Default-before-non-default lambda parameter ordering through both slash-with-default and ordinary default helpers is rejected by `cpython_invalid_lambda_parameters_subset` |
| `invalid_lambda_star_etc` | supported | Bare star, `*, **kwargs`, vararg default, and repeated star lambda forms are rejected by `cpython_invalid_lambda_parameters_subset` |
| `invalid_lambda_kwds` | supported | Kwarg defaults and parameters or `*`/`**`/`/` markers after `**kwargs` are rejected by `cpython_invalid_lambda_parameters_subset` |
| `lambda_param_no_default` | supported | Comma-terminated and colon-terminated lambda parameters without defaults are covered by `cpython_lambda_parameter_helper_rules_subset` and `cpython_grammar_lambda_subset` |
| `lambda_param_with_default` | supported | Comma-terminated and colon-terminated lambda parameters with defaults are covered by `cpython_lambda_parameter_helper_rules_subset` and `cpython_grammar_lambda_subset` |
| `lambda_param_maybe_default` | supported | Keyword-only lambda parameters with and without defaults are covered by `cpython_lambda_parameter_helper_rules_subset` and `cpython_grammar_lambda_subset` |
| `lambda_param` | supported | Lambda parameter names across positional, keyword-only, starred, and `**kwargs` forms are covered by `cpython_lambda_parameter_helper_rules_subset` and `cpython_grammar_lambda_subset` |

## Type Parameters

| CPython rule | Status | Rust evidence |
| --- | --- | --- |
| `type_params` | supported | Function, async function, class, and type-alias type parameter lists, including trailing commas and invalid empty lists, are covered by `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_params_subset`, `runs_function_class_and_alias_type_params`, and `runs_generic_alias_type_subscripts` |
| `type_param_seq` | supported | Single, multiple, variadic, ParamSpec, duplicate-name, and trailing-comma sequences are covered by `cpython_type_params_metadata_subset`, `cpython_type_params_duplicate_name_subset`, `cpython_type_params_generic_alias_subset`, `runs_function_class_and_alias_type_params`, and `runs_generic_alias_type_subscripts` |
| `type_param` | supported | Plain TypeVar, TypeVarTuple, ParamSpec, bound/default combinations, invalid variadic bounds, and invalid type-scope expressions are covered by `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_param_subset`, `runs_function_class_and_alias_type_params`, and `runs_generic_alias_type_subscripts` |
| `type_param_bound` | supported | Simple bounds, tuple constraint expressions, and named/yield/await rejection in bounds and constraints are covered by `cpython_type_params_metadata_subset`, `cpython_invalid_type_param_subset`, and `runs_function_class_and_alias_type_params` |
| `type_param_default` | supported | TypeVar and ParamSpec defaults with simple and generic-alias expressions plus invalid named/yield/await defaults are covered by `cpython_type_param_defaults_subset`, `cpython_invalid_type_param_subset`, and `runs_function_class_and_alias_type_params` |
| `type_param_starred_default` | supported | TypeVarTuple defaults with ordinary and starred expressions plus invalid yield defaults are covered by `cpython_type_param_defaults_subset`, `cpython_invalid_type_param_subset`, and `runs_function_class_and_alias_type_params` |
| `invalid_type_param` | supported | Bounds and constraints on TypeVarTuple and ParamSpec parameters, plus named/yield/await expressions in type parameter scopes, are rejected by `cpython_invalid_type_param_subset` |
| `invalid_type_params` | supported | Empty type parameter lists on functions, classes, and type aliases are rejected by `cpython_invalid_type_params_subset` |
