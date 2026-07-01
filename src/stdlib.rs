use crate::bytecode::Instruction;
use crate::lexer::{get_int_max_str_digits, set_int_max_str_digits};
use crate::value::{
    CodeMode, DictRef, DictStorage, DictViewKind, INT_SUBCLASS_STORAGE_FIELD, NamedTupleType,
    Scope, Value, dict_value, float_value, list_value, tuple_value,
};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) const PYCF_ONLY_AST: i64 = 0x0400;
pub(crate) const PYCF_ALLOW_TOP_LEVEL_AWAIT: i64 = 0x2000;
pub(crate) const PYCF_OPTIMIZED_AST: i64 = 0x8000 | PYCF_ONLY_AST;
pub(crate) const PICKLE_HIGHEST_PROTOCOL: i64 = 5;
pub(crate) const DIS_LOAD_CONST_OPCODE: i64 = 100;
pub(crate) const FUNCTOOLS_WRAPPER_ASSIGNMENTS: &[&str] = &[
    "__module__",
    "__name__",
    "__qualname__",
    "__doc__",
    "__annotate__",
    "__type_params__",
];
pub(crate) const FUNCTOOLS_WRAPPER_UPDATES: &[&str] = &["__dict__"];
pub(crate) const TYPES_ALL: &[&str] = &[
    "AsyncGeneratorType",
    "BuiltinFunctionType",
    "BuiltinMethodType",
    "CapsuleType",
    "CellType",
    "ClassMethodDescriptorType",
    "CodeType",
    "CoroutineType",
    "DynamicClassAttribute",
    "EllipsisType",
    "FrameType",
    "FunctionType",
    "GeneratorType",
    "GenericAlias",
    "GetSetDescriptorType",
    "LambdaType",
    "MappingProxyType",
    "MemberDescriptorType",
    "MethodDescriptorType",
    "MethodType",
    "MethodWrapperType",
    "ModuleType",
    "NoneType",
    "NotImplementedType",
    "SimpleNamespace",
    "TracebackType",
    "UnionType",
    "WrapperDescriptorType",
    "coroutine",
    "get_original_bases",
    "new_class",
    "prepare_class",
    "resolve_bases",
];
const COLLECTIONS_ABC_ALL: &[&str] = &[
    "Awaitable",
    "Coroutine",
    "AsyncIterable",
    "AsyncIterator",
    "AsyncGenerator",
    "Hashable",
    "Iterable",
    "Iterator",
    "Generator",
    "Reversible",
    "Sized",
    "Container",
    "Callable",
    "Collection",
    "Set",
    "MutableSet",
    "Mapping",
    "MutableMapping",
    "MappingView",
    "KeysView",
    "ItemsView",
    "ValuesView",
    "Sequence",
    "MutableSequence",
    "ByteString",
    "Buffer",
];
pub(crate) const SYS_BUILTIN_MODULE_NAMES: &[&str] = &["builtins", "sys", "time"];
pub(crate) const MINIPYTHON_VERSION_MAJOR: i64 = 0;
pub(crate) const MINIPYTHON_VERSION_MINOR: i64 = 1;
pub(crate) const MINIPYTHON_VERSION_MICRO: i64 = 0;
pub(crate) const MINIPYTHON_VERSION_RELEASELEVEL: &str = "final";
pub(crate) const MINIPYTHON_VERSION_SERIAL: i64 = 0;
pub(crate) const MINIPYTHON_HEXVERSION: i64 = 0x0001_00f0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SysFlags {
    pub bytes_warning: i64,
}

pub(crate) trait StdlibContext {
    fn stdlib_abs_value(&mut self, value: Value) -> Result<Value, String>;

    fn stdlib_add_values(&mut self, left: Value, right: Value) -> Result<Value, String>;

    fn stdlib_advance_iterator(
        &mut self,
        iterator: &mut Value,
    ) -> Result<StdlibIteratorAdvance, String>;

    fn stdlib_call_hash_method(
        &mut self,
        value: &Value,
    ) -> Result<Option<StdlibMethodCallResult>, String>;

    fn stdlib_call_repr_method(&mut self, value: &Value) -> Result<Option<Value>, String>;

