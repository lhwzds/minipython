# Sandbox Examples

Every Python file in this directory is both runnable documentation and the
exact source consumed by `tests/sandbox_examples.rs`.

```bash
mnpy examples/sandbox/blocked_host_capabilities.py
mnpy --max-steps 100 examples/sandbox/instruction_budget.py
mnpy --max-depth 3 examples/sandbox/call_depth_budget.py
mnpy --max-output-bytes 16 examples/sandbox/output_budget.py
mnpy --max-allocated-bytes 256 examples/sandbox/allocation_budget.py
```

The host-capability example completes and reports intentional blocks. Each
resource example completes under CPython and normal MiniPython defaults, but
the deliberately low limit shown above terminates it with a sandbox error.
