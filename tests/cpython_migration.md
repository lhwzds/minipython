# CPython Test Migration Plan

This file breaks the broad `partial` entries in `cpython_coverage.md` into
testable migration batches. Each batch should move through the same gates:

- `parse`: MiniPython accepts or rejects the same source shape as CPython.
- `ast`: MiniPython AST carries the same semantic shape, ignoring CPython
  source-location metadata.
- `compile`: register bytecode represents the same control/data flow.
- `vm`: observable output or error matches the supported semantic subset.
- `error`: CPython-invalid forms are rejected before VM execution.

The local source of truth is the CPython checkout at
`/Volumes/samsung/GitHub/cpython`.

`tests/cpython_grammar_inventory.md` is the rule-by-rule backlog. It currently
tracks all 276 CPython grammar rules. The inventory is a syntax/parser
coverage artifact only: a `supported` grammar row does not imply that the
runtime semantics, standard-library surface, sandbox behavior, or complete
CPython `Lib/test` module has been migrated.

`tests/cpython_diff.rs` is the executable CPython parity harness. It runs the
same source through CPython and MiniPython, comparing observable output for
accepted programs and rejection status for invalid programs. It defaults to
`python3`; set `MINIPYTHON_CPYTHON=/path/to/python` when validating newer
syntax against a local CPython checkout.

`tests/cpython_test_manifest.md` tracks CPython test modules by source test
method count. Use it to decide which CPython module or class group is actually
ported, partial, blocked by runtime/AST-module work, or not started.

## Current Snapshot

| Area | Current state | Next migration pressure |
| --- | --- | --- |
| Tokens | Many operator and indentation tokens are covered; numeric literal valid and invalid underscore/prefix forms are better covered; `lex_with_spans()` now exposes token start/end locations; string, f-string, t-string, and tokenizer error cases are still partial. | Continue from `Lib/test/test_tokenize.py`, f-string/t-string suites, and remaining tokenizer error forms. |
| Statements | Core simple and compound statement grammar rows are covered; remaining statement work is mostly runtime-coupled edge behavior and broader integration. | Continue CPython-derived runtime edge tests where syntax depends on VM behavior. |
| Expressions | Arithmetic, boolean-as-int numeric behavior, comparison, displays, comprehensions, calls, lambdas, slices, and user-defined subscript protocol exist as subsets. | Audit illegal expressions, generator parenthesization, walrus restrictions, and full call argument ordering. |
| Parameters | Positional-only, keyword-only, `*args`, and `**kwargs` exist as subsets. | Finish invalid parameter ordering and duplicate-name coverage from CPython. |
| Runtime coupling | VM covers enough builtins/classes/exceptions for syntax tests to execute. | Keep runtime additions scoped to what CPython syntax tests require. |
| Differential parity | A Rust integration test now compares selected supported programs and invalid forms directly against CPython output/rejection behavior with per-case CPython source labels. | Expand `tests/cpython_diff.rs` before and after each migration batch to catch semantic drift. |
| Object identity | Immutable singleton identity, class/instance basics, shared `list`/`dict`/`set` identity, expanded mutable container methods, first-pass dynamic dict views, and CPython-style dict/dict-view iterator invalidation diagnostics now exist for the supported subset. | Move the broader heap-object model, remaining container methods, dict view identity nuances, and subclass/custom-protocol behavior toward CPython before migrating the next aliasing-heavy tests. |
| Grammar inventory maintenance | CPython main currently has 276 grammar rules, and the inventory has matching rows. | Keep the inventory synchronized and guarded by `tests/cpython_inventory.rs`; use the runtime coverage matrix to decide what is still incomplete. |
| CPython test manifest | Syntax-adjacent CPython modules now have group-level method counts from the current local source: `test_grammar.py` has 75 methods, `test_syntax.py` has 55, `test_compile.py` has 186 methods, and `test_ast/test_ast.py` has 216. `test_grammar.py` currently has no module-level `test_*` functions; its executable tests are under `TokenTests` and `GrammarTests`. `TokenTests`, `GrammarTests`, `SyntaxWarningTest`, `SyntaxErrorTestCase`, `LazyImportRestrictionTestCase`, `TestBooleanExpression`, `TestStaticAttributes`, `TestExpressionStackSize`, and `TestStackSizeStability` are now fully ported at method level for the current local CPython source. Python-visible `ast.parse()` / `ast.dump()` now exposes first-pass node fields and AST type checks, first-pass lazy import `is_lazy` fields, first-pass `compile(..., ast.PyCF_ONLY_AST)` returns public AST nodes, first-pass `compile(public_ast, ...)` executes representative public AST trees, including cyclic public-AST `RecursionError` detection and the first `to_tuple()` snippet round-trips, all current `ASTConstructorTests` methods are now covered by direct method-level Rust evidence, the first public AST iteration helpers are covered, first-pass `ast.literal_eval()` values, decimal integer digit-limit diagnostics, syntax-error multiline indentation behavior, and syntax-error context preservation are covered, first-pass location helpers for generated nodes are covered, first-pass parser-generated source locations for common expression/call shapes and multiline docstring expression start positions are covered, first-pass `ast.get_docstring()` is covered, first-pass `ast.get_source_segment()` is covered for supported parsed nodes plus explicit-location multi-line extraction, first-pass function/class definition source spans are covered, first-pass lambda/subscript/display source spans are covered including starred call-argument end positions, first-pass yield/await/comprehension source spans are covered, first-pass suite/control-flow source spans are covered including CPython's explicit `elif` statement start-position checks, first-pass import/import-from source spans are covered including parenthesized multi-line import-from, first-pass f-string replacement-expression source spans are covered, first-pass `ast.dump(indent=...)` formatting is covered, and first-pass incomplete-node / `show_empty` dump behavior is covered; full parser source locations for remaining node families, broader compile-from-AST execution, remaining public-AST dump edge cases, deeper `literal_eval()` edge cases, full `to_tuple()` parity, and most `test_compile.py` code-object/optimization/source-position groups remain open. | Continue with partial `test_ast.py` and `test_compile.py` classes method-by-method. |

Completed in the `test_compile.py` boolean/control-flow pass:

- Added `Lib/test/test_compile.py` to the method-level manifest. The current
  local source has 186 methods across `TestSpecifics`,
  `TestBooleanExpression`, `TestSourcePositions`, `TestStaticAttributes`,
  `TestExpressionStackSize`, `TestStackSizeStability`, and
  `TestInstructionSequence`.
- Ported all 4 current `TestBooleanExpression` methods through
  `cpython_compile_boolean_expression_exact_subset` and the differential
  `boolean-expression-short-circuit-identity` case. This found and fixed a
  MiniPython compiler bug where mixed `and` / `or` expressions could call
  `__bool__` twice on an operand that CPython had already proven true or
  false through short-circuit control flow.
- Ported all 27 current `TestStackSizeStability` methods through
  `cpython_compile_stack_size_stability_control_flow_subset`. MiniPython does
  not expose CPython `co_stacksize`, so these are represented as repeated
  sync/async function compile-shape checks, backed by the existing output and
  differential control-flow tests.
- `TestSpecifics`, `TestSourcePositions`, and `TestInstructionSequence`
  remain open, with the last one blocked on
  CPython-only `_testinternalcapi` instruction-sequence objects.

Completed in the `test_compile.py` expression-stack pass:

- Ported all 17 current
  `Lib/test/test_compile.py::TestExpressionStackSize` methods through
  `cpython_compile_expression_stack_size_shapes_subset`.
- Covered long `and` / `or` / mixed boolean chains, chained comparisons,
  conditional expressions, binary expressions, list/tuple/set/dict displays,
  function and method positional/keyword calls, repeated boolean expressions
  inside a function body, the 3050-target unpack-assignment regression, and the
  3050-argument annotated-signature regression.
- MiniPython does not expose CPython `co_stacksize`, so these tests use the
  same method-level source shapes as register-compiler stability checks rather
  than CPython code-object stack-size assertions.

Completed in the `test_compile.py` static-attributes pass:

- Ported all 4 current
  `Lib/test/test_compile.py::TestStaticAttributes` methods through
  `cpython_compile_static_attributes_exact_subset`.
- The compiler now collects CPython-style class `__static_attributes__` from
  Store targets named exactly `self.<attr>`, sorted and deduplicated, while
  ignoring reads such as `self.f()` / `self.arr[3]` and non-self stores such as
  `obj.self = 8`.
- Nested functions contribute to the nearest enclosing class's tuple; nested
  classes collect their own tuple independently, and subclasses get only their
  own collected attributes.

Started in the `test_compile.py` source-positions direct-method pass:

- Added `cpython_compile_source_positions_multiline_assert_rewrite_subset`,
  migrating CPython
  `Lib/test/test_compile.py::TestSourcePositions::test_multiline_assert_rewritten_as_method_call`.
- This pins the public-AST compile path where a multiline `Assert` node's
  location is copied to a generated method call, `ast.fix_missing_locations()`
  fills child nodes, and `compile(public_ast, ...)` accepts the rewritten tree.
- `TestSourcePositions` remains `partial` because most methods still assert
  CPython code-object `co_positions()` and opcode/debug-range locations, which
  require a broader instruction source-span model in MiniPython.

Started in the `test_compile.py` TestSpecifics newline/indent pass:

- Added `cpython_compile_specifics_newline_and_indentation_subset`, migrating
  CPython `Lib/test/test_compile.py::TestSpecifics::test_no_ending_newline`,
  `test_empty`, `test_other_newlines`, and `test_indentation`.
- This pins `compile(source, filename, "exec")` acceptance for empty string
  source, non-newline-terminated source, CRLF source, lone-CR source, mixed
  newline source containing function definitions, and nested indented blocks.
- `TestSpecifics` remains `partial`; many remaining methods assert CPython
  code-object metadata, optimization behavior, warnings, filename handling,
  constants, and traceback/line-number details outside this input-boundary
  slice.

Expanded in the `test_compile.py` TestSpecifics syntax/import pass:

- Added `cpython_compile_specifics_syntax_error_boundaries_subset`, covering
  CPython `TestSpecifics` methods `test_debug_assignment`,
  `test_argument_handling`, `test_syntax_error`, `test_none_keyword_arg`,
  `test_duplicate_global_local`, `test_argument_order`, and
  `test_float_literals` through the public `compile()`, `eval()`, and `exec()`
  SyntaxError boundary.
- Added `cpython_compile_specifics_none_assignment_subset`, migrating
  `test_none_assignment` across both `single` and `exec` compile modes for
  assignment, augmented assignment, definitions, loop targets, parameters, and
  import aliases named `None`.
- Added `cpython_compile_specifics_import_syntax_subset`, migrating
  `test_import` as a compile-only grammar boundary for ordinary imports,
  aliases, parenthesized from-imports, future imports, and CPython-invalid
  malformed import shapes.
- Added `cpython_compile_specifics_compile_stability_subset`, covering
  `test_sequence_unpacking_error`, `test_annotation_limit`,
  `test_condition_expression_with_dead_blocks_compiles`,
  `test_condition_expression_with_redundant_comparisons_compiles`,
  `test_dead_code_with_except_handler_compiles`, and
  `test_try_except_in_while_with_chained_condition_compiles`.
- Added `cpython_compile_specifics_dict_evaluation_order_subset`, migrating
  `test_dict_evaluation_order` and pinning left-to-right key-before-value dict
  display evaluation.
- Added `cpython_compile_specifics_compile_filename_subset`, starting
  `test_compile_filename` by exposing public code-object `co_filename`,
  accepting string and bytes filenames, and rejecting unsupported bytearray/list
  filenames. The remaining memoryview and path-like filename branches require
  broader memoryview/path-like runtime support.
- `TestSpecifics` remains `partial`; the next useful slice is still the subset
  that avoids direct assertions about deeper CPython code-object internals such
  as constant merging, bytecode shape, line tables, and platform traceback
  metadata.

Completed in the AST snippets public-`to_tuple()` PEP 695 pass:

- Extended CPython `Lib/test/test_ast/test_ast.py::AST_Tests.test_snippets`
  migration with decorated function/async-function/class definitions,
  generator-expression decorator arguments, dotted decorator attributes,
  parenthesized and control-flow named expressions, positional-only parameter
  defaults, type aliases, and generic class/function/type-alias type
  parameters.
- Tightened public AST source-location annotation for parenthesized expression
  statements, call-site generator arguments, interleaved function defaults, and
  PEP 695 `TypeAlias` / `TypeVar` / `TypeVarTuple` / `ParamSpec` nodes so the
  migrated `to_tuple()` snapshot can also round-trip through
  `compile(public_ast, ...)`.

## Migration Batches

| Batch | CPython sources | Rules | MiniPython status | Acceptance |
| --- | --- | --- | --- | --- |
| Invalid assignment targets | `Lib/test/test_syntax.py` top-level invalid target doctests; `Lib/test/test_compile.py::test_argument_handling` | `assignment`, `star_targets`, `single_target`, `del_target`, `invalid_assignment` | Completed | Rust tests reject invalid assignment, augmented assignment, delete, `for`, `with`, and comprehension targets with parse errors, and cover every CPython `assignment` alternative. |
| Parameter syntax errors | `Lib/test/test_syntax.py` parameter block; `Lib/test/test_positional_only_arg.py`; `Lib/test/test_compile.py::test_argument_handling` | `parameters`, `slash_no_default`, `slash_with_default`, `star_etc`, `kwds`, `lambda_parameters` | Completed | Rust tests cover duplicate names, default ordering, `/`, `*`, `*args`, `**kwargs`, `__debug__`, and lambda equivalents. |
| Call argument ordering | `Lib/test/test_syntax.py` argument invalid forms; `Lib/test/test_grammar.py`; `Lib/test/test_call.py` subset | `arguments`, `args`, `kwargs`, `kwarg_or_starred`, `kwarg_or_double_starred` | Completed | Rust tests cover trailing commas, repeated `*` and `**` unpacking, keyword-after-star forms, invalid keyword slots, missing keyword values, unpack assignment forms, positional-after-keyword, iterable unpack after keyword unpack, duplicate keywords, and generator expression parenthesization. |
| Comprehension legality | `Lib/test/test_syntax.py` comprehension target errors; `Lib/test/test_grammar.py::test_comprehension_specials` | `for_if_clause`, `listcomp`, `setcomp`, `dictcomp`, `genexp` | Completed | Rust tests cover missing `in`, invalid targets, invalid unpacking elements, nested clauses, outer iterable binding, comprehension-internal `yield`, await-driven async-comprehension boundaries, starred list/set elements, generator element alternatives, async generator expressions, and dict-unpack comprehensions. |
| Scope declaration errors | `Lib/test/test_syntax.py` global/nonlocal doctests; `Lib/test/test_scope.py`; `Lib/test/test_global.py` | `global_stmt`, `nonlocal_stmt`, `function_def`, `class_def` | Completed | Rust tests reject use-before-global, assign-before-global, missing nonlocal binding, module-level nonlocal, global/nonlocal conflicts, and cover global/nonlocal writes across supported name-binding forms. |
| Match pattern edge cases | `Lib/test/test_syntax.py`; `Lib/test/test_patma.py`; `Lib/test/test_ast/test_ast.py` pattern cases | `match_stmt`, `patterns`, `mapping_pattern`, `class_pattern`, `or_pattern`, `as_pattern` | Completed | Rust tests cover valid match suites, invalid empty suites, inline and indented case bodies, invalid capture placement, duplicate mapping keys/rest, OR-pattern binding consistency, guards, and irrefutable-case ordering. |
| f-string and t-string grammar | `Lib/test/test_fstring.py`; `Lib/test/test_tstring.py`; `Lib/test/test_tokenize.py` | `fstring`, `fstring_replacement_field`, `tstring`, token trio rules | Partial | Rust tests cover nested expressions/specs, conversions, debug syntax, raw prefixes, invalid braces, and tokenizer split behavior. |
| Type parameter grammar | `Lib/test/test_type_params.py`; `Lib/test/test_compile.py` type alias/default coverage | `type_params`, `type_param_seq`, `type_param`, `type_param_default` | Completed | Rust tests cover class/function/async-function/type-alias type params, duplicate names, bounds, constraints, defaults, starred defaults, trailing commas, and invalid variadic bounds. |

## Immediate Next Slice

Continue promoting the highest-risk `missing` rows from
`tests/cpython_grammar_inventory.md` into real coverage rows and Rust tests.
Start with rules that are already partially implemented under a broader parent
rule, because those can move the coverage matrix toward CPython without
inventing large new runtime behavior.

Completed in the AST literal-eval complex pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_literal_eval_complex`
  by covering every accepted signed real-plus-imaginary literal form,
  parenthesized complex literal form, and every rejected non-literal complex
  expression shape from the CPython method.

Completed in the invalid-as-expression diagnostic pass:

- Migrated CPython
  `Lib/test/test_syntax.py::test_except_stmt_invalid_as_expr` by making
  `except ValueError as obj.attr` fail with
  `cannot use except statement with attribute` instead of a generic colon
  parse error, and pinned the diagnostic span to the full dotted target.
- Migrated CPython
  `Lib/test/test_syntax.py::test_match_stmt_invalid_as_expr` by making
  `case x as obj.attr` and starred sequence target attributes fail with
  `cannot use attribute as pattern target` instead of the generic unsupported
  match-pattern diagnostic, and pinned the diagnostic span to the full dotted
  target.

Completed in the conditional-expression invalid-statement pass:

- Extended `cpython_invalid_expression_rules_subset` with CPython
  `Lib/test/test_syntax.py::test_ifexp_else_stmt` statement keywords after a
  conditional-expression `else`, including `return`, `raise`, `del`, `yield`,
  `assert`, `break`, `continue`, `import`, and `from`.
- Tightened the parser so a conditional-expression `else` branch rejects those
  statement starts immediately. This closes the previous gap where
  `x = 1 if True else yield 2` parsed as a yield expression and failed later in
  compilation instead of being rejected as syntax.
- Migrated CPython
  `Lib/test/test_syntax.py::test_ifexp_body_stmt_else_expression` and
  `::test_ifexp_body_stmt_else_stmt` by rejecting `pass`, `break`, and
  `continue` before the `if` in conditional expressions with
  `expected expression before 'if', but statement is given`.

Completed in the static-nesting and complexity-limit pass:

- Migrated CPython-only
  `Lib/test/test_syntax.py::test_nested_named_except_blocks` by adding a
  compiler static-block depth limit and rejecting over-limit nested named
  `except Exception as e` blocks with `too many statically nested blocks`.
- Migrated CPython-only
  `Lib/test/test_syntax.py::test_with_statement_many_context_managers` and
  `::test_async_with_statement_many_context_managers` by counting each
  sync/async context manager as a static block and reserving one static block
  for generator functions, preserving CPython's accepted and rejected
  context-manager boundaries.
- Migrated CPython-only
  `Lib/test/test_syntax.py::test_syntax_error_on_deeply_nested_blocks` by
  applying the same static-block depth limit to nested `while` statements.
- Migrated CPython-only
  `Lib/test/test_syntax.py::test_error_on_parser_stack_overflow` by making
  very deep unary operator chains return `too complex` for exec, eval, and
  interactive entry points without stack overflow. A source-level fast path
  keeps the exact CPython-sized 100000-prefix sample cheap enough for regular
  Rust regression runs.
- Migrated CPython-only
  `Lib/test/test_syntax.py::test_deep_invalid_rule` by keeping the deep invalid
  PEG-rule source rejected without long-running parser backtracking.

Completed in the lazy-import restriction pass:

- Migrated all 9 current CPython
  `Lib/test/test_syntax.py::LazyImportRestrictionTestCase` methods into
  `cpython_lazy_import_syntax_subset`.
- Added `compile_source` so Rust tests can mirror CPython's compile-only valid
  lazy import cases without requiring MiniPython to implement every imported
  standard-library module at runtime.
- Covered lazy import rejection inside `try`, `try` with `except*`, `except*`
  handler bodies, ordinary functions, async functions, classes, nested
  function/class scopes, and module-level star import rejection.
- Preserved CPython's error-priority behavior for `lazy from ... import *`
  inside functions: the function-scope lazy import error is reported before
  the module-level star-import restriction.
- Added `cpython_ast_lazy_import_fields_subset` for the CPython public-AST
  surface of lazy imports: `Import._fields` / `ImportFrom._fields` now include
  `is_lazy`, parsed ordinary imports dump `is_lazy=0`, parsed lazy imports dump
  `is_lazy=1`, AST constructors default to `0`, and compile-from-public-AST
  accepts module-level lazy import nodes.

Completed in the CPython AST snippets parse inventory pass:

- Added `parse_source`, `parse_eval_source`, and `parse_interactive_source` so
  tests can verify grammar acceptance without executing the VM.
- Added `ast_dump_source`, `ast_dump_eval_source`, and
  `ast_dump_interactive_source` as a narrow internal-structure test surface for
  parser ASTs. This is not a CPython-compatible public `ast` module yet, but it
  lets Rust tests move representative snippet cases from parse-only acceptance
  toward structural assertions.
- Migrated the statement, interactive, and expression smoke inventory from
  `Lib/test/test_ast/snippets.py` into
  `cpython_ast_snippets_parse_inventory_subset`, covering CPython's one-sample
  AST surface for supported statements, expressions, type-parameter syntax,
  decorators, match statements, f-strings, and t-strings.
- Added `cpython_ast_snippets_structural_dump_subset`, which pins
  representative internal AST shapes for module, function, class, assignment,
  control-flow, match, call, comprehension, f-string, t-string, and interactive
  snippets.
- Full `Lib/test/test_ast/test_ast.py::test_snippets` parity remains open,
  but `cpython_ast_snippets_public_to_tuple_first_pass_subset` now adds the
  first public-AST `to_tuple()` checks for functions, classes, return/delete
  statements, `for`/`while`/`if`/`with` control flow, `try`/`try*`,
  `raise`/`assert`, ordinary and lazy imports, `global`,
  `pass`/`break`/`continue`, `for` unpacking targets, comprehension source
  spans, async functions/loops/context managers, unpacking displays, and
  `yield` / `yield from`, plus `eval` / `single` mode expression trees,
  including source positions and `compile(ast_tree, ...)` round-trips.
- Fixed public-AST `Delete` source locations so the statement node starts at
  the `del` keyword instead of the first target expression, matching CPython's
  `to_tuple()` position for `del v`.
- Fixed public-AST `With` / `AsyncWith` source locations so statement nodes
  start at the `with` / `async` keyword instead of the first context
  expression, matching CPython's `to_tuple()` positions.
- Fixed public-AST `Global` / `Nonlocal` source locations so declaration
  statement nodes span from the keyword through the final declared name,
  matching CPython's `to_tuple()` positions.
- Fixed public-AST `Starred` source locations so starred display elements span
  from the `*` token through the unpacked expression, matching CPython's
  `to_tuple()` position for `{*{1, 2}, 3}`.
- Added `complex` as a builtin type object for `type(1j)` /
  `isinstance(..., complex_type)` classinfo checks, which the migrated CPython
  `to_tuple()` helper shape needs.

Completed in the differential parity harness pass:

- Migrated CPython `Lib/test/test_tokenize.py::TestTokenize::test_exact_type`
  into `cpython_tokenize_exact_type_subset`, covering exact operator token
  distinctions for punctuation, arithmetic, comparison, augmented assignment,
  walrus, ellipsis, arrows, matrix multiplication, and mixed expression/set
  token streams. MiniPython exposes these as exact lexer token variants instead
  of CPython's `TokenInfo.exact_type` side channel.
- Migrated CPython `Lib/test/test_tokenize.py::TokenizeTest::test_selector`
  and `::test_method` into `cpython_tokenize_selector_and_method_subset`,
  covering dotted/subscript selector spans and decorator/function-header token
  spans under MiniPython's keyword-token parser input model.
- Migrated representative CPython
  `Lib/test/test_tokenize.py::TokenizeTest::test_async` source shapes into
  `cpython_tokenize_async_await_subset`, covering `async` / `await` in
  assignment-shaped tokenizer input, attribute positions, async compound
  headers, `async def`, and comments/newlines while keeping MiniPython's
  parser-ready keyword token variants.
- Migrated CPython `Lib/test/test_fstring.py::test_not_equal`,
  `::test_equal_equal`, and conversion-formatting cases into
  `cpython_f_string_conversion_operator_edge_subset`, covering the lexer/parser
  distinction between `!=`, debug `=`, and conversion `!`, plus conversion
  padding, string precision, `!<` format fill, and invalid conversion
  spellings.
- Migrated CPython `Lib/test/test_fstring.py::test_if_conditional`,
  `::test_empty_format_specifier`, `::test_str_format_differences`,
  `::test_loop`, and `::test_dict` into
  `cpython_f_string_contextual_runtime_subset`, covering f-string truthiness in
  `if`, empty format specs, f-string expression subscripts versus
  `str.format()` field-name subscripts, repeated loop rendering, and nested
  quote handling in dict subscripts.
- Migrated CPython `Lib/test/test_fstring.py::test_errors` into
  `cpython_f_string_format_error_subset`. The VM now reports CPython-style
  `TypeError` for tuple/function objects formatted with unsupported non-empty
  specs and `ValueError` for unknown scalar format codes such as `j`.
- Migrated CPython `Lib/test/test_tstring.py::test_raw_tstrings`,
  `::test_template_concatenation`, and `::test_triple_quoted` into
  `cpython_t_string_raw_concat_and_triple_subset`, covering raw t-string
  literal preservation, Template concatenation, Template/string TypeErrors, and
  triple-quoted t-string literal/interpolation segments. The VM now emits
  CPython-style TypeErrors for `Template + str` and `str + Template` instead of
  the generic unsupported-operand diagnostic.
- Added `tests/cpython_diff.rs` with CPython/MiniPython output parity checks for
  arithmetic, truthiness, bool-as-int arithmetic/bitwise behavior,
  `while`/`for`/`else`, `break`/`continue`, `finally` control-flow override
  behavior, function defaults, `*args`, `**kwargs`, closures, lambdas, list
  comprehensions, conditional-expression precedence, exceptions, classes,
  context managers, starred unpacking, list assignment/slicing, and sequence
  augmented assignment for list concat/repeat, including list alias preservation
  through `+=`, slice augmented assignment,
  and CPython's `testInList` / `testInDict` subscript augmented-assignment
  operator chain.
- Added CPython/MiniPython rejection parity checks for missing compound-statement
  colons/indentation, default-before-non-default parameters, invalid assignment
  targets, invalid `for` targets, invalid augmented-assignment unpacking, and
  invalid list-comprehension forms.
- The expanded harness exposed real VM gaps and these passes closed the smaller
  ones: bool now participates in numeric operations like CPython's `int`
  subclass, `list + list` / `tuple + tuple` concatenation works, list aliasing
  survives `+=` and slice augmented assignment, and `dict`/`set` share identity
  through ordinary assignment. List and dict subscript augmented assignment now
  matches the supported CPython operator chain. The mutable-container method
  pass covers `dict.update`, `dict.copy`, `dict.get`, `dict.pop`,
  `dict.popitem`, `dict.setdefault`, `dict.fromkeys`, PEP 584 `dict |` and
  `dict |=`, `set.add`, `set.update`, `set.copy`, `set.discard`,
  `set.remove`, `set.pop`, `set.clear`, set algebra methods/operators
  (`union`, `intersection`, `difference`, `symmetric_difference`, their update
  forms, and `|`, `&`, `-`, `^` with in-place variants), set ordering
  comparisons (`<`, `<=`, `>`, `>=`) plus CPython's rejection of non-set
  operator operands, first-pass exact `frozenset` construction/hashability and
  common set/frozenset joint operations from `Lib/test/test_set.py`,
  first-pass live dict view objects for `keys`, `values`, and
  `items` display/length/membership/iteration plus key/item set algebra and
  subset/superset comparisons, and the existing list
  `append`/`extend`/`copy`/`pop`/`clear` surface, plus first-pass RuntimeError
  invalidation for dict, keys-view, values-view, and items-view iterators when
  the key set changes during iteration. The iterator diagnostics now distinguish
  size changes from same-size key-set changes. The remaining object-model gap is
  broader heap-object behavior, the full method surface, dict view identity
  nuances, and subclass/custom-protocol behavior.
- Extended the CPython-derived container migration with `test_dict.py`
  `setdefault`/`popitem` and PEP 584 dict union behavior, plus `test_set.py`
  union, intersection, difference, symmetric difference, subset/superset,
  disjointness, and corresponding in-place set operators. These now have both
  direct language tests and CPython/MiniPython differential parity cases.
- Extended `test_dict.py` dict-view coverage so `keys`, `values`, and `items`
  remain live after mutation, support length, iteration, and membership, and
  support set-like operations and subset/superset comparisons for key/item views
  against sets and other views. `dict_values` still correctly rejects set-like
  comparisons.
- Migrated `test_dict.py` iterator mutation coverage for ordinary dict
  iteration plus `values()` and `items()` iteration. Inserting or deleting keys
  during iteration now raises `RuntimeError`, while replacing the value for an
  existing key remains allowed. The supported subset now matches CPython's
  distinct `dictionary changed size during iteration` and
  `dictionary keys changed during iteration` diagnostics.
- Migrated first-pass `reversed()` coverage from CPython's dict, bytes, and
  range tests. MiniPython now supports `reversed()` over lists, tuples, strings,
  bytes, ranges, dictionaries, and dict `keys`/`values`/`items` views, including
  empty dict-view cases, custom `__reversed__`, sequence-protocol fallback via
  `__len__` plus `__getitem__`, size-change invalidation for reverse dict
  iterators, live value reads, and common same-size key-set mutation behavior.
  Repeated same-size reverse-iterator mutations can still diverge from CPython
  because CPython scans dictionary table positions while MiniPython tracks
  original key positions over its compact insertion-order vector.
- Migrated the executable portion of `Lib/test/test_tuple.py::test_constructors`
  for the builtin `tuple` constructor: empty construction, list/string inputs,
  generator inputs, existing tuple identity preservation, and keyword-argument
  rejection. Broader immutable object identity remains future object-model work.
- Migrated first-pass scalar builtin constructor coverage from `test_bool.py`,
  `test_float.py`, and the corresponding integer/string builtin cases. The VM
  now handles empty and single-argument `bool`, `int`, `float`, and `str`
  constructors for the supported scalar/string/bytes subset, plus keyword and
  invalid-literal rejection parity. The `bool` constructor and VM truth tests
  now honor class-level `__bool__` first and `__len__` second, including
  CPython-style rejection for non-bool `__bool__`, negative `__len__`, and
  non-integer `__len__` results. The first numeric conversion protocol slice
  now honors custom `__int__`, `__float__`, and `__index__` for `int()`,
  `float()`, `range()`, `bytes(count)`, `enumerate(..., start)`, and supported
  sequence indexing/slicing, including CPython-style rejection when those
  methods return non-numeric values. The `test_intconversion` slice now also
  covers rejecting objects without numeric conversion methods and using
  `__int__` instead of `__trunc__` when both are present; the newer CPython
  rejection for `__trunc__`-only objects is left until the default oracle matches
  the local CPython source. The `int(value, base)` slice now covers
  positional and keyword base arguments, base supplied through `__index__`,
  binary/octal/hex prefix autodetection for base `0`, byte strings, bases
  2 through 36, explicit-base rejection for non-string inputs, and CPython's
  argument-count diagnostics when positional and keyword base arguments exceed
  the constructor's two-argument limit. The first
  `test_non_numeric_input_types` bytes-like slice now covers `bytearray`
  construction, display, `bytes(bytearray(...))`, length, iteration,
  `isinstance`, `int(bytearray(...))`, `int(bytearray(...), base)`,
  `float(bytearray(...))`, equality with `bytes`, and invalid-literal
  diagnostics. It now also validates string
  underscores in the same accepted positions as CPython's integer parser for
  the covered ASCII subset, including `test_issue31619` underscore-heavy
  strings for bases 2, 8, 16, and 32, and `test_invalid_signs` rejection for
  sign-only and space-separated sign strings. The next `test_underscores`
  constructor-only slice covers explicit-base underscores, byte-string
  underscores, accepted `0_100` constructor input, and `_100` / `+_100` /
  `1__00` / `100_` rejection. It carries the original string/bytes repr in
  invalid-literal diagnostics. The first Unicode
  decimal-digit slice now covers CPython's `test_unicode` behavior for
  Devanagari and Arabic-Indic digits plus other Unicode `Nd` digit blocks
  through a runtime decimal digit table. The next base-conversion slice covers
  CPython's 2-through-36 conversion
  regression for `2**32`, plus base-limit, float-base, keyword-argument, and
  `__index__`-supplied out-of-range base diagnostics. The next error-message
  slices cover CPython's string-float rejection, non-ASCII string diagnostics,
  embedded whitespace, embedded-NUL explicit-base variants, and non-UTF-8 bytes
  invalid-literal diagnostics by preserving or escaping the original repr.
  Broader exact error wording remains future runtime-model work.
- Migrated first-pass `iter()` and `next()` coverage from
  `Lib/test/test_builtin.py` and `Lib/test/test_iter.py`. Ordinary iterators are
  now shared heap values, so `it = iter(seq); next(it); next(it)` advances the
  same iterator object, `iter(it) is it` holds, `next(it, default)` handles
  exhaustion, and non-iterable/non-iterator rejection is covered by CPython
  differential tests. Direct `__iter__()` and `__next__()` calls are now exposed
  for the supported builtin iterables, iterators, and generators, including
  arity/attribute rejection parity for common bad calls. Two-argument sentinel
  `iter(callable, sentinel)` now produces a shared callable iterator that stops
  before yielding the sentinel and supports direct iterator dunders. Rich
  callable equality hooks and broader custom sequence fallback remain future
  work. The callable-sentinel iterator now also treats callable-raised
  `StopIteration` as exhaustion and preserves CPython's reentrant-exhaustion
  behavior from `test_iter_function_concealing_reentrant_exhaustion`: if the
  callable exhausts its own iterator before returning a non-sentinel value, the
  outer `next()` still stops instead of yielding that stale value.
- Migrated first-pass `all()`/`any()` coverage from `Lib/test/test_builtin.py`.
  The VM now exposes both builtins through the same iterator/generator path,
  including empty iterable results, truthy/falsy list cases, generator
  expression cases, short-circuiting before later generator exceptions, and
  custom `__bool__`/`__len__` truth protocol behavior.
- Migrated first-pass `enumerate()` and `zip()` coverage from
  `Lib/test/test_enumerate.py` and `Lib/test/test_builtin.py`. Both builtins now
  return real shared iterator objects and support `next()`, `list()`, generators,
  empty `zip()`, `zip(*[])`, and common argument rejection. Instances with
  `__iter__` returning an object with `__next__` now drive `iter()`, `next()`,
  `for`, `list()`, `enumerate()`, and `zip()`, and instances with `__getitem__`
  fall back to a sequence iterator that stops on `IndexError`. Builtin sequence,
  range, dict, set, reverse, and sequence-fallback iterators now expose direct
  `__length_hint__()` with remaining-count behavior, including `NotImplemented`
  for fallback sequence iterators without `__len__`. The next strict-zip slice
  covers `zip(strict=True)`, strict mismatch `ValueError` cases, invalid keyword
  rejection, and CPython's iterator-consumption side effect when a later
  argument is shorter. Tuple-reuse, subclassable builtin iterator types, and
  richer error wording remain future object-protocol/runtime work.
- Migrated CPython `Lib/test/test_iter.py` sink-state coverage for supported
  iterators. Builtin list/tuple/string/range/generator/enumerate iterators,
  callable-sentinel iterators, and sequence-protocol fallback iterators now
  stay exhausted after their first completion, even if the underlying fallback
  sequence later grows.
- Migrated first-pass `map()` and `filter()` coverage from
  `Lib/test/test_builtin.py` and `Lib/test/test_iter.py`. Both builtins now
  return shared lazy iterator objects, preserve iterator identity through
  `iter(obj) is obj`, operate over generators and user-defined `__getitem__`
  sequence fallback objects, stop at the shortest mapped iterable, support
  `filter(None, iterable)` truth filtering, and reject common CPython bad
  argument shapes. The strict-map slice now covers `map(..., strict=True)`,
  strict mismatch `ValueError` cases, invalid keyword rejection, the same
  iterator-consumption side effects as `zip(strict=True)`, and CPython's
  distinction between propagated custom iterator exceptions and strict-mode
  `StopIteration` converted into `ValueError`. Richer callable protocol hooks
  and complete set/method consumption of VM-only iterators remain future runtime
  work.
- Migrated first-pass attribute-introspection builtin coverage from
  `Lib/test/test_builtin.py`: `getattr`, `setattr`, `delattr`, `hasattr`, and
  `callable`. The next introspection slice adds `vars()` and `dir()` for local
  scopes, modules, classes, ordinary instances, custom `__dir__` results, and
  `__dict__` properties, with CPython/MiniPython differential coverage. It also
  exposes default `object.__dir__` binding for direct calls such as
  `[].__dir__()` and `object.__dir__([])`, and `vars(module)` /
  `module.__dict__` now return a live module namespace mapping for the supported
  string-key subset, including `__name__` mutation. The VM now also exposes
  `globals()` as a live module namespace mapping and `locals()` as the same live
  mapping at module scope plus a function-local snapshot for the supported
  scope model. The VM now supports dynamic attribute access and mutation for
  modules, classes, and instances, default values for missing `getattr`, boolean
  probing with `hasattr`, CPython-style class-level `__call__` lookup for
  callable instances, and catchable `AttributeError` from runtime attribute
  failures. The first-pass instance hook slice now routes
  missing instance attributes through class-level `__getattr__`, instance writes
  through `__setattr__`, deletes through `__delattr__`, and exposes
  `object.__getattribute__`, `object.__setattr__`, and `object.__delattr__` as
  hook-bypass primitives. The next hook slice now dispatches custom
  `__getattribute__` for every instance attribute read, falls back to
  `__getattr__` only for `AttributeError`, and keeps `object.__getattribute__`
  as a no-`__getattr__` bypass. The first descriptor slice now implements the
  builtin `property` data descriptor, including `@property`, `@x.setter`,
  `@x.deleter`, class access, direct `__get__` / `__set__` / `__delete__`, and
  `AttributeError` fallback into `__getattr__`. The next descriptor slice now
  supports user-defined data and non-data descriptors, including
  `__get__(obj, owner)`, class access via `__get__(None, owner)`, assignment via
  `__set__`, deletion via `__delete__`, subclass owner propagation, and the
  CPython precedence rule where data descriptors beat instance fields while
  non-data descriptors can be shadowed. The class binding slice now implements
  `staticmethod` and `classmethod` as non-data descriptors, including decorator
  use, direct `__func__`, direct `__get__`, subclass owner binding, and
  `isinstance(..., staticmethod/classmethod)` checks. Bound method metadata now
  preserves `__func__` identity, `__self__`, `__name__`, `__qualname__`,
  `__module__`, and `__doc__` for the supported descriptor model, including
  functions copied between class namespaces without rewriting their original
  owner qualname. Bound method objects now also expose `__get__` as a descriptor
  that keeps the original receiver and render CPython-style repr text with the
  method qualname plus receiver repr. User-class
  `isinstance(instance, Class)` now follows the instance's direct class and base
  classes instead of treating class objects themselves as instances of their
  bases. The first `super` slice now implements explicit two-argument
  `super(type, obj)` lookup across the class hierarchy, including instance
  method binding, classmethod owner binding, staticmethod passthrough,
  `__thisclass__`, `__self__`, and
  `isinstance(..., super)`. The next `super` slice now supports zero-argument
  `super()` inside methods, classmethods, and property accessors by attaching the
  defining class to class-created functions and exposing the method
  `__class__` cell name. The unbound `super` slice now supports `super(type)`,
  `super(type, None)`, descriptor rebinding through `super.__get__`,
  `__self_class__`, and CPython-compatible rejection of invalid first arguments
  and invalid descriptor receivers. The MRO slice now uses C3 linearization for
  class attribute lookup and `super()` chains, and rejects duplicate direct bases
  and inconsistent multiple-inheritance hierarchies at class creation time. The
  first `__slots__` slice now enforces slot-only instance attributes for classes
  that define string/tuple/list slots without `__dict__`, permits inherited slot
  writes, exposes slot names as class-level member descriptors with direct
  `__get__` / `__set__` / `__delete__`, gives subclasses without slots a
  dynamic instance dictionary, permits dynamic attributes when `__dict__` is
  present or inherited, and rejects common invalid slot declarations and class
  variable conflicts. The bound-method metadata slice now exposes `__func__`,
  `__self__`, `__name__`, `__qualname__`, `__module__`, and `__doc__`, preserves
  Python function identity through method binding, keeps alias methods attached
  to their original defining class, and covers bound-method `__get__`, object
  identity preservation for stored method objects, and stable repr metadata.
  `__dict__` and `__weakref__` descriptor/layout details, the remaining method
  object metadata surface, and exact Argument Clinic wording remain future
  object-model work.
- Migrated first-pass `abs()`, `min()`, and `sum()` coverage from
  `Lib/test/test_builtin.py`. The VM now supports numeric and complex absolute
  values, instance-level `__abs__`, multi-argument and single-iterable `min()`,
  and `sum()` over ordinary iterables, generators, iterator objects, and
  list-concat starts. `min()` and `max()` now also support keyword-only `key`
  and `default` for the supported callable/value model, and the covered
  `abs()` TypeError paths are catchable. Precise decimal/fraction semantics and
  the remaining custom numeric protocol hooks remain future runtime-model work.
- Migrated first-pass `chr()` and `ord()` coverage from
  `Lib/test/test_builtin.py::test_chr` and `::test_ord`. The VM now exposes both
  builtins for ordinary integer code points, one-character strings, and one-byte
  bytes/bytearray inputs. CPython's surrogate-code-point string model remains
  future Unicode-runtime work because MiniPython currently stores strings as
  UTF-8.
- Migrated first-pass `bin()`, `oct()`, and `hex()` coverage from
  `Lib/test/test_builtin.py::test_bin`, `::test_oct`, and `::test_hex`. The VM
  now exposes the three integer-base builtins for small ints, arbitrary-precision
  ints, bools, and objects implementing `__index__`, while preserving
  CPython-style negative sign placement before the base prefix.
- Migrated first-pass `ascii()` coverage from
  `Lib/test/test_builtin.py::test_ascii`. The VM now exposes `ascii()` as a
  builtin, shares the same CPython-style ASCII escaping with f-string `!a`, and
  handles recursive list/dict repr placeholders for the supported container
  model. Lone surrogate strings remain future Unicode-runtime work.
- Migrated first-pass `sorted()` coverage from
  `Lib/test/test_builtin.py::TestSorted`. The VM now exposes `sorted()` for
  supported iterables, keeps the input list unchanged, returns a new list,
  supports keyword-only `key=None`, callable `key`, and integer/bool
  `reverse`, preserves stable equal-key ordering, and rejects the CPython
  bad-argument shapes covered by the differential harness. Full `list.sort()`,
  custom comparison protocol behavior, mutation-during-sort protection, and
  richer Argument Clinic wording remain future runtime-model work.
- Migrated first-pass `list.reverse()` and `list.sort()` coverage from
  `Lib/test/list_tests.py` and `Lib/test/test_sort.py`. Lists now support
  in-place reverse and in-place stable sort, with `sort(key=None)`,
  callable `key`, integer/bool `reverse`, `None` return values, alias
  preservation, and CPython-aligned rejection for common positional, keyword,
  reverse-type, and incomparable-item errors. Mutation-during-sort protection,
  `cmp_to_key` object behavior, and full rich-comparison protocol coverage
  remain future runtime-model work.
- Migrated first-pass `list.insert()`, `list.remove()`, `list.count()`, and
  `list.index()` coverage from `Lib/test/list_tests.py` and
  `Lib/test/seq_tests.py`. Lists now support insertion with negative and
  out-of-range index clamping, first-match removal, counting via the current
  value equality model, `index(value[, start[, stop]])` with slice-style
  bounds, and CPython-aligned rejection for common missing, excessive, absent,
  and out-of-window argument cases. Custom `__eq__` dispatch, comparison-error
  propagation, and list mutation during comparison remain future object-model
  work.
- Migrated first-pass list special-method coverage from
  `Lib/test/seq_tests.py::CommonTest::test_subscript`. Lists now expose
  `__getitem__`, `__setitem__`, `__delitem__`, `__contains__`, and `__len__`
  as bound methods over the existing subscript, slice, membership, and length
  logic, including common direct-call arity and invalid-subscript rejection.
  Exact Argument Clinic wording and custom index protocol hooks remain future
  runtime-model work.
- Kept the harness on Python 3.9-compatible syntax by default; newer grammar
  migration batches should set `MINIPYTHON_CPYTHON` to a local CPython build
  before using CPython as the oracle.

Completed in the first promotion pass:

- `single_compound_stmt`, `statement_newline`, `simple_stmts`, `annotated_rhs`,
  and `augassign`.

Expanded in the parameter-helper promotion pass:

- `star_etc`, `kwds`, `param_no_default_star_annotation`, `star_annotation`,
  `lambda_params`, `lambda_parameters`, `lambda_star_etc`, `lambda_kwds`,
  `lambda_param_no_default`, `lambda_param_with_default`,
  `lambda_param_maybe_default`, and `lambda_param`.
- Broader function-parameter helpers such as `params`, `parameters`,
  `param_no_default`, `param_with_default`, `param_maybe_default`, `param`,
  `param_star_annotation`, and `default` remain partial until their remaining
  CPython type-comment/default-expression combinations are audited.

Completed in the call argument helper promotion pass:

- `arguments`, `args`, `kwargs`, `starred_expression`, `kwarg_or_starred`,
  `kwarg_or_double_starred`, `invalid_arguments`, and `invalid_kwarg`, plus
  executable CPython-derived coverage for trailing commas, repeated `*` and
  `**` unpacking, keyword-after-star forms, duplicate keywords, invalid keyword
  targets, missing keyword values, positional-after-keyword, iterable unpack
  after keyword unpack, and generator expression parenthesization.

Completed in the control-flow helper promotion pass:

- `else_block` and `finally_block`, plus CPython-derived regression tests for
  `break`, `continue`, and `return` interactions with `try/finally`, including
  the issue #37830 cases where `break` or `continue` in a `finally` block
  overrides a pending `return`.
- `break_stmt` and `continue_stmt` now also include CPython-derived checks for
  `while False` bodies, continue-through-`try` behavior, continue-through-finally
  behavior, and the old continue-then-break loop regression shape from
  `Lib/test/test_grammar.py::test_break_continue_loop`.
- Added executable coverage adapted from
  `Lib/test/test_compile.py::test_for_break_continue_inside_except_block` and
  `test_for_break_continue_inside_with_block`, checking CPython-equivalent
  output for break/continue from inside `except` and `with` blocks, including
  `with.__exit__` running before loop jumps.
- Extended that `Lib/test/test_compile.py` slice to cover
  `test_for_break_continue_inside_try_finally_block`,
  `test_for_break_continue_inside_finally_block`,
  `test_for_break_continue_inside_async_with_block`, and
  `test_return_inside_async_with_block`, so loop jumps and returns now have
  CPython-parity output checks across `try/finally`, `finally`, and
  `async with` cleanup paths.
- Extended the `with` control-flow slice with CPython
  `Lib/test/test_with.py::AssignmentTargetTestCase` behavior: after `__enter__`
  succeeds, exceptions raised while binding the `as` target now call `__exit__`,
  and `__exit__` may either suppress the binding error or let an outer
  `except ValueError` catch it. The same protected-target window is covered for
  `async with`, where target-binding failures await `__aexit__`.
- Added CPython `Lib/test/test_with.py::NestedWith` multi-manager cleanup
  coverage: if a later manager's expression / `__init__`, `__enter__`, or
  `__exit__` raises, the earlier manager's `__exit__` observes that exception,
  and may suppress the later `__exit__` failure.
- Added CPython `Lib/test/test_with.py` runtime coverage for `__exit__` result
  truthiness, complex `with ... as` sequence targets, and `yield` inside a
  `with` block. MiniPython now catches exceptions raised by `__bool__` /
  `__len__` during truthiness through the normal exception machinery instead of
  treating the interrupted call as a returned `None`.
- Updated integer `//` and `%` zero-division messages to CPython's
  `integer division or modulo by zero` wording so the migrated truthiness case
  can compare exact output.