    fn stdlib_call_value(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, String>;

    fn stdlib_ascii_repr_value(&self, value: &Value) -> Result<String, String>;

    fn stdlib_greater_values(&mut self, left: Value, right: Value) -> Result<bool, String>;

    fn stdlib_less_values(&mut self, left: Value, right: Value) -> Result<bool, String>;

    fn stdlib_divmod_values(&mut self, left: Value, right: Value)
    -> Result<(Value, Value), String>;

    fn stdlib_hash_value(&mut self, value: &Value) -> Result<Value, String>;

    fn stdlib_identity_value(&self, value: &Value) -> Value;

    fn stdlib_index_integer_value(&mut self, value: Value) -> Result<Value, String>;

    fn stdlib_is_callable(&self, value: &Value) -> bool;

    fn stdlib_iter_value(&mut self, value: Value) -> Result<Value, String>;

    fn stdlib_len_value(&mut self, value: Value) -> Result<usize, String>;

    fn stdlib_repr_value(&self, value: &Value) -> Result<String, String>;

    fn stdlib_truth_value(&mut self, value: Value) -> Result<bool, String>;

    fn stdlib_resolve_import_name_from_globals_value(
        &self,
        name: &str,
        level: usize,
        globals: Option<&Value>,
    ) -> Result<String, String>;

    fn stdlib_load_imported_module_value(
        &mut self,
        name: &str,
        return_root: bool,
    ) -> Result<Value, String>;

    fn stdlib_mark_iterable_coroutine_function(&mut self, identity: &Rc<()>);

    fn stdlib_count_elements_into_mapping(
        &mut self,
        mapping: Value,
        iterable: Value,
    ) -> Result<(), String>;
}

pub(crate) enum StdlibMethodCallResult {
    Returned(Value),
    Raised(Value),
}

pub(crate) enum StdlibIteratorAdvance {
    Yield(Value),
    Complete,
    Raised,
}

pub(crate) const DEFAULT_BUILTIN_ENTRY_NAMES: &[&str] = &[
    "Ellipsis",
    "NotImplemented",
    "__debug__",
    "__import__",
    "__build_class__",
    "print",
    "format",
    "eval",
    "exec",
    "compile",
    "breakpoint",
    "range",
    "next",
    "iter",
    "aiter",
    "anext",
    "len",
    "max",
    "min",
    "sum",
    "abs",
    "hash",
    "id",
    "divmod",
    "round",
    "bin",
    "oct",
    "hex",
    "pow",
    "any",
    "all",
    "sorted",
    "enumerate",
    "zip",
    "map",
    "filter",
    "reversed",
    "isinstance",
    "issubclass",
    "repr",
    "ascii",
    "chr",
    "ord",
    "getattr",
    "setattr",
    "delattr",
    "hasattr",
    "callable",
    "vars",
    "globals",
    "locals",
    "dir",
    "property",
    "super",
    "staticmethod",
    "classmethod",
    "slice",
    "int",
    "str",
    "bytes",
    "bytearray",
    "memoryview",
    "list",
    "dict",
    "tuple",
    "set",
    "frozenset",
    "float",
    "complex",
    "bool",
    "object",
    "type",
    "BaseException",
    "BaseExceptionGroup",
    "Exception",
    "ExceptionGroup",
    "GeneratorExit",
    "KeyboardInterrupt",
    "SystemExit",
    "ArithmeticError",
    "AssertionError",
    "AttributeError",
    "EOFError",
    "ImportError",
    "LookupError",
    "MemoryError",
    "NameError",
    "OSError",
    "ReferenceError",
    "RuntimeError",
    "StopAsyncIteration",
    "StopIteration",
    "SyntaxError",
    "SystemError",
    "TypeError",
    "ValueError",
    "Warning",
    "BlockingIOError",
    "FileExistsError",
    "FileNotFoundError",
    "InterruptedError",
    "IsADirectoryError",
    "NotADirectoryError",
    "PermissionError",
    "ProcessLookupError",
    "TimeoutError",
    "FloatingPointError",
    "OverflowError",
    "ZeroDivisionError",
    "IndexError",
    "KeyError",
    "ModuleNotFoundError",
    "NotImplementedError",
    "RecursionError",
    "IndentationError",
    "TabError",
    "UnicodeError",
    "UnicodeDecodeError",
    "UnicodeEncodeError",
    "UnicodeTranslateError",
    "BytesWarning",
    "DeprecationWarning",
    "EncodingWarning",
    "FutureWarning",
    "ImportWarning",
    "PendingDeprecationWarning",
    "ResourceWarning",
    "RuntimeWarning",
    "SyntaxWarning",
    "UnicodeWarning",
    "UserWarning",
];

pub(crate) const AST_MODULE_TYPE_NAMES: &[&str] = &[
    "AST",
    "mod",
    "stmt",
    "expr",
    "excepthandler",
    "pattern",
    "type_ignore",
    "type_param",
    "Module",
    "Expression",
    "Interactive",
    "FunctionType",
    "Pass",
    "Expr",
    "Assign",
    "AnnAssign",
    "TypeAlias",
    "AugAssign",
    "Delete",
    "FunctionDef",
    "AsyncFunctionDef",
    "ClassDef",
    "Import",
    "ImportFrom",
    "Return",
    "Global",
    "Nonlocal",
    "Assert",
    "Raise",
    "If",
    "Match",
    "Try",
    "TryStar",
    "With",
    "AsyncWith",
    "While",
    "For",
    "AsyncFor",
    "Break",
    "Continue",
    "Name",
    "Constant",
    "Attribute",
    "BinOp",
    "Compare",
    "UnaryOp",
    "BoolOp",
    "IfExp",
    "NamedExpr",
    "Yield",
    "YieldFrom",
    "Await",
    "Starred",
    "List",
    "ListComp",
    "SetComp",
    "GeneratorExp",
    "Tuple",
    "Dict",
    "DictComp",
    "Subscript",
    "Slice",
    "Call",
    "Lambda",
    "JoinedStr",
    "FormattedValue",
    "TemplateStr",
    "Interpolation",
    "expr_context",
    "boolop",
    "operator",
    "unaryop",
    "cmpop",
    "Load",
    "Store",
    "Del",
    "Add",
    "Sub",
    "Mult",
    "MatMult",
    "Div",
    "FloorDiv",
    "Mod",
    "Pow",
    "BitOr",
    "BitXor",
    "BitAnd",
    "LShift",
    "RShift",
    "Eq",
    "NotEq",
    "Lt",
    "LtE",
    "Gt",
    "GtE",
    "In",
    "NotIn",
    "Is",
    "IsNot",
    "Not",
    "UAdd",
    "USub",
    "Invert",
    "And",
    "Or",
    "arguments",
    "arg",
    "keyword",
    "alias",
    "ExceptHandler",
    "withitem",
    "comprehension",
    "match_case",
    "MatchValue",
    "MatchSingleton",
    "MatchAs",
    "MatchOr",
    "MatchSequence",
    "MatchMapping",
    "MatchClass",
    "MatchStar",
    "TypeVar",
    "TypeVarTuple",
    "ParamSpec",
    "TypeIgnore",
];

pub(crate) fn create_module(
    name: &str,
    sys_modules: Value,
    flags: SysFlags,
    import_dependency: &mut dyn FnMut(&str) -> Result<Value, String>,
) -> Result<Value, String> {
    match name {
        "test" => test_package_module(import_dependency),
        "test.typinganndata" => test_typinganndata_package_module(import_dependency),
        "test.typinganndata.ann_module" => Ok(test_typinganndata_ann_module()),
        "test.typinganndata.ann_module2" => Ok(test_typinganndata_ann_module2()),
        "test.typinganndata.ann_module3" => Ok(test_typinganndata_ann_module3()),
        "builtins" => Ok(builtins_module()),
        "sys" => Ok(module_value(
            "sys",
            vec![
                ("path", list_value(vec![Value::String(String::new())])),
                ("argv", list_value(Vec::new())),
                ("warnoptions", list_value(Vec::new())),
                ("dont_write_bytecode", Value::Bool(false)),
                (
                    "byteorder",
                    Value::String(
                        if cfg!(target_endian = "little") {
                            "little"
                        } else {
                            "big"
                        }
                        .to_string(),
                    ),
                ),
                ("implementation", sys_implementation_value()),
                (
                    "builtin_module_names",
                    string_tuple_value(SYS_BUILTIN_MODULE_NAMES),
                ),
                ("maxsize", Value::Number(i64::MAX)),
                ("float_repr_style", Value::String("short".to_string())),
                ("float_info", sys_float_info_value()),
                ("hash_info", sys_hash_info_value()),
                ("hexversion", Value::Number(MINIPYTHON_HEXVERSION)),
                ("stdin", stdio_stream_value("stdin")),
                ("stdout", stdio_stream_value("stdout")),
                ("stderr", stdio_stream_value("stderr")),
                (
                    "__breakpointhook__",
                    Value::Builtin("sys.__breakpointhook__".to_string()),
                ),
                (
                    "breakpointhook",
                    Value::Builtin("sys.__breakpointhook__".to_string()),
                ),
                ("version", Value::String("minipython".to_string())),
                ("version_info", sys_version_info_value()),
                (
                    "flags",
                    sys_structseq_value(
                        "flags",
                        vec![
                            "debug",
                            "inspect",
                            "interactive",
                            "optimize",
                            "dont_write_bytecode",
                            "no_user_site",
                            "no_site",
                            "ignore_environment",
                            "verbose",
                            "bytes_warning",
                            "quiet",
                            "hash_randomization",
                            "isolated",
                            "dev_mode",
                            "utf8_mode",
                            "warn_default_encoding",
                            "safe_path",
                            "int_max_str_digits",
                        ],
                        vec![
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(flags.bytes_warning),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Bool(false),
                            Value::Number(0),
                            Value::Number(0),
                            Value::Bool(false),
                            Value::Number(4300),
                        ],
                    ),
                ),
                (
                    "get_int_max_str_digits",
                    Value::Builtin("sys.get_int_max_str_digits".to_string()),
                ),
                ("exc_info", Value::Builtin("sys.exc_info".to_string())),
                (
                    "getdefaultencoding",
                    Value::Builtin("sys.getdefaultencoding".to_string()),
                ),
                (
                    "is_finalizing",
                    Value::Builtin("sys.is_finalizing".to_string()),
                ),
                (
                    "set_int_max_str_digits",
                    Value::Builtin("sys.set_int_max_str_digits".to_string()),
                ),
                ("_getframe", Value::Builtin("sys._getframe".to_string())),
                ("modules", sys_modules),
            ],
        )),
        "time" => Ok(module_value(
            "time",
            vec![
                ("time", Value::Builtin("time".to_string())),
                ("sleep", Value::Builtin("sleep".to_string())),
            ],
        )),
        "io" => Ok(module_value(
            "io",
            vec![
                ("BytesIO", Value::Builtin("io.BytesIO".to_string())),
                (
                    "UnsupportedOperation",
                    Value::Builtin("io.UnsupportedOperation".to_string()),
                ),
                ("SEEK_SET", Value::Number(0)),
                ("SEEK_CUR", Value::Number(1)),
                ("SEEK_END", Value::Number(2)),
            ],
        )),
        "math" => Ok(module_value(
            "math",
            vec![
                ("__package__", Value::String(String::new())),
                ("pi", float_value(std::f64::consts::PI)),
                ("e", float_value(std::f64::consts::E)),
                ("tau", float_value(std::f64::consts::TAU)),
                ("inf", float_value(f64::INFINITY)),
                ("nan", float_value(f64::NAN)),
                ("sqrt", Value::Builtin("sqrt".to_string())),
                ("isfinite", Value::Builtin("math.isfinite".to_string())),
                ("isinf", Value::Builtin("math.isinf".to_string())),
                ("isclose", Value::Builtin("math.isclose".to_string())),
                ("isnan", Value::Builtin("math.isnan".to_string())),
                ("isnormal", Value::Builtin("math.isnormal".to_string())),
                (
                    "issubnormal",
                    Value::Builtin("math.issubnormal".to_string()),
                ),
                ("dist", Value::Builtin("math.dist".to_string())),
                ("comb", Value::Builtin("math.comb".to_string())),
                ("factorial", Value::Builtin("math.factorial".to_string())),
                ("gcd", Value::Builtin("math.gcd".to_string())),
                ("hypot", Value::Builtin("math.hypot".to_string())),
                ("isqrt", Value::Builtin("math.isqrt".to_string())),
                ("lcm", Value::Builtin("math.lcm".to_string())),
                ("perm", Value::Builtin("math.perm".to_string())),
                ("pow", Value::Builtin("math.pow".to_string())),
                ("prod", Value::Builtin("math.prod".to_string())),
                ("sumprod", Value::Builtin("math.sumprod".to_string())),
                ("fabs", Value::Builtin("math.fabs".to_string())),
                ("fma", Value::Builtin("math.fma".to_string())),
                ("fmax", Value::Builtin("math.fmax".to_string())),
                ("fmin", Value::Builtin("math.fmin".to_string())),
                ("fmod", Value::Builtin("math.fmod".to_string())),
                ("frexp", Value::Builtin("math.frexp".to_string())),
                ("fsum", Value::Builtin("math.fsum".to_string())),
                ("ldexp", Value::Builtin("math.ldexp".to_string())),
                ("modf", Value::Builtin("math.modf".to_string())),
                ("nextafter", Value::Builtin("math.nextafter".to_string())),
                ("remainder", Value::Builtin("math.remainder".to_string())),
                ("copysign", Value::Builtin("math.copysign".to_string())),
                ("signbit", Value::Builtin("math.signbit".to_string())),
                ("trunc", Value::Builtin("math.trunc".to_string())),
                ("ulp", Value::Builtin("math.ulp".to_string())),
                ("ceil", Value::Builtin("math.ceil".to_string())),
                ("floor", Value::Builtin("math.floor".to_string())),
                ("degrees", Value::Builtin("math.degrees".to_string())),
                ("radians", Value::Builtin("math.radians".to_string())),
                ("cbrt", Value::Builtin("math.cbrt".to_string())),
                ("erf", Value::Builtin("math.erf".to_string())),
                ("erfc", Value::Builtin("math.erfc".to_string())),
                ("gamma", Value::Builtin("math.gamma".to_string())),
                ("lgamma", Value::Builtin("math.lgamma".to_string())),
                ("exp", Value::Builtin("math.exp".to_string())),
                ("exp2", Value::Builtin("math.exp2".to_string())),
                ("expm1", Value::Builtin("math.expm1".to_string())),
                ("log", Value::Builtin("math.log".to_string())),
                ("log1p", Value::Builtin("math.log1p".to_string())),
                ("log2", Value::Builtin("math.log2".to_string())),
                ("log10", Value::Builtin("math.log10".to_string())),
                ("acos", Value::Builtin("math.acos".to_string())),
                ("acosh", Value::Builtin("math.acosh".to_string())),
                ("asin", Value::Builtin("math.asin".to_string())),
                ("asinh", Value::Builtin("math.asinh".to_string())),
                ("atan", Value::Builtin("math.atan".to_string())),
                ("atan2", Value::Builtin("math.atan2".to_string())),
                ("atanh", Value::Builtin("math.atanh".to_string())),
                ("cos", Value::Builtin("math.cos".to_string())),
                ("cosh", Value::Builtin("math.cosh".to_string())),
                ("sin", Value::Builtin("math.sin".to_string())),
                ("sinh", Value::Builtin("math.sinh".to_string())),
                ("tan", Value::Builtin("math.tan".to_string())),
                ("tanh", Value::Builtin("math.tanh".to_string())),
            ],
        )),
        "math.integer" => Ok(module_value(
            "math.integer",
            vec![
                ("comb", Value::Builtin("math.integer.comb".to_string())),
                (
                    "factorial",
                    Value::Builtin("math.integer.factorial".to_string()),
                ),
                ("gcd", Value::Builtin("math.integer.gcd".to_string())),
                ("isqrt", Value::Builtin("math.integer.isqrt".to_string())),
                ("lcm", Value::Builtin("math.integer.lcm".to_string())),
                ("perm", Value::Builtin("math.integer.perm".to_string())),
            ],
        )),
        "os" => Ok(module_value(
            "os",
            vec![
                ("name", Value::String("posix".to_string())),
                ("path", import_dependency("os.path")?),
            ],
        )),
        "os.path" => Ok(module_value(
            "os.path",
            vec![("sep", Value::String("/".to_string()))],
        )),
        "re" => Ok(module_value(
            "re",
            vec![("findall", Value::Builtin("re.findall".to_string()))],
        )),
        "json" => Ok(module_value(
            "json",
            vec![
                ("__package__", Value::String("json".to_string())),
                ("loads", Value::Builtin("json.loads".to_string())),
                ("dumps", Value::Builtin("json.dumps".to_string())),
            ],
        )),
        "copy" => Ok(copy_module_value()),
        "weakref" => Ok(module_value(
            "weakref",
            vec![
                ("ref", Value::Builtin("weakref.ref".to_string())),
                ("proxy", Value::Builtin("weakref.proxy".to_string())),
                ("ReferenceType", builtin_type_value("weakref.ReferenceType")),
                ("ProxyType", builtin_type_value("weakref.ProxyType")),
                (
                    "CallableProxyType",
                    builtin_type_value("weakref.CallableProxyType"),
                ),
                (
                    "ProxyTypes",
                    tuple_value(vec![
                        builtin_type_value("weakref.ProxyType"),
                        builtin_type_value("weakref.CallableProxyType"),
                    ]),
                ),
            ],
        )),
        "unittest" => Ok(module_value(
            "unittest",
            vec![("mock", import_dependency("unittest.mock")?)],
        )),
        "unittest.mock" => Ok(module_value(
            "unittest.mock",
            vec![(
                "MagicMock",
                Value::Builtin("unittest.mock.MagicMock".to_string()),
            )],
        )),
        "_weakref" => Ok(module_value(
            "_weakref",
            vec![
                ("ref", Value::Builtin("weakref.ref".to_string())),
                ("proxy", Value::Builtin("weakref.proxy".to_string())),
                ("ReferenceType", builtin_type_value("weakref.ReferenceType")),
                ("ProxyType", builtin_type_value("weakref.ProxyType")),
                (
                    "CallableProxyType",
                    builtin_type_value("weakref.CallableProxyType"),
                ),
            ],
        )),
        "functools" => Ok(module_value(
            "functools",
            vec![
                ("__package__", Value::String(String::new())),
                (
                    "WRAPPER_ASSIGNMENTS",
                    string_tuple_value(FUNCTOOLS_WRAPPER_ASSIGNMENTS),
                ),
                (
                    "WRAPPER_UPDATES",
                    string_tuple_value(FUNCTOOLS_WRAPPER_UPDATES),
                ),
                (
                    "cmp_to_key",
                    Value::Builtin("functools.cmp_to_key".to_string()),
                ),
                ("cache", Value::Builtin("functools.cache".to_string())),
                (
                    "cached_property",
                    Value::Builtin("functools.cached_property".to_string()),
                ),
                (
                    "lru_cache",
                    Value::Builtin("functools.lru_cache".to_string()),
                ),
                ("partial", Value::Builtin("functools.partial".to_string())),
                (
                    "partialmethod",
                    Value::Builtin("functools.partialmethod".to_string()),
                ),
                ("reduce", Value::Builtin("functools.reduce".to_string())),
                (
                    "singledispatch",
                    Value::Builtin("functools.singledispatch".to_string()),
                ),
                (
                    "singledispatchmethod",
                    Value::Builtin("functools.singledispatchmethod".to_string()),
                ),
                (
                    "update_wrapper",
                    Value::Builtin("functools.update_wrapper".to_string()),
                ),
                (
                    "total_ordering",
                    Value::Builtin("functools.total_ordering".to_string()),
                ),
                ("wraps", Value::Builtin("functools.wraps".to_string())),
            ],
        )),
        "itertools" => Ok(module_value(
            "itertools",
            vec![
                ("__package__", Value::String(String::new())),
                (
                    "accumulate",
                    Value::Builtin("itertools.accumulate".to_string()),
                ),
                ("batched", Value::Builtin("itertools.batched".to_string())),
                ("chain", Value::Builtin("itertools.chain".to_string())),
                (
                    "combinations",
                    Value::Builtin("itertools.combinations".to_string()),
                ),
                (
                    "combinations_with_replacement",
                    Value::Builtin("itertools.combinations_with_replacement".to_string()),
                ),
                ("compress", Value::Builtin("itertools.compress".to_string())),
                ("count", Value::Builtin("itertools.count".to_string())),
                ("cycle", Value::Builtin("itertools.cycle".to_string())),
                (
                    "dropwhile",
                    Value::Builtin("itertools.dropwhile".to_string()),
                ),
                (
                    "filterfalse",
                    Value::Builtin("itertools.filterfalse".to_string()),
                ),
                ("groupby", Value::Builtin("itertools.groupby".to_string())),
                ("islice", Value::Builtin("itertools.islice".to_string())),
                ("pairwise", Value::Builtin("itertools.pairwise".to_string())),
                (
                    "permutations",
                    Value::Builtin("itertools.permutations".to_string()),
                ),
                ("product", Value::Builtin("itertools.product".to_string())),
                ("repeat", Value::Builtin("itertools.repeat".to_string())),
                ("starmap", Value::Builtin("itertools.starmap".to_string())),
                (
                    "takewhile",
                    Value::Builtin("itertools.takewhile".to_string()),
                ),
                ("tee", Value::Builtin("itertools.tee".to_string())),
                (
                    "zip_longest",
                    Value::Builtin("itertools.zip_longest".to_string()),
                ),
            ],
        )),
        "operator" => Ok(operator_module_value()),
        "decimal" => Ok(module_value(
            "decimal",
            vec![("Decimal", Value::Builtin("decimal.Decimal".to_string()))],
        )),
        "enum" => Ok(module_value(
            "enum",
            vec![
                ("IntEnum", builtin_type_value("enum.IntEnum")),
                ("StrEnum", builtin_type_value("enum.StrEnum")),
            ],
        )),
        "fractions" => Ok(module_value(
            "fractions",
            vec![("Fraction", Value::Builtin("fractions.Fraction".to_string()))],
        )),
        "pickle" => Ok(module_value(
            "pickle",
            vec![
                ("HIGHEST_PROTOCOL", Value::Number(PICKLE_HIGHEST_PROTOCOL)),
                ("dumps", Value::Builtin("pickle.dumps".to_string())),
                ("loads", Value::Builtin("pickle.loads".to_string())),
            ],
        )),
        "inspect" => Ok(module_value(
            "inspect",
            vec![
                (
                    "currentframe",
                    Value::Builtin("inspect.currentframe".to_string()),
                ),
                ("signature", Value::Builtin("inspect.signature".to_string())),
                ("CO_GENERATOR", Value::Number(0x0020)),
                ("CO_COROUTINE", Value::Number(0x0080)),
                ("CO_ITERABLE_COROUTINE", Value::Number(0x0100)),
                ("CO_ASYNC_GENERATOR", Value::Number(0x0200)),
            ],
        )),
        "dis" => Ok(module_value(
            "dis",
            vec![
                (
                    "hasconst",
                    list_value(vec![Value::Number(DIS_LOAD_CONST_OPCODE)]),
                ),
                (
                    "get_instructions",
                    Value::Builtin("dis.get_instructions".to_string()),
                ),
            ],
        )),
        "warnings" => Ok(module_value(
            "warnings",
            vec![
                (
                    "catch_warnings",
                    Value::Builtin("warnings.catch_warnings".to_string()),
                ),
                (
                    "simplefilter",
                    Value::Builtin("warnings.simplefilter".to_string()),
                ),
                (
                    "filterwarnings",
                    Value::Builtin("warnings.filterwarnings".to_string()),
                ),
                ("warn", Value::Builtin("warnings.warn".to_string())),
            ],
        )),
        "ast" => Ok(module_value("ast", ast_module_entries())),
        "annotationlib" => Ok(annotationlib_module()),
        "typing" => Ok(module_value(
            "typing",
            vec![
                ("Any", Value::Builtin("typing.Any".to_string())),
                ("Generic", builtin_type_value("Generic")),
                ("GenericAlias", builtin_type_value("GenericAlias")),
                ("Callable", Value::Builtin("typing.Callable".to_string())),
                ("Hashable", Value::Builtin("typing.Hashable".to_string())),
                ("BinaryIO", builtin_type_value("typing.BinaryIO")),
                ("ForwardRef", builtin_type_value("typing.ForwardRef")),
                ("IO", builtin_type_value("typing.IO")),
                ("List", Value::Builtin("typing.List".to_string())),
                ("Literal", Value::Builtin("typing.Literal".to_string())),
                (
                    "NamedTuple",
                    Value::Builtin("typing.NamedTuple".to_string()),
                ),
                ("NewType", builtin_type_value("typing.NewType")),
                ("NoDefault", Value::Builtin("typing.NoDefault".to_string())),
                ("NoReturn", Value::Builtin("typing.NoReturn".to_string())),
                ("Optional", Value::Builtin("typing.Optional".to_string())),
                ("Protocol", builtin_type_value("typing.Protocol")),
                ("TextIO", builtin_type_value("typing.TextIO")),
                ("Tuple", Value::Builtin("typing.Tuple".to_string())),
                ("TypedDict", Value::Builtin("typing.TypedDict".to_string())),
                ("Union", builtin_type_value("Union")),
                ("TypeVar", builtin_type_value("typing.TypeVar")),
                ("TypeVarTuple", builtin_type_value("typing.TypeVarTuple")),
                ("ParamSpec", builtin_type_value("typing.ParamSpec")),
                ("TypeAliasType", builtin_type_value("typing.TypeAliasType")),
                ("get_args", Value::Builtin("typing.get_args".to_string())),
                (
                    "get_type_hints",
                    Value::Builtin("typing.get_type_hints".to_string()),
                ),
                (
                    "get_origin",
                    Value::Builtin("typing.get_origin".to_string()),
                ),
            ],
        )),
        "_types" => Ok(types_accelerator_module()),
        "types" => Ok(types_module()),
        "collections" => Ok(module_value(
            "collections",
            vec![
                ("__package__", Value::String("collections".to_string())),
                ("ChainMap", Value::Builtin("ChainMap".to_string())),
                ("Counter", Value::Builtin("Counter".to_string())),
                (
                    "namedtuple",
                    Value::Builtin("collections.namedtuple".to_string()),
                ),
                (
                    "_count_elements",
                    Value::Builtin("collections._count_elements".to_string()),
                ),
                ("deque", Value::Builtin("deque".to_string())),
                ("defaultdict", Value::Builtin("defaultdict".to_string())),
                ("OrderedDict", Value::Builtin("OrderedDict".to_string())),
                ("UserList", Value::Builtin("UserList".to_string())),
                ("UserDict", Value::Builtin("UserDict".to_string())),
                ("UserString", Value::Builtin("UserString".to_string())),
                ("abc", import_dependency("collections.abc")?),
            ],
        )),
        "array" => Ok(module_value(
            "array",
            vec![
                ("array", Value::Builtin("array.array".to_string())),
                ("typecodes", Value::String("bBuwhHiIlLqQfd".to_string())),
            ],
        )),
        "collections.abc" => Ok(module_value(
            "collections.abc",
            vec![
                (
                    "__doc__",
                    Value::String(
                        "Abstract Base Classes (ABCs) for collections, according to PEP 3119.\n\nUnit tests are in test_collections.\n"
                            .to_string(),
                    ),
                ),
                ("__package__", Value::String(String::new())),
                ("__all__", string_list_value(COLLECTIONS_ABC_ALL)),
                ("Hashable", Value::Builtin("Hashable".to_string())),
                ("Iterable", Value::Builtin("Iterable".to_string())),
                ("Iterator", Value::Builtin("Iterator".to_string())),
                ("Generator", Value::Builtin("Generator".to_string())),
                ("Reversible", Value::Builtin("Reversible".to_string())),
                ("Awaitable", Value::Builtin("Awaitable".to_string())),
                ("Coroutine", Value::Builtin("Coroutine".to_string())),
                ("AsyncIterable", Value::Builtin("AsyncIterable".to_string())),
                ("AsyncIterator", Value::Builtin("AsyncIterator".to_string())),
                (
                    "AsyncGenerator",
                    Value::Builtin("AsyncGenerator".to_string()),
                ),
                ("Sized", Value::Builtin("Sized".to_string())),
                ("Container", Value::Builtin("Container".to_string())),
                ("Callable", Value::Builtin("Callable".to_string())),
                ("Collection", Value::Builtin("Collection".to_string())),
                ("Buffer", Value::Builtin("Buffer".to_string())),
                ("Sequence", Value::Builtin("Sequence".to_string())),
                (
                    "MutableSequence",
                    Value::Builtin("MutableSequence".to_string()),
                ),
                ("ByteString", Value::Builtin("ByteString".to_string())),
                ("Mapping", Value::Builtin("Mapping".to_string())),
                (
                    "MutableMapping",
                    Value::Builtin("MutableMapping".to_string()),
                ),
                ("MappingView", Value::Builtin("MappingView".to_string())),
                ("KeysView", Value::Builtin("KeysView".to_string())),
                ("ItemsView", Value::Builtin("ItemsView".to_string())),
                ("ValuesView", Value::Builtin("ValuesView".to_string())),
                ("Set", Value::Builtin("Set".to_string())),
                ("MutableSet", Value::Builtin("MutableSet".to_string())),
            ],
        )),
        "string" => Ok(module_value(
            "string",
            vec![("templatelib", import_dependency("string.templatelib")?)],
        )),
        "string.templatelib" => Ok(module_value(
            "string.templatelib",
            vec![
                ("Template", Value::Builtin("Template".to_string())),
                ("Interpolation", Value::Builtin("Interpolation".to_string())),
                (
                    "convert",
                    Value::Builtin("string.templatelib.convert".to_string()),
                ),
            ],
        )),
        _ => Err(format!("ModuleNotFoundError: No module named '{name}'")),
    }
}

pub(crate) fn import_from(module: Value, name: &str) -> Result<Value, String> {
    match module {
        Value::Module {
            name: module_name,
            attrs,
        } => attrs.borrow().get(name).cloned().ok_or_else(|| {
            format!("ImportError: cannot import name '{name}' from '{module_name}'")
        }),
        value => Err(format!(
            "ImportError: cannot import name '{name}' from {value}"
        )),
    }
}

pub(crate) fn call_sys_get_int_max_str_digits(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err(
            "TypeError: sys.get_int_max_str_digits() takes no keyword arguments".to_string(),
        );
    }
    if !args.is_empty() {
        return Err(format!(
            "TypeError: sys.get_int_max_str_digits() takes no arguments ({} given)",
            args.len()
        ));
    }

