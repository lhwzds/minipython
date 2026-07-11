# Sandbox Runtime MVP Surface

This matrix defines the core runtime stopping point for the MiniPython sandbox
MVP. It does not relabel CPython-wide migration rows as complete. A row may stay
`partial` in `cpython_coverage.md` while its sandbox-safe MVP surface is accepted
here with explicit exclusions.

## Required Runtime Surface

| Area | MVP support | Explicitly excluded from MVP completion |
| --- | --- | --- |
| Numbers | Integer, bigint, bool, float, complex literals; common arithmetic, comparisons, conversion, formatting, and public numeric protocols | Locale-sensitive formatting, exhaustive platform floating-point internals, and every CPython diagnostic variant |
| Strings and interpolation | Unicode strings, bytes literals, common string methods, formatting, f-strings, t-strings, escapes, and public tokenize split output | CPython's parser-internal token representation, full codec registry, locale behavior, and lone-surrogate storage outside Rust `String` |
| Containers | List, tuple, dict, set, frozenset, comprehensions, iteration, slicing, unpacking, mutation, comparison, and recursive display | CPython allocation/performance stress contracts, C-level layout, GC tracking, and thread-safety internals |
| Object model | Classes, inheritance, MRO, descriptors, properties, bound methods, metaclasses used by the supported surface, and public type metadata | C layout compatibility, weakref/GC lifecycle parity, refcounts, and unrestricted metaclass/internal slot parity |
| Functions and scope | Positional/keyword binding, defaults, annotations, closures, globals, nonlocals, decorators, recursion under sandbox limits, generators, coroutines, and async generators | CPython frame/opcode identity, exact stack size, specialization, and debugger integration |
| Exceptions | Builtin hierarchy, custom exceptions, chaining, traceback objects used by the supported runtime, exception groups, and public attributes | CPython traceback/frame implementation identity and interpreter-shutdown lifecycle |
| Bytes and bytearray | Construction, indexing/slicing, search, split/join, case and predicate methods, translation, mutation, copying, and sandbox allocation behavior | Binary pickle-byte identity, exhaustive full-buffer exporters, locale behavior, and C ABI matrices |
| Array | Pure-memory construction and mutation for the documented typecodes, slicing, iteration, bytes/list conversion, copying, and BytesIO transfer | Real file APIs, pickle reconstruction internals, platform C ABI layout guarantees, and every unsupported typecode edge |
| Memoryview | One-dimensional bytes, bytearray, and supported array exporters; read/write, slicing, cast subset, attributes, release, hashing, and BytesIO `readinto` | Multidimensional subviews/casts, ctypes exporters, full buffer protocol, GC callback timing, and refcount matrices |
| Async runtime | `async def`, await, async iteration/context management, coroutine/generator protocols, and supported ABC behavior | Event loops, threads, signals, sockets, subprocess integration, and host scheduling |

## Coverage Partial Disposition

Every current `partial` row in `tests/cpython_coverage.md` must remain listed
here. `accepted_mvp` means the required public sandbox behavior is implemented;
the row remains partial only because broader CPython behavior is intentionally
not claimed.

| Coverage row | Sandbox status | MVP boundary |
| --- | --- | --- |
| `NUMBER` | `accepted_mvp` | Common numeric syntax/runtime is required; locale and exhaustive platform/error matrices are excluded |
| `STRING` | `accepted_mvp` | Common literals, escapes, bytes, f-strings, and t-strings are required; full codec/lone-surrogate behavior is excluded |
| `STRING_RUNTIME` | `accepted_mvp` | Documented pure-memory string/bytes methods and formatting are required; full codec registry and locale behavior are excluded |
| `CONTAINER_RUNTIME` | `accepted_mvp` | Core container construction, iteration, slicing, mutation, comparison, and display are required; stress/performance/internal layout is excluded |
| `COLLECTIONS_ABC_RUNTIME` | `accepted_mvp` | Documented structural checks and mixin behavior are required; unrestricted ABCMeta internals are excluded |
| `FSTRING_START` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `FSTRING_MIDDLE` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `FSTRING_END` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `TSTRING_START` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `TSTRING_MIDDLE` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `TSTRING_END` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `ERRORTOKEN` | `accepted_mvp` | Representative invalid characters, delimiters, strings, null bytes, and nesting errors are required; exhaustive tokenizer diagnostics are not |
| `ENCODING` | `accepted_mvp` | UTF-8, BOM/cookie detection, documented legacy labels, and decode errors are required; CPython's full codecs registry is excluded |

`OP` remains `out_of_scope_runtime`: it is a tokenizer/operator inventory row,
not a separate runtime implementation target.

## CPython Partial Group Disposition

| CPython group | Sandbox disposition |
| --- | --- |
| `test_builtin.py::BuiltinTest` | Keep public pure-memory builtins in scope; host `open`/`input`, filesystem, process/environment interaction, full pickle/deallocation details, and implementation optimization tests remain excluded |
| `test_builtin.py::TestType` | The current public dynamic type subset is accepted; lone-surrogate encoding branches outside Rust `String` remain excluded |
| `test_memoryview.py` direct methods | The documented one-dimensional public subset is accepted; multidimensional, ctypes, full buffer protocol, GC/refcount, and callback timing remain excluded |
| `test_bytes.py::BaseBytesTest` | The documented common bytes/bytearray surface is accepted; exhaustive buffer exporters, binary pickle identity, stress/performance, and every diagnostic permutation remain excluded |

## Completion Rule

This runtime surface is proven only when the complete language/runtime suite,
CPython subset/diff/inventory/manifest suites, and the sandbox MVP release gate
all pass. Untrusted source enters through the single public `mnpy` CLI, which combines the
VM budgets with child-process memory containment for compiler and host
allocations. New public runtime behavior must either gain direct evidence here
or be documented as excluded without crossing the host capability stop-line.
