# Sandbox E2E Requirement Matrix

This matrix is the process-level complement to `sandbox_mvp.md`. An
`example_e2e` row runs checked-in Python source in real CPython and real
sandbox-default `mnpy`. A `harness_only` row covers a transport or launcher
condition that cannot be represented by a valid Python source file.

| Requirement | Proof class | Example | Executable evidence |
| --- | --- | --- | --- |
| Safe stdlib remains useful | `example_e2e` | `examples/sandbox/safe_stdlib.py` | `real_cpython_and_mnpy_classify_the_complete_safe_stdlib_example` |
| Host I/O, network, process, and C ABI stay unavailable | `example_e2e` | `examples/sandbox/blocked_host_capabilities.py` | `real_cpython_and_mnpy_diverge_only_at_host_capability_boundary` |
| Entry source-size limit precedes execution | `example_e2e` | `examples/sandbox/source_size_budget.py` | `real_cpython_completes_while_mnpy_enforces_source_size_budget` |
| VM instruction budget stops finite CPU work | `example_e2e` | `examples/sandbox/instruction_budget.py` | `real_cpython_completes_while_mnpy_enforces_instruction_budget` |
| Call-depth budget stops finite recursion | `example_e2e` | `examples/sandbox/call_depth_budget.py` | `real_cpython_completes_while_mnpy_enforces_call_depth_budget` |
| Captured-output budget stops oversized output | `example_e2e` | `examples/sandbox/output_budget.py` | `real_cpython_completes_while_mnpy_enforces_output_budget` |
| VM allocation budget stops value growth | `example_e2e` | `examples/sandbox/allocation_budget.py` | `real_cpython_completes_while_mnpy_enforces_allocation_budget` |
| Parent wall-clock deadline covers pre-VM and VM work | `example_e2e` | `examples/sandbox/wall_clock_budget.py` | `real_cpython_completes_while_mnpy_enforces_wall_clock_budget` |
| Worker process memory contains compiler pressure | `example_e2e` | `examples/sandbox/compiler_memory_pressure_generator.py` | `real_cpython_completes_while_mnpy_contains_compiler_memory_pressure` |
| Script-directory imports work for safe modules | `example_e2e` | `examples/sandbox/import_root/main.py` | `real_cpython_and_mnpy_match_for_safe_script_directory_imports` |
| Imported source modules inherit policy | `example_e2e` | `examples/sandbox/blocked_import_root/main.py` | `real_mnpy_propagates_policy_into_script_directory_imports` |
| `sys.modules` injection cannot bypass policy | `example_e2e` | `examples/sandbox/cache_injection.py` | `real_cpython_accepts_cache_injection_while_mnpy_rechecks_policy` |
| `eval`, `exec`, and compiled code inherit policy | `example_e2e` | `examples/sandbox/dynamic_imports.py` | `real_cpython_allows_dynamic_imports_while_mnpy_reuses_the_sandbox_policy` |
| Module symlinks cannot escape the canonical root | `example_e2e` | `examples/sandbox/symlink_escape_main.py` | `real_cpython_follows_module_symlink_while_mnpy_rejects_root_escape` |
| Non-UTF-8 CLI input is rejected before parsing | `harness_only` | Not a valid `.py` text example | `sandbox_boundary_rejects_non_utf8_source_files` |
| Hidden worker mode has no public CLI bypass | `harness_only` | Launcher invariant, not Python behavior | `sandbox_process_rejects_direct_worker_invocation` |
| Worker crash or memory kill is reaped by the parent | `harness_only` | Process-status invariant | `sandbox_process_contains_compiler_memory_pressure` |

Every new sandbox requirement must enter this matrix. Prefer `example_e2e`;
use `harness_only` only when the condition cannot be expressed as valid Python
source. Each example E2E must run real executables and compare exit status,
stdout, stderr or exception category, and termination behavior as applicable.