    Ok(Value::Number(get_int_max_str_digits() as i64))
}

pub(crate) fn call_sys_getdefaultencoding(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("TypeError: sys.getdefaultencoding() takes no keyword arguments".to_string());
    }
    if !args.is_empty() {
        return Err(format!(
            "TypeError: sys.getdefaultencoding() takes no arguments ({} given)",
            args.len()
        ));
    }

    Ok(Value::String("utf-8".to_string()))
}

pub(crate) fn call_sys_is_finalizing(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("TypeError: sys.is_finalizing() takes no keyword arguments".to_string());
    }
    if !args.is_empty() {
        return Err(format!(
            "TypeError: sys.is_finalizing() takes no arguments ({} given)",
            args.len()
        ));
    }

    Ok(Value::Bool(false))
}

pub(crate) fn call_sys_set_int_max_str_digits(
    mut args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    for (keyword, value) in keywords {
        if keyword != "maxdigits" {
            return Err(format!(
                "TypeError: set_int_max_str_digits() got an unexpected keyword argument '{keyword}'"
            ));
        }
        args.push(value);
    }
    if args.len() != 1 {
        return Err(if args.is_empty() {
            "TypeError: set_int_max_str_digits() missing required argument 'maxdigits' (pos 1)"
                .to_string()
        } else {
            format!(
                "TypeError: set_int_max_str_digits() takes at most 1 argument ({} given)",
                args.len()
            )
        });
    }

    let maxdigits = match args.into_iter().next().expect("length checked") {
        Value::Number(value) if value >= 0 => value as usize,
        Value::Bool(value) => bool_as_i64(value) as usize,
        Value::BigInt(value) if !value.is_negative() => value
            .to_usize()
            .ok_or_else(|| "ValueError: maxdigits is too large".to_string())?,
        Value::Number(_) | Value::BigInt(_) => {
            return Err("ValueError: maxdigits must be >= 640 or 0 for unlimited".to_string());
        }
        value => {
            return Err(format!(
                "TypeError: '{}' object cannot be interpreted as an integer",
                stdlib_sys_type_name(&value)
            ));
        }
    };

    set_int_max_str_digits(maxdigits)?;
    Ok(Value::None)
}