- Added CPython `Lib/test/test_with.py::FailureTestCase` context-manager
  protocol coverage for missing `__enter__`, `__exit__`, `__aenter__`, and
  `__aexit__`, including the newer sync/async manager mixup hints. The compiler
  now emits dedicated register-VM context-manager method-load instructions so
  ordinary attribute access remains an `AttributeError`, while `with` /
  `async with` setup reports CPython-style protocol `TypeError`s.

Completed in the control-flow compile edge pass:

- Added `cpython_compile_control_flow_edge_subset`, adapted from
  `Lib/test/test_compile.py::test_dead_code_with_except_handler_compiles` and
  `::test_try_except_in_while_with_chained_condition_compiles`, so dead compound
  branches and while conditions using chained comparisons compile without
  executing unreachable manager/raise paths.
- Extended `cpython_invalid_control_flow_context_subset` with CPython
  `Lib/test/test_syntax.py::test_break_outside_loop` and
  `::test_continue_outside_loop` cases for `if`, class-body, `else`, and `with`
  contexts, including CPython-style `continue` wording and diagnostic spans.
- Migrated CPython `Lib/test/test_syntax.py::test_unexpected_indent`,
  `::test_no_indent`, and `::test_bad_outdent` into the syntax-error message
  parity suite, with parser/lexer wording updated to CPython-style indentation
  diagnostics.
- Migrated CPython `Lib/test/test_syntax.py::test_kwargs_last`,
  `::test_kwargs_last2`, and `::test_kwargs_last3` into the syntax-error
  message parity suite and tightened the parser to distinguish positional
  arguments after ordinary keyword arguments from positional arguments after
  keyword unpacking.
- Migrated CPython
  `Lib/test/test_syntax.py::test_generator_in_function_call` into the
  syntax-error message parity suite, including the method's diagnostic span for
  the unparenthesized generator expression inside a function call.
- Migrated CPython `Lib/test/test_syntax.py::test_except_then_except_star` and
  `::test_except_star_then_except` into the syntax-error message parity suite,
  with parser diagnostics pinned to the mixed `except*` / `except` header.
- Migrated CPython `Lib/test/test_syntax.py::test_empty_line_after_linecont`
  into the explicit line-joining subset, covering both the empty-line and
  split-indented-function source shapes. The executable CPython differential
  suite includes the empty-line shape; the split-indented-function shape is kept
  subset-only because local Python 3.9 rejects it while the checked-out CPython
  source expects it to compile.
- Migrated CPython
  `Lib/test/test_syntax.py::test_continuation_bad_indentation` into the
  explicit line-joining subset. The system Python differential is skipped for
  this one because the local Python 3.9 accepts the shape while the checked-out
  CPython source expects `IndentationError`.
- Migrated CPython
  `Lib/test/test_syntax.py::test_disallowed_type_param_names` into a parser
  subset because local Python 3.9 does not support PEP 695 syntax. MiniPython
  now rejects `__classdict__` as a reserved type-parameter name while still
  accepting the compatibility names covered by CPython.
- Migrated CPython
  `Lib/test/test_syntax.py::test_barry_as_flufl_with_syntax_errors` into the
  syntax-error message parity suite, using a legacy-system-Python fallback for
  the older `invalid syntax` wording while MiniPython targets the current
  `expected ':'` message.
- Promoted the already-implemented invalid line-continuation lexer coverage to
  method-level CPython manifest entries for
  `test_invalid_line_continuation_error_position` and
  `test_invalid_line_continuation_left_recursive`.

Completed in the finally control-flow warning pass:

- Added `cpython_finally_control_flow_warning_subset`, adapted from CPython
  `Lib/test/test_syntax.py::test_return_in_finally` and
  `::test_break_and_continue_in_finally`.
- Promoted `Lib/test/test_syntax.py::SyntaxWarningTest` to method-level
  `ported` status by covering all current CPython return/break/continue
  warning source shapes, including direct, nested-`try`, and nested-`except`
  control-flow inside `finally`.
- Added a lightweight AST static-warning pass behind `source_warnings()`,
  `source_warning_diagnostics()`, and `run_source_with_warnings_as_errors()` so
  `return`, `break`, and `continue` that leave a `finally` block now expose
  CPython-style `SyntaxWarning` messages while nested function/class bodies are
  not reported as part of the enclosing finalizer.

Completed in the block-helper promotion pass:

- `block` and `invalid_block`, plus CPython-derived regression tests for
  indented suites, inline simple-statement bodies after `:`, semicolon-separated
  inline bodies, and missing-indentation errors.

Completed in the raw-definition invalid/type-comment promotion pass:

- `func_type_comment`, `invalid_double_type_comments`, `invalid_def_raw`, and
  `invalid_class_def_raw`, plus CPython-derived tests for inline and own-line
  function type comments, duplicate function type comments, invalid function
  headers, invalid async function headers, invalid class headers, and missing
  raw-definition suites.

Completed in the import-helper promotion pass:

- `import_name`, `import_from`, `import_from_targets`,
  `import_from_as_names`, `import_from_as_name`, `dotted_as_names`,
  `dotted_as_name`, and `dotted_name`.

Completed in the display/selector-helper promotion pass:

- `list`, `tuple`, `set`, `dict`, `kvpair`, `slices`, and `slice`,
  including first-class slice values, multi-item subscript slices, sequence
  slicing via `Value::Slice`, and slice arguments in generic aliases.

Completed in the assignment-target helper promotion pass:

- `star_targets`, `star_targets_list_seq`, `star_targets_tuple_seq`,
  `star_target`, `target_with_star_atom`, `star_atom`, `single_target`,
  `single_subscript_attribute_target`, `t_primary`, and `t_lookahead`, plus
  executable CPython-derived coverage for empty tuple targets, list targets,
  parenthesized targets, starred targets, attribute targets, subscript targets,
  slice-valued subscript targets, multi-item slice targets, chained target
  primaries, call-result attribute targets, call-result subscript targets, and
  generator-expression call-result targets.

Completed in the delete-target helper promotion pass:

- `del_targets` and `del_t_atom`, plus executable CPython-derived coverage for
  empty delete targets, parenthesized delete targets, list delete targets,
  attribute delete targets, subscript delete targets, slice delete targets, and
  chained target primaries.

Completed in the string/t-string helper promotion pass:

- `string`, `tstring_middle`, `tstring_replacement_field`,
  `tstring_full_format_spec`, `tstring_format_spec`, and
  `tstring_format_spec_replacement_field`, plus executable CPython-derived
  coverage for adjacent plain strings, raw strings, t-string literal middle
  parts, replacement fields, conversions, debug fields, and nested replacement
  fields inside format specs.

Completed in the subscript protocol promotion pass:

- `Lib/test/test_compile.py::test_subscripts` semantics for user-defined
  `__getitem__`, `__setitem__`, `__delitem__`, and `__contains__`, including
  index, tuple-index, slice, extended-slice, ellipsis, assignment, augmented
  assignment, deletion, and membership checks.

Expanded in the typing-helper coverage pass:

- `function_def_raw`, `type_expressions`, `func_type_comment`, and
  `invalid_double_type_comments` coverage, plus executable CPython-derived
  tests for FunctionType `*`/`**` argument markers, rejected FunctionType
  trailing commas, inline function type comments, own-line function type
  comments, async function type comments, and duplicate function type comments.

Completed in the invalid t-string promotion pass:

- `invalid_tstring_replacement_field`, `invalid_tstring_conversion_character`,
  and `invalid_string_tstring_concat`, plus executable CPython-derived coverage
  for empty t-string fields, invalid expression starts, invalid post-expression
  tokens, missing expressions before replacement-field punctuation, bad
  debug-field continuations, invalid debug conversions, missing/unsupported
  conversions, unterminated format specs, and mixed adjacent t-string literals
  with plain, unicode-prefixed, raw, f-string, raw f-string, bytes, and raw bytes
  literals in either order.

Completed in the invalid f-string promotion pass:

- `invalid_fstring_replacement_field` and
  `invalid_fstring_conversion_character`, plus executable CPython-derived
  coverage for empty f-string fields, invalid expression starts, invalid
  post-expression tokens, missing expressions before replacement-field
  punctuation, bad debug-field continuations, invalid debug conversions,
  missing/unsupported conversions, and unterminated format specs.

Completed in the invalid named-expression promotion pass:

- `invalid_named_expression`, plus executable CPython-derived coverage for
  invalid walrus targets and accidental `=` after name, literal, operator,
  function-call, subscript, and attribute expressions in named-expression
  contexts.

Expanded in the class raw-header coverage pass:

- `class_def_raw` and `invalid_class_def_raw` coverage, plus executable
  CPython-derived tests for raw class headers with names, empty argument lists,
  type parameters, bases, keyword/unpacked header arguments, missing colons,
  and missing indented class suites.

Completed in the invalid function raw-header promotion pass:

- `invalid_def_raw`, plus executable CPython-derived coverage for function and
  async-function headers that are missing `(`, missing `:`, or missing an
  indented suite after the header.

Completed in the invalid arithmetic/factor promotion pass:

- `invalid_arithmetic` and `invalid_factor`, plus executable CPython-derived
  coverage for rejected unparenthesized `not` after binary arithmetic
  operators and unary `+`, `-`, and `~`.

Completed in the invalid type-params promotion pass:

- `invalid_type_params`, plus executable CPython-derived coverage for empty
  type parameter lists on functions, classes, and type aliases.

Completed in the invalid single type-param promotion pass:

- `invalid_type_param`, plus executable CPython-derived coverage for rejected
  bounds and constraints on TypeVarTuple and ParamSpec parameters.

Completed in the type-parameter grammar promotion pass:

- `type_params`, `type_param_seq`, `type_param`, `type_param_bound`,
  `type_param_default`, and `type_param_starred_default`, plus executable
  CPython-derived coverage for function, async-function, class, and type-alias
  type parameter lists, trailing commas, TypeVar tuple constraints, generic
  alias defaults, and starred TypeVarTuple defaults.

Completed in the invalid dict display promotion pass:

- `invalid_double_starred_kvpairs`, `invalid_kvpair_unpacking`, and
  `invalid_kvpair`, plus executable CPython-derived coverage for missing
  dictionary key colons, missing dictionary values, invalid starred or
  double-starred dictionary keys/values, and unparenthesized conditional dict
  unpacking.

Completed in the invalid starred expression promotion pass:

- `invalid_starred_expression_unpacking`,
  `invalid_starred_expression_unpacking_sequence`, and
  `invalid_starred_expression`, plus executable CPython-derived coverage for
  unparenthesized conditional starred display expressions, dict unpacking in
  starred expression sequences, empty starred expressions, and starred/keyword
  call unpack assignment.

Completed in the invalid named-expression promotion pass:

- `invalid_named_expression`, plus executable CPython-derived coverage for
  invalid walrus targets such as literals, constants, operators, attributes,
  subscripts, and tuples, `__debug__` as a named-expression target, and
  accidental `=` in named-expression contexts.

Completed in the named-expression subscript-target alignment pass:

- Expanded CPython-derived named-expression coverage for subscript targets:
  `a[b:=0]` is valid in load, assignment, delete, and augmented-assignment
  contexts; parenthesized named expressions are valid in slice bounds; and
  unparenthesized named expressions before a slice colon remain syntax errors.

Completed in the comprehension named-expression alignment pass:

- Expanded CPython-derived coverage for assignment expressions inside
  comprehensions. The supported subset now allows walrus bindings in list
  comprehension elements and filters when they do not collide with iteration
  variables, rejects walrus expressions in comprehension iterable expressions,
  rejects rebinding of comprehension iteration variables, and rejects inner
  `for` targets that reuse a name previously bound by a comprehension filter,
  including names referenced through attribute and subscript target
  expressions. It also rejects walrus expressions directly inside
  comprehension target expressions, keeps left-to-right target diagnostics
  aligned with CPython, and rejects walrus expressions inside comprehensions
  directly in class bodies while preserving ordinary class-body walrus
  expressions, comprehensions inside class methods or lambdas, and CPython's
  lambda boundary where lambda bodies are their own scope but lambda defaults
  still belong to the containing comprehension expression.

Expanded in the invalid assignment promotion pass:

- `invalid_ann_assign_target`, plus executable CPython-derived coverage for
  tuple/list annotated-assignment targets and parenthesized invalid annotation
  targets. `invalid_assignment` also covers CPython's yield-assignment error
  branch, but remains partial until the remaining broader invalid assignment
  alternatives are audited.

Completed in the invalid simple-statement promotion pass:

- `invalid_raise_stmt`, `invalid_del_stmt`, and `invalid_assert_stmt`, plus
  executable CPython-derived coverage for missing `raise` expressions, invalid
  `del` targets including nested invalid delete targets, accidental assignment
  in `assert`, and unparenthesized named expressions in `assert`.

Completed in the invalid group promotion pass:

- `invalid_group`, plus executable CPython-derived coverage for parenthesized
  `*` and `**` group errors.

Completed in the invalid block/comprehension promotion pass:

- `invalid_block` and `invalid_comprehension`, plus executable
  CPython-derived coverage for compound statements with missing indented
  blocks, dict unpacking in list/generator comprehensions, and unparenthesized
  tuple targets in list/set comprehensions.

Completed in the comprehension helper promotion pass:

- `for_if_clauses`, `for_if_clause`, and `invalid_for_if_clause`, plus
  executable CPython-derived coverage for synchronous and asynchronous
  comprehension clauses, multiple clauses, multiple `if` filters, missing
  top-level `in`, invalid targets, and async-comprehension rejection outside
  async functions.

Completed in the invalid function-parameter promotion pass:

- `invalid_parameters`, `invalid_default`, `invalid_star_etc`,
  `invalid_kwds`, and `invalid_parameters_helper`, plus executable
  CPython-derived coverage for positional-only marker ordering, missing
  default expressions, parenthesized parameters, default ordering before and
  after `/`, bare/repeated `*`, bare-star type comments, `*, **kwargs`,
  vararg/kwarg defaults, and parameters or `*`/`**`/`/` markers after
  `**kwargs`. The positional-only runtime slice now also verifies that
  keyword use of a positional-only parameter is raised as a catchable
  `TypeError`.

Completed in the invalid lambda-parameter promotion pass:

- `invalid_lambda_parameters`, `invalid_lambda_parameters_helper`,
  `invalid_lambda_star_etc`, and `invalid_lambda_kwds`, plus executable
  CPython-derived coverage for lambda positional-only marker ordering,
  default ordering before and after `/`, parenthesized parameters, missing
  default expressions, bare/repeated `*`, `*, **kwargs`, vararg/kwarg defaults,
  and parameters or `*`/`**`/`/` markers after `**kwargs`.

Completed in the match-pattern helper promotion pass:

- `subject_expr`, `case_block`, `guard`, `patterns`, `pattern`, `as_pattern`,
  `or_pattern`, literal/value/group/sequence/mapping/class pattern helpers, and
  mapping/class subrules, plus focused executable CPython-derived coverage in
  `cpython_match_pattern_helper_rules_subset`.

Completed in the invalid match-pattern promotion pass:

- `invalid_match_stmt`, `invalid_case_block`, `invalid_as_pattern`,
  `invalid_class_pattern`, `invalid_mapping_pattern`, and
  `invalid_class_argument_pattern`, plus executable CPython-derived coverage
  for missing match/case delimiters and indentation, invalid as-pattern targets,
  class positional patterns after keyword patterns, and misplaced mapping rest
  patterns.

Completed in the comparison-helper promotion pass:

- `compare_op_bitwise_or_pair`, `eq_bitwise_or`, `noteq_bitwise_or`,
  `lte_bitwise_or`, `lt_bitwise_or`, `gte_bitwise_or`, `gt_bitwise_or`,
  `notin_bitwise_or`, `in_bitwise_or`, `isnot_bitwise_or`, and
  `is_bitwise_or`, plus executable CPython-derived coverage for comparison
  operators consuming right-hand `bitwise_or` expressions and mixed chained
  comparison short-circuiting.

Completed in the invalid call-argument promotion pass:

- `invalid_arguments` and `invalid_kwarg`, plus executable CPython-derived
  coverage for keyword-unpack followed by iterable-unpack, unparenthesized
  generator expressions in multi-argument calls, missing keyword values,
  positional-after-keyword calls, invalid keyword targets, and `**kwargs=...`
  forms.

Completed in the expression-invalid promotion pass:

- `expression_without_invalid`, `invalid_legacy_expression`,
  `invalid_expression`, and `invalid_if_expression`, plus executable
  CPython-derived coverage for conditional/disjunction/lambda expression
  alternatives, legacy print-statement syntax, missing commas, incomplete
  conditional expressions, statement-in-expression positions, and starred
  conditional else branches.

Completed in the final missing-rule promotion pass:

- `star_annotation` and `yield_expr`, plus executable CPython-derived coverage
  for starred parameter annotations before defaults, bare yield, yield with
  `star_expressions`, `yield from expression`, and the CPython boundary where
  `yield` is allowed in a comprehension's outermost iterable but rejected
  inside the comprehension body, filters, targets, and inner iterables.

Completed in the async comprehension await-boundary pass:

- Expanded `cpython_grammar_async_await_subset`,
  `cpython_for_if_clause_helper_rules_subset`, and
  `cpython_invalid_comprehension_subset` with CPython-derived coverage for
  awaited list/set/dict comprehension elements, filters, targets, and iterable
  positions inside async functions.
- Covered CPython's outside-async distinction where await in comprehension
  result/filter/target/inner iterable positions is rejected as an asynchronous
  comprehension, while await in the outermost iterable still reports the plain
  outside-async await error. Lambda defaults are treated as part of the
  containing comprehension expression, while lambda bodies keep their own
  outside-async await error.

Completed in the async generator expression pass:

- Expanded `cpython_grammar_generator_expression_subset` with CPython-derived
  async-generator coverage for generator expressions whose body/filter uses
  `await`, generator expressions whose clause uses `async for`, and ordinary
  generator expressions whose outermost iterable is awaited in an async
  function.
- Added executable async-generator protocol coverage for `async def` functions
  containing `yield`, `async for` over async generators, and awaited
  `__anext__` calls including `StopAsyncIteration` exhaustion.
- Added CPython-derived async-generator protocol coverage for `anext`,
  `anext(default)`, `asend`, `athrow`, and `aclose`, including the just-started
  non-`None` `asend` `TypeError` and generic async-iterator default fallback.
- Added the CPython compile-time boundary that rejects `return value` inside
  async generators, including unreachable-yield async generator bodies.
- Kept the CPython boundary where `await` in a generator expression's
  outermost iterable is evaluated in the containing scope, so it still reports
  the ordinary outside-async await error outside `async def`.

Completed in the async-for protocol pass:

- Migrated CPython `Lib/test/test_coroutines.py::test_for_2`,
  `::test_for_3`, `::test_for_4`, and `::test_for_11` behavior for async
  iteration protocol failures. MiniPython now reports catchable CPython-style
  `TypeError`s when an `async for` iterable lacks `__aiter__`, when `__aiter__`
  returns an object without `__anext__`, when `__anext__` returns a
  non-awaitable object, and when a returned object's `__await__` raises while
  being converted into the awaited value. The original `__await__` exception is
  preserved as `__cause__`.
- Migrated CPython `Lib/test/test_coroutines.py::test_for_6`, `::test_for_7`,
  and `::test_for_8` observable control-flow behavior. MiniPython now has
  parity coverage for `async with` surrounding `async for`, async-for `else`
  execution inside an async context manager, and `__aiter__` exceptions
  propagating before loop-body or post-loop statements run.
- Migrated CPython `Lib/test/test_coroutines.py::test_for_assign_raising_stop_async_iteration`
  and `::test_for_assign_raising_stop_async_iteration_2` observable behavior for
  `async for` target assignment. `StopAsyncIteration` raised by a subscript
  target's `__setitem__` or by tuple-target unpacking now propagates to the
  surrounding `except StopAsyncIteration` instead of being mistaken for async
  iterator exhaustion.

Completed in the async-with awaitable protocol pass:

- Migrated CPython `Lib/test/test_coroutines.py::test_with_6`,
  `::test_with_7`, `::test_with_8`, `::test_with_9`, `::test_with_10`,
  `::test_with_11`, `::test_with_12`, and `::test_with_13` behavior for
  asynchronous context manager protocol failures and exit propagation.
  MiniPython now reports catchable CPython-style `TypeError`s when
  `__aenter__` or `__aexit__` returns a non-awaitable object, preserves
  `__context__` for failing `__aexit__` paths, propagates `__aenter__`
  failures without calling `__aexit__`, and preserves truthy `__aexit__`
  suppression.
- Added `NotImplementedError` to the builtin exception surface because the
  migrated CPython `async with` enter-failure case raises and catches it.

Completed in the first supported-promotion pass:

- Promoted `yield_stmt`, `yield_expr`, and `star_annotation` from `partial` to
  `supported` after verifying each CPython grammar alternative has executable
  coverage, including `yield`, `yield star_expressions`, `yield from
  expression`, comprehension outer-iterable `yield`, comprehension-internal
  `yield` rejection, and starred parameter annotations before defaults.

Completed in the comparison supported-promotion pass:

- Promoted `comparison`, `compare_op_bitwise_or_pair`, `eq_bitwise_or`,
  `noteq_bitwise_or`, `lte_bitwise_or`, `lt_bitwise_or`, `gte_bitwise_or`,
  `gt_bitwise_or`, `notin_bitwise_or`, `in_bitwise_or`,
  `isnot_bitwise_or`, and `is_bitwise_or` from `partial` to `supported`
  after verifying the plain `bitwise_or` alternative, every comparison
  operator helper, and chained-comparison parsing/execution are covered.

Completed in the return-statement supported-promotion pass:

- Promoted `return_stmt` from `partial` to `supported` after adding
  executable coverage for `return star_expressions`, including tuple return
  unpacking and rejected bare starred return values.

Completed in the raise-statement supported-promotion pass:

- Promoted `raise_stmt` from `partial` to `supported` after adding executable
  coverage for bare re-raise without an active exception and bare re-raise from
  inside an active `except` handler.
- Extended `raise_stmt` / `try_stmt` runtime coverage for custom exception
  classes deriving from `Exception`, including subclass exceptions caught by a
  custom base handler. Runtime exception values now preserve a compact type
  hierarchy so `except BaseError` and `isinstance(error, BaseError)` work for a
  raised `ChildError(BaseError)`. Custom exception values now also retain their
  originating class object, so `error.__class__ is ChildError` and
  `error.__class__.__bases__[0] is BaseError` match CPython for this subset.
- Added a first builtin exception hierarchy slice from
  `Lib/test/test_exceptions.py`: `ArithmeticError` catches `OverflowError` and
  `ZeroDivisionError`, `LookupError` catches `IndexError` and `KeyError`,
  builtin exception `__bases__` exposes the direct base tuple, and
  `GeneratorExit` remains catchable by `BaseException` but not `Exception`.
- Migrated the first `BaseException` attribute slice from
  `Lib/test/test_exceptions.py::testAttributes`: builtin exceptions now preserve
  their original `args` tuple, `str(error)` follows CPython's zero-, one-, and
  multi-argument display behavior, and `repr(error)` renders constructor-like
  exception calls.
- Migrated the first `BaseException.with_traceback` slice from
  `Lib/test/test_exceptions.py::testWithTraceback` and
  `::testInvalidTraceback`: MiniPython now exposes `__traceback__` as `None`,
  accepts `with_traceback(None)`, and raises a catchable `TypeError` for
  non-`None` traceback values. Real traceback objects and CPython's object
  identity guarantee for `with_traceback` remain tied to the broader heap
  exception/object model.
- Migrated the `SystemExit` / `OSError` attribute slice from
  `Lib/test/test_exceptions.py::testAttributes`: builtin exception values now
  preserve `SystemExit.code`; `OSError` exposes `errno`, `strerror`,
  `filename`, and `filename2`; and three-or-more-argument `OSError`
  construction normalizes `.args` to the CPython two-item pair. `OSError`
  display now follows CPython's `[Errno ...] ...` formatting for the supported
  no-filename, one-filename, and two-filename cases.
- Migrated the `SyntaxError` attribute slice from
  `Lib/test/test_exceptions.py::testAttributes`: builtin `SyntaxError` values
  now expose `msg`, `filename`, `lineno`, `offset`, `text`, `end_lineno`,
  `end_offset`, and `print_file_and_line` for empty, message-only, location
  tuple, and ordinary multi-argument constructor forms. This covers the
  constructor-object attributes, not real parser traceback rendering.
- Migrated the Unicode exception attribute slice from
  `Lib/test/test_exceptions.py::testAttributes`: `UnicodeEncodeError`,
  `UnicodeDecodeError`, and `UnicodeTranslateError` now expose their supported
  `encoding`, `object`, `start`, `end`, and `reason` construction attributes.
  `UnicodeDecodeError` follows CPython by storing a `bytes` object attribute
  when constructed from `bytearray`, while preserving the original `.args`.
- Migrated the `AttributeError` keyword-attribute slice from
  `Lib/test/test_exceptions.py::testAttributes`: builtin `AttributeError`
  accepts the CPython-source `name=` and `obj=` keyword-only constructor
  attributes while preserving positional `.args`, and still rejects unexpected
  keywords. The default differential harness does not include this case because
  the local `python3` oracle predates this CPython behavior.

Completed in the match numeric-literal supported-promotion pass:

- Added `cpython_match_numeric_literal_helper_rules_subset` and promoted
  `complex_number`, `signed_number`, `signed_real_number`, `real_number`, and
  `imaginary_number` from `partial` to `supported` after covering positive and
  negative numeric literals, positive and negative real complex parts,
  imaginary literals, complex plus/minus patterns, and complex mapping keys.

Completed in the match literal/closed-pattern supported-promotion pass:

- Expanded `cpython_grammar_match_stmt_subset` and promoted
  `closed_pattern`, `literal_pattern`, and `literal_expr` from `partial` to
  `supported` after covering every CPython closed-pattern alternative,
  adjacent string literal patterns, literal mapping keys, singleton mapping
  keys, complex mapping keys, and CPython's compile-time rejection for
  f-string/t-string match values and f-string mapping keys.

Completed in the match capture/wildcard/group supported-promotion pass:

- Added `cpython_match_capture_wildcard_group_helper_rules_subset` and promoted
  `capture_pattern`, `wildcard_pattern`, and `group_pattern` from `partial` to
  `supported` after covering bare name capture, `_` wildcard behavior without
  rebinding, grouped wildcard, grouped capture, and grouped sequence patterns.

Completed in the scope-declaration supported-promotion pass:

- Added `cpython_scope_declaration_error_subset` and promoted `global_stmt`
  and `nonlocal_stmt` from `partial` to `supported` after covering comma
  separated declaration lists, module/function/class-body declarations,
  use-before-declaration errors, assign-before-declaration errors, parameter
  conflicts, missing enclosing nonlocal bindings, and global/nonlocal conflicts.
- Added `cpython_global_binding_targets_subset` and
  `cpython_nonlocal_binding_targets_subset`, adapted from
  `Lib/test/test_global.py`, to cover global/nonlocal writes through supported
  binding forms: unpacking assignment, assignment expressions, for targets,
  function/class/type-alias definitions, exception handlers, with-items,
  imports, and match capture patterns.
- Extended register bytecode exception-handler metadata with the handler-name
  binding kind so `except ... as name` respects `global` and `nonlocal` instead
  of always writing a local handler variable.

Completed in the memo-rule inventory and match target/star pass:

- Fixed `tests/cpython_grammar_inventory.md` to include CPython grammar rules
  declared with `(memo)`, including the no-space `import_stmt[stmt_ty](memo)`
  form, raising the tracked grammar total from 257 to 276 and eliminating an
  inventory blind spot for rules such as `simple_stmt`, `import_stmt`, `block`,
  `expression`, `star_pattern`, `arguments`, and target/delete helper rules.
- Added `cpython_match_capture_target_and_star_pattern_helper_rules_subset` and
  promoted `pattern_capture_target` and `star_pattern` to `supported` after
  covering capture targets in `as`, mapping-rest, class-keyword, and sequence
  star contexts, plus `_`, `.`, `(`, and `=` exclusion behavior.

Completed in the for-statement supported-promotion pass:

- Expanded `cpython_grammar_for_subset` and `cpython_grammar_async_for_subset`
  and promoted `for_stmt` from `partial` to `supported` after covering normal
  and async forms, `star_targets`, starred iterable expressions, optional type
  comments, `else` blocks, break/continue behavior, and async-for context
  errors.

Completed in the match value/attr/name-or-attr supported-promotion pass:

- Added `cpython_match_value_attr_name_or_attr_helper_rules_subset` and promoted
  `value_pattern`, `attr`, and `name_or_attr` from `partial` to `supported`
  after covering simple and recursive dotted value patterns, dotted mapping
  keys, bare and dotted class names, and invalid equality/dangling-dot forms.

