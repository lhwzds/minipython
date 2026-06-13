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

- Sandbox stdlib coverage is scoped by the `Sandbox Stdlib Manifest` in
  `tests/cpython_migration.md`. Required modules are `builtins`, `sys`,
  `types`, `collections` / `collections.abc`, `math` / `math.integer`,
  `array`, `copy`, `io.BytesIO`, `operator`, `functools`, `itertools`, and
  `json`. Each required module must keep concrete `cpython_diff` evidence and
  either matching `cpython_subset` or runtime guard evidence, and partial modules
  must keep their supported and excluded surfaces documented in that manifest
  rather than implying full CPython stdlib parity.
- CPython remains the behavior oracle, not an implementation source to copy.
  MiniPython must not wholesale port CPython `Lib/`; public behavior should be
  migrated through local Rust/runtime shims plus direct differential evidence.
- `collections.deque` is currently a documented pure-memory sandbox surface:
  `cpython_collections_deque_public_surface_subset` and
  `cpython_collections_deque_public_surface_diff_subset` cover pure-memory
  construction from iterables, `maxlen` truncation and readonly access,
  iteration, len/bool/repr, direct display/empty-format methods, recursive
  display, public `__doc__` / `__module__` / `__qualname__` metadata,
  `dir()` visibility, generic-alias repr, direct type-level public method
  calls, basic `append` / `appendleft` / `extend` / `extendleft` / `insert` /
  `remove` / `pop` / `popleft` / `count` / `index` / `rotate` / `reverse` /
  `clear` / `copy`, membership, rich comparison between deque instances,
  integer indexing/assignment/deletion, concat/repeat, in-place concat/repeat,
  `.copy()` / `__copy__()` / `copy.copy()`, reverse iteration, concrete type
  identity, and `MutableSequence` registration. Full deque construction/mutation APIs
  remain outside the default sandbox surface beyond this documented pure-memory
  subset; slicing, pickle/eval identity matrices, performance/lifetime internals,
  thread-safety stress, and unported ABC edge matrices also remain outside
  `collections` / `collections.abc` until separately promoted with direct
  public-behavior evidence.
- Bytes literal runtime behavior has direct CPython output parity evidence in
  `cpython_bytes_literal_runtime_diff_subset`; local subset-only diagnostics for
  mixed bytes/non-bytes literal rejection remain documented in the `STRING`
  coverage row and migration notes.
- Bytes/bytearray old-style `%` formatting and reflected modulo behavior also
  have direct CPython output parity evidence in
  `cpython_bytes_percent_format_and_rmod_diff_subset`; the broader subset tests
  keep MiniPython-specific implementation-safety and diagnostic-shape coverage.
- Bytes/bytearray `%b` / `%s` `__bytes__` dispatch and direct `__mod__`
  descriptor behavior have direct CPython output parity evidence in
  `cpython_bytes_percent_dunder_bytes_diff_subset`. Bytearray format-string
  re-entrancy protection remains subset/default-oracle boundary evidence
  because the default CPython oracle accepts the mutation while the migrated
  subset pins the current public safety behavior.
- Bytes/bytearray `hex()` separator, grouping, overflow, and nibble/length
  boundary output now has direct CPython output parity evidence in
  `cpython_bytes_hex_separator_diff_subset`; exact unbound descriptor
  diagnostics remain in `cpython_bytes_hex_descriptor_error_messages_subset`.
- Bytes/bytearray `fromhex()` over stable string input now has direct CPython
  output parity evidence in `cpython_bytes_fromhex_string_diff_subset`; newer
  bytes-like `fromhex()` inputs have gated direct CPython evidence in
  `cpython_bytes_fromhex_bytes_like_diff_subset`.
- Compatibility/test-support modules exposed by `src/stdlib.rs::create_module()`
  are tracked separately by the `Runtime Compatibility Module Registry` in
  `tests/cpython_migration.md`. They do not expand the default sandbox product
  scope: `sandbox_policy_denies_stdlib_imports`,
  `sandbox_policy_denies_required_sandbox_stdlib_surface`,
  `sandbox_policy_allows_required_sandbox_stdlib_surface`,
  `sandbox_policy_required_stdlib_allow_list_excludes_compatibility_shims`,
  `sandbox_policy_requires_explicit_allow_for_extra_stdlib_shims`, and
  `stdlib_create_module_registry_is_classified_by_scope` keep the runtime
  policy, registry classification, and documentation aligned.
- Direct sandbox stdlib `cpython_diff` evidence names are also mirrored here:
  `cpython_globals_locals_builtin_diff_subset`,
  `cpython_vars_dir_builtin_diff_subset`,
  `cpython_eval_builtin_diff_subset`,
  `cpython_exec_builtin_diff_subset`,
  `cpython_eval_exec_builtins_mapping_diff_subset`,
  `cpython_compile_builtin_code_object_diff_subset`,
  `cpython_isinstance_builtin_diff_subset`,
  `cpython_issubclass_builtin_diff_subset`,
  `cpython_attribute_introspection_builtins_diff_subset`,
  `cpython_ascii_builtin_diff_subset`,
  `cpython_chr_ord_builtin_diff_subset`,
  `cpython_builtin_cmp_absent_diff_subset`,
  `cpython_builtin_none_ne_direct_diff_subset`,
  `cpython_builtin_exception_hierarchy_diff_subset`,
  `cpython_runtime_exception_capture_diff_subset`,
  `cpython_base_exception_args_diff_subset`,
  `cpython_base_exception_with_traceback_diff_subset`,
  `cpython_system_exit_oserror_attributes_diff_subset`,
  `cpython_syntax_error_attributes_diff_subset`,
  `cpython_unicode_error_attributes_diff_subset`,
  `cpython_object_repr_str_direct_diff_subset`,
  `cpython_str_builtin_custom_dunder_diff_subset`,
  `cpython_builtin_bool_notimplemented_diff_subset`,
  `cpython_builtin_singleton_construction_and_attributes_diff_subset`,
  `cpython_all_any_builtin_diff_subset`,
  `cpython_len_builtin_diff_subset`,
  `cpython_min_max_sum_builtin_diff_subset`,
  `cpython_iter_next_builtin_diff_subset`,
  `cpython_aiter_anext_builtin_diff_subset`,
  `cpython_stop_iteration_value_diff_subset`,
  `cpython_map_filter_builtin_diff_subset`,
  `cpython_map_strict_builtin_diff_subset`,
  `cpython_enumerate_zip_sorted_builtin_diff_subset`,
  `cpython_builtin_sorted_exact_diff_subset`,
  `cpython_zip_strict_builtin_diff_subset`,
  `cpython_divmod_builtin_diff_subset`,
  `cpython_pow_builtin_diff_subset`,
  `cpython_abs_builtin_diff_subset`,
  `cpython_builtin_print_keyword_diff_subset`,
  `cpython_round_builtin_diff_subset`,
  `cpython_format_builtin_and_custom_dunder_format_diff_subset`,
  `cpython_hash_id_builtins_diff_subset`,
  `cpython_builtin_breakpoint_custom_hook_diff_subset`,
  `cpython_builtin_breakpoint_passthru_error_diff_subset`,
  `cpython_builtin_negation_sys_maxsize_diff_subset`,
  `cpython_int_max_str_digits_runtime_diff_subset`,
  `cpython_float_hash_and_sys_info_diff_subset`,
  `cpython_types_frame_locals_proxy_type_diff_subset`,
  `cpython_types_names_public_surface_diff_subset`,
  `cpython_types_singleton_type_aliases_diff_subset`,
  `cpython_types_module_type_diff_subset`,
  `cpython_types_generic_alias_union_type_diff_subset`,
  `cpython_types_union_public_operator_and_classinfo_diff_subset`,
  `cpython_types_union_forward_ref_diff_subset`,
  `cpython_types_union_forward_get_type_hints_diff_subset`,
  `cpython_types_union_typevar_parameter_diff_subset`,
  `cpython_types_union_parameter_substitution_diff_subset`,
  `cpython_types_union_copy_pickle_diff_subset`,
  `cpython_types_union_bad_module_guard_diff_subset`,
  `cpython_types_union_genericalias_subclass_bad_eq_diff_subset`,
  `cpython_types_union_bad_classinfo_checks_diff_subset`,
  `cpython_types_union_unhashable_metaclass_diff_subset`,
  `cpython_types_union_dynamic_hashability_diff_subset`,
  `cpython_types_union_newtype_diff_subset`,
  `cpython_types_union_io_diff_subset`,
  `cpython_types_union_typed_dict_diff_subset`,
  `cpython_types_union_protocol_diff_subset`,
  `cpython_types_union_special_form_diff_subset`,
  `cpython_types_union_literal_diff_subset`,
  `cpython_types_class_creation_new_class_meta_helper_diff_subset`,
  `cpython_types_class_creation_one_argument_type_diff_subset`,
  `cpython_types_class_creation_get_original_bases_diff_subset`,
  `cpython_types_class_creation_metaclass_new_error_diff_subset`,
  `cpython_types_class_creation_subclass_inherited_slot_update_diff_subset`,
  `cpython_types_class_creation_new_class_resolve_bases_diff_subset`,
  `cpython_types_class_creation_mro_entries_core_diff_subset`,
  `cpython_types_class_creation_mro_entries_multiple_diff_subset`,
  `cpython_types_class_creation_prepare_resolve_bases_diff_subset`,
  `cpython_types_class_creation_prepare_and_metaclass_callable_diff_subset`,
  `cpython_types_class_creation_metaclass_override_function_diff_subset`,
  `cpython_types_class_creation_non_type_metaclass_derivation_diff_subset`,
  `cpython_types_class_creation_metaclass_derivation_diff_subset`,
  `cpython_types_coroutine_public_diff_subset`,
  `cpython_types_coroutine_async_def_diff_subset`,
  `cpython_types_coroutine_generator_wrapper_diff_subset`,
  `cpython_types_coroutine_generator_frame_diff_subset`,
  `cpython_types_coroutine_generator_yieldfrom_diff_subset`,
  `cpython_types_coroutine_duck_generator_wrapper_diff_subset`,
  `cpython_types_coroutine_duck_generator_await_diff_subset`,
  `cpython_types_coroutine_duck_generator_proxy_diff_subset`,
  `cpython_types_function_type_diff_subset`,
  `cpython_types_code_traceback_type_aliases_diff_subset`,
  `cpython_types_frame_type_alias_diff_subset`,
  `cpython_types_runtime_type_aliases_diff_subset`,
  `cpython_types_float_constructor_edges_diff_subset`,
  `cpython_types_float_to_string_diff_subset`,
  `cpython_types_normal_integers_diff_subset`,
  `cpython_types_format_spec_errors_diff_subset`,
  `cpython_types_mappingproxy_exact_dict_diff_subset`,
  `cpython_types_mappingproxy_method_surface_diff_subset`,
  `cpython_types_mappingproxy_union_diff_subset`,
  `cpython_types_mappingproxy_hash_diff_subset`,
  `cpython_types_mappingproxy_contains_diff_subset`,
  `cpython_types_mappingproxy_views_diff_subset`,
  `cpython_types_mappingproxy_missing_diff_subset`,
  `cpython_types_mappingproxy_len_diff_subset`,
  `cpython_types_mappingproxy_iterators_diff_subset`,
  `cpython_types_mappingproxy_reversed_diff_subset`,
  `cpython_types_mappingproxy_copy_diff_subset`,
  `cpython_types_mappingproxy_richcompare_diff_subset`,
  `cpython_types_mappingproxy_custom_mapping_diff_subset`,
  `cpython_types_mappingproxy_chainmap_diff_subset`,
  `cpython_types_simple_namespace_basic_diff_subset`,
  `cpython_types_simple_namespace_recursive_diff_subset`,
  `cpython_types_simple_namespace_new_and_invalid_replace_diff_subset`,
  `cpython_types_simple_namespace_remaining_public_diff_subset`,
  `cpython_types_simple_namespace_state_order_diff_subset`,
  `cpython_types_simple_namespace_fake_comparison_diff_subset`,
  `cpython_types_method_descriptor_types_diff_subset`,
  `cpython_types_slot_and_method_wrapper_types_diff_subset`,
  `cpython_types_int_format_diff_subset`,
  `cpython_types_float_format_diff_subset`,
  `cpython_collections_counter_basics_diff_subset`,
  `cpython_collections_counter_public_diff_subset`,
  `cpython_collections_counter_conversions_diff_subset`,
  `cpython_collections_counter_init_update_diff_subset`,
  `cpython_collections_counter_comparison_diff_subset`,
  `cpython_collections_counter_fromkeys_diff_subset`,
  `cpython_collections_counter_most_common_diff_subset`,
  `cpython_collections_counter_mapping_mutation_diff_subset`,
  `cpython_collections_counter_repr_nonsortable_diff_subset`,
  `cpython_collections_counter_subtract_unary_diff_subset`,
  `cpython_collections_counter_copy_subclass_diff_subset`,
  `cpython_collections_counter_copying_diff_subset`,
  `cpython_collections_counter_order_preservation_diff_subset`,
  `cpython_collections_counter_update_reentrant_add_clears_counter_diff_subset`,
  `cpython_collections_counter_helper_function_diff_subset`,
  `cpython_collections_counter_multiset_operations_diff_subset`,
  `cpython_collections_counter_multiset_operations_matrix_diff_subset`,
  `cpython_collections_counter_multiset_operations_equivalent_to_set_operations_diff_subset`,
  `cpython_collections_counter_symmetric_difference_diff_subset`,
  `cpython_collections_counter_inplace_operations_diff_subset`,
  `cpython_collections_counter_inplace_operations_matrix_diff_subset`,
  `cpython_collections_chainmap_public_methods_diff_subset`,
  `cpython_collections_chainmap_copy_sharing_diff_subset`,
  `cpython_collections_namedtuple_factory_instance_diff_subset`,
  `cpython_collections_namedtuple_public_diff_subset`,
  `cpython_collections_namedtuple_defaults_rename_readonly_diff_subset`,
  `cpython_collections_namedtuple_repr_diff_subset`,
  `cpython_collections_namedtuple_name_conflicts_diff_subset`,
  `cpython_collections_namedtuple_subclass_issue_24931_diff_subset`,
  `cpython_collections_namedtuple_match_args_diff_subset`,
  `cpython_collections_namedtuple_large_size_diff_subset`,
  `cpython_collections_namedtuple_field_doc_diff_subset`,
  `cpython_collections_namedtuple_copy_keyword_generic_alias_diff_subset`,
  `cpython_collections_namedtuple_new_builtins_globals_diff_subset`,
  `cpython_collections_namedtuple_new_builtins_issue_43102_diff_subset`,
  `cpython_collections_userdict_userlist_public_diff_subset`,
  `cpython_collections_userdict_public_methods_diff_subset`,
  `cpython_collections_userlist_public_methods_diff_subset`,
  `cpython_collections_userlist_namedtuple_sequence_order_diff_subset`,
  `cpython_collections_userstring_protocol_and_userdict_missing_diff_subset`,
  `cpython_collections_deque_public_surface_diff_subset`,
  `cpython_collections_chainmap_missing_and_first_map_mutation_diff_subset`,
  `cpython_collections_chainmap_iter_does_not_call_getitem_diff_subset`,
  `cpython_collections_chainmap_new_child_custom_mapping_diff_subset`,
  `cpython_collections_chainmap_order_preservation_diff_subset`,
  `cpython_collections_chainmap_union_operators_diff_subset`,
  `cpython_collections_abc_core_runtime_diff_subset`,
  `cpython_collections_abc_iterable_iterator_diff_subset`,
  `cpython_collections_abc_iterable_sample_matrix_diff_subset`,
  `cpython_collections_abc_iterator_sample_matrix_diff_subset`,
  `cpython_collections_abc_sequence_diff_subset`,
  `cpython_collections_abc_sequence_mixins_diff_subset`,
  `cpython_collections_abc_mapping_diff_subset`,
  `cpython_collections_abc_mapping_view_diff_subset`,
  `cpython_collections_abc_mutable_sequence_diff_subset`,
  `cpython_collections_abc_mapping_mixins_diff_subset`,
  `cpython_collections_abc_mapping_mixin_views_diff_subset`,
  `cpython_collections_abc_userdict_view_snapshot_diff_subset`,
  `cpython_collections_abc_set_mutable_set_mixins_diff_subset`,
  `cpython_collections_abc_set_from_iterable_operator_diff_subset`,
  `cpython_collections_abc_set_real_set_interoperability_diff_subset`,
  `cpython_collections_abc_set_hash_matches_frozenset_diff_subset`,
  `cpython_collections_abc_issue26915_identity_first_object_diff_subset`,
  `cpython_collections_abc_set_noncomparable_comparison_diff_subset`,
  `cpython_collections_abc_reversible_diff_subset`,
  `cpython_collections_abc_reversible_direct_subclass_diff_subset`,
  `cpython_collections_abc_collection_direct_subclass_diff_subset`,
  `cpython_collections_abc_async_runtime_diff_subset`,
  `cpython_collections_abc_async_iterator_mixin_diff_subset`,
  `cpython_collections_abc_async_generator_core_mixin_diff_subset`,
  `cpython_collections_abc_async_generator_throw_close_mixin_diff_subset`,
  `cpython_collections_abc_generator_mixin_diff_subset`,
  `cpython_collections_abc_generator_sample_matrix_diff_subset`,
  `cpython_collections_abc_generator_runtime_diff_subset`,
  `cpython_collections_abc_types_coroutine_diff_subset`,
  `cpython_collections_abc_coroutine_mixin_diff_subset`,
  `cpython_collections_abc_abstract_methods_diff_subset`,
  `cpython_collections_abc_validate_isinstance_diff_subset`,
  `cpython_collections_abc_direct_subclassing_diff_subset`,
  `cpython_collections_abc_hashable_direct_subclass_diff_subset`,
  `cpython_collections_abc_registration_diff_subset`,
  `cpython_collections_abc_bytestring_buffer_diff_subset`,
  `cpython_collections_abc_bytestring_deprecation_warnings_diff_subset`,
  `cpython_collections_abc_composite_abstract_methods_diff_subset`,
  `cpython_math_core_diff_subset`,
  `cpython_math_constants_and_classification_diff_subset`,
  `cpython_math_isclose_diff_subset`,
  `cpython_math_hypot_dist_diff_subset`,
  `cpython_math_gcd_diff_subset`,
  `cpython_math_lcm_diff_subset`,
  `cpython_math_prod_diff_subset`,
  `cpython_math_integer_diff_subset`,
  `cpython_math_sqrt_diff_subset`,
  `cpython_math_fabs_diff_subset`,
  `cpython_math_copysign_diff_subset`,
  `cpython_math_signbit_diff_subset`,
  `cpython_math_trunc_diff_subset`,
  `cpython_math_ceil_diff_subset`,
  `cpython_math_floor_diff_subset`,
  `cpython_math_degrees_radians_diff_subset`,
  `cpython_math_cbrt_diff_subset`,
  `cpython_math_erf_erfc_diff_subset`,
  `cpython_math_gamma_lgamma_diff_subset`,
  `cpython_math_fma_diff_subset`,
  `cpython_math_fmax_fmin_diff_subset`,
  `cpython_math_exp_exp2_diff_subset`,
  `cpython_math_expm1_diff_subset`,
  `cpython_math_log_family_diff_subset`,
  `cpython_math_trig_diff_subset`,
  `cpython_math_hyperbolic_diff_subset`,
  `cpython_math_fmod_remainder_diff_subset`,
  `cpython_math_frexp_ldexp_modf_diff_subset`,
  `cpython_math_fsum_diff_subset`,
  `cpython_math_sumprod_diff_subset`,
  `cpython_math_nextafter_ulp_diff_subset`,
  `cpython_math_pow_diff_subset`,
  `cpython_array_module_and_constructor_public_surface_diff_subset`,
  `cpython_array_subclass_public_construction_diff_subset`,
  `cpython_array_one_byte_public_sequence_diff_subset`,
  `cpython_array_short_public_sequence_and_mutation_diff_subset`,
  `cpython_array_int_public_sequence_and_mutation_diff_subset`,
  `cpython_array_long_long_public_sequence_and_mutation_diff_subset`,
  `cpython_array_native_long_public_sequence_and_mutation_diff_subset`,
  `cpython_array_float_public_sequence_and_mutation_diff_subset`,
  `cpython_array_unicode_public_sequence_and_mutation_diff_subset`,
  `cpython_array_one_byte_public_mutation_methods_diff_subset`,
  `cpython_array_one_byte_public_clear_diff_subset`,
  `cpython_array_one_byte_public_subscript_mutation_diff_subset`,
  `cpython_array_one_byte_public_copy_byteswap_compare_diff_subset`,
  `cpython_array_one_byte_public_concat_repeat_diff_subset`,
  `cpython_array_one_byte_public_buffer_info_diff_subset`,
  `cpython_array_one_byte_public_unicode_method_rejection_diff_subset`,
  `cpython_array_one_byte_public_file_methods_diff_subset`,
  `cpython_copy_public_diff_subset`,
  `cpython_io_bytesio_public_diff_subset`,
  `cpython_memoryview_bytesio_readinto_diff_subset`,
  `cpython_operator_public_helpers_diff_subset`,
  `cpython_operator_length_hint_diff_subset`,
  `cpython_operator_comparison_predicate_diff_subset`,
  `cpython_operator_is_none_predicates_diff_subset`,
  `cpython_operator_arithmetic_bitwise_diff_subset`,
  `cpython_operator_sequence_member_diff_subset`,
  `cpython_operator_callable_helper_diff_subset`,
  `cpython_operator_call_helper_diff_subset`,
  `cpython_operator_inplace_helper_diff_subset`,
  `cpython_operator_module_metadata_diff_subset`,
  `cpython_operator_helper_instance_module_metadata_diff_subset`,
  `cpython_operator_signature_helper_diff_subset`,
  `cpython_operator_helper_repr_diff_subset`,
  `cpython_functools_public_helpers_diff_subset`,
  `cpython_functools_partial_diff_subset`,
  `cpython_functools_partial_instance_module_metadata_diff_subset`,
  `cpython_functools_reduce_diff_subset`,
  `cpython_functools_cmp_to_key_diff_subset`,
  `cpython_functools_update_wrapper_wraps_diff_subset`,
  `cpython_functools_total_ordering_diff_subset`,
  `cpython_functools_partialmethod_diff_subset`,
  `cpython_functools_cached_property_diff_subset`,
  `cpython_functools_cached_property_module_metadata_diff_subset`,
  `cpython_functools_cache_diff_subset`,
  `cpython_functools_cache_wrapper_module_metadata_diff_subset`,
  `cpython_functools_singledispatch_diff_subset`,
  `cpython_functools_singledispatchmethod_diff_subset`,
  `cpython_itertools_core_diff_subset`,
  `cpython_itertools_core_iterator_diff_subset`,
  `cpython_itertools_keyword_error_diff_subset`,
  `cpython_itertools_pairwise_diff_subset`,
  `cpython_itertools_product_diff_subset`,
  `cpython_itertools_combinations_diff_subset`,
  `cpython_itertools_combinations_with_replacement_diff_subset`,
  `cpython_itertools_permutations_diff_subset`,
  `cpython_itertools_tee_diff_subset`,
  `cpython_itertools_batched_diff_subset`,
  `cpython_itertools_groupby_diff_subset`,
  `cpython_itertools_repr_diff_subset`,
  `cpython_json_loads_dumps_diff_subset`,
  `cpython_json_loads_dumps_basic_diff_subset`,
  `cpython_json_keyword_argument_binding_diff_subset`,
  `cpython_json_loads_escape_and_duplicate_key_diff_subset`,
  `cpython_json_loads_unicode_escape_roundtrip_diff_subset`,
  `cpython_json_loads_strict_diff_subset`,
  `cpython_json_option_truthiness_diff_subset`,
  `cpython_json_dumps_string_escape_diff_subset`,
  `cpython_json_dumps_key_coercion_diff_subset`,
  `cpython_json_dumps_sequence_subclass_iter_diff_subset`,
  `cpython_json_dumps_allow_nan_diff_subset`,
  `cpython_json_dumps_check_circular_diff_subset`,
  `cpython_json_dumps_ensure_ascii_diff_subset`,
  `cpython_json_dumps_indent_diff_subset`,
  `cpython_json_dumps_skipkeys_diff_subset`,
  `cpython_json_dumps_sort_keys_diff_subset`,
  `cpython_json_dumps_separators_diff_subset`,
  `cpython_json_dumps_default_hook_diff_subset`,
  `cpython_json_dumps_float_spelling_diff_subset`,
  `cpython_json_loads_number_and_whitespace_diff_subset`,
  `cpython_json_loads_int_digit_limit_diff_subset`,
  `cpython_json_loads_top_level_scalar_and_empty_container_diff_subset`,
  `cpython_json_loads_nonfinite_constants_diff_subset`,
  `cpython_json_loads_parse_hooks_diff_subset`,
  `cpython_json_loads_object_hook_diff_subset`,
  `cpython_json_loads_object_pairs_hook_diff_subset`,
  `cpython_json_loads_dumps_error_boundary_diff_subset`, and
  `cpython_json_loads_string_error_boundary_diff_subset`.
- `NUMBER` also includes CPython `test_compile.py::test_literals_with_leading_zeroes`
  coverage for invalid leading-zero integer/prefixed forms and valid
  leading-zero float, exponent, and imaginary literals.
- `NUMBER` also includes CPython `BuiltinTest::test_round_large`, covering
  integral float `round()` stability across the `5e15 +/- n` boundary.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_from_number` through
  `cpython_float_from_number_subset`, covering exact float and float-subclass
  `from_number()` construction, real-number protocol conversion, NaN behavior,
  non-real rejection, and huge `__index__` overflow.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_float`, `::test_noargs`,
  `::test_error_message`, and the locale-independent assertions from
  `::test_float_with_comma` through `cpython_float_constructor_core_subset`,
  covering no-argument `float()`, numeric/string construction, Unicode
  decimal digit normalization, Unicode whitespace trimming, long string and
  bytes inputs, comma rejection, alternate exponent rejection, embedded-NUL and
  non-UTF-8 byte diagnostics, and non-string input `TypeError` wording.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_floatconversion` through
  `cpython_float_conversion_protocol_subset`, covering custom `__float__`,
  float-subclass and string-subclass conversions, strict-float-subclass
  `__float__` result normalization, and non-float result rejection.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_non_numeric_input_types` and
  `::test_float_memoryview` through
  `cpython_float_bytes_like_input_types_subset`, covering `float()` parsing for
  str/bytes/bytearray subclasses, memoryview and sliced memoryview inputs,
  `array('B')` bytes-like inputs, original-repr `ValueError` messages, and
  unsupported-input `TypeError` wording.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_hash` and `::test_hash_nan` through
  `cpython_float_hash_and_sys_info_subset`, covering public `sys.float_info`
  and `sys.hash_info` attributes used by the float tests, integer-valued float
  hash equality with `int`, the `-1` hash sentinel rule, inf hash constants,
  NaN identity hashing, and float-subclass NaN hash inheritance.
- `NUMBER` also includes `cpython_int_max_str_digits_runtime_subset`, covering
  `sys.set_int_max_str_digits()` / `sys.get_int_max_str_digits()` enforcement
  for decimal integer parsing and rendering. Direct CPython output parity is in
  capability-gated `cpython_int_max_str_digits_runtime_diff_subset` for oracles
  that expose those `sys` digit-limit APIs.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_issue_gh143006` through
  `cpython_float_int_comparison_boundaries_subset`, covering float/int-subclass
  comparisons without invoking user `__neg__` and exact float/int equality and
  ordering across arbitrary-precision integer boundaries, including NaN
  ordered-comparison false results against integers.
- `NUMBER` also includes CPython
  `test_float.py::FormatFunctionsTestCase::test_getformat` through
  `cpython_float_getformat_subset`, covering public `float.__getformat__()`
  for `double` and `float`, invalid-format `ValueError`, non-string
  `TypeError`, class/instance access, and float-subclass inheritance.
- `NUMBER` also includes CPython `test_float.py::FormatTestCase::test_format`
  and `::test_issue5864` through
  `cpython_float_default_precision_format_subset`, covering empty
  presentation-type float formatting with explicit precision, alternate/sign
  handling for that path, inf/nan specials, zero padding, and preserving
  str-like behavior when no precision is specified.
- `NUMBER` also includes CPython `test_float.py::FormatTestCase::test_format`
  through `cpython_float_fractional_grouping_format_subset`, covering
  precision-side float grouping for fractional digits, mixed integer and
  fractional grouping, width/alignment, sign, zero padding, scientific
  notation, and invalid mixed/duplicate grouping combinations.
- `NUMBER` also includes CPython
  `test_float.py::FormatTestCase::test_format_testfile` through
  `cpython_float_format_testfile_subset` and
  `cpython_float_format_testfile_full_subset`, covering the complete local
  CPython `mathdata/formatfloat_testcases.txt` dataset for old-style `%` and
  `format()` float formatting across `f`, `e`, `g`, and `%r`, including
  alternate `#` decimal-point preservation for precision-zero fixed,
  scientific, and general formats.
- `NUMBER` also includes CPython `test_float.py::FormatTestCase::test_issue35560`
  through `cpython_float_zero_width_format_subset`, covering zero-width float
  formatting for empty, fixed, exponent, and general presentation types with
  positive and negative values, including explicit precision and width-one
  variants that must not add padding when the rendered body is already wider.
- `NUMBER` also includes CPython `test_float.py::ReprTestCase::test_repr`
  through `cpython_float_repr_roundtrip_subset` and
  `cpython_float_repr_roundtrip_full_subset`, covering the public
  `repr(float)` to `eval()` round-trip invariant over the complete local
  CPython `mathdata/floating_points.txt` dataset, including signed zero, large
  positive exponents, subnormal-scale decimals, and long decimal spellings.
- `NUMBER` also includes CPython `test_float.py::ReprTestCase::test_short_repr`
  through `cpython_float_short_repr_subset`, covering public
  `sys.float_repr_style == "short"`, finite float `repr()` / `str()` equality,
  exact short-repr spelling for CPython's regression strings, and
  `eval(repr(x))` round-tripping for those finite values.
- `NUMBER` also includes CPython `test_float.py::RoundTestCase` through
  `cpython_float_round_specials_subset`, covering public `round(float,
  ndigits)` behavior for inf/nan, invalid `ndigits` `TypeError` messages,
  the complete CPython `test_large_n` and `test_small_n` grids for extreme
  positive and negative decimal exponents, signed zero preservation, overflow
  diagnostics, huge integer `ndigits`, `None` `ndigits`, and historical
  round-half-even edge cases.
- `NUMBER` also includes CPython
  `test_float.py::RoundTestCase::test_round_with_none_arg_direct_call` through
  `cpython_float_round_dunder_none_subset`, covering direct
  `float.__round__` calls, bound float and float-subclass `__round__`,
  `None` as the direct `ndigits` argument, descriptor receiver checks, keyword
  rejection, and `dir()` visibility.
- `NUMBER` also includes CPython
  `test_float.py::RoundTestCase::test_matches_float_format` through
  `cpython_float_round_matches_format_subset`, covering the public consistency
  relationship between `round(x, n)` and `float(format(x, ".nf"))` across
  6000 comparison points from the CPython thousandths grid, half-cent grid,
  and a 500-value deterministic pseudo-random replacement sweep.
- `NUMBER` also includes CPython
  `test_float.py::RoundTestCase::test_format_specials` through
  `cpython_float_format_specials_subset`, covering `%` and `format()` spelling
  for inf/nan across `e`, `f`, and `g` styles, precision and alternate forms,
  and explicit `+` / space sign handling for special float values.
- `NUMBER` also includes CPython `test_float.py::InfNanTest` through
  `cpython_float_inf_nan_string_subset`, covering decimal `float()` string
  parsing for inf/infinity/nan spellings, invalid near-miss diagnostics,
  `repr()` / `str()` display for arithmetic inf/nan results, and
  `math.copysign()` sign preservation for infinity and signed NaN inputs.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_keywords_in_subclass` through
  `cpython_float_keywords_in_subclass_subset`, covering float subclass
  construction, keyword forwarding through user `__init__` / `__new__`,
  inherited `float.__new__` lookup via `super()`, and keyword rejection.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_float_containment` through
  `cpython_float_containment_subset`, covering list/tuple/set/dict containment
  and self-equality for finite floats, infinities, and identical NaN objects
  while preserving distinct-NaN non-matches.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_float_floor` and `::test_float_ceil`
  through `cpython_float_floor_ceil_subset`, covering direct and unbound
  `float.__floor__()` / `float.__ceil__()` calls, float-subclass receivers,
  large integer results, NaN/inf errors, and argument TypeErrors.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_float_mod` through
  `cpython_float_mod_signed_zero_subset`, covering `%` and `operator.mod()`
  finite-float modulo behavior where zero remainders preserve the divisor's
  sign.
- `NUMBER` also includes CPython
  `test_float.py::GeneralFloatCases::test_float_pow` through
  `cpython_float_pow_special_cases_subset`, covering the active C99 F.9.4.4
  public matrix for NaN/inf identities, signed-zero results, zero-base
  finite-negative-exponent `ZeroDivisionError`, zero-base negative-infinity
  results, negative-infinity fractional powers, negative-real non-integral
  exponent complex results, large `+/-1` exponents, and underflow sign rows
  across `**`, `pow()`, and `operator.pow()`.
- `NUMBER` also includes CPython
  `test_float.py::HexFloatTestCase` coverage through
  `cpython_float_hex_fromhex_first_pass_subset`,
  `cpython_float_fromhex_accepted_variants_subset`,
  `cpython_float_fromhex_overflow_zero_underflow_subset`,
  `cpython_float_fromhex_rounding_boundaries_subset`,
  `cpython_float_fromhex_bpo44954_regression_subset`,
  `cpython_float_hex_fromhex_invalid_inputs_subset`,
  `cpython_float_hex_fromhex_ends_whitespace_subset`, and
  `cpython_float_hex_fromhex_roundtrip_matrix_subset` plus
  `cpython_float_hex_fromhex_subclass_subset`, covering `float.hex()`,
  `float.fromhex()`, IEEE-754 signed zero, infinity, NaN parsing semantics,
  accepted spelling variants, equivalent point-shifted hexadecimal spellings
  for pi, finite-overflow, round-to-maximum-finite, zero-sign preservation,
  subnormal and boundary rounding, round-half-even behavior near zero, `MIN`,
  and 1.0, the bpo-44954 subnormal rounding regression, invalid-input
  `ValueError`, finite overflow `OverflowError`, the complete deterministic
  CPython `test_from_hex` input matrix guarded by
  `cpython_test_manifest_float_fromhex_matrix_inputs_have_runtime_evidence`,
  the complete CPython invalid-input family, `MIN` /
  `TINY` / `EPS` / `MAX` endpoint equivalence against `math.ldexp()`, the
  CPython leading/trailing whitespace matrix,
  a 10,000-row deterministic `float.fromhex(value.hex())` round-trip sweep
  across exponent, mantissa, and sign ranges with CPython-style overflow
  skipping, and `float.fromhex()` subclass construction through
  user-defined `__new__` / `__init__`.
- The runtime stdlib surface includes `cpython_math_core_diff_subset`,
  `cpython_math_core_subset`,
  `cpython_math_constants_and_classification_diff_subset` gated for newer
  CPython oracles, and `cpython_math_constants_and_classification_subset`,
  covering CPython public function `__qualname__` / `__doc__` metadata,
  `pi` / `e` / `tau` / `inf` / `nan` constants plus
  `math.isfinite()`, `math.isnormal()`, `math.issubnormal()`, `math.isnan()`,
  and `math.isinf()` classification for finite normal/subnormal values, signed
  zero, infinities, NaNs, argument errors, and huge integer overflow.
- The bundled `collections` module includes
  `cpython_collections_counter_basics_diff_subset` /
  `cpython_collections_counter_basics_subset` and
  `cpython_collections_counter_public_diff_subset` /
  `cpython_collections_counter_public_subset`, covering CPython public
  `Counter` construction, mapping identity, missing-key zero lookup,
  values-based count totals, basic mutation helpers, `most_common()`,
  `elements()`, direct display/empty-format methods, update and subtract
  behavior, unary positive/negative filtering, multiset arithmetic, and equality
  with matching counts.
- The bundled `collections` module also includes
  `cpython_collections_counter_conversions_diff_subset` and
  `cpython_collections_counter_conversions_subset`, covering CPython public
  `Counter` conversion behavior for `elements()`, key iteration, `dict()`,
  `dict(items())`, and `set()` construction.
- The bundled `collections` module also includes
  `cpython_collections_counter_init_update_diff_subset` and
  `cpython_collections_counter_init_update_subset`, covering CPython public
  `Counter` construction and `update()` behavior for positional-only
  parameter names used as keys, `None` keyword values, and TypeError
  boundaries.
- The bundled `collections` module also includes
  `cpython_collections_counter_repr_nonsortable_diff_subset` and
  `cpython_collections_counter_repr_nonsortable_subset`, covering CPython
  public `Counter.__repr__()` behavior when count values cannot be directly
  sorted.
- The bundled `collections` module also includes
  `cpython_collections_counter_subtract_unary_diff_subset` and
  `cpython_collections_counter_subtract_unary_subset`, covering CPython public
  `Counter.subtract()` and unary plus/minus count filtering.
- The bundled `collections` module also includes
  `cpython_collections_counter_copy_subclass_diff_subset` and
  `cpython_collections_counter_copy_subclass_subset`, covering CPython public
  `Counter.copy()` behavior for Counter subclasses.
- The bundled `collections` module also includes
  `cpython_collections_counter_copying_diff_subset` and
  `cpython_collections_counter_copying_subset`, covering CPython public
  `Counter.copy()`, `copy.copy()`, `copy.deepcopy()`, `eval(repr(...))`,
  update, and constructor copying behavior.
- The bundled `collections` module also includes
  `cpython_collections_counter_order_preservation_diff_subset` and
  `cpython_collections_counter_order_preservation_subset`, covering CPython
  public Counter insertion-order preservation across construction,
  `elements()`, supported arithmetic and in-place arithmetic, `update()`, and
  `subtract()`.
- The bundled `collections` module also includes
  `cpython_collections_counter_update_reentrant_add_clears_counter_diff_subset` and
  `cpython_collections_counter_update_reentrant_add_clears_counter_subset`,
  covering CPython public Counter update behavior when count addition runs
  user code that clears the Counter before the replacement write.
- The bundled `collections` module also includes
  `cpython_collections_counter_helper_function_diff_subset` and
  `cpython_collections_counter_helper_function_subset`, covering CPython public
  `collections._count_elements()` behavior for exact dicts, OrderedDict order,
  and Counter subclass `__setitem__` / `get` hooks.
- The bundled `collections` module also includes
  `cpython_collections_counter_multiset_operations_diff_subset` and
  `cpython_collections_counter_multiset_operations_subset`, covering CPython
  public Counter multiset `+`, `-`, `|`, and `&` result filtering and direct
  dunder dispatch. Symmetric difference remains subset-only for the current
  local CPython oracle.
- The bundled `collections` module also includes
  `cpython_collections_counter_multiset_operations_matrix_diff_subset` and
  `cpython_collections_counter_multiset_operations_matrix_subset`, covering the
  deterministic CPython-style 1000-pair matrix for Counter multiset `+`, `-`,
  `|`, and `&` count formulas and positive-result filtering. Symmetric
  difference remains subset-only for the current local CPython oracle.
- The bundled `collections` module also includes
  `cpython_collections_counter_inplace_operations_diff_subset` and
  `cpython_collections_counter_inplace_operations_subset`, covering CPython
  public Counter in-place `+=`, `-=`, `|=`, and `&=` result parity and identity
  preservation. In-place symmetric difference remains subset-only for the
  current local CPython oracle.
- The bundled `collections` module also includes
  `cpython_collections_counter_inplace_operations_matrix_diff_subset` and
  `cpython_collections_counter_inplace_operations_matrix_subset`, covering the
  deterministic CPython-style 1000-pair matrix for in-place `+=`, `-=`, `|=`,
  and `&=` result parity and identity preservation. `^=` remains subset-only
  for the current local CPython oracle.
- The bundled `collections` module also includes
  `cpython_collections_chainmap_public_methods_diff_subset`, covering CPython public
  `ChainMap` construction, truthiness, combined iteration/items/dict coercion,
  membership and lookup across child/parent mappings, `get()` defaults,
  first-map mutation, direct display/empty-format methods, `parents`, and
  `new_child()` with a mapping input.
- The bundled `collections` module also includes
  `cpython_collections_chainmap_copy_sharing_diff_subset` and
  `cpython_collections_chainmap_copy_sharing_subset`, covering CPython public
  `ChainMap.copy()` and `copy.copy()` shallow-copy sharing behavior without
  pulling pickle/eval identity matrices into the default sandbox surface.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_public_diff_subset` and
  `cpython_collections_namedtuple_public_subset`, covering CPython public
  `namedtuple()` factory basics, generated tuple-like instances, field
  metadata, indexing, iteration, tuple/list conversion, `_make()`,
  `_replace()`, `_asdict()`, zero/one-field namedtuples, keyword construction,
  and representative invalid typename/field errors.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_defaults_rename_readonly_diff_subset` and
  `cpython_collections_namedtuple_defaults_rename_readonly_subset`, covering
  CPython public `namedtuple()` defaults, field renaming, module metadata,
  writable class docs, and readonly tuple-instance mutation boundaries.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_repr_diff_subset` and
  `cpython_collections_namedtuple_repr_subset`, covering CPython public
  `namedtuple` `repr()` spelling for base and subclass instances.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_name_conflicts_diff_subset` and
  `cpython_collections_namedtuple_name_conflicts_subset`, covering CPython
  public `namedtuple()` behavior for field names that collide with generated
  helper names, keywords, and documentation-only helper identifiers.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_subclass_issue_24931_diff_subset` and
  `cpython_collections_namedtuple_subclass_issue_24931_subset`, covering
  CPython public subclassed `namedtuple` `_asdict()` output and instance
  `__dict__` storage behavior.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_large_size_diff_subset` and
  `cpython_collections_namedtuple_large_size_subset`, covering deterministic
  CPython public `namedtuple()` behavior for many fields, `_make()`,
  `_replace()`, `_asdict()`, field lookup, and repr boundaries.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_field_doc_diff_subset` and
  `cpython_collections_namedtuple_field_doc_subset`, covering CPython public
  namedtuple field descriptor docs, independent descriptor doc mutation, and
  descriptor access through the class and instance.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_copy_keyword_generic_alias_diff_subset` and
  `cpython_collections_namedtuple_copy_keyword_generic_alias_subset`, covering
  CPython public namedtuple copy/deepcopy behavior, keyword-only factory
  argument handling, and generic alias construction/calls.
- The bundled `collections` module also includes
  `cpython_collections_namedtuple_new_builtins_globals_diff_subset` and
  `cpython_collections_namedtuple_new_builtins_globals_subset`, covering the
  CPython public generated namedtuple `__new__.__globals__['__builtins__']`
  empty-mapping behavior that is stable across the local oracle version.
  `cpython_collections_namedtuple_new_builtins_issue_43102_diff_subset` and
  `cpython_collections_namedtuple_new_builtins_issue_43102_subset` add gated
  direct evidence for the newer `function.__builtins__` public attribute.
- The bundled `collections` module also includes
  `cpython_collections_userdict_userlist_public_diff_subset` and
  `cpython_collections_userdict_userlist_public_subset`, covering CPython
  public `UserDict` and `UserList` construction, `.data` storage exposure,
  item assignment/deletion, lookup, iteration, containment, `get()`, length,
  `.copy()`, `copy.copy()`, shallow instance-attribute copying, and UserList
  construction from lists and other UserList objects.
- The bundled `collections` module also includes
  `cpython_collections_userdict_public_methods_diff_subset`, covering the
  CPython public `UserDict` method subset, including direct display and
  empty-format behavior, with direct output parity evidence.
- The bundled `collections` module also includes
  `cpython_collections_userlist_public_methods_diff_subset`, covering the
  CPython public `UserList` method subset, including direct display,
  empty-format, and recursive display behavior, with direct output parity
  evidence.
- The bundled `json` module includes `cpython_json_loads_dumps_diff_subset` /
  `cpython_json_loads_dumps_basic_diff_subset` /
  `cpython_json_loads_dumps_basic_subset`,
  `cpython_json_keyword_argument_binding_diff_subset` /
  `cpython_json_keyword_argument_binding_subset`,
  `cpython_json_loads_dumps_error_boundary_diff_subset` /
  `cpython_json_loads_dumps_error_boundary_subset`, plus
  `cpython_json_loads_escape_and_duplicate_key_diff_subset` /
  `cpython_json_loads_escape_and_duplicate_key_subset`,
  `cpython_json_loads_unicode_escape_roundtrip_diff_subset` /
  `cpython_json_loads_unicode_escape_roundtrip_subset`,
  `cpython_json_loads_string_error_boundary_diff_subset` /
  `cpython_json_loads_string_error_boundary_subset`,
  `cpython_json_loads_strict_diff_subset` /
  `cpython_json_loads_strict_subset`,
  `cpython_json_option_truthiness_diff_subset` /
  `cpython_json_option_truthiness_subset`,
  `cpython_json_loads_number_and_whitespace_diff_subset` /
  `cpython_json_loads_number_and_whitespace_subset`,
  `cpython_json_loads_int_digit_limit_diff_subset` /
  `cpython_json_loads_int_digit_limit_subset`,
  `cpython_json_loads_top_level_scalar_and_empty_container_diff_subset` /
  `cpython_json_loads_top_level_scalar_and_empty_container_subset`,
  `cpython_json_loads_nonfinite_constants_diff_subset` /
  `cpython_json_loads_nonfinite_constants_subset`,
  `cpython_json_loads_parse_hooks_diff_subset` /
  `cpython_json_loads_parse_hooks_subset`,
  `cpython_json_loads_object_hook_diff_subset` /
  `cpython_json_loads_object_hook_subset`,
  `cpython_json_loads_object_pairs_hook_diff_subset` /
  `cpython_json_loads_object_pairs_hook_subset`,
  `cpython_json_dumps_string_escape_diff_subset` /
  `cpython_json_dumps_string_escape_subset`,
  `cpython_json_dumps_key_coercion_diff_subset` /
  `cpython_json_dumps_key_coercion_subset`,
  `cpython_json_dumps_sequence_subclass_iter_diff_subset` /
  `cpython_json_dumps_sequence_subclass_iter_subset`,
  `cpython_json_dumps_allow_nan_diff_subset` /
  `cpython_json_dumps_allow_nan_subset`,
  `cpython_json_dumps_check_circular_diff_subset` /
  `cpython_json_dumps_check_circular_subset`,
  `cpython_json_dumps_ensure_ascii_diff_subset` /
  `cpython_json_dumps_ensure_ascii_subset`,
  `cpython_json_dumps_indent_diff_subset` /
  `cpython_json_dumps_indent_subset`,
  `cpython_json_dumps_skipkeys_diff_subset` /
  `cpython_json_dumps_skipkeys_subset`,
  `cpython_json_dumps_sort_keys_diff_subset` /
  `cpython_json_dumps_sort_keys_subset`,
  `cpython_json_dumps_separators_diff_subset` /
  `cpython_json_dumps_separators_subset`,
  `cpython_json_dumps_default_hook_diff_subset` /
  `cpython_json_dumps_default_hook_subset`, and
  `cpython_json_dumps_float_spelling_diff_subset` /
  `cpython_json_dumps_float_spelling_subset`, covering the pure in-memory
  first-pass `loads()` / `dumps()` public data model for objects,
  arrays, `str` / `bytes` / `bytearray` input values and subclasses, UTF-8 BOM
  and UTF-16/UTF-32 encoded byte input, ordinary `\uXXXX` escapes and valid
  surrogate-pair Unicode escapes, `loads()` / `dumps()` public function
  `__name__` / `__qualname__` / `__module__` / `__doc__` / `__dict__` / `__defaults__` /
  `__kwdefaults__` / `__annotations__` metadata, option truthiness through
  public `__bool__` dispatch with exception propagation, `strict=False` raw control-character string
  parsing, CPython default non-finite constants, strings and `str` /
  `int` / `float`
  subclass and `IntEnum` values/keys,
  list/tuple containers, list/tuple/namedtuple subclass containers through
  public iteration, non-empty dict subclass containers through public
  `items()` with CPython's 2-tuple item requirement, CPython's
  empty-dict-subclass `{}` fast path, `Counter` mapping containers, and
  namedtuples, standard string escapes,
  paired UTF-16 surrogate escapes, `allow_nan=False` rejection of non-finite
  float values and keys, `check_circular` cycle-error behavior,
  `ensure_ascii=False` direct non-ASCII string/key rendering,
  `indent` pretty-print formatting for int/string and `__index__` indent values,
  `skipkeys` omission of unsupported dict keys,
  `sort_keys` ordering for supported comparable keys, `separators`
  compact/custom rendering for two-string list/tuple values, subclasses, and
  general iterables plus CPython-style unpack length `ValueError` text and
  item/key separator element `TypeError` text, `default` hook handling for
  otherwise unsupported objects including nested values, exception propagation,
  CPython-style non-callable hook `TypeError` text, and returned-self circular
  detection,
  `separators=None` preserving indent's default item-separator behavior,
  duplicate-object-key last-value behavior, JSON whitespace, integer/float
  number grammar edges including leading-zero extra-data classification,
  `parse_int`, `parse_float`, and `parse_constant`
  hooks including CPython-style non-callable hook `TypeError` text,
  `object_hook` post-processing for decoded objects including nested
  objects, exception propagation, and non-callable hook `TypeError` text,
  `object_pairs_hook` pair-list
  post-processing with duplicate-key preservation and `object_hook`
  precedence plus non-callable hook `TypeError` text,
  `sys.set_int_max_str_digits()`
  enforcement for default parsed JSON integer values, top-level scalar values,
  empty containers, finite and
  default non-finite float spelling, booleans, null,
  CPython's basic dict-key coercion for `str` / `int` / `float` / `bool` /
  `None`, circular-reference rejection for list/dict/tuple/namedtuple
  container paths, raw control-character rejection, malformed escape rejection,
  invalid UTF-8 byte input `UnicodeDecodeError` type classification, and
  first-pass type, structural, literal, and data error classification. File APIs,
  non-`None` encoder/decoder hooks other than `object_hook`,
  `object_pairs_hook`, `parse_int`, `parse_float`, `parse_constant`, and
  `default`,
  `loads()` hooks/options other than `strict` / `object_hook` /
  `object_pairs_hook`,
  `dumps()` hooks/options other than `allow_nan` /
  `check_circular` / `ensure_ascii` / `indent` / `skipkeys` / `sort_keys` /
  `separators` / `default` such as non-`None` `cls`, the `JSONDecodeError` class and
  full `JSONDecodeError` compatibility, bytes/bytearray serialization, and
  unpaired surrogate storage remain intentionally outside this sandbox subset.
- The bundled `itertools` module includes
  `cpython_itertools_core_iterator_subset`,
  `cpython_itertools_keyword_error_subset`,
  `cpython_itertools_pairwise_subset`, and
  `cpython_itertools_product_subset`, and
  `cpython_itertools_combinations_subset`, and
  `cpython_itertools_combinations_with_replacement_subset`, and
  `cpython_itertools_permutations_subset`, and
  `cpython_itertools_tee_subset`, and
  `cpython_itertools_batched_subset`, and
  `cpython_itertools_groupby_subset`, and
  `cpython_itertools_repr_subset`, and
  `cpython_itertools_core_diff_subset`,
  `cpython_itertools_core_iterator_diff_subset`,
  `cpython_itertools_keyword_error_diff_subset`, plus the CPython 3.10+ gated
  `cpython_itertools_pairwise_diff_subset`, and
  `cpython_itertools_product_diff_subset`, and
  `cpython_itertools_combinations_diff_subset`, and
  `cpython_itertools_combinations_with_replacement_diff_subset`, and
  `cpython_itertools_permutations_diff_subset`, and
  `cpython_itertools_tee_diff_subset`, and
  `cpython_itertools_batched_diff_subset`, and
  `cpython_itertools_groupby_diff_subset`, and
  `cpython_itertools_repr_diff_subset`, covering the pure in-memory
  first-pass `accumulate()`, `count()`, `cycle()`, `repeat()`, `chain()`,
  `chain.from_iterable()`, `compress()`, `filterfalse()`, `takewhile()`,
  `dropwhile()`, `starmap()`, `zip_longest()`, `islice()`, `pairwise()`, and
  `product()`, `combinations()`, `combinations_with_replacement()`, and
  `permutations()`, `tee()`, `batched()`, and `groupby()` iterator protocol behavior, plus duplicate-keyword diagnostics for
  `accumulate()` and `zip_longest()`.
  This subset supports integer, float, complex, and bool `count()` arguments,
  nonnumeric `count()` argument rejection, finite and infinite
  `repeat()`, keyword forms for `count()` / `repeat()`, `chain()` over
  arbitrary supported iterables, lazy `chain.from_iterable()` flattening,
  default and callable `accumulate()` reduction with `initial`, cached
  `cycle()` replay over finite and generator sources,
  selector-truthy `compress()` filtering with shortest-input exhaustion,
  false-predicate `filterfalse()` selection with callable or `None`, and
  predicate-prefix `takewhile()` / `dropwhile()` termination behavior over
  finite and generator inputs, star-argument mapping over iterable argument
  rows, longest-zip padding with `fillvalue`, and
  non-negative integer `islice()` windows over finite, infinite, and
  generator-backed iterators,
  adjacent-pair iteration over finite, infinite, and generator-backed
  sources, and eager cartesian-product pools with `repeat`, `__index__`
  repeat conversion, zero-repeat empty tuple behavior, empty-pool exhaustion,
  generator inputs, and eager combination pools with `r`, keyword binding,
  `__index__` r conversion, zero-length tuple behavior, oversize-r exhaustion,
  generator inputs, and eager combinations-with-replacement pools with `r`,
  keyword binding, `__index__` r conversion, zero-length tuple behavior,
  empty-pool exhaustion, generator inputs, and eager permutations pools with
  default `r`, keyword binding, bool/int-subclass `r`, CPython's rejection of
  arbitrary `__index__` r values, zero-length tuple behavior, oversize-r
  exhaustion, generator inputs, and shared-buffer `tee()` clones with
  interleaved consumption, `n=0` / `n=1` / multi-clone behavior, `n=0`
  non-iterable short-circuiting, `_tee` input flattening with first-clone
  identity reuse, `n` via `__index__`, generator-backed inputs, and fixed-size
  `batched()` tuple batches with generator inputs, `n` via `__index__`, keyword binding,
  truth-tested `strict`, and incomplete-batch `ValueError`, and lazy
  `groupby()` runs with optional key functions, generator-backed input,
  shared `_grouper` invalidation when the parent advances, keyword binding,
  and public `repr()` shapes for `count()`, `repeat()`, `cycle()`, `_tee`, and
  `groupby()` without binding object addresses, plus public iterator/helper
  type `__module__` / `__qualname__` / `__doc__` metadata including
  `_grouper.__doc__ is None`, and public constructor `__qualname__` /
  `__module__` / `__doc__` metadata, including the gated `pairwise()` and
  `batched()` constructor surfaces. Full itertools module,
  pickling exactness, exact address repr, `tee()` cache compaction, and
  remaining public/helper types remain outside the sandbox subset.
- The bundled `math` module also includes `cpython_math_isclose_diff_subset`
  and `cpython_math_isclose_subset`, covering CPython
  `test_math.py::IsCloseTests` public relative and absolute tolerance behavior,
  identical values, near-zero comparisons, infinity/NaN handling, keyword-only
  tolerances, real-number input conversion, negative tolerance rejection, and
  catchable error classes.
- The bundled `math` module also includes `cpython_math_hypot_dist_diff_subset`
  and `cpython_math_hypot_dist_subset`, covering CPython
  `test_math.py::MathTests::testHypot` and `::testDist` public Euclidean
  norm/distance behavior, variadic `hypot()`, iterable `dist()` inputs,
  real-number conversion, signed-zero normalization, NaN/inf propagation,
  large/small-value scaling, dimension validation, and catchable error classes.
- The bundled `math` module also includes `cpython_math_gcd_diff_subset` and
  `cpython_math_gcd_subset`, covering CPython
  `test_math.py::MathTests::testGcd` integer, big-integer, variadic,
  empty-call, negative-input, `__index__`, and non-integer rejection behavior.
- The bundled `math` module also includes `cpython_math_lcm_diff_subset` and
  `cpython_math_lcm_subset`, covering CPython
  `test_math.py::MathTests::test_lcm` integer, big-integer, variadic,
  empty-call, zero, negative-input, `__index__`, and non-integer rejection
  behavior.
- The bundled `math` module also includes `cpython_math_integer_diff_subset`
  and `cpython_math_integer_subset`, covering CPython
  `test_math_integer.py::IntMathTests` and `::MathTests` public integer math
  behavior for `factorial()`, `isqrt()`, `comb()`, and `perm()` through both
  `math` and `math.integer`, including exact integer results,
  bool/int-subclass/`__index__` conversion, negative-domain errors, and
  catchable TypeError cases. The default diff covers the stable `math` module
  surface; the `math.integer` alias stays in the local subset because older
  system CPython oracles do not expose that submodule.
- The bundled `math` module also includes `cpython_math_pow_diff_subset` and
  `cpython_math_pow_subset`, covering CPython
  `test_math.py::MathTests::testPow` float-result power behavior, NaN/inf
  special cases, zero and signed-zero semantics, negative-base domain errors
  for non-integral exponents, finite overflow handling, `__float__` and
  `__index__` input conversion, and catchable error classes.
- The bundled `math` module also includes `cpython_math_prod_diff_subset` and
  `cpython_math_prod_subset`, covering CPython
  `test_math.py::MathTests::test_prod` iterable multiplication, keyword-only
  `start`, integer/float products, sequence repetition, zero, NaN/inf
  propagation, type preservation, and TypeError cases supported by the current
  runtime.
- The bundled `math` module also includes `cpython_math_fabs_diff_subset` and
  `cpython_math_fabs_subset`, covering CPython
  `test_math.py::MathTests::testFabs` real-number conversion, float result,
  signed-zero normalization, NaN/inf propagation, huge-integer overflow, and
  TypeError cases supported by the current runtime.
- The bundled `math` module also includes `cpython_math_fma_diff_subset`,
  gated for newer CPython oracles, and `cpython_math_fma_subset`, covering
  CPython `test_math.py::FMATests` public fused multiply-add behavior,
  single-round examples, signed-zero results, NaN propagation, infinity
  invalid-operation cases, finite overflow, real-number input conversion, and
  catchable error classes.
- The bundled `math` module also includes `cpython_math_fmax_fmin_diff_subset`,
  gated for newer CPython oracles, and `cpython_math_fmax_fmin_subset`,
  covering CPython `test_math.py::MathTests::test_fmax`, `::test_fmax_nans`,
  `::test_fmin`, and `::test_fmin_nans` public two-argument floating min/max
  behavior, NaN elision, infinity handling, real-number input conversion, and
  catchable error classes.
- The bundled `math` module also includes
  `cpython_math_fmod_remainder_diff_subset` and
  `cpython_math_fmod_remainder_subset`,
  covering CPython `test_math.py::MathTests::testFmod` and `::testRemainder`
  public floating remainder behavior, sign-preserving `fmod()`, IEEE-style
  nearest-even `remainder()`, NaN propagation, infinity/zero-domain errors,
  `__float__` and `__index__` input conversion, huge-index overflow,
  propagated conversion exceptions, and catchable error classes.
- The bundled `math` module also includes
  `cpython_math_frexp_ldexp_modf_diff_subset` and
  `cpython_math_frexp_ldexp_modf_subset`, covering CPython
  `test_math.py::MathTests::testFrexp`, `::testLdexp`,
  `::testLdexp_denormal`, and `::testModf` public floating
  decomposition/scaling behavior, signed-zero preservation, NaN/inf
  propagation, denormal `ldexp()` output, real-number input conversion,
  `ldexp()`'s strict integer exponent rule, overflow/underflow behavior, and
  catchable error classes.
- The bundled `math` module also includes `cpython_math_fsum_diff_subset` and
  `cpython_math_fsum_subset`, covering
  CPython `test_math.py::MathTests::testFsum` full-precision summation
  behavior, cancellation-sensitive inputs, half-even rounding boundaries,
  iterable input conversion, NaN/inf handling, finite overflow detection,
  propagated iterator exceptions, `__float__` and `__index__` input conversion,
  and catchable error classes.
- The bundled `math` module also includes `cpython_math_sumprod_diff_subset`,
  gated for newer CPython oracles, and `cpython_math_sumprod_subset`, covering
  CPython `test_math.py::MathTests::testSumProd` built-in numeric dot-product
  behavior supported by the current runtime: strict paired iteration, exact
  integer results, float/mixed numeric summation accuracy, NaN/inf handling,
  huge-integer float overflow, and catchable error classes.
- The bundled `math` module also includes
  `cpython_math_nextafter_ulp_diff_subset`, gated for newer CPython oracles
  with `nextafter(steps=...)`, and `cpython_math_nextafter_ulp_subset`,
  covering CPython `test_math.py::MathTests::test_nextafter` and `::test_ulp`
  IEEE-754 adjacent-float behavior, `steps`, signed-zero/subnormal
  transitions, infinity/NaN cases, ULP magnitudes, real-number input
  conversion, and catchable error classes.
- The bundled `math` module also includes `cpython_math_sqrt_diff_subset` and
  `cpython_math_sqrt_subset`, covering CPython
  `test_math.py::MathTests::testSqrt` zero, positive integer/float, infinity,
  NaN, float-result, negative-domain `ValueError`, huge-integer overflow, and
  TypeError cases supported by the current runtime.
- The bundled `math` module also includes `cpython_math_copysign_diff_subset`
  and `cpython_math_copysign_subset`, covering CPython
  `test_math.py::MathTests::testCopysign` sign transfer for zeroes,
  infinities, NaNs, huge-integer overflow, and TypeError cases supported by the
  current runtime.
- The bundled `math` module also includes the newer-CPython-oracle-gated
  `cpython_math_signbit_diff_subset` and `cpython_math_signbit_subset`,
  covering CPython `test_math.py::MathTests::test_signbit` negative sign-bit
  detection for zeroes, finite values, infinities, NaNs, bool/int conversion,
  huge-integer overflow, and TypeError cases supported by the current runtime.
- The bundled `math` module also includes `cpython_math_trunc_diff_subset` and
  `cpython_math_trunc_subset`, covering CPython
  `test_math.py::MathTests::test_trunc` integer/float truncation, bool and
  big-integer preservation, exact large finite-float integer results, normal
  `__trunc__` special-method dispatch with direct return-value propagation,
  propagated exceptions, NaN/inf integer conversion errors, and TypeError cases
  supported by the current runtime.
- The bundled `math` module also includes `cpython_math_ceil_diff_subset`,
  `cpython_math_ceil_subset`, `cpython_math_floor_diff_subset`, and
  `cpython_math_floor_subset`, covering CPython
  `test_math.py::MathTests::testCeil` and `::testFloor` public numeric
  rounding-to-integral behavior, bool and big-integer preservation, exact large
  finite-float integer results, normal `__ceil__` / `__floor__` dispatch with
  direct return-value propagation, `__float__` and `__index__` fallback,
  NaN/inf integer conversion errors, huge-index overflow, and TypeError cases
  supported by the current runtime.
- The bundled `math` module also includes
  `cpython_math_degrees_radians_diff_subset` and
  `cpython_math_degrees_radians_subset`, covering CPython
  `test_math.py::MathTests::testDegrees` and `::testRadians` angle conversion,
  float result semantics, non-finite propagation, `__float__` and `__index__`
  input conversion, huge-index overflow, propagated conversion exceptions, and
  TypeError cases supported by the current runtime.
- The bundled `math` module also includes the newer-CPython-oracle-gated
  `cpython_math_cbrt_diff_subset` and `cpython_math_cbrt_subset`, covering
  CPython `test_math.py::MathTests::testCbrt` cube-root behavior, float result
  semantics, signed zero, non-finite propagation, `__float__` and `__index__`
  input conversion, huge-index overflow, propagated conversion exceptions, and
  TypeError cases supported by the current runtime.
- The bundled `math` module also includes `cpython_math_erf_erfc_diff_subset`
  and `cpython_math_erf_erfc_subset`, covering a representative CPython
  `test_math.py::MathTests` public `erf()` / `erfc()` slice with six-decimal
  stable values, signed zero, non-finite propagation, `__float__` and
  `__index__` input conversion, huge-index overflow, propagated conversion
  exceptions, and TypeError cases. Exact platform libm precision remains out
  of scope unless separately promoted.
- The bundled `math` module also includes `cpython_math_gamma_lgamma_diff_subset`
  and `cpython_math_gamma_lgamma_subset`, covering representative CPython
  `test_math.py::MathTests` public `gamma()` / `lgamma()` values with
  six-decimal stable checks, non-finite propagation, pole/domain errors,
  `__float__` and `__index__` input conversion, huge-index overflow,
  propagated conversion exceptions, and TypeError cases. Exact platform libm
  special-function precision remains out of scope unless separately promoted.
  Platform/libm implementation quirks, exact libm special-function precision,
  and locale-sensitive parsing/formatting stay outside the sandbox `math`
  surface until separately promoted with direct public-behavior evidence.
- The bundled `math` module also includes the newer-CPython-oracle-gated
  `cpython_math_exp_exp2_diff_subset` and `cpython_math_exp_exp2_subset`,
  covering CPython `test_math.py::MathTests::testExp` and `::testExp2`
  exponential behavior, float result semantics, non-finite propagation,
  finite-input overflow errors, `__float__` and `__index__` input conversion,
  huge-index overflow, propagated conversion exceptions, and TypeError cases
  supported by the current runtime.
- The bundled `math` module also includes `cpython_math_expm1_diff_subset` and
  `cpython_math_expm1_subset`, covering CPython
  `test_math.py::MathTests::test_expm1` public exponential-minus-one behavior,
  signed zero, non-finite propagation, finite-input overflow errors,
  `__float__` and `__index__` input conversion, huge-index overflow,
  propagated conversion exceptions, and TypeError cases supported by the
  current runtime.
- The bundled `math` module also includes `cpython_math_log_family_diff_subset`
  and `cpython_math_log_family_subset`, covering CPython
  `test_math.py::MathTests::testLog`, `::testLog1p`, `::testLog2`,
  `::testLog2Exact`, and `::testLog10` public logarithm behavior,
  optional-base division, non-finite propagation, large-integer logarithms that
  avoid float-conversion overflow, `__float__` / `__index__` input conversion,
  and catchable error classes. The local subset additionally covers
  huge `__index__` inputs and OverflowError-to-`__index__` fallback for log
  helpers because the default CPython 3.9 oracle does not provide stable
  cross-version evidence for those boundaries.
- The bundled `math` module also includes `cpython_math_trig_diff_subset` and
  `cpython_math_trig_subset`, covering CPython
  `test_math.py::MathTests::testAcos`, `::testAsin`, `::testAtan`,
  `::testAtan2`, `::testCos`, `::testSin`, and `::testTan` public
  trigonometric behavior, domain errors, signed-zero `atan2()` behavior,
  non-finite propagation/rejection, `__float__` and `__index__` input
  conversion, huge-index overflow, propagated conversion exceptions, and
  catchable error classes.
- The bundled `math` module also includes
  `cpython_math_hyperbolic_diff_subset` and
  `cpython_math_hyperbolic_subset`, covering CPython
  `test_math.py::MathTests::testAcosh`, `::testAsinh`, `::testAtanh`,
  `::testCosh`, `::testSinh`, `::testTanh`, and `::testTanhSign` public
  hyperbolic behavior, domain errors, finite-input overflow errors, non-finite
  propagation, signed-zero `tanh()` behavior, `__float__` and `__index__` input
  conversion, huge-index overflow, propagated conversion exceptions, and
  catchable error classes.
- The bundled `functools` module includes
  `cpython_functools_public_helpers_diff_subset` and
  `cpython_functools_public_helpers_subset`, covering direct CPython
  output parity for representative public `reduce`, `partial`, `wraps`, and
  `cmp_to_key` helper behavior plus basic TypeError classification.
- The bundled `functools` module includes
  `cpython_functools_partial_diff_subset` and
  `cpython_functools_partial_subset`, covering CPython
  `test_functools.py::TestPartial` public call semantics, `func` / `args` /
  `keywords` attributes, live `keywords` mapping mutation including `str`
  subclass keys and non-string-key call rejection, caller keyword-dict
  isolation, constructor keyword copying, positional/keyword combinations,
  public `repr()` shape for function/type callables and stored arguments,
  exception propagation, nested partial calls, custom attributes, default and
  instance-overridden `__doc__` metadata, default and instance-overridden
  `__module__` metadata with deletion fallback, and readonly core attributes.
  The newer CPython partial instance `__module__` surface is also pinned by
  `cpython_functools_partial_instance_module_metadata_subset` and gated
  `cpython_functools_partial_instance_module_metadata_diff_subset`.
- The bundled `functools` module also includes
  `cpython_functools_partialmethod_diff_subset` and
  `cpython_functools_partialmethod_subset`, covering CPython
  `test_functools.py::TestPartialMethod` public descriptor behavior, instance
  and class access call argument order, nested `partialmethod` flattening,
  partial-over-partial calls, `staticmethod` and `classmethod` descriptors,
  keyword override behavior, bound and unbound `__self__` visibility,
  descriptor `__dict__` entries for `func`, `args`, and `keywords` including
  override/delete behavior, descriptor `__module__` / `__doc__` metadata
  including instance overrides, class-access `_method` function metadata, public
  `repr()` shape for empty, positional/keyword, and partial-over-partial
  descriptors plus instance-bound partialmethod calls, invalid constructor
  forms, and raw descriptor non-callability/type reporting.
- The bundled `functools` module also includes
  `cpython_functools_cmp_to_key_diff_subset` and
  `cpython_functools_cmp_to_key_subset`, covering CPython
  `test_functools.py::TestCmpToKey` public key-wrapper behavior, direct rich
  comparisons, `sorted(..., key=cmp_to_key(...))`, callable wrappers, mutable
  `obj`, public wrapper and object `repr()` / `str()` shape, unhashability,
  argument validation, non-wrapper comparison errors, and comparator exception
  propagation.
- The bundled `functools` module also includes
  `cpython_functools_update_wrapper_wraps_diff_subset` and
  `cpython_functools_update_wrapper_wraps_subset`, covering CPython
  `test_functools.py::TestUpdateWrapper` and `::TestWraps` public metadata
  copying behavior, wrapper constants, `__wrapped__`, default and selective
  assigned/updated attributes, callable `wraps()` decorators, missing-attribute
  handling, and the supported eager-annotation bridge through `__annotate__`.
- The bundled `functools` module also includes
  `cpython_functools_total_ordering_diff_subset` and
  `cpython_functools_total_ordering_subset`, covering CPython
  `test_functools.py::TestTotalOrdering` public decorator behavior for all four
  root ordering methods, generated method metadata, no-overwrite behavior,
  missing-root `ValueError`, direct `NotImplemented` propagation, and operator
  `TypeError` fallback without relying on pickle identity or metaclass ordering
  internals.
- The bundled `functools` module also includes
  `cpython_functools_cache_diff_subset` and
  `cpython_functools_cache_subset`, plus gated
  `cpython_functools_cache_wrapper_module_metadata_diff_subset` and
  `cpython_functools_cache_wrapper_module_metadata_subset`, covering CPython
  `test_functools.py::TestCache` and public `TestLRU` cache-wrapper behavior
  for `cache`, unbounded `lru_cache`, finite LRU eviction, `cache_info`,
  `cache_clear`, `cache_parameters`, `cache_info` / `cache_clear` bound-method
  metadata, `__wrapped__`, wrapper instance attributes, public wrapper
  `repr()` / `str()` shape, direct `@lru_cache` decoration, zero and negative
  maxsize behavior, user-function exceptions not being cached,
  `typed=True` top-level key separation, non-recursive typed tuple behavior,
  keyword-order-sensitive cache keys, full keyword-recursive `maxsize=None`
  statistics and clearing,
  recursive calls that mutate the cache during a miss, empty `**{}`
  equivalence with no keywords, `*args` key shape, cached method descriptor
  binding with shared cache statistics, wrapper-assignment metadata,
  bound-method wrapper metadata and instance-side cache control,
  wrapper `__module__` metadata override and deletion fallback to `functools`,
  cache-parameter snapshot isolation, unhashable arguments, shallow/deep copy
  identity preservation for cached wrappers, finite-cache exception misses, and
  CPython-compatible cache statistics including size-one/size-two LRU behavior
  and cached `builtins.len` reentrancy.
- The bundled `functools` module also includes
  `cpython_functools_cached_property_diff_subset` and
  `cpython_functools_cached_property_subset`, covering CPython
  `test_functools.py::TestCachedProperty` public descriptor behavior for
  instance `__dict__` caching, class-level descriptor access, copied doc,
  live public `__dict__` state for `func`, `attrname`, and `__doc__` metadata,
  public descriptor `repr()` / `str()` shape, runtime updates to `func` and
  string/`None` `attrname`, attributes whose names differ from the wrapped
  function, reuse rejection under different names, reuse under the same name
  across classes, explicit post-class assignment before `__set_name__`,
  slot-only instances without `__dict__`, and the shared user-descriptor
  `__set_name__` hook plus CPython-style
  `RuntimeError` wrapping for descriptor failures during both class statements
  and `type()` dynamic class creation.
  `cpython_functools_cached_property_module_metadata_subset` and gated
  `cpython_functools_cached_property_module_metadata_diff_subset` cover the
  newer wrapped-function `__module__` metadata plus override and deletion
  fallback.
- The bundled `functools` module also includes
  `cpython_functools_reduce_diff_subset` and
  `cpython_functools_reduce_subset`, covering CPython
  `test_functools.py::TestReduce` public reduction behavior over built-in
  iterables, sequence-protocol iterables, dictionaries, positional initializer
  values, empty-input errors, non-callable edge cases, and propagated
  iterator/function exceptions. The local subset also covers current CPython
  keyword `initial` binding, which stays out of the default diff because older
  system CPython oracles do not expose that keyword.
- The bundled `functools` module also includes
  `cpython_functools_singledispatch_diff_subset` and
  `cpython_functools_singledispatch_subset`, covering CPython
  `test_functools.py::TestSingleDispatch` public wrapper behavior, explicit
  type registration, decorator registration, `dispatch()` identity, registry
  mappingproxy exposure, public wrapper `__dict__` entries for `register`,
  `dispatch`, `registry`, and `_clear_cache` including override/deletion
  behavior, wrapper metadata copied from the wrapped function, public wrapper
  `repr()` / `str()` shape, C3-style user-class specificity,
  builtin `bool` / `int` dispatch, ABC registration over `Sized`,
  `MutableMapping`, and `MutableSequence`, no-op `_clear_cache()`,
  annotation-inferred registration, PEP 604 and
  `typing.Union` registration, lazy failure for non-callable implementations,
  and TypeError rejection for non-class registration/dispatch keys. The default
  diff covers the stable explicit-registration core; current strict invalid-key
  rejection stays in the local subset because older system CPython oracles
  accept some of those boundary calls.
- The bundled `functools` module also includes
  `cpython_functools_singledispatchmethod_diff_subset` and
  `cpython_functools_singledispatchmethod_subset`, covering CPython
  `test_functools.py::TestSingleDispatchMethod` public descriptor behavior,
  including instance and class access, descriptor `func` / `dispatcher` /
  `register` attributes, descriptor and bound callable `repr()` / `str()`
  shapes, descriptor `__module__` metadata override and deletion fallback to
  `functools`, explicit and decorator registration through raw, class-bound,
  and instance-bound access, `staticmethod` and `classmethod` implementations,
  annotation-inferred registration, PEP 604 and
  `typing.Union` registration, and public TypeError paths. The default diff
  covers stable explicit-registration and descriptor-composition behavior;
  missing-argument error classification stays in the local subset because older
  system CPython oracles raise a different exception class on that boundary.
  Full CPython cache implementation internals, weakref/lifecycle subtleties,
  and unsupported descriptor edge cases remain outside the sandbox subset until
  separately promoted with direct public-behavior evidence.
- The bundled `typing` module includes
  `cpython_typing_get_origin_args_subset`, covering CPython public
  `get_args()` and `get_origin()` behavior for builtin and user generic aliases,
  PEP 604 unions, `typing.Union[...]`, and non-generic values.
- The bundled `types` module also includes
  `cpython_types_generic_alias_union_type_subset`, covering CPython public
  `GenericAlias` and `UnionType` type objects, `type()` identity,
  `isinstance()` checks, public type metadata, GenericAlias construction, and
  catchable constructor TypeError paths. Direct output parity is guarded by
  `cpython_types_generic_alias_union_type_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_forward_ref_subset`, covering public
  `typing.ForwardRef` construction, metadata attributes, equality/hash
  behavior, `typing.Union[int, "str"]`, `types.UnionType[int, "str"]`,
  `typing.Optional["str"]`, TypeVar/string PEP 604 forward references, and
  invalid string/type union operands. Direct output parity is guarded by
  `cpython_types_union_forward_ref_diff_subset`.
  `cpython_types_union_forward_get_type_hints_subset`
  covers `typing.get_type_hints()` resolving those forward references through
  function globals while preserving `typing.get_args()` order. Direct output
  parity is guarded by
  `cpython_types_union_forward_get_type_hints_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_public_operator_and_classinfo_subset`, covering the
  supported CPython `UnionTests` public PEP 604 surface for order-insensitive
  union equality and hashing, legacy `typing.List` / `typing.Tuple` /
  `typing.Callable` alias operands, `typing.Hashable`, `typing.GenericAlias`
  compatibility construction, `None` /
  `NoneType` normalization, nested union flattening, single-member
  simplification, invalid operands, ordering TypeErrors, union `__args__`, long
  builtin union chains, namedtuple type operands, `typing.get_origin()` /
  `get_args()`, `repr()` / `str()` for supported union operands, and
  `isinstance()` / `issubclass()` classinfo dispatch including
  `collections.abc.Mapping`, plus GenericAlias union repr and invalid classinfo
  checks. Direct output parity is guarded by
  `cpython_types_union_public_operator_and_classinfo_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_genericalias_subclass_bad_eq_subset`, covering
  `types.GenericAlias` subclasses, subclass alias attributes, payload
  equality/hash/repr, PEP 604 union equality/deduplication/order behavior,
  invalid classinfo checks for GenericAlias-subclass unions, and propagation of
  bad metaclass `__eq__` exceptions during union equality and construction.
  Direct output parity is guarded by
  `cpython_types_union_genericalias_subclass_bad_eq_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_typevar_parameter_subset`, covering CPython
  `UnionTests` TypeVar union behavior for both operand orders, classinfo order
  semantics for unresolved TypeVars,
  one-parameter substitution that simplifies duplicate union operands,
  `__parameters__` tracking, nested generic-alias substitution, multi-TypeVar
  substitution, and post-substitution deduplication. Direct output parity is
  guarded by `cpython_types_union_typevar_parameter_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_parameter_substitution_subset`, covering CPython
  `UnionTests` parameter substitution for supported builtin, generic-alias,
  legacy typing-alias, `collections.abc`, nested union, deduplicating union, and
  remaining-TypeVar operands, plus `typing.Literal` / `typing.NewType`
  substitutions and the public arity `TypeError` path. Direct output parity is
  guarded by `cpython_types_union_parameter_substitution_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_copy_pickle_subset`, covering CPython `UnionTests`
  `copy.copy()`, `copy.deepcopy()`, and all exposed pickle protocol round trips
  for TypeVar-containing PEP 604 union aliases while preserving equality,
  arguments, parameters, non-identity copies, and public union type names.
  Direct output parity is guarded by
  `cpython_types_union_copy_pickle_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_bad_classinfo_checks_subset`, covering CPython
  `UnionTests` propagation of custom metaclass `__instancecheck__` and
  `__subclasscheck__` exceptions through PEP 604 union classinfo after leading
  concrete union members short-circuit successfully. Direct output parity is
  guarded by `cpython_types_union_bad_classinfo_checks_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_unhashable_metaclass_subset`, covering CPython
  `UnionTests` custom metaclass `__hash__ = None` behavior for PEP 604 union
  hashing, including preserved `__args__` ordering and catchable `TypeError`
  propagation with the metaclass name. Direct output parity is guarded by
  `cpython_types_union_unhashable_metaclass_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_dynamic_hashability_subset`, covering CPython
  `UnionTests` dynamic unhashable-to-hashable metaclass behavior for PEP 604
  union hashing, including the cached `union contains N unhashable elements`
  error on old union objects and immediate hashability for fresh unions. Direct
  output parity is guarded by
  `cpython_types_union_dynamic_hashability_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_newtype_subset`, covering public `typing.NewType`
  metadata, call pass-through behavior, and PEP 604 union equality. Direct
  output parity is guarded by `cpython_types_union_newtype_diff_subset`.
- The bundled `types` module also includes `cpython_types_union_io_subset`,
  covering public `typing.IO` metadata, `typing.IO | str` equality with
  `typing.Union[typing.IO, str]`, `typing.IO[str]` generic alias union
  behavior, and bare `typing.TextIO` / `typing.BinaryIO` union operands. Direct
  output parity is guarded by `cpython_types_union_io_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_typed_dict_subset`, covering class-based
  `typing.TypedDict` creation, call construction as a dict, and PEP 604 union
  equality with `typing.Union`. Direct output parity is guarded by
  `cpython_types_union_typed_dict_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_protocol_subset`, covering class-based
  `typing.Protocol` subclass creation and PEP 604 union equality with
  `typing.Union`. Direct output parity is guarded by
  `cpython_types_union_protocol_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_special_form_subset`, covering public PEP 604 union
  behavior for `typing.Any`, `typing.NoReturn`, `typing.Optional[int]`,
  optional-union flattening, and extending existing `typing.Union` aliases.
  Direct output parity is guarded by
  `cpython_types_union_special_form_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_literal_subset`, covering `typing.Literal[...]` union
  args, repr, deduplication, bool-vs-int distinct literal aliases, and the
  public `enum.IntEnum` literal rows that keep enum-member literals distinct
  from equal int/bool literals while preserving enum alias identity. Direct
  output parity is guarded by `cpython_types_union_literal_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_union_bad_module_guard_subset`, covering the CPython
  `UnionTests` bad-metaclass-module regression path by requiring catchable
  exceptions instead of a runtime crash. Direct output parity is guarded by
  `cpython_types_union_bad_module_guard_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_singleton_type_aliases_subset`, covering CPython public
  `NoneType`, `NotImplementedType`, and `EllipsisType` aliases, `type()`
  identity, `isinstance()` checks, public type metadata, zero-argument singleton
  construction, and catchable constructor TypeError paths.
- The bundled `types` module also includes `cpython_types_module_type_subset`,
  covering CPython public `types.ModuleType` alias identity, construction
  defaults, `name=` / `doc=` keyword construction, builtins metadata, module
  `__class__`, module attribute mutation, and catchable constructor TypeError
  paths.
- The bundled `types` module also includes
  `cpython_types_coroutine_public_subset`, covering CPython
  `Lib/test/test_types.py::CoroutineTests` public `types.coroutine()` behavior
  for wrong arguments, non-generator return values, native coroutine
  pass-through, coroutine-like and coroutine-generator-like object pass-through,
  returning already iterable-coroutine generators, generator-function decorator
  identity, and iterable-coroutine flags. Direct output parity is guarded by
  `cpython_types_coroutine_public_diff_subset`.
  `cpython_types_coroutine_async_def_subset`
  covers stable function `__code__` identity and native coroutine `cr_code`
  flags. Direct output parity is guarded by
  `cpython_types_coroutine_async_def_diff_subset`.
  `cpython_types_coroutine_generator_wrapper_subset`,
  `cpython_types_coroutine_generator_frame_subset`,
  `cpython_types_coroutine_generator_yieldfrom_subset`, and
  `cpython_types_coroutine_duck_generator_wrapper_subset`,
  `cpython_types_coroutine_duck_generator_await_subset`, and
  `cpython_types_coroutine_duck_generator_proxy_subset` cover
  `_GeneratorWrapper` type/ABC relationships, repr/dir, native-generator
  forwarding, exact native generator `__name__` / `__qualname__` / `gi_code` /
  `gi_frame` / `gi_yieldfrom` and wrapper `cr_code` / `cr_frame` / `cr_await`
  stable identity, direct duck-generator forwarding,
  duck-generator await execution,
  duck-generator attribute pass-through and `cr_*` aliases,
  `unittest.mock.MagicMock` proxy call verification, catchable wrapper argument
  TypeErrors, propagated throw exceptions, double-wrap avoidance, and
  `weakref.ref(wrapper)` alive-reference identity. Direct output parity for
  the native generator wrapper slice is guarded by
  `cpython_types_coroutine_generator_wrapper_diff_subset`; frame alias parity
  is guarded by `cpython_types_coroutine_generator_frame_diff_subset`; and
  yield-from alias parity is guarded by
  `cpython_types_coroutine_generator_yieldfrom_diff_subset`. Duck-generator
  wrapper parity is guarded by
  `cpython_types_coroutine_duck_generator_wrapper_diff_subset`; duck-generator
  await parity is guarded by
  `cpython_types_coroutine_duck_generator_await_diff_subset`; and
  duck-generator proxy parity is guarded by
  `cpython_types_coroutine_duck_generator_proxy_diff_subset`.
- The bundled `types` module also includes `cpython_types_function_type_subset`,
  covering CPython `Lib/test/test_types.py::FunctionTests` public
  `types.FunctionType` construction over MiniPython code objects, globals
  dictionaries, explicit and code-derived names, positional defaults,
  keyword-only defaults, function type identity, callable execution, and
  catchable wrong-default TypeError paths.
- The bundled `types` module also includes
  `cpython_types_names_public_surface_subset`, covering the current CPython
  `TypesTests::test_names` public `types.__all__` name set, visible module
  attributes for every exported name, function/lambda and builtin
  function/method alias identity, `_types` accelerator alias identity, and
  basic type-object shape for descriptor, capsule, lazy-import, and
  frame-locals-proxy aliases. Concrete wrapper descriptor behavior is covered
  separately, capsule behavior is classified with the CPython-internal
  capsule test, and frame-locals-proxy behavior is covered by
  `cpython_types_frame_locals_proxy_type_subset`. Direct output parity is
  guarded by `cpython_types_names_public_surface_diff_subset`, gated for
  CPython oracles with the current public `types.__all__` surface.
- The bundled `types` module also includes
  `cpython_types_float_constructor_edges_subset`, covering the CPython
  `TypesTests::test_float_constructor` empty-string and embedded-NUL
  `float()` constructor `ValueError` rows. Direct output parity is guarded by
  `cpython_types_float_constructor_edges_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_float_to_string_subset`, covering the CPython
  `TypesTests::test_float_to_string` exponent rendering matrix for `%e` and
  direct `float.__format__('e')`, three-digit exponent rows, `%g` / `%#g`,
  public numeric `__format__` exposure on `int`, `bool`, and `float`, and
  descriptor/type-error boundaries. Direct output parity is guarded by
  `cpython_types_float_to_string_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_normal_integers_subset`, covering the public CPython
  `TypesTests::test_normal_integers` integer addition, comparison,
  multiplication-commutativity regression, floor-division/multiplication rows
  around `sys.maxsize`, unified `int` result type checks, boundary
  `isinstance()` checks, and negative-shift `ValueError` behavior while
  leaving CPython's small-integer object-sharing assertion outside the
  MiniPython contract. Direct output parity is guarded by
  `cpython_types_normal_integers_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_int_format_subset`, covering CPython
  `TypesTests::test_int__format__` for the complete non-locale direct
  `int.__format__()` matrix: decimal, character, binary, octal, hexadecimal,
  sign/alignment interaction, alternate prefixes, zero fill, comma grouping,
  huge integers, disallowed precision, invalid `c` combinations, non-string
  format specs, invalid type-code sweep, float presentation types, and custom
  fill/alignment regression rows.
- The bundled `types` module also includes
  `cpython_types_float_format_subset`, covering CPython
  `TypesTests::test_float__format__` for the complete non-locale direct
  `float.__format__()` and `format()` matrix: default, fixed, scientific,
  general, percent, sign, no-explicit-type, zero padding, comma grouping,
  alternate form, huge fixed-output, invalid integer-presentation, non-string
  format specs, and custom fill/alignment rows.
- The bundled `types` module also includes
  `cpython_types_format_spec_errors_subset`, covering CPython
  `TypesTests::test_format_spec_errors` huge width, huge precision, combined
  huge width/precision, and comma-disallowed type-code `ValueError` rows for
  the shared format mini-language parser. Direct output parity is guarded by
  `cpython_types_format_spec_errors_diff_subset`.
- The bundled `types` module also includes
  `cpython_types_method_descriptor_types_subset`, covering CPython
  `TypesTests::test_method_descriptor_types` for `str.join` and `list.append`
  as method descriptors, bound string/list methods as builtin methods,
  `int.__dict__['from_bytes']` as a classmethod descriptor, `int.from_bytes`
  and `int.__new__` as builtin methods, and executable unbound descriptor calls
  for the covered list and int methods.
- The bundled `types` module also includes
  `cpython_types_slot_and_method_wrapper_types_subset`, covering CPython
  `TypesTests::test_slot_wrapper_types` and `::test_method_wrapper_types` for
  `object.__init__`, `object.__str__`, `object.__lt__`, `int.__lt__`, their
  bound method-wrapper forms, exact `types` alias identity, and direct callable
  behavior for the covered object and integer slots. Direct output parity is
  guarded by `cpython_types_slot_and_method_wrapper_types_diff_subset`.
  CPython object-layout internals, exact C descriptor types beyond public
  aliases, pickle identity matrices, and interpreter lifecycle behavior remain
  outside the sandbox `types` surface unless separately promoted with direct
  public-behavior evidence.
- The bundled `types` module also includes
  `cpython_types_code_traceback_type_aliases_subset`, covering the public
  `types.CodeType` and `types.TracebackType` aliases for MiniPython code and
  traceback runtime objects, including alias identity, `isinstance()`,
  `__class__`, and builtins type metadata.
- The bundled `types` module also includes
  `cpython_types_frame_type_alias_subset`, covering the public
  `types.FrameType` alias for real MiniPython frame objects returned by
  `sys._getframe()`, including alias identity, `isinstance()`, `__class__`,
  `f_back`, frame field access, object truthiness, hashability, and the absence
  of a namespace-style `__dict__`.
- The bundled `types` module also includes
  `cpython_types_runtime_type_aliases_subset`, covering public runtime aliases
  from CPython `Lib/test/test_types.py::TypesTests::test_names` that map to
  MiniPython objects: `LambdaType`, `GeneratorType`, `CoroutineType`,
  `AsyncGeneratorType`, `BuiltinFunctionType`, `BuiltinMethodType`, and
  `MethodType`, plus `types.__all__` membership for the exported names and
  the public `types.MethodType(function, instance)` constructor shape.
- `RUNTIME_BUILTINS` also includes CPython `BuiltinTest::test_zip_bad_iterable`,
  preserving the exact exception object raised by a failing `__iter__` through
  `zip()` and the sibling iterator constructors `iter()`, `enumerate()`,
  `map()`, and `filter()`.
- `RUNTIME_BUILTINS` also includes `cpython_builtin_iterator_pickle_subset`,
  covering CPython `BuiltinTest` filter/map/zip iterator pickle round trips,
  strict map/zip round trips, and strict-length failure preservation over
  MiniPython's internal pickle payload surface.
- The `BuiltinTest Iterator Builtins Method Audit` maps the current CPython
  filter/map/zip/iter methods to direct Rust evidence and explicitly classifies
  CPython's filter deallocation and zip GC-tracking regressions as
  implementation-internal rather than MiniPython portability requirements.
- The `BuiltinTest Attribute/Introspection Method Audit` maps `delattr`,
  `dir`, `getattr`, `hasattr`, `isinstance`, `issubclass`, `setattr`, `type`,
  and `vars` to direct Rust evidence, while keeping lone-surrogate
  attribute-name edges visible as partial runtime gaps.
- The `BuiltinTest Aggregate Builtins Method Audit` maps `max`, `min`, and
  `sum` to direct Rust evidence, including complex-constructor summation and
  complex signed-zero preservation, while classifying CPython's compensated
  `sum_accuracy` algorithm test as implementation-internal.
- `RUNTIME_BUILTINS` also includes `cpython_enumerate_reversed_pickle_subset`,
  covering CPython `test_enumerate.py` enumerate and reversed iterator pickle
  round trips, resumed already-advanced iterator pickles, empty enumerate, and
  ordinary plus large `start` values over MiniPython's internal pickle payload.
- The bundled `operator` module includes
  `cpython_operator_public_helpers_diff_subset`,
  `cpython_operator_index_normalization_diff_subset`, and
  `cpython_operator_public_helpers_subset`, covering CPython public helper
  parity for representative comparison, predicate, arithmetic, bitwise,
  sequence/member, `operator.index()` integer normalization including bool and
  int-subclass values, item mutation, `attrgetter`, `itemgetter`,
  `methodcaller`, and related callable-helper behavior. The direct
  `operator.index()` normalization diff is gated for CPython oracles with
  current int-subclass result normalization. Full pickle metadata and every CPython helper edge case
  remain outside the default sandbox `operator` surface until separately
  promoted with direct public-behavior evidence.
- The bundled `copy` module includes `cpython_copy_public_diff_subset` and
  `cpython_copy_public_subset`,
  covering CPython public `copy.Error` / `copy.error` exception metadata,
  `dispatch_table` module attribute shape, `copy.copy()` and
  `copy.deepcopy()` parity for supported pure-memory immutable scalar equality
  and identity, nested list/dict shallow-vs-deep copy behavior, `deepcopy()`
  memo preservation for shared list/dict/tuple members, explicit memo dict
  pre-seeding and population for supported identities, and self-referential
  lists, shared user-instance fields and user-instance self-cycles, and shared
  `UserList` / `UserDict` / `deque` members plus `UserList` self-cycles,
  `io.BytesIO` shallow/deep copy behavior for open in-memory buffers with
  position and custom attributes, independent bytearray copy buffers,
  dictionary copy independence, and representative arity/memo TypeError
  classification. Full pickle dispatch-table contents, pickle protocol byte compatibility,
  arbitrary extension-object copy hooks, and arbitrary mapping-protocol memo
  hooks remain outside the sandbox `copy` subset.
- The bundled `io.BytesIO` subset includes
  `cpython_io_bytesio_public_diff_subset` and
  `cpython_io_bytesio_public_subset`, covering CPython public in-memory
  construction from bytes-like objects, `read()`, `read1()`, `readline()`,
  `readlines()`, `write()` and `writelines()` over contiguous memoryview inputs
  plus non-contiguous memoryview `BufferError` rejection, `getvalue()`,
  `getbuffer()` writable memoryview mutation and active-export `BufferError` protection for
  `write()` / all `truncate()` requests / `close()` across direct and derived memoryviews,
  release of function-scoped, deleted-binding, and expression-temporary exported views,
  `readinto()` / `readinto1()` over
  writable bytearray and contiguous memoryview targets plus non-contiguous
  memoryview rejection with method-specific `TypeError` text, `tell()`, `seek()` with
  public `io.SEEK_SET` / `io.SEEK_CUR` / `io.SEEK_END` constants,
  `truncate()` including non-extending growth requests, position advancement,
  post-EOF empty reads/readinto results, sequential bytes-like line writes,
  sparse write NUL filling,
  size/sizehint line reads, line iteration through `__iter__()` / `__next__()`
  with EOF `StopIteration`, `readable()`, `writable()`, `seekable()`,
  `isatty()`, `flush()`, `fileno()` / `detach()` `io.UnsupportedOperation`
  stubs, type `__name__` / `__module__` / `__qualname__` / `__doc__`
  metadata, instance `__dict__` and custom attribute set/delete behavior,
  `close()`, `closed`, supported method visibility through `dir()`,
  `None` initial-value empty-buffer construction, closed-stream `ValueError`,
  context-manager entry/exit lifecycle behavior, and representative
  constructor/method TypeError/ValueError/OSError classification. Real files,
  buffering layers, text I/O, file descriptors, and OS-backed stream semantics
  remain outside the sandbox `io.BytesIO` subset.
- `RUNTIME_BUILTINS` also includes `cpython_operator_length_hint_subset`,
  covering CPython `test_operator.py::test_length_hint` fallback semantics and
  `test_enumerate.py::TestReversed::test_len` reversed iterator length hints,
  including TypeError default fallback, default-value normalization for bool
  and int-subclass inputs, hint-result normalization for bool and int-subclass
  returns, default and hint-result overflow rejection, non-TypeError
  propagation, finite `itertools.repeat()` remaining-length hints, and
  infinite-repeat direct `__length_hint__()` TypeError behavior. Direct CPython
  diff evidence is in `cpython_operator_length_hint_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_operator_comparison_predicate_subset` and
  `cpython_operator_is_none_predicates_subset`, covering CPython
  `test_operator.py` comparison and predicate helpers `lt/le/eq/ne/ge/gt`,
  `truth`, `not_`, identity helpers, and None predicates, including
  `operator.is_none` / `operator.is_not_none` plus custom rich
  comparison/truth exception propagation. Direct CPython diff evidence for the
  default-oracle stable slice is in
  `cpython_operator_comparison_predicate_diff_subset`; the newer `is_none` /
  `is_not_none` helpers have gated direct CPython evidence in
  `cpython_operator_is_none_predicates_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_operator_arithmetic_bitwise_subset`, covering CPython
  `test_operator.py` arithmetic and bitwise helpers `abs`, `add`, `sub`, `mul`,
  `floordiv`, `truediv`, `mod`, `pow`, `and_`, `or_`, `xor`, `lshift`,
  `rshift`, unary `neg`/`pos`/`invert` aliases, `matmul`, and `index`, including
  `NotImplemented` matrix-multiply reflected-method fallback plus
  representative TypeError/ValueError classification and CPython-style
  unsupported-operand text for unsupported `matmul` operands. Direct CPython
  diff evidence is in `cpython_operator_arithmetic_bitwise_diff_subset`.
- `RUNTIME_BUILTINS` also includes `cpython_operator_sequence_member_subset`,
  covering CPython `test_operator.py` sequence and member helpers `concat`,
  `countOf`, `indexOf`, `contains`, `getitem`, `setitem`, and `delitem`,
  including equality-based counting/search, iterator partial-consumption, and
  representative TypeError/ZeroDivisionError propagation. Direct CPython diff
  evidence is in `cpython_operator_sequence_member_diff_subset`.
- `RUNTIME_BUILTINS` also includes `cpython_operator_callable_helper_subset` and
  `cpython_operator_call_helper_subset`, covering CPython `test_operator.py`
  callable helpers `call`, `attrgetter`, `itemgetter`, and `methodcaller`,
  including dotted attribute traversal, `str` subclass attrgetter/methodcaller
  names, multi-result tuple packing, subscript forwarding, stored method
  args/keywords, many positional/keyword method arguments, callable forwarding,
  public `operator.call` metadata, and public exception propagation.
  Direct CPython diff evidence for the default-oracle
  stable getter/methodcaller slice is in
  `cpython_operator_callable_helper_diff_subset`; the newer `operator.call`
  helper has gated direct CPython evidence in
  `cpython_operator_call_helper_diff_subset`.
- `RUNTIME_BUILTINS` also includes `cpython_operator_inplace_helper_subset`,
  covering CPython `test_operator.py` in-place helper functions `iadd`, `isub`,
  `imul`, `imatmul`, `ifloordiv`, `itruediv`, `imod`, `ipow`, `ilshift`,
  `irshift`, `iand`, `ior`, `ixor`, and `iconcat`, including custom `__i*__`
  dispatch, numeric fallback behavior, list in-place mutation, and `iconcat`
  concat-type rejection. Direct CPython diff evidence is in
  `cpython_operator_inplace_helper_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_operator_module_metadata_subset`, covering CPython
  `test_operator.py::test___all__` and `::test_dunder_is_original` public
  module metadata: the exported `operator.__all__` names, `operator.*`
  callable `__module__` / `__name__` / `__qualname__` / `__doc__`
  introspection, helper instance `attrgetter` / `itemgetter` / `methodcaller`
  type names plus `__doc__` metadata and gated `__module__` metadata, and
  dunder aliases such as
  `__add__`, `__not__`, `__iconcat__`, and `__call__` preserving object
  identity with their public helpers. Direct CPython diff evidence for the
  default-oracle stable metadata slice is in
  `cpython_operator_module_metadata_diff_subset`; the newer `is_none` and
  `is_not_none` `operator.__all__` entries have gated direct CPython evidence in
  `cpython_operator_is_none_predicates_diff_subset`, and the newer
  `operator.call` entry has gated direct CPython evidence in
  `cpython_operator_call_helper_diff_subset`. Helper instance `__module__`
  metadata is covered locally by
  `cpython_operator_helper_instance_module_metadata_subset` and has gated direct
  CPython evidence in
  `cpython_operator_helper_instance_module_metadata_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_operator_signature_helper_subset`, covering CPython
  `test_operator.py` signature assertions for `operator.attrgetter`,
  `itemgetter`, and `methodcaller` constructors plus their helper instances via
  the public `str(inspect.signature(...))` surface, without claiming full
  `inspect.Signature` or broad callable signature introspection support. Direct
  CPython diff evidence is in `cpython_operator_signature_helper_diff_subset`,
  gated for CPython oracles where `inspect.signature(operator.attrgetter)` is
  supported.
- `RUNTIME_BUILTINS` also includes `cpython_operator_helper_repr_subset`,
  covering the public helper object repr/str shape exercised by CPython
  `test_operator.py::OperatorPickleTestCase` repr checks for `attrgetter`,
  `itemgetter`, and `methodcaller`, including dotted attributes, slice
  arguments, positional method args, and ordered keyword method args. Direct
  CPython diff evidence is in `cpython_operator_helper_repr_diff_subset`.
- `RUNTIME_BUILTINS` also includes `cpython_operator_pickle_helper_subset`,
  covering CPython `test_operator.py::OperatorPickleTestCase` public
  round-trip behavior for `operator.attrgetter`, `itemgetter`, and
  `methodcaller` helper objects across every exposed pickle protocol, including
  fresh restored helper identity and deep-copied stored methodcaller arguments
  over MiniPython's internal pickle payload surface.
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
  old-style string `%r` / `%a` calls user `__repr__`, propagates user
  exceptions, rejects non-string repr results, and applies CPython ASCII
  escaping/precision behavior; old-style string `%d` / `%i` / `%u` uses
  `__int__` before `__index__` while preserving float truncation, and
  `%x` / `%X` / `%o` uses `__index__` only, with propagated user exceptions
  and CPython public TypeError text for invalid integer conversion;
  old-style string `%f` / `%F` / `%e` / `%E` / `%g` / `%G` uses
  `__float__` before `__index__`, propagates user exceptions, and rejects
  invalid protocol results with CPython public TypeError text;
  old-style string mapping formats accept user `__getitem__` mapping objects
  and preserve catchable lookup exceptions;
  old-style string `%c` accepts `__index__` objects, propagates `__index__`
  exceptions, and preserves CPython public TypeError text for missing or
  invalid integer conversion;
  non-ASCII fill characters in
  `format()` alignment and `g` / `G` general
  floating-point formatting through `format()` and f-strings, plus float and
  complex-constructor precision formatting via `format()` for `f` / `F`,
  `e` / `E`, and `g` / `G` format codes, CPython
  `test_complex.py::test_format` coverage for CPython's complete deterministic
  complex formatting matrix: empty and omitted presentation types,
  alignment/fill/width, sign handling, precision, alternate-form formatting,
  comma grouping, large finite values, invalid zero-padding / `=` alignment /
  integer presentation errors, `complex.__format__()`, `str.format()`
  forwarding, and NaN/Inf casing for `f` / `F`, CPython
  `test_complex.py::test_constructor_from_string` coverage for real,
  imaginary, parenthesized, signed-unit-imaginary, Unicode-whitespace,
  underflow signed-zero, long, and malformed complex strings,
  `test_complex.py::test_constructor` coverage for exact built-in complex
  construction from real and imaginary arguments, keyword arguments,
  signed-zero preservation, `__float__` / `__index__` protocol conversion,
  `__complex__` / `__float__` / `__index__` rejection paths, custom exception
  propagation from `__complex__`, and representative public TypeError
  diagnostics, plus CPython's two-argument compatibility distinction where
  `real` may use `__complex__` providers but `imag` rejects arbitrary
  `__complex__` providers while still accepting actual complex and
  complex-subclass values, including the associated `DeprecationWarning`
  messages for two-argument complex compatibility, keyword-only `real`, and
  strict-complex-subclass `__complex__` results, plus exact complex object
  identity for `complex(c)`, `complex.__new__(complex, c)`, `c.__complex__()`,
  unary plus, and the non-identity keyword-only `complex(real=c)` path,
  complex subclass construction and `from_number()` rows through
  `cpython_complex_subclass_constructor_and_from_number_subset`, including
  native complex storage, `complex.__new__`, custom subclass `__new__`,
  strict-complex-subclass `__complex__` result normalization, inherited complex
  methods, custom complex-subclass `__complex__` constructor dispatch,
  truthiness, arithmetic, equality, hashing, `real` / `imag` attributes, and
  special-number subclass construction with signed zero, infinity, and NaN
  preservation,
  `test_complex.py::test_boolcontext`,
  `::test_constructor_special_numbers`,
  `::test_constructor_negative_nans_from_string`, `::test_underscores`,
  `::test_plus_minus_0j`, `::test_negated_imaginary_literal`, and
  `::test_repr_roundtrip` coverage for truthiness, signed zero, infinity, NaN
  sign preservation through `math.copysign()`, valid and invalid complex-string
  underscore placement across CPython's full filtered shared literal matrix,
  negated imaginary literal components, and `repr()`/`eval(repr(...))`
  reconstruction,
  `test_complex.py::test_conjugate`, `::test___complex__`,
  `::test_getnewargs`, `::test_from_number`, including
  `complex.from_number(cNAN) is cNAN` while preserving complex NaN
  self-inequality, `::test_float_containment`,
  `::test_float_floor`, `::test_float_ceil`, `::test_float_mod`, `::test_add`, `::test_sub`,
  `::test_mul`, including non-finite complex-by-complex multiplication recovery
  for infinities, NaNs, and overflowed intermediate products, `::test_truediv`,
  `::test_truediv_zero_division`, including non-finite quotient recovery and
  signed-zero results for complex/complex, complex/real, and real/complex
  division plus CPython's huge/tiny inverse checks, `::test_floordiv`,
  `::test_mod`, `::test_divmod`, including
  zero-denominator TypeError behavior for those unsupported operators,
  `::test_pow`, including zero exponent, self-comparison stress, complex
  exponentiation overflow, and boundary no-crash rows,
  `::test_pow_with_small_integer_exponents`, `::test_hash`,
  `::test_richcompare`, `::test_richcompare_boundaries`, `::test_abs`,
  `::test_repr_str`,
  `::test_negative_zero_repr_str`, `::test_pos`, `::test_neg`, and
  `::test_overflow` coverage for public complex methods, hash invariants,
  numeric conversion, construction, addition/subtraction/multiplication/division/power basics,
  small integer exponent type parity, unsupported floor/mod/divmod operations,
  complex modulo rejection, exact integer comparison boundaries, magnitude overflow, special-value
  representation, and unary operators, and
  CPython-style invalid format-specifier `ValueError` messages for `format()`,
  f-strings, and `str.format()`. It also
  covers CPython's `z` negative-zero coercion option for float and complex
  f-string formatting, including `%` percentage presentation, fill-character
  ordering, tiny negative values that round to zero, post-rounding sign
  preservation for values such as `-.09`, and invalid `z` specifier
  positions/types.
- `STRING_RUNTIME` also includes `cpython_bytes_percent_format_subset` and
  `cpython_bytes_percent_format_dunder_bytes_errors_subset`,
  covering CPython `BaseBytesTest::test_mod` and `::test_imod` public
  bytes/bytearray old-style `%` formatting for `%b`, `%s`, `%d`, `%i`, `%u`,
  `%x`, `%X`, `%o`, `%c`, `%f`, `%F`, `%e`, `%E`, `%g`, `%G`,
  literal percent escapes, NUL-containing format
  strings, bytes mapping keys, dynamic `*` width/precision, receiver-driven
  result types, `%d` / `%i` / `%u` `__int__` / `__index__` conversion,
  `%x` / `%X` / `%o` and `%c` `__index__` conversion, float conversions through
  direct numbers plus successful `__float__` / `__index__` protocols,
  user `__getitem__` mapping objects with bytes keys and propagated lookup
  exceptions,
  memoryview input for `%b` / `%s` while `%c` rejects memoryview, and
  representative catchable error classes including failed-protocol non-real
  float argument TypeErrors, plus `__bytes__` dispatch for `%b` / `%s`,
  non-bytes `__bytes__` result rejection, propagated `__bytes__` exceptions,
  non-ASCII `%r` / `%a` repr escaping, and CPython public error ordering for
  mapping keys with dynamic width or ordinary placeholders, including missing
  bytes mapping keys preserving `KeyError.args`. `STRING_RUNTIME` also includes
  `cpython_bytes_percent_dunder_and_reentrant_bytearray_subset`, covering direct
  `__mod__` descriptor calls, inherited subclass dispatch, and CPython
  `ByteArrayTest::test_mod_concurrent_mutation` public safety behavior where a
  bytearray format string stays resize-locked while `%a` invokes user
  `__repr__`.
- `STRING_RUNTIME` also includes `cpython_bytes_rmod_subset`, covering CPython
  `BaseBytesTest::test_rmod` public reflected modulo behavior: unsupported
  left operands raise catchable `TypeError`, and bytes/bytearray `__rmod__`
  returns `NotImplemented` for non-bytes formatting operands.
- `STRING_RUNTIME` also includes
  `cpython_bytes_empty_sequence_index_subset`, covering CPython
  `BaseBytesTest::test_empty_sequence` public empty bytes/bytearray indexing
  behavior for ordinary, `sys.maxsize`-sized, and arbitrary large
  positive/negative subscript indices.
- `STRING_RUNTIME` also includes
  `cpython_bytes_length_constructor_boundary_subset` and
  `cpython_bytes_constructor_overflow_guard_subset`, covering CPython
  `BaseBytesTest::test_from_int`, `test_from_ssize`, and
  `test_constructor_overflow` public bytes/bytearray constructor behavior for
  zero-filled integer lengths, string/buffer sources, `__index__` length
  conversion, catchable negative/overflow boundary exceptions, and
  address-space-sized allocation guards that raise `OverflowError` or
  `MemoryError` without crashing. Direct CPython diff evidence for the stable
  length and signed-size boundary portion is in
  `cpython_bytes_length_constructor_boundary_diff_subset`; the
  address-space-sized allocation guard remains local safety evidence because
  exact `MemoryError` / `OverflowError` classification can vary by runtime
  allocation policy.
- `STRING_RUNTIME` also includes `cpython_bytes_iterable_constructor_subset`,
  covering CPython `BaseBytesTest::test_from_iterable`, `test_from_tuple`,
  `test_from_list`, and `test_from_index` public bytes/bytearray construction
  from integer iterables including `range`, `iter(range(...))`, set inputs,
  generators without `__length_hint__`, list/tuple fast paths, `__getitem__`
  sequences, `__index__` item conversion/error paths, and the public
  `test_constructor_type_errors` / `test_constructor_value_errors` error class
  matrix for source, encoding/errors, and out-of-byte-range iterable items.
- `STRING_RUNTIME` also includes `cpython_bytes_hex_fromhex_subset`,
  `cpython_bytes_hex_separator_boundaries_subset`, and
  `cpython_bytes_hex_descriptor_error_messages_subset`, covering CPython
  `BaseBytesTest::test_fromhex`, `test_hex`, `test_hex_separator_basics`,
  `test_hex_separator_five_bytes`, `test_hex_separator_six_bytes`, and current
  CPython main `test_hex_simd_boundaries` / `test_hex_nibble_boundaries` public
  behavior for string and bytes-like `fromhex()` inputs including `memoryview`
  and `array.array('B')`, ASCII whitespace skipping including vertical tab,
  non-ASCII rejection, exact odd-hex-digit and invalid-position diagnostics,
  bytes/bytearray `hex()` separator grouping, separator-byte boundaries,
  `bytes_per_sep` `__index__` conversion, catchable C-int overflow errors, and
  public `hex()` output correctness across length and nibble boundary samples,
  plus exact unbound and invalid-receiver `hex()` descriptor diagnostics.
- `STRING_RUNTIME` also includes `cpython_bytes_search_methods_subset`,
  covering CPython `BaseBytesTest` public `count()`, `find()`, `rfind()`,
  `index()`, and `rindex()` behavior for bytes-like and integer byte needles,
  the search/count side of `test_none_arguments` with `None` start/stop bounds,
  missing-value results/errors, and `test_integer_arguments_out_of_byte_range`
  `ValueError` rejection for both bytes and bytearray. Direct CPython diff
  evidence for this search surface plus bytes/bytearray compare, reversed, and
  slice behavior is in `cpython_bytes_search_compare_slice_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_search_prefix_suffix_error_messages_subset`, covering CPython
  `BaseBytesTest::test_find_etc_raise_correct_error_messages` public
  Argument-Clinic-shaped over-arity `TypeError` text for bytes/bytearray search
  and prefix/suffix methods.
- `STRING_RUNTIME` also includes
  `cpython_bytes_prefix_suffix_typeerror_messages_subset`, covering CPython
  `BaseBytesTest` public `startswith()` / `endswith()` TypeError diagnostics
  where top-level invalid prefixes/suffixes use the first-argument message but
  invalid tuple candidates use the generic bytes-like-object message. Direct
  CPython diff evidence for the exact TypeError text is in
  `cpython_bytes_prefix_suffix_typeerror_messages_diff_subset`; broader
  prefix/suffix success, tuple, `None` bound, and empty tuple behavior remains
  covered by `cpython_bytes_prefix_suffix_methods_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_replace_partition_methods_subset`,
  covering CPython `BaseBytesTest::test_replace`, current CPython main
  `test_replace_count_keyword`, `test_replace_int_error`, `test_partition`,
  `test_rpartition`, `test_partition_string_error`, and
  `test_partition_int_error` for bytes/bytearray result types, positional and
  keyword `count`, bytes-like arguments, partition tuple shapes, empty
  separators, and representative public TypeError/ValueError paths.
- `STRING_RUNTIME` also includes
  `cpython_bytes_method_typeerror_messages_subset`, covering CPython
  `BaseBytesTest` public TypeError-message diagnostics for bytes/bytearray
  `split()` / `rsplit()`, `partition()` / `rpartition()`, `strip()` /
  `lstrip()` / `rstrip()`, and `center()` / `ljust()` / `rjust()` fill
  argument validation. Direct CPython diff evidence for the stable text rows is
  in `cpython_bytes_method_typeerror_messages_diff_subset`; current fill-length
  wording is covered by the CPython-version-gated
  `cpython_bytes_fill_length_typeerror_messages_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_more_method_typeerror_messages_subset`, covering CPython
  `BaseBytesTest` public TypeError-message diagnostics for bytes/bytearray
  ASCII case and predicate methods, `splitlines()`, `expandtabs()`, `zfill()`,
  `removeprefix()`, and `removesuffix()` unbound, arity, and integer-converter
  edge calls. Direct CPython diff evidence for stable rows is in
  `cpython_bytes_more_method_typeerror_messages_diff_subset`; current
  `expandtabs()` converter wording is covered by the CPython-version-gated
  `cpython_bytes_expandtabs_typeerror_message_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_core_method_typeerror_messages_subset`, covering CPython
  `BaseBytesTest` public TypeError-message diagnostics for bytes/bytearray
  `split()` / `rsplit()`, search, prefix/suffix, strip, alignment,
  partition, and replace unbound, arity, bound-index, width, maxsplit, and
  count-conversion edge calls. Direct CPython diff evidence for stable rows is
  in `cpython_bytes_core_method_typeerror_messages_diff_subset`; current
  search/prefix missing-argument wording is covered by the CPython-version-gated
  `cpython_bytes_search_missing_typeerror_messages_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_alignment_methods_subset`,
  covering bytes/bytearray `center()`, `ljust()`, and `rjust()` width and
  fill-byte behavior, unchanged-width cases, receiver-driven result types, and
  representative TypeError paths. Direct CPython diff evidence is in
  `cpython_bytes_alignment_methods_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_strip_methods_subset`,
  covering bytes/bytearray `strip()`, `lstrip()`, and `rstrip()` over default
  ASCII whitespace, explicit bytes-like strip sets including `memoryview` and
  `bytearray`, `None`, empty strip sets, receiver-driven result types, and
  representative TypeError paths. Direct CPython diff evidence is in
  `cpython_bytes_strip_methods_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_join_translate_maketrans_typeerror_messages_subset`, covering
  CPython `BaseBytesTest` public exact TypeError diagnostics for bytes and
  bytearray `join()`, `translate()`, and `maketrans()` unbound, missing
  argument, over-arity, non-iterable, and no-argument calls. Direct CPython
  diff evidence is in
  `cpython_bytes_join_translate_maketrans_typeerror_messages_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_replace_partition_methods_subset`, covering bytes/bytearray
  `replace()`, `partition()`, and `rpartition()` result-type behavior,
  bytes-like arguments, positional and keyword replacement counts, empty-needle
  replacement, empty separators, and representative TypeError/ValueError
  paths. Direct CPython diff evidence for the portable public replace and
  partition surface is in `cpython_bytes_replace_partition_methods_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_split_rsplit_methods_subset`,
  covering bytes/bytearray `split()` and `rsplit()` over default ASCII
  whitespace, explicit bytes-like separators, `maxsplit`, keyword arguments,
  receiver-driven result types, Unicode-whitespace boundary behavior, empty
  separators, and representative TypeError paths. Direct CPython diff evidence
  is in `cpython_bytes_split_rsplit_methods_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_splitlines_methods_subset`,
  covering bytes/bytearray `splitlines()` over CR, LF, and CRLF boundaries,
  `keepends`, receiver-driven result types, bytes-specific non-splitting
  behavior for Unicode text line separators, and representative TypeError
  paths. Direct CPython diff evidence is in
  `cpython_bytes_splitlines_methods_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_ascii_case_predicate_methods_subset`, covering the CPython
  `string_tests.py` ASCII case and predicate methods as applied to
  bytes/bytearray through `test_bytes.py`, including ASCII-only case
  transforms, predicates, non-ASCII byte preservation, empty-input predicate
  behavior, and representative TypeError paths. Direct CPython diff evidence
  is in `cpython_bytes_ascii_case_predicate_methods_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_expandtabs_zfill_methods_subset`, covering CPython
  `string_tests.py` `expandtabs()` and `zfill()` behavior as applied to
  bytes/bytearray through `test_bytes.py`, plus builtin type `dir()`
  visibility for those methods. Direct CPython diff evidence is in
  `cpython_bytes_expandtabs_zfill_methods_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_search_bounds_index_subset`,
  covering Python-level `__index__` conversion and exception propagation for
  bytes/bytearray search and prefix/suffix `start` / `stop` bounds. Direct
  CPython diff evidence for the same bound-conversion and propagated-exception
  surface is in `cpython_bytes_search_bounds_index_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_remove_affix_methods_subset`,
  covering bytes/bytearray `removeprefix()` and `removesuffix()` over
  bytes-like affixes, empty receiver and empty affix behavior, receiver-driven
  result types, and representative TypeError paths. Direct CPython diff
  evidence is in `cpython_bytes_remove_affix_methods_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_maketrans_translate_subset`,
  covering bytes/bytearray `maketrans()` and `translate()` 256-byte tables,
  bytes-like table/delete arguments including `memoryview`, `None` identity
  tables, deletion with positional and keyword arguments, receiver-driven
  result types, and representative TypeError/ValueError paths. Direct CPython
  diff evidence is in `cpython_bytes_maketrans_translate_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytearray_mutation_methods_subset`,
  covering bytearray `append()`, `extend()`, `insert()`, `pop()`, `remove()`,
  `reverse()`, `clear()`, and `copy()` in-place mutation behavior, including
  bytes-like and integer-iterable extension, index normalization, copy
  identity, and representative public error classes. Direct CPython diff
  evidence is in `cpython_bytearray_mutation_methods_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytearray_extend_subset`, covering
  CPython `ByteArrayTest::test_extend` behavior for self-extension, map and
  generator inputs, all-or-nothing invalid item handling, `__index__` item
  conversion, and bytearray-specific `TypeError` messages. Direct CPython diff
  evidence for the stable behavior and error classes is in
  `cpython_bytearray_extend_diff_subset`; direct text evidence is in
  `cpython_bytearray_extend_typeerror_message_diff_subset` when the selected
  CPython oracle exposes the current string-source wording.
- `STRING_RUNTIME` also includes `cpython_bytearray_resize_subset`, covering
  current CPython `ByteArrayTest::test_resize` public behavior for bytearray
  truncation, zero-filled growth, `__index__` length conversion, catchable
  arity/type/negative-length errors, `dir(bytearray)` visibility, and
  sandbox-safe `MemoryError` behavior for impractically large sizes. Direct
  CPython diff evidence for the small-size public behavior and catchable error
  classes is in the CPython-version-gated
  `cpython_bytearray_resize_diff_subset`; the impractically large allocation
  guard remains local sandbox subset evidence.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_alloc_and_subclass_mutation_subset`, covering CPython
  `ByteArrayTest::test_alloc` and `test_init_alloc` public `__alloc__()`
  behavior, including empty allocation value, allocation-greater-than-length
  semantics, generator-driven `bytearray.__init__()` intermediate mutation, and
  inherited bytearray mutation methods on subclasses without copying CPython's
  exact allocator growth policy. Direct CPython diff evidence for the stable
  allocation/subclass mutation slice is in
  `cpython_bytearray_alloc_and_subclass_mutation_diff_subset`; inherited
  subclass `resize()` behavior has gated direct diff evidence in
  `cpython_bytearray_resize_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytearray_resize_forbidden_subset`,
  covering current CPython `ByteArrayTest::test_resize_forbidden` public
  behavior for active memoryview exports blocking bytearray resizing through
  `resize()`, slice assignment, `pop()`, `remove()`, and deletion while
  preserving the original bytes. Direct CPython diff evidence for the
  sandbox-safe public behavior is in the CPython-version-gated
  `cpython_bytearray_resize_forbidden_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytearray_take_bytes_subset`, covering
  current CPython `ByteArrayTest::test_take_bytes` public behavior for
  take-and-delete bytearray prefix extraction, negative stop normalization,
  `None` stop, `__index__` stop conversion, active memoryview exporter
  `BufferError`, public error classes, inherited `bytearray` subclass method
  dispatch, and `dir(bytearray)` visibility without exposing the method on
  `bytes`. This remains local subset evidence because the default CPython
  oracle used by `cpython_diff` in this workspace does not expose
  `bytearray.take_bytes()`.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_iterator_length_hint_and_repeat_diff_subset`,
  `cpython_bytearray_exhausted_iterator_diff_subset`,
  `cpython_bytearray_iterator_length_hint_and_repeat_regressions_subset`
  and `cpython_bytearray_exhausted_iterator_subset`,
  covering current CPython `ByteArrayTest::test_iterator_length_hint` and
  `test_repeat_after_setslice` behavior for bytearray iterator exhaustion after
  clearing the original bytearray, exhausted iterators staying exhausted after
  exporter growth while sibling iterators can observe appended bytes, and
  repetition after resizing slice assignment.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_mutating_index_conversion_diff_subset` and
  `cpython_bytearray_mutating_index_safety_diff_subset`, covering current
  CPython `ByteArrayTest::test_mutating_index` and
  `test_mutating_index_inbounds` Python-level behavior for `__index__`
  conversion plus reentrancy during bytearray item and slice assignment. The
  safety diff is capability-gated for CPython oracles with the fixed public
  crash-regression behavior.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_search_reentrancy_buffererror_subset`, covering current
  CPython `ByteArrayTest::test_search_methods_reentrancy_raises_buffererror`
  public behavior for bytearray search, membership, `split()`, and `rsplit()`
  when current public `__buffer__` protocol argument conversion re-enters and
  attempts to resize the locked receiver bytearray. Direct CPython diff
  evidence is in the CPython-version-gated
  `cpython_bytearray_search_reentrancy_buffererror_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_extend_empty_buffer_overflow_subset`, covering current
  CPython `ByteArrayTest::test_extend_empty_buffer_overflow` public behavior for
  `bytearray.extend()` over zero-length-hint iterators and catchable
  `float(bytearray())` `ValueError` parsing failures. Capability-gated direct
  CPython diff evidence is in
  `cpython_bytearray_extend_empty_buffer_overflow_diff_subset` when the selected
  oracle has the fixed public behavior; older system CPython oracles can still
  exhibit the historical corrupted-bytearray behavior that CPython's regression test prevents.
- `STRING_RUNTIME` also includes `cpython_bytearray_regexps_subset`, covering
  CPython `ByteArrayTest::test_regexps` public behavior for the supported
  `re.findall()` bytes-pattern subset: ASCII `\w+` over bytes-like subjects
  returns ordinary bytes matches. Direct CPython diff evidence is in
  `cpython_bytearray_regexps_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_basics_and_ord_subset`, covering CPython
  `BaseBytesTest::test_basics` and `test_ord` public behavior for exact
  bytes/bytearray type and `__class__` identity plus `ord()` over one-byte
  slices at representative byte boundaries `[0, 65, 127, 128, 255]`. Direct
  CPython diff evidence is in
  `cpython_bytes_basics_and_empty_index_diff_subset`, which also covers empty
  sequence index-miss normalization.
- `STRING_RUNTIME` also includes
  `cpython_bytes_iterable_constructor_subset` and
  `cpython_bytes_constructor_exception_subset`, covering bytes/bytearray
  construction from integer iterables, `__getitem__` sequences, sets,
  generators, `__index__` elements, constructor argument error classes, and
  propagation of exceptions raised by `__index__` / `__iter__`. Direct CPython
  diff evidence is in `cpython_bytes_iterable_constructor_diff_subset` and
  `cpython_bytes_constructor_exception_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_mutating_list_constructor_subset`, covering CPython
  `BaseBytesTest::test_from_mutating_list` public behavior where bytes and
  bytearray constructors consume live list iterators that observe source list
  clears and appends during item `__index__` conversion. Direct CPython diff
  evidence is in `cpython_bytes_mutating_list_constructor_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_constructor_concat_repeat_contains_subset`, covering
  bytes/bytearray integer-length construction, mixed bytes/bytearray
  concatenation result types, repetition with zero/negative counts, repeat
  TypeErrors, and membership over integer and bytes-like needles. Direct
  CPython diff evidence is in
  `cpython_bytes_constructor_concat_repeat_contains_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytes_check_encoding_errors_devmode_subset`, covering CPython
  `BaseBytesTest::test_check_encoding_errors` public `-X dev` behavior where
  bytes/bytearray constructors and decode methods eagerly raise `LookupError`
  for invalid codec `encoding` / `errors` names. CPython default-mode lazy
  validation of invalid `errors` names remains a documented mode difference.
- `STRING_RUNTIME` also includes `cpython_string_bytes_codec_diff_subset` / `cpython_string_bytes_codec_subset`, covering
  CPython `Lib/test/test_str.py::test_codecs` plus the public bytes/bytearray
  constructor and decode slices from `BaseBytesTest::test_encoding` and
  `test_decode`: UTF-8/UTF-16 text constructors, Latin-1 strict encode failure
  and `ignore`, UTF-8 strict decode failure and `ignore` through positional and
  keyword arguments, direct `bytearray.decode()` success/error paths, and default
  UTF-8 decode.
- `STRING_RUNTIME` also includes `cpython_bytes_iterable_constructor_subset`,
  covering CPython `BaseBytesTest::test_from_iterable`, `test_from_tuple`,
  `test_from_list`, and `test_from_index` public bytes/bytearray construction
  from `range`, range iterators, set inputs, generators without
  `__length_hint__`, list/tuple inputs, `__getitem__` sequences, and
  valid/erroring `__index__` item conversion.
- `STRING_RUNTIME` also includes `cpython_bytes_buffer_constructor_diff_subset`,
  `cpython_bytes_buffer_constructor_subset`, and
  `cpython_bytes_array_array_buffer_subset`, covering the portable public part
  of CPython `BaseBytesTest::test_from_buffer` where bytes and bytearray
  constructors accept bytes, bytearray, memoryview, first-pass
  `array.array('B')`, and bytes-subclass sources, including fallback to
  bytes-like construction when a bytes subclass `__index__` raises `TypeError`.
  The array-backed buffer slice also covers `fromhex()`, search/replace,
  concat, membership, bytearray in-place concat and slice assignment, while
  preserving CPython's distinction that `bytes == array.array('B', ...)` is
  false and ordered comparison is unsupported. Non-`B` array formats and full
  buffer-protocol matrices remain in the broader buffer gap.
- `STRING_RUNTIME` also includes
  `cpython_bytes_bytearray_subclass_basics_subset` and
  `cpython_bytes_bytearray_subclass_ops_and_join_subset` plus
  `cpython_bytes_bytearray_subclass_fromhex_subset`,
  `cpython_bytearray_subclass_init_override_subset`,
  `cpython_bytes_bytearray_subclass_copy_subset`,
  `cpython_bytes_bytearray_subclass_pickle_subset`, and
  `cpython_bytes_dunder_bytes_and_blocking_subset`, covering public
  bytes/bytearray subclass construction, `SubclassTest::test_basic` /
  `test_join` comparison, concatenation, repetition, inherited method
  visibility, base-type `join()` result rules, `SubclassTest::test_fromhex`
  subclass-preserving `fromhex()` classmethods, bytes/bytearray `__new__`, and
  bytearray `__init__`, `ByteArraySubclassTest::test_init_override` custom
  bytearray subclass initializer binding, `SubclassTest::test_copy`
  shallow/deep copy behavior preserving concrete subclass type, value equality,
  user attributes, and nested subclass attribute values,
  `SubclassTest::test_pickle` public pickle round-trip behavior over the same
  subclass value/attribute surface through MiniPython's internal payload, plus
  `bytes()` `__bytes__` dispatch,
  bytes-subclass result preservation, non-bytes result rejection, `__bytes__`
  precedence over `__index__`, str-subclass `__bytes__` / explicit-encoding
  precedence from `BytesTest::test_custom`, bytes-subclass `__bytes__`
  regression cases, `str.__new__` returning concrete str subclasses, and
  `__bytes__ = None` fallback blocking. Direct CPython diff evidence for the
  stable `bytes(obj)` dispatch/blocking surface is in
  `cpython_bytes_dunder_bytes_dispatch_diff_subset`.
  Direct CPython diff evidence for the bytes/bytearray subclass construction
  basics slice is in `cpython_bytes_bytearray_subclass_basics_diff_subset`.
  Direct CPython diff evidence for bytes/bytearray subclass operations and
  base-type `join()` result behavior is in
  `cpython_bytes_bytearray_subclass_ops_and_join_diff_subset`.
  Direct CPython diff evidence for subclass-preserving `fromhex()` and the
  public `bytes.__new__` / `bytearray.__new__` / `bytearray.__init__`
  descriptor surface is in
  `cpython_bytes_bytearray_subclass_fromhex_diff_subset`.
  Direct CPython diff evidence for bytearray subclass initializer override
  binding is in `cpython_bytearray_subclass_init_override_diff_subset`.
  Direct CPython diff evidence for bytes/bytearray subclass shallow/deep copy
  behavior is in `cpython_bytes_bytearray_subclass_copy_diff_subset`.
  Direct CPython diff evidence for bytes/bytearray subclass pickle public
  round-trip behavior is in `cpython_bytes_bytearray_subclass_pickle_diff_subset`;
  this does not claim CPython binary pickle byte-stream compatibility.
  `cpython_bytes_dunder_bytes_method_subset` covers direct
  `BytesTest::test__bytes__` method calls on exact bytes and bytes subclasses,
  exact bytes result type, exact bytes self identity, bytes-subclass copy-out
  identity, inherited descriptor calls, and `dir()` visibility. Direct CPython
  diff evidence is in `cpython_bytes_dunder_bytes_method_diff_subset`, gated for
  CPython oracles that expose direct `bytes.__bytes__` method calls.
  `cpython_bytes_repeat_id_preserving_subset` covers CPython
  `BytesTest::test_repeat_id_preserving` for exact bytes repeat-by-one object
  identity in both operand orders, empty bytes singleton identity, and distinct
  exact bytes results for zero/negative/two repeats and bytes-subclass repeat
  operations. Direct CPython diff evidence is in
  `cpython_bytes_repeat_id_preserving_diff_subset`.
  `cpython_memoryview_bytesio_readinto_subset` covers the public read-only
  target rejection from CPython `BytesTest::test_buffer_is_readonly` through
  in-memory `io.BytesIO.readinto()`; the exact host raw-file fixture remains a
  runtime/filesystem-policy gap.
  `cpython_bytes_bytearray_index_error_and_hash_subset` covers CPython
  `BytesTest::test_getitem_error` and `ByteArrayTest::test_getitem_error`,
  `test_setitem_error`, and `test_nohash` public error paths for invalid
  bytes/bytearray indices and bytearray hashing. Direct CPython diff evidence
  is in `cpython_bytes_bytearray_index_error_and_hash_diff_subset`.
  `cpython_bytes_format_method_subset` covers CPython
  `AssortedBytesTest::test_format` public bytes/bytearray formatting behavior:
  omitted/empty specs render through `str()`, non-empty specs raise catchable
  `TypeError`, and explicit `!s` conversion remains string-formattable. Direct
  CPython diff evidence is in `cpython_bytes_format_method_diff_subset`.
  `cpython_bytes_bytearray_type_doc_subset` covers CPython
  `AssortedBytesTest::test_doc` public type docstrings for bytes and
  bytearray, including constructor-signature prefixes and `dir()` visibility.
  Direct CPython diff evidence is in
  `cpython_bytes_bytearray_type_doc_diff_subset`.
  `cpython_bytes_bytearray_subclass_repr_and_compare_subset` covers repr/str
  rendering, bytes-like equality against builtin bytes, bytearray, and
  memoryview values, bytewise ordering for supported bytes-like values. Direct
  CPython diff evidence is in
  `cpython_bytes_bytearray_subclass_repr_and_compare_diff_subset`.
  `cpython_bytes_bytearray_assorted_public_subset` covers CPython
  `AssortedBytesTest::test_from_bytearray` and
  `test_compare_bytes_to_bytearray` public behavior for bytearray construction
  from a memoryview-backed bytes object plus both operand orders for
  bytes/bytearray rich comparison. Direct CPython diff evidence is in
  `cpython_bytes_bytearray_assorted_public_diff_subset`.
  `cpython_bytes_warning_compare_diff_subset` /
  `cpython_bytes_warning_compare_subset` covers
  `AssortedBytesTest::test_compare` for `sys.flags.bytes_warning`,
  `BytesWarning` capture, and `-bb` warning-as-error behavior. This is followed by
  `cpython_bytearray_hex_reentrant_separator_buffererror_diff_subset` /
  `cpython_bytearray_hex_reentrant_separator_buffererror_subset`, covering the
  current CPython `ByteArrayTest::test_hex_use_after_free` behavior where
  bytearray `hex()` keeps the receiver resize-locked while a bytes-subclass
  separator executes re-entrant `__len__` code. The direct diff is capability
  gated because older CPython oracles still expose the old accepted-and-cleared
  behavior.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_extended_slice_assignment_subset`, covering bytearray
  extended slice assignment/deletion, integer-iterable RHS conversion,
  self-slice assignment, special method dispatch, and saturated large slice
  bounds. Direct CPython diff evidence is in
  `cpython_bytearray_extended_slice_assignment_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_inplace_concat_repeat_subset`, covering bytearray `+=`,
  `*=`, `__iadd__`, and `__imul__` alias-preserving in-place mutation,
  bytes-like concat operands, repeat counts, and representative catchable
  `TypeError` paths. Direct CPython diff evidence is in
  `cpython_bytearray_inplace_concat_repeat_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_nonmutating_methods_copy_buffers_subset` and
  `cpython_bytearray_pep3137_returns_new_copy_subset`, covering CPython
  `ByteArrayTest::test_copied`,
  `test_partition_bytearray_doesnt_share_nullstring`, and
  `BytearrayPEP3137Test::test_returns_new_copy` semantics, plus the public
  behavior of `AssortedBytesTest::test_return_self`, for independent bytearray
  objects returned by non-mutating operations, absent-separator
  partition/rpartition results, and no-op mutable bytearray methods including
  `zfill()`, `rjust()`, `ljust()`, `center()`, `split()`, `rsplit()`,
  `splitlines()`, `replace(b'', b'')`, and one-item `join()`. Direct CPython
  diff evidence for the non-mutating copy-buffer slice is in
  `cpython_bytearray_nonmutating_copy_buffers_diff_subset`; direct diff
  evidence for the return-copy slice is in
  `cpython_bytearray_pep3137_returns_new_copy_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_join_custom_iterator_diff_subset` and
  `cpython_bytearray_join_reentrant_resize_subset`, covering CPython
  `BuiltinTest::test_bytearray_join_with_custom_iterator` and
  `::test_bytearray_join_with_misbehaving_iterator`: bytearray `join()` accepts
  custom iterators and keeps the separator resize-locked while consuming the
  iterable so re-entrant separator resizing raises `BufferError`. The
  re-entrant resize check remains local runtime evidence because it differs
  across default system CPython versions.
- `STRING_RUNTIME` also includes `cpython_bytes_join_subset`, covering
  bytes/bytearray `join()` receiver-driven result types, list/tuple/iterator
  inputs, bytes-like items, empty joins/separators, reduced stress joins, and
  representative TypeError paths. It also pins CPython's distinction between
  contiguous sliced `memoryview` items, which are accepted, and non-contiguous
  `memoryview` items, which raise `TypeError` with `memoryview found`. Direct
  CPython diff evidence is in `cpython_bytes_join_diff_subset`.
- `STRING_RUNTIME` also includes
  `cpython_builtin_bytearray_translate_extend_errors_subset`, covering CPython
  `BuiltinTest::test_bytearray_translate` and
  `::test_bytearray_extend_error`: short translation tables raise
  `ValueError`, non-bytes-like delete arguments raise `TypeError` once the
  table is valid, and exceptions from `map(int, ...)` propagate out of
  `bytearray.extend()` without mutating the target array. Direct CPython diff
  evidence is in
  `cpython_builtin_bytearray_translate_extend_errors_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_copy_module_subset`, covering
  CPython `BaseBytesTest::test_copy` for bytes/bytearray through the supported
  `copy.copy()` / `copy.deepcopy()` module surface, including independent
  bytearray buffers for shallow and deep copies. Direct CPython diff evidence
  is in `cpython_bytes_copy_module_diff_subset`.
- `STRING_RUNTIME` also includes `cpython_bytes_pickle_roundtrip_diff_subset` /
  `cpython_bytes_pickle_roundtrip_subset`,
  covering CPython `BaseBytesTest::test_pickling` public value/type
  round-trips for supported bytes and bytearray values through MiniPython's
  current internal pickle payload surface.
- `STRING_RUNTIME` also includes
  `cpython_bytes_iterator_pickle_roundtrip_diff_subset` /
  `cpython_bytes_iterator_pickle_roundtrip_subset`, covering CPython
  `BaseBytesTest::test_iterator_pickling` public iterator value/type
  round-trips for initial and already-advanced bytes/bytearray iterators.
- `STRING_RUNTIME` also includes
  `cpython_bytearray_iterator_pickle_shared_exporter_diff_subset` /
  `cpython_bytearray_iterator_pickle_shared_exporter_subset`, covering CPython
  `ByteArrayTest::test_iterator_pickling2` shared copied-bytearray exporter
  behavior for initial, running, empty, and exhausted bytearray iterators.
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
  `cpython_ast_snippets_exec_to_tuple_match_subset` extends that public-AST
  `snippets.py::exec_tests` evidence to the match-statement snapshots,
  including `Match`, `match_case`, `MatchValue`, `Constant`, `Pass`, and
  wildcard `MatchAs` source locations plus compile-from-public-AST
  round-trips.
  `cpython_ast_snippets_exec_to_tuple_annotations_subset` extends that
  CPython public-AST snippet evidence to module/class docstrings, varargs,
  kwargs, unpacked vararg annotations, return annotations using starred
  generic aliases, and all-parameter-kind function signatures.
  `cpython_ast_snippets_exec_to_tuple_assignment_ops_subset` extends that
  public-AST evidence to `AnnAssign` with starred generic annotations and every
  augmented-assignment operator singleton from `Add` through `FloorDiv`.
  `cpython_ast_snippets_exec_to_tuple_assignment_targets_and_blocks_subset`
  extends it to tuple/list/subscript assignment targets, `for` / `while`
  `else` bodies, and CPython's nested-`If` public AST representation for
  `elif` chains.
  `cpython_ast_snippets_exec_to_tuple_with_raise_assert_subset` extends it to
  `withitem` variants, parenthesized with-items, `Raise` exception/cause
  shapes, and assert messages.
  `cpython_ast_snippets_exec_to_tuple_try_handlers_subset` extends it to `Try`,
  `TryStar`, `ExceptHandler` names, `else` bodies, and `finally` bodies.
  `cpython_ast_snippets_exec_to_tuple_import_control_subset` extends it to
  ordinary and lazy `Import` / `ImportFrom` nodes, `alias` spans, `Global`,
  `Pass`, `Break`, `Continue`, and tuple/list `For` targets.
  `cpython_ast_snippets_exec_to_tuple_decorators_namedexpr_subset` extends it
  to decorated `FunctionDef` / `AsyncFunctionDef` / `ClassDef` nodes,
  decorator-list spans, generator-expression and attribute decorators, and
  `NamedExpr` nodes in expression, `if`, and `while` statement positions.
  `cpython_ast_snippets_exec_to_tuple_positional_only_params_subset` extends it
  to positional-only `arguments.posonlyargs`, positional defaults,
  keyword-only `kw_defaults` `None` slots, and `kwarg` source spans.
  `cpython_ast_snippets_exec_to_tuple_type_params_subset` extends it to PEP 695
  `TypeAlias`, generic class/function `type_params`, `TypeVar` bounds/defaults,
  `TypeVarTuple`, and `ParamSpec` public AST nodes.
  `cpython_ast_start_modes_public_to_tuple_subset` pins public root-node
  `to_tuple()` shapes for `Expression`, `Interactive`, and `FunctionType`
  across `eval`, `single`, and `func_type` parsing modes.
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
  compile-from-public-AST round-trips. `cpython_ast_snippets_public_order_subset`
  adds the CPython `_assertTrueorder` invariant over the full current
  `test_snippets` 219-case `exec`, `single`, and `eval` matrix, including
  recursive source-position ordering and `_fields == __match_args__` on public
  AST instances; the same matrix also ports `AST_Tests.test_ast_validation` by
  parsing each snippet in default `exec` mode and compiling the resulting public
  AST. `cpython_test_manifest.md` now has a checked method-level `AST_Tests`
  audit for all 61 current methods, including explicit partial entries for the
  remaining public matrices and CPython-internal classifications for tests that
  should not drive MiniPython implementation shape.
  `cpython_ast_module_parse_dump_first_pass_subset`
  exposes Python-visible `ast.parse()` / `ast.dump()` across `exec`, `eval`,
  `single`, and `func_type` modes, plus first-pass public node fields such as
  `Module.body`, `Assign.targets`, `Name.id`, `_fields`, and
  `isinstance(..., ast.AST)`. `cpython_ast_parse_public_diff_subset` provides
  gated direct CPython output parity for the same public `ast.parse()` wrapper
  surface when the oracle has current default-field `ast.dump()` behavior.
  `cpython_ast_parse_null_bytes_subset` ports
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
	  The `ASTOptimizationTests` manifest audit now maps all 3 current CPython
	  methods to those optimization tests and has a drift guard against the local
	  CPython source.
  `cpython_ast_docstring_optimization_single_node_subset` and
  `cpython_ast_docstring_optimization_multiple_nodes_subset` port CPython's
  `optimize=2` docstring removal for class, function, and async-function
  bodies, including `Pass` replacement for single-docstring bodies.
  `cpython_ast_invalid_position_information_subset` and
  `cpython_ast_negative_locations_compile_subset` port CPython's public-AST
  location validation for invalid line/column ranges while preserving accepted
  negative-location compile cases. `cpython_ast_pep758_feature_version_subset`
  ports CPython's PEP 758 `feature_version` gate for unparenthesized multiple
  `except` / `except*` exception types and the full single-expression
  acceptance matrix for ordinary and starred handlers. `cpython_ast_feature_version_gates_subset`
  ports additional CPython `feature_version` gates for positional-only
  parameters, assignment expressions, t-strings, exception groups, type
  parameters, type-parameter defaults, and invalid major versions.
  `cpython_ast_compile_only_ast_first_pass_subset`
  adds first-pass `compile(..., ast.PyCF_ONLY_AST)` parity for `exec`, `eval`,
  and `single` modes. `cpython_builtin_compile_optimized_ast_subset` adds
  CPython `BuiltinTest::test_compile_ast` coverage for
  `compile(..., ast.PyCF_OPTIMIZED_AST)` over source and public-AST inputs,
  including default debug-value folding, explicit `optimize=1`, and preserving
  unoptimized `PyCF_ONLY_AST` output even when `optimize=2` is supplied.
  `cpython_builtin_compile_top_level_await_no_coro_subset` covers CPython
  `BuiltinTest::test_compile_top_level_await_no_coro`: MiniPython exposes
  `ast.PyCF_ALLOW_TOP_LEVEL_AWAIT`, accepts it for ordinary non-awaiting
  `single` and `exec` compilation samples, and keeps the returned code object's
  `co_flags` free of `inspect.CO_COROUTINE`.
  `cpython_builtin_compile_top_level_await_subset` covers CPython
  `BuiltinTest::test_compile_top_level_await`: top-level `await`,
  `async for`, `async with`, async comprehensions, optimized-away async assert
  and `__debug__` branch samples require `ast.PyCF_ALLOW_TOP_LEVEL_AWAIT`,
  produce `inspect.CO_COROUTINE` code objects across `single` / `exec` and
  optimize levels `-1`, `0`, `1`, and `2`, and execute correctly through both
  `types.FunctionType(co, globals)` and `eval(co, globals)` with module-code
  globals writeback.
  `cpython_builtin_compile_top_level_await_invalid_cases_subset` covers CPython
  `BuiltinTest::test_compile_top_level_await_invalid_cases`, preserving
  `SyntaxError` rejection for nested ordinary functions that use `await`, async
  comprehensions, `async for`, or `async with`, both with and without
  `ast.PyCF_ALLOW_TOP_LEVEL_AWAIT`.
  `cpython_builtin_compile_async_generator_flag_subset` covers CPython
  `BuiltinTest::test_compile_async_generator`, proving the top-level-await flag
  does not change async-generator function classification and the resulting
  object remains `types.AsyncGeneratorType`.
  `cpython_ast_parse_exact_subset` splits CPython
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
  unpacked keyword handling. The `ASTConstructorTests` manifest audit now maps
  those current CPython methods to the direct evidence and has a drift guard
  against the local CPython source.
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
  The `cpython_ast_copy_*_exact_subset` tests now split current `CopyTests`
  methods into direct method-level evidence, including AST `__reduce__()` state
  snapshots for copied location attributes. The manifest also has a
  `CopyTests` method-audit drift guard. Custom AST subclass field replacement
  now preserves CPython's string and object identity checks.
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
  `_attributes`, fieldless operator nodes, and missing runtime
  fields/attributes. `cpython_ast_compare_literals_exact_subset` ports
  CPython `AST_Tests.test_compare_literals`, including signed integers, float
  infinities, non-ASCII strings, tuples, frozensets, and exact type inequality
  for same-looking int/float/bool/complex constants.
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
  replacement, in-place node mutation, and node replacement. The
  `NodeTransformerTests` manifest audit now maps those five current CPython
  methods to this evidence and has a drift guard against the local CPython
  source.
  `cpython_ast_constant_compile_first_pass_subset` ports the current
  `ConstantTests` methods for compiling public `ast.Constant` nodes holding
  supported singleton/value constants, rejecting invalid list constants,
  rejecting `Constant` assignment targets, module docstring lookup, replacing
  `BinOp` operands for `literal_eval()`, preserving supported string-prefix
  `kind` metadata, and observing supported `LOAD_CONST` values through a
  minimal `dis` module subset. The `ConstantTests` manifest audit now maps all
  8 current CPython methods to this evidence and has a drift guard against the
  local CPython source.
  `cpython_ast_literal_eval_first_pass_subset` adds first-pass
  `ast.literal_eval()` coverage for safe literal containers, bytes, sets,
  numeric signs, complex literals, AST-node input, and malformed expression
  rejection. `cpython_ast_literal_eval_exact_subset` splits CPython
  `ASTHelpers_Test::test_literal_eval` into direct method-level coverage.
  `cpython_ast_literal_eval_public_diff_subset` provides direct CPython output
  parity for the stable public literal, AST-node input, complex-literal, and
  malformed-expression rejection surface.
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
  `cpython_compile_source_positions_code_positions_first_pass_subset` adds the
  first public `code.co_positions()` iterator surface for a simple assignment,
  exposing the real assignment line plus statement-aligned column bounds that
  satisfy CPython's AST-offset membership invariant.
  `cpython_compile_source_positions_public_invariants_diff_subset` provides
  gated direct CPython output parity for public `co_positions()` and
  `co_lines()` invariants without asserting opcode-count or exact opcode-level
  position identity.
  `cpython_compile_source_positions_lambda_return_position_subset` ports
  CPython's public lambda-return position invariant for the representative
  `lambda: x`, `lambda: 42`, `lambda: 1 + 2`, and `lambda: a + b` snippets,
  requiring each exposed lambda `co_positions()` tuple to stay inside the lambda
  body expression columns.
  `cpython_compile_source_positions_weird_attribute_position_regressions_subset`
  ports CPython's public safety invariant for unusual multiline attribute
  chains by requiring every exposed function `co_positions()` tuple to have
  non-`None` bounds and ordered start/end source coordinates.
  `cpython_compile_source_positions_multistatement_code_lines_subset` extends
  that first-pass line-span model so runtime `compile(..., "exec")` code
  objects expose every statement-leading source line through both `co_lines()`
  and `co_positions()`.
  `cpython_compile_specifics_lineno_after_no_code_first_pass_subset` starts the
  function `__code__` line-table surface for no-code function bodies, exposing
  source-token-derived `co_firstlineno`, one public `co_lines()` span whose line
  matches `co_firstlineno`, plus matching `co_positions()` line/`None`-column
  tuples.
  `cpython_compile_specifics_lineno_procedure_call_subset` ports CPython's
  public procedure-call line-table invariant that an opening-paren-only
  physical line is not reported by function `co_lines()`.
  `cpython_compile_specifics_lineno_attribute_subset` extends the first-pass
  function line-table model to CPython's public multiline attribute load,
  method-call, store, and augmented-store `co_lines()` sequences.
  `cpython_compile_specifics_lineno_public_invariants_diff_subset` provides
  gated direct CPython output parity for these public `co_lines()` line-number
  invariants while deliberately avoiding `dis` opcode-line assertions.
  `cpython_compile_specifics_line_number_implicit_return_after_async_for_subset`
  ports CPython's public async-function `co_lines()` sequence for an implicit
  return after an `async for` loop.
  `cpython_compile_specifics_lineno_after_implicit_return_subset` ports
  CPython's public `sys._getframe()` line-number behavior for implicit returns
  after executed and skipped `if` bodies.
  `cpython_compile_specifics_if_implicit_return_code_lines_subset` pins the
  corresponding public `co_lines()` order for `if` bodies with implicit returns.
  `cpython_compile_specifics_lineno_of_backward_jump_conditional_in_loop_subset`
  pins the public loop-backedge `co_lines()` line for a conditional inside a
  loop.
  `cpython_compile_specifics_synthetic_jump_line_tables_subset` ports the
  public function `co_lines()` order for CPython's synthetic-jump
  multiple-predecessor try/loop cold-block shapes.
  `cpython_compile_specifics_lineno_propagation_empty_blocks_subset` ports the
  public function `co_lines()` order for CPython's empty-block
  while/try/except/else smoke test.
  `cpython_compile_specifics_line_number_genexp_subset` ports CPython's public
  nested generator-expression code-object line table exposed through an outer
  function's `co_consts`.
  Together these line-table tests provide method-level evidence for the
  public-compatible `TestSpecifics` line-number methods; the original
  `co_code` length and `dis.Bytecode(...).positions` opcode assertions are
  classified as CPython bytecode/debug-position internals.
  `cpython_compile_specifics_big_dict_literal_subset` ports CPython
  `TestSpecifics::test_big_dict_literal` at the public source level by
  evaluating a 0xFFFF+1-entry dict display and preserving the full length.
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
  relative-import bytecode-level tracking, package-context runtime resolution
  against the builtin stdlib module table, and `_pyrepl/reader.py`
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
  `cpython_ast_repr_eval_expression_snapshot_subset`,
  `cpython_ast_repr_full_snapshot_from_cpython_source_subset`, and
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
  method-level coverage. The manifest's `EndPositionTests` method audit maps
  all 28 current methods to direct Rust evidence and has a drift guard against
  the local CPython source. The `ModuleStateTests` and `CommandLineTests`
  manifest audits classify all 16 current methods as `blocked_by_ast_module`
  and guard those classifications against the local CPython source; these tests
  exercise CPython `_ast` module lifecycle, subinterpreter teardown, and
  `python -m ast` / `ast.main()` CLI behavior rather than MiniPython language
  semantics.
  `cpython_ast_dump_plain_first_pass_subset` ports CPython
  `ASTHelpers_Test::test_dump` plain `ast.dump()` rendering for default,
  `annotate_fields=False`, and `include_attributes=True` forms.
  `cpython_ast_dump_exact_subset` splits the same CPython method into direct
  method-level `exact_subset` coverage. `cpython_ast_dump_public_diff_subset`
  provides gated direct CPython output parity for the current default-field
  rendering behavior, plus `annotate_fields=False`, `include_attributes=True`,
  indentation, and incomplete-node dump forms.
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
  execution. The separate `LazyImportTest::test_lazy_import` method is
  classified as `blocked_by_runtime` because CPython's current test only checks
  `ensure_lazy_imports("ast", ...)` child-process import side effects.
  Exact CPython warning behavior,
  subclassing, field validation, full `to_tuple()` snippet coverage, parser
  source-location spans for remaining node families, remaining generated-node
  dump edge cases, deeper `literal_eval()` edge cases such as integer digit
  limits, and broader compile-from-public-AST parity remain open.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_core_runtime_subset`, covering the supported
  `Hashable`, `Sized`, `Container`, `Callable`, and `Collection` ABC public
  runtime surface, built-in container/type relationships, structural user
  classes, direct ABC subclassing, and CPython-style `None` blocking for
  special methods. Direct CPython diff evidence is in
  `cpython_collections_abc_core_runtime_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_sequence_subset`, covering `Sequence` for supported
  built-in sequence registrations including `memoryview`, explicit Sequence
  subclassing, CPython's non-structural Sequence behavior, and inheritance
  through `Reversible`, `Collection`, `Sized`, `Iterable`, and `Container`.
  Direct CPython diff evidence is in
  `cpython_collections_abc_sequence_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_sequence_mixins_subset`, covering CPython
  `TestCollectionABCs::test_Sequence_mixins` for explicit `Sequence`
  subclasses, `index()` parity against native list/string start-stop behavior,
  `count()`, containment, iteration, reverse iteration, and keyword argument
  binding. Direct CPython diff evidence is in
  `cpython_collections_abc_sequence_mixins_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_mapping_subset`, covering `Mapping` and
  `MutableMapping` for registered `dict`, ABC inheritance, direct subclassing,
  and CPython's non-structural mapping behavior. Direct CPython diff evidence
  is in `cpython_collections_abc_mapping_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_mapping_view_subset`, covering `MappingView`,
  `KeysView`, `ItemsView`, and `ValuesView` for built-in dict views, set-like
  keys/items behavior, values-view collection behavior, ABC inheritance, direct
  ABC subclassing, and CPython's non-structural view behavior. Direct CPython
  diff evidence is in `cpython_collections_abc_mapping_view_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_mutable_sequence_subset`, covering
  `MutableSequence` registrations for list, bytearray, `collections.deque`,
  and `array.array`, inheritance through `Sequence` and related ABCs,
  CPython's non-structural protocol behavior, explicit subclass mixins, and
  self-extension. Direct CPython diff evidence is in capability-gated
  `cpython_collections_abc_mutable_sequence_diff_subset` for CPython oracles
  that expose the current `array.array` registration.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_set_mutable_set_mixins_subset`, covering `Set` and
  `MutableSet` registrations for set/frozenset and supported set-like dict
  views, inheritance through `Collection` / `Sized` / `Iterable` / `Container`,
  explicit subclass mixins for comparison, binary set operations, `_hash`,
  `_from_iterable`, mutable update methods, and self-clearing regressions.
  Direct CPython diff evidence is in
  `cpython_collections_abc_set_mutable_set_mixins_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_set_from_iterable_operator_subset`, covering CPython
  `test_Set_from_iterable` operator dispatch through `MutableSet` mixins,
  instance `_from_iterable` overrides, and `^=` with a non-Set iterable.
  Direct CPython diff evidence is in
  `cpython_collections_abc_set_from_iterable_operator_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_set_real_set_interoperability_subset`, covering
  CPython `test_Set_interoperability_with_real_sets` for custom `Set`
  subclasses interacting with built-in `set` and plain iterables across `&`,
  `|`, `-`, `^`, ordering, equality, inequality, and TypeError paths for
  non-Set ordering operands. Direct CPython diff evidence is in
  `cpython_collections_abc_set_real_set_interoperability_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_set_hash_matches_frozenset_subset`, covering CPython
  `test_Set_hash_matches_frozenset` for the current public `Set._hash()` mixin
  algorithm over supported hashable samples, including `None`, numbers,
  strings, booleans, object identities, NaN, nested frozensets, large integers,
  range-derived frozensets, and the CPython `sys.maxsize` range stress sample.
  Direct CPython diff evidence is in
  `cpython_collections_abc_set_hash_matches_frozenset_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_set_noncomparable_comparison_subset`, covering
  CPython `TestCollectionABCs::test_issue16373` for `Set` subclass comparison
  fallback when the left operand returns `NotImplemented`. Direct CPython diff
  evidence is in
  `cpython_collections_abc_set_noncomparable_comparison_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_reversible_subset`, covering `Reversible` for
  supported built-in reversible containers/views, non-reversible scalar,
  container, and iterator samples, `Sequence` inheritance, structural
  `__iter__` + `__reversed__` user classes, direct ABC subclassing, and `None`
  blocking. Direct CPython diff evidence is in
  `cpython_collections_abc_reversible_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_async_runtime_subset`, covering `Awaitable`,
  `Coroutine`, `AsyncIterable`, and `AsyncIterator` for native coroutine
  objects, structural user classes, non-samples, ABC inheritance, and `None`
  blocking. Direct CPython diff evidence is in
  `cpython_collections_abc_async_runtime_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_mapping_mixins_subset`, covering explicit
  `Mapping` / `MutableMapping` subclass mixins for `get`, containment, key/item
  listing, equality, `pop`, `popitem`, `clear`, `update`, and `setdefault`.
  Direct CPython diff evidence is in
  `cpython_collections_abc_mapping_mixins_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_mapping_mixin_views_subset`, covering live
  `KeysView` / `ItemsView` / `ValuesView` objects returned by explicit
  `Mapping` mixins, including membership, iteration after mutation, and
  set-like key/item view operators. Direct CPython diff evidence is in
  `cpython_collections_abc_mapping_mixin_views_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_userdict_view_snapshot_subset`, covering CPython
  `TestCollectionABCs::test_MutableMapping_subclass` for `UserDict`
  keys/items/values view ABC relationships and eager set-operation snapshots
  that are not affected by later `UserDict` mutation. Direct CPython diff
  evidence is in
  `cpython_collections_abc_userdict_view_snapshot_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_bytestring_buffer_subset`, covering CPython
  `TestCollectionABCs::test_ByteString` and `::test_Buffer` public ABC
  relationships for `ByteString`, `Buffer`, bytes-like builtins, memoryview,
  direct subclassing, and structural `__buffer__` checks. Direct CPython diff
  evidence is in `cpython_collections_abc_bytestring_buffer_diff_subset`,
  gated for CPython oracles with collections.abc.Buffer.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_bytestring_deprecation_warnings_subset`, covering
  CPython `TestCollectionABCs::test_ByteString` and
  `::test_ByteString_attribute_access` deprecation warnings for public
  `ByteString` import, fresh attribute access, `isinstance()`, class-statement
  subclass creation, and dynamic `type(..., (ByteString,), ...)` subclass
  creation. Direct CPython diff evidence is in
  `cpython_collections_abc_bytestring_deprecation_warnings_diff_subset`, gated
  for CPython oracles that warn for `ByteString`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_issue26915_identity_first_object_subset`, covering
  CPython `TestCollectionABCs::test_issue26915` identity-first membership for
  `support.NEVER_EQ`-style objects and distinct `float('nan')` objects across
  `Sequence`, `ItemsView`, `KeysView`, and `ValuesView`, plus
  `Sequence.index()` / `count()`. Direct CPython diff evidence is in
  `cpython_collections_abc_issue26915_identity_first_object_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_composite_abstract_methods_subset`, covering
  CPython `TestCollectionABCs` abstract-method rejection behavior for `Set`,
  `MutableSet`, `Mapping`, `MutableMapping`, `Sequence`, `MutableSequence`,
  `ByteString`, and `Buffer`, including direct ABC constructor rejection and
  incomplete explicit-subclass rejection. Direct CPython diff evidence is in
  `cpython_collections_abc_composite_abstract_methods_diff_subset`, gated for
  CPython oracles with collections.abc.Buffer.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_abstract_methods_subset`, covering the public
  `ABCTestCase.validate_abstract_methods` behavior used by CPython's
  one-trick pony ABC tests: complete direct subclasses instantiate,
  subclasses missing each required abstract method raise `TypeError` with
  CPython-style abstract-method text, and direct ABC constructors reject
  instantiation with the exact public method-name list without copying
  CPython's ABCMeta cache internals. Direct CPython diff evidence is in
  `cpython_collections_abc_abstract_methods_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_direct_subclassing_subset`, covering CPython's
  `TestOneTrickPonyABCs::test_direct_subclassing` loop for `Hashable`,
  `Iterable`, `Iterator`, `Reversible`, `Sized`, `Container`, and `Callable`.
  Direct CPython diff evidence is in
  `cpython_collections_abc_direct_subclassing_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_registration_subset`, covering CPython's
  `TestOneTrickPonyABCs::test_registration` public `ABC.register()` behavior
  for `Hashable`, `Iterable`, `Iterator`, `Reversible`, `Sized`, `Container`,
  and `Callable`. Direct CPython diff evidence is in
  `cpython_collections_abc_registration_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_iterable_sample_matrix_subset`, covering CPython's
  `Iterable` public non-sample/sample matrix, including dict views, native
  generators, generator expressions, direct-subclass `super().__iter__()`
  mixin behavior, and `__iter__ = None` blocking. Direct CPython diff evidence
  is in `cpython_collections_abc_iterable_sample_matrix_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_iterable_iterator_subset`, covering the supported
  `Iterable` and `Iterator` public ABC surface for built-in containers,
  iterators, scalar non-samples, structural user classes, direct ABC
  subclassing, and `Iterator` inheritance through `Iterable`. Direct CPython
  diff evidence is in `cpython_collections_abc_iterable_iterator_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_validate_isinstance_subset`, covering the public
  `ABCTestCase.validate_isinstance` helper behavior used by CPython's one-trick
  pony ABC tests for `Hashable`, `AsyncIterable`, `Iterable`, `Sized`,
  `Container`, and `Callable`, including dynamic `setattr()` of the target
  special method and `__hash__ = None` blocking through an explicit `object`
  base. Direct CPython diff evidence is in
  `cpython_collections_abc_validate_isinstance_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_async_runtime_subset`, covering CPython's
  `AsyncIterator` non-sample matrix for `None`, `object`, and `list` through
  both `isinstance` and `issubclass(type(...))`, plus the supported
  `Awaitable`, `Coroutine`, and `AsyncIterable` public runtime cases.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_async_iterator_mixin_subset`, covering CPython's
  `AsyncIterator` direct-subclass mixin behavior where inherited
  `__aiter__()` returns `self`. Direct CPython diff evidence is in
  `cpython_collections_abc_async_iterator_mixin_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_async_generator_core_mixin_subset`, covering
  CPython's `AsyncGenerator` direct-subclass `__aiter__` and `__anext__`
  mixins, including `__anext__` delegation through `asend(None)`. Direct
  CPython diff evidence is in
  `cpython_collections_abc_async_generator_core_mixin_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes expanded
  `cpython_collections_abc_generator_runtime_subset` evidence for CPython's
  `AsyncGenerator` negative protocol matrix, including the exact `NonAGen1`,
  `NonAGen2`, and `NonAGen3` shapes from `TestOneTrickPonyABCs`. Direct
  CPython diff evidence is in
  `cpython_collections_abc_generator_runtime_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_set_noncomparable_comparison_subset`, covering
  CPython `TestCollectionABCs::test_issue16373` for `Set` subclass rich
  comparison fallback when the left operand returns `NotImplemented`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_async_generator_throw_close_mixin_subset`, covering
  CPython's `AsyncGenerator` direct-subclass default `athrow()` and
  `aclose()` mixin behavior for coroutine object type / `Awaitable` /
  `Coroutine` parity, `.send(None)` drive-to-`StopIteration`, `.close()`,
  reuse errors, raised exception types, explicit exception instances with
  `tb=None`, accepted real traceback-object arguments with CPython's raised
  traceback replacement behavior, invalid non-traceback `tb` rejection,
  swallowed `GeneratorExit` /
  `StopAsyncIteration`, close-time error propagation, and ignored-exit
  `RuntimeError`. Direct CPython diff evidence is in
  `cpython_collections_abc_async_generator_throw_close_mixin_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_types_coroutine_subset`, covering public
  `types.coroutine()` generator-function behavior, the CPython distinction
  that iterable-coroutine generators can be awaited but are not
  `Awaitable` / `Coroutine` ABC instances, and `Coroutine.register()`
  propagation through `Awaitable`. Direct CPython diff evidence is in
  `cpython_collections_abc_types_coroutine_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_hashable_direct_subclass_subset`, covering
  CPython's `Hashable` direct-subclass mixin behavior where
  `super().__hash__()` returns the ABC fallback value `0` and unrelated builtin
  types are not subclasses of the generated subclass. Direct CPython diff
  evidence is in `cpython_collections_abc_hashable_direct_subclass_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_reversible_direct_subclass_subset`, covering
  CPython's `Reversible` direct-subclass runtime behavior where a subclass
  implementing `__iter__` and `__reversed__` returns an empty iterator through
  `reversed()` and unrelated builtin types are not subclasses of that generated
  subclass. Direct CPython diff evidence is in
  `cpython_collections_abc_reversible_direct_subclass_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_collection_direct_subclass_subset`, covering
  CPython's `Collection` direct-subclass runtime behavior, derived-subclass
  iteration, generated-subclass rejection for unrelated builtin classes,
  missing-method non-samples, direct `None` blocking, and inherited
  `__contains__ = None` blocking. Direct CPython diff evidence is in
  `cpython_collections_abc_collection_direct_subclass_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_iterator_sample_matrix_subset`, covering CPython's
  `Iterator` public sample matrix with non-iterator scalar/container values,
  bytes/string/tuple/list/dict/set/frozenset/dict-view iterators, native
  generators, generator expressions, and the Issue 10565 `__next__`-only
  rejection. Direct CPython diff evidence is in
  `cpython_collections_abc_iterator_sample_matrix_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_generator_sample_matrix_subset` and
  `cpython_collections_abc_generator_mixin_subset`, covering CPython's
  `Generator` public non-sample/sample matrix, including lambda-yield generator
  sources, direct `Generator` subclass mixins for `__iter__`, `__next__`, default
  `throw`, `close`, and the `FailOnClose` / `IgnoreGeneratorExit` close
  edge cases. Direct CPython diff evidence is in
  `cpython_collections_abc_generator_sample_matrix_diff_subset` and
  `cpython_collections_abc_generator_mixin_diff_subset`.
- `COLLECTIONS_ABC_RUNTIME` also includes
  `cpython_collections_abc_coroutine_mixin_subset`, covering CPython's
  `Coroutine` direct-subclass mixins for default `send`, `throw`, and `close`,
  including `StopIteration`, exception propagation, swallowed `GeneratorExit`,
  ignored-exit `RuntimeError`, and close-time error propagation. Direct CPython
  diff evidence is in `cpython_collections_abc_coroutine_mixin_diff_subset`.
- `CONTAINER_RUNTIME` also includes `cpython_dict_view_richcompare_subset`,
  covering set-style rich comparisons for dict key/item views and propagation
  of Python-level `__eq__` errors during item-view comparisons.
- `CONTAINER_RUNTIME` also includes `cpython_dict_view_mappingproxy_subset`,
  covering dict-view `.mapping`, the read-only `mappingproxy` type object,
  live equality with the underlying dict, lookup, membership, and assignment
  rejection, plus direct dict-view `__len__` / `__contains__` / `__repr__`
  methods where CPython exposes them.
- `CONTAINER_RUNTIME` also includes
  `cpython_dict_numeric_key_equivalence_subset`, backed by direct CPython
  output parity in `cpython_dict_numeric_key_equivalence_diff_subset`, covering
  dict/set key matching for `bool`, exact `int`, exact `float`, exact
  `complex`, and numeric subclasses whose equality and hash values match,
  while preserving identity lookup for a stored `NaN` key.
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
  length, iteration, display, empty-format, copy, get, keys, items, and values
  calls to user-defined mapping objects.
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
  `collections.ChainMap` mapping sources. Direct ChainMap output parity is
  guarded by `cpython_types_mappingproxy_chainmap_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_userdict_public_methods_diff_subset` and
  `cpython_collections_userdict_public_methods_subset`, covering CPython
  `TestUserObjects` public `UserDict` behavior for `dir(UserDict)` protocol
  coverage against `dict`, item assignment/deletion, lookup, iteration,
  containment, `get()`, `.data`, direct display/empty-format methods, recursive
  display, `.copy()`, and `copy.copy()` with shallow instance-attribute
  copying.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_userlist_public_methods_diff_subset` and
  `cpython_collections_userlist_public_methods_subset`, covering CPython
  `TestUserObjects` public `UserList` behavior for `dir(UserList)` protocol
  coverage against `list`, construction from lists and other UserList objects,
  `.data`, list mutation/iteration/length/containment, direct
  display/empty-format methods, recursive display, `.copy()`, and `copy.copy()`
  with shallow instance-attribute copying.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_userlist_namedtuple_sequence_order_diff_subset` and
  `cpython_collections_userlist_namedtuple_sequence_order_subset`, covering
  CPython public sequence comparison behavior shared by `UserList` / `list`
  and `namedtuple` / `tuple`, including `NotImplemented` and `TypeError`
  boundaries for mixed sequence types.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_userstring_protocol_and_userdict_missing_diff_subset`
  and
  `cpython_collections_userstring_protocol_and_userdict_missing_subset`,
  covering the remaining CPython `TestUserObjects` public behavior for
  `dir(UserString)` protocol coverage against `str` plus `UserDict` subclass
  `__missing__` dispatch for subscript lookup and `.get()`.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_chainmap_public_methods_subset`, covering CPython
  `TestChainMap` public behavior for construction, truthiness, first-map
  assignment/deletion, `maps`, `parents`, `new_child()`, ordering, dict
  coercion, iteration, views, containment, lookup, `get()`, and shallow copies.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_chainmap_copy_pickle_eval_identity_subset`, covering
  the remaining CPython `TestChainMap::test_basics` copy and identity
  assertions: exact repr alternatives, shallow-copy parent-map sharing,
  pickle round trips across every exposed protocol, `copy.deepcopy()`, and
  `eval(repr(...))`.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_chainmap_missing_and_first_map_mutation_diff_subset`
  and
  `cpython_collections_chainmap_missing_and_first_map_mutation_subset`, covering
  ChainMap subclass `__missing__`, missing-safe `get()` and membership,
  first-map `pop()`, `popitem()`, `clear()`, and subclass-backed item
  assignment/deletion.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_chainmap_iter_does_not_call_getitem_diff_subset` and
  `cpython_collections_chainmap_iter_does_not_call_getitem_subset`, covering
  CPython `TestChainMap::test_iter_not_calling_getitem_on_maps` semantics:
  ChainMap iteration over a `UserDict` subclass must use keys without invoking
  the underlying map's overridden `__getitem__`.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_chainmap_new_child_custom_mapping_diff_subset` and
  `cpython_collections_chainmap_new_child_custom_mapping_subset`, covering
  CPython `TestChainMap::test_new_child` lowerdict behavior: a custom child
  mapping's `__contains__` and `__getitem__` must drive ChainMap containment,
  `get()`, and subscript lookup rather than direct internal dict-entry scans.
  The subset also keeps MiniPython's keyword-child construction coverage, which
  is outside this local CPython oracle's direct diff surface.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_chainmap_order_preservation_diff_subset` and
  `cpython_collections_chainmap_order_preservation_subset`, covering CPython
  `TestChainMap::test_order_preservation`: ChainMap iteration and `items()`
  must preserve the combined order produced by the OrderedDict multi-map
  matrix.
- `CONTAINER_RUNTIME` also includes
  `cpython_collections_chainmap_union_operators_diff_subset` and
  `cpython_collections_chainmap_union_operators_subset`, covering CPython
  `TestChainMap::test_union_operators` semantics for ChainMap/mapping union,
  in-place union, iterable-pair update, iterable-pair rejection for `|`, and
  subclass result-type rules including `SubclassRor.__ror__ -> super()`.
- `CONTAINER_RUNTIME` also includes
  `cpython_ordered_dict_constructor_update_subset` and
  `cpython_ordered_dict_mapping_mutation_subset`, with direct coverage in
  `cpython_program_output_parity_smoke_diff_subset`, covering the minimal
  `OrderedDict` sandbox surface for keyword/pair/mapping construction,
  iterable-plus-keyword update, constructor/update error classes,
  `setdefault()`, `update()`, `pop()`, `get()`, key membership, direct
  `__contains__()`, key iteration, direct `__iter__()`, `__len__()`,
  `__getitem__()`, `__setitem__()`, item deletion, and `clear()` while
  preserving ordered storage.
- `CONTAINER_RUNTIME` also includes
  `cpython_ordered_dict_move_pop_keyword_subset`, backed by
  `cpython_program_output_parity_smoke_diff_subset` through the
  `ordered-dict-move-pop-keyword-subset` case, covering `move_to_end()` and
  `popitem()` `last=` keyword binding, type-level method calls, missing-key
  errors, empty-pop errors, unexpected-keyword errors, and duplicate-argument
  errors without promoting the full OrderedDict runtime surface.
- `CONTAINER_RUNTIME` also includes
  `cpython_ordered_dict_view_display_subset`, backed by
  `cpython_program_output_parity_smoke_diff_subset` through the
  `ordered-dict-view-display-subset` case and by gated
  `cpython_ordered_dict_view_mapping_diff_subset` evidence for CPython oracles
  with `dictview.mapping`, covering live `keys()` / `items()` / `values()`
  views, `odict_keys` / `odict_items` / `odict_values` public type names and
  reprs, type-level view calls, membership, ordered live `.mapping`
  `mappingproxy` display, empty-view display, and a normal-dict regression
  guard.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_new_class_resolve_bases_subset`, covering the
  first CPython `ClassCreationTests` public slice for `types.new_class()`,
  `types.prepare_class()`, and `types.resolve_bases()`: exported names,
  default object bases, explicit builtin bases, `exec_body` namespace mutation,
  callable metaclass keyword forwarding without mutating the caller's `kwds`
  mapping, default and custom `__prepare__` namespaces, tuple identity
  preservation when base resolution is unnecessary, derived metaclass
  selection and inherited custom `__prepare__` namespaces for direct class
  statements and `types.new_class()`, custom metaclass `__new__` / `__init__`
  ordering for direct class statements and `types.new_class()`, catchable
  metaclass conflict `TypeError`, class-object
  `isinstance(..., metaclass)` recognition for those same paths,
  tuple-subclass bases preserved through `type()` and `types.new_class()`,
  `typing.List[int]` and `list[int]` base resolution including
  `__orig_bases__` and public `__mro__` shape,
  class-based and call-based `typing.NamedTuple` / `typing.TypedDict`
  original-bases preservation with their public tuple/dict runtime bases,
  `__mro_entries__` replacement/removal, `__orig_bases__` preservation, and
  non-tuple `__mro_entries__` rejection. The stable core of this public slice
  is guarded by `cpython_types_class_creation_new_class_resolve_bases_diff_subset`;
  generic-alias base-resolution behavior is guarded separately by
  `cpython_types_class_creation_mro_entries_core_diff_subset` and
  `cpython_types_class_creation_prepare_resolve_bases_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_new_class_meta_helper_subset`, covering CPython
  `ClassCreationTests::test_new_class_basics` through
  `::test_new_class_meta_with_base`: `type.__prepare__` returns a dict
  namespace, the CPython helper metaclass can call `super().__prepare__`,
  staticmethod metaclass `__new__` can delegate to `super().__new__`, metaclass
  `__init__` can delegate to `super().__init__`, `types.new_class()` preserves
  the caller's `kwds` mapping, accepts `exec_body=None`, and supports the
  keyword API with an explicit builtin base. Direct output parity is guarded by
  `cpython_types_class_creation_new_class_meta_helper_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_new_class_metaclass_keywords_subset`, covering
  CPython `ClassCreationTests::test_new_class_metaclass_keywords`: a callable
  metaclass passed to `types.new_class()` receives the requested class name,
  original bases including `(int, object)`, an empty prepared namespace, and
  class keywords with the `metaclass` entry consumed before dispatch. Direct
  output parity is included in
  `cpython_types_class_creation_new_class_meta_helper_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_mro_entries_core_subset`, covering CPython
  `ClassCreationTests::test_new_class_with_mro_entry`,
  `::test_new_class_with_mro_entry_genericalias`,
  `::test_new_class_with_mro_entry_none`, and
  `::test_new_class_with_mro_entry_error`: single instance
  `__mro_entries__` replacement receives the original bases tuple,
  `typing.List[int]` and `list[int]` generic aliases resolve to their public
  runtime bases with `__orig_bases__` preserved, empty tuple results remove
  a base while preserving the original bases, and non-tuple provider results
  raise a catchable `TypeError`. Direct output parity is guarded by
  `cpython_types_class_creation_mro_entries_core_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_prepare_resolve_bases_subset`, covering CPython
  `ClassCreationTests::test_prepare_class`, `::test_resolve_bases`, and
  `::test_resolve_bases_with_mro_entry`: derived metaclass selection in
  `prepare_class()` when explicit `metaclass=type` is overridden by a more
  specific base metaclass, custom `__prepare__` namespace identity, empty
  remaining keywords, `resolve_bases(())`, tuple identity preservation for
  class-only base tuples, instance `__mro_entries__` replacement/removal, and
  `typing.List[int]` / `list[int]` generic-alias base replacement. Direct
  output parity is guarded by
  `cpython_types_class_creation_prepare_resolve_bases_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_mro_entries_multiple_subset`, covering CPython
  `ClassCreationTests::test_new_class_with_mro_entry_multiple` and
  `::test_new_class_with_mro_entry_multiple_2`: multiple instance
  `__mro_entries__` providers each receive the original bases tuple, returned
  tuples expand left-to-right around ordinary class bases, `__orig_bases__`
  preserves the original provider instances, and the public `__mro__` reflects
  the expanded bases. Direct output parity is guarded by
  `cpython_types_class_creation_mro_entries_multiple_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_metaclass_derivation_subset`, covering CPython
  `ClassCreationTests::test_metaclass_derivation`: `types.new_class()` derives
  the most specific metaclass from compatible bases, preserves metaclass
  `__new__` call order through `super()`, invokes the winner metaclass
  `__prepare__`, keeps the result independent of compatible base order, and
  still chooses the derived metaclass when the caller explicitly passes a
  compatible ancestor metaclass such as `type` or `AMeta`. Direct output
  parity is guarded by
  `cpython_types_class_creation_metaclass_derivation_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_get_original_bases_subset`, covering CPython
  `ClassCreationTests::test_get_original_bases`: ordinary classes without
  explicit bases report `object`, generic user classes preserve original
  subscripted bases, builtin classes return their public direct bases, and
  non-type arguments raise catchable `TypeError`. Direct output parity is
  guarded by `cpython_types_class_creation_get_original_bases_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_prepare_and_metaclass_callable_subset`,
  covering the next CPython `ClassCreationTests` public slice for bad
  `__prepare__` and non-class metaclass dispatch: class statements now raise a
  catchable `TypeError` when a type or non-type metaclass `__prepare__`
  returns a non-mapping, `types.prepare_class()` returns the raw
  `__prepare__` namespace without class-statement validation, function
  metaclasses can return arbitrary class-statement objects, and callable-object
  metaclasses receive `__prepare__` / `__call__` name, bases, namespace, and
  class keyword arguments. Direct output parity is guarded by
  `cpython_types_class_creation_prepare_and_metaclass_callable_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_metaclass_override_function_subset`, covering
  CPython `ClassCreationTests::test_metaclass_override_function`:
  `types.new_class()` treats a function metaclass as a direct callable rather
  than a class, forwards the name, bases, empty namespace, and keywords, and
  skips winner-metaclass calculation even for `object` bases and bases whose
  own metaclass is custom. Direct output parity is guarded by
  `cpython_types_class_creation_metaclass_override_function_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_non_type_metaclass_derivation_subset`, covering
  CPython `ClassCreationTests::test_metaclass_override_callable`:
  `types.new_class()` derives the winning metaclass from `type(base)` when
  earlier non-`type` metaclass calls return ordinary objects, preserves
  `__prepare__` / `__new__` call order through `ANotMeta` / `BNotMeta`, accepts
  `object()` bases in that non-`type` metaclass path, and reports catchable
  metaclass conflicts for incompatible `type` / `int()` mixes. Direct output
  parity is guarded by
  `cpython_types_class_creation_non_type_metaclass_derivation_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_one_argument_type_subset`, covering CPython
  `ClassCreationTests::test_one_argument_type`: builtin `type(obj)` remains the
  only one-argument type-inspection form, type subclasses raise catchable
  `TypeError` when called with one argument, and type subclasses still support
  the three-argument dynamic class-construction form. Direct output parity is
  guarded by `cpython_types_class_creation_one_argument_type_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_metaclass_new_error_subset`, covering CPython
  `ClassCreationTests::test_metaclass_new_error`: three-argument `type()`
  derives the winning metaclass from base classes, `super().__new__` in a
  metaclass reaches `type.__new__`, and exceptions raised by the winning
  metaclass constructor propagate as catchable Python exceptions. Direct output
  parity is guarded by
  `cpython_types_class_creation_metaclass_new_error_diff_subset`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_class_creation_subclass_inherited_slot_update_subset`, covering
  CPython `ClassCreationTests::test_subclass_inherited_slot_update`: dict
  subclasses honor dynamic `__getitem__` replacement for subscript lookup and
  delegate back to `dict.__getitem__` after reassignment. Direct output parity
  is guarded by
  `cpython_types_class_creation_subclass_inherited_slot_update_diff_subset`.
- `Lib/test/test_types.py::ClassCreationTests` is now method-audited as
  `ported` in `tests/cpython_test_manifest.md`; all 25 current CPython methods
  have Rust evidence covering the public class-creation helper, metaclass,
  `__mro_entries__`, `__orig_bases__`, `type()`, dynamic slot update, and
  tuple-subclass-bases surfaces.
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
  `cpython_types_simple_namespace_new_and_invalid_replace_subset`, covering
  CPython main `SimpleNamespaceTests::test_replace_invalid_subtype` and public
  `types.SimpleNamespace.__new__` / `.__replace__` behavior: direct allocation,
  subtype validation, exact and subclass replacement methods, and catchable
  `TypeError` when a namespace subclass constructor returns a non-namespace
  object during replacement. Direct CPython output parity is tracked by
  `cpython_types_simple_namespace_new_and_invalid_replace_diff_subset`, gated
  for CPython oracles that expose `copy.replace()` and
  `types.SimpleNamespace.__replace__`.
- `CONTAINER_RUNTIME` also includes
  `cpython_types_simple_namespace_remaining_public_subset`, covering additional
  CPython `SimpleNamespaceTests` public behavior: constructor insertion order,
  underlying `__dict__` lifetime after the namespace object is deleted,
  missing-attribute deletion, delete/reassign cycles, nested namespace
  references, repr insertion order, MiniPython internal-payload pickle round
  trips across exposed protocols, unsupported rich ordering, and fake-namespace
  comparison safety. Direct output parity is guarded by
  `cpython_types_simple_namespace_remaining_public_diff_subset`, gated for
  CPython oracles with positional mapping construction.
- `CONTAINER_RUNTIME` also includes
  `cpython_set_and_frozenset_subclass_subset`, covering first-pass CPython
  set/frozenset subclass construction, iteration, membership, `len`, conversion
  back to exact `set`, builtin method result types, in-place set mutation,
  `super().__init__`, custom `__new__` via `super().__new__`,
  default `repr()` / `str()` / f-string display for empty and non-empty
  subclasses, direct `object.__format__` fallback display, non-empty format spec
  rejection, custom `__format__` priority, frozenset-subclass hashing,
  frozenset subclass copy/constructor identity, empty frozenset subclass
  identity behavior, basic subclass `__slots__`, and Set/MutableSet/Hashable ABC
  registration.
- `RUNTIME_BUILTINS` also includes
  `cpython_all_any_builtin_subset`, covering CPython
  `BuiltinTest::test_all` and `::test_any` semantics for empty iterables,
  truthy/falsy lists, generator expressions, short-circuiting before later
  truthiness failures, RuntimeError propagation from `__bool__` and `__iter__`,
  non-iterable rejection, and catchable argument-count `TypeError`s.
- `RUNTIME_BUILTINS` also includes
  `cpython_builtin_negation_sys_maxsize_subset`, covering CPython
  `BuiltinTest::test_neg` for the `-sys.maxsize - 1` integer boundary,
  `isinstance(..., int)`, and negation back to `sys.maxsize + 1`. Direct CPython
  output parity is tracked by `cpython_builtin_negation_sys_maxsize_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_len_builtin_subset`, covering CPython `BuiltinTest::test_len`
  behavior for supported builtin containers and custom `__len__` methods,
  including propagated exceptions, non-integer and negative return rejection,
  `sys.maxsize + 1` overflow rejection, large negative return rejection, missing
  `__len__`, and argument-count `TypeError`s.
- `RUNTIME_BUILTINS` also includes `cpython_hash_builtin_subset` and
  `cpython_id_builtin_subset` with direct CPython output parity in
  `cpython_hash_id_builtins_diff_subset`, covering CPython
  `BuiltinTest::test_hash`, `::test_invalid_hash_typeerror`, and `::test_id`
  through portable hash invariants, hash TypeError paths, process-specific id
  return typing, and stable identity relationships without comparing concrete
  randomized hash values or process-specific ids.
- `RUNTIME_BUILTINS` also includes
  `cpython_min_max_sum_builtin_subset`, covering CPython
  `BuiltinTest::test_max`, `::test_min`, and `::test_sum` public aggregate
  semantics for positional and iterable inputs, `key=None`, callable `key=`,
  `default=`, `sum(start=...)`, boolean starts, large integer starts, float
  totals, negative-zero float rendering, infinity results checked through
  `math.isinf()`, huge-integer float/complex `OverflowError`,
  complex-constructor summation, complex signed-zero preservation, BadSeq
  exception propagation, and catchable TypeError/ValueError paths for invalid
  arguments and string/bytes/bytearray sum starts. CPython's
  `test_sum_accuracy`
  compensated-floating algorithm remains treated as implementation-specific
  rather than a MiniPython portability requirement.
- `RUNTIME_BUILTINS` also includes
  `cpython_builtin_cmp_absent_subset` with direct CPython output parity in
  `cpython_builtin_cmp_absent_diff_subset`, covering CPython
  `BuiltinTest::test_cmp` by proving `builtins.cmp` is absent and attribute
  access raises `AttributeError`, and by proving bare `cmp(1, 2)` raises
  `NameError`.
- `RUNTIME_BUILTINS` also includes
  expanded `cpython_attribute_introspection_builtins_subset` with direct
  CPython output parity in
  `cpython_attribute_introspection_builtins_diff_subset`, covering CPython
  `BuiltinTest::test_callable`, `::test_getattr`, and `::test_hasattr` public
  behavior, including class-level and inherited `__call__` lookup,
  instance-level `__call__` non-participation in `callable()`, `sys.stdout`
  attribute lookup, `from sys import stdin, stderr, stdout`, missing high
  Unicode-name attribute errors, and the rule that `hasattr()` only suppresses
  `AttributeError` while propagating `SystemExit` and `ValueError`.
- `RUNTIME_BUILTINS` also includes the focused
  `cpython_builtin_callable_public_subset` with direct CPython output parity in
  `cpython_builtin_callable_public_diff_subset`, covering the supported
  `BuiltinTest::test_callable` slice for builtin functions, Python functions,
  classes, bound methods, class-level and inherited `__call__`, ignored
  instance-level `__call__`, and public TypeError arity validation.
- `RUNTIME_BUILTINS` also includes the focused
  `cpython_builtin_getattr_public_subset` with direct CPython output parity in
  `cpython_builtin_getattr_public_diff_subset`, covering the supported
  `BuiltinTest::test_getattr` slice for module attributes, default values,
  instance/class attribute lookup, `sys.stdout`, maximum-valid-Unicode-scalar
  missing-name behavior, and public TypeError arity/name validation.
- `RUNTIME_BUILTINS` also includes the focused
  `cpython_builtin_setattr_delattr_public_subset` with direct CPython output
  parity in `cpython_builtin_setattr_delattr_public_diff_subset`, covering the
  supported `BuiltinTest::test_setattr` / `::test_delattr` slice for module
  attributes, instance attributes, class attributes seen through instances,
  class-attribute deletion, immutable-scalar write failure, and public
  TypeError name/arity validation.
- `RUNTIME_BUILTINS` also includes the focused
  `cpython_builtin_hasattr_public_subset` with direct CPython output parity in
  `cpython_builtin_hasattr_public_diff_subset`, covering the supported
  `BuiltinTest::test_hasattr` slice for present/missing module and instance
  attributes, class attributes seen through instances, maximum-valid-Unicode
  scalar missing names, `AttributeError` suppression, non-`AttributeError`
  propagation, and public TypeError arity/name validation.
- `RUNTIME_BUILTINS` also includes `cpython_vars_dir_builtin_subset` with
  direct CPython output parity in `cpython_vars_dir_builtin_diff_subset`,
  covering `BuiltinTest::test_dir` / `::test_vars` public behavior for local
  namespace introspection, module `__dict__` liveness, class/instance
  dictionaries, tuple-returning `__dir__`, property-backed `__dict__`, TypeError
  boundaries, and list/object `__dir__` ordering.
- `RUNTIME_BUILTINS` also includes `cpython_globals_locals_builtin_subset`,
  `cpython_isinstance_builtin_subset`, `cpython_issubclass_builtin_subset`,
  `cpython_enumerate_zip_sorted_builtin_subset`, `cpython_zip_strict_builtin_subset`,
  `cpython_map_filter_builtin_subset`, and `cpython_abs_builtin_subset`, covering
  public namespace introspection including live module-scope `globals()` /
  `locals()` mapping `clear()` / `copy()` / `get()` / `pop()` / `popitem()` /
  `setdefault()` behavior, recursive self-reference repr placeholders for
  scope-backed namespace mappings, live `dict_keys` / `dict_items` /
  `dict_values` views over scope-backed mappings, stable `reversed()` iteration
  over scope mappings and their keys/items/values views, deletion/reinsertion
  order for scope-backed mappings, PEP 584 union and in-place union for
  scope-backed mappings, rich equality against dict-compatible mappings,
  `dict` class identity and `isinstance(..., dict)` parity for scope-backed
  module namespaces, recursive repr guards for those views, and function-local snapshot
  `copy()` / `get()` / `pop()` / `setdefault()` behavior, class/type relationship checks,
  iterator-producing builtins, strict zip, map/filter, and absolute-value
  behavior used by the sandbox builtin surface. Direct CPython output parity for
  the supported `globals()` / `locals()` subset is tracked by
  `cpython_globals_locals_builtin_diff_subset`; class/type relationship checks are tracked by
  `cpython_isinstance_builtin_diff_subset` and
  `cpython_issubclass_builtin_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_builtin_code_object_subset`, covering first-pass
  `compile(source, filename, mode)` for string and bytes sources in `exec`,
  `eval`, and `single` modes, plus feeding the resulting `code` objects through
  `eval()` and `exec()` with dict-backed globals/locals.
- `RUNTIME_BUILTINS` also includes
  `cpython_builtin_sorted_exact_subset` and
  `cpython_builtin_sorted_exact_diff_subset`, covering all current CPython
  `Lib/test/test_builtin.py::TestSorted` methods through deterministic sorted
  input, source-list preservation, `key=`, `reverse=`, iterable input type
  coverage, and positional/keyword argument rejection.
- `RUNTIME_BUILTINS` also includes
  `cpython_builtin_none_ne_direct_subset` with direct CPython output parity in
  `cpython_builtin_none_ne_direct_diff_subset`, covering CPython
  `BuiltinTest::test___ne__` direct `None.__ne__` behavior plus the inherited
  `object.__eq__` / `object.__ne__` direct-call identity comparison,
  `NoneType` `NotImplemented` fallback for unrelated objects, and CPython's
  direct `object.__ne__` bool result for builtin containers.
- `RUNTIME_BUILTINS` also includes `cpython_object_repr_str_direct_subset` with
  direct CPython output parity in
  `cpython_object_repr_str_direct_diff_subset`, covering direct
  `object.__repr__` and `object.__str__` descriptor lookup and calls, inherited
  instance bindings, `object.__repr__` generic object display,
  `object.__str__` delegation to `__repr__` rather than custom `__str__`, raw
  direct-call return behavior for non-string `__repr__` results, container
  subclass display, and arity/keyword TypeError paths.
- `RUNTIME_BUILTINS` also includes
  `cpython_str_builtin_custom_dunder_subset` with direct CPython output parity
  in `cpython_str_builtin_custom_dunder_diff_subset`, covering ordinary
  `str()` / `print()` / f-string `!s` / default f-string /
  `object.__format__(..., "")` / string `%s` dispatch through class-level
  `__str__`, non-string result rejection, propagated exceptions, ignored
  instance-level `__str__`, custom `__format__` precedence, and empty-format
  behavior for `str` subclasses overriding `__str__`.
- The `BuiltinTest Core Runtime Method Audit` in `cpython_test_manifest.md`
  now pins the direct Rust evidence for 27 scalar, representation, and
  introspection methods that were previously covered only through the broader
  `BuiltinTest` group prose. It keeps the remaining Unicode lone-surrogate,
  and CPython optimizer/code-object gaps explicitly classified as `partial`.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_newline_and_indentation_subset`, covering CPython
  `TestSpecifics` compile acceptance for empty string source, missing trailing
  newlines, CRLF and lone-CR source newlines, mixed newline source with nested
  definitions, nested indented blocks, and first-pass public code-object
  `co_firstlineno` / `co_lines()` behavior for leading blank physical lines.
- `RUNTIME_BUILTINS` also includes warning capture for runtime `compile()` via
  `cpython_compile_specifics_runtime_warning_capture_subset`,
  `cpython_compile_specifics_warning_in_finally_subset`,
  `cpython_compile_specifics_filter_syntax_warnings_by_module_subset`, and
  `cpython_compile_specifics_pep_765_warning_subset`, covering tokenizer
  SyntaxWarnings, identity-literal SyntaxWarnings, finally-body warning
  de-duplication, module-filtered runtime `compile()` warning capture through
  the public `module=` keyword, and PEP 765 finally-control-flow warnings for
  both source and public-AST compile paths.
- `RUNTIME_BUILTINS` also includes the
  `cpython_compile_specifics_*` TestSpecifics expansion for public
  `compile()`, `eval()`, and `exec()` boundaries: SyntaxError propagation for
  invalid assignments, duplicate parameters, invalid keyword targets, bad float
  literals, and invalid parameter ordering; `__debug__` assignment rejection
  plus public `builtins.__debug__` mutation isolation; `None` target rejection in
  `single` and `exec` modes; optimize-level function, class, and module
  docstring behavior across source, public AST, and `single` compile modes,
  including literal-only f-strings not being docstrings; compile-time decimal
  integer source literal limits with `SyntaxError.lineno` plus unlimited
  hexadecimal literals; source -> public AST -> code compile samples,
  including the full local CPython `Lib/test/test_compile.py` self-compile
  sample, with code-object equality independent of filename and second-compile
  `co_filename`; public argument-conversion error paths for invalid `mode`,
  oversized `optimize`, and `dont_inherit` truth-value conversion; import
  grammar acceptance/rejection; and compile-stability shapes for CPython's
  long-expression/loop `test_extended_arg` source behavior, large annotated
  signatures, conditional expressions, dead blocks, and try/except/finally
  control flow.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_invalid_public_ast_subset`, covering public
  compile-from-AST diagnostics for invalid `NamedExpr.target` and
  `TypeAlias.name` node shapes.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_integer_constant_edges_subset`, covering CPython
  `TestSpecifics` public integer-boundary behavior for 64-bit hexadecimal
  `eval()`, unary minus on arbitrary-precision integers, signed minimum-boundary
  decimal literals, and large integer values exposed through function
  `__code__.co_consts`.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_dont_merge_constants_public_subset`, covering
  CPython `TestSpecifics::test_dont_merge_constants` public behavior for
  distinct lambda code-object identity and type-sensitive `co_consts` equality
  across signed zero floats, int-vs-float tuple constants, str-vs-bytes
  constants, signed-zero complex constants, and set-membership constants.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_public_regression_shapes_subset`, covering CPython
  `TestSpecifics` public compile/runtime regressions for conditional-expression
  empty blocks, multiline lambda keyword arguments, duplicate assignment targets,
  dependent stores, while/try/except compile stability, global declarations in
  `except` / `except*` handlers used from `else`, async match plus async dict
  comprehension compile stability, and globals dict-subclass function lookup.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_compile_filename_subset`, covering first-pass
  CPython `compile()` filename behavior for string, bytes, and path-like
  filename objects, memoryview filename rejection, and the public
  `code.co_filename` attribute.
- `RUNTIME_BUILTINS` also includes
  `cpython_compile_specifics_null_terminated_memoryview_subset`, covering
  CPython memoryview source behavior for `compile()`, `eval()`, and `exec()`,
  including sliced memoryviews and embedded-NUL rejection.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_minimal_runtime_subset` and
  `cpython_memoryview_basic_methods_and_release_subset`, covering the first
  CPython `test_memoryview.py` runtime slice for construction across bytes,
  bytearray, and memoryview exporters, CPython-style public constructor
  argument diagnostics, byte iteration, equality with bytes-like objects,
  read-only hashing, writable-hash rejection, one-dimensional byte-view
  attributes, deleted-binding, display-container, augmented-assignment,
  comprehension-frame/target, and expression-temporary exported-view release,
  and direct method-level evidence for
  `test_tobytes`, `test_tolist`, `test_attributes_readonly`,
  `test_attributes_writable`, `test_contextmanager`, `test_release`, and
  `test_toreadonly`. It also covers `hex()`, `count()`, and `index()`, the
  public `release()` lifecycle, context-manager entry/exit behavior,
  released-object `ValueError` checks for supported operations, released
  `str()` / `repr()`, same-object identity through `with ... as`,
  expression-temporary exported-view release after `Pop`, module-level
  comprehension scope/frame release, and reversed iteration.
- `RUNTIME_BUILTINS` also includes `cpython_memoryview_getbuf_fail_subset`,
  covering CPython `test_memoryview.py::AbstractMemoryTests::test_getbuf_fail`
  public constructor rejection for non-buffer objects. Direct CPython diff
  evidence is in `cpython_memoryview_getbuf_fail_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_writable_setitem_subset`, covering the supported
  bytearray-backed writable memoryview slice for shared `bytearray` object
  storage, item assignment, same-size slice assignment, overlapping self-copy,
  read-only assignment errors, deletion errors, bounds checks, one-dimensional
  tuple-key scalar get/set behavior, tuple-key `NotImplementedError` for
  unsupported subviews and multidimensional slicing, mixed tuple-key
  `TypeError`, and no-resize assignment errors. Direct CPython diff evidence is
  split across `cpython_memoryview_writable_setitem_diff_subset` and
  `cpython_memoryview_tuple_key_setitem_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_slice_reference_subset`, covering the first CPython
  `test_memoryview.py` one-dimensional slice-reference behavior: sliced
  memoryviews share the original bytearray buffer, view writes and backing
  writes stay mutually visible, subview slice assignment preserves fixed-size
  semantics, slice-of-slice keeps sharing, negative-stride subviews write back
  through the correct physical byte positions, and readonly status survives
  slicing.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_public_buffer_attributes_subset`, covering supported
  one-dimensional public buffer attributes from CPython `test_memoryview.py`
  and `test_buffer.py`: exporter identity through `obj`, `memoryview(m)`, and
  `toreadonly()`, slice exporter preservation, positive/negative/empty-slice
  `strides`, `c_contiguous`, `f_contiguous`, `contiguous`, bytes-backed
  exporter values, released-attribute `ValueError`, and bytearray identity for
  `is` / `id()` semantics. `cpython_memoryview_slice_and_attributes_diff_subset`
  directly compares the slice-reference and public-attribute surface against
  CPython.
- `RUNTIME_BUILTINS` also has direct CPython diff evidence for the supported
  public memoryview constructor/equality/hash surface in
  `cpython_memoryview_minimal_runtime_diff_subset` and the supported
  method/attribute/release/context-manager surface in
  `cpython_memoryview_methods_release_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_array_b_buffer_subset`, covering the CPython public
  one-byte `array.array('B')` exporter surface for `memoryview()`: writable
  `B`-format attributes, `obj` identity, `tolist()` / `tobytes()`, scalar and
  same-size slice writeback into the original array, subview stride
  preservation, and `toreadonly()` retaining the array exporter.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_array_signed_byte_buffer_subset`, covering the CPython
  public signed-byte `array.array('b')` exporter surface for `memoryview()`:
  raw bytes/bytearray initialization, signed iterable initialization and
  iteration, `b`-format attributes, signed `tolist()` / scalar getitem,
  scalar write range/type errors, same-format memoryview slice assignment, and
  structure mismatch errors for bytes and unsigned-byte views.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_array_non_byte_public_read_subset`, covering the CPython
  public one-dimensional non-byte numeric `array.array` exporter read surface
  for `memoryview()`: element-sized `len()` / `itemsize` / `shape` /
  `strides` / `nbytes` attributes, scalar getitem, `tolist()`, logical
  `tobytes()`, slicing, `c_contiguous`, and byte casts for `h`, `H`, `i`,
  `I`, `f`, and `d` formats.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_array_non_byte_writeback_subset`, covering the CPython
  public one-dimensional same-format writeback surface for non-byte numeric
  `array.array` exporters through `memoryview()`: scalar item assignment,
  contiguous and extended slice assignment, backing-array visibility,
  `__index__` scalar conversion, and structure-mismatch rejection for bytes
  and differently formatted memoryviews.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_module_and_constructor_public_surface_subset`, covering the
  CPython public `array` module constructor surface: `array.typecodes`,
  legacy typecode construction, str-subclass typecode arguments, invalid
  typecode `ValueError`, non-string and wrong-arity constructor `TypeError`s,
  keyword rejection, and zero-length array self-slice / concat / repeat
  behavior without constructing half-initialized arrays.
  Real file descriptors and C buffer/allocator internals remain outside the
  sandbox `array` surface; file-oriented behavior is limited to pure in-memory
  `io.BytesIO` evidence.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_subclass_public_construction_subset`, covering CPython public
  `array.array` subclass construction: ordinary subclasses, custom
  `__init__`, custom `__new__` through `array.array.__new__`, inherited
  storage-backed methods, subclass-specific `repr()`, `isinstance()` /
  `issubclass()` relationships, direct `array.array.__new__` allocation, and
  copy behavior that returns a base array copy.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_one_byte_public_sequence_subset`, covering the supported
  public `array.array('B')` / `array.array('b')` sequence and bytes surface:
  `typecode`, `itemsize`, `len()`, truthiness, `tolist()`, `tobytes()`, scalar
  indexing, slicing, `reversed()`, and direct dunder method calls for the
  currently stored one-byte typecodes.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_short_public_sequence_and_mutation_subset`, extending the
  public `array.array` storage to native-endian signed and unsigned short
  typecodes `h` / `H`: two-byte `itemsize`, element-count `len()` /
  `buffer_info()` length behavior, `tolist()` / `tobytes()` / `repr()`, scalar
  and slice access, iteration, `append()` / `insert()` / `fromlist()`, raw
  `frombytes()` and `fromfile()` validation, `byteswap()`, `pop()` /
  `count()` / `index()`, concat/repeat, `__index__` conversion, overflow
  errors, and array-source constructor conversion through public elements.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_int_public_sequence_and_mutation_subset`, extending the same
  public `array.array` storage to fixed-width native-endian signed and unsigned
  int typecodes `i` / `I`: four-byte `itemsize`, element-count sequence
  behavior, `tolist()` / `tobytes()` / `repr()`, scalar and slice access,
  mutation methods, raw byte validation, `byteswap()`, concat/repeat,
  `fromfile()` short-read behavior, `__index__` conversion, signed and
  unsigned overflow errors, and array-source constructor conversion through
  public elements.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_long_long_public_sequence_and_mutation_subset`, extending the
  same public `array.array` storage to fixed-width native-endian signed and
  unsigned long long typecodes `q` / `Q`: eight-byte `itemsize`, element-count
  sequence behavior, `tolist()` / `tobytes()` / `repr()`, scalar and slice
  access, mutation methods, raw byte validation, `byteswap()`, concat/repeat,
  `fromfile()` short-read behavior, `__index__` conversion, signed and
  unsigned overflow errors, BigInt-backed `Q` values above `i64::MAX`, and
  array-source constructor conversion through public elements.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_native_long_public_sequence_and_mutation_subset`, extending the
  same public `array.array` storage to native C long signed and unsigned
  typecodes `l` / `L`: platform-native `itemsize`, element-count sequence
  behavior, raw byte round-trips and validation, `byteswap()`, mutation methods,
  concat/repeat, `fromfile()` short reads, `__index__` conversion,
  signed/unsigned overflow errors, BigInt-backed unsigned values above
  `i64::MAX` on 64-bit C long platforms, and array-source constructor
  conversion through public elements.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_float_public_sequence_and_mutation_subset`, extending the same
  public `array.array` storage to native-endian float and double typecodes
  `f` / `d`: four- and eight-byte `itemsize`, element-count sequence behavior,
  raw byte round-trips and validation, `byteswap()`, mutation methods,
  concat/repeat, `fromfile()` short reads, `__float__` conversion before
  `__index__` fallback, conversion error propagation, and array-source
  constructor conversion through public elements.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_unicode_public_sequence_and_mutation_subset`, extending the
  same public `array.array` storage to native-endian Unicode typecodes `u` /
  `w`: four-byte `itemsize`, string constructor initialization, `tolist()` /
  `tobytes()` / `repr()` / `tounicode()`, scalar and slice access, mutation
  methods, `fromunicode()`, raw byte round-trips and validation, `byteswap()`
  invalid-code-point errors, concat/repeat, `fromfile()` short reads, invalid
  item type errors, and array-source constructor conversion through public
  elements.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_one_byte_public_mutation_methods_subset`, covering the
  supported public `array.array('B')` / `array.array('b')` mutable sequence
  methods: `append()`, `insert()`, `extend()`, `pop()`, `reverse()`,
  `count()`, `index()`, `remove()`, `fromlist()`, `frombytes()`, and
  `clear()`, including one-byte overflow errors, empty-pop errors, cross-kind
  `extend()` rejection, type checks for `fromlist()` / `frombytes()`, and
  CPython's empty-array `repr()` shape. Direct CPython diff evidence is in
  `cpython_array_one_byte_public_mutation_methods_diff_subset`, with
  `array.clear()` additionally pinned by focused
  `cpython_array_one_byte_public_clear_subset` and gated
  `cpython_array_one_byte_public_clear_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_one_byte_public_subscript_mutation_subset`, covering the
  supported public `array.array('B')` / `array.array('b')` mutable subscript
  surface: scalar assignment with negative and `__index__` indices, direct
  `__setitem__()`, contiguous and extended slice assignment, same-kind array
  RHS validation, extended-slice length errors, direct `__delitem__()`,
  contiguous and extended slice deletion, and CPython's array assignment index
  error shape for the supported one-byte typecodes.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_one_byte_public_copy_byteswap_compare_subset`, covering the
  supported public `array.array('B')` / `array.array('b')` copy and comparison
  surface: `__copy__()`, `__deepcopy__()`, `copy.copy()`, `copy.deepcopy()`,
  independent copied storage, one-byte `byteswap()` as a no-op, array-vs-array
  numeric element equality and ordering across `B` / `b`, direct comparison
  dunders returning `NotImplemented` for non-array operands, and TypeError
  order comparisons against `list` / `bytes`.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_one_byte_public_concat_repeat_subset`, covering the supported
  public `array.array('B')` / `array.array('b')` sequence operator surface:
  same-kind concatenation through `+` and `__add__()`, cross-kind and non-array
  rejection, repetition through `*`, reflected `*`, `__mul__()`, and
  `__rmul__()`, `__index__`-driven repeat counts, zero/negative repeat counts,
  operator versus direct-dunder non-integer diagnostics, and identity-preserving
  `+=`, `__iadd__()`, `*=`, and `__imul__()`.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_one_byte_public_buffer_info_subset`, covering the supported
  public `array.array('B')` / `array.array('b')` `buffer_info()` surface:
  method visibility, two-item tuple shape, integer nonzero address surrogate,
  current element count after mutation, bound-method dispatch, and arity
  rejection without depending on CPython's exact buffer address.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_one_byte_public_unicode_method_rejection_subset`, covering the
  supported public `array.array('B')` / `array.array('b')` Unicode helper
  rejection surface: `fromunicode()` / `tounicode()` method visibility,
  non-unicode-array `ValueError`s, arity and non-string argument TypeErrors,
  and receiver preservation after rejected calls.
- `RUNTIME_BUILTINS` also includes
  `cpython_array_one_byte_public_file_methods_subset`, covering the supported
  public `array.array('B')` / `array.array('b')` `tofile()` / `fromfile()`
  surface through in-memory `io.BytesIO`: method visibility, writing raw bytes
  through `write()`, reading exact and short byte counts through `read()`,
  partial append before `EOFError`, zero-count reads, negative and non-integer
  count rejection, non-bytes `read()` result rejection, and the `BytesIO`
  `read()` / `write()` / `getvalue()` methods needed for this public protocol.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_cast_one_byte_format_subset`, covering the supported
  public one-byte `memoryview.cast()` surface from CPython
  `test_memoryview.py`: `B` / `b` / `c` formats, positional and keyword
  `format` / one-dimensional `shape` binding, format preservation through
  `memoryview(m)`, slicing, and `toreadonly()`, `c`-format scalar/list/reversed
  values, `c`-format item and slice assignment, bytes-like membership, and
  non-contiguous cast rejection. `cpython_memoryview_cast_one_byte_format_diff_subset`
  directly compares this one-byte format surface against CPython.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_getitem_index_count_compare_subset`, covering CPython
  `test_memoryview.py` getitem/index/count/compare behavior for the supported
  one-dimensional bytes-like surface: integer and negative indexing, invalid
  index exception classes, `index()` `start`/`stop` hits and misses, `count()`
  over logical view contents, equality with bytes/bytearray/memoryview,
  inequality with non-buffer objects, and TypeError for unsupported ordered
  comparisons. The matching `cpython_memoryview_count_index_diff_subset`
  differential is capability-gated for CPython oracles that expose
  `memoryview.count()` / `memoryview.index()`.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_hex_separator_subset`, covering CPython
  `test_memoryview.py` logical-byte `hex()` behavior on reversed
  non-contiguous views, separator positional/keyword binding, positive and
  negative `bytes_per_sep` grouping, and overlarge grouping counts. Direct
  CPython diff evidence is in `cpython_memoryview_hex_separator_diff_subset`.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_hex_reentrant_release_subset`, covering CPython
  `test_memoryview.py::AbstractMemoryTests::test_hex_use_after_free` for
  supported bytearray-backed views: ordinary released-view `hex()` still raises
  `ValueError`, while separator conversion keeps the exporter resize-locked so
  re-entrant view release plus bytearray clearing raises `BufferError`.
  `cpython_memoryview_hex_released_view_diff_subset` directly compares the
  released-view `hex()` behavior, and gated
  `cpython_memoryview_hex_reentrant_release_diff_subset` evidence directly
  compares the stricter re-entrant resize guard when the CPython oracle has the
  current BufferError fix. Older CPython oracles that still accept the bytearray
  clear path are explicitly skipped.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_copy_rejection_subset`, covering CPython
  `test_memoryview.py::OtherTest::test_copy` for public `copy.copy()` and
  `copy.deepcopy()` rejection of supported read-only and writable memoryview objects.
  `cpython_memoryview_rejection_and_hash_diff_subset` directly compares the
  copy rejection, deepcopy rejection, pickle rejection, and hash/release-cache
  surfaces against CPython.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_pickle_rejection_subset`, covering CPython
  `test_memoryview.py::OtherTest::test_pickle` for public `pickle.dumps()`
  rejection of supported memoryview objects across every exposed pickle
  protocol, including nested-container rejection.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_hash_release_cache_subset`, covering CPython
  `test_memoryview.py::AbstractMemoryTests::test_hash` and
  `test_hash_writable` for supported one-dimensional views: read-only
  memoryview hash equality with bytes, cached hash availability after release,
  first hash after release `ValueError`, and writable-view hash rejection.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_release_during_index_subset`, covering the supported
  one-dimensional public slice of CPython
  `test_memoryview.py::OtherTest::test_use_released_memory`: index objects
  whose `__index__` releases the source view make scalar getitem and item/slice
  assignment fail with released-view `ValueError`, slice getitem preserves a
  live subview of the original bytes, RHS byte conversion through `__index__`
  cannot write after release, and the bound `__getitem__` / `__setitem__`
  methods share the same VM behavior. `cpython_memoryview_release_during_index_read_diff_subset`
  directly compares the read/getitem portion against CPython; the stricter
  write-after-release rejection remains MiniPython subset evidence.
- `RUNTIME_BUILTINS` also includes
  `cpython_memoryview_bytesio_readinto_subset`, covering the in-memory
  CPython `test_memoryview.py::AbstractMemoryTests::test_writable_readonly`
  slice where `io.BytesIO.readinto()` fills writable `bytearray` and
  bytearray-backed `memoryview` targets, fills contiguous sliced writable
  memoryviews, rejects read-only bytes-backed and non-contiguous writable
  memoryview targets for both `readinto()` and `readinto1()`, advances the
  stream position, and accepts `initial_bytes=`.
  `cpython_memoryview_bytesio_readinto_diff_subset` directly compares this
  in-memory protocol surface against CPython.
- `RUNTIME_BUILTINS` also includes `cpython_memoryview_weakref_live_subset`,
  covering the live-reference slice of CPython
  `test_memoryview.py::AbstractMemoryTests::test_weakref`: bytes- and
  bytearray-backed memoryviews can be wrapped by `weakref.ref()`, refs remain
  callable, calls return the live view, `ReferenceType` classification works,
  callback arguments are accepted, and `callback=None` is valid at
  construction time. `cpython_memoryview_weakref_live_diff_subset` directly
  compares this live-reference construction surface against CPython.
- `RUNTIME_BUILTINS` also includes
  `cpython_weakref_ref_supported_target_matrix_subset`, covering first-pass
  CPython public `weakref.ref()` construction behavior: unsupported built-in
  instances such as scalars, bytes-like containers, list, dict, tuple, range,
  and bare `object()` are rejected; supported functions, classes, ordinary
  instances, `__weakref__` slot instances, set, frozenset, memoryview, and
  builtin type objects are accepted; and keyword arguments to `weakref.ref()`
  raise `TypeError`.
- `RUNTIME_BUILTINS` also includes
  `cpython_weakref_ref_callback_attribute_subset`, covering public
  `weakref.ref.__callback__` metadata for omitted callback, function callback,
  and `callback=None`, plus readonly assignment behavior.
- `RUNTIME_BUILTINS` also includes
  `cpython_weakref_ref_type_identity_subset`, covering canonical
  `weakref.ReferenceType` identity through `type(ref)`, `ref.__class__`,
  `object.__getattribute__(ref, "__class__")`, and `isinstance(ref, type(ref))`.
- `RUNTIME_BUILTINS` also includes
  `cpython_weakref_proxy_type_aliases_subset`, covering public
  `weakref.ProxyType` / `weakref.CallableProxyType` type aliases, their
  `weakref` module/name/qualname metadata, the `weakref.ProxyTypes` tuple, and
  matching `_weakref` aliases without `_weakref.ProxyTypes`.
- `RUNTIME_BUILTINS` also includes
  `cpython_weakref_proxy_live_forwarding_subset`, covering first-pass live
  `weakref.proxy()` behavior: proxy construction through `weakref` /
  `_weakref`, `ProxyType` versus `CallableProxyType` classification,
  `weakref.ProxyTypes` membership, target `__class__` forwarding, attribute
  read/write/delete forwarding, bound-method forwarding, subscript
  get/set/delete forwarding through `__getitem__` / `__setitem__` /
  `__delitem__`, `operator.index()` / `__index__` forwarding, `__bytes__` /
  `__dir__` forwarding, floor-division and matrix-multiply special method
  forwarding including in-place variants, `__iter__`, `__reversed__`, and
  `__bool__` forwarding, callable proxy positional/keyword calls, built-in
  `list` subclass target truthiness, `len()`, iteration, membership, method
  forwarding, item and slice mutation, and reversed iteration, positional
  `callback=None`, keyword-argument rejection, and unhashable proxy behavior.
- `RUNTIME_BUILTINS` also includes
  `cpython_weakref_ref_live_repr_subset`, covering public live `weakref.ref`
  `repr()` / `str()` shape for ordinary instances, class objects, functions,
  sets, and frozensets without asserting exact memory addresses.
- `RUNTIME_BUILTINS` also includes
  `cpython_weakref_ref_dunder_methods_subset`, covering direct public access to
  live `weakref.ref` `__repr__`, `__str__`, `__hash__`, `__call__`, `__eq__`,
  and `__ne__` methods without depending on CPython's internal method-wrapper
  type; direct equality methods return `NotImplemented` for non-weakref
  operands like CPython.
- `RUNTIME_BUILTINS` also includes
  `cpython_weakref_ref_live_compare_hash_subset`, covering live
  `weakref.ref` equality/inequality through referent equality, callback-agnostic
  equality, referent hash reuse, and same-live-target set/dict key behavior.
- `RUNTIME_BUILTINS` also includes expanded `cpython_eval_builtin_subset` and
  `cpython_exec_builtin_subset` coverage for `source`, `globals`, and `locals`
  keyword binding, bytes-source execution through the same decoding path as
  `compile()`, CPython-style globals preparation before source compile/type
  errors, and exec writeback of assignments that happen before runtime
  exceptions. It also covers same-mapping globals/locals behavior for
  `eval(source, g, g)` named-expression writes and `exec(source, g, g)`
  ordinary/global assignment writes, the supported public
  `BuiltinTest::test_general_eval` general-mapping locals behavior for
  `__getitem__`, `keys()` / `dir()`, `globals()`, `locals()`, nested lookups,
  dict subclasses, and invalid mapping shapes, plus
  `BuiltinTest::test_exec_redirected` behavior where `sys.stdout = None` still
  leaves `exec('a')` raising a catchable `NameError`.
- `RUNTIME_BUILTINS` also includes
  `cpython_eval_exec_builtins_mapping_subset`, covering first-pass
  `globals['__builtins__']` lookup for restricted builtin dictionaries and
  exact-dict `mappingproxy` builtin mappings, default builtins injection into
  supplied eval/exec globals, dict-subclass builtin mappings, custom and default
  `__import__` lookup for import statements, and dict-subclass `__getitem__`
  exception propagation for globals and builtin mappings. Direct CPython output
  parity is tracked by `cpython_eval_exec_builtins_mapping_diff_subset`. It also covers the
  supported public part of `BuiltinTest::test_exec_globals_frozen`:
  `builtins.__build_class__` is exposed, empty explicit builtins make class
  creation raise catchable `NameError`, empty read-only dict-subclass builtins
  take the same `__build_class__` error path, custom read-only builtins can
  provide `__build_class__`, writes into that builtins mapping call the dict
  subclass `__setitem__` error path, and read-only dict-subclass globals
  writeback calls `__setitem__` only for the changed global name. The same
  subset now covers `BuiltinTest::test_eval_builtins_mapping_reduce` by checking
  list/tuple iterator `__reduce__()` results and lookup of `iter` through the
  active mappingproxy builtins.
- `RUNTIME_BUILTINS` also includes `cpython_exec_closure_subset`, covering
  CPython `BuiltinTest::test_exec_closure` for executable function `__code__`
  objects, `co_freevars`, function `__closure__` cells, manual
  `types.CellType(value)` cells, and `exec(..., closure=...)` validation and
  nonlocal writeback.
- `RUNTIME_BUILTINS` also includes
  `cpython_exec_filter_syntax_warnings_by_module_subset`, covering CPython
  `BuiltinTest::test_exec_filter_syntax_warnings_by_module` for source executed
  through `exec()`, six compile-time `SyntaxWarning` records with `<string>`
  filenames, and warning module filtering driven by explicit globals
  `__name__`.
- `IMPORT_RUNTIME` includes `cpython_import_sys_modules_cache_subset`,
  covering `sys.modules` import caching, direct cache replacement, `None`
  import-abort sentinels, dotted-module parent binding, and polluted-parent
  rejection for dotted imports. It also covers builtin `__import__` `fromlist`
  truthiness over false/true scalar and container values plus user-defined
  `__bool__` / `__len__` results, plus `level` argument conversion for false
  bool, negative integers, and representative non-integer error messages.
  `cpython_import_builtin_subset` covers CPython `BuiltinTest::test_import`
  ordinary builtin imports for `sys`, `time`, and `string`, keyword `name` /
  `level` binding, missing-module `ModuleNotFoundError`, non-string name
  `TypeError`, empty-name `ValueError`, duplicate-name `TypeError`, and embedded
  null module-name rejection.
  Runtime import errors now flow through the VM exception path, so
  `ModuleNotFoundError` can be caught by `try` / `except`; focused language
  tests also cover sandboxed virtual source modules, package modules, and
  package-relative imports without VM host filesystem access. Public sandbox
  directory loading now maps `.py` files under an approved root into the same
  virtual module table while rejecting invalid module names and symlink escapes.
  Sandbox callers can also deny all builtin stdlib imports or allow only named
  stdlib modules; the policy propagates into virtual module execution and
  applies before returning non-virtual `sys.modules` cache entries.
  The `sys` sandbox surface remains limited to in-memory metadata, `modules`,
  placeholder stdio objects, numeric/runtime limits, frame inspection, and
  breakpoint hook metadata; Real argv/process state and real stdin/stdout/stderr streams
  stay outside the product surface, as do implementation refcount/GC/debug APIs.
  `out_of_scope_host_io_network_and_process_surfaces_stay_unavailable` guards
  the default blocked runtime surface so host I/O (`open()`, `input()` and
  host TTY behavior plus non-`None` `print(file=...)` targets), network and
  process modules (`asyncio`, `http`, `ssl`, `socket`, `subprocess`, `signal`,
  `threading`, `pty`, `urllib`, and `multiprocessing`), C ABI / extension
  modules (`_ssl`, `_socket`, `_ctypes`, and `_testcapi`), CPython-internal
  contracts (`co_stacksize`, refcount, GC-tracking, opcode identity, and
  specialization), locale-sensitive behavior, default `pdb` / `breakpoint`
  integration, and process/environment side effects stay outside the sandbox
  product surface unless the scope is explicitly promoted.

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
  variants. The corresponding `cpython_tokenize_exact_type_diff_subset` keeps
  the executable operator-token surface tied to direct CPython output parity.
  `cpython_tokenize_selector_and_method_subset` ports CPython's
  selector and decorator/method tokenizer span examples. The corresponding
  `cpython_tokenize_selector_and_method_diff_subset` keeps those
  selector/decorator source shapes tied to direct CPython execution parity.
- `cpython_tokenize_async_await_subset` ports representative CPython
  `test_async` tokenizer source shapes while preserving MiniPython's
  parser-ready `async` / `await` keyword token variants. The corresponding
  `cpython_tokenize_async_await_diff_subset` keeps valid async/await source
  shapes tied to direct CPython execution parity.
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
| `NAME` | supported | `lexes_print_number`, `lexes_unicode_identifiers`, `lexes_underscore_relative_import_module_after_dot`, `cpython_ast_assignment_and_name_load_subset`, `cpython_unicode_identifier_subset`, `cpython_tokenize_spanned_tokens_subset` including keyword/name spans, `cpython_tokenize_formfeed_whitespace_subset` / `cpython_tokenize_formfeed_whitespace_diff_subset` including formfeed-separated name/operator tokens |
| `NUMBER` | partial | `lexes_float_literals`, `lexes_imaginary_literals`, `lexes_number_separators`, `lexes_large_integer_literals`, `lexes_number_keyword_boundaries`, `rejects_invalid_number_separators`, `lexes_prefixed_integer_literals`, `rejects_invalid_prefixed_integer_literals`, `rejects_nonzero_leading_decimal_zeroes`, `cpython_tokenize_valid_number_token_stream_subset` covering the migrated CPython `test_int`, `test_long`, and `test_float` raw-token text/spans for integer/operator, large-integer, and float forms, including trailing-dot, uppercase exponent, and large exponent float spellings, `cpython_tokenize_underscore_number_token_stream_subset` covering CPython `test_underscore_literals` raw NUMBER text preservation/rejection behavior, `cpython_grammar_prefixed_integer_literals_subset`, `cpython_float_literal_forms_subset`, `cpython_float_exponent_tokenization_subset`, `cpython_end_of_numerical_literals_subset` including adjacent-name, hexadecimal adjacent-name, and fraction-slash spans, `cpython_tokenize_invalid_python_token_stream_subset` including tokenizer-only `2sin(x)` number/name split, `01234`, `0_7`, and `09_99` leading-zero NUMBER tokenization, invalid decimal underscore/exponent token splitting, and CPython `test_invalid_syntax` binary/octal/hex token splitting for invalid digits, invalid underscore suffixes, and missing prefixed-integer digits, `cpython_numeric_literal_warning_subset` including CPython `test_end_of_numerical_literals` keyword-boundary warnings, `test_tokenizer_fstring_warning_in_first_line` binary-boundary warning source `0b1and 2`, and warning-as-error spans for decimal, imaginary, binary, octal, and hexadecimal literals, `cpython_valid_underscore_number_literals_subset`, `cpython_large_integer_literals_subset` including CPython `test_long_integers` prefix-case and huge-literal forms, `cpython_integer_bit_methods_subset`, `cpython_integer_ratio_and_component_methods_subset`, `cpython_float_string_underscore_subset`, `cpython_float_conversion_protocol_subset`, `cpython_float_from_number_subset`, `cpython_float_keywords_in_subclass_subset`, `cpython_float_containment_subset`, `cpython_float_floor_ceil_subset`, `cpython_float_mod_signed_zero_subset`, `cpython_float_pow_special_cases_subset`, `cpython_float_ratio_and_component_methods_subset`, `cpython_float_hex_fromhex_first_pass_subset`, `cpython_float_fromhex_accepted_variants_subset`, `cpython_float_fromhex_overflow_zero_underflow_subset`, `cpython_float_fromhex_rounding_boundaries_subset`, `cpython_float_fromhex_bpo44954_regression_subset`, `cpython_float_hex_fromhex_invalid_inputs_subset`, `cpython_float_hex_fromhex_ends_whitespace_subset`, `cpython_float_hex_fromhex_roundtrip_matrix_subset`, `cpython_float_hex_fromhex_subclass_subset`, `cpython_integer_base_builtins_subset`, `cpython_int_constructor_base_conversion_subset` covering CPython `int()` constructor underscore parsing, base 2-through-36 big-integer conversions for `2**32` and `2**32 + 1`, base-limit errors, non-integer base rejection, and `__index__`-supplied out-of-range base diagnostics, `cpython_int_constructor_error_message_subset` covering CPython `int()` invalid-literal diagnostics for float-looking strings, non-ASCII strings, embedded whitespace/NUL, explicit-base NUL strings, and NUL/non-UTF-8 bytes while leaving lone-surrogate rows outside MiniPython's executable Rust string model, `cpython_int_max_str_digits_runtime_subset` covering CPython `test_int.py::IntStrDigitLimitsTests` runtime digit-limit behavior for `int()`, `str()`, top-level/container `repr()`, sign/space padding, underscores, and unlimited power-of-two bases, `cpython_int_max_str_digits_formatting_subset` extending that digit-limit coverage to `format()`, f-strings, `str.format()`, and old-style `%s` / `%r` / `%a` / `%d` / `%i` / `%u` decimal formatting while preserving unlimited hexadecimal formatting, `cpython_compile_specifics_int_literals_too_long_subset` covering `compile()`-time decimal integer literal limits, offending-line `SyntaxError.lineno`, and unlimited hexadecimal source literals, `cpython_divmod_builtin_subset`, `cpython_round_builtin_subset`, `cpython_pow_builtin_subset`, `cpython_bad_numerical_literals_subset` including representative bad-literal spans, `cpython_syntax_error_message_parity_diff_subset`, `cpython_grammar_imaginary_literals_subset`, `runs_float_literals`, `runs_imaginary_literals`, `runs_prefixed_integer_literals` |
| `STRING` | partial | `lexes_string`, `lexes_string_line_continuations`, `lexes_string_octal_escapes`, `lexes_unicode_name_escapes`, `lexes_unicode_name_alias_escapes`, `lexes_bytes_literals`, `lexes_bytes_line_continuations`, `lexes_cpython_string_prefix_matrix`, `rejects_non_ascii_bytes_literals`, `rejects_cpython_unterminated_string_forms`, `rejects_cpython_invalid_string_escape_forms`, `rejects_cpython_unterminated_interpolated_string_forms`, `lexes_f_string_parts`, `lexes_f_string_escaped_brace_literals`, `lexes_f_string_backslash_before_doubled_braces`, `lexes_f_string_line_continuations`, `lexes_f_string_format_specs`, `lexes_raw_and_non_raw_f_string_format_spec_escapes`, `lexes_f_string_debug_expressions`, `lexes_raw_f_string_literals_and_empty_format_specs`, `cpython_tokenize_string_span_subset` including quote, embedded quote payloads, ordinary single- and double-quoted string expressions, raw-prefix matrix, `u`/`U` prefixes, `b`/`B` bytes prefixes, single- and double-quoted `br`/`rb` raw-bytes prefix matrix, split string/name/string tokenization, adjacent same-line string tokens before parser concatenation, multiline line-continuation, unicode-prefixed line-continuation, triple-quoted, unicode-prefixed triple-quoted, single-quoted raw bytes, triple-quoted raw bytes, escaped CRLF text inside a string token, and indented non-ASCII triple-quoted source spans, `cpython_string_literal_and_concat_subset`, `cpython_string_startswith_endswith_diff_subset` / `cpython_string_startswith_endswith_subset`, `cpython_string_find_index_diff_subset` / `cpython_string_find_index_subset`, `cpython_string_count_case_diff_subset` / `cpython_string_count_case_subset`, `cpython_string_capitalize_title_swapcase_casefold_diff_subset` / `cpython_string_capitalize_title_swapcase_casefold_subset`, `cpython_string_predicate_methods_diff_subset` / `cpython_string_predicate_methods_subset` including the CPython `isascii()` alignment matrix, `cpython_string_identifier_printable_diff_subset` / `cpython_string_identifier_printable_subset`, `cpython_string_expandtabs_diff_subset` / `cpython_string_expandtabs_subset`, `cpython_string_splitlines_diff_subset` / `cpython_string_splitlines_subset`, `cpython_string_replace_diff_subset` / `cpython_string_replace_subset`, `cpython_string_remove_affix_diff_subset` / `cpython_string_remove_affix_subset`, `cpython_string_split_rsplit_diff_subset` / `cpython_string_split_rsplit_subset`, `cpython_string_strip_diff_subset` / `cpython_string_strip_subset`, `cpython_string_alignment_and_zfill_diff_subset` / `cpython_string_alignment_and_zfill_subset`, `cpython_string_partition_rpartition_diff_subset` / `cpython_string_partition_rpartition_subset`, `cpython_string_join_diff_subset` / `cpython_string_join_subset`, `cpython_string_line_continuation_subset`, `cpython_string_octal_escape_subset`, `cpython_string_escape_warning_subset` including warning-as-error behavior, `cpython_string_invalid_escape_ascii_table_subset`, `cpython_string_escape_warning_location_subset`, `cpython_f_string_escape_warning_subset` including warning-as-error behavior, `cpython_unicode_name_escape_subset`, `cpython_bytes_literal_subset`, `cpython_string_prefix_matrix_subset`, `cpython_invalid_string_prefix_matrix_subset` adapted from CPython `test_invalid_string_prefixes`, `cpython_invalid_string_literal_subset` including CPython `test_invalid_syntax` unterminated ordinary, bytes, one-line triple, and multiline triple-quoted string spans plus non-ASCII bytes literal spans, `cpython_string_and_tstring_helper_rules_subset`, `cpython_f_string_basic_subset`, `cpython_f_string_triple_quoted_expression_subset`, `cpython_f_string_many_expressions_subset`, `cpython_f_string_format_specifier_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_raw_f_string_format_spec_subset`, `cpython_invalid_f_string_syntax_subset`, `cpython_invalid_t_string_syntax_subset`, `runs_python_string_literal_forms`, `runs_python_bytes_literal_forms`, `runs_f_strings`, `runs_f_string_expressions` |
| `STRING_RUNTIME` | partial | `cpython_ascii_builtin_diff_subset` and `cpython_ascii_builtin_subset` cover first-pass `ascii()` builtin behavior, CPython-style non-ASCII repr escaping, f-string `!a`, and recursive list/dict repr placeholders; `cpython_chr_ord_builtin_diff_subset` and `cpython_chr_ord_builtin_subset` cover `chr()` and `ord()` builtins for ordinary and CPython boundary Unicode scalar values, one-character/one-byte inputs, and negative, out-of-range, and very large integer `chr()` `ValueError` paths; `cpython_old_style_string_percent_format_subset` covers first-pass old-style `%` string formatting for `%s`, `%r`, `%a`, `%%`, `%d`, `%i`, `%u`, `%x`, `%X`, `%o`, `%c`, `%f`, `%F`, `%e`, `%E`, `%g`, `%G`, ignored `h` / `l` / `L` length modifiers, tuple argument consumption, `%(key)` mapping arguments, static and dynamic `*` flags/width/precision for text, integer, and float conversions, mapping-to-positional mixing errors, extra-argument errors, non-integer `*` errors, mapping-key `*` errors, non-real float-format errors, isolated/unsupported-format `ValueError` paths, and out-of-range `%c` errors; `cpython_old_style_string_percent_repr_protocol_subset` covers `%r` / `%a` user `__repr__` dispatch, propagated exceptions, non-string result TypeErrors, and ASCII escaping with precision; `cpython_old_style_percent_c_index_protocol_subset` covers string/bytes/bytearray `%c` `__index__` dispatch, propagated exceptions, non-string index-result TypeErrors, and no-`__index__` TypeErrors; `cpython_string_format_and_format_map_subset` covers first-pass `str.format()` and `str.format_map()` literal rendering, escaped braces, positional/automatic/keyword/mapping fields, simple attribute and item lookup, conversions, and existing mini-format specs; `cpython_f_string_contextual_runtime_subset` covers f-string truthiness, empty format specs, f-string indexing versus `str.format()` field indexing, loop evaluation, and nested-quote dict subscripts; `cpython_f_string_format_error_subset` covers CPython-style f-string formatting TypeErrors and ValueErrors for unsupported object specs and unknown scalar format codes; `cpython_string_maketrans_translate_diff_subset` / `cpython_string_maketrans_translate_subset` cover `str.maketrans()` and `str.translate()` dictionary translation, deletion, integer/string replacements, non-ASCII replacements, and error paths; `cpython_string_bytes_codec_diff_subset` / `cpython_string_bytes_codec_subset` cover first-pass `str.encode()`, `bytes.decode()`, `bytearray.decode()`, `str(bytes, encoding)`, `bytes(str, encoding)`, `bytearray(str, encoding)`, codec constructor keyword behavior, `encoding_rs` label fallback with `cp1251` and `cp1252`, and CPython-style strict / `ignore` / `replace` behavior for undefined codec bytes; `cpython_bytes_iterable_constructor_diff_subset` / `cpython_bytes_iterable_constructor_subset`, `cpython_bytes_mutating_list_constructor_diff_subset` / `cpython_bytes_mutating_list_constructor_subset`, and `cpython_bytes_constructor_exception_diff_subset` / `cpython_bytes_constructor_exception_subset` cover bytes/bytearray construction from supported integer iterables, `__getitem__` sequences, live mutating lists, and `__index__` elements, including invalid item classes, source-list clear/append behavior during item conversion, and propagation of exceptions from `__index__` / `__iter__`; `cpython_bytes_dunder_bytes_and_blocking_subset` covers `bytes()` `__bytes__` dispatch, bytes-subclass result preservation, non-bytes result rejection, `__bytes__` precedence over `__index__`, and `__bytes__ = None` fallback blocking; `cpython_bytes_bytearray_index_error_and_hash_subset` covers invalid-index TypeError messages for bytes and bytearray plus bytearray unhashability; `cpython_bytes_constructor_concat_repeat_contains_diff_subset` / `cpython_bytes_constructor_concat_repeat_contains_subset` cover bytes/bytearray integer-length construction, mixed bytes/bytearray concatenation result types, repetition with zero/negative counts, repeat TypeErrors, and membership over integer and bytes-like needles; `cpython_bytes_compare_slice_reversed_subset` covers bytes/bytearray lexicographic comparisons, all CPython byte-order comparison-against-`str` rows, reversed byte iteration, ordinary slicing, and CPython's extended-slice matrix against list slicing; `cpython_bytes_bytearray_assorted_public_subset` covers bytearray construction from memoryview-backed bytes and both operand orders for bytes/bytearray rich comparison; `cpython_bytearray_regexps_subset` covers the supported `re.findall()` bytes-pattern subset for ASCII `\w+` over bytes-like subjects; `cpython_bytes_search_compare_slice_diff_subset` / `cpython_bytes_search_methods_subset` cover bytes/bytearray `count()`, `find()`, `rfind()`, `index()`, and `rindex()` over bytes-like and integer byte needles with start/stop bounds including `None`; `cpython_bytes_search_bounds_index_diff_subset` / `cpython_bytes_search_bounds_index_subset` cover Python-level `__index__` conversion and exception propagation for bytes/bytearray search and prefix/suffix `start` / `stop` bounds; `cpython_bytes_prefix_suffix_methods_diff_subset` / `cpython_bytes_prefix_suffix_methods_subset` cover bytes/bytearray `startswith()` and `endswith()` over bytes-like and tuple prefixes/suffixes with start/stop bounds including `None`; `cpython_bytes_prefix_suffix_typeerror_messages_subset` covers exact TypeError diagnostics for invalid top-level prefixes/suffixes versus invalid tuple candidates; `cpython_bytes_split_rsplit_methods_diff_subset` / `cpython_bytes_split_rsplit_methods_subset` cover bytes/bytearray `split()` and `rsplit()` over default ASCII whitespace, explicit bytes-like separators, `maxsplit`, keyword arguments, receiver-driven result types, Unicode-whitespace boundary behavior, empty separators, and representative TypeError paths; `cpython_bytes_splitlines_methods_diff_subset` / `cpython_bytes_splitlines_methods_subset` cover bytes/bytearray `splitlines()` over CR, LF, and CRLF boundaries, `keepends`, receiver-driven result types, bytes-specific non-splitting behavior for Unicode text line separators, and representative TypeError paths; `cpython_bytes_ascii_case_predicate_methods_diff_subset` / `cpython_bytes_ascii_case_predicate_methods_subset` cover bytes/bytearray ASCII case transforms and predicates, non-ASCII byte preservation for case transforms, empty-input predicate behavior, receiver-driven result types, and representative extra-argument TypeErrors; `cpython_bytes_expandtabs_zfill_methods_diff_subset` / `cpython_bytes_expandtabs_zfill_methods_subset` cover bytes/bytearray `expandtabs()` byte-level tab expansion with `tabsize` and `zfill()` sign-aware zero fill, receiver-driven result types, builtin type `dir()` visibility, and representative TypeErrors; `cpython_bytes_more_method_typeerror_messages_subset` covers exact bytes/bytearray ASCII case/predicate, splitlines, expandtabs, zfill, removeprefix, and removesuffix TypeError diagnostics for unbound, arity, and non-integer conversion calls; `cpython_bytes_core_method_typeerror_messages_subset` covers exact bytes/bytearray split/rsplit, search, prefix/suffix, strip, alignment, partition, and replace TypeError diagnostics for unbound, arity, and integer-conversion calls; `cpython_bytes_strip_methods_diff_subset` / `cpython_bytes_strip_methods_subset` cover bytes/bytearray `strip()`, `lstrip()`, and `rstrip()` over default ASCII whitespace and explicit bytes-like strip sets, preserving receiver-driven result types and representative TypeErrors; `cpython_bytes_alignment_methods_diff_subset` / `cpython_bytes_alignment_methods_subset` cover bytes/bytearray `center()`, `ljust()`, and `rjust()` width and fill-byte behavior, receiver-driven result types, unchanged-width cases, and representative TypeErrors; `cpython_bytes_maketrans_translate_diff_subset` / `cpython_bytes_maketrans_translate_subset` cover bytes/bytearray `maketrans()` and `translate()` 256-byte tables, `None` identity tables, optional deletion bytes including `delete=`, bytes-like table/delete arguments, class and instance `maketrans()` lookup, receiver-driven result types, and representative TypeError/ValueError paths; `cpython_builtin_bytearray_translate_extend_errors_subset` pins CPython BuiltinTest bytearray translation-table/delete error ordering and `bytearray.extend(map(int, ...))` exception propagation without mutation; `cpython_bytes_remove_affix_methods_diff_subset` / `cpython_bytes_remove_affix_methods_subset` cover bytes/bytearray `removeprefix()` and `removesuffix()` over bytes-like affixes, empty receiver/affix behavior, receiver-driven result types, and representative TypeErrors; `cpython_bytes_join_diff_subset` / `cpython_bytes_join_subset` cover bytes/bytearray `join()` receiver-driven result types, list/tuple/iterator inputs, bytes-like items, empty joins/separators, reduced stress joins, and representative TypeErrors; `cpython_bytearray_join_custom_iterator_diff_subset` and `cpython_bytearray_join_reentrant_resize_subset` cover bytearray `join()` accepting custom iterators while rejecting re-entrant separator resizing with `BufferError`; `cpython_bytes_replace_partition_methods_diff_subset` / `cpython_bytes_replace_partition_methods_subset` cover bytes/bytearray `replace()`, `partition()`, and `rpartition()` result-type behavior, bytes-like arguments, replacement count handling, empty-needle replacement, empty separators, and representative TypeError/ValueError paths; `cpython_bytearray_mutation_methods_diff_subset` / `cpython_bytearray_mutation_methods_subset` cover bytearray `append()`, `extend()`, `insert()`, `pop()`, `remove()`, `reverse()`, `clear()`, and `copy()` in-place mutation behavior; `cpython_bytearray_extended_slice_assignment_diff_subset` / `cpython_bytearray_extended_slice_assignment_subset` cover bytearray extended slice assignment/deletion, integer-iterable RHS conversion, self-slice assignment, special method dispatch, and saturated large slice bounds; `cpython_bytes_copy_module_diff_subset` / `cpython_bytes_copy_module_subset` cover bytes/bytearray `copy.copy()` and `copy.deepcopy()` type and equality preservation plus independent bytearray copy buffers; `cpython_bytes_hex_separator_diff_subset` / `cpython_bytes_fromhex_string_diff_subset` / `cpython_bytes_fromhex_bytes_like_diff_subset` / `cpython_bytes_hex_fromhex_subset` cover bytes/bytearray `fromhex()` and bytes/bytearray `hex()` with separator grouping; `cpython_templatelib_constructor_subset` covers the supported `string.templatelib.Template` and `Interpolation` constructors, Template `values`, builtin type metadata, and conversion error paths; `cpython_templatelib_final_type_and_iterator_subset` covers final templatelib type inheritance errors plus TemplateIter type metadata, identity iteration, yielded interpolation objects, and repeated exhaustion; `cpython_t_string_raw_concat_and_triple_subset` covers raw t-string literal preservation, Template + Template concatenation, Template/string concatenation TypeErrors, and triple-quoted t-string segments |
| `CONTAINER_RUNTIME` | partial | `cpython_sequence_constructor_builtins_subset` covers first-pass list, tuple, and set constructors over builtins, strings, generator expressions, existing tuple identity preservation, keyword rejection, non-iterable rejection, unhashable set elements, exact `set.__init__` reinitialization behavior including self-input clearing and partial mutation before an unhashable element error, plus exact `TestSet` constructor identity, literal equality, left-to-right literal insertion/evaluation order, unhashable set values, and `set.copy()` equality/type/identity; `cpython_list_subclass_core_sequence_subset` covers built-in `list` subclasses for construction from iterables, `isinstance`, truthiness, `len()`, iteration, membership, method forwarding, item and slice mutation, `dir()` method names, reversed iteration, `repr()` / `str()` / f-string display including recursive storage, and constructor error paths; `cpython_tuple_subclass_core_sequence_subset` covers built-in `tuple` subclasses for construction from iterables, `isinstance`, truthiness, `len()`, iteration, indexing, slicing, reversed iteration, default `repr()` / `str()` / f-string display, empty instances, and constructor error paths; `cpython_dict_subclass_core_mapping_subset` covers built-in `dict` subclasses for construction from mappings and pair iterables, `isinstance`, `len()`, key iteration, `get()`, item assignment/replacement/deletion through subscript syntax, `update()`, membership, `repr()` / `str()` / f-string display, recursive storage, and missing-key deletion errors; `cpython_set_mutation_methods_subset` covers first-pass `TestSet` mutation method behavior for `clear`, `add`, `remove`, `discard`, `pop`, and `update`, including duplicate-add no-op, unhashable argument errors, missing-key `KeyError`, nested set/frozenset lookup equivalence, pop-until-empty behavior, and update result/error paths; `cpython_set_direct_lookup_and_keyerror_payload_subset` covers direct set-key membership/discard/remove behavior plus preservation of the original missing key in `KeyError.args[0]`; `cpython_set_hash_exception_propagation_subset` covers propagation of exceptions raised by user-defined `__hash__` during set membership, `add`, and `discard`; `cpython_set_bad_comparison_errors_subset` covers hash-collision rich equality and propagation of exceptions raised by user-defined `__eq__` during set construction, membership, `add`, `discard`, and `remove`; `cpython_set_bad_comparison_algebra_errors_subset` covers the same rich-equality exception propagation across set/frozenset equality and ordering checks, relation methods, algebra methods, and `&`, `|`, `-`, and `^`; `cpython_set_iterator_mutation_subset` covers CPython set iterator size-change invalidation and the clear/refill-to-original-size no-crash regression; `cpython_set_reentrant_mutation_subset` covers set updates whose rich equality clears the source set plus hash-collision `set.add()` re-entering the same set from Python-level `__eq__`; `cpython_set_operations_mutating_subset` covers CPython `TestOperationsMutating` stable cases for set equality, ordering, algebra, relation methods, and update methods when element equality clears both participating sets; `cpython_set_rich_compare_reflection_subset` covers CPython set ordering fallback through `NotImplemented` into the right operand's reflected rich-comparison method; `cpython_set_inplace_algebra_methods_subset` covers `TestSet` iterable operand support for `update`, `intersection_update`, `difference_update`, and `symmetric_difference_update`, in-place set operator identity preservation, strict `TypeError` for unhashable iterable operands, and partial mutation before `set.update()` encounters an unhashable element; `cpython_set_only_sets_in_binary_ops_subset` covers CPython `TestOnlySetsInBinaryOps` equality, ordering, binary operator, in-place operator, and method-form iterable behavior for non-set operands including generators; `cpython_dict_constructor_update_fromkeys_subset` covers first-pass dict construction, update, and `fromkeys`; `cpython_dict_view_mappingproxy_subset` covers dict-view `.mapping` read-only `mappingproxy` type identity, live equality, lookup, membership, and item-assignment rejection; `cpython_iter_next_builtin_subset` covers first-pass iterator identity, `next(default)`, callable-sentinel iterator exhaustion, callable-raised `StopIteration`, reentrant callable-sentinel exhaustion, and supported iterator sink-state behavior after exhaustion; `cpython_map_strict_builtin_subset` covers strict `map()` length checks, iterator-consumption side effects, and propagated custom iterator exceptions versus strict-mode `StopIteration` conversion; `cpython_reversed_builtin_subset` covers first-pass reversed iteration over supported sequence, dict, and dict-view values |
| `COLLECTIONS_ABC_RUNTIME` | partial | `cpython_collections_abc_iterable_iterator_subset` covers the supported `collections.abc.Iterable` and `Iterator` module surface, `isinstance` checks for built-in containers, built-in iterators, `TemplateIter`, non-iterable scalar values, and structural user classes, plus `issubclass` checks for structural user classes and `Iterator` inheriting from `Iterable`; `cpython_collections_abc_core_runtime_subset` covers the supported `Hashable`, `Sized`, `Container`, `Callable`, and `Collection` ABC surface, including built-in container/type relationships, structural user classes, direct ABC subclassing, and CPython-style `None` blocking for special methods; `cpython_collections_abc_registration_subset` covers public `ABC.register()` behavior for `Hashable`, `Iterable`, `Iterator`, `Reversible`, `Sized`, `Container`, and `Callable`, including pre-registration rejection, return identity, `issubclass()`, `isinstance()`, and subclass propagation; `cpython_collections_abc_types_coroutine_subset` covers public `types.coroutine()` behavior for generator functions, the CPython distinction that iterable-coroutine generators can be awaited but are not `Awaitable` / `Coroutine` ABC instances, and `Coroutine.register()` propagation through `Awaitable`; `cpython_collections_abc_sequence_subset` covers `Sequence` for supported built-in sequence registrations including `memoryview`, explicit Sequence subclassing, CPython's non-structural Sequence behavior, and Sequence inheritance through Reversible, Collection, Sized, Iterable, and Container; `cpython_collections_abc_sequence_mixins_subset` covers `Sequence` mixins for explicit subclasses, including index parity against native list/str start/stop behavior plus `count`, `__contains__`, `__iter__`, `__reversed__`, membership fallback, and keyword calls; `cpython_collections_abc_bytestring_buffer_subset` covers `ByteString` and `Buffer` for supported bytes/bytearray/memoryview registrations, ByteString inheritance through Sequence, Buffer `__buffer__` structural subclasshook behavior, direct ABC subclassing, `memoryview` exclusion from `ByteString`, and CPython-style `None` blocking for `__buffer__`; `cpython_collections_abc_bytestring_deprecation_warnings_subset` covers public `ByteString` import, fresh attribute access, `isinstance()`, and subclass-creation deprecation warnings; `cpython_collections_abc_mutable_sequence_subset` covers `MutableSequence` for list, bytearray, `collections.deque`, and `array.array` registrations, inheritance through Sequence/Reversible/Collection/Sized/Iterable/Container, CPython's non-structural protocol behavior, explicit subclass mixins, and self-extension; `cpython_collections_abc_mapping_subset` covers `Mapping` and `MutableMapping` for registered `dict`, ABC inheritance, direct subclassing, and CPython's non-structural mapping behavior; `cpython_collections_abc_mapping_view_subset` covers `MappingView`, `KeysView`, `ItemsView`, and `ValuesView` for built-in dict views, `KeysView`/`ItemsView` set behavior, `ValuesView` collection behavior, ABC inheritance, direct ABC subclassing, and CPython's non-structural view behavior; `cpython_collections_abc_set_mutable_set_mixins_subset` covers `Set` and `MutableSet` registrations for set/frozenset and supported set-like dict views, inheritance through Collection/Sized/Iterable/Container, explicit subclass mixins for comparison, binary set operations, `_hash`, `_from_iterable`, mutable update methods, and self-clearing regressions; `cpython_collections_abc_set_noncomparable_comparison_subset` covers CPython `test_issue16373` for `Set` subclass comparison fallback when the left operand returns `NotImplemented`; `cpython_collections_abc_set_from_iterable_operator_subset` covers CPython `test_Set_from_iterable` operator dispatch through MutableSet mixins, instance `_from_iterable` overrides, and `^=` with a non-Set iterable; `cpython_collections_abc_set_real_set_interoperability_subset` covers CPython `test_Set_interoperability_with_real_sets` for custom Set subclasses interacting with built-in `set` and plain iterables across `&`, `|`, `-`, `^`, ordering, equality, inequality, and TypeError paths for non-Set ordering operands; `cpython_collections_abc_set_hash_matches_frozenset_subset` covers CPython `test_Set_hash_matches_frozenset` for supported hashable samples, including `None`, numbers, strings, booleans, object identities, NaN, nested frozensets, large integers, range-derived frozensets, and the CPython `sys.maxsize` range stress sample; `cpython_frozenset_basic_subset`, `cpython_set_frozenset_joint_ops_subset`, `cpython_set_frozenset_relationship_matrix_subset`, and `cpython_set_frozenset_algebra_matrix_subset` cover first-pass exact `frozenset` construction, empty singleton identity, no-op exact `frozenset.__init__`, immutable set algebra, equality with `set`, order-independent hashing for hashable elements, dict/set key behavior, shared set/frozenset joint operations from CPython `test_set.py`, the `isdisjoint` constructor matrix, set-of-frozensets uniqueness, non-mutating set algebra constructor matrices, multi-operand union/intersection/difference, and the Issue #6573 empty-set union regression; `cpython_collections_abc_reversible_subset` covers `Reversible` for supported built-in reversible containers/views, non-reversible scalar/container/iterator samples, `Sequence` inheritance, structural `__iter__` + `__reversed__` user classes, direct ABC subclassing, and `None` blocking; `cpython_collections_abc_async_runtime_subset` covers `Awaitable`, `Coroutine`, `AsyncIterable`, and `AsyncIterator` for native coroutine objects, structural user classes, non-samples, ABC inheritance, and `None` blocking; `cpython_collections_abc_generator_runtime_subset` covers `Generator` and `AsyncGenerator` for native generator objects, structural protocol classes, incomplete protocol non-samples, direct ABC subclassing, and `None` blocking |

`cpython_dict_constructor_update_fromkeys_subset` now also covers instance-level
`{}.fromkeys`, and `cpython_globals_locals_builtin_subset` covers
classmethod-style `fromkeys()` lookup on scope-backed module namespace mappings.
`cpython_types_generic_alias_union_type_subset` covers `dict.__class_getitem__`
and `{}.__class_getitem__`, while `cpython_globals_locals_builtin_subset` covers
the same GenericAlias entry point on scope-backed module namespace mappings.
`cpython_dict_constructor_update_fromkeys_subset` covers `dict.__repr__` /
`dict.__str__` and instance `{}.__repr__` / `{}.__str__`; the globals/locals
subset covers the same display methods on scope-backed module namespaces.
It also covers direct empty-format `dict.__format__` / `{}.__format__` and the
matching scope-backed namespace `__format__` lookup.
`cpython_globals_locals_builtin_subset` now also covers direct
`__len__` / `__contains__` / `__repr__` methods on scope-backed namespace views.
| `NEWLINE` | supported | `lexes_newline`, `cpython_compile_crlf_newlines_subset`, `cpython_compile_specifics_newline_and_indentation_subset`, `cpython_tokenize_explicit_line_joining_subset` and `cpython_tokenize_explicit_line_joining_diff_subset` including continuation-only lines that do not emit statement newlines, token-kind/text parity with the no-continuation spelling, comment backslashes that do not continue, and bad-indentation continuation rejection, `cpython_tokenize_implicit_line_joining_subset` and `cpython_tokenize_implicit_line_joining_diff_subset` including the logical newline after a bracketed block containing comments and CPython's bracketed tuple/list/dict continuation semantics, `cpython_tokenize_spanned_tokens_subset` including newline span, `cpython_tokenize_trailing_space_without_newline_subset` covering tokenizer-mode preservation of a final whitespace-only physical line and final comment-only physical line, `cpython_tokenize_bytes_encoding_token_subset` covering a synthesized final newline after very long comment-only bytes source without a final newline, and `cpython_tokenize_invalid_python_token_stream_subset` including tokenizer-mode synthetic final newline |
| `INDENT` | supported | `lexes_if_block_indentation`, `lexes_tabs_in_indentation`, `cpython_tokenize_indentation_blank_line_subset`, `cpython_tokenize_indentation_blank_line_diff_subset`, `cpython_tokenize_nested_indentation_subset`, `cpython_tokenize_nested_indentation_diff_subset`, `cpython_tokenize_max_indent_subset`, `cpython_tokenize_unmatched_indentation_subset` / `cpython_tokenize_unmatched_indentation_diff_subset` including CPython-style tab expansion and inconsistent tab/space indentation rejection, `cpython_tokenize_formfeed_whitespace_subset` / `cpython_tokenize_formfeed_whitespace_diff_subset` including leading-formfeed indentation reset, `cpython_tokenize_explicit_line_joining_subset` including continuation-only lines that suppress unrelated indentation while still allowing a pending post-colon block indent, `cpython_tokenize_spanned_tokens_subset` including indent span |
| `DEDENT` | supported | `lexes_if_block_indentation`, `cpython_tokenize_indentation_blank_line_subset`, `cpython_tokenize_indentation_blank_line_diff_subset`, `cpython_tokenize_nested_indentation_subset`, `cpython_tokenize_nested_indentation_diff_subset`, `cpython_tokenize_max_indent_subset`, `cpython_tokenize_unmatched_indentation_subset` / `cpython_tokenize_unmatched_indentation_diff_subset` including unmatched-dedent spans, `cpython_tokenize_spanned_tokens_subset` including dedent span |
| `LPAR` | supported | `lexes_print_number` |
| `RPAR` | supported | `lexes_print_number` |
| `LSQB` | supported | `lexes_list_brackets` |
| `RSQB` | supported | `lexes_list_brackets` |
| `COLON` | supported | `lexes_if_block_indentation` |
| `COMMA` | supported | `lexes_comma`, `prints_multiple_arguments` |
| `SEMI` | supported | `lexes_semicolon`, `cpython_grammar_semicolon_simple_statements_subset` |
| `PLUS` | supported | `lexes_plus`, `cpython_grammar_additive_ops_subset` |
| `MINUS` | supported | `lexes_arithmetic_operators`, `cpython_grammar_additive_ops_subset` |
| `STAR` | supported | `lexes_arithmetic_operators`, `cpython_grammar_multiplicative_ops_subset`, `cpython_tokenize_multiplicative_operators_diff_subset` |
| `SLASH` | supported | `lexes_arithmetic_operators`, `cpython_grammar_multiplicative_ops_subset`, `cpython_tokenize_multiplicative_operators_diff_subset` |
| `VBAR` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_bitwise_and_shift_subset` |
| `AMPER` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_bitwise_and_shift_subset` |
| `LESS` | supported | `lexes_comparison_operators`, `cpython_grammar_ordering_comparison_subset` |
| `GREATER` | supported | `lexes_comparison_operators`, `cpython_grammar_ordering_comparison_subset` |
| `EQUAL` | supported | `lexes_equal`, `assigns_and_reads_variable` |
| `DOT` | supported | `lexes_attribute_dot_after_parenthesized_number`, `reports_attribute_errors` |
| `PERCENT` | supported | `lexes_arithmetic_operators`, `cpython_grammar_multiplicative_ops_subset`, `cpython_tokenize_multiplicative_operators_diff_subset` |
| `LBRACE` | supported | `lexes_dict_braces`, `cpython_ast_dict_literal_subset`, `cpython_dict_display_unpacking_subset` |
| `RBRACE` | supported | `lexes_dict_braces`, `cpython_ast_dict_literal_subset`, `cpython_dict_display_unpacking_subset` |
| `EQEQUAL` | supported | `lexes_equal_equal`, `cpython_grammar_equal_comparison_subset` |
| `NOTEQUAL` | supported | `lexes_comparison_operators`, `cpython_grammar_equal_comparison_subset` |
| `LESSEQUAL` | supported | `lexes_comparison_operators`, `cpython_grammar_ordering_comparison_subset` |
| `GREATEREQUAL` | supported | `lexes_comparison_operators`, `cpython_grammar_ordering_comparison_subset` |
| `TILDE` | supported | `lexes_bitwise_and_shift_operators`, `cpython_grammar_unary_ops_subset`, `cpython_tokenize_unary_operators_diff_subset` |
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
| `DOUBLESLASH` | supported | `lexes_arithmetic_operators`, `cpython_grammar_multiplicative_ops_subset`, `cpython_tokenize_multiplicative_operators_diff_subset` |
| `DOUBLESLASHEQUAL` | supported | `lexes_augmented_assignment_operators`, `cpython_ast_augmented_assignment_subset` |
| `AT` | supported | `lexes_decorator_at_sign`, `cpython_tokenize_exact_type_subset` including CPython pathological trailing whitespace, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, `cpython_tokenize_matrix_multiply_and_ellipsis_diff_subset`, `runs_matrix_multiply_special_methods` |
| `ATEQUAL` | supported | `lexes_matrix_multiply_and_ellipsis_tokens`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, `cpython_tokenize_matrix_multiply_and_ellipsis_diff_subset`, `runs_matrix_multiply_special_methods` |
| `RARROW` | supported | `lexes_function_return_arrow`, `cpython_grammar_annotations_subset` |
| `ELLIPSIS` | supported | `lexes_matrix_multiply_and_ellipsis_tokens`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, `cpython_tokenize_matrix_multiply_and_ellipsis_diff_subset` |
| `COLONEQUAL` | supported | `lexes_colon_equal`, `cpython_assignment_expression_subset`, and named-expression runtime tests cover the walrus token |
| `EXCLAMATION` | supported | f-string and t-string conversion `!` is covered by `lexes_f_string_parts`, `lexes_f_string_debug_expressions`, `cpython_tokenize_f_string_split_token_subset`, `cpython_tokenize_t_string_split_token_subset`, `cpython_f_string_helper_rules_subset`, `cpython_f_string_basic_subset`, `cpython_f_string_debug_expression_subset`, `cpython_invalid_f_string_syntax_subset`, `runs_f_strings`, and `runs_f_string_expressions` |
| `OP` | out_of_scope_runtime | Aggregate tokenize.py token category rather than parser input token |
| `TYPE_IGNORE` | supported | `lexes_type_comments_and_type_ignores`, `cpython_type_comments_and_ignores_subset`, `cpython_type_comment_modern_syntax_and_ignores_subset`, and `skips_type_comments_and_type_ignores` cover type-ignore trivia and public `TypeIgnore.tag` preservation |
| `TYPE_COMMENT` | supported | `lexes_type_comments_and_type_ignores`, `cpython_type_comments_and_ignores_subset`, `cpython_func_type_comment_helper_rules_subset`, `cpython_type_comment_public_ast_metadata_subset`, `cpython_type_comment_argument_ast_metadata_subset`, `cpython_inappropriate_type_comments_subset`, `cpython_type_comment_modern_syntax_and_ignores_subset`, and `skips_type_comments_and_type_ignores` cover statement, function, and parameter type comments, public AST `type_comment` metadata, non-ASCII comments, and CPython-style rejection of misplaced type comments when `ast.parse(..., type_comments=True)` is requested |
| `SOFT_KEYWORD` | supported | `keeps_match_and_case_as_soft_keywords`, `cpython_grammar_match_stmt_subset`, `cpython_type_alias_statement_subset`, `cpython_type_params_metadata_subset`, and `cpython_lazy_import_syntax_subset` cover contextual soft-keyword behavior |
| `FSTRING_START` | partial | MiniPython's parser still consumes collapsed `Token::FString`, while `tokenize_cpython_with_spans()` now exposes first-pass split tokens; covered by `cpython_tokenize_f_string_split_token_subset` for CPython `FSTRING_START` spans across plain, raw-prefix, recursively nested, escaped-brace, debug-padding, multiline literal, non-ASCII/emoji, cross-line expression, and multiline triple-quoted f-strings, plus `lexes_f_string_parts`, `lexes_f_string_format_specs`, `lexes_f_string_debug_expressions`, `lexes_raw_f_string_literals_and_empty_format_specs`, `cpython_tokenize_f_string_span_subset` including CPython-derived start/end source spans, plain and raw-prefix f-strings, single/triple-quoted f-strings, multiline triple f-strings, multiline non-ASCII and emoji f-strings, replacement fields with lambda/non-ASCII/newline expression source, nested f-string expression source, ordinary and raw line continuations, `!r` conversions, debug expressions, and format specs, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_f_string_basic_subset`, `cpython_f_string_conversion_operator_edge_subset`, `cpython_f_string_contextual_runtime_subset`, `cpython_f_string_format_error_subset`, `cpython_f_string_many_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_invalid_f_string_syntax_subset`, `runs_f_strings`, `runs_f_string_expressions` |
| `FSTRING_MIDDLE` | partial | MiniPython's parser still stores f-string middles as collapsed `TokenFStringPart::Literal` values, while `tokenize_cpython_with_spans()` now exposes first-pass split middles; covered by `cpython_tokenize_f_string_split_token_subset` for CPython literal middles, escaped-brace middles around replacement fields, conversion-expression separators, debug-expression padding, multiline debug replacement fields, literal middles between multiple replacement fields, multiline expression newlines without synthetic indent/dedent tokens, physical `NL` tokens inside bracketed replacement expressions, multiline format-spec middles including unevenly indented post-colon literal middles, non-ASCII/emoji middles, nested f-string expression token boundaries, and nested format-spec replacement fields, plus `lexes_f_string_parts`, `lexes_f_string_escaped_brace_literals`, `lexes_f_string_backslash_before_doubled_braces`, `lexes_f_string_line_continuations`, `lexes_f_string_format_specs`, `lexes_f_string_debug_expressions`, `lexes_raw_f_string_literals_and_empty_format_specs`, `cpython_tokenize_f_string_span_subset` including literal/expression part checks, escaped-brace, backslash-before-doubled-brace, ordinary/raw line-continuation literals, multiline triple-quoted middles, non-ASCII and emoji middle text around replacement fields, lambda/non-ASCII/newline replacement expression source, conversion fields, format-spec middles, and debug-expression labels in `cpython_f_string_basic_subset` / `cpython_string_line_continuation_subset`, warning coverage in `cpython_f_string_escape_warning_subset`, `cpython_f_string_conversion_operator_edge_subset`, `cpython_f_string_contextual_runtime_subset`, `cpython_f_string_format_error_subset`, `cpython_f_string_many_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_invalid_f_string_syntax_subset`, `runs_f_strings`, `runs_f_string_expressions` |
| `FSTRING_END` | partial | MiniPython's parser still consumes collapsed `Token::FString`, while `tokenize_cpython_with_spans()` now exposes first-pass split end tokens; covered by `cpython_tokenize_f_string_split_token_subset` for CPython end-token spans across single-quoted, double-quoted, triple-quoted, multiline triple-quoted, non-ASCII/emoji, cross-line-expression, and recursively nested f-strings, plus `lexes_f_string_parts`, `lexes_f_string_format_specs`, `lexes_f_string_debug_expressions`, `lexes_raw_f_string_literals_and_empty_format_specs`, `cpython_tokenize_f_string_span_subset` including whole-literal source spans for single-line, triple-quoted, multiline, non-ASCII multiline, line-continuation, conversion, format-spec, debug-expression, lambda/newline-expression, and nested-expression f-strings, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_f_string_basic_subset`, `cpython_f_string_conversion_operator_edge_subset`, `cpython_f_string_contextual_runtime_subset`, `cpython_f_string_format_error_subset`, `cpython_f_string_many_expressions_subset`, `cpython_f_string_debug_expression_subset`, `cpython_invalid_f_string_syntax_subset`, `runs_f_strings`, `runs_f_string_expressions` |
| `TSTRING_START` | partial | MiniPython's parser still consumes collapsed `Token::TString`, while `tokenize_cpython_with_spans()` now exposes first-pass split tokens; covered by `cpython_tokenize_t_string_split_token_subset` for CPython-style t-string start spans across ordinary, raw-prefix, debug-padding, nested-format, multiline literal, non-ASCII/emoji, cross-line expression, and multiline triple-quoted t-strings, plus `lexes_t_string_parts`, `cpython_tokenize_t_string_span_subset` including CPython-derived whole-literal source spans, literal-only templates, ordinary and multiple interpolations, expression-source preservation, `rt`/`tr` raw prefixes, `!s`/`!r`/`!a` conversions, debug fields, format specs, nested format-spec replacement fields, and multiline triple-quoted t-strings, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_t_string_basic_subset`, `cpython_t_string_nested_template_and_runtime_error_subset`, `cpython_t_string_raw_concat_and_triple_subset`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_invalid_t_string_syntax_subset`, `runs_t_strings` |
| `TSTRING_MIDDLE` | partial | MiniPython's parser still stores t-string middles as collapsed `TokenFStringPart::Literal` values, while `tokenize_cpython_with_spans()` now exposes first-pass split middles; covered by `cpython_tokenize_t_string_split_token_subset` for literal middles, literal middles between multiple interpolation fields, raw-prefix literal tails after replacement fields, `!r` conversion tokens, debug-expression padding, multiline expression newlines without synthetic indent/dedent tokens, physical `NL` tokens inside bracketed replacement expressions, format-spec middles including unevenly indented post-colon literal middles, non-ASCII/emoji middles, and nested format-spec replacement fields, plus `lexes_t_string_parts`, `cpython_tokenize_t_string_span_subset` including literal/interpolation part checks, raw-prefix literal middles, conversion fields, debug labels, literal and nested-expression format specs, and triple-quoted multiline literal segments, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_t_string_basic_subset`, `cpython_t_string_nested_template_and_runtime_error_subset`, `cpython_t_string_raw_concat_and_triple_subset`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_invalid_t_string_syntax_subset`, `runs_t_strings` |
| `TSTRING_END` | partial | MiniPython's parser still consumes collapsed `Token::TString`, while `tokenize_cpython_with_spans()` now exposes first-pass split end tokens; covered by `cpython_tokenize_t_string_split_token_subset` for CPython-style t-string end spans across ordinary, raw-prefix, nested-format, multiline, non-ASCII/emoji, and cross-line-expression t-strings, plus `lexes_t_string_parts`, `cpython_tokenize_t_string_span_subset` including final source spans across single-line, conversion, debug, raw-prefix, nested-format, and multiline triple-quoted t-strings, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_t_string_basic_subset`, `cpython_t_string_nested_template_and_runtime_error_subset`, `cpython_t_string_raw_concat_and_triple_subset`, `cpython_string_and_tstring_helper_rules_subset`, `cpython_invalid_t_string_syntax_subset`, `runs_t_strings` |
| `COMMENT` | supported | Comments and type-comment/ignore comments are covered by `cpython_tokenize_comments_subset`, `cpython_tokenize_comments_diff_subset`, `lexes_type_comments_and_type_ignores`, and `cpython_type_comments_and_ignores_subset` |
| `NL` | supported | Non-logical physical newlines inside blank/comment lines, implicit line joining including comments and physical newlines inside bracketed blocks, explicit continuation-only lines including blank physical lines after a backslash join, a final whitespace-only physical line without a trailing newline, and a final comment-only physical line without a trailing newline are covered by `cpython_tokenize_comments_subset`, `cpython_tokenize_implicit_line_joining_subset`, `cpython_tokenize_implicit_line_joining_diff_subset`, `cpython_tokenize_explicit_line_joining_subset`, `cpython_tokenize_explicit_line_joining_diff_subset`, and `cpython_tokenize_trailing_space_without_newline_subset` |
| `ERRORTOKEN` | partial | `rejects_unknown_character`, `rejects_invalid_non_printable_characters`, `rejects_null_bytes_with_cpython_message`, `rejects_unclosed_bracketed_statements`, `rejects_unmatched_closing_brackets`, `rejects_unterminated_string`, `rejects_cpython_unterminated_string_forms`, `rejects_cpython_invalid_string_escape_forms`, `rejects_cpython_unterminated_interpolated_string_forms`, `cpython_invalid_string_literal_subset`, `cpython_invalid_f_string_syntax_subset`, `cpython_invalid_t_string_syntax_subset`, `cpython_tokenize_explicit_line_joining_subset` including invalid line-continuation spans, `cpython_tokenize_error_token_subset` including invalid-character, non-breaking-space, and CPython `test_invisible_characters` non-printable-control spans, unmatched and mismatched bracket spans, cross-line mismatched closing bracket spans, EOF-in-multiline spans for bare and populated open brackets, unterminated triple-quoted string spans, single-quoted multiline f-string token errors, line-continuation EOF, null-byte spans, and too-deep bracket nesting |
| `ENCODING` | partial | `cpython_source_encoding_detection_subset` covers representative CPython `Lib/test/test_tokenize.py::TestDetectEncoding` and `Lib/test/test_source_encoding.py` behavior for default UTF-8 detection, first- and second-line coding cookies, latin-1 and UTF-8 normalization including CPython's 12-character `get_normal_name()` prefix behavior, ignored second-line cookies after real code, UTF-8 BOM stripping, BOM/cookie mismatch errors, null bytes in coding lines, unknown encodings, and ASCII/UTF-8 decode failures. `cpython_tokenize_bytes_encoding_token_subset` covers CPython-style leading `ENCODING` tokens from byte tokenization, BOM stripping before tokenization, latin-1 decoding, iso-8859-15 decoding, `cp1252`, `cp949`, `cp932`, and `cp1251` decoding, source CRLF normalization in triple-quoted strings, a `latin-1` long-comment source without a final newline, ASCII rejection of non-ASCII source bytes, and default-UTF-8 rejection of invalid bytes inside f-string middle text. `cpython_source_encoding_execution_subset` covers CPython-style execution of supported bytes source through decoding, parser, compiler, and VM, including UTF-8 default/BOM, first- and second-line iso-8859-15 cookies, double coding lines, long coding-cookie lines, long latin-1 coding-name normalization, long UTF-8 comment lines, non-UTF-8 coding-cookie comments, ignored third-line cookies, source CRLF/CR normalization in triple-quoted strings, non-UTF-8 shebangs with matching cookies, very long `latin-1` comment-only source without a final newline, `cp1252`, `cp949`, `cp932`, and `cp1251` source execution, partial UTF-8 BOM decode errors, representative BOM/cookie and ASCII decode errors, and invalid default-UTF-8 bytes inside f-string middle text. `cpython_bytes_source_output_parity_diff_subset` adds CPython differential output parity for actual bytes files using default UTF-8 decoding, UTF-8 cookies, ISO-8859-15 first-line/second-line/empty-first-line cookies, ignored third-line cookies, double coding-line precedence, ISO-8859-15 cookie lines containing non-UTF-8 bytes, UTF-8 BOM default/comment/cookie handling, a UTF-8 BOM empty source line, CPython encoded-module `iso-8859-1` and `koi8-r` samples, CPython `tokenizedata/coding20731.py`, `cp949`, `cp932`, `cp1252`, and `cp1251`; `cpython_bytes_exec_source_output_parity_diff_subset` covers current CPython `exec(bytes)` parity for long first- and second-line coding cookies, long coding-cookie lines, long normalized Latin-1 coding names, and long UTF-8 comment-only lines; `cpython_bytes_source_rejection_parity_diff_subset` adds differential rejection parity for unknown cookies, BOM/cookie mismatches including second-line and fake-cookie BOM cases, partial UTF-8 and UTF-16-LE BOMs, ASCII-cookie body decode failures, default-UTF-8 second- and third-line decode failures, invalid f-string middle bytes, and CPython `tokenizedata/bad_coding*.py` samples. MiniPython still relies on the migrated manual decoders plus the `encoding_rs` label set rather than CPython's full codecs registry. |

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
| `type_alias` | supported | `cpython_type_alias_statement_subset`, `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_evaluate_functions_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_scope_expression_subset`, `runs_function_class_and_alias_type_params`, `runs_generic_alias_type_subscripts` |
| `return_stmt` | supported | `cpython_ast_return_stmt_subset`, `cpython_finally_control_flow_warning_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset`, `cpython_invalid_control_flow_context_subset`, `returns_none_from_bare_return`, `reports_return_outside_function` |
| `raise_stmt` | supported | `cpython_grammar_raise_and_try_except_subset` including custom exception classes deriving from `Exception`, custom exception `__class__` identity, and subclass exceptions caught by a custom base handler, `cpython_base_exception_args_subset` / `cpython_base_exception_args_diff_subset` including BaseException argument tuple/display/repr behavior, `cpython_base_exception_with_traceback_subset` / `cpython_base_exception_with_traceback_diff_subset` including traceback object identity preservation, the `with_traceback(None)` path, and non-traceback rejection, `cpython_system_exit_oserror_attributes_subset` / `cpython_system_exit_oserror_attributes_diff_subset` including `SystemExit.code` and `OSError` attribute/args normalization plus CPython-style `OSError.__str__`, `cpython_syntax_error_attributes_subset` / `cpython_syntax_error_attributes_diff_subset` including `SyntaxError` construction attributes, `cpython_unicode_error_attributes_subset` / `cpython_unicode_error_attributes_diff_subset` including Unicode encode/decode/translate construction attributes, `cpython_attribute_error_keyword_attributes_subset` including source-migration-only `AttributeError(name=..., obj=...)` under the current default oracle, `cpython_builtin_exception_hierarchy_subset` / `cpython_builtin_exception_hierarchy_diff_subset` including standard builtin exception base-class catches, `cpython_invalid_simple_statement_subset`, `reports_unhandled_raise`, `preserves_explicit_exception_cause`, `preserves_implicit_exception_context`, `supports_raise_from_none`, `rejects_invalid_exception_cause` |
| `pass_stmt` | supported | `cpython_grammar_pass_statement_subset`, `runs_pass_statement` |
| `break_stmt` | supported | `cpython_grammar_break_continue_subset` covers plain break, `while False` break, nested try/except continue-then-break regression behavior, and if/else/break loop flow; `cpython_finally_control_flow_warning_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset`, `cpython_invalid_control_flow_context_subset`, `runs_break_inside_while_loop` |
| `continue_stmt` | supported | `cpython_grammar_break_continue_subset` covers plain continue, `while False` continue, continue through try/except and try/finally, and nested continue-then-break regression behavior; `cpython_finally_control_flow_warning_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset`, `cpython_invalid_control_flow_context_subset`, `runs_continue_inside_while_loop` |
| `global_stmt` | supported | `cpython_grammar_global_stmt_subset`, `cpython_global_binding_targets_subset`, `cpython_scope_declaration_error_subset`, `writes_global_name_from_function`, `augassigns_global_name_from_function` |
| `nonlocal_stmt` | supported | `cpython_scope_closure_and_nonlocal_subset`, `cpython_nonlocal_binding_targets_subset`, `cpython_scope_declaration_error_subset`, `cpython_type_params_nonlocal_scope_subset`, `writes_nonlocal_name_from_nested_function`, `nonlocal_writes_nearest_enclosing_scope` |
| `del_stmt` | supported | `cpython_delete_target_helper_rules_subset`, `cpython_grammar_del_stmt_subset`, `cpython_invalid_assignment_target_subset`, `cpython_invalid_simple_statement_subset`, `deletes_names_attributes_and_subscripts`, `deletes_list_slices`, `reports_delete_errors` |
| `del_targets` | supported | `cpython_delete_target_helper_rules_subset`, `cpython_grammar_del_stmt_subset`, `deletes_names_attributes_and_subscripts`, `deletes_list_slices` |
| `del_target` | supported | Name, attribute, subscript, tuple/list, and parenthesized delete targets are covered by `cpython_delete_target_helper_rules_subset`, `cpython_grammar_del_stmt_subset`, and `deletes_names_attributes_and_subscripts` |
| `del_t_atom` | supported | `cpython_delete_target_helper_rules_subset`, `cpython_grammar_del_stmt_subset`, `deletes_names_attributes_and_subscripts`, `deletes_list_slices` |
| `type_expressions` | supported | `cpython_func_type_input_subset`, `cpython_type_expression_helper_rules_subset`, `parses_func_type_input_mode` |
| `func_type_comment` | supported | Inline and own-line function type comments for `def` and `async def` are accepted by `cpython_func_type_comment_helper_rules_subset`; `cpython_type_comment_public_ast_metadata_subset` verifies public AST metadata when `type_comments=True`; unrelated `# type: ignore` comments are skipped by `skips_type_comments_and_type_ignores` |
| `invalid_double_type_comments` | supported | Duplicate inline plus own-line function type comments are rejected for `def` and `async def` by `cpython_func_type_comment_helper_rules_subset` |
| `yield_stmt` | supported | `cpython_grammar_yield_stmt_subset`, `cpython_yield_expression_helper_rule_subset`, `runs_generator_yield_with_next`, `runs_for_loop_over_generator`, `runs_yield_from_expression`, `runs_generator_send_values`, `runs_generator_throw_values`, `runs_generator_close_values`, `catches_stop_iteration_from_next`; comprehension outer-iterable `yield` is accepted while comprehension-internal `yield` is rejected |
| `assert_stmt` | supported | `cpython_grammar_assert_stmt_subset`, `cpython_invalid_simple_statement_subset`, `runs_assert_statement_when_condition_is_truthy` |
| `invalid_raise_stmt` | supported | Missing raise value and missing raise cause forms are rejected by `cpython_invalid_simple_statement_subset` |
| `invalid_del_stmt` | supported | Literal, starred, function-call, conditional, operator, named-expression, and nested invalid delete targets are rejected by `cpython_invalid_simple_statement_subset` |
| `invalid_assert_stmt` | supported | Accidental assignment and unparenthesized named-expression assert forms are rejected by `cpython_invalid_simple_statement_subset` |
| `import_stmt` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, `cpython_lazy_import_syntax_subset`, `cpython_import_sys_modules_cache_subset`, `parses_relative_import_ellipsis_levels`, `cpython_ast_validator_stdlib_recursive_compile_seed_subset`, `rejects_invalid_import_forms_with_cpython_messages`, `runs_import_statement`, `runs_from_import_statement`, `runs_import_aliases_and_star_import`, `runs_lazy_import_syntax` |
| `import_name` | supported | `cpython_import_helper_rules_subset`, `cpython_grammar_import_stmt_subset`, `cpython_compile_specifics_import_syntax_subset`, `cpython_lazy_import_syntax_subset`, `cpython_import_sys_modules_cache_subset`, `runs_import_statement`, `runs_import_aliases_and_star_import`, `runs_lazy_import_syntax` |
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
| `for_stmt` | supported | `cpython_grammar_for_subset`, `cpython_grammar_async_for_subset`, `cpython_builtin_range_for_iteration_subset`, `runs_for_loop_over_list`, `runs_for_loop_over_mutating_list`, `cpython_ast_tuple_unpacking_subset`, `cpython_invalid_assignment_target_subset`, `cpython_invalid_control_flow_syntax_subset`, `cpython_control_flow_in_finally_override_subset`, `cpython_control_flow_inside_except_and_with_subset` including CPython-derived break/continue through `try/finally`, inside `finally`, inside `with`, and inside `async with` |
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
| `try_stmt` | supported | `cpython_grammar_raise_and_try_except_subset` including custom exception subclass matching and custom exception `__class__` identity, `cpython_base_exception_args_subset` / `cpython_base_exception_args_diff_subset` including caught exception argument preservation, `cpython_base_exception_with_traceback_subset` / `cpython_base_exception_with_traceback_diff_subset` including traceback object identity preservation and catchable invalid-traceback `TypeError`, `cpython_system_exit_oserror_attributes_subset` / `cpython_system_exit_oserror_attributes_diff_subset`, `cpython_syntax_error_attributes_subset` / `cpython_syntax_error_attributes_diff_subset`, `cpython_unicode_error_attributes_subset` / `cpython_unicode_error_attributes_diff_subset`, and `cpython_attribute_error_keyword_attributes_subset` including source-migration-only builtin exception attribute preservation under the current default oracle, `cpython_builtin_exception_hierarchy_subset` / `cpython_builtin_exception_hierarchy_diff_subset` including `ArithmeticError` and `LookupError` handler matching plus `GeneratorExit` / `Exception` separation, `cpython_invalid_control_flow_syntax_subset`, `cpython_invalid_control_flow_context_subset`, `cpython_runtime_exception_capture_subset` / `cpython_runtime_exception_capture_diff_subset`, `cpython_control_flow_in_finally_subset`, `cpython_control_flow_in_finally_override_subset` including issue #37830 pending-return override cases, `cpython_control_flow_inside_except_and_with_subset`, `cpython_grammar_try_star_subset`, `cpython_except_star_split_semantics_subset`, `cpython_except_star_rejects_exception_group_types_subset`, `catches_raised_exceptions`, `catches_tuple_exception_handlers`, `catches_dotted_exception_handler_type`, `catches_dynamic_exception_handler_type_expression`, `runs_except_star_handlers`, `splits_exception_groups_for_except_star_handlers`, `rejects_exception_group_types_in_except_star_handlers`, `runs_try_else_when_no_exception`, `runs_finally_without_exception`, `runs_finally_after_handled_exception`, `runs_finally_before_reraising_exception`, `runs_finally_before_returning_from_function`, `runs_finally_before_breaking_from_loop`, `runs_finally_before_continuing_loop` |
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
| `function_def` | supported | Decorated and undecorated function definitions, including decorated async functions, are covered by `cpython_function_def_decorated_rule_subset`, `cpython_ast_function_def_subset`, `cpython_grammar_decorators_subset`, `cpython_grammar_annotations_subset`, `cpython_func_type_comment_helper_rules_subset`, `cpython_invalid_function_def_raw_subset`, `cpython_positional_only_arguments_subset`, `cpython_ast_function_defaults_and_keywords_subset`, `cpython_ast_starred_function_parameters_subset`, `cpython_type_params_metadata_subset`, `cpython_type_params_dunder_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_param_subset`, `cpython_invalid_assignment_target_subset`, `cpython_function_globals_attribute_subset` for supported `function.__globals__` and `dir(function)` behavior, `cpython_function_builtins_attribute_subset` for supported `function.__builtins__` behavior, `defines_and_calls_function`, `runs_function_default_parameters`, `runs_function_keyword_arguments`, `runs_function_decorators`, `runs_function_annotations`, `runs_function_class_and_alias_type_params`, `runs_generic_alias_type_subscripts`, `runs_positional_only_parameters`, `runs_varargs_functions`, `runs_kwargs_functions`, `runs_keyword_only_parameters`, and `runs_recursive_function` |
| `function_def_raw` | supported | Ordinary `def` and `async def` headers with optional type params, params, return annotations, function type comments, and inline/indented bodies are covered by `cpython_function_def_raw_rule_subset`, `cpython_ast_function_def_subset`, `cpython_func_type_comment_helper_rules_subset`, `cpython_invalid_function_def_raw_subset`, `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `defines_and_calls_function`, and `runs_function_annotations` |
| `invalid_def_raw` | supported | Missing function-header parentheses, missing colons, and missing indented suites for ordinary, async, typed, and type-parameterized definitions are rejected by `cpython_invalid_function_def_raw_subset` including parser diagnostic spans |
| `async_funcdef` | supported | `cpython_async_funcdef_rule_subset`, `cpython_grammar_async_await_subset`, `cpython_grammar_generator_expression_subset`, `cpython_func_type_comment_helper_rules_subset`, `cpython_invalid_function_def_raw_subset`, `cpython_invalid_assignment_target_subset`, `runs_async_function_and_await_expression`, and `runs_coroutine_throw_and_close_methods` cover CPython's `async def` rule shape, empty coroutine bodies, type parameters, complex parameters, return annotations, function type comments, nested async functions, coroutine functions, CPython's conversion of unhandled coroutine `StopIteration` into `RuntimeError` with `__cause__`, CPython-style coroutine `__await__()` wrappers with iterator identity/reuse behavior, non-None send rejection on just-started coroutines, returned `StopIteration` objects staying ordinary return values, custom `__await__` iterators whose yielded values suspend through the outer coroutine and whose return values complete the await expression, CPython-style rejection of objects without `__await__` and of `__await__` returning `None`, a coroutine, or another non-iterator object, awaited expression composition, nested await, await in keyword arguments and tuple values, send/throw forwarding into suspended await expressions including CPython's custom-exception `test_await_14` shape, rejection of awaiting an already-suspended coroutine, returned exception values with cleared await context, async generators, async-generator `anext`/`asend`/`athrow`/`aclose`, `StopAsyncIteration` exhaustion, and CPython's rejection of `yield from` inside async functions and `return value` inside async generators |
| `class_def` | supported | Decorated and undecorated class definitions are covered by `cpython_class_def_decorated_rule_subset`, `cpython_grammar_class_def_subset`, `cpython_class_def_raw_helper_rules_subset`, `cpython_grammar_decorators_subset`, `cpython_grammar_annotations_subset`, `cpython_type_params_metadata_subset`, `cpython_type_params_dunder_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_scope_expression_subset`, `cpython_invalid_assignment_target_subset`, `defines_and_instantiates_class`, `runs_class_attributes`, `cpython_compile_static_attributes_exact_subset` for CPython-style `__static_attributes__`, `runs_class_decorators`, `runs_class_annotations`, `runs_class_inheritance_and_header_arguments`, CPython parity for user-class `isinstance` over direct instances and base classes, `cpython_issubclass_builtin_subset` for supported class hierarchy checks, `cpython_type_builtin_subset` for one-argument `type()`, `cpython_type_dynamic_class_subset` for first-pass three-argument dynamic class construction, `cpython_type_name_qualname_subset` / `cpython_type_name_qualname_diff_subset` for mutable dynamic-class `__name__` and `__qualname__`, `cpython_type_doc_and_firstlineno_subset` / `cpython_type_doc_and_firstlineno_diff_subset` for dynamic-class `__doc__` and `__firstlineno__`, `cpython_type_bad_slots_subset` for public invalid dynamic-class `__slots__` error classes, `cpython_type_nokwargs_subset` for `type()` keyword rejection, `cpython_type_typeparams_subset` for class `__type_params__` assignment/delete behavior, `cpython_type_namespace_order_subset` for minimal `OrderedDict` metadata/display/recursive-display/copy/equality/generic-alias/fromkeys/popitem/reversed/union coverage plus ordered dynamic-class namespace preservation, `cpython_vars_dir_builtin_subset` for first-pass class/instance `vars()` and `dir()` introspection, `cpython_bound_method_metadata_subset` for bound-method metadata and class-body method aliasing, `cpython_bound_method_descriptor_and_repr_subset` for bound-method `__get__` and stable repr metadata, `cpython_bound_method_identity_subset` for stored bound-method identity and fresh attribute-access method objects, `cpython_unbound_super_descriptor_subset` for one-argument/unbound `super` descriptor rebinding and metadata, `runs_function_class_and_alias_type_params`, `runs_generic_alias_type_subscripts`, and `runs_instance_methods` |
| `class_def_raw` | supported | Raw class headers with optional type params, empty argument lists, positional bases, keyword/unpacked header arguments, and inline/indented bodies are covered by `cpython_class_def_raw_helper_rules_subset`, `cpython_grammar_class_def_subset`, `cpython_type_params_metadata_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_scope_expression_subset`, `runs_class_inheritance_and_header_arguments`, `runs_function_class_and_alias_type_params`, and `runs_generic_alias_type_subscripts` |
| `invalid_class_def_raw` | supported | Missing class-header colons and missing indented suites for plain, based, and type-parameterized class headers are rejected by `cpython_class_def_raw_helper_rules_subset` |
| `match_stmt` | supported | Valid match suites, multiple case blocks, inline and indented case bodies, invalid empty match suites, and the delegated invalid alternatives are covered by `cpython_grammar_match_stmt_subset` / `cpython_grammar_match_stmt_diff_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_invalid_match_pattern_subset`, `runs_match_literal_cases`, `runs_match_complex_literal_patterns`, `runs_match_class_patterns`, `runs_match_wildcard_case`, `runs_match_or_pattern`, `runs_match_sequence_patterns`, `runs_match_star_sequence_patterns`, `runs_match_as_patterns`, `runs_match_value_patterns`, `runs_match_mapping_patterns`, `runs_match_open_sequence_patterns`, `runs_match_capture_pattern`, `runs_match_guards`, and `keeps_match_and_case_as_soft_keywords` |
| `subject_expr` | supported | Named-expression subjects, tuple subjects, optional trailing commas, and starred tuple subjects are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset` and `cpython_grammar_match_stmt_subset` |
| `case_block` | supported | Multiple case blocks, wildcard cases, guarded cases, indented suites, and inline suites are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, and `cpython_invalid_match_pattern_subset` |
| `guard` | supported | Guarded cases, boolean guards, and named expressions inside guards are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_guards` |
| `patterns` | supported | Open sequence patterns and single patterns are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_open_sequence_patterns` |
| `pattern` | supported | `as_pattern` and `or_pattern` alternatives are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, `runs_match_as_patterns`, and `runs_match_or_pattern` |
| `as_pattern` | supported | As-pattern binding, nested as-patterns, wildcard captures, invalid targets, and parenthesized OR-as patterns are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, `cpython_invalid_match_pattern_subset`, and `runs_match_as_patterns` |
| `or_pattern` | supported | Literal/value OR-patterns, same-name capture alternatives, reordered capture alternatives, mapping alternatives, parenthesized `as` alternatives, non-final irrefutable alternatives, and different-name binding errors are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_or_pattern` |
| `closed_pattern` | supported | Literal, capture, wildcard, value, group, sequence, mapping, and class alternatives are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, and the focused match helper tests |
| `literal_pattern` | supported | Numeric, signed, complex, adjacent-string, boolean, and None literal patterns are covered by `cpython_grammar_match_stmt_subset`; f-string and t-string match values are rejected by the parser like CPython |
| `literal_expr` | supported | Literal mapping keys, adjacent-string keys, complex keys, singleton keys, dotted value keys, duplicate literal-key rejection, and rejected f-string/t-string mapping keys are covered by `cpython_match_pattern_helper_rules_subset` / `cpython_match_pattern_helper_rules_diff_subset` and `cpython_grammar_match_stmt_subset` |
| `complex_number` | supported | Complex literal plus/minus alternatives in patterns and mapping keys are covered by `cpython_match_numeric_literal_helper_rules_subset` / `cpython_match_numeric_literal_helper_rules_diff_subset` |
| `signed_number` | supported | Positive and negative numeric literal patterns are covered by `cpython_match_numeric_literal_helper_rules_subset` / `cpython_match_numeric_literal_helper_rules_diff_subset` |
| `signed_real_number` | supported | Positive and negative real parts of complex literal patterns are covered by `cpython_match_numeric_literal_helper_rules_subset` / `cpython_match_numeric_literal_helper_rules_diff_subset` |
| `real_number` | supported | Real numeric literal patterns are covered by `cpython_match_numeric_literal_helper_rules_subset` / `cpython_match_numeric_literal_helper_rules_diff_subset` |
| `imaginary_number` | supported | Imaginary literal patterns and complex imaginary parts are covered by `cpython_match_numeric_literal_helper_rules_subset` / `cpython_match_numeric_literal_helper_rules_diff_subset` |
| `capture_pattern` | supported | Capture patterns are covered by `cpython_match_capture_wildcard_group_helper_rules_subset` / `cpython_match_capture_wildcard_group_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, and `runs_match_capture_pattern` |
| `pattern_capture_target` | supported | Capture targets and their `_`, `.`, `(`, and `=` lookahead exclusions are covered by `cpython_match_capture_target_and_star_pattern_helper_rules_subset` / `cpython_match_capture_target_and_star_pattern_helper_rules_diff_subset` and `cpython_match_pattern_helper_rules_subset` |
| `wildcard_pattern` | supported | Wildcard cases and grouped wildcards are covered by `cpython_match_capture_wildcard_group_helper_rules_subset` / `cpython_match_capture_wildcard_group_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_wildcard_case` |
| `value_pattern` | supported | Simple and recursive dotted value patterns, dotted mapping keys, and invalid equality/dangling-dot forms are covered by `cpython_match_value_attr_name_or_attr_helper_rules_subset` / `cpython_match_value_attr_name_or_attr_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, and `runs_match_value_patterns` |
| `attr` | supported | Recursive dotted attributes in value patterns, mapping keys, and class names are covered by `cpython_match_value_attr_name_or_attr_helper_rules_subset` / `cpython_match_value_attr_name_or_attr_helper_rules_diff_subset` and `runs_match_value_patterns` |
| `name_or_attr` | supported | Bare class names and dotted class/value pattern prefixes are covered by `cpython_match_value_attr_name_or_attr_helper_rules_subset` / `cpython_match_value_attr_name_or_attr_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `group_pattern` | supported | Parenthesized capture, wildcard, and sequence group patterns are covered by `cpython_match_capture_wildcard_group_helper_rules_subset` / `cpython_match_capture_wildcard_group_helper_rules_diff_subset` and `cpython_match_pattern_helper_rules_subset` |
| `sequence_pattern` | supported | Bracketed and parenthesized sequence alternatives, empty sequences, optional trailing commas, and star-containing sequences are covered by `cpython_match_sequence_helper_rules_subset` / `cpython_match_sequence_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_sequence_patterns` |
| `open_sequence_pattern` | supported | Naked comma sequence patterns and parenthesized comma sequence patterns are covered by `cpython_match_sequence_helper_rules_subset` / `cpython_match_sequence_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_open_sequence_patterns` |
| `maybe_sequence_pattern` | supported | One-or-more sequence subpatterns with optional trailing commas are covered by `cpython_match_sequence_helper_rules_subset` / `cpython_match_sequence_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `maybe_star_pattern` | supported | Plain and starred sequence subpatterns, including wildcard star targets and duplicate-star rejection, are covered by `cpython_match_sequence_helper_rules_subset` / `cpython_match_sequence_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_star_sequence_patterns` |
| `star_pattern` | supported | `*name` and `*_` sequence star patterns are covered by `cpython_match_capture_target_and_star_pattern_helper_rules_subset` / `cpython_match_capture_target_and_star_pattern_helper_rules_diff_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_star_sequence_patterns` |
| `mapping_pattern` | supported | Empty mappings, pure rest mappings, item-only mappings, item-plus-rest mappings, trailing commas, invalid rest placement, static duplicate literal key rejection, and dynamic duplicate dotted-key `ValueError` behavior are covered by `cpython_match_mapping_helper_rules_subset` / `cpython_match_mapping_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, `runs_match_mapping_patterns`, and `raises_value_error_for_dynamic_duplicate_match_mapping_keys` |
| `items_pattern` | supported | Single and multiple mapping pattern items are covered by `cpython_match_mapping_helper_rules_subset` / `cpython_match_mapping_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `key_value_pattern` | supported | Literal keys, dotted value keys, nested value patterns, and dynamic duplicate key checks are covered by `cpython_match_mapping_helper_rules_subset` / `cpython_match_mapping_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `raises_value_error_for_dynamic_duplicate_match_mapping_keys` |
| `double_star_pattern` | supported | Mapping rest patterns, optional trailing comma, and invalid `_` rest targets are covered by `cpython_match_mapping_helper_rules_subset` / `cpython_match_mapping_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_mapping_patterns` |
| `class_pattern` | supported | Empty, positional-only, keyword-only, positional-plus-keyword, dotted-name, trailing-comma, builtin match-self classes, zero-positional builtin classes, non-class callees, and invalid keyword/positional ordering forms are covered by `cpython_match_class_helper_rules_subset` / `cpython_match_class_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, `cpython_grammar_match_stmt_subset`, and `runs_match_class_patterns` |
| `positional_patterns` | supported | One-or-more positional class subpatterns with optional trailing commas, nested subpatterns, builtin positional-count errors, and no-binding-on-TypeError behavior are covered by `cpython_match_class_helper_rules_subset` / `cpython_match_class_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `keyword_patterns` | supported | One-or-more class keyword subpatterns, keyword-only forms, and mixed positional-plus-keyword forms are covered by `cpython_match_class_helper_rules_subset` / `cpython_match_class_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
| `keyword_pattern` | supported | Individual class keyword subpatterns with nested pattern values and duplicate-keyword rejection are covered by `cpython_match_class_helper_rules_subset` / `cpython_match_class_helper_rules_diff_subset`, `cpython_match_pattern_helper_rules_subset`, and `cpython_grammar_match_stmt_subset` |
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
| `compare_op_bitwise_or_pair` | supported | All comparison operators consuming right-hand `bitwise_or` expressions are covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `eq_bitwise_or` | supported | Equality comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `noteq_bitwise_or` | supported | Inequality comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `lte_bitwise_or` | supported | Less-than-or-equal comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `lt_bitwise_or` | supported | Less-than comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `gte_bitwise_or` | supported | Greater-than-or-equal comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `gt_bitwise_or` | supported | Greater-than comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `notin_bitwise_or` | supported | `not in` comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `in_bitwise_or` | supported | `in` comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset` |
| `isnot_bitwise_or` | supported | `is not` comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset`; singleton and mutable-container identity behavior is covered by `cpython_grammar_identity_comparison_subset` |
| `is_bitwise_or` | supported | `is` comparison with a right-hand `bitwise_or` expression is covered by `cpython_comparison_helper_rules_subset` / `cpython_comparison_helper_rules_diff_subset`; singleton and mutable-container identity behavior is covered by `cpython_grammar_identity_comparison_subset` |
| `bitwise_or` | supported | `cpython_grammar_bitwise_and_shift_subset`; CPython differential parity for bitwise operator precedence is covered by `cpython_program_output_parity_smoke_diff_subset` |
| `bitwise_xor` | supported | `cpython_grammar_bitwise_and_shift_subset`; CPython differential parity for bitwise operator precedence is covered by `cpython_program_output_parity_smoke_diff_subset` |
| `bitwise_and` | supported | `cpython_grammar_bitwise_and_shift_subset`; CPython differential parity for bitwise operator precedence is covered by `cpython_program_output_parity_smoke_diff_subset` |
| `shift_expr` | supported | `cpython_grammar_bitwise_and_shift_subset`; CPython differential parity for shift associativity is covered by `cpython_program_output_parity_smoke_diff_subset` |
| `sum` | supported | `cpython_grammar_additive_ops_subset`, `runs_arithmetic_precedence`, and CPython differential parity for left-associative additive chains in `cpython_program_output_parity_smoke_diff_subset` |
| `term` | supported | Multiplication, division, floor division, modulo, matrix multiplication, and left-associative term chains are covered by `cpython_grammar_multiplicative_ops_subset`, `cpython_tokenize_multiplicative_operators_diff_subset`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, `runs_matrix_multiply_special_methods`, `runs_division_modulo_and_power`, `runs_sequence_repetition_and_basic_len_list_builtins`, `reports_arithmetic_type_errors`, `reports_division_by_zero`, and CPython differential parity for matrix-multiply special-method behavior in `cpython_program_output_parity_smoke_diff_subset` |
| `factor` | supported | `cpython_grammar_unary_ops_subset`, `cpython_tokenize_unary_operators_diff_subset`, `runs_unary_arithmetic`, and CPython differential parity for unary precedence in `cpython_program_output_parity_smoke_diff_subset` |
| `invalid_arithmetic` | supported | Unparenthesized `not` after `+`, `-`, `*`, `/`, `%`, `//`, and `@` is rejected by `cpython_invalid_arithmetic_and_factor_syntax_subset` |
| `invalid_factor` | supported | Unparenthesized `not` after unary `+`, `-`, and `~` is rejected by `cpython_invalid_arithmetic_and_factor_syntax_subset` |
| `power` | supported | `cpython_grammar_power_and_paren_precedence_subset`, `runs_division_modulo_and_power` |
| `primary` | supported | `cpython_primary_rule_subset` covers recursive attribute, call, generator-expression call, subscript, and atom alternatives; broader selector/call/subscript behavior is covered by `cpython_selector_helper_rules_subset`, `cpython_ast_redundant_parentheses_and_call_trailer_subset`, `cpython_ast_function_defaults_and_keywords_subset`, `cpython_ast_starred_call_arguments_subset`, `cpython_invalid_call_argument_syntax_subset`, `cpython_compile_expression_stack_size_shapes_subset` large function/method call compile-shape checks, `cpython_grammar_subscript_index_subset`, `cpython_grammar_slice_subset`, `cpython_grammar_class_def_subset`, `cpython_ast_subscript_assignment_subset`, `cpython_ast_slice_assignment_subset`, `cpython_user_defined_subscript_protocol_subset`, `cpython_type_params_generic_alias_subset`, and `runs_generic_alias_type_subscripts` |
| `slices` | supported | Single index, one-dimensional slice, tuple index, tuple of slices, starred subscript items, generic-alias unpack items, load/store/delete subscript paths, and first-pass `slice.indices()` normalization are covered by `cpython_selector_helper_rules_subset`, `cpython_grammar_subscript_index_subset`, `cpython_grammar_slice_subset`, `cpython_ast_subscript_assignment_subset`, `cpython_ast_slice_assignment_subset`, `cpython_user_defined_subscript_protocol_subset`, `runs_subscript_errors`, `slices_lists_and_strings`, `slices_ranges`, and `cpython_program_output_parity_smoke_diff_subset` selector parity |
| `slice` | supported | The `a:b:c` slice alternative with omitted and present start/stop/step parts plus the `named_expression` index alternative and runtime `slice.indices()` behavior are covered by `cpython_selector_helper_rules_subset`, `cpython_grammar_subscript_index_subset`, `cpython_grammar_slice_subset`, `cpython_ast_slice_assignment_subset`, `cpython_assignment_expression_subset`, `cpython_user_defined_subscript_protocol_subset`, `slices_lists_and_strings`, and `slices_ranges` |
| `atom` | supported | `cpython_atom_rule_subset` covers names, singletons, strings, numbers, tuple/group/generator forms, list/list-comprehension forms, dict/set/comprehension forms, and ellipsis; broader literal/display behavior is covered by `cpython_display_helper_rules_subset`, `cpython_ast_constant_values_subset`, `cpython_grammar_prefixed_integer_literals_subset`, `cpython_grammar_imaginary_literals_subset`, `cpython_tokenize_matrix_multiply_and_ellipsis_subset`, string/f-string/t-string helper tests, display/comprehension tests, runtime literal tests, and `cpython_program_output_parity_smoke_diff_subset` atom display parity |
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
| `fstring_format_spec` | supported | Literal format middle text and nested replacement fields, including raw/non-raw escapes, greedy `}` field termination, width, precision, alignment, alternate-form base prefixes, simple `s`/`d`/`b`/`o`/`x`/`X`/`c`/`f` format codes, zero-fill and `=` alignment, custom `__format__` format-spec delivery, inherited `object.__format__` rejection for non-empty specs, duplicate/mixed `,`/`_` grouping-option errors, and numeric grouping rendering for decimal integers, fixed-point floats, and underscore-grouped non-decimal integers, are covered by `cpython_f_string_helper_rules_subset`, `cpython_f_string_basic_subset`, `cpython_f_string_format_specifier_expressions_subset`, `cpython_format_builtin_and_custom_dunder_format_subset`, `cpython_format_grouping_option_errors_subset`, `cpython_format_grouping_rendering_subset`, `cpython_format_integer_codes_and_zero_alignment_subset`, `cpython_raw_f_string_format_spec_subset`, `runs_f_string_expressions`, and `formats_values_with_format_specs`, with CPython differential parity for nested format-spec expression output in `cpython_program_output_parity_smoke_diff_subset` |
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

Namedtuple coverage note: `cpython_collections_namedtuple_factory_instance_subset`,
`cpython_collections_namedtuple_factory_instance_diff_subset`,
`cpython_collections_namedtuple_defaults_rename_readonly_subset`,
`cpython_collections_namedtuple_field_doc_subset`,
`cpython_collections_namedtuple_name_conflicts_subset`,
`cpython_collections_namedtuple_repr_subset`,
`cpython_collections_namedtuple_subclass_issue_24931_subset`,
`cpython_collections_namedtuple_match_args_diff_subset`,
`cpython_collections_namedtuple_match_args_subset`,
`cpython_collections_namedtuple_new_builtins_issue_43102_subset`,
`cpython_collections_namedtuple_pickle_subset`,
`cpython_collections_namedtuple_large_size_subset`, and
`cpython_collections_namedtuple_copy_keyword_generic_alias_subset` port the
current public behavior slices from CPython `TestNamedTuple`, covering
`collections.namedtuple()` import/factory behavior, generated type metadata,
`_fields`, `__match_args__`, tuple-like construction, positional/keyword/star
and double-star calls, field attributes, indexing, iteration, tuple/list
conversion, `_make()`, `_replace()`, `_asdict()`, comma/tuple field specs,
zero/one-field namedtuples, invalid-name errors, inherited tuple type methods,
weakref exclusion, `defaults=`, `_field_defaults`, `__new__.__defaults__`,
`rename=True`, `module=`, mutable class `__doc__`, field descriptor `__doc__`,
readonly field/item behavior, shallow/deep copy, keyword-only factory options,
CPython's field-name conflict matrix, generated namedtuple/subclass repr,
generated `__match_args__` class-pattern execution, generated `__new__`
builtins metadata, deterministic large generated types, pickle round trips over
MiniPython's internal payload, and namedtuple type generic-alias subscription.
The remaining CPython descriptor reuse/repr tests are tracked as
`blocked_by_cpython_internal`.

Counter coverage note: `cpython_collections_counter_basics_subset` ports the
current CPython `TestCounter::test_basics` method: construction from iterables,
mappings, and keywords; dict/Mapping instance and subclass checks; missing-key
zero lookup; count updates; views; iteration; exact repr; `most_common()`;
`elements()`; `pop`; `popitem`; `clear`; `setdefault()`; unhashability; and
Reversible registration for OrderedDict and Counter plus reverse key iteration.
`cpython_collections_counter_init_update_subset` adds method-level Counter
coverage for `Counter(...)` and `update()` keyword names that are real keys,
`iterable=None` direct insertion into an empty Counter, bad sources, too many
arguments, and unbound method TypeErrors.
`cpython_collections_counter_fromkeys_diff_subset` and
`cpython_collections_counter_fromkeys_subset` add direct CPython output parity
and runtime coverage for `Counter.fromkeys()` and instance `fromkeys()` raising
`NotImplementedError`.
`cpython_collections_counter_most_common_diff_subset` and
`cpython_collections_counter_most_common_subset` add direct CPython output
parity and runtime coverage for `most_common()` full output, zero limit,
positive limit, `None` limit, and increasing limit slices.
`cpython_collections_counter_mapping_mutation_diff_subset` and
`cpython_collections_counter_mapping_mutation_subset` add direct CPython output
parity and runtime coverage for `pop()`, missing-key `pop(default)`,
`setdefault()`, `popitem()`, `clear()`, and zero lookup after deletion.
`cpython_collections_counter_comparison_diff_subset` and
`cpython_collections_counter_comparison_subset` add method-level Counter
coverage for `total()`, membership over stored zero/negative-count keys,
equality with missing keys treated as zero, and `<=` / `<` / `>=` / `>` rich
comparison over count values. The direct diff is gated because older system
CPython oracles predate `Counter.total()` and the new zero-count
rich-comparison rules.
`cpython_collections_counter_conversions_subset` adds method-level Counter
coverage for `elements()`, Counter iteration, `dict(Counter(...))`,
`dict(Counter(...).items())`, and `set(Counter(...))` conversion behavior.
`cpython_collections_counter_subtract_unary_subset` adds method-level Counter
coverage for `subtract()` over keyword, Counter, and iterable inputs plus
unary `+Counter` / `-Counter` filtering of positive, zero, and negative counts.
`cpython_collections_counter_repr_nonsortable_subset` adds method-level Counter
coverage for `repr()` preserving entries whose counts are not directly
comparable with numeric counts.
`cpython_collections_counter_copy_subclass_subset` adds method-level Counter
coverage for subclass construction, `Counter.copy()` preserving the concrete
subclass type, `isinstance()` relationships, missing-key zero lookup on the
copy, and copy independence for subsequent updates.
`cpython_collections_counter_copying_subset` adds method-level Counter coverage
for `copy()`, `copy.copy()`, `copy.deepcopy()`, pickle round trips,
`eval(repr(...))`, `update(words)`, `Counter(words)`, type preservation, and
copy independence after mutation.
`cpython_collections_counter_order_preservation_subset` adds method-level
Counter coverage for insertion-order preservation across construction, equal
counts, `elements()`, unary plus/minus, binary multiset operations, in-place
multiset operations, `update()`, and `subtract()`.
`cpython_collections_counter_update_reentrant_add_clears_counter_subset` adds
method-level Counter coverage for update reentrancy where `old + 1` invokes a
user-defined int-subclass `__add__` that clears the Counter before the computed
replacement count is written back.
`cpython_collections_counter_helper_function_subset` adds method-level Counter
coverage for `collections._count_elements()` over exact dicts, OrderedDict
insertion order, and Counter subclasses that override `__setitem__` or `get`.
`cpython_collections_counter_multiset_operations_subset` adds deterministic
method-level Counter coverage for `+`, `-`, `|`, `&`, and `^` multiset
arithmetic, direct dunder dispatch, and stripping zero/negative result counts.
`cpython_collections_counter_multiset_operations_matrix_subset` ports the
CPython 1000-pair randomized count-formula matrix with deterministic samples,
including `Counter.__add__`, `__sub__`, `__or__`, `__and__`, and `__xor__`
result counts plus positive-count filtering.
`cpython_collections_counter_inplace_operations_subset` adds deterministic
method-level Counter coverage for `__iadd__`, `__isub__`, `__ior__`, `__iand__`,
and `__ixor__`, including result parity with binary operations and identity
preservation through `id()`.
`cpython_collections_counter_inplace_operations_matrix_subset` ports the CPython
1000-pair randomized in-place matrix with deterministic samples, checking
regular-operation parity, receiver mutation, and identity preservation for all
five in-place Counter operations.
`cpython_collections_counter_multiset_operations_equivalent_to_set_operations_subset`
and gated
`cpython_collections_counter_multiset_operations_equivalent_to_set_operations_diff_subset`
port CPython's full 64-by-64 zero/one-count matrix equating Counter multiset
operations and rich comparisons with regular set operations.
`cpython_collections_counter_symmetric_difference_subset` ports CPython's full
9^4 `Counter ^ Counter` matrix, while gated
`cpython_collections_counter_symmetric_difference_diff_subset` keeps direct
CPython parity evidence when the configured oracle exposes `Counter.__xor__`.
The matrix includes elementwise absolute-difference invariants, positive
filtering, input-order preservation, and `^=` parity.

## Function Parameters

| CPython rule | Status | Rust evidence |
| --- | --- | --- |
| `params` | supported | Valid function parameter alternatives and invalid function parameter alternatives are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_invalid_parameter_syntax_subset`, `cpython_invalid_parameters_subset`, and `cpython_ast_function_def_subset` |
| `parameters` | supported | Positional-only, positional-or-keyword, defaulted, starred, keyword-only, and `**kwargs` function parameter alternatives are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_ast_function_defaults_and_keywords_subset`, `cpython_ast_starred_function_parameters_subset`, and `cpython_positional_only_arguments_subset` |
| `slash_no_default` | supported | `cpython_positional_only_arguments_subset`, `runs_positional_only_parameters` |
| `slash_with_default` | supported | `cpython_positional_only_arguments_subset`, `runs_positional_only_parameters` |
| `star_etc` | supported | Varargs, starred-annotation varargs, keyword-only parameters, `**kwargs`, parameter type comments, and invalid star alternatives are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_type_comment_argument_ast_metadata_subset`, `cpython_ast_starred_function_parameters_subset`, `cpython_invalid_parameter_syntax_subset`, `cpython_invalid_parameters_subset`, `runs_varargs_functions`, `runs_keyword_only_parameters`, and `runs_kwargs_functions` |
| `kwds` | supported | `**kwargs` with optional trailing comma, annotation, type comment metadata, invalid defaults, and invalid followers are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_type_comment_argument_ast_metadata_subset`, `cpython_ast_starred_function_parameters_subset`, `cpython_invalid_parameter_syntax_subset`, `cpython_invalid_parameters_subset`, and `runs_kwargs_functions` |
| `param_no_default` | supported | Comma-terminated, close-paren-terminated, type-comment-bearing, public `ast.arg.type_comment` metadata, and positional-only parameters without defaults are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_type_comment_argument_ast_metadata_subset`, `cpython_positional_only_arguments_subset`, and `cpython_ast_function_def_subset` |
| `param_no_default_star_annotation` | supported | Starred vararg annotations with and without a following `**kwargs` are covered by `cpython_function_parameter_helper_rules_subset` |
| `param_with_default` | supported | Comma-terminated, close-paren-terminated, type-comment-bearing, public `ast.arg.type_comment` metadata, and positional-only parameters with defaults are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_type_comment_argument_ast_metadata_subset`, `cpython_ast_function_defaults_and_keywords_subset`, and `runs_function_default_parameters` |
| `param_maybe_default` | supported | Keyword-only parameters with and without defaults, including final, comma-terminated, and type-comment-bearing forms, are covered by `cpython_function_parameter_helper_rules_subset`, `cpython_type_comment_argument_ast_metadata_subset`, and `cpython_ast_starred_function_parameters_subset` |
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
| `type_params` | supported | Function, async function, class, and type-alias type parameter lists, including trailing commas, invalid empty lists, rejected non-default parameters after defaults, dunder metadata access/assignment, CPython-style `__constraints__` for tuple-constrained type parameters, `typing.NoDefault` for missing defaults, CPython private-name mangling for type parameters while preserving public `__name__`, traditional `typing.TypeVar` interoperability in ordinary annotations, PEP 695 generic-class rejection of explicit `Generic[...]` or undeclared traditional TypeVars in bases, runtime compatibility with `typing.TypeVar`, `typing.TypeVarTuple`, and `typing.ParamSpec` including variance/default metadata, generator/coroutine capture, and weakref construction, nested generic class-header missing-name errors that surface as catchable runtime `NameError`, nested generic function/class type-parameter closure identity, generic class/function `__qualname__` metadata for module-level and function-local definitions, lazy recursive class bound/constraints lookup, deferred missing-name errors for type-parameter bounds/constraints with later re-evaluation, lazy type-parameter default evaluation and caching, public evaluate functions for alias values, bounds, constraints, and defaults through annotationlib VALUE/FORWARDREF/STRING formats, exact type-parameter object identity in supported annotation/type-alias scopes, implicit `Generic` class bases with `types.get_original_bases()`, generic metaclass evaluation through `metaclass=MyMeta[A, B]` plus exact `type(cls)` / `cls.__class__` metadata, class-header starargs and `**kwargs` propagation into `__init_subclass__`, CPython class-scope lookup for aliases/generic method bounds and annotations across prior class bindings, enclosing nonlocal bindings, future class bindings, explicit `global`/`nonlocal`, lazy later-mutation visibility, and nested free-variable cases, class/type-alias lambda closure capture, class-base lambda capture observed through `typing.get_args()`, generic-alias, nested-class, and bound/value generator/list-comprehension type-scope capture, generic-method generator-expression annotation capture, previous and current type-parameter bound references, and type-parameter/nonlocal scope interactions, are covered by `cpython_type_params_metadata_subset`, `cpython_type_params_mangling_subset`, `cpython_type_params_traditional_typevars_subset`, `cpython_type_params_typevar_runtime_subset`, `cpython_type_params_typevartuple_paramspec_runtime_subset`, `cpython_type_params_weakrefs_subset`, `cpython_type_params_runtime_name_error_subset`, `cpython_type_params_access_core_subset`, `cpython_type_params_class_scope_first_pass_subset`, `cpython_type_params_class_scope_lazy_subset`, `cpython_type_params_lazy_evaluation_qualname_subset`, `cpython_type_params_lazy_evaluation_bounds_subset`, `cpython_type_params_complex_calls_subset`, `cpython_type_params_dunder_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_evaluate_functions_subset`, `cpython_type_param_starred_invalid_subset`, `cpython_type_param_defaults_lazy_and_symtable_subset`, `cpython_type_param_nondefault_after_default_subset`, `cpython_type_params_nonlocal_scope_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_params_subset`, `runs_function_class_and_alias_type_params`, and `runs_generic_alias_type_subscripts` |
| `type_param_seq` | supported | Single, multiple, variadic, ParamSpec, duplicate-name, trailing-comma, default-ordering, nonlocal-conflict, and runtime `typing.TypeVarTuple` / `typing.ParamSpec` compatibility sequences are covered by `cpython_type_params_metadata_subset`, `cpython_type_params_duplicate_name_subset`, `cpython_type_params_typevartuple_paramspec_runtime_subset`, `cpython_type_param_nondefault_after_default_subset`, `cpython_type_params_nonlocal_scope_subset`, `cpython_type_params_generic_alias_subset`, `runs_function_class_and_alias_type_params`, and `runs_generic_alias_type_subscripts` |
| `type_param` | supported | Plain TypeVar, TypeVarTuple, ParamSpec, bound/default combinations, previous-type-parameter bound references, traditional `typing.TypeVar` / `TypeVarTuple` / `ParamSpec` constructors, public evaluate functions, invalid variadic bounds, and invalid type-scope expressions are covered by `cpython_type_params_metadata_subset`, `cpython_type_params_access_core_subset`, `cpython_type_params_typevar_runtime_subset`, `cpython_type_params_typevartuple_paramspec_runtime_subset`, `cpython_type_param_defaults_subset`, `cpython_type_params_evaluate_functions_subset`, `cpython_type_params_generic_alias_subset`, `cpython_invalid_type_param_subset`, `runs_function_class_and_alias_type_params`, and `runs_generic_alias_type_subscripts` |
| `type_param_bound` | supported | Simple bounds, tuple constraint expressions, previous-type-parameter generic bounds such as `Sequence[S]`, public bound/constraint evaluate functions, and named/yield/await rejection in bounds and constraints are covered by `cpython_type_params_metadata_subset`, `cpython_type_params_access_core_subset`, `cpython_type_params_evaluate_functions_subset`, `cpython_invalid_type_param_subset`, and `runs_function_class_and_alias_type_params` |
| `type_param_default` | supported | TypeVar and ParamSpec defaults with simple and generic-alias expressions, public default evaluate functions, invalid starred defaults on non-TypeVarTuple parameters, lazy evaluation/caching, symtable-key regression coverage, invalid named/yield/await defaults, and non-default-after-default rejection are covered by `cpython_type_param_defaults_subset`, `cpython_type_params_evaluate_functions_subset`, `cpython_type_param_starred_invalid_subset`, `cpython_type_param_defaults_lazy_and_symtable_subset`, `cpython_type_param_nondefault_after_default_subset`, `cpython_invalid_type_param_subset`, and `runs_function_class_and_alias_type_params` |
| `type_param_starred_default` | supported | TypeVarTuple defaults with ordinary and starred expressions, CPython-style `Unpack[...]` values from starred defaults and one-shot `GenericAlias.__iter__`, lazy default evaluation/caching, plus invalid yield defaults are covered by `cpython_type_param_defaults_subset`, `cpython_type_param_defaults_lazy_and_symtable_subset`, `cpython_invalid_type_param_subset`, and `runs_function_class_and_alias_type_params` |
| `invalid_type_param` | supported | Bounds and constraints on TypeVarTuple and ParamSpec parameters, plus named/yield/await expressions in type parameter scopes, are rejected by `cpython_invalid_type_param_subset` |
| `invalid_type_params` | supported | Empty type parameter lists on functions, classes, and type aliases are rejected by `cpython_invalid_type_params_subset` |

Runtime note: `cpython_type_params_runtime_class_namespace_subset` covers
metaclass `__prepare__` returning a dict subclass as the class namespace,
including class-body assignment capture and dict-subclass `__missing__`
exceptions during nested generic class base lookup.

Compile manifest note: `TestBooleanExpression`, `TestStaticAttributes`,
`TestExpressionStackSize`, and `TestStackSizeStability` now have method-level
audits in `cpython_test_manifest.md` that map all 52 current CPython methods
to direct Rust evidence, with drift guards against the local CPython
`Lib/test/test_compile.py` source. `TestSourcePositions` now has a separate
method-level audit that classifies all 33 current methods, keeping the direct
public-AST compile path and first-pass public code-position surfaces as
`ported`, while exact CPython opcode/debug-range assertions are classified as
`blocked_by_cpython_internal`.
`TestInstructionSequence` has a method-level audit that keeps all 3 current
methods classified as `blocked_by_cpython_internal` because they require
CPython's `_testinternalcapi` instruction-sequence object and opcode metadata.

Builtin manifest note: `TestBreakpoint`, `PtyTests`, `ShutdownTest`, and
`ImmortalTests` now have method-level audits that keep all 23 current CPython
methods classified as runtime or CPython-internal coverage, with drift guards
against the local `Lib/test/test_builtin.py` source. `TestType` has a separate
method-level audit that maps all 10 current CPython methods to Rust evidence or
explicit partial gaps. Eight methods are `ported`; `test_type_name` and
`test_type_doc` remain `partial` for surrogate-code-point `UnicodeEncodeError`
branches.

Builtin runtime note: `cpython_builtin_bool_notimplemented_subset` now ports
`BuiltinTest::test_bool_notimplemented`, rejecting `NotImplemented` in
`bool()`, `if`, and `not` boolean contexts. Capability-gated direct CPython
rejection evidence is in `cpython_builtin_bool_notimplemented_diff_subset` for
oracles with the current TypeError behavior. The differential harness keeps the
version-stable singleton identity/equality and unsupported set-dunder
`NotImplemented` surface separate from the current-source boolean-context
rejection, because older default system `python3` oracles can still have the
legacy deprecation-warning behavior.

Builtin async-iterator note: `cpython_aiter_anext_builtin_subset` and
`cpython_aiter_anext_builtin_diff_subset` cover the public `aiter()` builtin
alongside existing `anext()` async-generator coverage: one-argument arity,
`__aiter__` dispatch, async-iterator return validation through `__anext__`,
missing-protocol `TypeError`s, and propagation of exceptions raised by
`__aiter__`.

Builtin singleton note: `cpython_builtin_construct_singletons_subset` now ports
`BuiltinTest::test_construct_singletons`, covering zero-argument construction of
`NoneType`, `ellipsis`, and `NotImplementedType` back to the existing singleton
objects and TypeError rejection for positional or keyword constructor
arguments.
`cpython_builtin_singleton_attribute_access_subset` now ports
`BuiltinTest::test_singleton_attribute_access`, covering `__class__` identity
for `NotImplemented` and `Ellipsis`, their type objects being instances of
`type`, instance attribute read/write rejection, and class attribute assignment
rejection for the singleton type objects. Direct CPython output parity for both
methods is in `cpython_builtin_singleton_construction_and_attributes_diff_subset`.
Builtin breakpoint note: `cpython_builtin_breakpoint_custom_hook_subset`,
`cpython_builtin_breakpoint_default_stub_subset`,
`cpython_builtin_breakpoint_passthru_error_subset`,
`cpython_builtin_breakpoint_custom_hook_diff_subset`, and
`cpython_builtin_breakpoint_passthru_error_diff_subset`
cover the sandbox-safe public subset from
`Lib/test/test_builtin.py::TestBreakpoint`: builtin and `builtins.breakpoint`
visibility, `sys.breakpointhook` / `sys.__breakpointhook__` metadata, custom
hook dispatch, positional/keyword passthrough, hook return values, custom-hook
TypeError propagation, reset identity, a sandbox no-op default hook returning
`None`, and the lost-hook `RuntimeError`.
default pdb-backed breakpoint behavior, `PYTHONBREAKPOINT`, environment
lookup, import warnings, and interactive debugger behavior remain
runtime-blocked.
`cpython_builtin_generator_dynamic_lookup_subset` now ports the public semantic
part of `BuiltinTest::test_all_any_tuple_list_set_optimization`, covering
dynamic global and builtins-module lookup for `all`, `any`, `tuple`, `list`,
and `set` when used around generator expressions.
`cpython_builtin_print_keyword_diff_subset` and
`cpython_builtin_print_keyword_subset` cover the sandbox-safe `print()` keyword
surface: `sep`, `end`, `file=None`, `flush`, string-subclass separators/endings,
partial-line output joining for `end=''`, and representative keyword/type
errors. Non-`None` `file` targets remain outside the sandbox subset because
they imply file-like write dispatch.
`cpython_stop_iteration_value_diff_subset` and
`cpython_stop_iteration_value_subset` cover public `StopIteration.value`
behavior for direct exception construction, generator return values, and
`StopIteration` subclasses.