pub(crate) fn call_dis_get_instructions(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("TypeError: get_instructions() does not accept keyword arguments".to_string());
    }
    if args.len() != 1 {
        return Err(format!(
            "TypeError: get_instructions() expected 1 argument, got {}",
            args.len()
        ));
    }

    let code = args.into_iter().next().expect("length checked");
    let Value::CodeObject {
        mode, instructions, ..
    } = code
    else {
        return Err(format!(
            "TypeError: don't know how to disassemble {} objects",
            stdlib_type_name(&code)
        ));
    };

    let mut rows = Vec::new();
    for instruction in instructions {
        if let Instruction::LoadConst { value, .. } = instruction {
            rows.push(dis_instruction_value(DIS_LOAD_CONST_OPCODE, value)?);
        }
    }

    if mode == CodeMode::Exec {
        rows.push(dis_instruction_value(DIS_LOAD_CONST_OPCODE, Value::None)?);
    }

    Ok(list_value(rows))
}

pub(crate) fn call_code_lines(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("co_lines() does not accept keyword arguments".to_string());
    }
    let [code] = args.as_slice() else {
        return Err(format!(
            "co_lines() expected 0 arguments, got {}",
            args.len().saturating_sub(1)
        ));
    };
    let Value::CodeObject { line_spans, .. } = code else {
        return Err(format!(
            "TypeError: descriptor 'co_lines' for 'code' objects doesn't apply to '{}'",
            stdlib_type_name(code)
        ));
    };
    let items = line_spans
        .iter()
        .map(|span| {
            tuple_value(vec![
                Value::Number(span.start as i64),
                Value::Number(span.end as i64),
                Value::Number(span.line),
            ])
        })
        .collect();
    Ok(stdlib_list_iterator_from_values(items))
}

pub(crate) fn call_code_positions(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("co_positions() does not accept keyword arguments".to_string());
    }
    let [code] = args.as_slice() else {
        return Err(format!(
            "co_positions() expected 0 arguments, got {}",
            args.len().saturating_sub(1)
        ));
    };
    let Value::CodeObject { line_spans, .. } = code else {
        return Err(format!(
            "TypeError: descriptor 'co_positions' for 'code' objects doesn't apply to '{}'",
            stdlib_type_name(code)
        ));
    };
    let items = line_spans
        .iter()
        .map(|span| {
            tuple_value(vec![
                Value::Number(span.line),
                Value::Number(span.end_line),
                span.column.map(Value::Number).unwrap_or(Value::None),
                span.end_column.map(Value::Number).unwrap_or(Value::None),
            ])
        })
        .collect();
    Ok(stdlib_list_iterator_from_values(items))
}

pub(crate) fn call_collections_count_elements<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("_count_elements", &keywords)?;
    let [mapping, iterable] = args.as_slice() else {
        return Err(format!(
            "_count_elements() expected 2 arguments, got {}",
            args.len()
        ));
    };
    context.stdlib_count_elements_into_mapping(mapping.clone(), iterable.clone())?;
    Ok(Value::None)
}

pub(crate) fn call_id<C: StdlibContext + ?Sized>(
    context: &C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("id", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: id() expected 1 argument, got {}",
            args.len()
        ));
    };

    Ok(context.stdlib_identity_value(value))
}

pub(crate) fn call_divmod_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("TypeError: divmod() takes no keyword arguments".to_string());
    }
    let [left, right] = args.as_slice() else {
        return Err(format!(
            "TypeError: divmod expected 2 arguments, got {}",
            args.len()
        ));
    };

    let (quotient, remainder) = context.stdlib_divmod_values(left.clone(), right.clone())?;
    Ok(tuple_value(vec![quotient, remainder]))
}

pub(crate) fn call_any_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("any", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: any() expected 1 argument, got {}",
            args.len()
        ));
    };

    let mut iterator = context.stdlib_iter_value(value.clone())?;
    loop {
        match context.stdlib_advance_iterator(&mut iterator)? {
            StdlibIteratorAdvance::Yield(value) => {
                if context.stdlib_truth_value(value)? {
                    return Ok(Value::Bool(true));
                }
            }
            StdlibIteratorAdvance::Complete | StdlibIteratorAdvance::Raised => {
                return Ok(Value::Bool(false));
            }
        }
    }
}

pub(crate) fn call_all_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("all", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: all() expected 1 argument, got {}",
            args.len()
        ));
    };

    let mut iterator = context.stdlib_iter_value(value.clone())?;
    loop {
        match context.stdlib_advance_iterator(&mut iterator)? {
            StdlibIteratorAdvance::Yield(value) => {
                if !context.stdlib_truth_value(value)? {
                    return Ok(Value::Bool(false));
                }
            }
            StdlibIteratorAdvance::Complete | StdlibIteratorAdvance::Raised => {
                return Ok(Value::Bool(true));
            }
        }
    }
}

pub(crate) fn call_min_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    call_minmax_builtin(context, "min", args, keywords, false)
}

pub(crate) fn call_max_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    call_minmax_builtin(context, "max", args, keywords, true)
}

fn call_minmax_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    name: &str,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
    choose_max: bool,
) -> Result<Value, String> {
    let (options, unexpected_keyword) = minmax_options(name, keywords)?;
    if args.is_empty() {
        return Err(format!(
            "TypeError: {name} expected at least 1 argument, got 0"
        ));
    }

    if let Some(keyword) = unexpected_keyword {
        return Err(format!(
            "TypeError: {name}() got an unexpected keyword argument '{keyword}'"
        ));
    }

    if args.len() > 1 {
        if options.default.is_some() {
            return Err(format!(
                "TypeError: Cannot specify a default for {name}() with multiple positional arguments"
            ));
        }
        return minmax_values(context, name, args, options, choose_max);
    }

    let [iterable] = args.as_slice() else {
        unreachable!("minmax arity was checked above");
    };
    minmax_iterable(context, name, iterable.clone(), options, choose_max)
}

#[derive(Default)]
struct MinMaxOptions {
    key: Option<Value>,
    default: Option<Value>,
}

fn minmax_options(
    name: &str,
    keywords: Vec<(String, Value)>,
) -> Result<(MinMaxOptions, Option<String>), String> {
    let mut options = MinMaxOptions::default();
    let mut unexpected_keyword = None;
    for (keyword, value) in keywords {
        match keyword.as_str() {
            "key" => {
                if options.key.is_some() {
                    return Err(format!(
                        "TypeError: {name}() got multiple values for keyword argument 'key'"
                    ));
                }
                options.key = Some(value);
            }
            "default" => {
                if options.default.is_some() {
                    return Err(format!(
                        "TypeError: {name}() got multiple values for keyword argument 'default'"
                    ));
                }
                options.default = Some(value);
            }
            _ => {
                unexpected_keyword.get_or_insert(keyword);
            }
        }
    }
    Ok((options, unexpected_keyword))
}

fn minmax_iterable<C: StdlibContext + ?Sized>(
    context: &mut C,
    name: &str,
    iterable: Value,
    options: MinMaxOptions,
    choose_max: bool,
) -> Result<Value, String> {
    let mut iterator = context.stdlib_iter_value(iterable)?;
    let mut best_value = loop {
        match context.stdlib_advance_iterator(&mut iterator)? {
            StdlibIteratorAdvance::Yield(value) => break value,
            StdlibIteratorAdvance::Complete | StdlibIteratorAdvance::Raised => {
                return match options.default {
                    Some(default) => Ok(default),
                    None => Err(format!("ValueError: {name}() arg is an empty sequence")),
                };
            }
        }
    };

    let mut best_key = minmax_key_value(context, options.key.as_ref(), &best_value)?;
    loop {
        match context.stdlib_advance_iterator(&mut iterator)? {
            StdlibIteratorAdvance::Yield(value) => {
                let candidate_key = minmax_key_value(context, options.key.as_ref(), &value)?;
                if minmax_should_replace(context, &candidate_key, &best_key, choose_max)? {
                    best_value = value;
                    best_key = candidate_key;
                }
            }
            StdlibIteratorAdvance::Complete | StdlibIteratorAdvance::Raised => {
                return Ok(best_value);
            }
        }
    }
}

fn minmax_values<C: StdlibContext + ?Sized>(
    context: &mut C,
    name: &str,
    values: Vec<Value>,
    options: MinMaxOptions,
    choose_max: bool,
) -> Result<Value, String> {
    let mut values = values.into_iter();
    let Some(mut best_value) = values.next() else {
        return match options.default {
            Some(default) => Ok(default),
            None => Err(format!("ValueError: {name}() arg is an empty sequence")),
        };
    };
    let mut best_key = minmax_key_value(context, options.key.as_ref(), &best_value)?;

    for value in values {
        let candidate_key = minmax_key_value(context, options.key.as_ref(), &value)?;
        if minmax_should_replace(context, &candidate_key, &best_key, choose_max)? {
            best_value = value;
            best_key = candidate_key;
        }
    }

    Ok(best_value)
}

fn minmax_key_value<C: StdlibContext + ?Sized>(
    context: &mut C,
    key: Option<&Value>,
    value: &Value,
) -> Result<Value, String> {
    match key {
        None | Some(Value::None) => Ok(value.clone()),
        Some(key) => context.stdlib_call_value(key.clone(), vec![value.clone()]),
    }
}

fn minmax_should_replace<C: StdlibContext + ?Sized>(
    context: &mut C,
    candidate_key: &Value,
    best_key: &Value,
    choose_max: bool,
) -> Result<bool, String> {
    if choose_max {
        context.stdlib_greater_values(candidate_key.clone(), best_key.clone())
    } else {
        context.stdlib_less_values(candidate_key.clone(), best_key.clone())
    }
}