Completed in the match OR-pattern capture promotion pass:

- Expanded `cpython_match_pattern_helper_rules_subset`,
  `cpython_grammar_match_stmt_subset`, and `runs_match_or_pattern`; promoted
  `or_pattern` from `partial` to `supported` after adding same-name capture
  alternatives, reordered capture alternatives, mapping alternatives,
  parenthesized `as` alternatives, non-final irrefutable rejection, and
  different-name binding rejection.

Completed in the match subject/as/guard supported-promotion pass:

- Expanded `cpython_grammar_match_stmt_subset`,
  `cpython_match_pattern_helper_rules_subset`, `runs_match_as_patterns`, and
  `runs_match_guards`; promoted `subject_expr`, `case_block`, `guard`,
  `patterns`, `pattern`, and `as_pattern` from `partial` to `supported` after
  covering named-expression subjects, starred tuple subjects, nested
  as-patterns, wildcard captures, and named expressions inside guards.

Completed in the matrix-multiply object-protocol promotion pass:

- Expanded `cpython_tokenize_matrix_multiply_and_ellipsis_subset` and added
  `runs_matrix_multiply_special_methods`; promoted `term` from `partial` to
  `supported` after covering `@`, `@=`, `__matmul__`, `__rmatmul__`, and
  `__imatmul__` dispatch, while keeping unsupported builtin operand pairs as
  runtime errors.

Completed in the invalid type-scope expression pass:

- Expanded `cpython_invalid_type_param_subset` and added
  `cpython_invalid_type_scope_expression_subset` plus
  `rejects_invalid_expressions_in_type_scopes`; MiniPython now rejects named
  expressions, yield expressions, and await expressions in TypeVar
  bounds/constraints/defaults, type aliases, and generic class base
  definitions.

Added in the assignment-expression comprehension validation pass:

- Added `cpython_assignment_expression_comprehension_subset` with
  CPython-derived coverage for legal list/dict comprehension walrus usage,
  walrus filters, rejected rebinding of comprehension iteration variables, the
  special inner-loop rebinding error, and rejected assignment expressions in
  comprehension iterable expressions.

Completed in the with-statement supported-promotion pass:

- Expanded `cpython_grammar_with_stmt_subset` and
  `cpython_grammar_async_with_subset`, then promoted `with_stmt` and
  `with_item` from `partial` to `supported` after covering parenthesized and
  non-parenthesized managers, optional type comments, items with and without
  `as`, tuple/list/starred targets, mixed multiple managers, async-with context
  errors, and invalid `as` targets.

Completed in the match sequence/mapping supported-promotion pass:

- Added `cpython_match_sequence_helper_rules_subset` and promoted
  `sequence_pattern`, `open_sequence_pattern`, `maybe_sequence_pattern`, and
  `maybe_star_pattern` from `partial` to `supported` after covering bracketed,
  parenthesized, and naked comma sequence forms, empty sequences, optional
  trailing commas, plain and starred subpatterns, wildcard star targets, and
  duplicate-star rejection.
- Added `cpython_match_mapping_helper_rules_subset` and promoted
  `mapping_pattern`, `items_pattern`, `key_value_pattern`, and
  `double_star_pattern` from `partial` to `supported` after covering empty,
  pure-rest, item-only, item-plus-rest, trailing-comma, literal-key, dotted-key,
  nested-value, invalid rest-target, invalid rest-order, and duplicate-key
  mapping pattern forms.

Completed in the try-statement supported-promotion pass:

- Expanded `cpython_grammar_raise_and_try_except_subset` and
  `cpython_grammar_try_star_subset`, then promoted `try_stmt`, `except_block`,
  and `except_star_block` from `partial` to `supported` after covering
  `try/finally`, ordinary `except` chains, bare `except`, `except ... as`,
  unparenthesized multi-type handlers without `as`, `else` and `finally`
  combinations, `except* ... as`, `except*` with `else`/`finally`,
  ExceptionGroup split behavior, invalid mixed `except`/`except*`, and invalid
  handler target or missing-colon forms.

Completed in the match class-pattern supported-promotion pass:

- Added `cpython_match_class_helper_rules_subset` and promoted
  `class_pattern`, `positional_patterns`, `keyword_patterns`, and
  `keyword_pattern` from `partial` to `supported` after covering empty,
  positional-only, keyword-only, positional-plus-keyword, dotted class,
  trailing-comma, nested subpattern, builtin class, duplicate keyword, and
  invalid keyword-then-positional forms.

Completed in the decorators supported-promotion pass:

- Expanded `cpython_grammar_decorators_subset` and promoted `decorators` from
  `partial` to `supported` after covering repeated decorators, top-to-bottom
  evaluation and bottom-to-top application, PEP 614 expression decorators,
  named-expression decorators, subscript decorators, decorated ordinary
  functions, decorated async functions, decorated classes, and invalid
  decorator placement.

Completed in the invalid match-pattern supported-promotion pass:

- Expanded `cpython_invalid_match_pattern_subset` and promoted
  `invalid_match_stmt`, `invalid_case_block`, `invalid_as_pattern`,
  `invalid_class_pattern`, `invalid_mapping_pattern`, and
  `invalid_class_argument_pattern` from `partial` to `supported` after covering
  missing match/case colons, missing indented match/case blocks, top-level
  `case`, guard-bearing invalid case forms, `_`, literal, attribute, and call
  as-pattern targets, keyword-then-positional class patterns with and without a
  leading positional pattern, and mapping `**rest` before later mapping items.

Completed in the invalid control-flow syntax supported-promotion pass:

- Expanded `cpython_invalid_control_flow_syntax_subset` and promoted
  `invalid_if_stmt`, `invalid_elif_stmt`, `invalid_else_stmt`,
  `invalid_while_stmt`, `invalid_for_stmt`, and `invalid_for_target` from
  `partial` to `supported` after covering missing colons, missing indented
  blocks, top-level `elif`/`else`, `elif` after an `else` block, ordinary and
  async `for` header errors, and invalid ordinary/async `for` assignment
  targets.

Completed in the invalid with-statement supported-promotion pass:

- Expanded `cpython_invalid_control_flow_syntax_subset` and promoted
  `invalid_with_stmt`, `invalid_with_stmt_indent`, and `invalid_with_item`
  from `partial` to `supported` after covering ordinary and async missing
  `with` colons, parenthesized with forms, multiple items without colons,
  missing indented ordinary and async `with` blocks, and invalid ordinary,
  async, parenthesized, and comma-separated `with ... as` targets.

Completed in the invalid try/except/finally supported-promotion pass:

- Expanded `cpython_invalid_control_flow_syntax_subset` and promoted
  `invalid_try_stmt`, `invalid_except_stmt`, `invalid_except_star_stmt`,
  `invalid_finally_stmt`, `invalid_except_stmt_indent`, and
  `invalid_except_star_stmt_indent` from `partial` to `supported` after
  covering missing try/finally/except/except* blocks, try statements without
  except/finally, mixed except and except* handlers, missing except and except*
  colons, missing except* types, unparenthesized multi-type handlers with `as`,
  and invalid exception-handler `as` targets.

Completed in the lambda-parameter supported-promotion pass:

- Expanded `cpython_lambda_parameter_helper_rules_subset` and promoted
  `lambda_params`, `lambda_parameters`, `lambda_star_etc`, `lambda_kwds`,
  `lambda_param_no_default`, `lambda_param_with_default`,
  `lambda_param_maybe_default`, and `lambda_param` from `partial` to
  `supported` after covering slash-with-default lambda parameters, starred
  lambda parameters followed by keyword-only parameters, bare-star
  keyword-only lambdas with and without defaults, `**kwargs` lambda parameters,
  trailing comma forms, and the corresponding invalid lambda parameter rules.
- Expanded `cpython_invalid_lambda_parameters_subset` with CPython
  `Lib/test/test_syntax.py` cases where a later second `*` takes diagnostic
  precedence over earlier duplicate names in the lambda parameter list.
- Added CPython `Lib/test/test_syntax.py` lambda default-boundary cases for
  missing default values before `,` and non-default parameters following
  ordinary or positional-only defaults.

Completed in the function-parameter supported-promotion pass:

- Expanded `cpython_function_parameter_helper_rules_subset` and promoted
  `params`, `parameters`, `param_no_default`, `param_with_default`,
  `param_maybe_default`, `param`, `param_star_annotation`, and `default` from
  `partial` to `supported` after covering slash-with-default function
  parameters, starred and bare-star keyword-only parameters with and without
  defaults, `**kwargs` parameters, inline parameter type comments, ordinary and
  starred annotations, trailing comma forms, and the corresponding invalid
  parameter rules.
- Expanded `cpython_invalid_parameters_subset` with CPython
  `Lib/test/test_syntax.py` cases where `* may appear only once` is reported
  even when the first var-positional parameter name duplicates an earlier
  positional-only parameter.
- Added CPython `Lib/test/test_syntax.py` function default-boundary coverage for
  an annotated parameter with a missing default before `,`.
- Delayed MiniPython's var-positional, keyword-only, and var-keyword uniqueness
  checks until the parameter list has no higher-priority `*`-placement syntax
  error, matching CPython's invalid-parameter diagnostic priority more closely.

Completed in the invalid expression helper supported-promotion pass:

- Expanded `cpython_invalid_expression_rules_subset` and promoted
  `expression_without_invalid`, `invalid_legacy_expression`,
  `invalid_if_expression`, `invalid_arithmetic`, and `invalid_factor` from
  `partial` to `supported` after covering conditional-expression,
  disjunction, and lambda alternatives; legacy `print` and `exec` statement
  expression syntax; starred and double-starred conditional-expression else
  branches; and every CPython `not`-after-arithmetic or unary-operator invalid
  branch.

Completed in the invalid grammar cleanup pass:

- Promoted `invalid_expression`, `invalid_assignment`, and
  `invalid_comprehension` from `partial` to `supported` after covering all
  current CPython alternatives for missing commas, string-adjacent expressions,
  incomplete conditional expressions, statement-in-expression positions,
  unparenthesized f-string/t-string lambdas, invalid annotated targets, chained
  invalid assignment targets, yield assignment, illegal augmented-assignment
  expressions, dict unpacking in list/generator comprehensions,
  unparenthesized list/set comprehension targets, and named-expression
  conflicts in comprehension iterable, filter, and iteration-target positions.

Completed in the import helper supported-promotion pass:

- Expanded `cpython_grammar_import_stmt_subset` and promoted `import_stmt`,
  `import_name`, `import_from`, `import_from_targets`,
  `import_from_as_names`, `import_from_as_name`, `dotted_as_names`,
  `dotted_as_name`, `dotted_name`, `invalid_import`,
  `invalid_dotted_as_name`, `invalid_import_from_as_name`, and
  `invalid_import_from_targets` from `partial` to `supported` after covering
  lazy and ordinary imports, absolute and relative from-import syntax, dotted
  modules, aliases, parenthesized and star import targets, missing targets,
  non-parenthesized trailing commas, reversed `import ... from ...` order,
  `__debug__` binding rejection, and CPython-style invalid alias target
  classes for literals, attributes, function calls, lists, tuples, and
  subscripts.
- A later recursive stdlib compile seed keeps the grammar support honest for
  relative imports inside real packages, including `from ._threading_handler
  import ...`; runtime relative import resolution remains unsupported without a
  custom importer.

Completed in the delete-statement supported-promotion pass:

- Expanded `cpython_grammar_del_stmt_subset`,
  `cpython_delete_target_helper_rules_subset`, and
  `cpython_invalid_simple_statement_subset`; promoted `del_stmt`,
  `del_targets`, `del_target`, `del_t_atom`, and `invalid_del_stmt` from
  `partial` to `supported` after covering multi-target and trailing-comma
  delete statements, empty tuple/list deletes, parenthesized targets, nested
  tuple/list targets, attribute and subscript/slice deletes, chained primary
  delete targets, `__debug__` deletion rejection, CPython-style `True`,
  `False`, `None`, and `Ellipsis` error names, and invalid starred,
  function-call, conditional, operator, named-expression, and nested invalid
  delete targets.

Completed in the invalid raise/assert supported-promotion pass:

- Expanded `cpython_grammar_assert_stmt_subset` and
  `cpython_invalid_simple_statement_subset`; promoted `invalid_raise_stmt` and
  `invalid_assert_stmt` from `partial` to `supported` after covering bare
  `raise from ...`, `raise expression from` without a cause expression,
  parenthesized assert named expressions as valid syntax, accidental `=` inside
  assert conditions and messages, and unparenthesized assert named-expression
  forms.

Completed in the function-type supported-promotion pass:

- Expanded `cpython_func_type_input_subset` and
  `cpython_type_expression_helper_rules_subset`; promoted `func_type` and
  `type_expressions` from `partial` to `supported` after covering empty,
  ordinary, starred, double-starred, ordinary-plus-starred,
  ordinary-plus-double-starred, and ordinary-plus-starred-plus-double-starred
  function type arguments, return expressions, optional trailing newlines, and
  invalid marker ordering/trailing comma forms.

Completed in the match singleton-pattern pass:

- Added an explicit `Pattern::Singleton` AST variant for `case None`,
  `case True`, and `case False`; these now compile to VM identity checks
  (`Is`) instead of ordinary equality checks, matching CPython's
  `MatchSingleton` semantics from `Lib/test/test_patma.py`.
- Expanded `cpython_grammar_match_stmt_subset` and language-level match tests
  for `False` versus `0`, `True` versus `1`, and successful singleton matches.

Completed in the annotation and match wrapper supported-promotion pass:

- Added `cpython_annotation_helper_rule_subset` and promoted `annotation` from
  `partial` to `supported` after covering the CPython `':' expression` wrapper
  with conditional, tuple, and generic-alias annotation expressions in function,
  module, and class contexts.
- Expanded `cpython_grammar_match_stmt_subset` and
  `cpython_invalid_match_pattern_subset`; promoted `match_stmt` from `partial`
  to `supported` after covering the wrapper-level match suite shape, inline case
  bodies, multiple case blocks, invalid empty suites, and delegated invalid
  `match`/`case` alternatives.

Completed in the assignment parent-rule supported-promotion pass:

- Added `cpython_assignment_rule_alternatives_subset` and promoted `assignment`
  from `partial` to `supported` after covering annotated name assignments,
  annotated attribute/subscript assignments, chained `star_targets =` groups,
  augmented assignment, `annotated_rhs` star-expression values, and delegated
  invalid-assignment forms. The invalid-assignment migration batch is now
  completed because `star_targets`, `single_target`, `del_target`, and
  `invalid_assignment` were already supported.

Completed in the function-definition supported-promotion pass:

- Added `cpython_function_def_raw_rule_subset` for CPython's
  `function_def_raw` alternatives, covering ordinary `def`, `async def`,
  optional type parameters, parameters, return annotations, function type
  comments, and inline/indented bodies.
- Added `cpython_function_def_decorated_rule_subset` for the parent
  `function_def` rule, covering decorated ordinary and async functions.
- Promoted `function_def_raw` and `function_def` from `partial` to
  `supported`; `class_def` was left for the following compound-statement pass.

Completed in the parser diagnostic bridge pass:

- Added `ParseError` plus `source_parse_error_diagnostic()` as a bridge API for
  existing parser errors. It preserves the string-based parser contract while
  deriving a first useful source span from the `found ...` token in the parser
  message.
- Extended `cpython_invalid_function_def_raw_subset` with spans for missing
  function-header parentheses, missing function colons, and missing indented
  function blocks.
- Extended `cpython_invalid_block_subset` with spans for missing indented suites
  after compound-statement headers.
- Extended the bridge to derive spans for invalid assignment targets before `=`
  and invalid walrus targets before `:=`, then promoted representative
  CPython-style cases in `cpython_invalid_named_expression_subset`,
  `cpython_invalid_assignment_target_subset`, and
  `cpython_invalid_assignment_and_annotation_subset`.
- Added `SpannedToken` plus `lex_with_spans()` so parser diagnostics can consume
  lexer-produced token locations without breaking the existing `Vec<Token>`
  parser/compiler API. The invalid assignment diagnostic path now prefers these
  token spans and falls back to source-text scanning when needed.
- Added `ParserDiagnostic` plus `parse_with_diagnostic()` so the parser can
  report the token index associated with a failed parse while preserving the
  existing string-based `parse()` API. `source_parse_error_diagnostic()` now
  prefers that parser token index for `found ...` errors, which fixes repeated
  token-shape cases such as a second `print` being the actual unexpected token.
- Extended parser-token-index spans to representative parser errors that do not
  include a `found ...` suffix, including empty inline suites after `:`, empty
  parenthesized `with` item lists, missing function default values, missing call
  keyword values, and the missing comma between `/` and `*` in parameters.
- Kept this as a bridge rather than the final parser model: parser internals
  still consume bare `Token`s, so deeper CPython `SyntaxError` parity should
  eventually carry token spans directly inside parser errors.

Completed in the match mapping literal-key duplicate pass:

- Expanded parser-side mapping pattern duplicate-key checks to normalize static
  literal keys with Python equality semantics, covering CPython
  `Lib/test/test_patma.py` cases such as `{0: _, False: _}`, `{0: _, 0.0: _}`,
  `{0: _, -0: _}`, and `{0: _, 0j: _}`.
- Added CPython-subset and language-level tests for those duplicate literal key
  forms, plus invalid f-string mapping keys.

Completed in the match mapping dynamic-key duplicate pass:

- Added a register-VM `MatchMappingKeys` instruction so mapping patterns first
  evaluate all keys, perform the CPython-style mapping length gate, reject
  dynamic duplicate keys with catchable `ValueError`, and only then load values
  for subpattern matching.
- Migrated the CPython `Lib/test/test_patma.py`
  `test_mapping_pattern_checks_duplicate_key_1` semantics for
  `{Keys.KEY: y, "a": z}`, including the guarantee that failed dynamic
  duplicate-key matches do not bind `y` or `z`.

Completed in the match class-pattern runtime TypeError pass:

- Refined builtin class-pattern handling so MiniPython distinguishes
  match-self builtins such as `int` and `str` from zero-positional builtins such
  as `range`, `slice`, `object`, `type`, and exception classes.
- Migrated CPython `Lib/test/test_patma.py` TypeError semantics for
  `range(10)`, non-tuple `__match_args__` values such as `"XYZ"` and
  `["spam", "eggs"]`, and verified these failed class-pattern matches leave
  capture variables unbound.

Completed in the match class-pattern non-class-callee pass:

- Added `max` as a basic builtin and migrated CPython
  `Lib/test/test_patma.py::test_class_pattern_not_type`, so a class pattern
  whose callee resolves to a non-class object raises a catchable `TypeError`
  with no capture binding side effects.
- Aligned the existing dotted-value callee failure path, such as `case A.B()`
  where `A.B` is an ordinary value, with CPython's "called match pattern must be
  a class" behavior.

Completed in the generator-expression walrus scope pass:

- Expanded `cpython_assignment_expression_comprehension_subset` with CPython
  `Lib/test/test_named_expressions.py` semantics for assignment expressions in
  generator expressions: the assignment target updates the containing module or
  function scope after iteration, not the generator frame's private locals.
- Added CPython-derived global/nonlocal comprehension walrus cases covering
  list comprehensions, generator expressions, explicit `global`, explicit
  `nonlocal`, and the no-`nonlocal` nested-function case where the binding stays
  in the immediate containing function rather than leaking to an outer function.
- Added the `any` builtin and migrated the CPython `containsOne =
  any((lastNum := num) == 1 for num in [1, 2, 3])` scope behavior, including
  short-circuiting so the walrus target keeps the first truthy element rather
  than the final iterable element.
- Migrated more CPython `NamedExpressionScopeTest` cases for accumulation,
  nested walrus calls, filters, nested comprehensions, and same-name walrus
  reassignments inside already-supported comprehension shapes.
- Added a `StoreOuterName` register-VM instruction for this closure-write path,
  while keeping explicit `global`/`nonlocal` declarations on the existing
  global/nonlocal store instructions.

Completed in the lambda grammar supported-promotion pass:

- Migrated additional CPython `Lib/test/test_grammar.py` lambda/comprehension
  interaction cases, including lambda calls inside comprehensions, tuple
  iterables of lambdas, and conditional-expression lambda bodies.
- Promoted `lambdef` from `partial` to `supported` after confirming the rule's
  optional-parameter and expression-body alternatives are covered by executable
  CPython-derived tests plus the existing lambda parameter helper suites.
- Promoted the parent `expression` rule from `partial` to `supported` after its
  invalid-expression, conditional-expression, boolean-disjunction, and lambda
  alternatives were all covered by supported child rows.

Completed in the star-expression helper supported-promotion pass:

- Added `cpython_star_expressions_helper_rules_subset` for CPython
  `star_expressions` / `star_expression` contexts: assignment RHS, return
  values, for-loop iterables, annotated RHS values, and yield values.
- Expanded `cpython_star_named_expression_helper_rules_subset` with trailing
  comma cases for named expressions and starred elements in list, tuple, and
  set displays.
- Promoted `star_expressions`, `star_expression`,
  `star_named_expressions`, `star_named_expressions_sequence`,
  `star_named_expression`, and `star_named_expression_sequence` from `partial`
  to `supported`.
- Added `cpython_expressions_helper_rules_subset` and promoted the parent
  `expressions` rule after covering comma-separated expression tuples,
  single-expression trailing-comma tuples, and the single-expression
  alternative.

Completed in the slices supported-promotion pass:

- Added CPython-current starred-subscript coverage to
  `cpython_selector_helper_rules_subset`, including `mapping[*items]`
  load/store/augassign/delete behavior and `tuple[*Ts]` generic-alias unpack
  syntax.
- Extended the parser so `slices` accepts `starred_expression` items in
  subscript brackets and lowers single starred subscript items to tuple-style
  indexes, matching CPython's `slices` grammar shape.
- Added an `Unpack` runtime value and `BuildUnpack` bytecode instruction so
  type-scoped generic aliases can preserve `tuple[*Ts]` as an unpack argument
  rather than erasing the star.
- Promoted `slices` and `slice` from `partial` to `supported`.

Completed in the slice indices runtime pass:

- Extended `cpython_selector_helper_rules_subset` and the differential parity
  harness with `Lib/test/test_slice.py::SliceTest::test_indices` cases.
- Added first-pass `slice.indices(length)` runtime support, including positive
  and negative step normalization, out-of-range clipping, negative-length and
  zero-step `ValueError` paths, non-indexable `TypeError` paths, and custom
  `__index__` objects for start, stop, step, and length.

Completed in the group supported-promotion pass:

- Added `cpython_group_helper_rule_subset` for CPython's `group` rule, covering
  the parenthesized `yield_expr` and parenthesized `named_expression`
  alternatives.
- Reused existing invalid-group coverage for parenthesized `*expr` and
  `**expr` forms, plus existing redundant-parentheses coverage.
- Promoted `group` from `partial` to `supported`.

Completed in the dict kvpair supported-promotion pass:

- Added `cpython_dict_kvpair_helper_rules_subset` for CPython's `dict`,
  `double_starred_kvpairs`, `double_starred_kvpair`, and `kvpair` rules,
  covering empty dictionaries, expression key/value pairs, `**` unpack entries,
  mixed entries, parenthesized conditional unpack values, and optional trailing
  commas.
- Reused existing invalid dict display coverage for
  `invalid_double_starred_kvpairs`, `invalid_kvpair_unpacking`, and
  `invalid_kvpair`.
- Promoted `dict`, `double_starred_kvpairs`, `double_starred_kvpair`, and
  `kvpair` from `partial` to `supported`.

Completed in the sequence display supported-promotion pass:

- Added `cpython_sequence_display_helper_rules_subset` for CPython's `list`,
  `tuple`, and `set` rules, covering empty lists, empty tuples,
  comma-disambiguated tuple syntax, star-named expression sequences, named
  expressions, iterable unpacking, duplicate-collapsing set displays, and
  optional trailing commas.
- Reused existing starred-display and literal-display coverage for runtime and
  AST-shaped behavior.
- Promoted `list`, `tuple`, and `set` from `partial` to `supported`.

Completed in the named-expression supported-promotion pass:

- Added `cpython_named_expression_helper_rules_subset` for CPython's
  `assignment_expression` and `named_expression` rules, covering
  `NAME := expression`, plain expression usage in named-expression positions,
  and invalid walrus syntax.
- Reused existing named-expression runtime and comprehension-scoping tests for
  nested walrus bindings, condition/call/subscript positions, invalid targets,
  and comprehension rebinding restrictions.
- Promoted `assignment_expression` and `named_expression` from `partial` to
  `supported`.

Completed in the comprehension expression supported-promotion pass:

- Added `cpython_comprehension_expression_rules_subset` for CPython's
  `listcomp`, `setcomp`, `genexp`, and `dictcomp` rules, covering star-named
  list/set elements, ordinary generator elements, assignment-expression
  generator elements, starred generator elements, key/value dict
  comprehensions, and `**expr` dict-unpack comprehensions.
- Reused existing `for_if_clauses`, `for_if_clause`,
  `invalid_comprehension`, async-comprehension, named-expression scoping,
  yield-boundary, and unpacking tests for the shared clause and error
  alternatives.
- Promoted `listcomp`, `setcomp`, `genexp`, and `dictcomp` from `partial` to
  `supported`.

Completed in the type-alias/simple-statement supported-promotion pass:

- Added `cpython_type_alias_statement_subset` for CPython's `type_alias` rule
  and PEP 695 soft-keyword behavior, covering plain aliases, generic aliases,
  function/class-local aliases, `type` as an ordinary assignment target, an
  alias named `type`, and the missing-`=` error shape.
- Promoted `type_alias` and the parent `simple_stmt` row from `partial` to
  `supported`.

Completed in the class/compound-statement supported-promotion pass:

- Added `cpython_class_def_decorated_rule_subset` for the parent `class_def`
  rule, covering decorated and undecorated class definitions independently of
  the broader decorator helper tests.
- Added `cpython_compound_stmt_rule_alternatives_subset` for the parent
  `compound_stmt` dispatch surface, covering function, if, class, with, for,
  try, while, and match alternatives in one executable slice.
- Promoted `class_def`, `class_def_raw`, and the parent `compound_stmt` row from
  `partial` to `supported`; `function_def` and `function_def_raw` were already
  covered by the focused function definition tests and synchronized in the
  inventory.

Completed in the await-primary supported-promotion pass:

- Added `cpython_await_primary_rule_subset` for CPython's `await_primary` rule,
  covering awaited calls, attributes, subscripts, grouped primaries, invalid
  unary operands after `await`, and the power precedence boundary where
  `await f() ** 2` means `(await f()) ** 2`.
- Fixed MiniPython's parser to model CPython's `power: await_primary '**'
  factor` shape instead of parsing `await` as a broader factor operand.
- Promoted `await_primary` and the coverage-only `await_expression` row from
  `partial` to `supported`.

Completed in the primary supported-promotion pass:

- Added `cpython_primary_rule_subset` for CPython's recursive `primary` rule,
  covering attribute chains, ordinary calls, generator-expression calls,
  subscripts, and atom fallthrough in one executable source.
- Promoted `primary` from `partial` to `supported`.

Completed in the atom supported-promotion pass:

- Added `cpython_atom_rule_subset` for CPython's `atom` alternatives, covering
  names, `True`/`False`/`None`, strings, numbers, generator/group/tuple forms,
  list/list-comprehension forms, dict/set/comprehension forms, and ellipsis.
- Promoted `atom` from `partial` to `supported`.

Completed in the string-family supported-promotion pass:

- Added `cpython_f_string_helper_rules_subset` for CPython's f-string helper
  rules, covering literal middles, replacement fields, `annotated_rhs` yield
  expressions, debug fields, conversions, empty/full format specs, and nested
  replacement fields.
- Extended `cpython_string_and_tstring_helper_rules_subset` so `string`,
  `strings`, t-string middles/replacement fields/conversions/full format specs,
  and t-string-only concatenation have focused executable evidence.
- Promoted the remaining expression grammar rows from `partial` to `supported`:
  `string`, `strings`, all f-string helper rules, and all t-string helper
  rules. The CPython grammar inventory now has `0` partial rows and `276`
  supported rows.

Completed in the first tokenizer-row promotion pass:

- Promoted token rows with focused lexer/parser/runtime evidence:
  `COLONEQUAL`, `EXCLAMATION`, `TYPE_IGNORE`, `TYPE_COMMENT`, `SOFT_KEYWORD`,
  `COMMENT`, and `NL`.
- Left the broader token rows `NUMBER`, `STRING`, f-string/t-string token
  triples, and `ERRORTOKEN` as `partial` because those are tokenizer-surface
  modeling decisions or larger invalid-input families.

Completed in the tokenizer pathological whitespace pass:

- Extended `cpython_tokenize_exact_type_subset` with CPython
  `Lib/test/test_tokenize.py::test_pathological_trailing_whitespace`, proving
  an `@` token is preserved when followed only by trailing spaces.

Completed in the Unicode identifier pass:

- Added Unicode XID identifier lexing for `NAME` tokens, keeping ASCII keyword
  recognition unchanged while allowing names such as `tenπ`, `变量`, `加一`,
  and `盒子`.
- Added CPython-style NFKC normalization for non-ASCII identifier names, so
  compatibility spellings such as `K` / `K`, fullwidth `ｘ` / `x`, and
  micro-sign `µ` / Greek `μ` address the same binding while f-string debug
  labels preserve the original source text.
- Added CPython-derived coverage from `Lib/test/test_fstring.py` for a
  non-ASCII f-string debug expression, plus executable coverage for Unicode
  identifiers in assignments, function definitions, class definitions, calls,
  and attribute lookup.

Completed in the numeric-token invalid-literal pass:

- Added `cpython_bad_numerical_literals_subset`, adapted from
  `Lib/test/test_grammar.py::test_bad_numerical_literals`,
  `Lib/test/test_tokenize.py` token-error cases, and
  `Lib/test/support/numbers.py::INVALID_UNDERSCORE_LITERALS`.
- Extended the same subset with invalid leading-zero and prefixed-base forms
  from CPython `Lib/test/test_compile.py::test_literals_with_leading_zeroes`,
  including `0xj`, `0x.`, `0BADCAFE`, `0b101j`, `0o153j`, and
  prefixed exponent-like spellings.
- Tightened lexer rejection for CPython-invalid underscore positions such as
  `1._4` and `._5`, which were previously tokenized as dotted expressions.
- Kept `NUMBER` as `partial` because MiniPython still uses an `i64` integer
  value model and does not yet model every CPython tokenizer diagnostic or
  adjacent-number warning/error edge.

Completed in the leading-zero numeric literal pass:

- Added `cpython_compile_literals_with_leading_zeroes_subset`, adapted from
  CPython `Lib/test/test_compile.py::test_literals_with_leading_zeroes`.
- Covered valid leading-zero float, exponent, and imaginary literal forms such
  as `0777.`, `0777e1`, `0000e-012`, `09.5`, `0777j`, and `00j`, plus
  prefixed integer forms with leading zero digits.

Completed in the bytes-literal pass:

- Added lexer, AST, compiler, and VM support for bytes literals, including
  `b`/`B`, raw `br`/`rb` prefix combinations, adjacent bytes-literal
  concatenation, ASCII-only literal validation, common byte escapes, repr/str,
  equality, ordering, concatenation, repetition, `len`, indexing, slicing,
  iteration, and the minimal `bytes()` constructor surface.
- Added `cpython_bytes_literal_subset`, adapted from `Lib/test/test_tokenize.py`,
  `Lib/test/test_bytes.py`, and `Lib/test/test_ast/test_ast.py`.
- Kept `STRING` as `partial` because the row still tracks the broader CPython
  tokenizer string surface, including exact tokenizer token-stream modeling and
  remaining invalid-literal diagnostics.

Completed in the string-prefix matrix pass:

- Added `lexes_cpython_string_prefix_matrix` and
  `cpython_string_prefix_matrix_subset`, adapted from
  `Lib/test/test_tokenize.py::StringPrefixTest` and
  `Lib/test/test_grammar.py::test_string_prefixes` / `::test_bytes_prefixes`.
- Covered the legal MiniPython/CPython prefix matrix for plain strings,
  f-strings, t-strings, and bytes, plus representative invalid prefix
  combinations that must be rejected before VM execution.
- Added `cpython_invalid_string_prefix_matrix_subset`, adapted from
  `Lib/test/test_fstring.py::test_invalid_string_prefixes`, covering the
  CPython single- and double-quote matrix for incompatible `f`/`u`/`r`/`b`
  prefix families such as `fu''`, `ufr''`, `rfu''`, `fb''`, and `bf''`.
  MiniPython now rejects those forms in the lexer with `prefixes are
  incompatible` before parser or VM execution.

Completed in the async function definition pass:

- Added `cpython_async_funcdef_rule_subset`, adapted from CPython's
  `async_funcdef` grammar rule and async-generator syntax tests.
- Covered empty coroutine bodies, type-parameterized async functions, complex
  parameter lists, return annotations, function type comments, nested async
  functions, and CPython's rejection of `yield from` inside `async def`.
- Migrated CPython `Lib/test/test_coroutines.py::test_func_2` observable
  coroutine exception behavior. Unhandled `StopIteration` raised inside a
  coroutine now becomes `RuntimeError("coroutine raised StopIteration")` with
  the original exception exposed through `__cause__`, while
  `StopAsyncIteration` still propagates unchanged.
- Migrated CPython `Lib/test/test_coroutines.py::test_func_13`,
  `::test_func_18`, and `::test_coro_wrapper_send_stop_iterator` observable
  coroutine wrapper behavior. `coro.__await__()` now returns an iterator-style
  wrapper whose `iter()` identity is stable, whose exhausted reuse raises
  `RuntimeError`, whose `close()` is idempotent after exhaustion, and whose
  underlying coroutine still treats returned `StopIteration` objects as values
  rather than as raised coroutine termination.
- Migrated CPython `Lib/test/test_coroutines.py::test_await_3`,
  `::test_await_6`, and `::test_await_7` observable await protocol behavior.
  Custom `__await__` iterators now yield through the suspended outer coroutine,
  retain their pending iterator state across `send(None)`, and use the
  iterator's completion value as the result of the `await` expression.
- Migrated CPython `Lib/test/test_coroutines.py::test_await_5`,
  `::test_await_12`, and `::test_await_13` observable await return-type
  errors. `__await__()` now rejects `None`, coroutine objects, and other
  non-iterator return values with CPython-style catchable `TypeError`
  messages.
- Migrated CPython `Lib/test/test_coroutines.py::test_await_8`,
  `::test_await_9`, `::test_await_10`, and `::test_await_11` observable await
  expression behavior. Objects without `__await__` now raise the expected
  catchable `TypeError`, and await expressions compose through arithmetic,
  nested awaits, call keyword arguments, and tuple values.
- Migrated CPython `Lib/test/test_coroutines.py::test_await_14`,
  `::test_await_15`, and `::test_await_16` observable await-resume behavior.
  Suspended await expressions now receive both `send(value)` and `throw(exc)`,
  coroutine `__await__()` wrappers forward those resumes to the underlying
  coroutine, attempts to await an already-suspended coroutine raise
  `RuntimeError`, and exception objects returned from awaited coroutines do not
  inherit the surrounding exception context. The `test_await_14` case now uses
  CPython's original custom `Marker(Exception)` shape rather than a builtin
  exception substitute, so user exception classes deriving from `Exception` can
  be raised, caught, and displayed in this subset.
- Promoted `async_funcdef` from `partial` to `supported`; the remaining partial
  rows are now tokenizer-surface rows rather than parser grammar rules.

Completed in the large-integer literal pass:

- Added a `BigInt` path for integer literals that exceed `i64`, while keeping
  the existing `Number(i64)` path for small integers so current bytecode and VM
  tests remain stable.
- Added `lexes_large_integer_literals` and `cpython_large_integer_literals_subset`,
  adapted from CPython's arbitrary-precision integer behavior in
  `Lib/test/test_long.py`, integer literal grammar examples, and underscore
  literal tokenizer coverage.
- Covered large decimal, binary, octal, and hexadecimal integer literals,
  uppercase and lowercase binary/octal/hex prefixes from
  `Lib/test/test_grammar.py::test_long_integers`, underscore normalization,
  arithmetic past the old `i64` boundary, bitwise operations, shifts,
  exponentiation, comparison, truthiness, formatting as `int`, and
  `isinstance(..., int)`.
- Kept `NUMBER` as `partial` because the row still includes broader tokenizer
  diagnostics and float/imaginary edge cases beyond this integer-literal pass.

Completed in the valid underscore number literal pass:

- Added `cpython_valid_underscore_number_literals_subset`, adapted from
  CPython's shared `VALID_UNDERSCORE_LITERALS` table in
  `Lib/test/support/numbers.py`.
- Migrated all currently relevant valid underscore forms in that table:
  decimal, binary, octal, hexadecimal, floats, exponent floats, imaginary
  literals, and parenthesized complex arithmetic.
- Matched CPython's test strategy by asserting each literal evaluates the same
  as the spelling with underscores removed.

Completed in the float literal forms pass:

- Added `cpython_float_literal_forms_subset`, adapted from
  `Lib/test/test_grammar.py::test_floats`.
- Expanded `lexes_float_literals` to cover CPython's accepted float spellings:
  plain decimals, trailing-dot floats, leading-zero fractional forms, leading
  dot floats, exponent forms with `e`/`E`, signed exponents, point floats with
  exponents, and fractional exponent floats.
- Kept `NUMBER` as `partial` because CPython's numeric-token boundary tests
  still include warning/error behavior around adjacent keywords/names and
  non-ASCII characters after numeric literals.

Completed in the numeric-token boundary pass:

- Added `cpython_float_exponent_tokenization_subset`, adapted from
  `Lib/test/test_grammar.py::test_float_exponent_tokenization`.
- Added `cpython_end_of_numerical_literals_subset`, adapted from
  `Lib/test/test_grammar.py::test_end_of_numerical_literals`.
- Added `lexes_number_keyword_boundaries` to lock in CPython-style token
  splitting for `1else`, `1jand`, `0xfor`, and `0x1ffor`.
- Added lexer-level invalid-literal diagnostics for adjacent non-keyword names
  such as `1spam`, `1Else`, `1jspam`, and `0xfand`, while preserving the
  CPython keyword-boundary warning path.
- Added the CPython `invalid character '⁄' (U+2044)` diagnostic for the
  fraction-slash shape after numeric literals.
- Tightened decimal exponent scanning so `1else` is tokenized as `1 else`,
  while malformed exponent starts such as `1e+` and `1e_1` still reject as
  invalid number literals.
