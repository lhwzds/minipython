# Sandbox Examples

Every Python file in this directory is both runnable documentation and the
exact source consumed by `tests/sandbox_examples.rs`.

```bash
mnpy examples/sandbox/blocked_host_capabilities.py
mnpy --max-steps 100 examples/sandbox/instruction_budget.py
mnpy --max-depth 3 examples/sandbox/call_depth_budget.py
mnpy --max-output-bytes 16 examples/sandbox/output_budget.py
mnpy --max-allocated-bytes 256 examples/sandbox/allocation_budget.py
mnpy examples/sandbox/import_root/main.py
mnpy examples/sandbox/blocked_import_root/main.py
mnpy examples/sandbox/cache_injection.py
```

The host-capability example completes and reports intentional blocks. Each
resource example completes under CPython and normal MiniPython defaults, but
the deliberately low limit shown above terminates it with a sandbox error.
The safe import-root example prints `7`. The blocked import-root and cache
injection examples show that policy is applied after loading local modules and
before accepting cached modules. `symlink_escape_main.py` and
`symlink_escape_target.py` are setup sources for the Unix E2E that places the
main file inside a temporary root and symlinks `escape.py` to the external
target.