pub(crate) fn call_sum_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("TypeError: sum() takes at least 1 positional argument (0 given)".to_string());
    }

    let keyword_count = keywords.len();
    let mut start_keyword = None;
    let mut unexpected_keyword = None;
    for (keyword, value) in keywords {
        if keyword != "start" {
            unexpected_keyword.get_or_insert(keyword);
            continue;
        }
        if start_keyword.is_some() {
            return Err(
                "TypeError: sum() got multiple values for keyword argument 'start'".to_string(),
            );
        }
        start_keyword = Some(value);
    }

    let supplied_count = args.len() + keyword_count;
    if supplied_count > 2 {
        return Err(format!(
            "TypeError: sum() takes at most 2 arguments ({supplied_count} given)"
        ));
    }

    if let Some(keyword) = unexpected_keyword {
        return Err(format!(
            "TypeError: sum() got an unexpected keyword argument '{keyword}'"
        ));
    }

    let (iterable, start) = match args.as_slice() {
        [iterable] => (iterable.clone(), start_keyword.unwrap_or(Value::Number(0))),
        [iterable, start] => (iterable.clone(), start.clone()),
        values => {
            return Err(format!(
                "TypeError: sum() takes at most 2 arguments ({} given)",
                values.len()
            ));
        }
    };

    reject_sum_start(&start)?;

    let mut total = start;
    let mut iterator = context.stdlib_iter_value(iterable)?;
    loop {
        match context.stdlib_advance_iterator(&mut iterator)? {
            StdlibIteratorAdvance::Yield(value) => {
                if matches!(value, Value::String(_) | Value::IdentityString { .. }) {
                    return Err("TypeError: sum() can't sum strings".to_string());
                }
                total = context.stdlib_add_values(total, value)?;
            }
            StdlibIteratorAdvance::Complete | StdlibIteratorAdvance::Raised => return Ok(total),
        }
    }
}

fn reject_sum_start(value: &Value) -> Result<(), String> {
    match value {
        Value::String(_) | Value::IdentityString { .. } => {
            Err("TypeError: sum() can't sum strings [use ''.join(seq) instead]".to_string())
        }
        Value::Bytes(_) => {
            Err("TypeError: sum() can't sum bytes [use b''.join(seq) instead]".to_string())
        }
        Value::ByteArray(_) => {
            Err("TypeError: sum() can't sum bytearray [use b''.join(seq) instead]".to_string())
        }
        _ => Ok(()),
    }
}

fn sys_version_info_value() -> Value {
    let fields = vec![
        "major".to_string(),
        "minor".to_string(),
        "micro".to_string(),
        "releaselevel".to_string(),
        "serial".to_string(),
    ];
    let values = vec![
        Value::Number(MINIPYTHON_VERSION_MAJOR),
        Value::Number(MINIPYTHON_VERSION_MINOR),
        Value::Number(MINIPYTHON_VERSION_MICRO),
        Value::String(MINIPYTHON_VERSION_RELEASELEVEL.to_string()),
        Value::Number(MINIPYTHON_VERSION_SERIAL),
    ];
    Value::NamedTuple {
        typ: Rc::new(NamedTupleType {
            name: "version_info".to_string(),
            fields,
            bases: vec![builtin_type_value("tuple")],
            original_bases: None,
            field_docs: (0..5)
                .map(|index| RefCell::new(format!("Alias for field number {index}")))
                .collect(),
            field_defaults: Vec::new(),
            new_defaults: None,
            module: Value::String("sys".to_string()),
            doc: RefCell::new(
                "sys.version_info\n\nVersion information as a named tuple.".to_string(),
            ),
            identity: Rc::new(()),
        }),
        values: Rc::new(values),
    }
}

fn sys_implementation_value() -> Value {
    Value::SimpleNamespace {
        fields: stdlib_dict_ref_from_entries(vec![
            (
                Value::String("name".to_string()),
                Value::String("minipython".to_string()),
            ),
            (
                Value::String("cache_tag".to_string()),
                Value::String("minipython-0.1".to_string()),
            ),
            (
                Value::String("version".to_string()),
                sys_version_info_value(),
            ),
            (
                Value::String("hexversion".to_string()),
                Value::Number(MINIPYTHON_HEXVERSION),
            ),
        ]),
    }
}

pub(crate) fn call_import_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    let mut values = bind_import_args(args, keywords)?;
    let name = match values[0]
        .take()
        .ok_or_else(|| "TypeError: __import__() missing required argument 'name'".to_string())?
    {
        Value::String(name) | Value::IdentityString { value: name, .. } => name,
        value => {
            return Err(format!(
                "TypeError: __import__() argument 1 must be str, not {}",
                stdlib_type_name(&value)
            ));
        }
    };
    let globals_arg = values[1].take();
    let fromlist = values[3].take().unwrap_or(Value::None);
    let level = values[4]
        .take()
        .map(import_level_argument)
        .transpose()?
        .unwrap_or(0);
    if level == 0 && name.is_empty() {
        return Err("ValueError: Empty module name".to_string());
    }
    let resolved_name = context.stdlib_resolve_import_name_from_globals_value(
        &name,
        level as usize,
        globals_arg.as_ref(),
    )?;
    let return_root = level == 0 && !context.stdlib_truth_value(fromlist)?;

    context.stdlib_load_imported_module_value(&resolved_name, return_root)
}

fn sys_float_info_value() -> Value {
    sys_structseq_value(
        "float_info",
        vec![
            "max",
            "max_exp",
            "max_10_exp",
            "min",
            "min_exp",
            "min_10_exp",
            "dig",
            "mant_dig",
            "epsilon",
            "radix",
            "rounds",
        ],
        vec![
            float_value(f64::MAX),
            Value::Number(1024),
            Value::Number(308),
            float_value(f64::MIN_POSITIVE),
            Value::Number(-1021),
            Value::Number(-307),
            Value::Number(15),
            Value::Number(53),
            float_value(f64::EPSILON),
            Value::Number(2),
            Value::Number(1),
        ],
    )
}

fn sys_hash_info_value() -> Value {
    sys_structseq_value(
        "hash_info",
        vec![
            "width",
            "modulus",
            "inf",
            "nan",
            "imag",
            "algorithm",
            "hash_bits",
            "seed_bits",
            "cutoff",
        ],
        vec![
            Value::Number(64),
            Value::Number(2_305_843_009_213_693_951),
            Value::Number(314_159),
            Value::Number(0),
            Value::Number(1_000_003),
            Value::String("siphash13".to_string()),
            Value::Number(64),
            Value::Number(128),
            Value::Number(0),
        ],
    )
}

fn sys_structseq_value(name: &str, fields: Vec<&str>, values: Vec<Value>) -> Value {
    Value::NamedTuple {
        typ: Rc::new(NamedTupleType {
            name: name.to_string(),
            fields: fields.iter().map(|field| (*field).to_string()).collect(),
            bases: vec![builtin_type_value("tuple")],
            original_bases: None,
            field_docs: (0..values.len())
                .map(|index| RefCell::new(format!("Alias for field number {index}")))
                .collect(),
            field_defaults: Vec::new(),
            new_defaults: None,
            module: Value::String("sys".to_string()),
            doc: RefCell::new(sys_structseq_doc(name, &fields).to_string()),
            identity: Rc::new(()),
        }),
        values: Rc::new(values),
    }
}

fn sys_structseq_doc(name: &str, fields: &[&str]) -> String {
    match name {
        "float_info" => {
            "sys.float_info\n\nA named tuple holding information about the float type.".to_string()
        }
        "hash_info" => {
            "hash_info\n\nA named tuple providing parameters used for computing hashes.".to_string()
        }
        "flags" => {
            "sys.flags\n\nFlags provided through command line arguments or environment vars."
                .to_string()
        }
        _ => format!("{name}({})", fields.join(", ")),
    }
}

pub(crate) fn call_abs_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("abs", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: abs() expected 1 argument, got {}",
            args.len()
        ));
    };

    context.stdlib_abs_value(value.clone())
}

pub(crate) fn call_hash_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("hash", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: hash() expected 1 argument, got {}",
            args.len()
        ));
    };

    if let Value::MappingProxyObject { mapping, .. } = value {
        return call_hash_builtin(context, vec![mapping.as_ref().clone()], Vec::new());
    }

    if let Some(result) = context.stdlib_call_hash_method(value)? {
        return match result {
            StdlibMethodCallResult::Returned(value) => hash_result_from_special_method(value),
            StdlibMethodCallResult::Raised(value) => Ok(value),
        };
    }

    context.stdlib_hash_value(value)
}

pub(crate) fn call_repr_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("repr", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: repr() expected 1 argument, got {}",
            args.len()
        ));
    };

    if let Some(result) = context.stdlib_call_repr_method(value)? {
        return match result {
            Value::String(value) | Value::IdentityString { value, .. } => Ok(Value::String(value)),
            value => Err(format!(
                "TypeError: __repr__ returned non-string (type {})",
                stdlib_type_name(&value)
            )),
        };
    }

    Ok(Value::String(context.stdlib_repr_value(value)?))
}

fn hash_result_from_special_method(value: Value) -> Result<Value, String> {
    if let Some(value) = stdlib_int_subclass_integer(&value) {
        return hash_result_from_special_method(value);
    }
    match stdlib_numeric_bool_value(value) {
        Value::Number(value) => Ok(Value::Number(value)),
        Value::BigInt(value) => Ok(normalize_stdlib_big_int(value)),
        _ => Err("TypeError: __hash__ method should return an integer".to_string()),
    }
}

fn stdlib_numeric_bool_value(value: Value) -> Value {
    match value {
        Value::Bool(value) => Value::Number(i64::from(value)),
        value => value,
    }
}

fn stdlib_int_subclass_integer(value: &Value) -> Option<Value> {
    let Value::Instance {
        fields,
        class_bases,
        ..
    } = value
    else {
        return None;
    };
    if !stdlib_class_bases_include_builtin(class_bases, "int") {
        return None;
    }
    match fields.borrow().get(INT_SUBCLASS_STORAGE_FIELD).cloned() {
        Some(value @ (Value::Number(_) | Value::BigInt(_))) => Some(value),
        _ => None,
    }
}

fn stdlib_class_bases_include_builtin(bases: &[Value], target_name: &str) -> bool {
    bases
        .iter()
        .any(|base| stdlib_class_inherits_builtin(base, target_name))
}

fn stdlib_class_inherits_builtin(value: &Value, target_name: &str) -> bool {
    match value {
        Value::Builtin(name) => name == target_name || (name == "bool" && target_name == "int"),
        Value::Class { bases, .. } => stdlib_class_bases_include_builtin(bases, target_name),
        _ => false,
    }
}

fn normalize_stdlib_big_int(value: BigInt) -> Value {
    value
        .to_i64()
        .map(Value::Number)
        .unwrap_or(Value::BigInt(value))
}

pub(crate) fn call_bool_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("bool", &keywords)?;
    match args.as_slice() {
        [] => Ok(Value::Bool(false)),
        [value] => Ok(Value::Bool(context.stdlib_truth_value(value.clone())?)),
        values => Err(format!(
            "bool() expected at most 1 argument, got {}",
            values.len()
        )),
    }
}

pub(crate) fn call_len_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("len", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: len() expected 1 argument, got {}",
            args.len()
        ));
    };

    let len = context.stdlib_len_value(value.clone())?;
    let len =
        i64::try_from(len).map_err(|_| "OverflowError: len() result is too large".to_string())?;
    Ok(Value::Number(len))
}

pub(crate) fn call_int_base_builtin<C: StdlibContext + ?Sized>(
    context: &mut C,
    name: &str,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err(format!("TypeError: {name}() takes no keyword arguments"));
    }
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: {name}() expected 1 argument, got {}",
            args.len()
        ));
    };

    let value = context.stdlib_index_integer_value(value.clone())?;
    let value = match value {
        Value::Number(value) => BigInt::from(value),
        Value::BigInt(value) => value,
        _ => unreachable!("stdlib_index_integer_value returns an integer"),
    };
    let (prefix, radix) = match name {
        "bin" => ("0b", 2),
        "oct" => ("0o", 8),
        "hex" => ("0x", 16),
        _ => unreachable!("caller filters supported integer-base builtins"),
    };

    Ok(Value::String(format_prefixed_integer(
        &value, prefix, radix,
    )))
}

pub(crate) fn call_ascii<C: StdlibContext + ?Sized>(
    context: &C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("ascii", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: ascii() expected 1 argument, got {}",
            args.len()
        ));
    };

    Ok(Value::String(context.stdlib_ascii_repr_value(value)?))
}