- Aligned trailing-dot numeric tokenization with CPython: `1.and x` is treated
  as `1. and x`, while ordinary attribute access remains available through a
  parenthesized number such as `(1).value`.
- Kept `NUMBER` as `partial` because the broader CPython numeric-tokenizer
  surface still has additional location and diagnostic edge cases to migrate.

Completed in the string octal escape pass:

- Added `lexes_string_octal_escapes` and
  `cpython_string_octal_escape_subset`, adapted from
  `Lib/test/test_string_literals.py`.
- Extended ordinary string escape lexing to consume one-, two-, and three-digit
  octal escapes such as `\1`, `\01`, `\001`, `\377`, `\400`, and `\777` as
  Unicode code points.
- Kept `STRING` as `partial` because MiniPython still does not model CPython's
  `SyntaxWarning` channel for invalid escape sequences / high octal escapes,
  exact warning locations, or Unicode-name escape handling.

Completed in the Unicode name escape pass:

- Added the `unicode_names2` dependency to resolve Unicode standard character
  names for Python `\N{...}` string escapes.
- Added `lexes_unicode_name_escapes`, `lexes_unicode_name_alias_escapes`, and
  `cpython_unicode_name_escape_subset`, adapted from
  `Lib/test/test_string_literals.py` and `Lib/test/test_fstring.py`.
- Implemented `\N{...}` for ordinary strings plus f-string and t-string
  literal segments, including CPython-style case-insensitive standard names and
  raw-string preservation.
- Added `unic-ucd-name_aliases` to accept exact Unicode alias spellings such as
  `LF`, `LINE FEED`, `NEW LINE`, `NUL`, `BACKSPACE`, and `BOM` while still
  rejecting CPython-invalid loose spellings such as `NEW_LINE`, repeated spaces,
  and missing word separators.
- Rejected malformed `\N` escapes and unknown names, including loose names with
  underscores that `unicode_names2` would otherwise accept but CPython rejects.
- Kept `STRING` as `partial` because MiniPython still does not model CPython's
  `SyntaxWarning` channel, exact error offsets, source-encoding detection, or
  exact CPython Unicode-version synchronization.

Completed in the invalid string literal pass:

- Added `rejects_cpython_unterminated_string_forms`,
  `rejects_cpython_invalid_string_escape_forms`, and
  `cpython_invalid_string_literal_subset`, adapted from CPython SyntaxError
  coverage for unterminated string literals and tokenizer errors for malformed
  string/bytes escapes.
- Tightened lexer diagnostics so unterminated triple-quoted strings report a
  distinct error and malformed `\x`, `\u`, and `\U` escapes reject before parser
  or VM execution.
- Added structured `LexError` span coverage to `cpython_invalid_string_literal_subset`
  for ordinary, raw-prefixed, bytes-prefixed, newline-terminated, and triple-quoted
  unterminated strings.
- Extended the same structured diagnostics to non-ASCII bytes literals, covering
  the whole offending bytes literal as CPython's doctest examples do.
- Adjusted string and bytes hex escape reading so the closing quote is treated
  as the end of the literal, not as part of the malformed escape text.
- Kept `STRING` and `ERRORTOKEN` as `partial` because they still cover broader
  tokenizer-surface parity beyond this invalid-literal slice.

Completed in the unterminated string tokenizer span expansion pass:

- Extended `cpython_invalid_string_literal_subset` with CPython
  `Lib/test/test_tokenize.py::test_invalid_syntax` source spellings for
  unterminated ordinary strings and near-closed triple-quoted strings such as
  `'''sdfsdf''`.
- Added span checks for multiline triple-quoted EOF and unterminated bytes
  literals, and mirrored the triple-quoted token-error shapes in
  `cpython_tokenize_error_token_subset`.
- Kept `STRING` and `ERRORTOKEN` as `partial` because MiniPython still reports
  a collapsed lexer error instead of CPython's exact tokenize token stream and
  still does not model source-encoding detection.

Completed in the unterminated interpolated string pass:

- Added `rejects_cpython_unterminated_interpolated_string_forms` and extended
  `cpython_invalid_f_string_syntax_subset` / `cpython_invalid_t_string_syntax_subset`.
- Migrated the CPython source shapes for `f'`, `f'''`, `t'`, `t'''`, and
  `t''''` from `Lib/test/test_fstring.py::test_not_closing_quotes` and
  `Lib/test/test_tstring.py::test_syntax_errors`.
- Tightened lexer diagnostics so normal and triple-quoted f/t-string literals
  report distinct unterminated-literal errors.

Completed in the unterminated replacement field pass:

- Extended `rejects_invalid_f_string_forms`,
  `cpython_invalid_f_string_syntax_subset`, and
  `cpython_invalid_t_string_syntax_subset` with CPython source shapes such as
  `f'{3'`, `f'{3!'`, `f'{3!s'`, `f'x{'`, `f'x{x'`, `t'{'`, and `t'{a'`.
- Threaded the outer f/t-string quote into replacement-field lexing so the
  lexer reports `expecting '}'` when a field reaches the string terminator
  before its closing brace.

Completed in the conversion and format-spec error pass:

- Extended the invalid f/t-string subset tests with CPython conversion forms
  `! s`, `!ss`, `!ss:`, and `!ss:s` from
  `Lib/test/test_fstring.py::test_conversions`.
- Added f-string and t-string format-spec newline rejection from
  `Lib/test/test_fstring.py::test_newlines_in_format_specifiers` and
  `Lib/test/test_tstring.py::test_syntax_errors`, while keeping triple-quoted
  f-string format specs able to lex newlines like CPython's valid compile-only
  cases.
- Centralized conversion parsing in the lexer so missing conversions, separated
  conversions, multi-character conversions, and valid `!s`/`!r`/`!a` remain
  consistent for regular and debug replacement fields.
- Added CPython-derived rejection coverage for post-expression separators and
  nested format-spec expression errors such as `f'{x;y}'`, `f'{x:{;}}'`,
  `f'{x!:}'`, `t'{x;y}'`, `t'{lambda:1}'`, and `t'{x:{;}}'`, with f-string
  forms also covered by the differential CPython/MiniPython rejection harness.

Completed in the raw f-string format-spec pass:

- Added `cpython_raw_f_string_format_spec_subset` from
  `Lib/test/test_fstring.py::test_raw_fstring_format_spec`.
- Threaded the outer raw-string flag into f-string format-spec lexing so normal
  format specs decode escapes such as `\x33`, while raw format specs preserve
  literal backslashes; nested replacement-field expressions continue to use
  their own string prefixes.
- Used integer width formatting and runtime rejection for raw `\x33` to make the
  CPython raw/non-raw distinction observable independently from custom object
  formatting.

Completed in the f-string comments pass:

- Added CPython-derived coverage from `Lib/test/test_fstring.py::test_comments`
  to `cpython_f_string_basic_subset` and
  `cpython_invalid_f_string_syntax_subset`.
- Updated replacement-field scanning so `#` starts a comment outside nested
  quoted strings; braces inside that comment no longer close the f-string field.
- Filtered newline and type-comment tokens when parsing f/t-string replacement
  expressions, matching CPython's implicit-line-joining behavior inside
  replacement fields and allowing triple-quoted expressions with inline
  comments.
- Extended the comment handling to strip comment bodies from f-string debug
  labels while preserving the newline/indentation around them, covering CPython
  debug-comment cases such as `f"{1+2 = # my comment\n  }"`.
- Filtered synthetic `INDENT` / `DEDENT` tokens from replacement-expression
  parsing so comment-first multiline fields like `f'{ # comment\n  """hello"""=}'`
  parse as expressions instead of suites.

Completed in the f-string format-spec greedy-matching pass:

- Added CPython-derived coverage from
  `Lib/test/test_fstring.py::test_fstring_format_spec_greedy_matching` to
  `cpython_f_string_basic_subset` and
  `cpython_invalid_f_string_syntax_subset`.
- Changed format-spec scanning so the first `}` closes the replacement field;
  a following `}}` is then handled by the outer f-string literal scanner. This
  keeps `f'{1:}}}'` as empty format spec plus literal `}`, and avoids treating
  `}}` as a literal right brace inside the format spec.

Completed in the escaped-brace f-string literal pass:

- Added CPython-derived coverage from
  `Lib/test/test_fstring.py::test_backslashes_in_expression_part` to
  `cpython_f_string_basic_subset` and
  `cpython_invalid_f_string_syntax_subset`.
- Added rejected f-string and raw f-string cases from
  `Lib/test/test_fstring.py::test_invalid_backslashes_inside_fstring_context`
  for expression sources beginning with line-continuation backslashes.
- Covered multiline replacement expressions and confirmed string escapes inside
  replacement expressions are parsed with normal expression string rules,
  independent of the outer f-string prefix.
- Added CPython-derived coverage from
  `Lib/test/test_fstring.py::test_no_escapes_for_braces` to
  `cpython_f_string_basic_subset`.
- Added `lexes_f_string_escaped_brace_literals` for direct lexer coverage of
  decoded brace escapes in f-string literal middle text.
- Verified escaped braces produced by `\x7b`, `\u007b`, and Unicode name
  escapes are literal f-string middle text; only source-level `{` and `}` drive
  replacement-field parsing.

Completed in the f-string replacement newline pass:

- Added CPython-derived coverage from
  `Lib/test/test_fstring.py::test_newlines_in_expressions` to
  `cpython_f_string_basic_subset`.
- Covered raw triple-quoted f-strings whose replacement expression spans a
  physical newline, preserving CPython's behavior that expression parsing owns
  those newlines.

Completed in the f-string lambda expression pass:

- Added CPython-derived coverage from `Lib/test/test_fstring.py::test_lambda`
  to `cpython_f_string_basic_subset` and
  `cpython_invalid_f_string_syntax_subset`.
- Extended unparenthesized-lambda detection for f-string/t-string format-spec
  ambiguity from expression sources that start with `lambda` to top-level tuple
  items such as `1, lambda:x`, without treating `+ lambda` or quoted text as the
  dedicated lambda-without-parentheses error.

Completed in the f-string starred/debug syntax-error pass:

- Added CPython-derived coverage from
  `Lib/test/test_fstring.py::test_syntax_error_for_starred_expressions` to
  `cpython_invalid_f_string_syntax_subset`, covering rejected `*expr` and
  `**expr` replacement fields.
- Added CPython-derived coverage from
  `Lib/test/test_fstring.py::test_syntax_error_after_debug`, covering bad
  replacement fields immediately after debug-expression fields such as
  `f'{1=}{;` and `f'{1=}{1;}'`.
- Added CPython-derived coverage from
  `Lib/test/test_fstring.py::test_invalid_syntax_error_message`, covering
  illegal operators inside replacement fields such as `f'{a $ b}'`.
- The existing f-string lexer/parser path already rejected these source shapes,
  so this pass tightened migration coverage without adding new opcodes or VM
  behavior.

Completed in the f-string raw prefix and backslash-brace pass:

- Added CPython-derived coverage from
  `Lib/test/test_fstring.py::test_valid_prefixes`,
  `::test_roundtrip_raw_quotes`, and
  `::test_fstring_backslash_prefix_raw`, and
  `::test_fstring_backslash_before_double_bracket` to
  `cpython_f_string_basic_subset`.
- Added `lexes_f_string_backslash_before_doubled_braces` for direct lexer
  coverage of raw and non-raw backslash-plus-doubled-brace scanning.
- Adjusted f-string literal scanning so a backslash before `{` or `}` preserves
  the backslash but leaves the brace to participate in source-level f-string
  brace handling. This matches CPython's observable output for the supported
  subset; warning channels are still not modeled.
- Adjusted raw f-string literal scanning so backslashes are literal without
  swallowing following braces, while still allowing quote-delimiter escapes to
  round-trip as raw text.

Completed in the expanded numerical literal boundary pass:

- Extended `cpython_end_of_numerical_literals_subset` with the remaining
  executable CPython source shapes from `Lib/test/test_grammar.py`:
  literal-adjacent `or`, conditional expressions where `else` follows the
  numeric literal, literal-adjacent `is`, and hexadecimal literal boundaries
  before `is`, `in`, and `not in`.
- Kept SyntaxWarning-specific assertions out of the Rust API for now because
  MiniPython does not yet expose a warning channel.

Completed in the immutable sequence special-method pass:

- Added CPython parity coverage for `tuple`, `str`, `bytes`, and `range`
  `__getitem__`, `__contains__`, and `__len__` behavior using the existing
  executable `tests/cpython_diff.rs` harness.
- Covered shared subscript semantics, slicing behavior, membership type checks,
  and arity/index rejection cases before wiring the VM method surface.

Completed in the dict special-method pass:

- Added CPython parity coverage for `dict.__getitem__`, `__setitem__`,
  `__delitem__`, `__contains__`, and `__len__`.
- Reused the existing VM subscript, membership, and length semantics so direct
  mapping operations and special-method calls stay aligned.
- Covered missing-key, unhashable-key, and special-method arity rejection cases.

Completed in the set special-method pass:

- Added CPython parity coverage for direct `set` special methods:
  `__contains__`, `__len__`, set algebra dunders, subset/superset comparison
  dunders, and equality dunders for set operands.
- Reused existing set operator helpers so `s | t` and `s.__or__(t)` share the
  same result path for supported operands.
- Tightened set membership to reject unhashable lookup values, matching CPython
  for both `in` and `__contains__`.

Completed in the NotImplemented singleton pass:

- Added a `NotImplemented` runtime value and built-in name lookup alongside
  existing singleton handling for `Ellipsis`.
- Migrated CPython parity coverage for `NotImplemented` display, truthiness,
  identity, and equality.
- Updated direct `set` dunders to return `NotImplemented` for unsupported
  non-set operands while preserving operator-level TypeError behavior.

Completed in the string line-continuation tokenizer pass:

- Added CPython-derived explicit-line-joining coverage for ordinary string,
  triple-quoted string, raw string, bytes, raw bytes, f-string, and raw f-string
  literal text.
- Added direct lexer coverage for raw CRLF normalization in string, bytes,
  f-string, and t-string format-spec text.
- Preserved raw f-string brace handling: raw backslash-newline is consumed as
  literal text, but raw backslash before `{` or `}` still leaves the brace to
  participate in source-level f-string scanning.
- Kept `STRING`, f-string/t-string token triples, and `ERRORTOKEN` as `partial`
  because warning-channel behavior and broader tokenizer error parity are still
  tracked separately.

Completed in the numeric-literal diagnostics pass:

- Added `cpython_syntax_error_message_parity_subset` for CPython
  `Lib/test/test_grammar.py::test_bad_numerical_literals` diagnostic messages.
- Split MiniPython's generic `invalid number: ...` lexer errors into CPython-like
  categories for invalid binary, octal, hexadecimal, and decimal literals.
- Added CPython's leading-zero decimal literal guidance message for `012` and
  `0_7`.
- Added structured `LexError` spans to `cpython_bad_numerical_literals_subset`
  for representative invalid binary/octal digits, missing prefixed-integer
  digits, invalid underscores, invalid exponents, leading-zero decimal integers,
  and malformed imaginary literals.
- Added structured spans for representative adjacent-name and fraction-slash
  numeric boundary errors in `cpython_end_of_numerical_literals_subset`.
- Kept `NUMBER` as `partial` because exact source offsets for every
  adjacent-name/non-ASCII numeric literal edge are still not exhaustively migrated.

Completed in the invalid numeric token-stream expansion pass:

- Extended `cpython_tokenize_invalid_python_token_stream_subset` with CPython
  `Lib/test/test_tokenize.py::test_invalid_syntax` number-token shapes for
  trailing decimal and exponent underscores, invalid binary/octal digits after
  a valid prefix digit, invalid prefixed-literal underscore suffixes, and
  missing binary/octal/hex digits.
- Kept `NUMBER` as `partial` because the row still tracks the broader tokenizer
  surface, including exact warning/error parity for adjacent-name and
  non-ASCII numeric-literal edges.

Completed in the expanded number-keyword warning pass:

- Extended `cpython_numeric_literal_warning_subset` to cover the CPython
  `Lib/test/test_grammar.py::test_end_of_numerical_literals` keyword-boundary
  matrix for decimal, zero, trailing-dot float, exponent float, imaginary,
  binary, octal, and hexadecimal literals across `and`, `or`, `in`, `not in`,
  conditional `if`/`else`, comprehension-like `for`, and `is` suffixes.
- Extended `cpython_end_of_numerical_literals_subset` with structured
  diagnostics for `0or`, hexadecimal adjacent-name errors such as `0xfelse`
  and `0xfspam`, and the hexadecimal fraction-slash form `0xf⁄7`.
- Kept `NUMBER` as `partial` because broader tokenizer parity still includes
  source-encoding behavior and additional CPython tokenizer offset families
  beyond the grammar-suite numerical literal boundary tests.

Completed in the tokenizer warning-channel pass:

- Added `source_warnings()` as a lexer-only Rust API for collecting warning
  messages without changing `run_source()` execution behavior.
- Added `cpython_numeric_literal_warning_subset` for CPython-style
  SyntaxWarning messages around number-keyword boundaries such as `1and`,
  `1else`, `1.is`, `1jand`, and prefixed integer forms.
- Added `cpython_string_escape_warning_subset` for invalid string/bytes escapes
  and out-of-byte-range octal escapes, while preserving raw string/bytes
  behavior with no warning.
- Added `cpython_f_string_escape_warning_subset` from
  `Lib/test/test_fstring.py::test_backslashes_in_string_part` and
  `::test_fstring_backslash_before_double_bracket`, covering invalid f-string
  escapes such as `\g`, `\{`, and `\}` while preserving raw f-string behavior
  with no warning.
- Added `source_warning_diagnostics()` and
  `cpython_string_escape_warning_location_subset`, migrating CPython's
  `SyntaxWarning` category plus start/end line and column checks for multiline
  invalid string and bytes escapes.
- Added `run_source_with_warnings_as_errors()` and
  `source_warning_as_error_diagnostic()` so Rust tests can model CPython warning
  filters that promote tokenizer `SyntaxWarning`s into errors.
- Extended numeric, string, bytes, and f-string warning tests to assert the first
  promoted warning message and span while keeping the older `run_source()` path
  warning-tolerant.
- Kept `NUMBER` and `STRING` as `partial` because MiniPython still does not
  model filenames or every tokenizer warning edge.

Completed in the explicit line-joining tokenizer pass:

- Added lexer support for CPython-style explicit line joining outside string
  literals: `\\\n` and `\\\r\n` are consumed without emitting a `NEWLINE`, and
  indentation on the continued physical line is ignored.
- Added `cpython_tokenize_explicit_line_joining_subset`, adapted from
  `Lib/test/test_tokenize.py::test_backslash_continuation` and
  `Lib/test/test_syntax.py::test_invalid_line_continuation_error_position`.
- Extended it with CPython
  `Lib/test/test_tokenize.py::CTokenizeTest.test_continuation_lines_indentation`.
  Continuation-only physical lines no longer create unrelated `INDENT` tokens;
  they still allow a pending post-colon block indent and preserve the current
  block indentation across consecutive continuation-only lines.
- Extended the same subset with token-kind/text parity checks against CPython's
  no-continuation spellings. A whitespace-plus-backslash physical line followed
  by a blank physical line now suppresses the blank line's logical `NEWLINE`,
  matching CPython's tokenizer output for the covered continuation-only case.
- Covered invalid ordinary-tokenizer backslash forms such as `a = 3 \ 4` and
  `1,\#`, which now report `unexpected character after line continuation
  character` instead of a generic unexpected-character lexer error.
- Extended the invalid continuation coverage with CPython
  `Lib/test/test_syntax.py::test_invalid_line_continuation_left_recursive`,
  including attribute-like prefixes before the backslash and a final
  backslash-newline at EOF. MiniPython now rejects that final explicit line
  continuation in the lexer instead of executing the joined prefix.
- Added `LexError` and `source_lex_error_diagnostic()` so this subset can assert
  the CPython-style start/end line and column for invalid line continuations
  without changing the older public `lex error: ...` string path.
- Extended `cpython_tokenize_implicit_line_joining_subset`, adapted from CPython
  `Lib/test/test_tokenize.py::test_newline_after_parenthesized_block_with_comment`.
  MiniPython still does not expose COMMENT/NL tokens separately, but the token
  stream now directly asserts that bracketed physical newlines and comments do
  not produce logical `Newline` tokens, while the newline after the closing
  bracket does.
- Extended continuation coverage with CPython
  `Lib/test/test_tokenize.py::test_continuation`,
  `::test_backslash_continuation`, and
  `Lib/test/test_syntax.py::test_continuation_bad_indentation`. The executable
  subset now checks that bracketed continuations preserve tuple/list/dict
  semantics, a backslash in a comment does not join the following line, and a
  backslash split that retreats to a non-existent indentation level is rejected.
- Added `cpython_tokenize_error_token_subset` from
  `Lib/test/test_tokenize.py` TokenError/ERRORTOKEN cases, covering rejection of
  invalid non-identifier characters such as `€`, non-breaking-space input,
  unmatched and mismatched bracket input, cross-line mismatched closing brackets,
  EOF-in-multiline input from bare and populated open brackets, line-continuation
  EOF, embedded NUL source, and too-deep bracket nesting. The `€` and `]` cases
  are also in the differential CPython/MiniPython rejection harness.
- Added structured lexer spans for invalid characters such as `€`, the CPython
  `import ä £` shape from `Lib/test/test_syntax.py`, and embedded NUL source.
- Added the CPython-style 200-level bracket nesting limit used by the tokenizer
  for `(`, `[`, and `{`; the 201st nested opener now rejects with
  `too many nested parentheses`.
- Kept `ERRORTOKEN` as `partial` because structured coverage is still limited to
  selected invalid line-continuation and invalid-character cases; file names,
  source text, and the broader tokenizer error-token surface are still not modeled.

Completed in the tokenizer max-indent pass:

- Added CPython's `MAXINDENT` behavior from
  `Lib/test/test_tokenize.py::test_max_indent`: 99 nested indentation levels are
  accepted, while the 100th nested level is rejected during lexing.
- Implemented a lexer indentation-stack limit matching CPython's effective
  maximum stack size, including the base indentation level.
- Added structured lexer spans for unmatched dedent and tab-indentation
  diagnostics in `cpython_tokenize_unmatched_indentation_subset`.

Completed in the tokenizer tab-indentation pass:

- Migrated CPython tab-indentation behavior from
  `Lib/test/test_tokenize.py::test_tabs` and
  `::test_indentation_semantics_retained`: tab indentation is now accepted and
  expanded to eight-column stops for indentation levels.
- Added CPython-style alternate-column tracking so visually equal indentation
  made from incompatible tab/space mixtures is rejected with `inconsistent use
  of tabs and spaces in indentation`.
- Extended `cpython_tokenize_unmatched_indentation_subset` with executable tab
  indentation and inconsistent tab/space rejection cases.

Completed in the tokenizer formfeed-whitespace pass:

- Added `cpython_tokenize_formfeed_whitespace_subset`, grounded in CPython
  `Parser/lexer/lexer.c` formfeed handling. MiniPython now treats `\x0c` as
  ordinary whitespace between tokens and as a leading-whitespace character that
  resets indentation columns.
- Covered executable parity for `x\x0c=1` and leading-formfeed indentation after
  a compound header, plus tokenizer span coverage for the skipped formfeed
  between `NAME` and `EQUAL`.

Completed in the f-string comment migration pass:

- Extended `cpython_f_string_basic_subset` with the remaining executable
  `Lib/test/test_fstring.py::test_comments` cases where triple-quoted f-string
  literal text preserves `#` lines, while comments inside replacement fields are
  ignored and multiline expressions still evaluate correctly.
- Kept `STRING` and the f-string token rows as `partial` because MiniPython still
  uses a collapsed `Token::FString` representation and does not expose CPython's
  token trio or source-location metadata for all f-string parts.

Completed in the f-string many-expressions pass:

- Added `cpython_f_string_many_expressions_subset` from
  `Lib/test/test_fstring.py::test_many_expressions`, covering 250-259
  replacement fields in one f-string, adjacent concatenation of 256 large
  f-strings, a trailing nested-width format spec, and 1024 repeated adjacent
  f-string/plain-string groups.
- The Rust tests compare rendered lengths instead of printing the full large
  strings, preserving the CPython part-count and concatenation semantics while
  keeping test output small.
- Kept `STRING` and the f-string token rows as `partial` for the same collapsed
  token-representation reason.

Completed in the f-string format-spec expression pass:

- Added `cpython_f_string_format_specifier_expressions_subset` from
  `Lib/test/test_fstring.py::test_format_specifier_expressions`, focused on
  nested replacement fields that construct `#10x` and `-#10x` format specs.
- Added the same supported nested-format-spec expression slice to
  `tests/cpython_diff.rs`, so CPython and MiniPython outputs are compared for
  dynamically assembled alternate-form hexadecimal specs.
- Extended the VM's mini format-spec parser with sign parsing and alternate-form
  `#` support for hexadecimal integer formatting, including Python-style
  negative hexadecimal output such as `-0xa` instead of Rust's two's-complement
  formatting.
- Left Decimal coverage for a later object-protocol/runtime pass.

Completed in the custom `__format__` pass:

- Added `cpython_format_builtin_and_custom_dunder_format_subset` from
  `Lib/test/test_fstring.py::test_custom_format_specifier`,
  `::test_side_effect_order`, `::test__format__lookup`, and
  `Lib/test/test_builtin.py::test_format`.
- Added a CPython/MiniPython differential parity case for the same supported
  subset.
- Added the `format()` builtin for one- and two-argument calls and routed both
  `format(value, spec)` and f-string `FormatValue` execution through VM-level
  class `__format__` lookup. Instance attributes named `__format__` remain
  visible through ordinary attribute access, but do not override the formatting
  protocol lookup, matching CPython's type-level special-method behavior.
- Added first-pass `object.__format__` inheritance for ordinary instances:
  empty specs return the object's string form, non-string specs raise
  `TypeError`, and non-empty specs on classes without a custom `__format__`
  raise `TypeError` naming the target type's `__format__`.
- The remaining formatting gaps are Decimal/general numeric formatting, exact
  object/class repr parity, complete format mini-language flags, subclassed
  `str` format specs, and exact Argument Clinic error wording.

Completed in the string format/format_map method pass:

- Added `cpython_string_format_and_format_map_subset`, adapted from
  `Lib/test/test_str.py::test_format` and `::test_format_map`, covering literal
  rendering, escaped braces, positional/manual and automatic fields, keyword
  fields, `format_map` mapping fields, simple attribute/item lookup,
  conversions, and supported mini-format specs.
- Added a CPython/MiniPython differential parity case for the same supported
  subset.
- Added first-pass VM support for `str.format()` and `str.format_map()` by
  parsing replacement fields and reusing the existing VM `format()` / f-string
  formatting path for value rendering.

Completed in the format grouping-option error pass:

- Added `cpython_format_grouping_option_errors_subset` from
  `Lib/test/test_format.py` grouping-option error tests for duplicate `,,`,
  duplicate `__`, and mixed `,_` / `_,` markers, including the float-format
  variants `.,_f` and `._,f`.
- Added CPython/MiniPython rejection parity cases for the same supported
  invalid format specs.
- Extended the VM's mini format-spec parser to recognize grouping-option slots
  and raise CPython-style `ValueError` diagnostics before falling through to
  generic unsupported-format handling.

Completed in the format grouping rendering pass:

- Added `cpython_format_grouping_rendering_subset` and a differential
  CPython/MiniPython parity case for decimal integer comma/underscore grouping,
  width padding after grouping, f-string grouping specs, fixed-point float
  comma grouping, underscore-grouped hexadecimal formatting, and CPython's
  `ValueError` for comma grouping with `x`.
- Extended VM numeric formatting so decimal integer and fixed-point float
  grouping use three-digit groups, while underscore-grouped hexadecimal
  formatting groups digits in fours after any base prefix. Bool values now use
  the same numeric formatting path for non-empty numeric format specs.

Completed in the integer format-code and zero-alignment pass:

- Added `cpython_format_integer_codes_and_zero_alignment_subset` and a
  differential CPython/MiniPython parity case for binary, octal, and character
  format codes, alternate-form `0b` / `0o` prefixes, underscore grouping for
  binary and octal, sign-aware zero fill, explicit `=` alignment, base-prefix
  zero fill for hex and octal, grouped zero padding, and representative
  CPython `ValueError` cases for invalid grouping, alternate character format,
  and integer precision.
- Extended the VM format-spec parser to accept `=` alignment and the `0` flag,
  and extended sign/prefix-aware padding so zeros land after signs and base
  prefixes rather than before them.

Completed in the tokenizer null-byte error pass:

- Added CPython-derived null-byte coverage from
  `Lib/test/test_tokenize.py::test_nul_in_first_coding_line` /
  `::test_nul_in_second_coding_line`. MiniPython does not model source-encoding
  detection, but now rejects `\0` during lexing with CPython's
  `source code cannot contain null bytes` message and records the offending
  source span in `cpython_tokenize_error_token_subset`.
- Kept `ERRORTOKEN` as `partial` because MiniPython still reports a collapsed
  lexer error instead of exposing CPython's full tokenize token stream.

Completed in the tokenizer non-printable-character pass:

- Added CPython-derived invisible-character coverage from
  `Lib/test/test_syntax.py::test_invisible_characters` and CPython
  `Parser/lexer/lexer.c`'s `invalid non-printable character U+%04X`
  tokenizer path.
- MiniPython now rejects non-printable control characters such as U+0017 and
  U+0001, plus non-standard whitespace such as U+00A0, with CPython-style
  `invalid non-printable character U+....` diagnostics before parser execution.
- Extended `cpython_tokenize_error_token_subset` with source-span checks for
  the CPython `print\x17("Hello")` and `with(0,,):\n\x01` shapes.

Completed in the tokenizer multiline-bracket error pass:

- Migrated more of `Lib/test/test_tokenize.py`'s tokenizer `TokenError`
  coverage by replacing MiniPython's scalar bracket-depth counter with a
  bracket stack. The lexer now rejects unclosed bracketed input such as `(1\n`,
  `[1`, and `{1: 2` as `EOF in multi-line statement`, and it reports unmatched
  closing brackets before parser execution.
- Added focused lexer unit coverage plus source-span checks in
  `cpython_tokenize_error_token_subset`; the latest span pass adds CPython
  `test_invalid_syntax` shapes `(1+2]`, `(1+2}`, `{1+2]`, a mismatched bracket
  on the next physical line, bare `(` / `[` / `{` EOF cases, non-breaking-space
  input, and trailing backslash EOF.
- Extended `cpython_tokenize_error_token_subset` with the CPython
  `test_invalid_syntax` single-quoted multiline f-string form
  `f'__{\n    x:d\n}__'`, preserving the tokenizer-level requirement that this
  source is rejected before execution.
- Kept `ERRORTOKEN` as `partial` because MiniPython still collapses these cases
  into `LexError` instead of exposing CPython's exact token stream and token
  categories.

Completed in the string invalid-escape ASCII table pass:

- Added `cpython_string_invalid_escape_ascii_table_subset`, adapted from
  `Lib/test/test_string_literals.py::test_eval_str_invalid_escape` and
  `::test_eval_bytes_invalid_escape`. It now checks every CPython-invalid
  one-character ASCII escape from 1 through 127 for ordinary strings and bytes
  literals, including warning-as-error behavior.
- Kept `STRING` as `partial` because MiniPython still exposes a collapsed token
  model and does not yet mirror every CPython tokenizer diagnostic and token
  stream detail.

Completed in the invalid-Python token stream pass:

- Added `tokenize_with_spans()` as a tokenizer-oriented API separate from the
  stricter compile/parser lexer path.
- Migrated CPython `Lib/test/test_tokenize.py` invalid-Python token-stream
  shapes where tokenization can still expose useful tokens before parsing or
  compilation rejects the source: `2sin(x)`, leading-zero tokenizer forms such
  as `01234`, `0_7`, and `09_99`, invalid decimal underscore/exponent shapes
  such as `1_`, `1__0`, `1_.4`, `1._4`, `._5`, `1e_1`, `1e+`, and `1.4j_`,
  invalid binary/octal/hex prefixed literal shapes such as `0b2`, `0b10102`,
  `0o8`, `0xg`, `0x1g`, and `0b1_`, plus `); x` and `(]`.
- Tokenize mode now emits a synthetic final `NEWLINE` before `EOF` for
  non-newline-terminated token streams, matching the CPython tokenizer shape
  more closely while leaving parse mode unchanged.
- Extended tokenizer-mode final-newline handling with
  `cpython_tokenize_trailing_space_without_newline_subset`, adapted from CPython
  `Lib/test/test_tokenize.py::test_newline_and_space_at_the_end_of_the_source_without_newline`.
  A final whitespace-only physical line is now preserved as a synthetic newline,
  and `EOF` moves to the following line in tokenize mode.
- Extended the same subset with CPython
  `Lib/test/test_tokenize.py::test_comment_at_the_end_of_the_source_without_newline`.
  MiniPython still does not expose COMMENT tokens, but tokenizer mode now emits
  the empty non-logical newline marker before EOF for a final comment-only
  physical line, matching CPython's token-boundary behavior for the covered
  shape.
- Kept compile execution strict for the same source strings, so this pass moves
  MiniPython closer to CPython's tokenizer/compile split without weakening
  normal `run_source()` rejection behavior.

Completed in the valid NUMBER token stream pass:

- Added `cpython_tokenize_valid_number_token_stream_subset`, adapted from
  CPython `Lib/test/test_tokenize.py::TokenizeTest.test_int`, `::test_long`,
  and `::test_float`.
- Covered raw token text and spans for the full migrated integer, long-integer,
  and float source shapes from those CPython tests: hexadecimal, binary, octal,
  decimal, unary/binary operator boundaries, large integer, leading-dot float,
  trailing-dot float family, and signed-exponent float source shapes through
  `tokenize_with_spans()`. The span subset now also includes CPython's
  trailing-dot, uppercase exponent, and large exponent float spellings,
  `314159.`, `3E123`, and `3.14e159`.
- Added `cpython_tokenize_underscore_number_token_stream_subset`, adapted from
  CPython `Lib/test/test_tokenize.py::TokenizeTest.test_underscore_literals`.
  It verifies that valid underscore numeric literals preserve their full raw
  NUMBER token text, while invalid underscore numeric spellings are not exposed
  as a complete NUMBER token in tokenizer mode. The CPython-extra tokenizer
  forms `0_7` and `09_99` remain accepted as complete tokenizer NUMBER text
  while compile mode rejects them.

Completed in the string token span pass:

- Added `cpython_tokenize_string_span_subset`, adapted from
  `Lib/test/test_tokenize.py` STRING position cases. It verifies source text and
  1-based start/end spans for empty quoted strings, ordinary single- and
  double-quoted string expressions, raw string prefix matrix spellings,
  `u`/`U` string prefixes, `b`/`B` bytes prefixes, embedded quote payloads,
  single- and double-quoted `br`/`rb` raw-bytes prefix matrix spellings, split
  string/name/string tokenization, multiline line-continuation strings,
  unicode-prefixed line-continuation strings, triple-quoted line-continuation
  strings, unicode-prefixed triple-quoted line-continuation strings,
  single/triple-quoted raw bytes strings, escaped CRLF text inside a string
  token, and CPython's indented non-ASCII triple-quoted string span case through
  `tokenize_with_spans()`.
- Extended the same subset with CPython's `test_string_concatenation` token
  shape: adjacent same-line string literals remain separate tokenizer `String`
  tokens (`'' ''`) and are not merged until the parser's literal-concatenation
  layer.
- Kept `STRING` as `partial` because MiniPython still collapses CPython's
  tokenizer string family into parsed `String` / `Bytes` / `FString` / `TString`
  payload tokens instead of exposing every CPython token-stream detail.

Completed in the f-string token span pass:

- Re-exported `TokenFStringPart` and `TokenFStringConversion` from the public
  crate API so integration tests can assert tokenizer-oriented f-string part
  shapes without reaching into the private lexer module.
- Added `cpython_tokenize_f_string_span_subset`, adapted from CPython
  `Lib/test/test_tokenize.py` FSTRING position cases. It verifies the collapsed
  `Token::FString` source span and internal literal/expression parts for plain
  f-strings, raw-prefix f-strings, escaped braces, conversion fields, `!r`
  conversion fields, debug expressions, format specs, single-line triple-quoted
  f-strings, multiline triple-quoted f-strings, ordinary line continuations, and
  raw f-string line continuations.
- Extended the same span test with CPython's multiline non-ASCII f-string
  cases, including Polish text in a triple-quoted f-string and emoji text around
  a replacement field.
- Extended it again with CPython's gh-139516 tokenizer case where a replacement
  field contains a lambda argument, a non-ASCII string literal, and an implicit
  newline before `)}`.
- Extended it again with CPython's nested f-string tokenizer case
  `f"""{f'''{f'{f"{1+1}"}'}'''}"""`, covering replacement-field source that is
  itself a triple-quoted f-string containing nested f-strings.
- Added the same nested f-string source to `tests/cpython_diff.rs` so the
  executable result remains checked against the CPython oracle.
- Kept `FSTRING_START`, `FSTRING_MIDDLE`, and `FSTRING_END` as `partial`
  because MiniPython still exposes one collapsed f-string token instead of
  CPython's exact tokenize token trio.

Completed in the f-string split-token tokenizer pass:

- Added `Token::FStringStart`, `Token::FStringMiddle`, `Token::FStringEnd`,
  and `tokenize_cpython_with_spans()` as a CPython-tokenize view layered on top
  of the parser-oriented tokenizer. The parser path still receives collapsed
  `Token::FString` values.
- Added `cpython_tokenize_f_string_split_token_subset`, adapted from CPython
  `Lib/test/test_tokenize.py` FSTRING token-stream examples. It covers split
  start/middle/end tokens for `f"abc"`, raw-prefix replacement fields,
  escaped-brace middles with `!r` conversion tokens, and literal format-spec
  middles.
- Extended the same split-token subset with CPython's nested f-string tokenizer
  case `f"""{f'''{f'{f"{1+1}"}'}'''}"""`, proving the compatibility view
  recursively expands f-strings found inside replacement-field expressions.
- Extended the same split-token subset with nested format-spec replacement
  fields from CPython `Lib/test/test_fstring.py::test_format_specifier_expressions`,
  covering `f"{value:{width}.{prec}f}"` as split outer expression, inner
  replacement fields, and literal format-spec middles.
