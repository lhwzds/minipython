use crate::bytecode::Instruction;
use crate::lexer::{get_int_max_str_digits, set_int_max_str_digits};
use crate::value::{
    CodeMode, DictRef, DictStorage, DictViewKind, Scope, Value, dict_value, list_value, tuple_value,
};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) const PYCF_ONLY_AST: i64 = 1024;
pub(crate) const PICKLE_HIGHEST_PROTOCOL: i64 = 5;
pub(crate) const DIS_LOAD_CONST_OPCODE: i64 = 100;

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

    fn stdlib_hash_value(&self, value: &Value) -> Result<Value, String>;

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
    "print",
    "format",
    "eval",
    "exec",
    "compile",
    "range",
    "next",
    "iter",
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
                ("maxsize", Value::Number(i64::MAX)),
                ("version", Value::String("minipython".to_string())),
                (
                    "get_int_max_str_digits",
                    Value::Builtin("sys.get_int_max_str_digits".to_string()),
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
        "math" => Ok(module_value(
            "math",
            vec![
                ("pi", Value::Float(std::f64::consts::PI)),
                ("tau", Value::Float(std::f64::consts::TAU)),
                ("sqrt", Value::Builtin("sqrt".to_string())),
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
        "copy" => Ok(module_value(
            "copy",
            vec![
                ("copy", Value::Builtin("copy.copy".to_string())),
                ("deepcopy", Value::Builtin("copy.deepcopy".to_string())),
                ("replace", Value::Builtin("copy.replace".to_string())),
            ],
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
                ("CO_GENERATOR", Value::Number(0x0020)),
                ("CO_COROUTINE", Value::Number(0x0080)),
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
                ("Generic", builtin_type_value("Generic")),
                ("NoDefault", Value::Builtin("typing.NoDefault".to_string())),
                ("Tuple", builtin_type_value("tuple")),
                ("TypeVar", builtin_type_value("typing.TypeVar")),
                ("TypeVarTuple", builtin_type_value("typing.TypeVarTuple")),
                ("ParamSpec", builtin_type_value("typing.ParamSpec")),
                ("TypeAliasType", builtin_type_value("typing.TypeAliasType")),
                ("get_args", Value::Builtin("typing.get_args".to_string())),
            ],
        )),
        "types" => Ok(module_value(
            "types",
            vec![
                (
                    "MappingProxyType",
                    Value::Builtin("mappingproxy".to_string()),
                ),
                (
                    "SimpleNamespace",
                    Value::Builtin("SimpleNamespace".to_string()),
                ),
                ("coroutine", Value::Builtin("types.coroutine".to_string())),
                (
                    "get_original_bases",
                    Value::Builtin("types.get_original_bases".to_string()),
                ),
            ],
        )),
        "collections" => Ok(module_value(
            "collections",
            vec![
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
                ("OrderedDict", Value::Builtin("OrderedDict".to_string())),
                ("UserList", Value::Builtin("UserList".to_string())),
                ("UserDict", Value::Builtin("UserDict".to_string())),
                ("UserString", Value::Builtin("UserString".to_string())),
                ("abc", import_dependency("collections.abc")?),
            ],
        )),
        "collections.abc" => Ok(module_value(
            "collections.abc",
            vec![
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
        return Err("get_int_max_str_digits() does not accept keyword arguments".to_string());
    }
    if !args.is_empty() {
        return Err(format!(
            "get_int_max_str_digits() takes no arguments ({} given)",
            args.len()
        ));
    }

    Ok(Value::Number(get_int_max_str_digits() as i64))
}

pub(crate) fn call_sys_set_int_max_str_digits(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("set_int_max_str_digits() does not accept keyword arguments".to_string());
    }
    if args.len() != 1 {
        return Err(format!(
            "set_int_max_str_digits() takes exactly one argument ({} given)",
            args.len()
        ));
    }

    let maxdigits = match args.into_iter().next().expect("length checked") {
        Value::Number(value) if value >= 0 => value as usize,
        Value::Bool(value) => bool_as_i64(value) as usize,
        Value::BigInt(value) if !value.is_negative() => value
            .to_usize()
            .ok_or_else(|| "ValueError: maxdigits is too large".to_string())?,
        Value::Number(_) | Value::BigInt(_) => {
            return Err("ValueError: maxdigits must be non-negative".to_string());
        }
        value => {
            return Err(format!(
                "TypeError: 'maxdigits' must be an integer, not {}",
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
    Ok(stdlib_shared_iterator(Value::ListIterator {
        items,
        index: 0,
    }))
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
                Value::Number(span.line),
                Value::None,
                Value::None,
            ])
        })
        .collect();
    Ok(stdlib_shared_iterator(Value::ListIterator {
        items,
        index: 0,
    }))
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
        return Err(format!("any() expected 1 argument, got {}", args.len()));
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
        return Err(format!("all() expected 1 argument, got {}", args.len()));
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
    let options = minmax_options(name, keywords)?;
    if args.is_empty() {
        return Err(format!("{name} expected at least 1 argument, got 0"));
    }

    if args.len() > 1 {
        if options.default.is_some() {
            return Err(format!(
                "Cannot specify a default for {name}() with multiple positional arguments"
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

fn minmax_options(name: &str, keywords: Vec<(String, Value)>) -> Result<MinMaxOptions, String> {
    let mut options = MinMaxOptions::default();
    for (keyword, value) in keywords {
        match keyword.as_str() {
            "key" => {
                if options.key.is_some() {
                    return Err(format!(
                        "{name}() got multiple values for keyword argument 'key'"
                    ));
                }
                options.key = Some(value);
            }
            "default" => {
                if options.default.is_some() {
                    return Err(format!(
                        "{name}() got multiple values for keyword argument 'default'"
                    ));
                }
                options.default = Some(value);
            }
            _ => {
                return Err(format!(
                    "'{keyword}' is an invalid keyword argument for {name}()"
                ));
            }
        }
    }
    Ok(options)
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
                    None => Err(format!("{name}() arg is an empty sequence")),
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
            None => Err(format!("{name}() arg is an empty sequence")),
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
    reject_stdlib_keywords("sum", &keywords)?;
    let (iterable, start) = match args.as_slice() {
        [] => return Err("sum() takes at least 1 positional argument (0 given)".to_string()),
        [iterable] => (iterable.clone(), Value::Number(0)),
        [iterable, start] => (iterable.clone(), start.clone()),
        values => {
            return Err(format!(
                "sum() takes at most 2 positional arguments ({} given)",
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
                if matches!(value, Value::String(_)) {
                    return Err("sum() can't sum strings".to_string());
                }
                total = context.stdlib_add_values(total, value)?;
            }
            StdlibIteratorAdvance::Complete | StdlibIteratorAdvance::Raised => return Ok(total),
        }
    }
}

fn reject_sum_start(value: &Value) -> Result<(), String> {
    match value {
        Value::String(_) => Err("sum() can't sum strings [use ''.join(seq) instead]".to_string()),
        Value::Bytes(_) => Err("sum() can't sum bytes [use b''.join(seq) instead]".to_string()),
        Value::ByteArray(_) => {
            Err("sum() can't sum bytearray [use b''.join(seq) instead]".to_string())
        }
        _ => Ok(()),
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
        Value::String(name) => name,
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
    let resolved_name = context.stdlib_resolve_import_name_from_globals_value(
        &name,
        level as usize,
        globals_arg.as_ref(),
    )?;
    let return_root = level == 0 && !context.stdlib_truth_value(fromlist)?;

    context.stdlib_load_imported_module_value(&resolved_name, return_root)
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

    if let Value::MappingProxyObject { mapping } = value {
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
            Value::String(value) => Ok(Value::String(value)),
            value => Err(format!(
                "TypeError: __repr__ returned non-string (type {})",
                stdlib_type_name(&value)
            )),
        };
    }

    Ok(Value::String(context.stdlib_repr_value(value)?))
}

fn hash_result_from_special_method(value: Value) -> Result<Value, String> {
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
        return Err(format!("{name}() does not accept keyword arguments"));
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
        .map_err(|_| "TypeError: chr() does not accept keyword arguments".to_string())?;
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
        return Err("staticmethod() does not accept keyword arguments".to_string());
    }
    let [function] = args.as_slice() else {
        return Err(format!(
            "staticmethod() expected 1 argument, got {}",
            args.len()
        ));
    };

    Ok(Value::StaticMethod {
        function: Box::new(function.clone()),
    })
}

pub(crate) fn call_classmethod_constructor(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err("classmethod() does not accept keyword arguments".to_string());
    }
    let [function] = args.as_slice() else {
        return Err(format!(
            "classmethod() expected 1 argument, got {}",
            args.len()
        ));
    };

    Ok(Value::ClassMethod {
        function: Box::new(function.clone()),
    })
}

pub(crate) fn call_property_constructor(
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Value, String> {
    if args.len() > 4 {
        return Err(format!(
            "property() expected at most 4 arguments, got {}",
            args.len()
        ));
    }

    let mut fget = None;
    let mut fset = None;
    let mut fdel = None;
    let mut doc = None;

    for (index, value) in args.into_iter().enumerate() {
        match index {
            0 => fget = Some(value),
            1 => fset = Some(value),
            2 => fdel = Some(value),
            3 => doc = Some(value),
            _ => unreachable!("property positional arity is checked above"),
        }
    }

    for (keyword, value) in keywords {
        match keyword.as_str() {
            "fget" => set_property_constructor_slot("property", "fget", &mut fget, value)?,
            "fset" => set_property_constructor_slot("property", "fset", &mut fset, value)?,
            "fdel" => set_property_constructor_slot("property", "fdel", &mut fdel, value)?,
            "doc" => set_property_constructor_slot("property", "doc", &mut doc, value)?,
            _ => {
                return Err(format!(
                    "property() got an unexpected keyword argument '{keyword}'"
                ));
            }
        }
    }

    Ok(Value::Property {
        fget: optional_property_part(fget),
        fset: optional_property_part(fset),
        fdel: optional_property_part(fdel),
        doc: optional_property_part(doc),
    })
}

fn set_property_constructor_slot(
    function_name: &str,
    slot_name: &str,
    slot: &mut Option<Value>,
    value: Value,
) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!(
            "{function_name}() got multiple values for argument '{slot_name}'"
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn optional_property_part(value: Option<Value>) -> Option<Box<Value>> {
    match value {
        Some(Value::None) | None => None,
        Some(value) => Some(Box::new(value)),
    }
}

pub(crate) fn call_ord(args: Vec<Value>, keywords: Vec<(String, Value)>) -> Result<Value, String> {
    reject_stdlib_keywords("ord", &keywords)
        .map_err(|_| "TypeError: ord() does not accept keyword arguments".to_string())?;
    let [value] = args.as_slice() else {
        return Err(format!(
            "TypeError: ord() expected 1 argument, got {}",
            args.len()
        ));
    };

    let codepoint = match value {
        Value::String(value) => {
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
    if !keywords.is_empty() {
        return Err("coroutine() does not accept keyword arguments".to_string());
    }
    let [function] = args.as_slice() else {
        return Err(format!(
            "TypeError: coroutine() expected 1 argument, got {}",
            args.len()
        ));
    };
    if !context.stdlib_is_callable(function) {
        return Err("TypeError: types.coroutine() expects a callable".to_string());
    }

    if let Value::Function {
        is_generator,
        is_async,
        identity,
        ..
    } = function
    {
        if *is_generator && !*is_async {
            context.stdlib_mark_iterable_coroutine_function(identity);
        }
    }

    Ok(function.clone())
}

fn dis_instruction_value(opcode: i64, argval: Value) -> Result<Value, String> {
    Ok(Value::SimpleNamespace {
        fields: stdlib_dict_ref_from_entries(vec![
            (Value::String("opcode".to_string()), Value::Number(opcode)),
            (Value::String("argval".to_string()), argval),
        ]),
    })
}

fn stdlib_shared_iterator(iterator: Value) -> Value {
    Value::Iterator(Rc::new(RefCell::new(iterator)))
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
        Value::String(_) => "str",
        Value::Bytes(_) => "bytes",
        Value::ByteArray(_) => "bytearray",
        Value::MemoryView(_) => "memoryview",
        Value::Bool(_) => "bool",
        Value::List(_) => "list",
        Value::Tuple(_) => "tuple",
        Value::Set(_) => "set",
        Value::FrozenSet(_) => "frozenset",
        Value::Dict(_) | Value::ScopeDict(_) => "dict",
        Value::OrderedDict(_) => "OrderedDict",
        Value::Counter { .. } => "Counter",
        Value::DictView { kind, .. } => stdlib_dict_view_type_name(*kind),
        Value::MappingView { kind, .. } => stdlib_dict_view_type_name(*kind),
        Value::MappingProxy { .. } | Value::MappingProxyObject { .. } => "mappingproxy",
        Value::ChainMap { .. } => "ChainMap",
        Value::UserList { .. } => "UserList",
        Value::UserDict { .. } => "UserDict",
        Value::NamedTupleType(_) => "type",
        Value::NamedTuple { typ, .. } => typ.name.as_str(),
        Value::NamedTupleFieldDescriptor { .. } => "namedtuple_field_descriptor",
        Value::NamedTupleTypeMethod { .. } | Value::NamedTupleInstanceMethod { .. } => "method",
        Value::SimpleNamespace { .. } => "types.SimpleNamespace",
        Value::PicklePayload(_) => "pickle payload",
        Value::AstNode { kind, .. } => kind.as_str(),
        Value::CodeObject { .. } => "code",
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
        Value::CallIterator { .. } => "callable_iterator",
        Value::SequenceIterator { .. } => "iterator",
        Value::Iterator(state) => stdlib_iterator_type_name(&state.borrow()),
        Value::Function { .. } => "function",
        Value::Generator(_) => "generator",
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
        Value::DeferredTypeParamExpr(_) => "DeferredTypeParamExpr",
        Value::TypeAlias { .. } => "TypeAliasType",
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
        Value::BoundMethod { .. } => "method",
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
        Value::CallIterator { .. } => "callable_iterator",
        Value::SequenceIterator { .. } => "iterator",
        Value::Iterator(state) => stdlib_iterator_type_name(&state.borrow()),
        _ => "iterator",
    }
}

fn stdlib_dict_view_type_name(kind: DictViewKind) -> &'static str {
    match kind {
        DictViewKind::Keys => "dict_keys",
        DictViewKind::Values => "dict_values",
        DictViewKind::Items => "dict_items",
    }
}

fn stdlib_dict_ref_from_entries(entries: Vec<(Value, Value)>) -> DictRef {
    Rc::new(RefCell::new(DictStorage::new(entries)))
}

fn reject_stdlib_keywords(name: &str, keywords: &[(String, Value)]) -> Result<(), String> {
    if keywords.is_empty() {
        Ok(())
    } else {
        Err(format!("{name}() does not accept keyword arguments"))
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
        builtin_type_value("tuple"),
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
        ("PyCF_ONLY_AST", Value::Number(PYCF_ONLY_AST)),
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