pub(crate) fn call_chr<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("chr", &keywords)
        .map_err(|_| "TypeError: chr() takes no keyword arguments".to_string())?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: chr() expected 1 argument, got {}",
            args.len()
        ));
    };

    let value = context.stdlib_index_integer_value(value.clone())?;
    let codepoint = match value {
        Value::Number(value) if (0..=0x10ffff).contains(&value) => Some(value as u32),
        Value::BigInt(value) => value.to_u32().filter(|value| *value <= 0x10ffff),
        _ => None,
    };
    let Some(ch) = codepoint.and_then(char::from_u32) else {
        return Err("ValueError: chr() arg not in range(0x110000)".to_string());
    };

    Ok(Value::String(ch.to_string()))
}

pub(crate) fn call_hashable_hash(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("__hash__() does not accept keyword arguments".to_string());
    }
    let [_receiver] = args.as_slice() else {
        return Err(format!(
            "__hash__() expected 0 arguments, got {}",
            args.len().saturating_sub(1)
        ));
    };

    Ok(Value::Number(0))
}

pub(crate) fn call_staticmethod_constructor(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("staticmethod() takes no keyword arguments".to_string());
    }
    let [function] = args.as_slice() else {
        return Err(format!(
            "staticmethod expected 1 argument, got {}",
            args.len()
        ));
    };

    Ok(Value::StaticMethod {
        function: Box::new(function.clone()),
        identity: Rc::new(()),
    })
}

pub(crate) fn call_classmethod_constructor(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("classmethod() takes no keyword arguments".to_string());
    }
    let [function] = args.as_slice() else {
        return Err(format!(
            "classmethod expected 1 argument, got {}",
            args.len()
        ));
    };

    Ok(Value::ClassMethod {
        function: Box::new(function.clone()),
        identity: Rc::new(()),
    })
}

pub(crate) fn call_ord(args: Vec<Value>, keywords: Vec<(String, Value)>) -> Result<Value, String> {
    reject_stdlib_keywords("ord", &keywords)
        .map_err(|_| "TypeError: ord() takes no keyword arguments".to_string())?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: ord() expected 1 argument, got {}",
            args.len()
        ));
    };

    let codepoint = match value {
        Value::String(value) | Value::IdentityString { value, .. } => {
            let mut chars = value.chars();
            let Some(ch) = chars.next() else {
                return Err(
                    "TypeError: ord() expected a character, but string of length 0 found"
                        .to_string(),
                );
            };
            if chars.next().is_some() {
                return Err(format!(
                    "TypeError: ord() expected a character, but string of length {} found",
                    value.chars().count()
                ));
            }
            ch as u32
        }
        Value::Bytes(value) => {
            let [byte] = value.as_slice() else {
                return Err(format!(
                    "TypeError: ord() expected a character, but string of length {} found",
                    value.len()
                ));
            };
            u32::from(*byte)
        }
        Value::ByteArray(value) => {
            let value = value.borrow();
            let [byte] = value.as_slice() else {
                return Err(format!(
                    "TypeError: ord() expected a character, but string of length {} found",
                    value.len()
                ));
            };
            u32::from(*byte)
        }
        value => {
            return Err(format!(
                "TypeError: ord() expected string of length 1, but {} found",
                stdlib_type_name(value)
            ));
        }
    };

    Ok(Value::Number(i64::from(codepoint)))
}

pub(crate) fn call_callable<C: StdlibContext + ?Sized>(
    context: &C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    reject_stdlib_keywords("callable", &keywords)?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: callable() takes exactly one argument ({} given)",
            args.len()
        ));
    };

    Ok(Value::Bool(context.stdlib_is_callable(value)))
}

pub(crate) fn call_types_coroutine<C: StdlibContext + ?Sized>(
    context: &mut C,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if args.len() > 1 {
        return Err(format!(
            "TypeError: coroutine() expected 1 argument, got {}",
            args.len()
        ));
    }
    let mut function = args.first().cloned();
    for (keyword, value) in keywords {
        if keyword != "func" {
            return Err(format!(
                "TypeError: coroutine() got an unexpected keyword argument '{keyword}'"
            ));
        }
        if function.is_some() {
            return Err(
                "TypeError: coroutine() got multiple values for argument 'func'".to_string(),
            );
        }
        function = Some(value);
    }
    let function = function.ok_or_else(|| {
        "TypeError: coroutine() missing 1 required positional argument: 'func'".to_string()
    })?;
    if !context.stdlib_is_callable(&function) {
        return Err("TypeError: types.coroutine() expects a callable".to_string());
    }

    if let Value::Function {
        is_generator,
        is_async,
        identity,
        ..
    } = &function
    {
        if *is_generator && !*is_async {
            context.stdlib_mark_iterable_coroutine_function(identity);
            return Ok(function);
        }
        if *is_async {
            return Ok(function);
        }
    }

    Ok(Value::TypesCoroutineFunction {
        function: Box::new(function),
        identity: Rc::new(()),
    })
}

fn dis_instruction_value(opcode: i64, argval: Value) -> Result<Value, String> {
    Ok(Value::SimpleNamespace {
        fields: stdlib_dict_ref_from_entries(vec![
            (Value::String("opcode".to_string()), Value::Number(opcode)),
            (Value::String("argval".to_string()), argval),
        ]),
    })
}

fn stdio_stream_value(name: &str) -> Value {
    Value::SimpleNamespace {
        fields: stdlib_dict_ref_from_entries(vec![(
            Value::String("name".to_string()),
            Value::String(format!("<{name}>")),
        )]),
    }
}

fn stdlib_shared_iterator(iterator: Value) -> Value {
    Value::Iterator(Rc::new(RefCell::new(iterator)))
}

fn stdlib_list_iterator_from_values(items: Vec<Value>) -> Value {
    let Value::List(items) = list_value(items) else {
        unreachable!("list_value returns a list")
    };
    stdlib_shared_iterator(Value::ListIterator {
        items,
        index: 0,
        exhausted: false,
    })
}

fn format_prefixed_integer(value: &BigInt, prefix: &str, radix: u32) -> String {
    let digits = value.abs().to_str_radix(radix);
    if value.is_negative() {
        format!("-{prefix}{digits}")
    } else {
        format!("{prefix}{digits}")
    }
}