- Extended the same split-token subset with more CPython
  `Lib/test/test_tokenize.py` FSTRING stream shapes: escaped braces around a
  replacement field, debug `=` followed by padding before the closing brace, and
  a multiline triple-quoted replacement field whose expression indentation must
  not leak synthetic `INDENT` / `DEDENT` tokens into the f-string token stream.
- Extended the same split-token subset with CPython's multi-field format/debug
  source `f'some words {a+b:.3f} more words {c+d=} final words'` and the
  multiline debug-expression source `f'''{\n3\n=}'''`, covering adjacent
  format-spec middles, debug `=`, literal middles between fields, and physical
  newline tokens inside debug replacement fields.
- Extended the same subset with the CPython multiline literal and non-ASCII
  tokenizer cases from `Lib/test/test_tokenize.py`, including `None`
  replacement fields, emoji middle text, and gh-139516 cross-line expression
  tokenization.
- Extended the same subset with CPython's deeper multiline format-spec middle
  case `f'''__{\n    x:a\n    b\n     c\n      d\n}__'''`, preserving the
  whole post-colon literal middle across unevenly indented physical lines.
- Added CPython-tokenize-view synthesis for physical `NL` tokens inside
  bracketed replacement-field expressions, so a newline inside `f(a=lambda:
  'à'\n)` is preserved in the split-token stream instead of being suppressed by
  the parser-oriented tokenizer.
- Kept `FSTRING_START`, `FSTRING_MIDDLE`, and `FSTRING_END` as `partial`
  because the split-token view now covers the first representative CPython
  stream shapes, but not yet the full comment tokenizer surface.

Completed in the t-string token span pass:

- Added `cpython_tokenize_t_string_span_subset`, adapted from CPython
  `Lib/test/test_tstring.py` t-string surface cases and the tokenizer span
  strategy already used for f-strings. It verifies the collapsed
  `Token::TString` source span and internal literal/interpolation parts for
  literal-only t-strings, ordinary and multiple interpolations, expression
  source preservation, `rt`/`tr` raw-prefix t-strings, `!s`/`!r`/`!a`
  conversions, debug fields with and without format specs, ordinary and nested
  format specs, and triple-quoted multiline t-strings.
- Kept `TSTRING_START`, `TSTRING_MIDDLE`, and `TSTRING_END` as `partial`
  because MiniPython still exposes one collapsed t-string token instead of
  CPython's exact tokenize token trio.

Completed in the t-string split-token tokenizer pass:

- Added `Token::TStringStart`, `Token::TStringMiddle`, and
  `Token::TStringEnd` to the same `tokenize_cpython_with_spans()` compatibility
  view used for f-strings. The parser path still receives collapsed
  `Token::TString` values.
- Added `cpython_tokenize_t_string_split_token_subset`, covering split
  start/middle/end tokens for ordinary t-string interpolation and raw-prefix
  t-string interpolation with `!r` conversion plus a literal format spec, and
  nested format-spec replacement fields in `t'{value:{width}.{prec}f}'`.
- Extended the same split-token subset with CPython `test_tstring.py`
  interpolation shapes for multiple adjacent interpolations and raw `rt`
  prefixes, covering literal middles between fields and raw literal tails after
  a replacement field.
- Extended the same t-string split-token subset with the debug-padding and
  multiline expression/format-spec cases now covered for f-strings, using the
  same no-synthetic-`INDENT` / `DEDENT` rule for replacement-field expressions.
- Extended the same t-string split-token subset with t-string counterparts for
  multiline literal, non-ASCII/emoji, and gh-139516 cross-line expression token
  shapes.
- Extended the same t-string split-token subset with the CPython f-string deep
  multiline format-spec shape, preserving the corresponding t-string middle
  across unevenly indented physical lines.
- Kept `TSTRING_START`, `TSTRING_MIDDLE`, and `TSTRING_END` as `partial`
  because the split-token view is still a representative subset of the full
  CPython tokenizer surface.

Completed in the t-string interpolation semantics pass:

- Extended `cpython_t_string_basic_subset`, adapted from CPython
  `Lib/test/test_tstring.py`, with multiple interpolations, function-call
  expressions, attribute and method-call expressions, dictionary subscript
  expressions, whitespace-preserving debug fields, comments inside replacement
  fields, and nested Template values.
- T-string debug replacement fields now accept CPython-style padding after the
  debug `=`, preserving that padding in the generated Template literal text and
  ignoring comments before the closing brace.

Completed in the t-string nested/runtime error pass:

- Added `cpython_t_string_nested_template_and_runtime_error_subset`, adapted
  from CPython `Lib/test/test_tstring.py::test_nested_templates` and
  `::test_runtime_errors`.
- Covered nested Template values inside outer Interpolation objects, including
  inner `strings`, inner interpolation `value`, `expression`, `conversion`, and
  empty `format_spec` metadata.
- Added explicit missing-variable runtime coverage for t-string replacement
  fields while building the Template value.

Completed in the f-string debug conversion migration pass:

- Extended `cpython_f_string_debug_expression_subset`, adapted from CPython
  `Lib/test/test_fstring.py::test_debug_conversion`, with whitespace around the
  debug `=`, debug `!s`/`!r`/`!a` conversions followed by alignment specs,
  nested debug f-strings, and debug expressions inside format specs.
- Extended the same subset with representative
  `Lib/test/test_fstring.py::test_debug_expressions_are_raw_strings` cases so
  ordinary escaped string, raw string, and bytes literals inside debug
  replacement fields preserve their source labels while still evaluating to the
  same runtime values.
- Adjusted string `repr()` rendering to match CPython's quote selection for
  values containing a single quote but no double quote, so `repr("'")` uses
  double quotes.
- Added the CPython f-string walrus disambiguation case from
  `Lib/test/test_fstring.py::test_walrus`: `f'{x:=10}'` remains a format-spec
  expression while `f'{(x:=10)}'` performs the assignment expression.

Completed in the f-string debug comparison regression pass:

- Extended `cpython_f_string_debug_expression_subset` with CPython
  `Lib/test/test_fstring.py::test_gh129093`, covering debug replacement fields
  whose expression contains `==`, `!=`, chained comparisons, and nested
  f-strings before the final debug `=`.
- Added the Python-3.9-compatible portion of the same regression to
  `tests/cpython_diff.rs`, so MiniPython output is compared directly with the
  local CPython oracle for comparison debug fields.
- Added lexer-level coverage so comparison operators inside f-string
  replacement expressions are not mistaken for the debug `=` delimiter.

Completed in the f-string scope and format lookup pass:

- Added `cpython_f_string_scope_and_format_lookup_subset`, adapted from
  CPython `Lib/test/test_fstring.py::test_multiple_vars`, `::test_closure`,
  `::test_arguments`, `::test_locals`, `::test_missing_variable`, and
  `::test_missing_format_spec`.
- Covered local variables, closure reads, global variables used by function
  bodies, dynamic width format specs, missing-name failures inside replacement
  fields, and custom `__format__` receiving both non-empty and empty specs.
- Added accepted CPython-output parity for the Python-3.9-compatible scope and
  format cases, plus rejection parity for the missing-variable case.

Completed in the f-string yield-expression migration pass:

- Added `cpython_f_string_yield_expression_subset`, adapted from CPython
  `Lib/test/test_fstring.py::test_yield` and `::test_yield_send`, so f-string
  replacement fields now have regression coverage for generator suspension and
  resume values.
- Added the same source shape to `tests/cpython_diff.rs` so ordinary CPython
  remains the oracle for the supported f-string/yield subset.

Completed in the f-string triple-quoted expression migration pass:

- Added `cpython_f_string_triple_quoted_expression_subset`, adapted from CPython
  `Lib/test/test_fstring.py::test_expressions_with_triple_quoted_strings`, so
  replacement fields now cover triple-quoted string literals and adjacent
  string-literal concatenation inside f-string expressions.
- Added the same source shape to `tests/cpython_diff.rs` to keep this parser
  behavior checked against the CPython oracle.

Completed in the f-string/t-string missing-expression whitespace pass:

- Extended `cpython_invalid_f_string_syntax_subset`, adapted from CPython
  `Lib/test/test_fstring.py::test_missing_expression`, with whitespace-only
  replacement fields before `}`, `=`, `!`, and `:` using CPython's accepted
  expression-whitespace set: space, tab, newline, carriage return, and
  formfeed. The test now includes the simple empty-field forms, nested
  format-spec empty replacement fields, and the invalid-conversion variants
  CPython reports as missing expressions rather than conversion errors.
- Extended `cpython_invalid_t_string_syntax_subset`, adapted from CPython
  `Lib/test/test_tstring.py::test_syntax_errors` and the same replacement-field
  whitespace semantics, so t-strings reject whitespace-only fields before `}`,
  `=`, `!`, and `:` on the same path as f-strings.
- Changed empty-expression detection so non-breaking space is not treated as
  generic Rust/Unicode whitespace inside an interpolated string replacement
  field. It now
  reaches expression lexing and reports `invalid non-printable character
  U+00A0`, matching CPython's tokenizer behavior for this slice.

Completed in the integer method/property pass:

- Added `int.bit_length()` and `int.bit_count()` runtime support for small
  integers, arbitrary-precision `BigInt` values, and `bool` values.
- Added `cpython_integer_bit_methods_subset`, adapted from
  `Lib/test/test_long.py::test_bit_length` and `::test_bit_count`, covering
  zero, positive, negative, large power-of-two values, and bool receivers.
- Added `int.numerator`, `int.denominator`, `int.real`, `int.imag`,
  `int.conjugate()`, and `int.as_integer_ratio()` for the same receiver set.
  `cpython_integer_ratio_and_component_methods_subset` covers ordinary values,
  arbitrary-precision values, negative values, and bool receivers.
- Added a differential CPython/MiniPython smoke case for `bit_length()`. The
  smoke case avoids `bit_count()` because the default system `python3` oracle
  used by this test can be older than the CPython version that introduced it.

Completed in the float method/property pass:

- Added `float.real`, `float.imag`, `float.conjugate()`,
  `float.is_integer()`, and `float.as_integer_ratio()` runtime support for the
  supported float subset.
- Added `cpython_float_ratio_and_component_methods_subset`, adapted from
  `Lib/test/test_float.py::test_is_integer` and `::test_floatasratio`, covering
  finite exact ratios, signed values, integer-valued floats, and the CPython
  `OverflowError` / `ValueError` behavior for infinities and NaN.
- Added a differential CPython/MiniPython smoke case for the finite float
  component and ratio behavior.
- Runtime-error conversion now recognizes `OverflowError` as a real exception
  type, so `except Exception` catches infinite `float.as_integer_ratio()` cases
  like CPython.

Completed in the control-flow return stack-shape pass:

- Expanded `cpython_control_flow_inside_except_and_with_subset` from
  `Lib/test/test_compile.py::test_return_inside_except_block` and
  `::test_return_inside_with_block`.
- Added CPython/MiniPython differential parity for return from an `except`
  handler and return from inside a `with` body, including the requirement that
  `__exit__` runs before the return completes.
- Updated `return_stmt` and `except_block` coverage rows to point at the same
  migrated control-flow test.

Completed in the string prefix/suffix method pass:

- Added `cpython_string_startswith_endswith_subset`, adapted from
  `Lib/test/string_tests.py::test_startswith` and `::test_endswith`.
- Added runtime support for `str.startswith()` and `str.endswith()` including
  optional `start` / `end`, negative bounds, `None` bounds, tuple prefix/suffix
  arguments, empty tuple behavior, and representative `TypeError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  string prefix/suffix behavior, including the `start > end` empty-string edge
  case.

Completed in the string search method pass:

- Added `cpython_string_find_index_subset`, adapted from
  `Lib/test/string_tests.py::test_find`, `::test_rfind`, `::test_index`, and
  `::test_rindex`.
- Added runtime support for `str.find()`, `str.rfind()`, `str.index()`, and
  `str.rindex()` including optional `start` / `end`, `None` bounds, empty
  needle behavior, Unicode character indexes, `TypeError` for non-string
  needles, and `ValueError` for failed `index` / `rindex` searches.
- Added a CPython/MiniPython differential parity case for the same supported
  string search behavior.

Completed in the string count/case method pass:

- Added `cpython_string_count_case_subset`, adapted from
  `Lib/test/string_tests.py::test_count`, `::test_lower`, and `::test_upper`.
- Added runtime support for `str.count()`, `str.lower()`, and `str.upper()`
  including bounded non-overlapping counts, empty-needle edge cases,
  Unicode-aware case conversion, and representative `TypeError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  count and case-conversion behavior.

Completed in the string capitalize/title/swapcase/casefold method pass:

- Added `cpython_string_capitalize_title_swapcase_casefold_subset`, adapted from
  `Lib/test/string_tests.py::test_capitalize`, `::test_title`, `::test_swapcase`,
  and `Lib/test/test_str.py::test_casefold`.
- Added runtime support for `str.capitalize()`, `str.title()`, `str.swapcase()`,
  and `str.casefold()` including representative ASCII behavior, common Unicode
  expansions such as `ß`, `ﬁ`, and `µ`, combining iota case folding, contextual
  Greek final sigma lowercasing in `lower()`, `capitalize()`, and `title()`, and
  representative `TypeError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  capitalization, title, swapcase, and casefold behavior.

Completed in the string predicate method pass:

- Added `cpython_string_predicate_methods_subset`, adapted from
  `Lib/test/string_tests.py::test_islower`, `::test_isupper`,
  `::test_istitle`, `::test_isspace`, `::test_isalpha`, `::test_isalnum`,
  `::test_isdigit`, and `::test_isascii`, plus representative Unicode
  decimal/numeric checks from `Lib/test/test_str.py`.
- Added runtime support for `str.islower()`, `str.isupper()`, `str.istitle()`,
  `str.isspace()`, `str.isalpha()`, `str.isalnum()`, `str.isdigit()`,
  `str.isdecimal()`, `str.isnumeric()`, and `str.isascii()` including
  empty-string behavior, cased-word state, Unicode alphabetic/numeric
  predicates, the CPython `isascii()` alignment matrix from
  `Lib/test/string_tests.py`, and representative `TypeError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  string predicate behavior.

Completed in the string identifier/printable predicate pass:

- Added `cpython_string_identifier_printable_subset`, adapted from
  `Lib/test/test_str.py::test_isidentifier` and `::test_isprintable`.
- Added runtime support for `str.isidentifier()` and `str.isprintable()`,
  using Unicode XID identifier checks and Unicode general-category printable
  checks. MiniPython excludes surrogate-only CPython cases because its strings
  store Rust Unicode scalar values.
- Added a CPython/MiniPython differential parity case for the same supported
  identifier and printable-character predicate behavior.

Completed in the string expandtabs method pass:

- Added `cpython_string_expandtabs_subset`, adapted from
  `Lib/test/string_tests.py::test_expandtabs`.
- Added runtime support for `str.expandtabs()` including default, positional,
  and keyword `tabsize`, CR/LF/CRLF column resets, zero/negative tab sizes,
  bool-as-int tab sizes, and representative `TypeError` / `OverflowError`
  paths.
- Added a CPython/MiniPython differential parity case for the same supported
  tab-expansion behavior.

Completed in the string splitlines method pass:

- Added `cpython_string_splitlines_subset`, adapted from
  `Lib/test/string_tests.py::test_splitlines`.
- Added runtime support for `str.splitlines()` including CR, LF, CRLF,
  terminal-break behavior, `keepends` as positional or keyword argument,
  Unicode line separators, and representative `TypeError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  line-splitting behavior.

Completed in the string replace method pass:

- Added `cpython_string_replace_subset`, adapted from
  `Lib/test/string_tests.py::test_replace`.
- Added runtime support for positional `str.replace(old, new[, count])`
  including empty-string insertion, deletion, left-to-right non-overlapping
  replacement, bounded counts, Unicode text, and representative `TypeError`
  paths.
- Added a CPython/MiniPython differential parity case for the same supported
  replacement behavior.

Completed in the string removeprefix/removesuffix method pass:

- Added `cpython_string_remove_affix_subset`, adapted from
  `Lib/test/string_tests.py::test_removeprefix` and `::test_removesuffix`.
- Added runtime support for `str.removeprefix()` and `str.removesuffix()`
  including matching, non-matching, empty-affix, full-affix, Unicode, and
  representative `TypeError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  affix-removal behavior.

Completed in the string alignment/zfill method pass:

- Added `cpython_string_alignment_and_zfill_subset`, adapted from
  `Lib/test/string_tests.py::test_ljust`, `::test_rjust`, `::test_center`, and
  `::test_zfill`, plus non-BMP fill-character checks from `Lib/test/test_str.py`.
- Confirmed runtime support for `str.ljust()`, `str.rjust()`, `str.center()`,
  and `str.zfill()` including width handling, custom one-character fills,
  sign-aware zero filling, Unicode fill characters, and representative
  `TypeError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  alignment and zero-fill behavior.

Completed in the string split/rsplit method pass:

- Added `cpython_string_split_rsplit_subset`, adapted from
  `Lib/test/string_tests.py::test_split` and `::test_rsplit`.
- Added runtime support for `str.split()` and `str.rsplit()` including default
  whitespace splitting, explicit separators, right splitting, `maxsplit`,
  keyword `sep` / `maxsplit`, empty input, empty fields, and representative
  `TypeError` / `ValueError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  split behavior.

Completed in the string strip method pass:

- Added `cpython_string_strip_subset`, adapted from
  `Lib/test/string_tests.py::test_strip_whitespace` and `::test_strip`.
- Added runtime support for `str.strip()`, `str.lstrip()`, and `str.rstrip()`
  including default whitespace stripping, explicit `None`, character-set
  stripping, endpoint-only behavior, empty character-set behavior, and
  representative `TypeError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  stripping behavior.

Completed in the string partition/rpartition method pass:

- Added `cpython_string_partition_rpartition_subset`, adapted from
  `Lib/test/string_tests.py::test_partition` and `::test_rpartition`.
- Added runtime support for `str.partition()` and `str.rpartition()` including
  first/last separator search, not-found triples, Unicode separators, and
  representative `TypeError` / `ValueError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  partition behavior.

Completed in the string join method pass:

- Added `cpython_string_join_subset`, adapted from
  `Lib/test/string_tests.py::test_join`.
- Added runtime support for `str.join()` including list, tuple, generator,
  sequence-protocol objects, sequence objects with inaccurate `__len__`,
  repeated strings, and representative `TypeError` paths.
- Changed iterable collection to propagate iterator/generator exceptions instead
  of treating a raised exception as a completed sequence.
- Changed unsupported `+` operands to raise CPython-style `TypeError` messages so
  `join()` preserves generator error messages containing the `+` operator.
- Added a CPython/MiniPython differential parity case for the same supported join
  behavior.

Completed in the string maketrans/translate method pass:

- Added `cpython_string_maketrans_translate_subset`, adapted from
  `Lib/test/test_str.py::test_maketrans_translate`.
- Added runtime support for `str.translate()` with dictionary mappings from
  Unicode code point to `None`, integer code point, or replacement string.
- Added runtime support for `str.maketrans()` one-dict and three-string forms,
  including deletion entries, character-key normalization, non-ASCII
  replacements, invalid Unicode code point errors, and representative
  `TypeError` / `ValueError` paths.
- Added a CPython/MiniPython differential parity case for the same supported
  translation behavior.

Completed in the string/bytes codec method pass:

- Added `cpython_string_bytes_codec_subset`, adapted from
  `Lib/test/test_str.py::test_codecs` and
  `Lib/test/test_bytes.py::test_encoding` / `::test_decode`.
- Added runtime support for first-pass `str.encode()` and `bytes.decode()`,
  including default UTF-8, ASCII, Latin-1, UTF-16 little/big-endian spellings,
  keyword `encoding=` / `errors=`, and `strict` / `ignore` / `replace`
  behavior for the supported codecs.
- Extended the same runtime codec path to `cp1251` and `cp1252`, including
  CPython's undefined-byte behavior under `strict`, `ignore`, and `replace`.
- Extended constructors for `str(bytes, encoding)`, `bytes(str, encoding)`, and
  `bytearray(str, encoding)` over the same supported codec surface.
- Extended the same constructor slice with `object=`, `source=`, `encoding=`,
  and `errors=` keyword forms, including CPython-style rejection when encoding
  or errors are supplied without a string/bytes-compatible source.
- Added a CPython/MiniPython differential parity case for the same supported
  codec behavior.

Completed in the bytes hex/fromhex method pass:

- Added `cpython_bytes_hex_fromhex_subset`, adapted from
  `Lib/test/test_bytes.py::test_fromhex`, `::test_hex`, and the hex separator
  tests.
- Added runtime support for `bytes.fromhex()` and `bytearray.fromhex()` over
  string input, plus MiniPython support for bytes/bytearray input matching the
  newer local CPython source.
- Added runtime support for `bytes.hex()` and `bytearray.hex()` including
  optional `sep` and `bytes_per_sep` positional/keyword arguments, ASCII
  separator validation, positive right-grouping, and negative left-grouping.
- Added a CPython/MiniPython differential parity case for the Python 3.9-safe
  portion of the same supported hex behavior.

Completed in the divmod builtin pass:

- Added `cpython_divmod_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::test_divmod`.
- Added runtime support for the `divmod()` builtin over MiniPython's supported
  integer, bool, BigInt, and float numeric values.
- Reused the existing floor-division and modulo helpers so CPython's quotient
  and remainder sign rules stay aligned with `//` and `%`.
- Added CPython-style rejection for the covered arity, keyword, zero-division,
  and unsupported-operand paths.
- Added a CPython/MiniPython differential parity case for the supported
  `test_divmod` subset.

Completed in the round builtin pass:

- Added `cpython_round_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::test_round` and `::test_bug_27936`.
- Added runtime support for `round()` with integer and float inputs, omitted
  `ndigits`, `ndigits=None`, positive and negative integer `ndigits`, and the
  covered `number=` / `ndigits=` keyword forms.
- Implemented CPython's ties-to-even behavior for the supported integer and
  floating-point paths, preserving the CPython return-type rule that
  `round(float)` returns an `int` while `round(float, ndigits)` returns a
  `float`.
- Added first-pass `__round__` protocol dispatch for ordinary instances.
  Decimal/Fraction behavior, extreme `ndigits`, and exact Argument Clinic
  diagnostics remain future numeric-runtime work.
- Added first-pass one-argument `type(obj)` support for builtins, classes,
  instances, and exceptions so migrated tests can assert CPython return-type
  rules. The three-argument dynamic class constructor remains future
  object-model work.
- Added a CPython/MiniPython differential parity case for the supported
  `test_round` subset.

Completed in the type builtin pass:

