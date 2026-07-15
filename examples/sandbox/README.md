# Sandbox Examples

Every Python file in this directory is both runnable documentation and the
exact source consumed by `tests/sandbox.rs`.

```bash
mnpy examples/sandbox/blocked_host_capabilities.py
mnpy examples/sandbox/safe_stdlib.py
mnpy --max-steps 100 examples/sandbox/instruction_budget.py
mnpy --max-source-bytes 8 examples/sandbox/source_size_budget.py
mnpy --max-depth 3 examples/sandbox/call_depth_budget.py
mnpy --max-output-bytes 16 examples/sandbox/output_budget.py
mnpy --max-allocated-bytes 256 examples/sandbox/allocation_budget.py
mnpy --max-time-ms 1 --max-steps 100000000 examples/sandbox/wall_clock_budget.py
python3 examples/sandbox/compiler_memory_pressure_generator.py | \
  mnpy --max-memory-bytes 67108864 --max-source-bytes 524288
mnpy examples/sandbox/import_root/main.py
mnpy examples/sandbox/blocked_import_root/main.py
mnpy examples/sandbox/cache_injection.py
mnpy examples/sandbox/dynamic_imports.py
```

The host-capability example completes and reports intentional blocks.
`safe_stdlib.py` exercises every module in the positive allowlist. Thirteen
modules produce byte-for-byte identical output; the example makes the one
versioned difference explicit because CPython 3.14.6 does not provide
`math.integer` while MiniPython does. Each
resource example completes under CPython and normal MiniPython defaults, but
the deliberately low limit shown above terminates it with a sandbox error. The
wall-clock example demonstrates the parent-process deadline, which also covers
time spent before VM instruction accounting begins.
The safe import-root example prints `7`. The blocked import-root and cache
injection examples show that policy is applied after loading local modules and
before accepting cached modules. `symlink_escape_main.py` and
`symlink_escape_target.py` are setup sources for the Unix E2E that places the
main file inside a temporary root and symlinks `escape.py` to the external
target.
The compiler-memory generator keeps the checked-in example readable while
producing the same finite 120,000-element literal used by the process-memory
containment E2E.
`dynamic_imports.py` proves that `eval`, `exec`, and compiled code reuse the
same import policy instead of creating an unrestricted nested VM.