fn bind_import_args(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Vec<Option<Value>>, String> {
    let mut values = vec![None, None, None, None, None];
    let names = ["name", "globals", "locals", "fromlist", "level"];
    if args.len() > values.len() {
        return Err(format!(
            "TypeError: __import__() expected at most 5 arguments, got {}",
            args.len()
        ));
    }
    for (index, value) in args.into_iter().enumerate() {
        values[index] = Some(value);
    }
    for (name, value) in keywords {
        let Some(index) = names.iter().position(|candidate| candidate == &name) else {
            return Err(format!(
                "TypeError: __import__() got an unexpected keyword argument '{name}'"
            ));
        };
        if values[index].is_some() {
            return Err(format!(
                "TypeError: __import__() got multiple values for argument '{name}'"
            ));
        }
        values[index] = Some(value);
    }
    if values[0].is_none() {
        return Err("TypeError: __import__() missing required argument 'name'".to_string());
    }
    Ok(values)
}

fn import_level_argument(value: Value) -> Result<i64, String> {
    let level = match value {
        Value::Bool(value) => bool_as_i64(value),
        Value::Number(value) => value,
        Value::BigInt(value) => value.to_i64().ok_or_else(|| {
            "OverflowError: Python int too large to convert to C long".to_string()
        })?,
        Value::Float(_) => {
            return Err("TypeError: integer argument expected, got float".to_string());
        }
        value => {
            return Err(format!(
                "TypeError: an integer is required (got type {})",
                stdlib_type_name(&value)
            ));
        }
    };
    if level < 0 {
        Err("ValueError: level must be >= 0".to_string())
    } else {
        Ok(level)
    }
}

fn bool_as_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn stdlib_sys_type_name(value: &Value) -> &str {
    stdlib_type_name(value)
}

fn stdlib_type_name(value: &Value) -> &str {
    match value {
        Value::Number(_) | Value::BigInt(_) => "int",
        Value::Float(_) => "float",
        Value::Complex { .. } => "complex",
        Value::String(_) | Value::IdentityString { .. } => "str",
        Value::Bytes(_) => "bytes",
        Value::ByteArray(_) => "bytearray",
        Value::MemoryView(_) => "memoryview",
        Value::BytesIO(_) => "_io.BytesIO",
        Value::Bool(_) => "bool",
        Value::List(_) => "list",
        Value::Tuple(_) => "tuple",
        Value::Set(_) => "set",
        Value::FrozenSet(_) => "frozenset",
        Value::Dict(_) | Value::ScopeDict(_) => "dict",
        Value::OrderedDict(_) => "OrderedDict",
        Value::DefaultDict { .. } => "defaultdict",
        Value::Counter { .. } => "Counter",
        Value::DictView { kind, ordered, .. } => stdlib_dict_view_type_name(*kind, *ordered),
        Value::MappingView { kind, .. } => stdlib_dict_view_type_name(*kind, false),
        Value::MappingProxy { .. } | Value::MappingProxyObject { .. } => "mappingproxy",
        Value::ChainMap { .. } => "ChainMap",
        Value::UserList { .. } => "UserList",
        Value::Deque { .. } => "deque",
        Value::UserDict { .. } => "UserDict",
        Value::NamedTupleType(_) => "type",
        Value::NamedTuple { typ, .. } => typ.name.as_str(),
        Value::NamedTupleFieldDescriptor { .. } => "namedtuple_field_descriptor",
        Value::NamedTupleTypeMethod { .. } | Value::NamedTupleInstanceMethod { .. } => "method",
        Value::SimpleNamespace { .. } => "types.SimpleNamespace",
        Value::PicklePayload(_) => "pickle payload",
        Value::AstNode { kind, .. } => kind.as_str(),
        Value::CodeObject { .. } => "code",
        Value::Cell { .. } => "cell",
        Value::Frame { .. } => "frame",
        Value::FrameLocalsProxy { .. } => "FrameLocalsProxy",
        Value::Range { .. } => "range",
        Value::Slice { .. } => "slice",
        Value::RangeIterator { .. } => "range_iterator",
        Value::ListIterator { .. } => "list_iterator",
        Value::TupleIterator { .. } => "tuple_iterator",
        Value::TemplateIterator { .. } => "TemplateIter",
        Value::StringIterator { .. } => "str_iterator",
        Value::BytesIterator { .. } => "bytes_iterator",
        Value::ByteArrayIterator { .. } => "bytearray_iterator",
        Value::SetIterator { .. } => "set_iterator",
        Value::DictIterator { .. } => "dict_keyiterator",
        Value::ReverseIterator { .. } => "list_reverseiterator",
        Value::DictReverseIterator { .. } => "dict_reversekeyiterator",
        Value::SequenceReverseIterator { .. } => "reversed",
        Value::EnumerateIterator { .. } => "enumerate",
        Value::ZipIterator { .. } => "zip",
        Value::MapIterator { .. } => "map",
        Value::FilterIterator { .. } => "filter",
        Value::ItertoolsCount { .. } => "count",
        Value::ItertoolsRepeat { .. } => "repeat",
        Value::ItertoolsCycle { .. } => "cycle",
        Value::ItertoolsAccumulate { .. } => "accumulate",
        Value::ItertoolsChain { .. } | Value::ItertoolsChainFromIterable { .. } => "chain",
        Value::ItertoolsCompress { .. } => "compress",
        Value::ItertoolsDropwhile { .. } => "dropwhile",
        Value::ItertoolsFilterFalse { .. } => "filterfalse",
        Value::ItertoolsTakewhile { .. } => "takewhile",
        Value::ItertoolsStarmap { .. } => "starmap",
        Value::ItertoolsZipLongest { .. } => "zip_longest",
        Value::ItertoolsIslice { .. } => "islice",
        Value::ItertoolsPairwise { .. } => "pairwise",
        Value::ItertoolsProduct { .. } => "product",
        Value::ItertoolsCombinations { .. } => "combinations",
        Value::ItertoolsCombinationsWithReplacement { .. } => "combinations_with_replacement",
        Value::ItertoolsPermutations { .. } => "permutations",
        Value::ItertoolsTee { .. } => "_tee",
        Value::ItertoolsBatched { .. } => "batched",
        Value::ItertoolsGroupBy { .. } => "groupby",
        Value::ItertoolsGroup { .. } => "_grouper",
        Value::CallIterator { .. } => "callable_iterator",
        Value::SequenceIterator { .. } => "iterator",
        Value::Iterator(state) => stdlib_iterator_type_name(&state.borrow()),
        Value::Function { .. } => "function",
        Value::TypesCoroutineFunction { function, .. } => stdlib_type_name(function),
        Value::MagicMock { .. } => "MagicMock",
        Value::MockMethod { .. } => "MagicMock",
        Value::WeakRef { .. } => "ReferenceType",
        Value::WeakProxy { callable, .. } => {
            if *callable {
                "CallableProxyType"
            } else {
                "ProxyType"
            }
        }
        Value::Generator(_) => "generator",
        Value::GeneratorWrapper { .. } => "_GeneratorWrapper",
        Value::Coroutine(_) => "coroutine",
        Value::CoroutineAwait(_) => "coroutine_wrapper",
        Value::AwaitIterator(_) => "await_iterator",
        Value::AsyncGenerator(_) => "async_generator",
        Value::AsyncGeneratorNext { .. } | Value::AsyncGeneratorThrow { .. } => {
            "async_generator_asend"
        }
        Value::AsyncGeneratorClose(_) => "async_generator_athrow",
        Value::AsyncGeneratorAthrowMixin { .. } | Value::AsyncGeneratorAcloseMixin { .. } => {
            "coroutine"
        }
        Value::AnextDefault { .. } => "anext_awaitable",
        Value::Class { .. } => "type",
        Value::Builtin(_) => "builtin_function_or_method",
        Value::TypeParam { kind, .. } => kind.as_str(),
        Value::ParamSpecAccess { is_kwargs, .. } => {
            if *is_kwargs {
                "ParamSpecKwargs"
            } else {
                "ParamSpecArgs"
            }
        }
        Value::DeferredTypeParamExpr(_) => "DeferredTypeParamExpr",
        Value::TypeAlias { .. } => "TypeAliasType",
        Value::ForwardRef { .. } => "ForwardRef",
        Value::NewType { .. } => "NewType",
        Value::ConstEvaluator { .. } => "_typing._ConstEvaluator",
        Value::GenericAlias { .. } => "GenericAlias",
        Value::Unpack(_) => "Unpack",
        Value::Template { .. } => "Template",
        Value::TemplateInterpolation(_) => "Interpolation",
        Value::Instance { class_name, .. } => class_name.as_str(),
        Value::Property { .. } => "property",
        Value::MemberDescriptor { .. } => "member_descriptor",
        Value::StaticMethod { .. } => "staticmethod",
        Value::ClassMethod { .. } => "classmethod",
        Value::Super { .. } => "super",
        Value::BoundMethod { function, .. } if matches!(function.as_ref(), Value::Builtin(_)) => {
            "builtin_function_or_method"
        }
        Value::BoundMethod { .. } => "method",
        Value::Partial { .. } => "partial",
        Value::PartialMethod { .. } => "partialmethod",
        Value::PartialMethodCall {
            expects_self_arg: true,
            ..
        } => "function",
        Value::PartialMethodCall { .. } => "partial",
        Value::LruCacheWrapper { .. } => "_lru_cache_wrapper",
        Value::SingleDispatch { .. }
        | Value::SingleDispatchRegister { .. }
        | Value::SingleDispatchMethodCallable { .. } => "function",
        Value::SingleDispatchMethod { .. } => "singledispatchmethod",
        Value::CachedProperty { .. } => "cached_property",
        Value::CmpToKey { .. } | Value::CmpToKeyObject { .. } => "functools.KeyWrapper",
        Value::OperatorAttrGetter { .. } => "attrgetter",
        Value::OperatorItemGetter { .. } => "itemgetter",
        Value::OperatorMethodCaller { .. } => "methodcaller",
        Value::InspectSignature { .. } => "Signature",
        Value::Module { .. } => "module",
        Value::Traceback { .. } => "traceback",
        Value::Exception { type_name, .. } => type_name.as_str(),
        Value::None => "NoneType",
        Value::NotImplemented => "NotImplementedType",
        Value::Ellipsis => "ellipsis",
    }
}

fn stdlib_iterator_type_name(iterator: &Value) -> &'static str {
    match iterator {
        Value::RangeIterator { .. } => "range_iterator",
        Value::ListIterator { .. } => "list_iterator",
        Value::TupleIterator { .. } => "tuple_iterator",
        Value::TemplateIterator { .. } => "TemplateIter",
        Value::StringIterator { .. } => "str_iterator",
        Value::BytesIterator { .. } => "bytes_iterator",
        Value::ByteArrayIterator { .. } => "bytearray_iterator",
        Value::SetIterator { .. } => "set_iterator",
        Value::DictIterator { .. } => "dict_keyiterator",
        Value::ReverseIterator { .. } => "list_reverseiterator",
        Value::DictReverseIterator { .. } => "dict_reversekeyiterator",
        Value::SequenceReverseIterator { .. } => "reversed",
        Value::EnumerateIterator { .. } => "enumerate",
        Value::ZipIterator { .. } => "zip",
        Value::MapIterator { .. } => "map",
        Value::FilterIterator { .. } => "filter",
        Value::ItertoolsAccumulate { .. } => "accumulate",
        Value::ItertoolsCycle { .. } => "cycle",
        Value::ItertoolsDropwhile { .. } => "dropwhile",
        Value::ItertoolsFilterFalse { .. } => "filterfalse",
        Value::ItertoolsTakewhile { .. } => "takewhile",
        Value::ItertoolsStarmap { .. } => "starmap",
        Value::ItertoolsZipLongest { .. } => "zip_longest",
        Value::ItertoolsProduct { .. } => "product",
        Value::ItertoolsCombinations { .. } => "combinations",
        Value::ItertoolsCombinationsWithReplacement { .. } => "combinations_with_replacement",
        Value::ItertoolsPermutations { .. } => "permutations",
        Value::ItertoolsTee { .. } => "_tee",
        Value::ItertoolsBatched { .. } => "batched",
        Value::ItertoolsGroupBy { .. } => "groupby",
        Value::ItertoolsGroup { .. } => "_grouper",
        Value::CallIterator { .. } => "callable_iterator",
        Value::SequenceIterator { .. } => "iterator",
        Value::Iterator(state) => stdlib_iterator_type_name(&state.borrow()),
        _ => "iterator",
    }
}

fn stdlib_dict_view_type_name(kind: DictViewKind, ordered: bool) -> &'static str {
    match (kind, ordered) {
        (DictViewKind::Keys, true) => "odict_keys",
        (DictViewKind::Values, true) => "odict_values",
        (DictViewKind::Items, true) => "odict_items",
        (DictViewKind::Keys, false) => "dict_keys",
        (DictViewKind::Values, false) => "dict_values",
        (DictViewKind::Items, false) => "dict_items",
    }
}

fn stdlib_dict_ref_from_entries(entries: Vec<(Value, Value)>) -> DictRef {
    Rc::new(RefCell::new(DictStorage::new(entries)))
}

fn reject_stdlib_keywords(name: &str, keywords: &[(String, Value)]) -> Result<(), String> {
    if keywords.is_empty() {
        Ok(())
    } else {
        Err(format!("TypeError: {name}() takes no keyword arguments"))
    }
}

fn new_scope() -> Scope {
    Rc::new(RefCell::new(HashMap::new()))
}

fn module_value(name: &str, attrs: Vec<(&str, Value)>) -> Value {
    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        values.insert("__name__".to_string(), Value::String(name.to_string()));
        for (name, value) in attrs {
            values.insert(name.to_string(), value);
        }
    }

    Value::Module {
        name: name.to_string(),
        attrs: scope,
    }
}

fn copy_module_value() -> Value {
    let error = stdlib_exception_class("Error", "copy");
    module_value(
        "copy",
        vec![
            ("Error", error.clone()),
            ("error", error),
            ("dispatch_table", dict_value(Vec::new())),
            ("copy", Value::Builtin("copy.copy".to_string())),
            ("deepcopy", Value::Builtin("copy.deepcopy".to_string())),
            ("replace", Value::Builtin("copy.replace".to_string())),
        ],
    )
}

fn stdlib_exception_class(name: &str, module: &str) -> Value {
    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        values.insert("__module__".to_string(), Value::String(module.to_string()));
    }

    Value::Class {
        name: name.to_string(),
        type_params: Vec::new(),
        metaclass: None,
        bases: vec![builtin_type_value("Exception")],
        attrs: scope,
    }
}

fn types_module() -> Value {
    let mut attrs = Vec::from([
        ("__all__", string_list_value(TYPES_ALL)),
        ("_GeneratorWrapper", builtin_type_value("_GeneratorWrapper")),
        ("coroutine", Value::Builtin("types.coroutine".to_string())),
        (
            "get_original_bases",
            Value::Builtin("types.get_original_bases".to_string()),
        ),
        ("new_class", Value::Builtin("types.new_class".to_string())),
        (
            "prepare_class",
            Value::Builtin("types.prepare_class".to_string()),
        ),
        (
            "resolve_bases",
            Value::Builtin("types.resolve_bases".to_string()),
        ),
    ]);
    attrs.extend(types_accelerator_attrs());
    module_value("types", attrs)
}

fn types_accelerator_module() -> Value {
    module_value("_types", types_accelerator_attrs())
}

fn types_accelerator_attrs() -> Vec<(&'static str, Value)> {
    vec![
        ("AsyncGeneratorType", builtin_type_value("async_generator")),
        (
            "BuiltinFunctionType",
            builtin_type_value("builtin_function_or_method"),
        ),
        (
            "BuiltinMethodType",
            builtin_type_value("builtin_function_or_method"),
        ),
        ("CapsuleType", builtin_type_value("PyCapsule")),
        ("CellType", Value::Builtin("CellType".to_string())),
        (
            "ClassMethodDescriptorType",
            builtin_type_value("classmethod_descriptor"),
        ),
        ("CodeType", builtin_type_value("code")),
        ("CoroutineType", builtin_type_value("coroutine")),
        (
            "DynamicClassAttribute",
            builtin_type_value("DynamicClassAttribute"),
        ),
        ("EllipsisType", builtin_type_value("ellipsis")),
        ("FrameType", builtin_type_value("frame")),
        ("FunctionType", builtin_type_value("function")),
        ("GeneratorType", builtin_type_value("generator")),
        ("GenericAlias", builtin_type_value("GenericAlias")),
        (
            "GetSetDescriptorType",
            builtin_type_value("getset_descriptor"),
        ),
        ("LambdaType", builtin_type_value("function")),
        (
            "MappingProxyType",
            Value::Builtin("mappingproxy".to_string()),
        ),
        (
            "MemberDescriptorType",
            builtin_type_value("member_descriptor"),
        ),
        (
            "MethodDescriptorType",
            builtin_type_value("method_descriptor"),
        ),
        ("MethodType", builtin_type_value("method")),
        ("MethodWrapperType", builtin_type_value("method-wrapper")),
        ("ModuleType", builtin_type_value("module")),
        ("NoneType", builtin_type_value("NoneType")),
        (
            "NotImplementedType",
            builtin_type_value("NotImplementedType"),
        ),
        (
            "SimpleNamespace",
            Value::Builtin("SimpleNamespace".to_string()),
        ),
        ("TracebackType", builtin_type_value("traceback")),
        ("UnionType", builtin_type_value("UnionType")),
        (
            "WrapperDescriptorType",
            builtin_type_value("wrapper_descriptor"),
        ),
    ]
}