- Added `cpython_type_builtin_subset` and `cpython_type_dynamic_class_subset`,
  adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_type`,
  `TestType::test_bad_args`, `TestType::test_type_name`, and the
  `Lib/test/test_descr.py` one-argument `type(instance)` behavior.
- Covered one-argument `type(obj)` over supported builtin values, `None`,
  builtin function objects, user classes, and user instances, including
  `type(type(obj)).__name__` for supported type objects.
- Added first-pass three-argument `type(name, bases, dict)` dynamic class
  construction, including string class names, tuple bases, dict namespaces,
  inherited class attributes, method binding from namespace functions,
  `__name__`, `__qualname__`, `__module__`, and covered bad-argument
  diagnostics.
- Added `cpython_type_name_qualname_subset`, adapted from
  `TestType::test_type_name` and `::test_type_qualname`, covering mutable
  class `__name__` and `__qualname__`, namespace `__qualname__`, namespace
  `__name__` staying a regular `__dict__` entry, and supported invalid
  assignment/type-constructor paths.
- Added `cpython_type_doc_and_firstlineno_subset`, adapted from
  `TestType::test_type_doc` and `::test_type_firstlineno`, covering class
  `__doc__` construction and assignment over the supported value set plus
  CPython's removal of `__firstlineno__` when `__module__` is reassigned.
- Added a CPython/MiniPython differential parity case for the supported
  `type()` subset.

Completed in the pow builtin pass:

- Added `cpython_pow_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::test_pow`.
- Added runtime support for the `pow()` builtin with positional and keyword
  `base`, `exp`, and `mod` arguments.
- Reused MiniPython's numeric power path for two-argument `pow()`, including
  CPython-style float results for negative integer exponents and
  `ZeroDivisionError` for zero raised to a negative exponent.
- Added integer modular exponentiation for three-argument `pow()`, including
  zero-modulus rejection, negative-modulus result normalization, and modular
  inverse handling for negative exponents.
- Added a CPython/MiniPython differential parity case for the supported
  `test_pow` subset.

Completed in the issubclass builtin pass:

- Added `cpython_issubclass_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_issubclass`.
- Added runtime support for `issubclass()` over MiniPython's supported class
  values, including direct user-class inheritance, tuple `classinfo`, the
  `bool`-as-`int` builtin relationship, implicit `object` inheritance, and the
  builtin exception hierarchy.
- Added CPython-style TypeError rejection for the covered non-class first
  argument, non-class `classinfo`, and missing-argument paths.
- Added a CPython/MiniPython differential parity case for the supported
  `test_issubclass` subset.

Completed in the isinstance builtin pass:

- Added `cpython_isinstance_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_isinstance`.
- Covered direct user-class instances, inherited user-class instances, tuple
  `classinfo`, the `bool`-as-`int` builtin relationship, implicit `object`
  matching, and builtin exception hierarchy checks.
- Tightened invalid `classinfo` and missing-argument paths so they become
  catchable CPython-style `TypeError` exceptions.
- Added a CPython/MiniPython differential parity case for the supported
  `test_isinstance` subset.

Completed in the attribute introspection builtin pass:

- Added `cpython_attribute_introspection_builtins_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_callable`, `::test_getattr`,
  `::test_hasattr`, `::test_setattr`, and `::test_delattr`.
- Added `cpython_instance_attribute_hooks_subset`, adapted from CPython
  descriptor/getattr-setattr hook coverage plus `test_hasattr` propagation
  checks.
- Covered `callable()` over builtins, functions, classes, bound methods, plain
  instances, and class-level `__call__`, including the CPython rule that
  instance-level `__call__` does not make an object callable.
- Covered `getattr`, `hasattr`, `setattr`, and `delattr` over module,
  instance, and class attributes, default values, custom attribute hooks, and
  TypeError/AttributeError paths for the supported subset.
- Tightened this builtin family's argument/type errors so they become catchable
  CPython-style `TypeError` exceptions.

Completed in the bound method metadata pass:

- Added `cpython_bound_method_metadata_subset`, adapted from
  `Lib/test/test_funcattrs.py::InstancemethodAttrTest` and
  `Lib/test/test_descr.py::ClassPropertiesAndMethods::test_methods`.
- Covered bound-method `__func__`, `__self__`, `__name__`, `__qualname__`,
  `__module__`, `__doc__`, readonly metadata assignment errors, function identity
  preservation through method binding, and CPython's class-body method aliasing
  behavior where an already-owned function keeps its original defining class.
- Added a CPython/MiniPython differential parity case for this supported
  bound-method metadata subset.

Completed in the function globals attribute pass:

- Added `cpython_function_globals_attribute_subset`, adapted from
  `Lib/test/test_funcattrs.py::FunctionPropertiesTest::test___globals__`.
- Exposed supported Python function `__globals__` as the live module globals
  mapping, added it to the function `dir()` surface, covered readonly
  assignment errors, and added a CPython/MiniPython differential parity case for
  definition-time global lookup through later `exec()` calls.

Completed in the function builtins attribute pass:

- Added `cpython_function_builtins_attribute_subset`, adapted from
  `Lib/test/test_funcattrs.py::FunctionPropertiesTest::test___builtins__`.
- Exposed supported Python function `__builtins__` as the globals-provided
  builtins mapping when present, otherwise as a MiniPython default builtins
  dictionary. The subset covers direct builtin lookup through the mapping,
  `dir(function)`, readonly assignment errors, and functions defined through
  `exec()` with a restricted `__builtins__` dictionary.
- This pass intentionally keeps the case out of `tests/cpython_diff.rs` because
  the default local `python3` oracle can predate `function.__builtins__`; use a
  newer `MINIPYTHON_CPYTHON` oracle before turning it into a cross-version
  differential case.

Completed in the bound method descriptor/repr pass:

- Added `cpython_bound_method_descriptor_and_repr_subset`, adapted from
  `Lib/test/test_descr.py::test_instance_method_get_behavior` and the stable
  method/receiver-name checks from `::test_bound_method_repr`.
- Implemented bound method `__get__` so rebinding a bound method keeps calling
  the original receiver, matching CPython's method descriptor behavior for the
  migrated case.
- Changed bound method repr from MiniPython's short `<bound method name>` form to
  include the method qualname and receiver repr, and added a differential parity
  smoke case using address-independent CPython checks.

Completed in the bound method identity pass:

- Added `cpython_bound_method_identity_subset`, covering CPython's distinction
  between a stored bound method object's identity and fresh method objects
  produced by repeated attribute access.
- Added a per-method identity token to `Value::BoundMethod`; `is` now recognizes
  the same stored method object, while ordinary method equality continues to
  compare the function and receiver.
- Preserved bound method identity through `method.__get__`, so
  `m.__get__(obj) is m` and `m.__get__(None, owner) is m` match CPython for the
  migrated case.

Completed in the hash/id builtin pass:

- Added `cpython_hash_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_hash`.
- Added first-pass runtime support for `hash()` over supported immutable values,
  including CPython's portable invariants for numeric equality, bool-as-int,
  string/bytes equality for ASCII payloads, tuple recursion, and function
  objects.
- Added class-level `__hash__` dispatch for ordinary instances, including
  integer/large-integer return values, `__hash__ = None`, and TypeError
  rejection when `__hash__` returns a non-integer or when mutable containers are
  hashed.
- Added a CPython/MiniPython differential parity case for the portable
  `test_hash` subset; exact process-randomized hash values are intentionally not
  compared.
- Added `cpython_id_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_id`, covering the portable
  return-type guarantee and stable identity relationships for aliases versus
  separate mutable objects without comparing process-specific address values.

Completed in the len builtin pass:

- Added `cpython_len_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_len`.
- Covered `len()` for supported builtins including strings, tuples, lists, and
  dictionaries.
- Tightened custom `__len__` handling so raised exceptions and invalid return
  values become catchable CPython-style `ValueError`, `TypeError`, and
  `OverflowError` paths for the supported subset.
- Added a CPython/MiniPython differential parity case for the supported
  `test_len` subset.

Completed in the repr builtin pass:

- Added `cpython_repr_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_repr` and
  `::test_repr_blocked`.
- Covered builtin scalar/container repr for the CPython subset plus recursive
  list and dict placeholders.
- Added runtime support for `repr(instance)` dispatch through class-level
  `__repr__`, including TypeError rejection for blocked or non-string-returning
  `__repr__` paths.
- Added a CPython/MiniPython differential parity case for the supported
  `test_repr` subset.

Completed in the integer digit-limit runtime pass:

- Added `cpython_int_max_str_digits_runtime_subset`, adapted from
  `Lib/test/test_int.py::IntStrDigitLimitsTests`.
- Covered runtime `sys.set_int_max_str_digits()` enforcement for decimal
  `int()` string parsing, `str(huge_int)`, `repr(huge_int)`, and recursive
  container repr containing oversized integers.
- Preserved CPython's sign/space padding and underscore digit-count behavior,
  and kept bases `2`, `4`, `8`, `16`, and `32` unlimited for `int(text, base)`.
- Added `cpython_int_max_str_digits_formatting_subset`, extending the same
  digit-limit behavior through `format()`, f-strings, `str.format()`, and
  old-style `%s` / `%r` / `%a` / `%d` / `%i` / `%u` formatting, while
  preserving unlimited hexadecimal formatting.

Completed in the iter/next builtin pass:

- Added `cpython_iter_next_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_iter` and `::test_next`.
- Covered iteration over tuples, lists, strings, ranges, user-defined iterators,
  and generators, including repeated exhaustion, `StopIteration`, and
  `next(iterator, default)`.
- Tightened `iter()` and `next()` arity/protocol failures so they become
  catchable CPython-style `TypeError` exceptions in the supported subset.
- Isolated custom `__iter__` calls so exceptions raised there do not consume the
  caller's active exception handler.
- Extended `iter(callable, sentinel)` with CPython
  `Lib/test/test_iter.py::test_iter_function_stop` and
  `::test_iter_function_concealing_reentrant_exhaustion` semantics: callable
  `StopIteration` now exhausts the callable iterator, and reentrant exhaustion
  of the same iterator prevents a stale non-sentinel return value from being
  yielded.
- Extended the same test with CPython `Lib/test/test_iter.py` sink-state
  semantics: supported iterators remain exhausted after completion, including
  sequence-protocol fallback iterators whose backing object later grows.

Completed in the enumerate/zip/map/filter/sorted builtin pass:

- Added `cpython_enumerate_zip_sorted_builtin_subset`, adapted from
  `Lib/test/test_enumerate.py::EnumerateTestCase` and
  `Lib/test/test_builtin.py::BuiltinTest::test_zip` / `::test_sorted`.
- Added `cpython_map_filter_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_map` and `::test_filter`.
- Covered iterator identity, sequence `__getitem__` fallback, custom
  `__iter__` / `__next__` objects, generator-backed iteration, keyword
  arguments, short zip termination, sorted `key` / `reverse`, and map/filter
  iterator consumption.
- Tightened arity and keyword failures for these builtins into catchable
  `TypeError` exceptions in the supported subset.
- Fixed iterable collection through `list()` so exceptions raised while
  advancing nested iterators jump to the active handler once instead of being
  raised a second time after the handler was already selected.

Completed in the container constructor/reversed pass:

- Added `cpython_sequence_constructor_builtins_subset`, adapted from
  `Lib/test/test_list.py::ListTest::test_basic`,
  `Lib/test/test_tuple.py::TupleTest::test_constructors`, and set constructor
  / literal coverage from `Lib/test/test_set.py`.
- Added `cpython_dict_constructor_update_fromkeys_subset`, adapted from
  `Lib/test/test_dict.py::DictTest::test_constructor`, `::test_update`, and
  `::test_fromkeys`.
- Added `cpython_reversed_builtin_subset`, adapted from
  `Lib/test/test_enumerate.py::TestReversed::test_simple` and CPython's dict
  reverse-iterator coverage.
- Covered list/tuple/set constructors over builtins, strings, generator
  expressions, existing tuple identity preservation, keyword rejection,
  non-iterable rejection, unhashable set elements, and exact `set.__init__`
  reinitialization behavior, including self-input clearing, arity/non-iterable
  errors, and partial mutation before an unhashable element error.
- Migrated first-pass exact `TestSet` behavior for constructor identity,
  literal equality, left-to-right literal insertion/evaluation order,
  unhashable `set` values, and `set.copy()` equality/type/identity.
- Added `cpython_set_mutation_methods_subset`, adapted from CPython
  `Lib/test/test_set.py::TestSet` mutation method cases for `clear`, `add`,
  `remove`, `discard`, `pop`, and `update`, including duplicate-add no-op,
  unhashable argument errors, missing-key `KeyError`, nested set/frozenset
  lookup equivalence, pop-until-empty behavior, and update result/error paths.
- Added `cpython_set_direct_lookup_and_keyerror_payload_subset`, adapted from
  CPython `TestSet` remove `KeyError` payload and direct set-key membership
  cases, including preserving the original missing key in `KeyError.args[0]`.
- Added `cpython_set_hash_exception_propagation_subset`, adapted from CPython
  `TestSet.test_unhashable_element`, covering propagation of non-`TypeError`
  exceptions raised by user-defined `__hash__` during set membership, `add`, and
  `discard`.
- Added `cpython_set_bad_comparison_errors_subset`, adapted from CPython
  `Lib/test/test_set.py::TestJointOps.test_badcmp`, covering hash-collision
  rich equality and `RuntimeError` propagation during set construction,
  `__contains__`, `add`, `discard`, and `remove`.
- Added `cpython_set_bad_comparison_algebra_errors_subset`, extending CPython
  bad-comparison coverage across set/frozenset equality and ordering checks,
  relation methods, algebra methods, and `&`, `|`, `-`, and `^` operators so
  every hash-collision path uses Python rich equality instead of Rust structural
  equality.
- Added `cpython_set_iterator_mutation_subset`, adapted from CPython
  `Lib/test/test_set.py::TestBasicOps.test_changingSizeWhileIterating` and
  `TestWeirdBugs.test_iter_and_mutate`, covering set iterator size-change
  invalidation plus the non-crashing clear/refill-to-original-size regression.
- Added `cpython_set_reentrant_mutation_subset`, adapted from CPython
  `Lib/test/test_set.py::TestWeirdBugs.test_merge_and_mutate` and
  `::test_hash_collision_concurrent_add`, covering set updates whose rich
  equality clears the source set plus hash-collision `set.add()` re-entering the
  same set from Python-level `__eq__`.
- Added `cpython_set_operations_mutating_subset`, adapted from CPython
  `Lib/test/test_set.py::TestOperationsMutating`, covering a deterministic
  stable subset of set equality, ordering, algebra, relation methods, and
  update methods whose element `__eq__` clears both participating sets.
- Added `cpython_set_rich_compare_reflection_subset`, adapted from CPython
  `Lib/test/test_set.py::TestSet.test_rich_compare`, covering set ordering
  returning `NotImplemented` for unrelated operands and dispatching the right
  operand's reflected rich-comparison method.
- Added `cpython_set_inplace_algebra_methods_subset`, adapted from CPython
  `Lib/test/test_set.py::TestSet` update and in-place set algebra cases for
  iterable operands, multi-operand `update` / `intersection_update` /
  `difference_update`, `symmetric_difference_update`, in-place operator
  identity preservation, strict `TypeError` for unhashable iterable operands,
  and partial mutation before `set.update()` encounters an unhashable element.
- Added `cpython_set_only_sets_in_binary_ops_subset`, adapted from CPython
  `Lib/test/test_set.py::TestOnlySetsInBinaryOps`, covering equality with
  unrelated operands, `TypeError` for ordering and binary/in-place set
  operators with non-set operands, and method-form acceptance of iterable
  operands including generators.
- Extended `dict()` to consume supported mapping-style objects through
  `keys()` plus `__getitem__`, and extended `dict.fromkeys()` to consume the VM
  iterator protocol rather than only static sequence values.
- Tightened hashability, reversed arity/non-reversible, keyword-argument, and
  dictionary-update length failures into CPython-style catchable exceptions for
  the supported subset.

Completed in the nested selector assignment parity pass:

- Extended `cpython_ast_subscript_assignment_subset` and
  `cpython_ast_slice_assignment_subset` with CPython-style mutation of mutable
  values reached through immutable containers, such as a list stored inside a
  tuple.
- Fixed the compiler so subscript and slice assignment/deletion no longer writes
  the mutated selector receiver back to its parent expression. This matches
  Python's `obj.attr[key] = value` and `items[0][0] = value` behavior and avoids
  corrupting live `__dict__` views such as `types.SimpleNamespace.__dict__`.

Completed in the abs protocol pass:

- Extended `cpython_abs_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_abs`, with CPython's custom
  `__abs__` instance hook case.
- Changed `abs(obj)` to dispatch class-level `__abs__` for ordinary instances,
  returning the method result directly like CPython.
- Kept arity and unsupported-type failures catchable as `TypeError` in the
  supported subset, and extended the CPython/MiniPython differential parity
  case that covers `abs()`, `min()`, and `sum()`.

Completed in the namespace builtin pass:

- Added `cpython_globals_locals_builtin_subset`, adapted from CPython builtin
  namespace behavior and `Lib/test/test_scope.py` locals coverage.
- Added runtime support for zero-argument `globals()` and `locals()`: module
  scope returns the same live namespace mapping for both, while function scope
  returns a live module mapping for `globals()` and a snapshot of current locals
  for `locals()`.
- Added a CPython/MiniPython differential parity case for the supported
  `globals()` / `locals()` subset.

Completed in the eval builtin pass:

- Added `cpython_eval_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_eval`.
- Added first-pass runtime support for `eval(str)` by feeding the source string
  through MiniPython's existing lexer, eval-mode parser, compiler, and VM.
- Evaluated expressions use the caller's current global and local scopes, carry
  print/output side effects back to the caller, trim the same leading/trailing
  expression whitespace covered by CPython's basic eval tests, and turn parse
  failures into catchable `SyntaxError` values.
- Added a CPython/MiniPython differential parity case for the supported
  `eval()` subset.

Completed in the eval globals/locals dict pass:

- Extended `cpython_eval_builtin_subset` with the dict-backed
  `eval(source, globals)` and `eval(source, globals, locals)` cases from
  `Lib/test/test_builtin.py::BuiltinTest::test_eval`.
- Added name resolution over supplied dict environments, including CPython's
  locals-before-globals lookup order for the supported string-key subset.
- Added eval-internal `globals()` and `locals()` reads over supplied
  environments, plus catchable `TypeError` for unsupported globals/locals
  argument shapes.
- Eval now prepares supplied globals before compiling the source, matching
  CPython's behavior where valid globals receive `__builtins__` even when the
  source later raises `SyntaxError` or source-type `TypeError`.
- Eval now writes named-expression assignments back to supplied local mappings,
  including the `eval(source, g, g)` same-dict case and assignments that happen
  before a runtime exception.
- Updated the CPython/MiniPython differential parity case. General mapping
  objects and mutation identity details remain future eval/exec runtime work.

Completed in the exec builtin pass:

- Added `cpython_exec_builtin_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_exec`.
- Added first-pass runtime support for `exec(str)` by feeding source through the
  existing lexer, file-mode parser, compiler, and VM.
- Added dict-backed `exec(source, globals)` and
  `exec(source, globals, locals)` for the supported string-key subset, including
  writeback of assigned and global-declared names into the supplied dictionaries.
- Exec now syncs writes that happened before a runtime exception back to the
  supplied globals/locals dictionaries, and prepares supplied globals before
  compiling the source so `__builtins__` is present even after `SyntaxError` or
  source-type `TypeError` paths.
- Exec now reuses one scope when globals and locals point at the same mapping,
  matching CPython's `exec(source, g, g)` behavior for ordinary assignments,
  `global` declarations, and runtime-exception writeback.
- Added catchable `TypeError` for unsupported argument shapes and catchable
  `SyntaxError` for parse failures, plus a CPython/MiniPython differential
  parity case. Broader custom mapping mutation semantics remain future exec
  runtime work.

Completed in the compile builtin code-object pass:

- Added `cpython_compile_builtin_code_object_subset`, adapted from
  `Lib/test/test_builtin.py::BuiltinTest::test_compile`.
- Added first-pass `compile(source, filename, mode)` runtime support for string
  and bytes sources in `exec`, `eval`, and `single` modes, including positional
  and keyword argument binding plus catchable `TypeError`, `ValueError`, and
  `SyntaxError` paths.
- Added a minimal `code` runtime value that carries register bytecode and can
  be fed back into `eval()` and `exec()`. `exec()` now preserves output from
  compiled `exec` and `single` code objects, while `eval()` returns values for
  eval-mode code and `None` for exec/single-mode code, matching the migrated
  CPython behavior.
- Added CPython/MiniPython differential parity for the supported compile/code
  object slice.

Completed in the eval/exec keyword and bytes-source pass:

- Extended `cpython_eval_builtin_subset` and `cpython_exec_builtin_subset` with
  current CPython `Lib/test/test_builtin.py` keyword-argument cases for
  `source`, `globals`, and `locals`.
- Reused the compile-source decoding path so `eval()` and `exec()` accept
  bytes and bytearray sources in the supported encoding subset.
- Added keyword-only locals handling for `eval()` and `exec()`: default globals
  are preserved while the supplied locals mapping is used for local name
  resolution or local writes.

Completed in the eval/exec builtins-mapping pass:

- Added `cpython_eval_exec_builtins_mapping_subset`, adapted from CPython
  `Lib/test/test_builtin.py::BuiltinTest::test_exec_globals` and
  `::test_eval_builtins_mapping`.
- Name lookup now honors `globals['__builtins__']` when supplied: an empty
  builtins dict hides default builtins, invalid builtin containers raise
  `TypeError`, and exact-dict `mappingproxy` values can provide builtin names.
- `eval()` and `exec()` now inject a default `__builtins__` dictionary into
  supplied globals when it is absent, including globals/locals split execution,
  eval error paths, dict subclasses, and default `__import__` support for import
  statements executed under injected builtins.
- Extended the same subset with CPython
  `Lib/test/test_builtin.py::BuiltinTest::test_exec_globals_dict_subclass` and
  the import half of `::test_exec_builtins_mapping_import`: dict subclasses can
  serve as builtin mappings, and import statements resolve through a supplied
  `__import__` builtin or raise `ImportError` when it is absent.
- Extended the same subset with CPython's custom `__getitem__` error path for
  dict-subclass globals and builtin mappings. User-defined function objects now
  retain their definition-time globals, so methods invoked during mapping
  lookup resolve their own global names without recursively consulting the
  caller's custom globals mapping.
- Added CPython/MiniPython differential parity for the accepted dict-subclass
  builtins and custom-`__import__` paths.

Completed in the t-string templatelib pass:

- Added `cpython_t_string_templatelib_iteration_subset`, adapted from
  `Lib/test/test_tstring.py` and `Lib/test/test_string/_support.py`.
- Added the supported `string.templatelib` surface with `Template`,
  `Interpolation`, and `convert()`.
- Made Template values iterable over literal strings and interpolation objects,
  and made `Interpolation(value, expression, conversion, format_spec)` usable
  in class patterns for the supported t-string rendering helper shape.

Completed in the templatelib constructor pass:

- Added `cpython_templatelib_constructor_subset`, adapted from
  `Lib/test/test_string/test_templatelib.py`.
- Added runtime calls for `Template(*args)` and
  `Interpolation(value, expression='', conversion=None, format_spec='')`,
  including CPython-style interleaving of strings and interpolation objects.
- Added `Template.values` and `Template` / `Interpolation` `__qualname__` and
  `__module__` metadata for the supported templatelib surface.

Completed in the templatelib final-type and iterator pass:

- Added `cpython_templatelib_final_type_and_iterator_subset`, adapted from
  `Lib/test/test_string/test_templatelib.py`.
- Added a dedicated `TemplateIter` runtime value for `iter(Template)`, including
  type metadata, self-iteration, interpolation yields, and repeated exhausted
  `StopIteration` behavior.
- Rejected subclassing `Template`, `Interpolation`, and `TemplateIter` with
  catchable `TypeError`, matching the supported CPython final-type surface.

Completed in the collections ABC iterator pass:

- Added `cpython_collections_abc_iterable_iterator_subset`, adapted from
  `Lib/test/test_string/test_templatelib.py::TemplateIterTests::test_abc` and
  CPython's `Lib/test/test_collections.py` Iterable/Iterator ABC checks.
- Added the supported `collections` / `collections.abc` import surface for
  `Iterable` and `Iterator`.
- Added `isinstance` and `issubclass` support for `Iterable` / `Iterator` across
  built-in containers, built-in iterators, `TemplateIter`, scalar non-samples,
  structural user classes, and direct ABC subclassing.
- Kept CPython's key distinction that `Iterator` requires both `__iter__` and
  `__next__`; a class with only `__next__` is not an Iterator ABC instance.

Completed in the collections ABC core-runtime pass:

- Added `cpython_collections_abc_core_runtime_subset`, adapted from
  `Lib/test/test_collections.py` Hashable, Sized, Container, Callable, and
  Collection ABC checks.
- Extended `collections.abc` with `Hashable`, `Sized`, `Container`, `Callable`,
  `Collection`, and the currently relation-only `Sequence`, `Mapping`,
  `MutableMapping`, `Set`, and `MutableSet` ABC names.
- Added `isinstance` and `issubclass` support for the core ABCs across
  supported built-in values, function/bound-method callability, structural user
  classes, direct ABC subclassing, and Collection's Sized + Iterable +
  Container relationship.
- Added CPython-style `None` blocking for ABC special methods such as
  `__len__ = None`, `__contains__ = None`, `__call__ = None`, `__iter__ = None`,
  `__next__ = None`, and `__hash__ = None`.

Completed in the collections ABC reversible pass:

- Added `cpython_collections_abc_reversible_subset`, adapted from
  `Lib/test/test_collections.py::test_Reversible`.
- Extended `collections.abc` with `Reversible`.
- Added `isinstance` and `issubclass` support for `Reversible` across supported
  built-in reversible values, dict views, scalar/container/iterator
  non-samples, structural user classes with both `__iter__` and
  `__reversed__`, direct ABC subclassing, and `Sequence` inheriting from
  `Reversible`.
- Preserved CPython's rule that `__reversed__` without `__iter__` is not enough,
  and that `__iter__ = None` / `__reversed__ = None` blocks ABC recognition.

Completed in the collections ABC sequence pass:

- Added `cpython_collections_abc_sequence_subset`, adapted from CPython
  `Lib/test/test_collections.py::test_Sequence`.
- Added `issubclass` support for CPython's registered built-in sequence types:
  `list`, `tuple`, `str`, `bytes`, `bytearray`, and `range`.
- Added `isinstance` support for explicit `Sequence` subclasses while preserving
  CPython's rule that a plain user class with `__len__` + `__getitem__` is not
  structurally considered a `Sequence`.
- Completed the visible `Sequence` inheritance links to `Reversible`,
  `Collection`, `Sized`, `Iterable`, and `Container`.

Completed in the collections ABC sequence-mixins pass:

- Added `cpython_collections_abc_sequence_mixins_subset`, adapted from CPython
  `Lib/test/test_collections.py::test_Sequence_mixins`.
- Added `Sequence` mixin method lookup for explicit subclasses while preserving
  normal Python precedence: instance fields and user-defined class attributes
  still override ABC mixins.
- Implemented `Sequence.index()` with CPython-style negative `start` / `stop`
  handling and `IndexError` termination, verified against native `list.index`
  and `str.index` across the migrated start/stop matrix.
- Implemented the remaining supported `Sequence` mixins: `count`,
  `__contains__`, `__iter__`, and `__reversed__`, including membership fallback
  for user instances that expose the sequence protocol.
- Added keyword-call support for Python-defined mixin methods such as
  `wrapped.index(value='b')` and `wrapped.count(value='a')`.

Completed in the collections ABC mapping pass:

- Added `cpython_collections_abc_mapping_subset`, adapted from CPython
  `Lib/test/test_collections.py` Mapping and MutableMapping ABC checks.
- Added `dict` registrations for `Mapping` and `MutableMapping`.
- Completed visible `Mapping` and `MutableMapping` inheritance through
  `Collection`, `Sized`, `Iterable`, and `Container`.
- Preserved CPython's non-structural behavior: arbitrary user classes with
  `__iter__`, `__len__`, `__getitem__`, `__setitem__`, and `__delitem__` are not
  automatically considered mapping ABC subclasses.
Completed in the collections ABC byte-string/buffer pass:

- Added `cpython_collections_abc_bytestring_buffer_subset`, adapted from CPython
  `Lib/test/test_collections.py::test_ByteString` and `::test_Buffer`.
- Extended `collections.abc` with `ByteString` and `Buffer`.
- Added `bytes` / `bytearray` registrations for both ABCs, while preserving
  CPython's distinction that `ByteString` itself does not inherit `Buffer`.
- Added `Buffer` structural `__buffer__` checks for user classes and
  CPython-style `__buffer__ = None` blocking.
- Left `memoryview` parity and `ByteString` deprecation warnings for future
  runtime-warning and buffer-object support.

Completed in the collections ABC mutable-sequence pass:

- Added `cpython_collections_abc_mutable_sequence_subset`, adapted from CPython
  `Lib/test/test_collections.py::test_MutableSequence` and
  `::test_MutableSequence_mixins`.
- Extended `collections.abc` with `MutableSequence`.
- Added `list` / `bytearray` registrations and visible `MutableSequence`
  inheritance through `Sequence`, `Reversible`, `Collection`, `Sized`,
  `Iterable`, and `Container`.
- Preserved CPython's non-structural behavior: a user class with
  `__len__`, `__getitem__`, `__setitem__`, `__delitem__`, and `insert` is not
  automatically a `MutableSequence`.
- Added `MutableSequence` mixins for explicit subclasses: `append`, `clear`,
  `reverse`, `extend`, `pop`, `remove`, and `__iadd__`, including the
  self-extension case covered by CPython issue 34427.
- Routed augmented `+=` on user instances through visible `__iadd__` methods so
  ABC mixins participate in register-bytecode execution.

Completed in the collections ABC set/mutable-set mixins pass:

- Added `cpython_collections_abc_set_mutable_set_mixins_subset`, adapted from
  CPython `Lib/test/test_collections.py::test_Set`, `::test_hash_Set`,
  `::test_isdisjoint_Set`, `::test_equality_Set`, `::test_arithmetic_Set`,
  `::test_MutableSet`, and MutableSet regression cases.
- Completed `Set` / `MutableSet` ABC inheritance through `Collection`,
  `Sized`, `Iterable`, and `Container`, plus `set` registration as a
  `MutableSet` and `frozenset` registration as a non-mutable `Set`.
- Added explicit-subclass `Set` mixins for comparison, equality,
  `isdisjoint`, binary set operations, `_from_iterable`, and `_hash`.
- Added explicit-subclass `MutableSet` mixins for `remove`, `pop`, `clear`,
  `__ior__`, `__iand__`, `__ixor__`, and `__isub__`, including identity
  clearing behavior for self-subtraction and self-symmetric-difference.

Completed in the frozenset first-pass runtime pass:

- Added `cpython_frozenset_basic_subset`, adapted from CPython
  `Lib/test/test_set.py::TestFrozenSet`, shared `TestJointOps`, and
  `Lib/test/test_collections.py` Set/MutableSet ABC registration checks.
- Added `cpython_set_frozenset_joint_ops_subset`, adapted from CPython
  `Lib/test/test_set.py::TestJointOps`, so the common set/frozenset contract
  is tested for membership, nested hashable frozensets, set-operator operand
  rejection, method result types, subset/superset comparisons, and iterable
  method operands.
- Added `cpython_set_frozenset_relationship_matrix_subset`, adapted from
  `TestJointOps` equality, set-of-frozensets, and `isdisjoint` constructor
  matrix cases.
- Added `cpython_set_frozenset_algebra_matrix_subset`, adapted from
  `TestJointOps` non-mutating `union`, `intersection`, `difference`, and
  `symmetric_difference` constructor matrix cases, including multi-operand
  union/intersection/difference and the Issue #6573 empty-set union regression.
- Added exact builtin `frozenset` construction, iteration, truthiness,
  `repr(frozenset())`, empty-frozenset singleton identity, constructor
  identity preservation for existing frozensets, no-op exact `frozenset`
  `__init__`, equality with set/frozenset, order-independent hashing for
  hashable elements, and use as dict/set keys.
- Added readonly frozenset methods and operators with frozenset-preserving
  result type when frozenset is the left operand and set-preserving result type
  when set is the left operand.
- Registered frozenset as `Set`, `Hashable`, `Sized`, `Iterable`, `Container`,
  and `Collection`, while keeping it out of `MutableSet`.
- Added `cpython_set_and_frozenset_subclass_subset`, adapted from CPython
  `Lib/test/test_set.py::TestSetSubclass` and `TestFrozenSetSubclass`, covering
  first-pass set/frozenset subclass construction, keyword rejection for inherited
  constructors, iteration, membership, `len`, `set()` conversion, builtin method
  result types, in-place set mutation preserving subclass identity,
  `super().__init__` for set subclasses, custom `__new__` using
  `super().__new__`, frozenset-subclass hashing, frozenset subclass copy and
  constructor identity, empty frozenset subclass identity behavior, basic
  set/frozenset subclass `__slots__`, and `Set`/`MutableSet`/`Hashable` ABC
  registration.
- Left set/frozenset subclass pickle support, CPython's exact frozenset hash
  algorithm/cache behavior, and mutation-during-operation edge cases for later
  object-model work.

Completed in the collections ABC mapping-view pass:

- Added `cpython_collections_abc_mapping_view_subset`, adapted from CPython
  `Lib/test/test_collections.py` mapping view checks.
- Extended `collections.abc` with `MappingView`, `KeysView`, `ItemsView`, and
  `ValuesView`.
- Added `isinstance` and `issubclass` support for built-in dict key/item/value
  views, `KeysView` and `ItemsView` behaving as `Set`, `ValuesView` behaving as
  `Collection` but not `Set`, and the `MappingView` / `Sized` inheritance edge.
- Preserved CPython's non-structural behavior: an arbitrary class with
  `__len__`, `__iter__`, and `__contains__` is not automatically a mapping
  view.

Completed in the collections ABC mapping-mixins pass:

- Added `cpython_collections_abc_mapping_mixins_subset`, adapted from CPython
  `Lib/test/test_collections.py::test_Mapping` and
  `::test_MutableMapping`.
- Added explicit-subclass `Mapping` mixins for `get`, `__contains__`, `keys`,
  `items`, `values`, and `__eq__`, including `NotImplemented` for non-mapping
  equality operands and the CPython `Mapping.__reversed__ = None` behavior.
- Added explicit-subclass `MutableMapping` mixins for `pop`, `popitem`,
  `clear`, `update`, and `setdefault`, including mapping sources, iterable
  pair sources, and keyword updates.
- Added lexicographic list/tuple comparison in the VM so CPython-style sorted
  item pairs behave correctly for migrated collection tests.

Completed in the collections ABC mapping-mixin-view pass:

- Added `cpython_collections_abc_mapping_mixin_views_subset`, adapted from
  CPython `Lib/test/test_collections.py` mapping view expectations for
  explicit `MutableMapping` subclasses.
- Changed `Mapping.keys()`, `Mapping.items()`, and `Mapping.values()` mixins to
  return live `KeysView`, `ItemsView`, and `ValuesView` runtime objects instead
  of eager lists.
- Added VM support for iterating, sizing, truth testing, membership testing, and
  ABC recognition of custom mapping views.
- Added set-like operators and comparisons for key/item views by materializing a
  snapshot at operation time, while keeping later mapping mutations visible to
  the original view object.

Completed in the dict view rich-comparison pass:

- Added `cpython_dict_view_richcompare_subset`, adapted from CPython
  `Lib/test/test_dict.py::test_keys_contained` and
  `::test_errors_in_view_containment_check`.
- Tightened VM comparison instructions so comparison errors are converted back
  into catchable Python exceptions through the same runtime path as membership
  and truth tests.
- Added VM-aware set-style comparison for dict key/item views, including
  subset/superset/equality checks and Python-level `__eq__` dispatch inside
  tuple item comparisons.
- Updated dict item-view rich comparisons to compare values with Python-level
  `__eq__` after matching keys, so exceptions raised by item values propagate
  across equality and subset/superset comparisons.
- Preserved snapshot-based set-like behavior for existing dict and mapping view
  operators while moving view comparisons closer to CPython's error propagation.

Completed in the dict view mappingproxy pass:

- Added `cpython_dict_view_mappingproxy_subset`, adapted from CPython
  `Lib/test/test_dict.py::test_views_mapping`.
- Added the read-only `mappingproxy` runtime value exposed by built-in dict
  views through `.mapping`, including `type(type.__dict__)`, `isinstance`,
  live dict equality, item lookup, membership, and item-assignment rejection.
- Registered `mappingproxy` as a read-only `Mapping`, `Sized`, `Iterable`,
  `Container`, `Collection`, and `Reversible` built-in type while keeping it
  outside `MutableMapping`.
- Added `cpython_types_mappingproxy_exact_dict_subset`, adapted from CPython
  `Lib/test/test_types.py::MappingProxyTests` for the exact-dict constructor
  path.
- Added the `types` module surface for `MappingProxyType`, plus mappingproxy
  `get`, `copy`, `keys`, `items`, `values`, iteration, reverse iteration, and
  invalid constructor/write error behavior. Dict subclasses and ChainMap remain
  future object-model work.
- Added `cpython_types_mappingproxy_union_subset`, adapted from CPython
  `MappingProxyTests::test_union` for exact `dict` and `mappingproxy` operands.
- Extended VM bitwise-or handling so `mappingproxy | dict`,
  `dict | mappingproxy`, and `mappingproxy | mappingproxy` return a fresh dict,
  while `mappingproxy |= ...` raises a catchable `TypeError`.
- Added `cpython_types_mappingproxy_method_surface_subset`, adapted from
  CPython `MappingProxyTests::test_methods`, covering the public mappingproxy
  method names plus callable `__or__`, `__ror__`, `__ior__`, and
  `__class_getitem__` behavior.
- Added `cpython_types_mappingproxy_custom_mapping_subset`, adapted from
  CPython `MappingProxyTests::test_customdict`; MiniPython uses a
  user-defined mapping object to cover the same forwarding behavior before
  dict-subclass storage support lands.
- Added `cpython_types_mappingproxy_hash_subset`, adapted from CPython
  `MappingProxyTests::test_hash`, covering unhashable exact-dict proxies plus
  hash forwarding for hashable user-defined mapping objects.
- Added `cpython_types_mappingproxy_richcompare_subset`, adapted from CPython
  `MappingProxyTests::test_richcompare`, covering `mappingproxy` equality,
  inequality, and CPython-style catchable `TypeError` ordering errors.
- Added `cpython_types_mappingproxy_contains_subset`,
  `cpython_types_mappingproxy_views_subset`,
  `cpython_types_mappingproxy_len_subset`,
  `cpython_types_mappingproxy_iterators_subset`,
  `cpython_types_mappingproxy_reversed_subset`, and
  `cpython_types_mappingproxy_copy_subset`, adapted from the matching CPython
  `MappingProxyTests` exact-dict behavior methods.
- Added `cpython_types_mappingproxy_missing_subset`, adapted from CPython
  `MappingProxyTests::test_missing`; dict subclasses now carry internal dict
  storage, use `__missing__` for `__getitem__`, and keep `get`/membership from
  invoking `__missing__`.
- Added `cpython_types_mappingproxy_chainmap_subset`, adapted from CPython
  `MappingProxyTests::test_chainmap`; `collections.ChainMap` now has enough
  mapping behavior for mappingproxy forwarding, copying, iteration, views,
  length, containment, and `isinstance(..., collections.ChainMap)`.
- Added first-pass old-style string formatting for `%s`, `%r`, `%a`, `%d`,
  `%i`, `%x`, `%X`, `%o`, `%c`, and `%%`, plus `%(key)` mapping arguments from
  CPython `Lib/test/test_format.py::test_str_format`; this also supports
  CPython's `dict.__missing__` test body.
- Extended that old-style formatting slice with static flags, width, and
  precision for string/repr/ascii, decimal integers, hexadecimal/octal
  integers, and `%c`, including zero padding, sign/space flags, left alignment,
  alternate integer prefixes, and precision truncation/padding.
- Added dynamic `*` width and precision for old-style string formatting,
  including CPython's negative-width left alignment, negative precision
  normalization, argument consumption order, non-integer `*` TypeErrors, and
  rejection of `*` with parenthesized mapping keys.
- Added first-pass old-style float formatting for `%f`, `%e`, `%E`, `%g`, and
  `%G`, including width, precision, sign, zero padding, alternate `%#g`
  trailing-zero preservation, normalized two-digit exponents, and CPython's
  `%d` acceptance of float inputs by truncation.
- Added CPython old-style formatting aliases and ignored length modifiers:
  `%u` as a decimal integer alias, `%F` as a fixed-float alias, and `h` / `l` /
  `L` length modifiers before supported conversion codes.
- Expanded CPython `Lib/test/test_format.py::test_common_format` old-style
  formatting coverage for arbitrary-precision decimal, hexadecimal, and octal integers,
  including sign/space handling, width, left alignment, zero padding, integer
  precision, uppercase hexadecimal, alternate-form prefixes, and CPython's
  zero-flag behavior when width and integer precision are both present.
- Added the small-int `test_common_format` matrix for old-style `%d`, `%x`,
  `%X`, `%o`, alternate prefixes, zero values, negative hexadecimal/octal
  output, and `%d` truncation of float inputs.
- Migrated representative old-style formatting error paths from
  `Lib/test/test_format.py`, including isolated `%`, unsupported conversion
  codes such as `%z` / `%b` / `%I`, and not-enough-arguments errors.
- Extended the same CPython old-style formatting error slice with malformed
  percent specifiers, unsupported control/flag characters, malformed mapping
  key shapes, star width/precision arity and type errors, and numeric
  conversion type rejection for `%d`, `%x`, and `%g`.
- Migrated CPython `Lib/test/test_format.py::test_non_ascii` and
  `::test_g_format_has_no_trailing_zeros` slices for the `format()` /
  f-string mini-language, including non-ASCII fill characters for left, right,
  and center alignment plus `g` / `G` general floating-point formatting with
  alternate-form trailing-zero preservation.
- Migrated the executable portion of CPython
  `Lib/test/test_format.py::test_precision` for `format()` precision on floats
  and complex numbers. MiniPython now formats float and complex `f` / `F`,
  `e` / `E`, and `g` / `G` components with CPython-style real-part sign
  handling, `+` / `-` imaginary separators, normalized scientific exponents,
  and alternate-form trailing-zero preservation.
- Migrated CPython `Lib/test/test_format.py::test_better_error_message_format`
  and `::test_unicode_in_error_message` slices. Invalid format mini-language
  specs now report CPython-style `ValueError: Invalid format specifier ... for
  object of type ...` messages through `format()`, f-strings, and
  `str.format()`.
- Migrated the supported executable slice of CPython
  `Lib/test/test_format.py::test_negative_zero`. MiniPython now parses the `z`
  sign option, normalizes negative zero after rounding for float and complex
  `f` / `e` / `E` / `g` formatting plus float `%` percentage presentation,
  covers the CPython tiny-negative-value and post-rounding-sign cases,
  preserves genuinely negative rounded values, and keeps CPython's
  fill-character ordering for specs such as `z>6.1f`, `z>z6.1f`, and
  `x>z6.1f`.
- Migrated CPython `Lib/test/test_format.py::test_specifier_z_error` for the
  supported string-formatting surface. Invalid `z` positions now produce
  CPython-style invalid-specifier `ValueError`s, while invalid non-floating
  presentation types such as `zd` and `zs` report
  `Negative zero coercion (z) not allowed`; old-style `%z.1f` remains rejected.
- Added `cpython_types_simple_namespace_basic_subset`, adapted from CPython
  `Lib/test/test_types.py::SimpleNamespaceTests`. The VM now exposes
  `types.SimpleNamespace`, supports construction from dicts, minimal
  `collections.UserDict` mapping sources, pair iterables, and keywords, keeps
  `__dict__` live with `vars()`, implements attribute get/set/delete,
  repr/equality, subclass construction through the inherited initializer, and
  preserves catchable TypeError/ValueError behavior for unsupported operations
  and invalid constructor inputs.
- Added `cpython_types_simple_namespace_recursive_and_replace_subset`, adapted
  from CPython `SimpleNamespaceTests::test_recursive`,
  `::test_recursive_repr`, `::test_replace`, and `::test_replace_subclass`.
  MiniPython now exposes a minimal `copy` module with `copy.replace()` for
  exact `SimpleNamespace` objects and subclasses, returns shallow independent
  copies with keyword field overrides, preserves subclass identity, and protects
  both `repr(ns)` and direct display for recursive namespace graphs.

Completed in the collections ABC async-runtime pass:

- Added `cpython_collections_abc_async_runtime_subset`, adapted from CPython
  `Lib/test/test_collections.py` Awaitable, Coroutine, AsyncIterable, and
  AsyncIterator ABC checks.
- Extended `collections.abc` with `Awaitable`, `Coroutine`, `AsyncIterable`,
  and `AsyncIterator`.
- Added `isinstance` and `issubclass` support for native coroutine objects,
  structural `__await__`, `send`/`throw`/`close`, `__aiter__`, and `__anext__`
  user classes, ABC inheritance, and non-sample rejection.
- Preserved CPython's distinctions that a plain generator is not Awaitable,
  an `__await__`-only object is not a Coroutine, and `__anext__` without
  `__aiter__` is not an AsyncIterator.

Completed in the collections ABC generator-runtime pass:

- Added `cpython_collections_abc_generator_runtime_subset`, adapted from
  CPython `Lib/test/test_collections.py` Generator and AsyncGenerator ABC
  checks.
- Extended `collections.abc` with `Generator` and `AsyncGenerator`.
- Added `isinstance` and `issubclass` support for native generator and async
  generator objects, structural protocol user classes, incomplete protocol
  non-samples, direct ABC subclassing, and inherited `Iterator` /
  `AsyncIterator` relationships.
- Preserved CPython-style `None` blocking for generator protocol methods such
  as `send = None` and async-generator protocol methods such as `asend = None`.

Completed in the selector/atom differential-parity pass:

- Extended `cpython_program_output_parity_smoke_subset` with CPython
  `Lib/test/test_grammar.py::test_selectors` tuple-subscript key behavior,
  including assignment and lookup through `d[1,]`, `d[1, 2]`, and
  `d[1, 2, 3]`.
- Added a CPython `Lib/test/test_grammar.py::test_atoms` parity case for
  grouped expressions, tuple/list displays, empty dict displays, and
  boolean-expression dictionary keys.

Completed in the operator differential-parity pass:

- Extended `cpython_program_output_parity_smoke_subset` with CPython
  `Lib/test/test_grammar.py::test_binary_mask_ops`, `::test_shift_ops`,
  `::test_additive_ops`, `::test_multiplicative_ops`, and `::test_unary_ops`
  parity coverage for bitwise, shift, additive, multiplicative, and unary
  precedence/associativity.
- Added CPython differential parity for `@`, `@=`, `__matmul__`,
  `__rmatmul__`, and `__imatmul__`, tying the existing matrix-multiply object
  protocol implementation to a CPython oracle in addition to MiniPython's
  direct subset tests.

Completed in the source-encoding detection pass:

- Added `detect_source_encoding()` as a byte-oriented PEP 263 helper separate
  from the existing UTF-8 `&str` parser path.
- Added `cpython_source_encoding_detection_subset`, adapted from CPython
  `Lib/test/test_tokenize.py::TestDetectEncoding` and
  `Lib/test/test_source_encoding.py`.
- Covered default UTF-8 detection, first-line and second-line coding cookies,
  ignored second-line cookies after real code, latin-1 and UTF-8 normalization,
  UTF-8 BOM stripping, BOM/cookie mismatch errors, null bytes in coding lines,
  unknown encodings, and ASCII/UTF-8 decode failures.
- Moved the `ENCODING` row from `out_of_scope_runtime` to `partial`: MiniPython
  now models source-encoding detection explicitly, but still does not emit a
  CPython-style leading `ENCODING` token from `tokenize_with_spans()` or decode
  file input directly into the parser.

Completed in the bytes-tokenizer encoding pass:

- Added `Token::Encoding` and `tokenize_bytes_with_spans()` so byte-oriented
  tokenization now emits a leading CPython-style `ENCODING` token at `(0, 0)`.
- Kept ordinary `tokenize_with_spans()` and the parser path unchanged, so the
  parser still receives the same token stream as before.
- Added `cpython_tokenize_bytes_encoding_token_subset`, adapted from CPython's
  `Lib/test/test_tokenize.py::TokenizeTest.check_tokenize` convention and
  `TestDetectEncoding` source shapes.
- Covered UTF-8 source, UTF-8 BOM stripping, latin-1 bytes decoding,
  iso-8859-15 bytes decoding, and ASCII source rejection for non-ASCII bytes.

Completed in the bytes-source execution pass:

- Added `run_source_bytes()` so supported bytes source is decoded with the same
  PEP 263 helper before entering the ordinary parser/compiler/register-VM path.
- Added `cpython_source_encoding_execution_subset`, adapted from CPython
  `Lib/test/test_source_encoding.py::AbstractSourceEncodingTest` and
  `MiscSourceEncodingTest::test_compilestring`.
- Covered default UTF-8 execution, UTF-8 BOM stripping, first- and second-line
  iso-8859-15 cookies, ignored third-line cookies, UTF-8 cookie execution,
  non-UTF-8 shebangs paired with matching cookies, and representative
  BOM/cookie mismatch plus ASCII decode failures.

Completed in the source-newline normalization pass:

- Normalized CRLF and lone CR source newlines after bytes decoding and before
  tokenization/execution, matching CPython source processing for supported bytes
  input.
- Extended `cpython_tokenize_bytes_encoding_token_subset` with a triple-quoted
  string token whose source newline is CRLF and whose decoded string value is
  `\n`.
- Extended `cpython_source_encoding_execution_subset` with CPython
  `Lib/test/test_source_encoding.py` cases for double coding lines, long
  coding-cookie lines, non-UTF-8 coding-cookie comments, and CR/CRLF
  normalization inside triple-quoted string literals.
- Extended both byte tokenization and byte-source execution with CPython
  `Lib/test/test_tokenize.py::CTokenizerBufferTests::test_newline_at_the_end_of_buffer`.
  The migrated source has a `latin-1` coding cookie, two very long comment
  lines, and no final newline; MiniPython now asserts the synthesized final
  newline token and empty-program execution behavior for that shape.

Completed in the source-encoding edge pass:

- Matched CPython's `get_normal_name()` behavior for source encoding cookies:
  only the first 12 normalized characters are used to recognize `utf-8-*`,
  `latin-1-*`, `iso-8859-1-*`, and `iso-latin-1-*` aliases.
- Extended `cpython_source_encoding_detection_subset` with long UTF-8 and
  latin-1 alias names that exercise that prefix behavior.
- Extended `cpython_source_encoding_execution_subset` with CPython
  `Lib/test/test_source_encoding.py::test_long_coding_line`,
  `::test_long_coding_name`, `::test_long_first_utf8_line`,
  `::test_long_second_utf8_line`, and partial UTF-8 BOM decode-error shapes.

Completed in the extended source-encoding parity pass:

- Extended `cpython_source_encoding_detection_subset` with more CPython
  `Lib/test/test_tokenize.py::TestDetectEncoding` cases: non-comment
  `#coding` text, BOM with a non-cookie second line, empty-first-line
  second-line cookies, ignored third-line cookies, short single-line files,
  second-line null bytes, second-line BOM/cookie mismatch, second-line ASCII
  decode failures, second-line default UTF-8 failures, BOM default UTF-8
  failures, and first-line latin-1 bytes without a coding cookie.
- Extended `cpython_source_encoding_execution_subset` with CPython
  `Lib/test/test_source_encoding.py` cases for UTF-8 BOM comments,
  second-line BOM/cookie mismatch, non-UTF-8 shebangs without cookies,
  non-UTF-8 second and third source lines, explicit `utf-8` / `utf8`
  third-line decode failures, and null bytes in the second coding line.

Completed in the source-encoding normalization matrix pass:

- Extended `cpython_source_encoding_detection_subset` with CPython
  `Lib/test/test_tokenize.py::TestDetectEncoding` latin-1 and UTF-8
  normalization matrices, including hyphen and underscore spellings for
  `latin-1`, `iso-8859-1`, `iso-latin-1`, their 12-character-prefix suffix
  forms, and `utf-8-mac` / `utf-8-unix`.
- Added detection coverage for short BOM-prefixed source with code on the first
  line, non-UTF-8 shebang bytes paired with a second-line non-UTF-8 cookie, and
  UTF-8 shebang bytes rejected by a second-line ASCII cookie.
- Extended `cpython_source_encoding_execution_subset` with executable
  `latin-1-unix` and `utf_8_mac` cookie spellings.

Completed in the broader source-codec pass:

- Added `encoding_rs` as the external codec backend for source decoding and
  wired a label-lookup fallback for encodings beyond the hand-written
  UTF-8/latin-1/latin-9 paths.
- Extended `detect_source_encoding()`, `tokenize_bytes_with_spans()`, and
  `run_source_bytes()` coverage for `cp1252`, `cp949`, `cp932`, and `cp1251`.
- Ported CPython `Lib/test/test_source_encoding.py::test_exec_valid_coding`
  for `cp949`, plus the `cp1252` long-line/multiline-file parsing shape from
  `::test_file_parse`.
- Added CPython `Lib/test/test_source_encoding.py::test_issue2301`-style
  `cp932` source decoding through the detection, byte-tokenization, and
  executable byte-source paths.
- Added CPython-style `cp1251` source decoding through the same paths, proving
  a non-hard-coded `encoding_rs` label can reach MiniPython's ordinary lexer as
  Unicode string tokens.
- Added byte-tokenization checks proving `cp1252`, `cp949`, `cp932`, and
  `cp1251` decoded source reaches the ordinary lexer as Unicode string tokens.
- Added CPython-style source decode rejection for undefined `cp1251` and
  `cp1252` bytes before those bytes can become control characters in parsed
  string literals.

Completed in the tokenizer source warning/error edge pass:

- Extended `cpython_numeric_literal_warning_subset` with CPython
  `Lib/test/test_source_encoding.py::test_tokenizer_fstring_warning_in_first_line`
  source `0b1and 2`, preserving the binary literal boundary warning.
- Extended `cpython_tokenize_bytes_encoding_token_subset` and
  `cpython_source_encoding_execution_subset` with CPython
  `Lib/test/test_tokenize.py::test_invalid_character_in_fstring_middle`, so
  invalid default-UTF-8 bytes inside f-string middle text are rejected before
  parsing or execution.

Completed in the bytes-source differential encoding pass:

- Added `cpython_bytes_source_output_parity_subset` to
  `tests/cpython_diff.rs`. The harness now writes CPython oracle inputs as
  actual byte files and compares their output with MiniPython's
  `run_source_bytes()` path.
- Migrated executable CPython `Lib/test/test_source_encoding.py` slices for
  UTF-8 coding cookies, ISO-8859-15 first-line cookies, ignored third-line
  cookies, UTF-8 BOM plus UTF-8 cookies, `cp949`, `cp932`, and `cp1252`, plus
  existing `cp1251` fallback coverage.
- Extended output parity with CPython `AbstractSourceEncodingTest` success
  paths for default UTF-8 source decoding, second-line and empty-first-line
  ISO-8859-15 cookies, double coding-line precedence, non-UTF-8 bytes on
  ISO-8859-15 coding-cookie lines, UTF-8 BOM default decoding, and UTF-8 BOM
  files with UTF-8 comment lines.
- Added `cpython_bytes_exec_source_output_parity_subset` for current CPython
  `exec(bytes)` long-line source-encoding cases: long first- and second-line
  coding cookies, long coding-cookie lines, long normalized Latin-1 coding
  names, and long UTF-8 comment-only lines. The ordinary file-oracle
  differential test intentionally stays on cases accepted by the host
  `/usr/bin/python3` 3.9 file reader.
- Migrated the executable semantics of CPython
  `Lib/test/test_source_encoding.py::test_import_encoded_module` for the
  `encoded_modules/module_iso_8859_1.py` and `module_koi8_r.py` samples by
  running equivalent bytes-source files through the CPython/MiniPython
  differential output harness.
- Migrated CPython `Lib/test/test_source_encoding.py::test_20731` via the
  `tokenizedata/coding20731.py` bytes source, preserving the accepted
  `latin1` coding cookie plus CRLF-only body path in differential output
  parity.
- Added CPython `Lib/test/test_source_encoding.py::test_error_message`
  differential output parity for a UTF-8 BOM followed by an otherwise empty
  source line.
- Added `cpython_bytes_source_rejection_parity_subset` for CPython bytes-file
  rejection parity. It covers unknown coding cookies, BOM/cookie mismatches,
  partial UTF-8 BOM inputs, ASCII-cookie decode failures in the source body,
  default-UTF-8 decode failures after the first two lines, and invalid bytes
  inside an f-string middle.
- Added the CPython `tokenizedata/bad_coding.py` and `bad_coding2.py` rejection
  samples, covering the misspelled `uft-8` cookie and `utf8` spelling paired
  with a UTF-8 BOM.
- Added CPython BOM/error edge cases for a second-line non-UTF-8 cookie after a
  UTF-8 BOM, a fake cookie after a UTF-8 BOM, a one-byte UTF-16-LE BOM prefix,
  and a default-UTF-8 decode failure on the second physical line.
- Kept the non-UTF-8 shebang plus second-line coding-cookie case out of the
  default differential harness because the default system `python3` rejects it
  before reaching the newer CPython source-test behavior.

Completed in the SyntaxError parenthesis/string diagnostics pass:

- Migrated CPython
  `Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_parenthesis` by
  aligning MiniPython unmatched-delimiter messages with CPython's
  `unmatched ')'` style, reporting mismatched delimiters against the opening
  parenthesis, preserving `was never closed` for unclosed delimiters that flow
  into a later assignment-like line, and covering the latin-cookie bytes source.
- Migrated CPython
  `Lib/test/test_syntax.py::SyntaxErrorTestCase::test_error_string_literal` by
  changing ordinary and triple-quoted unterminated string diagnostics to
  CPython's `unterminated string literal` wording and preserving the escaped end
  quote hint for both ordinary and raw string prefixes.
- Extended `cpython_syntax_error_message_parity_subset`,
  `cpython_syntax_error_parenthesis_subset`,
  `cpython_invalid_string_literal_subset`, and
  `cpython_bytes_source_rejection_parity_subset` so these method-level ports are
  covered both by MiniPython-local assertions and by CPython differential
  rejection checks.

Completed in the SyntaxError invisible-character and soft-keyword call pass:

- Promoted CPython
  `Lib/test/test_syntax.py::SyntaxErrorTestCase::test_invisible_characters` to
  method-level coverage by adding a syntax-error message differential case for
  `print\x17("Hello")`, keeping the existing MiniPython span checks for both
  the string and bytes-source inputs, and adding the bytes-source rejection to
  the CPython differential harness.
- Promoted CPython
  `Lib/test/test_syntax.py::SyntaxErrorTestCase::test_match_call_does_not_raise_syntax_error`
  and `::test_case_call_does_not_raise_syntax_error` by running the exact
  compile-only source shapes through CPython/MiniPython output parity and
  adding a local soft-keyword call subset for both names.
- Kept the differential harness compatible with the default macOS Python 3.9
  oracle by accepting its older generic `invalid syntax` wording for current
  CPython's newer invalid-non-printable-character SyntaxError text.

Completed in the SyntaxError multiline diagnostic pass:

- Migrated CPython
  `Lib/test/test_syntax.py::SyntaxErrorTestCase::test_multiline_compiler_error_points_to_the_end`
  by changing duplicate keyword call parsing to CPython's
  `keyword argument repeated: name` wording and pinning the diagnostic to the
  repeated keyword on line 3.
- Migrated CPython
  `Lib/test/test_syntax.py::SyntaxErrorTestCase::test_multiline_string_concat_missing_comma_points_to_last_string`
  by recognizing adjacent string literal concatenation followed by a missing
  comma before a keyword-like argument and reporting `Perhaps you forgot a
  comma` with the span on the final adjacent string token.
- Extended `cpython_syntax_error_message_parity_subset`,
  `cpython_invalid_call_argument_helper_rules_subset`, and
  `cpython_multiline_string_concat_missing_comma_subset` to cover both the
  CPython-facing message parity and MiniPython's exact diagnostic spans.

Completed in the f-string AST end-position pass:

- Migrated CPython `Lib/test/test_ast/test_ast.py::EndPositionTests`
  f-string source-location cases into
  `cpython_ast_fstring_end_positions_first_pass_subset`.
- Added AST location annotation for `JoinedStr` replacement expressions by
  reusing the CPython-style split f-string token stream to annotate
  `FormattedValue.value` nodes, including multi-line replacement expressions.

Completed in the AST dump indentation pass:

- Migrated CPython `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_dump`
  into `cpython_ast_dump_plain_first_pass_subset`, pinning plain
  `ast.dump()` output for default rendering, `annotate_fields=False`, and
  `include_attributes=True`.
- Added `cpython_ast_dump_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_dump` into direct method-level evidence.
- Migrated CPython `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_dump_indent`
  into `cpython_ast_dump_indent_first_pass_subset`.
- Added `cpython_ast_dump_indent_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_dump_indent` into direct method-level evidence.
- Implemented CPython-style `ast.dump(indent=...)` formatting for integer and
  string indentation, including `annotate_fields=False` and
  `include_attributes=True` output.
- Aligned default `ast.dump(..., show_empty=False)` behavior for load contexts
  so implicit `ctx=Load()` fields are omitted like current CPython.

Completed in the AST dump incomplete/show-empty pass:

- Migrated first-pass CPython
  `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_dump_incomplete` cases
  into `cpython_ast_dump_incomplete_first_pass_subset`, covering incomplete
  `Raise`, partially populated `arguments`, omitted defaults, positional
  `annotate_fields=False` buffering, and attribute output.
- Added `cpython_ast_dump_incomplete_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_dump_incomplete` into direct method-level evidence.
- Migrated first-pass CPython
  `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_dump_show_empty` cases
  into `cpython_ast_dump_show_empty_first_pass_subset`, covering
  `show_empty=True` / `show_empty=False` behavior for supported hand-built and
  parsed public AST nodes.
- Added `cpython_ast_dump_show_empty_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_dump_show_empty` into direct method-level evidence.
- Tightened `ast.dump()` default-field skipping so optional `None` defaults,
  empty-list defaults, and implicit `Load` contexts follow CPython's
  `show_empty` behavior without hiding real payloads like `Constant(None)` or
  `Constant([])`.

Completed in the AST literal-eval diagnostics pass:

- Migrated first-pass CPython
  `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_literal_eval_malformed_dict_nodes`,
  `test_literal_eval_trailing_ws`, and `test_literal_eval_malformed_lineno`
  behavior into `cpython_ast_literal_eval_diagnostics_first_pass_subset`.
- Split eval-mode AST parsing from `literal_eval()` input normalization so
  public `ast.literal_eval()` strips only leading spaces and tabs, while
  newline-prefixed indentation now raises `IndentationError`.
- Added CPython-style malformed-node line-number text when a rejected public
  AST node has a truthy `lineno` attribute.

Completed in the AST parse invalid-AST pass:

- Migrated CPython `Lib/test/test_ast/test_ast.py::AST_Tests::test_parse_invalid_ast`
  into `cpython_ast_parse_invalid_ast_subset`, covering the `optimize`
  `-1`, `0`, `1`, and `2` cases.
- Tightened `ast.parse()` so public AST input is accepted only for complete
  root nodes (`Module`, `Expression`, `Interactive`, and `FunctionType`) and
  non-root nodes such as `ast.Constant(42)` raise `TypeError`.

Completed in the AST parse `__debug__` optimization pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_optimization_levels__debug__`
  into `cpython_ast_parse_optimize_debug_subset`, covering `optimize` values
  `-1`, `0`, `1`, and `2`.
- Added a public-AST optimization pass for `ast.parse()` /
  `compile(..., ast.PyCF_ONLY_AST, optimize=...)` that folds load-context
  `__debug__` names to `ast.Constant(False)` only when `optimize > 0`, while
  preserving location attributes on the replacement node.

Completed in the AST percent-format optimization pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::ASTOptimizationTests::test_folding_format`
  into `cpython_ast_optimization_format_folding_subset`.
- Extended the public-AST optimization pass so `ast.parse(..., optimize=1)`
  rewrites `'%s' % (a,)` from a `BinOp` with `Mod` into a `JoinedStr`
  containing a `FormattedValue` with `conversion=115`, while leaving
  `optimize=-1` unoptimized.

Completed in the AST docstring optimization pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_docstring_optimization_single_node`
  into `cpython_ast_docstring_optimization_single_node_subset`, covering
  class, function, and async-function docstring-only bodies.
- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_docstring_optimization_multiple_nodes`
  into `cpython_ast_docstring_optimization_multiple_nodes_subset`, covering
  class, function, and async-function bodies where a docstring is followed by
  another statement.
- Extended the public-AST optimization pass so `optimize=2` removes docstring
  expression statements from class/function/async-function bodies, replacing a
  docstring-only body with a `Pass` node whose location begins at the original
  docstring statement.

Completed in the public-AST location validation pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_invalid_position_information`
  into `cpython_ast_invalid_position_information_subset`, covering invalid
  `lineno` / `end_lineno` and `col_offset` / `end_col_offset` ranges during
  `compile(public_ast, ...)`.
- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_negative_locations_for_compile`
  into `cpython_ast_negative_locations_compile_subset`, covering accepted
  negative-location cases for `compile()` and `ast.parse(..., optimize=2)`.
- Extended compile-from-public-AST validation to check explicit CPython-style
  line and column ranges on statements, expressions, and assignment targets
  without rejecting AST nodes whose location attributes are absent.

Completed in the compile-only-AST pass:

- Migrated first-pass CPython
  `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_parse` behavior into
  `cpython_ast_compile_only_ast_first_pass_subset`.
- Added `cpython_ast_parse_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_parse` into direct method-level evidence.
- Implemented `compile(source, filename, mode, ast.PyCF_ONLY_AST)` so it returns
  public AST nodes for supported `exec`, `eval`, and `single` modes instead of
  rejecting the `PyCF_ONLY_AST` flag.

Completed in the AST parse-in-error context pass:

- Migrated first-pass CPython
  `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_parse_in_error`
  behavior into `cpython_ast_parse_in_error_first_pass_subset`.
- Added `cpython_ast_parse_in_error_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_parse_in_error` into direct method-level evidence.
- Confirmed MiniPython preserves the active exception as
  `SyntaxError.__context__` when `ast.literal_eval()` raises a parser
  `SyntaxError` inside an `except` block.

Completed in the AST type-ignore line-number pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_increment_lineno_on_module`
  into `cpython_ast_increment_lineno_on_module_type_ignores_subset`.
- Added public `ast.TypeIgnore` nodes for `ast.parse(..., type_comments=True)`
  and CPython-compatible `# type: ignore<tag>` token classification for ASCII
  non-alphanumeric tag starters such as `@`.
- Extended `ast.increment_lineno()` so `TypeIgnore.lineno` moves with
  `Module.type_ignores`, matching CPython's special handling for type-ignore
  nodes.

Completed in the AST ImportFrom validation pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::ASTHelpers_Test::test_bad_integer` and
  `test_level_as_none` behavior into
  `cpython_ast_importfrom_level_none_validation_subset`.
- Added compile-from-public-AST validation for explicit `lineno=None` /
  `col_offset=None` on statement nodes while preserving MiniPython's existing
  first-pass support for hand-built nodes with missing location fields.
- Matched CPython's public-AST `ImportFrom.level=None` behavior by compiling it
  as level 0, and expanded the `time` module surface enough for `from time
  import sleep` binding checks.

Completed in the AST ImportFrom exact-method pass:

- Added `cpython_ast_bad_integer_exact_subset` and
  `cpython_ast_level_as_none_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_bad_integer` and `test_level_as_none` out of the older
  combined ImportFrom validation smoke test.
- These keep the public-AST `ImportFrom.level=None` behavior as direct CPython
  method-level evidence.

Completed in the compile-from-public-AST pass:

- Added `cpython_ast_compile_public_ast_first_pass_subset` as first-pass
  CPython `Lib/test/test_ast/test_ast.py::test_snippets` compile-subtest
  coverage for representative public `Module`, `Expression`, `Interactive`,
  and hand-built `Module` trees.
- Implemented a public-AST-to-internal-AST bridge for common executable nodes:
  `Module`, `Expression`, `Interactive`, `Expr`, `Assign`, simple control-flow
  statements, `Name`, `Constant`, `BinOp`, `UnaryOp`, `BoolOp`, `Compare`,
  `Call`, `List`, `Tuple`, `Set`, `Dict`, `IfExp`, `Subscript`, and `Slice`.
- Added second-pass compile-from-public-AST coverage for more statement and
  expression nodes: `AnnAssign`, `TypeAlias`, `AugAssign`, `Delete`, `Import`,
  `ImportFrom`, `Global`, `Nonlocal`, `Assert`, `AsyncFunctionDef`,
  `AsyncFor`, `AsyncWith`, `TryStar`, `NamedExpr`, `Yield`, `YieldFrom`,
  `Await`, `Starred`, `ListComp`, `SetComp`, `GeneratorExp`, `DictComp`, and
  `Lambda`.
- Added `cpython_ast_compile_public_ast_match_second_pass_subset`, covering
  parser-generated and hand-built public `Match`, `match_case`, `MatchValue`,
  `MatchSingleton`, `MatchSequence`, `MatchMapping`, `MatchClass`,
  `MatchStar`, `MatchAs`, and `MatchOr` nodes through `compile(public_ast, ...)`
  and VM execution.
- Added `cpython_ast_compile_public_ast_interpolated_string_second_pass_subset`,
  covering parser-generated and hand-built public `JoinedStr`,
  `FormattedValue`, `TemplateStr`, and `Interpolation` nodes through
  `compile(public_ast, ...)`, including conversion codes and nested `JoinedStr`
  format specs.
- Added `cpython_ast_node_transformer_first_pass_subset`, covering the current
  CPython `NodeTransformerTests` scenarios plus the supporting `NodeVisitor`
  dispatch path: single-field removal, list-field removal, list-return
  replacement, in-place node mutation, and node replacement.
- Added `cpython_ast_constant_compile_first_pass_subset`, porting all current
  CPython `ConstantTests` methods. It covers public `ast.Constant` compile
  validation for supported singleton/value constants, invalid list constants,
  illegal assignment targets, module docstring lookup, `literal_eval()` after
  replacing `BinOp` operands, string-prefix `kind` metadata, and the supported
  bytecode/disassembly slice by observing `LOAD_CONST` values through
  `dis.hasconst` and `dis.get_instructions()`, including tuple constants.
- Added `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset` as
  the first method-oriented `GrammarTests` slice. It ports `test_eval_input`,
  the executable subset of `test_var_annot_basics`,
  `test_var_annot_syntax_errors`, the target/annotation execution-order parts
  of `test_var_annot_basic_semantics`, and
  `test_annotations_inheritance`.
- Fixed two runtime/compiler gaps exposed by that slice: annotation targets
  without a value now still evaluate attribute/subscript target expressions,
  and classes without local annotations no longer inherit base-class
  `__annotations__`.
- Corrected the manifest split for current `Lib/test/test_grammar.py`: there
  are no module-level `test_*` functions in the local CPython source; the file
  currently has 14 `TokenTests` methods and 61 `GrammarTests` methods.
- Expanded the `GrammarTests` manifest into a complete 61-row method audit so
  remaining work can be driven method-by-method instead of through one broad
  partial group.
- Extended `cpython_grammar_tests_eval_and_var_annotation_first_pass_subset`
  with CPython's annotated-assignment execution-order cases, function-local
  annotation binding behavior, class-body bad-target failures, annotated RHS
  tuple/yield/starred forms, and class-body exceptions that are catchable by an
  enclosing `try`.
- Added a minimal `typing` module surface with `Tuple` so
  `test_var_annot_rhs` now uses the CPython-style
  `from typing import Tuple` / `Tuple[int, ...]` source shape instead of a
  test-local `Tuple = tuple` stand-in. The method audit now marks
  `test_var_annot_rhs` as ported.
- Added synthetic `test.typinganndata` fixture modules for the current
  `test_var_annot_module_semantics` method and a minimal PEP 604 union value
  for type-object `|` expressions. The migrated test now checks
  `test.__annotations__`, `ann_module.__annotations__`,
  `ann_module.M.__annotations__`, and `ann_module2.__annotations__` through the
  CPython import paths, including `typing.Tuple[int, int]` and `int | float`.
  The method audit now marks `test_var_annot_module_semantics` as ported.
- Added the synthetic `test.typinganndata.ann_module3` fixture for
  `test_var_annot_in_module`. The migrated method now imports the same CPython
  fixture path and checks that `f_bad_ann()`, `g_bad_ann()`, and
  `D_bad_ann(5)` raise catchable `NameError`s with CPython-style messages. The
  method audit now marks `test_var_annot_in_module` as ported.
- Migrated `test_var_annot_simple_exec` to the current CPython deferred
  annotation shape for separate `exec(source, globals, locals)` scopes. MiniPython
  now exposes a minimal `annotationlib.Format.VALUE` surface and installs a
  locals-only `__annotate__` function that returns the collected annotation dict
  for the supported exec subset. The method audit now marks
  `test_var_annot_simple_exec` as ported.
- Added `cpython_grammar_tests_funcdef_first_pass_subset`, a method-level port
  of CPython `GrammarTests::test_funcdef`. The migrated coverage exercises
  function `__code__.co_varnames`, ordinary/defaulted/vararg calls, keyword-only
  parameters, invalid parameter and call-unpack syntax, keyword-after-star and
  `**kwargs` calls, evaluated annotations including private-name mangling,
  PEP 614 decorator expressions, closure capture shapes, and trailing-comma
  parameter lists. MiniPython now exposes a minimal function `__code__` object,
  callable `__call__` attributes for functions/methods, catchable TypeError
  conversion for call-unpack argument collection, and order-insensitive rich
  equality for dict values. The method audit now marks `test_funcdef` as ported.
- Audited CPython `GrammarTests::test_lambdef` against the existing executable
  lambda coverage. Added the missing uncalled `lambda: a[d]` expression-boundary
  case, while retaining the adapted bool-list output for CPython's
  `self.assertEqual(l3(), [0, 1, 0])` comparison. The method audit now marks
  `test_lambdef` as ported.
- Audited CPython `GrammarTests::test_simple_stmt` and added the exact
  top-level and function-body `x = 1; pass; del x` semicolon shapes, including
  the trailing semicolon inside the function body. The method audit now marks
  `test_simple_stmt` as ported.
- Added `cpython_grammar_tests_expr_stmt_subset`, a method-level port of
  CPython `GrammarTests::test_expr_stmt`. The migrated coverage exercises pure
  expression statements, tuple-valued assignments, chained assignments,
  unpacking targets, the mixed chained/unpacking assignment, and the two invalid
  assignment-target cases from the CPython method. The method audit now marks
  `test_expr_stmt` as ported.
- Added `cpython_grammar_tests_former_statements_refer_to_builtins_subset`, a
  method-level port of CPython
  `GrammarTests::test_former_statements_refer_to_builtins`. The parser now
  emits CPython-style missing-parentheses diagnostics for statement-shaped
  `print foo` and `exec foo` at top level, inline-suite, and indented-block
  positions, while malformed parenthesized variants stay on the generic syntax
  error path. The method audit now marks
  `test_former_statements_refer_to_builtins` as ported.
- Added `cpython_grammar_tests_del_stmt_subset`, a method-level port of CPython
  `GrammarTests::test_del_stmt`. The migrated source runs the CPython delete
  sequence across names, nested tuple/list delete targets, empty tuple delete,
  list slice delete, and compile-only complex delete targets. The method audit
  now marks `test_del_stmt` as ported.
- Audited CPython `GrammarTests::test_pass_stmt` and added the bare `pass`
  method shape to the existing pass-statement grammar test. The method audit now
  marks `test_pass_stmt` as ported.
- Audited CPython `GrammarTests::test_break_stmt` and added the exact
  `while 1: break` method shape to the existing break/continue grammar test. The
  method audit now marks `test_break_stmt` as ported.
- Audited CPython `GrammarTests::test_continue_stmt` and
  `GrammarTests::test_break_continue_loop` against the existing break/continue
  grammar test. Added the exact inline `while i: i = 0; continue` method shape;
  the try/except, try/finally, and nested continue-then-break regression cases
  were already covered with observable outputs. The method audit now marks both
  methods as ported.
- Audited CPython `GrammarTests::test_return` and expanded the return-statement
  test with the method-level `g1`, `g2`, and `g3` functions, including
  unparenthesized starred tuple return and rejection of `class foo:return 1`. The
  method audit now marks `test_return` as ported.
- Audited CPython `GrammarTests::test_control_flow_in_finally` against
  `cpython_control_flow_in_finally_override_subset`. Existing coverage already
  ports all current method cases: six break-in-finally overrides, six
  continue-in-finally overrides, three return-in-finally overrides, and the four
  issue #37830 return-with-break/continue-in-finally cases. The method audit now
  marks `test_control_flow_in_finally` as ported.
- Audited CPython `GrammarTests::test_yield` and expanded the yield grammar test
  with method-level standalone yield/yield-from definitions, yield RHS
  definitions, implicit tuple yield forms, parenthesized subexpression and
  call-argument yield forms, unparenthesized syntax rejections, top-level and
  class-scope rejections, and the annotation-yield rejection. The method audit
  now marks `test_yield` as ported.
- Audited CPython `GrammarTests::test_yield_in_comprehensions` and expanded the
  yield/comprehension tests with the method-level allowed outer-iterable
  yield/yield-from cases plus rejected yield/yield-from in list, set, dict, and
  generator comprehension element/filter/inner-iterable/target/module/class
  positions. The method audit now marks `test_yield_in_comprehensions` as
  ported.
- Audited CPython `GrammarTests::test_raise` and added the method-level
  `RuntimeError('just testing')` and `KeyboardInterrupt` try/except shapes to
  the raise/try grammar test. The method audit now marks `test_raise` as
  ported.
- Audited CPython `GrammarTests::test_import` and added the missing
  parenthesized `from sys import (path, argv)` method shape. The import grammar
  test now covers every current method-level ordinary and from-import form, and
  the method audit marks `test_import` as ported.
- Audited CPython `GrammarTests::test_global` and `GrammarTests::test_nonlocal`.
  Added the method-level multi-name global declarations and the nested
  `nonlocal x` / `nonlocal x, y` declarations. The method audit now marks both
  methods as ported.
- Audited CPython `GrammarTests::test_assert` and expanded the assert grammar
  test with method-level truthy asserts, message expressions, lambda expressions,
  and the non-failing `assert True` / `assert True, msg` cases. The method audit
  now marks `test_assert` as ported.
- Audited CPython `GrammarTests::test_assert_failures`,
  `GrammarTests::test_assert_syntax_warnings`, and
  `GrammarTests::test_assert_warning_promotes_to_syntax_error`. The assert
  grammar test now catches failing assertions and checks `AssertionError.args`
  like CPython, and MiniPython's static warning path now emits and promotes the
  non-empty tuple-condition warning for `assert(x, "msg")`,
  `assert(False, "msg")`, and `assert(False,)`. The method audit now marks all
  three methods as ported.
- Audited CPython `GrammarTests::test_if` and `GrammarTests::test_while`.
  Added the exact inline pass-only `if`/`elif`/`else` and `while 0` method
  shapes, including CPython's Issue1920 `while 0 ... else` preservation case.
  The method audit now marks both methods as ported.
- Audited CPython `GrammarTests::test_for` and expanded the for-loop grammar
  test with the method-level empty-iterable `for ... else`, growing
  sequence-protocol iteration through `__getitem__`, tuple-unpack loop target,
  and starred iterable sequence shapes. The method audit now marks `test_for`
  as ported.
- Audited CPython `GrammarTests::test_try` and `GrammarTests::test_try_star`.
  Added the method-level typed, bare, tuple, comma-list, and tuple-`as` handler
  shapes for `except` and `except*`, plus `try/finally` and invalid
  attribute/subscript handler targets. Existing runtime coverage already checks
  exception matching, causes, contexts, ExceptionGroup splitting, and
  except-star restrictions. The method audit now marks both methods as ported.
- Audited CPython `GrammarTests::test_suite` and added the method-level inline
  suite, indented pass suite, and comment-only-line/pass sequence inside an
  indented suite. The method audit now marks `test_suite` as ported.
- Audited CPython `GrammarTests::test_test` and added the method-level boolean
  expression `if ...: pass` shapes for `not`, `and`, `or`, nested `not`, and the
  mixed boolean chain. The method audit now marks `test_test` as ported.
- Audited CPython `GrammarTests::test_comparison` and
  `GrammarTests::test_comparison_is_literal`. Added method-level comparison
  shapes for equality, ordering, identity, membership, and the long mixed
  chained comparison, plus CPython-style `is` / `is not` literal
  `SyntaxWarning` coverage and singleton no-warning checks. The method audit now
  marks both comparison methods as ported.
- Audited CPython `GrammarTests::test_warn_missed_comma` and added the
  method-level static `SyntaxWarning` coverage from CPython's compiler checks:
  non-callable literal/display calls, non-subscriptable literal/display
  subscripts, invalid static index types, and the no-warning cases for
  lambda calls, name/int/bool/slice indexes, and dict tuple keys. The method
  audit now marks `test_warn_missed_comma` as ported.
- Audited CPython `GrammarTests::test_binary_mask_ops`,
  `GrammarTests::test_shift_ops`, `GrammarTests::test_additive_ops`,
  `GrammarTests::test_multiplicative_ops`, and `GrammarTests::test_unary_ops`.
  Added the exact method-level assignment shapes for bitwise, shift, additive,
  multiplicative, and unary expressions, plus executable value checks for each
  operator family. The method audit now marks all five operator methods as
  ported.
- Audited CPython `GrammarTests::test_selectors` and added a method-level
  executable selector slice for module attribute calls, `sys.path[0]`,
  `sys.modules['time'].time()`, string index/slice forms, and tuple-key dict
  selector assignments with deterministic sorting. The `sys` module fixture now
  exposes a CPython-like empty-string path entry and a minimal `modules`
  dictionary containing `time`. The method audit now marks `test_selectors` as
  ported.
- Audited CPython `GrammarTests::test_atoms` and added a method-level atom
  slice for grouped expressions, tuple/list/dict/set displays, boolean
  expression keys and values inside dict displays, bare name self-assignment,
  string atoms, and number atoms. The method audit now marks `test_atoms` as
  ported.
- Audited CPython `GrammarTests::test_classdef` and added a method-level class
  definition slice for bare and empty-parentheses classes, single and multiple
  inheritance, class-body method definitions, simple class decorators, and PEP
  614 class decorator expressions including boolean, named-expression, lambda,
  subscript, decorator-call-chain, and `__call__.__call__` decorators. The
  method audit now marks `test_classdef` as ported.
- Audited CPython `GrammarTests::test_dictcomps`,
  `GrammarTests::test_listcomps`, `GrammarTests::test_genexps`, and
  `GrammarTests::test_comprehension_specials`. Added method-level executable
  slices for dict comprehensions, list-comprehension strip/arithmetic/filtering
  and nested-for shapes, nested list comprehensions inside lambdas and helper
  functions, invalid list-comprehension syntax, generator StopIteration and
  TypeError behavior, nested generator/list sums, parenthesized-generator
  syntax errors, outermost iterable precomputation, lazy inner-expression
  lookup, adjacent comprehension filters, and single-element tuple-unpack
  targets. The method audit now marks all four comprehension methods as ported.
- Audited CPython `GrammarTests::test_with_statement` and added a method-level
  with-statement slice for ordinary and parenthesized context-manager forms,
  including no target, simple target, tuple-unpack target, multiple managers,
  mixed `as`/bare managers, trailing commas, and three-manager parenthesized
  groups. The test also checks target bindings and nested cleanup order. The
  method audit now marks `test_with_statement` as ported.
- Audited CPython `GrammarTests::test_if_else_expr` and added a method-level
  conditional-expression slice for lambda/list-comprehension forms,
  `_checkeval` branch short-circuiting, boolean precedence, `not` precedence,
  arithmetic branches, and comparison/else precedence. The method audit now
  marks `test_if_else_expr` as ported.
- Audited CPython `GrammarTests::test_paren_evaluation` and added a
  method-level parenthesized-evaluation slice for floor-division grouping and
  identity comparisons where parentheses change comparison-chain grouping. The
  method audit now marks `test_paren_evaluation` as ported.
- Audited CPython `GrammarTests::test_matrix_mul` and added a method-level
  matrix-multiplication slice for `@`, `@=`, `__matmul__`, `__imatmul__`, and
  attribute assignment performed by in-place matrix multiplication. The method
  audit now marks `test_matrix_mul` as ported.
- Audited CPython `GrammarTests::test_async_await` and added a method-level
  async-await slice for async function metadata, `inspect.CO_COROUTINE`,
  function `__code__.co_flags`, decorators on async functions, and user-defined
  function attributes set with `setattr()`. The method audit now marks
  `test_async_await` as ported.
- Audited CPython `GrammarTests::test_async_for` and added a method-level
  async-for slice for empty async iteration, tuple-unpack targets, `else`, and
  final user exception propagation. The method audit now marks
  `test_async_for` as ported.
- Audited CPython `GrammarTests::test_async_with` and added a method-level
  async-with slice for no-target managers, `as` targets, tuple-unpack targets,
  multiple managers, and mixed `as`/bare manager forms. The method audit now
  marks `test_async_with` as ported.
- Audited CPython `GrammarTests::test_complex_lambda` and added a method-level
  complex-lambda slice for lambda keyword arguments inside a multi-line f-string
  replacement expression. The method audit now marks `test_complex_lambda` as
  ported, and the `GrammarTests` source group is now fully ported.

Completed in the PEP 758 feature-version pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_pep758_except_without_parens`
  and `::test_pep758_except_star_without_parens` by making
  `ast.parse(..., feature_version=(3, 14))` accept unparenthesized multiple
  exception types in `except` and `except*`, while
  `feature_version=(3, 13)` raises `SyntaxError`.
- Added the single-expression acceptance cases from
  `::test_pep758_except_with_single_expr`, preserving ordinary single
  exception and parenthesized tuple exception parsing for both feature
  versions.
- `ast.parse()` now reads the `feature_version` argument and applies this
  PEP 758 syntax gate before producing public AST nodes.

Completed in the broader AST feature-version pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_positional_only_feature_version`
  by accepting positional-only parameters at `feature_version=(3, 8)` and
  rejecting them at `(3, 7)` for both function definitions and lambdas.
- Migrated `::test_assignment_expression_feature_version` by accepting
  assignment expressions at `(3, 8)` and rejecting them at `(3, 7)`.
- Migrated `::test_pep750_tstring` by accepting t-strings at `(3, 14)` and
  rejecting them at `(3, 13)`.
- Migrated `::test_exception_groups_feature_version` by accepting `except*`
  at `(3, 11)` and rejecting it at `(3, 10)`.
- Migrated `::test_type_params_feature_version` and
  `::test_type_params_default_feature_version` by rejecting type-parameter
  syntax before `(3, 12)` and type-parameter defaults before `(3, 13)`.
- Migrated `::test_invalid_major_feature_version` and
  `::test_conditional_context_managers_parse_with_low_feature_version`, keeping
  invalid major versions as `ValueError` and allowing the conditional context
  manager regression sample at `(3, 8)`.

Completed in the AST Name identifier validation pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_invalid_identifier`,
  `::test_constant_as_name`, and `::test_constant_as_unicode_name`.
- `compile(public_ast, ...)` now rejects `ast.Name` nodes whose `id` field is
  not a string, raising a catchable `TypeError` with the CPython identifier
  diagnostic fragment.
- Public AST compilation and `ast.parse()` now reject `Name.id` values
  normalized to `True`, `False`, or `None`, raising `ValueError` instead of
  allowing those singleton constants to masquerade as identifiers.
- The parse-side check covers the CPython Unicode-normalized byte-source cases:
  `b"Tru\xe1\xb5\x89"`, `b"Fal\xc5\xbfe"`, and `b"N\xc2\xbane"`.

Completed in the AST validator basic-error pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::AST_Tests::test_invalid_sum`,
  `::test_invalid_constant`, and `::test_empty_yield_from`.
- Added the public `ast.expr` abstract constructor surface needed to build the
  CPython invalid-sum tree, then made `compile(public_ast, ...)` reject it with
  the expected `but got expr()` diagnostic fragment.
- `ast.Constant` validation now reports type-object payloads as
  `invalid type in Constant: type`, including nested tuple and frozenset
  constants containing a type object.
- `YieldFrom.value=None` now raises a catchable `ValueError` with the CPython
  required-field diagnostic fragment instead of falling through to a generic
  expression type error.

Completed in the AST validator load-context pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::ASTValidatorTests::test_module` by rejecting
  `ast.Interactive([ast.Expr(ast.Name(..., ast.Store()))])` in `single` mode
  and `ast.Expression(ast.Name(..., ast.Store()))` in `eval` mode with the
  expected `must have Load context` diagnostic fragment.
- Added the first load-context cases from `ASTValidatorTests::test_expr`,
  `::test_boolop`, `::test_unaryop`, and `::test_yield`, covering nested
  invalid `Name(..., Store())` nodes under expression statements, boolean
  operations, unary operations, `Yield`, and `YieldFrom`.
- Public AST compilation now validates `ctx=Load()` on expression-position
  `Name`, `Attribute`, `Subscript`, `Starred`, `List`, and `Tuple` nodes before
  lowering them to MiniPython's internal syntax AST.

Completed in the AST validator BoolOp/Compare shape pass:

- Migrated the remaining CPython
  `Lib/test/test_ast/test_ast.py::ASTValidatorTests::test_boolop` structure
  checks by rejecting empty and one-element `BoolOp.values` lists with the
  expected `less than 2 values` diagnostic fragment.
- Public AST compilation now rejects direct `None` entries in required
  expression positions with `ValueError: None disallowed`, preserving
  `ast.Constant(None)` as the valid way to represent a Python `None` literal.
- Migrated the first CPython `ASTValidatorTests::test_compare` checks by
  rejecting `Compare` nodes with no comparators and nodes where the comparator
  count differs from the operator count.

Completed in the AST validator first exact pass:

- Added `cpython_ast_validator_module_exact_subset`, splitting CPython
  `ASTValidatorTests.test_module` out of the older broad load-context test.
- Added `cpython_ast_validator_delete_exact_subset`,
  `cpython_ast_validator_assign_exact_subset`, and
  `cpython_ast_validator_augassign_exact_subset`, splitting CPython
  `test_delete`, `test_assign`, and `test_augassign` into direct method-level
  evidence.
- Added `cpython_ast_validator_core_expr_exact_subset`, splitting CPython
  `test_expr`, `test_boolop`, `test_unaryop`, `test_yield`, and
  `test_compare` into direct method-level evidence while preserving the two
  valid compare cases.

Completed in the AST validator statement exact pass:

- Added `cpython_ast_validator_funcdef_exact_subset`,
  `cpython_ast_validator_classdef_exact_subset`,
  `cpython_ast_validator_try_exact_subset`, and
  `cpython_ast_validator_try_star_exact_subset`, splitting CPython
  `test_funcdef`, `test_classdef`, `test_try`, and `test_try_star` into
  direct method-level evidence.
- Added `cpython_ast_validator_for_exact_subset`,
  `cpython_ast_validator_while_exact_subset`,
  `cpython_ast_validator_if_exact_subset`, and
  `cpython_ast_validator_with_exact_subset`, splitting CPython
  `test_for`, `test_while`, `test_if`, and `test_with` into direct
  method-level evidence.
- Added `cpython_ast_validator_raise_exact_subset`,
  `cpython_ast_validator_assert_exact_subset`,
  `cpython_ast_validator_import_exact_subset`,
  `cpython_ast_validator_importfrom_exact_subset`,
  `cpython_ast_validator_global_exact_subset`, and
  `cpython_ast_validator_nonlocal_exact_subset`, splitting CPython
  `test_raise`, `test_assert`, `test_import`, `test_importfrom`,
  `test_global`, and `test_nonlocal` into direct method-level evidence.

Completed in the AST validator expression-context pass:

- Migrated the first CPython `ASTValidatorTests` checks for `Lambda`,
  `IfExp`, `Dict`, `Set`, `Call`, `Attribute`, and `Subscript`.
- Exposed the public `ast.Set` constructor and connected it to the existing
  public-AST compile path for MiniPython set literal expressions.