fn string_tuple_value(values: &[&str]) -> Value {
    tuple_value(
        values
            .iter()
            .map(|value| Value::String((*value).to_string()))
            .collect(),
    )
}

fn string_list_value(values: &[&str]) -> Value {
    list_value(
        values
            .iter()
            .map(|value| Value::String((*value).to_string()))
            .collect(),
    )
}

const OPERATOR_ALL: &[&str] = &[
    "abs",
    "add",
    "and_",
    "attrgetter",
    "call",
    "concat",
    "contains",
    "countOf",
    "delitem",
    "eq",
    "floordiv",
    "ge",
    "getitem",
    "gt",
    "iadd",
    "iand",
    "iconcat",
    "ifloordiv",
    "ilshift",
    "imatmul",
    "imod",
    "imul",
    "index",
    "indexOf",
    "inv",
    "invert",
    "ior",
    "ipow",
    "irshift",
    "is_",
    "is_none",
    "is_not",
    "is_not_none",
    "isub",
    "itemgetter",
    "itruediv",
    "ixor",
    "le",
    "length_hint",
    "lshift",
    "lt",
    "matmul",
    "methodcaller",
    "mod",
    "mul",
    "ne",
    "neg",
    "not_",
    "or_",
    "pos",
    "pow",
    "rshift",
    "setitem",
    "sub",
    "truediv",
    "truth",
    "xor",
];

const OPERATOR_DUNDER_ALIASES: &[(&str, &str)] = &[
    ("__lt__", "lt"),
    ("__le__", "le"),
    ("__eq__", "eq"),
    ("__ne__", "ne"),
    ("__ge__", "ge"),
    ("__gt__", "gt"),
    ("__not__", "not_"),
    ("__abs__", "abs"),
    ("__add__", "add"),
    ("__and__", "and_"),
    ("__call__", "call"),
    ("__floordiv__", "floordiv"),
    ("__index__", "index"),
    ("__inv__", "inv"),
    ("__invert__", "invert"),
    ("__lshift__", "lshift"),
    ("__mod__", "mod"),
    ("__mul__", "mul"),
    ("__matmul__", "matmul"),
    ("__neg__", "neg"),
    ("__or__", "or_"),
    ("__pos__", "pos"),
    ("__pow__", "pow"),
    ("__rshift__", "rshift"),
    ("__sub__", "sub"),
    ("__truediv__", "truediv"),
    ("__xor__", "xor"),
    ("__concat__", "concat"),
    ("__contains__", "contains"),
    ("__delitem__", "delitem"),
    ("__getitem__", "getitem"),
    ("__setitem__", "setitem"),
    ("__iadd__", "iadd"),
    ("__iand__", "iand"),
    ("__iconcat__", "iconcat"),
    ("__ifloordiv__", "ifloordiv"),
    ("__ilshift__", "ilshift"),
    ("__imod__", "imod"),
    ("__imul__", "imul"),
    ("__imatmul__", "imatmul"),
    ("__ior__", "ior"),
    ("__ipow__", "ipow"),
    ("__irshift__", "irshift"),
    ("__isub__", "isub"),
    ("__itruediv__", "itruediv"),
    ("__ixor__", "ixor"),
];

fn operator_builtin(name: &str) -> Value {
    Value::Builtin(format!("operator.{name}"))
}

fn operator_module_value() -> Value {
    let mut attrs = Vec::with_capacity(OPERATOR_ALL.len() + OPERATOR_DUNDER_ALIASES.len() + 2);
    attrs.push(("__package__", Value::String(String::new())));
    for name in OPERATOR_ALL {
        attrs.push((*name, operator_builtin(name)));
    }
    attrs.push((
        "__all__",
        list_value(
            OPERATOR_ALL
                .iter()
                .map(|name| Value::String((*name).to_string()))
                .collect(),
        ),
    ));
    for (alias, target) in OPERATOR_DUNDER_ALIASES {
        attrs.push((*alias, operator_builtin(target)));
    }
    module_value("operator", attrs)
}

fn builtin_type_value(name: &str) -> Value {
    Value::Builtin(name.to_string())
}

fn string_key_dict(entries: Vec<(&str, Value)>) -> Value {
    dict_value(
        entries
            .into_iter()
            .map(|(key, value)| (Value::String(key.to_string()), value))
            .collect(),
    )
}

fn generic_alias_value(origin: Value, args: Vec<Value>) -> Value {
    Value::GenericAlias {
        origin: Box::new(origin),
        args,
        union_unhashable_count: 0,
    }
}

fn synthetic_class_value(name: &str, bases: Vec<Value>, attrs: Vec<(&str, Value)>) -> Value {
    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        values.insert(
            "__module__".to_string(),
            Value::String("__main__".to_string()),
        );
        for (name, value) in attrs {
            values.insert(name.to_string(), value);
        }
    }

    Value::Class {
        name: name.to_string(),
        type_params: Vec::new(),
        metaclass: None,
        bases,
        attrs: scope,
    }
}

fn union_type_value(left: Value, right: Value) -> Value {
    let mut args = Vec::new();
    extend_union_args(&mut args, left);
    extend_union_args(&mut args, right);
    generic_alias_value(builtin_type_value("Union"), args)
}

fn extend_union_args(args: &mut Vec<Value>, value: Value) {
    match value {
        Value::GenericAlias {
            origin,
            args: nested,
            ..
        } if matches!(origin.as_ref(), Value::Builtin(name) if name == "Union") => {
            for value in nested {
                push_unique_union_arg(args, value);
            }
        }
        value => push_unique_union_arg(args, value),
    }
}

fn push_unique_union_arg(args: &mut Vec<Value>, value: Value) {
    let value = match value {
        Value::None => builtin_type_value("NoneType"),
        value => value,
    };
    if !args.iter().any(|existing| existing == &value) {
        args.push(value);
    }
}

fn builtins_module() -> Value {
    let entries = DEFAULT_BUILTIN_ENTRY_NAMES
        .iter()
        .map(|name| (*name, builtin_module_entry_value(name)))
        .collect::<Vec<_>>();
    module_value("builtins", entries)
}

fn builtin_module_entry_value(name: &str) -> Value {
    match name {
        "Ellipsis" => Value::Ellipsis,
        "NotImplemented" => Value::NotImplemented,
        "__debug__" => Value::Bool(true),
        name => Value::Builtin(name.to_string()),
    }
}

fn annotationlib_module() -> Value {
    module_value(
        "annotationlib",
        vec![
            (
                "Format",
                synthetic_class_value(
                    "Format",
                    vec![builtin_type_value("object")],
                    vec![
                        ("VALUE", Value::Number(1)),
                        ("FORWARDREF", Value::Number(2)),
                        ("STRING", Value::Number(3)),
                    ],
                ),
            ),
            (
                "call_evaluate_function",
                Value::Builtin("annotationlib.call_evaluate_function".to_string()),
            ),
        ],
    )
}

fn test_package_module(
    import_dependency: &mut dyn FnMut(&str) -> Result<Value, String>,
) -> Result<Value, String> {
    Ok(module_value(
        "test",
        vec![
            ("__annotations__", string_key_dict(Vec::new())),
            ("typinganndata", import_dependency("test.typinganndata")?),
        ],
    ))
}

fn test_typinganndata_package_module(
    import_dependency: &mut dyn FnMut(&str) -> Result<Value, String>,
) -> Result<Value, String> {
    Ok(module_value(
        "test.typinganndata",
        vec![
            (
                "ann_module",
                import_dependency("test.typinganndata.ann_module")?,
            ),
            (
                "ann_module2",
                import_dependency("test.typinganndata.ann_module2")?,
            ),
            (
                "ann_module3",
                import_dependency("test.typinganndata.ann_module3")?,
            ),
        ],
    ))
}

fn test_typinganndata_ann_module() -> Value {
    let tuple_int_int = generic_alias_value(
        Value::Builtin("typing.Tuple".to_string()),
        vec![builtin_type_value("int"), builtin_type_value("int")],
    );
    let int_or_float = union_type_value(builtin_type_value("int"), builtin_type_value("float"));
    let metaclass = synthetic_class_value(
        "M",
        vec![builtin_type_value("type")],
        vec![(
            "__annotations__",
            string_key_dict(vec![("o", builtin_type_value("type"))]),
        )],
    );

    module_value(
        "test.typinganndata.ann_module",
        vec![
            (
                "__annotations__",
                string_key_dict(vec![
                    ("x", builtin_type_value("int")),
                    ("y", builtin_type_value("str")),
                    ("f", tuple_int_int),
                    ("u", int_or_float),
                ]),
            ),
            ("M", metaclass),
        ],
    )
}

fn test_typinganndata_ann_module2() -> Value {
    module_value(
        "test.typinganndata.ann_module2",
        vec![("__annotations__", string_key_dict(Vec::new()))],
    )
}

fn test_typinganndata_ann_module3() -> Value {
    let d_bad_ann = synthetic_class_value(
        "D_bad_ann",
        vec![builtin_type_value("object")],
        vec![(
            "__init__",
            Value::Builtin("test.typinganndata.ann_module3.D_bad_ann.__init__".to_string()),
        )],
    );

    module_value(
        "test.typinganndata.ann_module3",
        vec![
            (
                "f_bad_ann",
                Value::Builtin("test.typinganndata.ann_module3.f_bad_ann".to_string()),
            ),
            (
                "g_bad_ann",
                Value::Builtin("test.typinganndata.ann_module3.g_bad_ann".to_string()),
            ),
            ("D_bad_ann", d_bad_ann),
        ],
    )
}

fn ast_module_entries() -> Vec<(&'static str, Value)> {
    let mut entries = AST_MODULE_TYPE_NAMES
        .iter()
        .map(|name| {
            let builtin_name = if *name == "Interpolation" {
                "ast.Interpolation"
            } else {
                *name
            };
            (*name, Value::Builtin(builtin_name.to_string()))
        })
        .collect::<Vec<_>>();
    entries.extend([
        ("Set", Value::Builtin("ast.Set".to_string())),
        (
            "PyCF_ALLOW_TOP_LEVEL_AWAIT",
            Value::Number(PYCF_ALLOW_TOP_LEVEL_AWAIT),
        ),
        ("PyCF_ONLY_AST", Value::Number(PYCF_ONLY_AST)),
        ("PyCF_OPTIMIZED_AST", Value::Number(PYCF_OPTIMIZED_AST)),
        ("parse", Value::Builtin("ast.parse".to_string())),
        ("dump", Value::Builtin("ast.dump".to_string())),
        ("compare", Value::Builtin("ast.compare".to_string())),
        (
            "literal_eval",
            Value::Builtin("ast.literal_eval".to_string()),
        ),
        (
            "copy_location",
            Value::Builtin("ast.copy_location".to_string()),
        ),
        (
            "fix_missing_locations",
            Value::Builtin("ast.fix_missing_locations".to_string()),
        ),
        (
            "increment_lineno",
            Value::Builtin("ast.increment_lineno".to_string()),
        ),
        (
            "get_docstring",
            Value::Builtin("ast.get_docstring".to_string()),
        ),
        (
            "get_source_segment",
            Value::Builtin("ast.get_source_segment".to_string()),
        ),
        ("iter_fields", Value::Builtin("ast.iter_fields".to_string())),
        (
            "iter_child_nodes",
            Value::Builtin("ast.iter_child_nodes".to_string()),
        ),
        ("walk", Value::Builtin("ast.walk".to_string())),
        ("NodeVisitor", Value::Builtin("NodeVisitor".to_string())),
        (
            "NodeTransformer",
            Value::Builtin("NodeTransformer".to_string()),
        ),
    ]);
    entries
}