- Updated public `ast.Dict` validation to reject mismatched `keys` / `values`
  lists as a catchable `ValueError` with CPython's
  `same number of keys as values` diagnostic fragment.
- Extended the validator coverage for nested invalid `Load` contexts and
  direct `None` expression-list entries through calls, attributes, subscripts,
  slices, sets, dictionaries, lambdas, and conditional expressions.

Completed in the AST validator expression exact pass:

- Added `cpython_ast_validator_lambda_exact_subset`,
  `cpython_ast_validator_ifexp_exact_subset`,
  `cpython_ast_validator_dict_exact_subset`,
  `cpython_ast_validator_set_exact_subset`,
  `cpython_ast_validator_call_exact_subset`,
  `cpython_ast_validator_attribute_exact_subset`, and
  `cpython_ast_validator_subscript_exact_subset`, splitting CPython
  `test_lambda`, `test_ifexp`, `test_dict`, `test_set`, `test_call`,
  `test_attribute`, and `test_subscript` into direct method-level evidence.
- Added `cpython_ast_validator_starred_exact_subset`,
  `cpython_ast_validator_list_exact_subset`, and
  `cpython_ast_validator_tuple_exact_subset`, splitting CPython
  `test_starred`, `test_list`, and `test_tuple` into direct method-level
  evidence.

Completed in the AST validator comprehension exact pass:

- Added `cpython_ast_validator_listcomp_exact_subset`,
  `cpython_ast_validator_setcomp_exact_subset`,
  `cpython_ast_validator_generatorexp_exact_subset`, and
  `cpython_ast_validator_dictcomp_exact_subset`, splitting CPython
  `test_listcomp`, `test_setcomp`, `test_generatorexp`, and `test_dictcomp`
  into direct method-level evidence.
- With this pass, all 40 current CPython `ASTValidatorTests` methods now have
  method-level Rust evidence.

Completed in the AST validator statement-context pass:

- Migrated the next CPython `ASTValidatorTests` statement checks for `Delete`,
  `Assign`, `AugAssign`, `For`, `While`, `If`, `With`, `Raise`, `Assert`,
  `Import`, `ImportFrom`, `Global`, and `Nonlocal`.
- Public AST compilation now validates `Store` context for assignment-like
  targets, `Del` context for delete targets, and `Load` context for nested
  expression positions inside those statements.
- Added CPython-style public-AST `ValueError` checks for empty statement
  bodies, empty `targets` / `items` / `names` lists, direct `None` entries in
  target lists, `Raise` nodes with a cause but no exception, and negative
  `ImportFrom.level` values.

Completed in the AST validator definition-and-try pass:

- Migrated the next CPython `ASTValidatorTests` checks for `FunctionDef`,
  `ClassDef`, `Try`, and `TryStar`.
- Public AST compilation now rejects empty function/class/try/except-handler
  bodies, invalid function argument annotations/defaults, invalid class bases,
  class keywords, and class decorators, plus `Try` / `TryStar` nodes that have
  neither handlers nor final bodies or that have `orelse` without handlers.
- Added Load-context validation for exception handler type expressions and
  preserved the accepted minimal `ast.FunctionDef('x', ast.arguments(),
  [ast.Pass()])` constructor shape.

Completed in the AST validator FunctionDef pattern-matching pass:

- Migrated CPython
  `Lib/test/test_ast/test_ast.py::ASTValidatorTests::test_funcdef_pattern_matching`.
- Built-in AST node classes now expose class-pattern positional matching
  through their constructor field order, matching CPython's `_fields` /
  `__match_args__` behavior for supported public AST nodes. The migrated test
  proves nested `ast.FunctionDef`, `ast.arguments`, `ast.arg`, `ast.Pass`,
  `ast.Name`, and `ast.Load` patterns match a parsed function definition.

Completed in the AST validator comprehension-and-sequence pass:

- Migrated the next CPython `ASTValidatorTests` checks for `ListComp`,
  `SetComp`, `GeneratorExp`, `DictComp`, `Starred`, `List`, and `Tuple`.
- Public AST compilation now rejects comprehension expressions whose
  `generators` list is empty before validating the element/key/value
  expressions, matching CPython's diagnostic priority for
  `comprehension with no generators`.
- Extended recursive Store/Load validation through comprehension targets,
  iterables, filters, subscript tuple slices, starred assignment targets, and
  direct `None` / invalid-context entries in list and tuple expressions.

Completed in the AST validator match-pattern pass:

- Migrated CPython `ASTValidatorTests::test_match_validation_pattern` for
  invalid public `MatchValue`, `MatchSingleton`, `MatchSequence`,
  `MatchMapping`, `MatchClass`, `MatchAs`, `MatchOr`, and `MatchStar` nodes.
- Public AST compilation now rejects malformed match patterns as `ValueError`
  before lowering into bytecode, including invalid singleton/value patterns,
  invalid class targets, mismatched mapping/class pattern list lengths, invalid
  capture names, top-level star patterns, and invalid sequence-star captures.

Started in the AST validator stdlib-validation pass:

- Added a first file-backed migration seed for CPython
  `ASTValidatorTests::test_stdlib_validates`.
- Expanded `cpython_ast_validator_stdlib_compile_seed_subset` to the current
  CPython `STDLIB_FILES` set: all 150 top-level `.py` files from
  `/Volumes/samsung/GitHub/cpython/Lib`, plus `test/test_grammar.py` and
  `test/test_unpack_ex.py`. This covers small import shims, token/opcode
  metadata modules, pure-Python utility modules, the remaining large modules
  through `typing.py`, and CPython's two extra stdlib-validation test files.
- Added `cpython_ast_validator_stdlib_recursive_compile_seed_subset` for 255
  recursive `.py` files from `__phello__`, `_pyrepl`, `asyncio`,
  `collections`, `compression`, `concurrent`, `ctypes`, `curses`, `dbm`,
  `email`, and `encodings`.
- Relative `ImportFrom` now compiles with an import level carried in bytecode,
  so compile-only CPython stdlib validation can pass package-relative imports
  such as `_pyrepl/__main__.py`; the default VM still raises `ImportError` for
  unresolved relative imports at runtime.
- The lexer now treats `from ._module import name` as a relative import module
  name instead of the invalid numeric-looking `._5` form, covering the
  `_pyrepl/reader.py` regression.
- MiniPython now keeps exception handler types as AST expressions internally,
  preserving public-AST shape for `Name`, `Tuple`, and dotted `Attribute`
  exception types before lowering static forms into the current VM's exception
  matcher. It also supports dynamic handler type expressions through a matcher
  register, covering `_py_warnings.py`'s
  `except re.PatternError if message or module else ()` shape.
- Added a `compileall.py`-driven parser regression for `with (expr) as target`,
  distinguishing grouped context expressions from parenthesized with-item
  lists.
- Added a `_pydatetime.py`-driven parser regression for blank lines between a
  decorator and the decorated function definition.
- Added a `dataclasses.py`-driven lexer regression for same-quote nested
  f-strings inside replacement expressions, such as
  `f'{f' {decorator}\n' if decorator else ''} ...'`. Full parity still
  requires expanding beyond the official CPython `STDLIB_FILES` smoke set into
  broader recursive `Lib` coverage, exact CPython timing/side-effect behavior
  for exception type expression evaluation, and public-AST round-tripping.

Completed in the AST constructor subclass fields pass:

- Added `cpython_ast_constructor_subclass_fields_first_pass_subset`, migrating
  the next executable CPython `ASTConstructorTests` slice for custom
  `ast.AST` subclasses.
- Custom AST subclasses now use an AST-aware default constructor when no user
  `__init__` is present. It binds positional and keyword values through
  `_fields`, accepts custom `_attributes`/unknown keyword attributes, preserves
  CPython-style missing-field attribute lookup behavior, tolerates malformed
  non-string `_fields` entries without crashing, and materializes implicit
  empty-list defaults for `_field_types` entries such as `list[str]`.
- Type-union expressions now accept `None` as `NoneType`, so class-body
  annotations and `_field_types` shapes like `int | None` execute like CPython.
- Added `cpython_ast_constructor_non_str_kwarg_first_pass_subset`, covering the
  TypeError side of CPython `ASTConstructorTests::test_non_str_kwarg`.
  AST constructor calls now normalize unpacked keyword keys through
  Python-level equality against known AST field names, so a user object equal
  to `"id"` collides with the positional `Name.id` argument like CPython while
  unrelated non-string keys still reject as unexpected constructor keywords.
- Added a minimal runtime `warnings` module surface for
  `warnings.catch_warnings(record=True)`, `warnings.simplefilter()`, and
  `warnings.warn()`, then migrated
  `cpython_ast_constructor_deprecation_warnings_subset`.
- AST constructors now record CPython-style `DeprecationWarning` messages for
  missing required builtin fields such as `FunctionDef.name` and `Name.id`,
  arbitrary custom-subclass keyword attributes outside `_attributes`, missing
  subclass `_field_types` entries, malformed non-string `_fields` entries, and
  unexpected non-string unpacked keyword keys.
- Added the CPython `_field_types` `expr_context` constructor special case:
  `ast.expr_context` is now exposed from the `ast` module, `Load` / `Store` /
  `Del` inherit from it for `isinstance()` / `issubclass()`, and missing custom
  AST subclass fields typed as `ast.expr_context` default to `ast.Load()`.

Completed in the AST constructor exact-method pass:

- Added direct method-level Rust tests for all 11 current CPython
  `ASTConstructorTests` methods:
  `test_FunctionDef`, `test_expr_context`,
  `test_custom_subclass_with_no_fields`, `test_fields_but_no_field_types`,
  `test_fields_and_types`, `test_custom_attributes`,
  `test_fields_and_types_no_default`, `test_incomplete_field_types`,
  `test_malformed_fields_with_bytes`, `test_complete_field_types`, and
  `test_non_str_kwarg`.
- Moved `ASTConstructorTests` from `partial` to `ported` in the strict
  CPython test manifest. The manifest now records 12 ported groups / 254
  methods and 3 partial groups / 76 methods.

Completed in the first AST copy/replace pass:

- Added `cpython_ast_copy_replace_first_pass_subset`, migrating the first
  executable CPython `CopyTests` surface that does not require pickle or
  recursive `copy.deepcopy()`.
- Native public AST nodes now expose `__replace__` and are supported by
  `copy.replace()`. Replacement copies only declared `_fields` and known
  location attributes, applies keyword replacements, drops unknown instance
  attributes, and rejects missing required fields or unexpected keywords with
  CPython-style `TypeError` messages.
- Custom `ast.AST` subclasses now participate in the same shallow replace path
  through their declared `_fields` and `_attributes`, while preserving class
  defaults for attributes that are not instance fields.
- Remaining after this first replace pass was pickle round-tripping, recursive
  `copy.deepcopy()` behavior for parent links, and broader native class
  iteration through `__subclasses__()`.

Completed in the AST native replace iteration pass:

- Added recursive native AST class traversal through `ast.AST.__subclasses__()`
  for MiniPython's exposed public AST class set.
- Exposed class-level `_fields`, `_attributes`, `__match_args__`, `__bases__`,
  `__base__`, and `__replace__` on AST builtin classes, plus AST class
  inheritance checks such as `issubclass(ast.Name, ast.expr)` and
  `isinstance(ast.Name("x"), ast.expr)`.
- Added `cpython_ast_replace_native_class_iteration_first_pass_subset`,
  migrating the core behavior of CPython `CopyTests.test_replace_interface`
  and the broad native loop from `test_replace_native`: every exposed AST class
  is constructible from `dict.fromkeys(_fields)`, rejects positional
  `copy.replace()` / `__replace__()` calls, shallow-copies unchanged fields and
  attributes, and can replace each declared field/location attribute without
  mutating the source node.
- Remaining `CopyTests` parity is now mostly exact CPython native-class
  inventory parity and the real binary pickle protocol beyond the AST snapshot
  payload.

Completed in the AST native abstract hierarchy pass:

- Aligned the native AST class hierarchy with CPython's generated ASDL sum
  classes: `AST -> mod -> Module`, `AST -> stmt -> FunctionDef` / statement
  nodes, `AST -> pattern -> MatchValue` / pattern nodes,
  `AST -> excepthandler -> ExceptHandler`, `AST -> type_ignore -> TypeIgnore`,
  and `AST -> type_param -> TypeVar` / `ParamSpec` / `TypeVarTuple`.
- Added direct `__subclasses__()` and `__bases__` coverage for those abstract
  classes, plus `issubclass()` / `isinstance()` checks against `stmt`, `mod`,
  `pattern`, `type_ignore`, and `type_param`.
- Corrected public type-parameter AST field shapes so `TypeVar._fields` is
  `("name", "bound", "default_value")`, while `TypeVarTuple._fields` and
  `ParamSpec._fields` are `("name", "default_value")`, matching
  `Parser/Python.asdl`.
- Added `cpython_ast_native_abstract_class_hierarchy_subset` as the first
  explicit regression test for CPython's generated native AST abstract classes.
- Added `cpython_ast_base_classes_exact_subset`, directly porting CPython
  `AST_Tests.test_base_classes` for representative `issubclass()` checks
  across concrete nodes, abstract sum nodes, `comprehension`, and operator
  singleton classes.
- Added `cpython_ast_asdl_inventory_exact_subset` to make this precise instead
  of sample-based: the test verifies all 126 generated public AST class names,
  125 direct subclass edges, 198 `_fields` entries, and every ASDL-backed
  `_attributes` tuple exposed through the public `ast` module.
- Added `cpython_ast_asdl_signature_doc_subset`, porting the `__doc__`
  signature checks from CPython `AST_Tests.test_ast_asdl_signature` for
  concrete product types such as `withitem` and `Name`, enum-like sum types
  such as `cmpop`, and the generated multi-line `expr` sum signature.
- Added `cpython_ast_arguments_annotations_subset`, porting the
  `AST_Tests.test_arguments` checks for `ast.arguments.__annotations__` and
  the generated `_field_types` surface. The VM now maps ASDL field types to
  runtime type values such as `list[ast.arg]`, `ast.arg | None`,
  `list[ast.expr]`, `str`, `object`, `list[ast.type_ignore]`, and
  `int | None`.

Completed in the first AST deepcopy pass:

- Added `copy.deepcopy()` to the minimal `copy` module.
- Added recursive AST deep-copy support with memoization, so cyclic custom
  attributes such as `child.parent = node` are copied without recursing forever
  and copied children point back to copied parents.
- Added `cpython_ast_deepcopy_parent_links_first_pass_subset`, migrating the
  parent-link behavior from CPython `CopyTests.test_copy_with_parents`.
- Exposed the abstract AST helper types `ast.boolop`, `ast.operator`,
  `ast.unaryop`, and `ast.cmpop` for the `isinstance()` checks used by that
  CPython test.
- Remaining `CopyTests` parity still needs full binary pickle protocol parity
  and broader native AST class iteration through `__subclasses__()`.

Completed in the first AST pickle round-trip pass:

- Added a minimal `pickle` module exposing `HIGHEST_PROTOCOL`, `dumps()`, and
  `loads()`. This intentionally stores an internal snapshot payload rather than
  claiming CPython's binary pickle byte stream is implemented.
- Added `cpython_ast_pickle_roundtrip_first_pass_subset`, migrating the core
  observable behavior from CPython `CopyTests.test_pickling`: public AST trees
  produced by `compile(source, ..., PyCF_ONLY_AST)` round-trip through all
  supported protocol numbers and compare structurally with attributes.
- Covered snapshot isolation by mutating the original AST after `dumps()` and
  proving `loads()` still returns the pre-mutation tree.
- Remaining pickle work is the real binary pickle format, general object
  serialization, and broader CPython pickle tests outside the AST copy surface.

Completed in the CopyTests method-evidence pass:

- Added direct method-level Rust tests for most current CPython `CopyTests`
  methods: `test_pickling`, `test_copy_with_parents`,
  `test_replace_interface`, `test_replace_native`,
  `test_replace_accept_known_class_fields`,
  `test_replace_accept_known_class_attributes`,
  `test_replace_accept_known_custom_class_attributes`,
  `test_replace_ignore_known_custom_instance_fields`,
  `test_replace_reject_missing_field`,
  `test_replace_accept_missing_field_with_default`,
  `test_replace_reject_known_custom_instance_fields_commits`,
  `test_replace_reject_unknown_instance_fields`, and
  `test_replace_non_str_kwarg`.
- Added native AST node `__reduce__()` support for the CopyTests state checks:
  `node.__reduce__()` returns a constructor placeholder, empty args, and a
  state dictionary containing the AST node fields and location attributes.
- Added `cpython_ast_copy_replace_accept_known_custom_class_fields_first_pass_subset`
  for `test_replace_accept_known_custom_class_fields`. It verifies shallow
  replacement and object-field identity, but adapts CPython's string-field
  `assertIs()` to value equality because MiniPython strings are still stored as
  value objects rather than identity-preserving runtime objects.
- `CopyTests` remains `partial` in the strict manifest until the broader
  string/object identity model can satisfy that CPython string-field identity
  assertion.

Completed in the first AST compare pass:

- Added `cpython_ast_compare_first_pass_subset`, migrating the first public
  `ast.compare()` surface from CPython `AST_Tests`.
- `ast.compare()` now recursively compares public AST nodes by exact node type,
  declared runtime `_fields`, list field contents, exact primitive value types,
  and optionally `_attributes` when `compare_attributes=True`.
- Native AST nodes now read instance-level `_fields` / `_attributes` overrides
  before their generated defaults, which is needed for CPython's mutable AST
  metadata tests.

Completed in the AST compare modes pass:

- Added `cpython_ast_compare_modes_snippets_subset`, migrating CPython
  `AST_Tests.test_compare_modes`.
- The test runs every current CPython `Lib/test/test_ast/snippets.py`
  `exec_tests`, `eval_tests`, and `single_tests` sample through MiniPython's
  public `ast.parse()` and `ast.compare()` APIs.
- Remaining AST compare work is now mostly broader edge parity beyond the
  current CPython compare methods, such as unsupported non-AST argument
  diagnostics and custom subclass corner cases outside the migrated surface.

Completed in the AST node class metadata pass:

- Added `cpython_ast_node_class_metadata_subset`, migrating more public
  `AST_Tests` behavior from `test_field_attr_writable`, `test_classattrs`,
  `test_nodeclasses`, `test_no_fields`, `test_constant_subclasses`, and
  `test_module`.
- User-defined subclasses of native AST classes now inherit the native AST
  constructor field layout when they do not define their own `_fields`, so
  classes such as `class N2(ast.Constant): pass` can be initialized with the
  same `value` / `kind` constructor surface.
- User-defined AST subclasses with their own `__init__` now run that method,
  and `super().__init__(*args, **kwargs)` can initialize the existing subclass
  instance through the native AST constructor path. This aligns
  `class N(ast.Constant)` with CPython's observable `value`, custom attribute,
  `type()`, and `isinstance()` behavior.
- The migrated test also covers writable instance-level `_fields`, missing
  `AttributeError` behavior for absent AST fields/attributes, native
  constructor arity errors, arbitrary deprecated keyword attributes, fieldless
  operator nodes, and hand-built `ast.Module` body identity.

Completed in the AST field-attribute existence pass:

- Added `cpython_ast_field_attr_existence_subset`, porting CPython
  `AST_Tests.test_field_attr_existence`.
- The test walks `ast.__dict__.items()`, filters public AST classes with the
  same shape as CPython's `_is_ast_node()` helper, constructs classes from
  their generated `__annotations__`, and checks that every constructed AST node
  has tuple-valued `_fields`.
- Tightened `isinstance(value, type)` for `Value::Builtin` so builtin
  functions such as `ast.get_docstring` are no longer treated as class objects;
  only real builtin type objects remain `type` instances.
- Added empty `__annotations__` metadata for `ast.NodeVisitor` and
  `ast.NodeTransformer`, matching the class surface needed by the CPython
  field-attribute existence walk.

Completed in the base AST object and missing `_fields` pass:

- Added `cpython_ast_base_object_and_missing_fields_subset`, migrating CPython
  `AST_Tests.test_AST_objects` and the `_fields` deletion crash-regression
  behavior from `AST_Tests.test_AST_fields_NULL_check`.
- Base `ast.AST()` nodes now expose an empty `_fields` tuple, support arbitrary
  instance attributes through `__dict__`, and still reject positional
  construction.
- Added a narrow class-attribute override path for builtin AST classes so
  deleting `ast.AST._fields` affects subsequent `ast.AST()` construction and
  raises `AttributeError` with the CPython-style missing class-attribute
  message. Restoring `ast.AST._fields` returns the class surface to normal.

Completed in the AST parse warning capture pass:

- Added `cpython_ast_filter_syntax_warnings_by_module_subset`, migrating the
  core observable behavior from CPython
  `AST_Tests.test_filter_syntax_warnings_by_module`.
- `warnings.filterwarnings()` now exists on the supported `warnings` module as
  a compatibility no-op for the covered filter-shaping calls.
- `ast.parse()` now runs string and bytes source through the tokenizer warning
  path after successful parsing and emits tokenizer-originated `SyntaxWarning`
  records into `warnings.catch_warnings(record=True)`, preserving line numbers,
  category identity, the default `<unknown>` filename, and explicit filename
  arguments.
- This intentionally covers tokenizer/parser warnings only; codegen/static
  warnings such as assert-tuple, literal-identity, and return-in-finally remain
  part of MiniPython's compile/run warning helpers rather than `ast.parse()`.

Completed in the AST required-`None` validator pass:

- Added `cpython_ast_none_required_fields_subset`, migrating CPython
  `AST_Tests.test_none_checks`.
- The test walks parser-built public AST trees with `ast.walk()`, mutates one
  required field to `None`, and verifies compile-from-AST raises the exact
  required-field `ValueError` for `alias.name`, `arg.arg`,
  `comprehension.target`, `comprehension.iter`, `keyword.value`,
  `match_case.pattern`, and `withitem.context_expr`.
- Public-AST compile validation now treats `None` in required string and
  required child-node fields as a validator error instead of letting the value
  fall through into a generic type error or unsupported-node error.

Completed in the AST parse NUL-byte pass:

- Added `cpython_ast_parse_null_bytes_subset`, migrating CPython
  `AST_Tests.test_null_bytes`.
- Public `ast.parse()` now rejects source strings containing NUL bytes with
  `SyntaxError: source code string cannot contain null bytes`, matching the
  CPython-facing `ast.parse()` message while leaving the lower tokenizer and
  source-encoding `source code cannot contain null bytes` checks intact.

Completed in the AST BinOp/decorator location pass:

- Added `cpython_ast_binop_and_dotted_decorator_locations_subset`, migrating
  CPython `AST_Tests.test_issue18374_binop_col_offset` and
  `AST_Tests.test_issue39579_dotted_name_end_col_offset`.
- Parser-generated public AST locations now cover nested binary-operation
  end positions across explicit line joining and dotted decorator expressions.
- `FunctionDef` and `ClassDef` location annotation now visits
  `decorator_list` before consuming the `def` / `class` header, so decorator
  expression AST nodes receive source locations without changing the function
  or class node's own body-oriented span.

Completed in the AST t-string structure pass:

- Added `cpython_ast_tstring_structure_subset`, migrating CPython
  `AST_Tests.test_tstring`.
- Parser-generated public AST now has direct regression coverage for
  t-string `TemplateStr` nodes, literal `Constant` values, and
  interpolation `Interpolation` nodes.

Completed in the AST repr first-pass migration:

- Added `cpython_ast_repr_first_pass_subset`, migrating the first CPython
  `AST_Tests.test_repr` snapshots for supported module, function, class,
  return, delete, assignment, assignment-target, annotated assignment,
  augmented assignment, for/while/if, with, raise, try, try-star, assert,
  import/from-import/lazy-import, global, expression, pass/break/continue,
  comprehension, async, unpacking, yield/yield-from, decorator, named
  expression, positional-only argument, type-alias, generic class/function, and
  match source shapes.
- Added `cpython_ast_repr_eval_expression_snapshot_subset`, migrating the full
  current CPython `snippets.py::eval_tests` expression snapshot tail from
  `AST_Tests.test_repr`, including constants, boolean/binary/unary operations,
  lambdas, displays, comprehensions, comparisons, calls, selectors, slices,
  conditional expressions, f-strings, and t-strings.
- Added `cpython_ast_repr_large_input_crash_subset`, migrating
  `AST_Tests.test_repr_large_input_crash` so AST repr now propagates
  `ValueError` when an oversized integer constant would be converted to
  decimal text.
- `repr()` on public AST nodes now renders a CPython-style structural AST
  representation with full fields for shallow nodes and `Kind(...)` summaries
  for deeper child nodes.
- Long AST child lists in nested nodes now use CPython-style
  `[first, ..., last]` compression. This leaves `ast.dump()` behavior separate.

Completed in the AST match-case optimization pass:

- Added `cpython_ast_optimization_match_case_folding_subset`, migrating CPython
  `ASTOptimizationTests.test_folding_match_case_allowed_expressions` and
  `ASTOptimizationTests.test_match_case_not_folded_in_unoptimized_ast`.
- Public `ast.parse(..., optimize=1/2)` now folds signed numeric literals and
  real-plus-imaginary literals inside match patterns to `Constant` nodes.
- The folding pass is scoped to pattern literals in `MatchValue`, mapping
  pattern keys, and nested sequence patterns; `optimize=0` still preserves the
  unoptimized `BinOp` shape for `case 1+2j`.

Completed in the AST field/source-segment parity pass:

- Added `cpython_ast_import_alias_slice_fields_subset`, migrating CPython
  `AST_Tests.test_slice`, `AST_Tests.test_from_import`,
  `AST_Tests.test_alias`, `AST_Tests.test_non_interned_future_from_ast`, and
  `AST_Tests.test_compilation_of_ast_nodes_with_default_end_position_values`.
- This locks in public AST slice `None` defaults, relative import
  `module is None`, alias name/asname and source-span fields, future-import
  module mutation before compile-from-AST, and import nodes whose end-position
  attributes are left at constructor defaults.
- Added `cpython_ast_source_segment_tabs_and_mixed_newlines_subset`, migrating
  the tab/form-feed indentation and mixed line-ending source segment cases
  from CPython `EndPositionTests`.
- `ast.get_source_segment(..., padded=True)` now has direct regression
  coverage for method definitions indented with spaces, tab, and form feed;
  source extraction also preserves CPython's `\n`, `\r`, and `\r\n` function
  body segments.

Completed in the AST helper exact-location pass:

- Added `cpython_ast_fix_missing_locations_module_append_subset`, migrating the
  exact CPython `ASTHelpers_Test.test_fix_missing_locations` shape where a
  generated `Expr(Call(...))` is appended to a parsed module before
  `ast.fix_missing_locations()`.
- Added `cpython_ast_fix_missing_locations_exact_subset`, splitting the same
  CPython method into direct `exact_subset` evidence.
- Added `cpython_ast_increment_lineno_exact_subset`, migrating the exact
  CPython `ASTHelpers_Test.test_increment_lineno` snapshots for root-node and
  child-node increments plus the `end_lineno is None` preservation case.
- Added `cpython_ast_increment_lineno_on_module_exact_subset`, splitting
  CPython `ASTHelpers_Test.test_increment_lineno_on_module` into direct
  method-level evidence.
- Added `cpython_ast_copy_location_call_none_attrs_subset`, migrating the
  remaining exact CPython `ASTHelpers_Test.test_copy_location` call-node case
  where `lineno` / `col_offset` are preserved when the source node has
  explicit `None` values and `end_lineno` / `end_col_offset` are cleared.
- Added `cpython_ast_copy_location_exact_subset`, covering the full CPython
  `ASTHelpers_Test.test_copy_location` method as direct method-level evidence.
- These tests tighten the public AST location-helper contract without changing
  parser or VM code; the existing implementation already matched the CPython
  snapshots after correcting the expected `ast.dump(..., show_empty=False)`
  omission of empty `type_ignores`.

Completed in the AST helper source-location exact pass:

- Added `cpython_ast_multiline_docstring_location_exact_subset`, splitting
  CPython
  `ASTHelpers_Test.test_multi_line_docstring_col_offset_and_lineno_issue16806`
  into direct method-level evidence.
- Added `cpython_ast_elif_stmt_start_position_exact_subset`,
  `cpython_ast_elif_stmt_start_position_with_else_exact_subset`, and
  `cpython_ast_starred_expr_end_position_within_call_exact_subset`, splitting
  the corresponding CPython source-location helper methods out of the older
  combined location smoke test.

Completed in the AST helper exact-iteration pass:

- Added `cpython_ast_iter_helpers_exact_subset`, migrating the exact CPython
  `ASTHelpers_Test.test_iter_fields` and
  `ASTHelpers_Test.test_iter_child_nodes` assertions for call-node fields,
  child count, child order, and keyword-node dump output.
- This builds on the older first-pass iterator smoke test and gives these two
  CPython helper methods direct method-level parity evidence.

Completed in the AST helper docstring exact pass:

- Added `cpython_ast_get_docstring_exact_subset`, migrating CPython
  `ASTHelpers_Test.test_get_docstring` for module, class, function, and async
  function docstring extraction, `clean=False`, and unsupported-node
  `TypeError`.
- Added `cpython_ast_get_docstring_none_exact_subset`, migrating CPython
  `ASTHelpers_Test.test_get_docstring_none` across empty modules, module-level
  non-docstring assignments, classes, functions, and async functions.
- This splits the no-docstring cases out from the older broad first-pass
  `ast.get_docstring()` smoke test so the CPython method has direct evidence.

Completed in the AST helper literal-eval exact pass:

- Added `cpython_ast_literal_eval_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_literal_eval` out of the older broad
  `cpython_ast_literal_eval_first_pass_subset`.
- Added `cpython_ast_literal_eval_complex_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_literal_eval_complex` out of the older broad
  `cpython_ast_literal_eval_complex_full_subset`.
- Added `cpython_ast_literal_eval_str_int_limit_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_literal_eval_str_int_limit` out of the older broad
  `cpython_ast_literal_eval_str_int_limit_subset`.
- The direct method-level test covers safe literal containers, bytes, `set()`,
  numeric signs, negative zero rendering, and the CPython `ValueError`
  rejection cases for non-literal expression shapes, plus complex literal
  acceptance/rejection and integer-string digit-limit behavior.

Completed in the AST helper literal-eval diagnostics exact pass:

- Added `cpython_ast_literal_eval_malformed_dict_nodes_exact_subset`,
  `cpython_ast_literal_eval_trailing_ws_exact_subset`,
  `cpython_ast_literal_eval_malformed_lineno_exact_subset`, and
  `cpython_ast_literal_eval_syntax_errors_exact_subset`.
- These tests split CPython `ASTHelpers_Test` methods out of the older broad
  `cpython_ast_literal_eval_diagnostics_first_pass_subset`, preserving the same
  behaviors as direct method-level migration evidence.

Completed in the AST helper recursion exact pass:

- Added `cpython_ast_recursion_direct_exact_subset` and
  `cpython_ast_recursion_indirect_exact_subset`, splitting CPython
  `ASTHelpers_Test.test_recursion_direct` and `test_recursion_indirect` out of
  the older combined recursion smoke test.
- Both tests construct cyclic public AST expression trees and require
  compile-from-AST to raise `RecursionError`.

Completed in the AST snippets eval public-`to_tuple()` pass:

- Added `cpython_ast_snippets_eval_to_tuple_core_expr_subset`, migrating the
  first CPython `Lib/test/test_ast/snippets.py::eval_tests` public AST
  `to_tuple()` snapshots beyond the old `1 + 2` smoke case.
- The new batch covers constants, boolean operators, binary operators, unary
  operators, lambda, dict, and set expressions, and keeps the CPython
  compile-from-public-AST round-trip check for each expression.
- Tightened parser-generated public AST source locations for `UnaryOp` and
  `Lambda` so their spans start at the operator / `lambda` token instead of at
  the operand or body expression.

Completed in the AST snippets eval display/call/slice pass:

- Added `cpython_ast_snippets_eval_to_tuple_display_comp_subset`, migrating the
  next `snippets.py::eval_tests` public AST snapshots for multi-line dict,
  list, tuple, and set displays plus list, set, dict, and generator
  comprehensions with tuple/list targets.
- Added `cpython_ast_snippets_eval_to_tuple_compare_call_slice_subset`,
  covering comparison operators, chained comparisons, call arguments including
  interleaved keyword and `*` / `**` unpacking forms, generator arguments,
  constants, attributes, subscripts, omitted-bound slices, tuple/list displays,
  and conditional expressions.
- Fixed compile-from-public-AST for `ast.Call.args` and class bases containing
  `ast.Starred` by converting them to MiniPython's internal unpack-call
  representation.
- Tightened parser-generated public AST source locations for call keywords
  whose source order is interleaved with starred arguments, `**kwargs`
  keyword-node spans, omitted-bound `Slice` spans, and `IfExp` nodes whose AST
  field order differs from source order.

Completed in the AST snippets eval interpolated-string pass:

- Added `cpython_ast_snippets_eval_to_tuple_interpolated_string_subset`,
  migrating the remaining CPython `Lib/test/test_ast/snippets.py::eval_tests`
  public AST snapshots for f-string `JoinedStr` / `FormattedValue` and
  t-string `TemplateStr` / `Interpolation` expression nodes.
- This covers plain replacement fields, format specs, `!r` conversions,
  literal text around replacement fields, parser-generated source positions,
  nested format-spec `JoinedStr` spans, and compile-from-public-AST round trips.
- Replaced the older f-string-only replacement-expression annotator with a
  shared interpolated-string source-location path that handles f-strings and
  t-strings, including literal `Constant` parts and `Interpolation` nodes.

Completed in the redundant-parentheses trailer source-segment pass:

- Added `cpython_ast_redundant_parentheses_source_segment_subset`, migrating
  CPython `EndPositionTests.test_redundant_parenthesis` and
  `EndPositionTests.test_trailers_with_redundant_parenthesis`.
- Parser-generated public AST source locations now preserve CPython's split:
  redundant parentheses around a plain `BinOp` are excluded from the `BinOp`
  source segment, but redundant parentheses around a primary followed by a
  trailer belong to the outer `Call`, `Subscript`, or `Attribute` node.
- The annotator now lets trailer nodes consume redundant leading parentheses
  and their matching closing parentheses, including when nested under `await`.

Completed in the binop/boolop end-position pass:

- Added `cpython_ast_binop_boolop_end_positions_subset`, migrating the next
  CPython `EndPositionTests.test_binop` and `test_boolop` cases for
  parser-generated end positions and `ast.get_source_segment()`.
- Parser-generated public AST source locations now include parentheses that
  wrap a binary-operation or boolean-operation child when computing the parent
  expression span, while preserving the child node's own inner source segment.

Completed in the multiline slice end-position pass:

- Added `cpython_ast_multiline_slice_end_positions_subset`, migrating the
  multi-line tuple-slice branch of CPython `EndPositionTests.test_slices`.
- Parser-generated public AST source locations now stop optional slice-step
  colon lookahead at commas, so sibling slice items no longer steal each
  other's `:` token when annotating nested tuple slices.

Completed in the string literal end-position pass:

- Added `cpython_ast_string_literal_end_positions_subset`, migrating CPython
  `EndPositionTests.test_multi_line_str` and `test_continued_str`.
- Parser-generated public AST `Constant` source locations now match adjacent
  plain string, literal-only f-string, and bytes token sequences before falling
  back to single-token constants, so implicitly concatenated string literals
  carry the full CPython source span.

Completed in the call/source-segment end-position pass:

- Added `cpython_ast_call_keyword_end_positions_subset`, migrating CPython
  `EndPositionTests.test_call` keyword-value and `**kwargs` value source
  segments.
- Added `cpython_ast_source_segment_multi_tuple_subset`, migrating CPython
  `EndPositionTests.test_source_segment_multi` for a multi-line tuple used as
  the left operand of a binary operation.
- No runtime or annotator change was needed in this pass; the new Rust tests
  pin already-supported CPython behavior at method level.

Completed in the source-segment missing-location pass:

- Added `cpython_ast_source_segment_missing_info_exact_subset`, migrating
  CPython `EndPositionTests.test_source_segment_missing_info` over mixed
  CR/LF/CRLF source endings.
- No runtime change was needed; MiniPython already returns `None` from
  `ast.get_source_segment()` when any required location attribute is missing
  from the public AST node.

Completed in the direct EndPositionTests method pass:

- Added exact method-level tests for CPython `EndPositionTests.test_call_noargs`,
  `test_lambda`, `test_class_kw`, `test_attribute_spaces`, and
  `test_source_segment_padded`.
- Changed parser-generated public AST source locations to use UTF-8 byte
  columns for `col_offset` / `end_col_offset`, while leaving lexer diagnostics
  on character columns. This fixes padded source extraction for non-ASCII
  source text such as the CPython `"ЖЖЖЖЖ"` docstring case.

Completed in the next direct EndPositionTests expansion pass:

- Added exact method-level tests for CPython `EndPositionTests.test_func_def`,
  `test_class_def`, `test_tuples`, `test_displays`,
  `test_source_segment_endings`, `test_source_segment_tabs`, and
  `test_source_segment_newlines`.
- No implementation change was needed in this pass; the new tests split
  already-supported source-location behavior out of broader first-pass tests
  into direct CPython method evidence.

Completed in the f-string/import/control-flow direct EndPositionTests pass:

- Added exact method-level tests for CPython `EndPositionTests.test_suites`,
  `test_fstring`, `test_fstring_multi_line`, `test_import_from_multi_line`,
  `test_comprehensions`, and `test_yield_await`.
- No implementation change was needed in this pass; the new tests split
  existing first-pass source-location coverage into direct CPython method
  evidence.

Completed in the remaining direct EndPositionTests split pass:

- Added exact method-level tests for CPython `EndPositionTests.test_call`,
  `test_multi_line_str`, `test_continued_str`, `test_slices`, `test_binop`,
  `test_boolop`, `test_redundant_parenthesis`,
  `test_trailers_with_redundant_parenthesis`, and
  `test_source_segment_multi`.
- `EndPositionTests` now has direct method-level Rust coverage for all 28
  current CPython methods in the local CPython checkout, so the migration
  manifest moves that group from `partial` to `ported`.
- No implementation change was needed in this pass; the new tests converted
  already-supported first-pass source-location behavior into exact CPython
  method evidence.

Next:

1. Move from grammar-rule inventory to the broader CPython migration surface:
   tighten tokenizer rows that are still partial, expand semantic/runtime
   parity tests for already-parsed syntax, and keep importing CPython test
   cases by feature rather than wholesale-copying the upstream suite.
