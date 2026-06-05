use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::bytecode::Register;
use crate::bytecode::{ExceptHandler, Instruction, instructions_without_debug_positions};

pub type Scope = Rc<RefCell<HashMap<String, Value>>>;
pub type ListRef = Rc<RefCell<Vec<Value>>>;
pub type TupleRef = Rc<Vec<Value>>;
pub type SetRef = Rc<RefCell<Vec<Value>>>;
pub type FrozenSetRef = Rc<Vec<Value>>;
pub type FloatRef = Rc<f64>;
pub type DictRef = Rc<RefCell<DictStorage>>;
pub type ByteArrayRef = Rc<RefCell<ByteArrayStorage>>;
pub type MemoryViewRef = Rc<RefCell<MemoryViewState>>;
pub type NamedTupleTypeRef = Rc<NamedTupleType>;
pub type DeferredTypeParamExprRef = Rc<DeferredTypeParamExpr>;

pub const EXCEPTION_TRACEBACK_ATTR: &str = "\0minipython_traceback";
pub const INT_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_int_storage";
pub const NAMED_TUPLE_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_namedtuple_storage";

#[derive(Debug, Clone)]
pub struct NamedTupleType {
    pub name: String,
    pub fields: Vec<String>,
    pub field_docs: Vec<RefCell<String>>,
    pub field_defaults: Vec<(String, Value)>,
    pub new_defaults: Option<Vec<Value>>,
    pub module: Value,
    pub doc: RefCell<String>,
    pub identity: Rc<()>,
}

#[derive(Debug, Clone)]
pub struct ByteArrayStorage {
    bytes: Vec<u8>,
    exports: usize,
}

#[derive(Debug)]
pub struct DictStorage {
    pub entries: Vec<(Value, Value)>,
    pub version: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryViewState {
    pub bytes: ByteArrayRef,
    pub obj: Value,
    pub exported_bytearray: Option<ByteArrayRef>,
    pub hash_cache: Option<Vec<u8>>,
    pub format: String,
    pub offset: usize,
    pub len: usize,
    pub stride: isize,
    pub readonly: bool,
    pub released: bool,
}

impl ByteArrayStorage {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, exports: 0 }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn has_active_exports(&self) -> bool {
        self.exports > 0
    }

    pub fn retain_export(&mut self) {
        self.exports += 1;
    }

    pub fn release_export(&mut self) {
        self.exports = self.exports.saturating_sub(1);
    }
}

impl std::ops::Deref for ByteArrayStorage {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl std::ops::DerefMut for ByteArrayStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}

impl PartialEq for ByteArrayStorage {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for ByteArrayStorage {}

impl DictStorage {
    pub fn new(entries: Vec<(Value, Value)>) -> Self {
        Self {
            entries,
            version: 0,
        }
    }
}

impl std::ops::Deref for DictStorage {
    type Target = Vec<(Value, Value)>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl std::ops::DerefMut for DictStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictViewKind {
    Keys,
    Values,
    Items,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeMode {
    Exec,
    Eval,
    Single,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLineSpan {
    pub start: usize,
    pub end: usize,
    pub line: i64,
    pub column: Option<i64>,
    pub end_column: Option<i64>,
}

pub fn list_value(items: Vec<Value>) -> Value {
    Value::List(Rc::new(RefCell::new(items)))
}

pub fn tuple_value(items: Vec<Value>) -> Value {
    Value::Tuple(Rc::new(items))
}

pub fn float_value(value: f64) -> Value {
    Value::Float(Rc::new(value))
}

pub fn set_value(items: Vec<Value>) -> Value {
    Value::Set(Rc::new(RefCell::new(items)))
}

thread_local! {
    static EMPTY_FROZEN_SET: FrozenSetRef = Rc::new(Vec::new());
}

pub fn frozen_set_value(items: Vec<Value>) -> Value {
    if items.is_empty() {
        return EMPTY_FROZEN_SET.with(|items| Value::FrozenSet(items.clone()));
    }

    Value::FrozenSet(Rc::new(items))
}

pub fn dict_value(entries: Vec<(Value, Value)>) -> Value {
    Value::Dict(Rc::new(RefCell::new(DictStorage::new(entries))))
}

pub fn byte_array_value(bytes: Vec<u8>) -> Value {
    Value::ByteArray(Rc::new(RefCell::new(ByteArrayStorage::new(bytes))))
}

pub fn memory_view_value(bytes: Vec<u8>, readonly: bool) -> Value {
    let len = bytes.len();
    let obj = Value::Bytes(bytes.clone());
    memory_view_from_parts(
        Rc::new(RefCell::new(ByteArrayStorage::new(bytes))),
        obj,
        0,
        len,
        1,
        readonly,
    )
}

pub fn memory_view_from_byte_array(bytes: ByteArrayRef, readonly: bool) -> Value {
    let len = bytes.borrow().len();
    let obj = Value::ByteArray(bytes.clone());
    memory_view_from_parts(bytes, obj, 0, len, 1, readonly)
}

pub fn memory_view_from_parts(
    bytes: ByteArrayRef,
    obj: Value,
    offset: usize,
    len: usize,
    stride: isize,
    readonly: bool,
) -> Value {
    memory_view_from_parts_with_format(bytes, obj, offset, len, stride, readonly, "B".to_string())
}

pub fn memory_view_from_parts_with_format(
    bytes: ByteArrayRef,
    obj: Value,
    offset: usize,
    len: usize,
    stride: isize,
    readonly: bool,
    format: String,
) -> Value {
    let exported_bytearray = match &obj {
        Value::ByteArray(bytearray) => {
            bytearray.borrow_mut().retain_export();
            Some(bytearray.clone())
        }
        _ => None,
    };
    Value::MemoryView(Rc::new(RefCell::new(MemoryViewState {
        bytes,
        obj,
        exported_bytearray,
        hash_cache: None,
        format,
        offset,
        len,
        stride,
        readonly,
        released: false,
    })))
}

fn memory_view_physical_index(state: &MemoryViewState, logical_index: usize) -> Option<usize> {
    let offset = isize::try_from(state.offset).ok()?;
    let logical_index = isize::try_from(logical_index).ok()?;
    let physical_index = offset.checked_add(logical_index.checked_mul(state.stride)?)?;
    usize::try_from(physical_index).ok()
}

fn memory_view_state_bytes(state: &MemoryViewState) -> Option<Vec<u8>> {
    if state.released {
        return None;
    }
    let bytes = state.bytes.borrow();
    let mut view = Vec::with_capacity(state.len);
    for logical_index in 0..state.len {
        let physical_index = memory_view_physical_index(state, logical_index)?;
        view.push(*bytes.get(physical_index)?);
    }
    Some(view)
}

pub fn dict_view_value(kind: DictViewKind, entries: DictRef) -> Value {
    Value::DictView { kind, entries }
}

pub fn mapping_view_value(kind: DictViewKind, mapping: Value) -> Value {
    Value::MappingView {
        kind,
        mapping: Box::new(mapping),
    }
}

pub fn mapping_proxy_value(entries: DictRef) -> Value {
    Value::MappingProxy { entries }
}

pub fn dict_view_values(kind: DictViewKind, entries: &DictRef) -> Vec<Value> {
    entries
        .borrow()
        .iter()
        .map(|(key, value)| match kind {
            DictViewKind::Keys => key.clone(),
            DictViewKind::Values => value.clone(),
            DictViewKind::Items => tuple_value(vec![key.clone(), value.clone()]),
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct GeneratorState {
    pub name: String,
    pub instructions: Vec<Instruction>,
    pub ip: usize,
    pub registers: Vec<Option<Value>>,
    pub globals: Scope,
    pub locals: Scope,
    pub closure: Vec<Scope>,
    pub current_class: Option<Value>,
    pub first_arg_name: Option<String>,
    pub qualname_prefix: Option<String>,
    pub exception_handlers: Vec<Vec<ExceptHandler>>,
    pub current_exception: Option<Value>,
    pub pending_exception_after_clear: Option<Value>,
    pub resume_dst: Option<Register>,
    pub done: bool,
    pub is_iterable_coroutine: bool,
    pub first_line: usize,
    pub line_sequence: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct CoroutineState {
    pub name: String,
    pub instructions: Vec<Instruction>,
    pub ip: usize,
    pub registers: Vec<Option<Value>>,
    pub globals: Scope,
    pub locals: Scope,
    pub closure: Vec<Scope>,
    pub current_class: Option<Value>,
    pub first_arg_name: Option<String>,
    pub qualname_prefix: Option<String>,
    pub exception_handlers: Vec<Vec<ExceptHandler>>,
    pub current_exception: Option<Value>,
    pub pending_exception_after_clear: Option<Value>,
    pub done: bool,
    pub first_line: usize,
    pub line_sequence: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateInterpolation {
    pub value: Box<Value>,
    pub expression: String,
    pub conversion: Option<String>,
    pub format_spec: String,
}

#[derive(Debug, Clone)]
pub struct DeferredTypeParamExpr {
    pub body: Vec<Instruction>,
    pub globals: Scope,
    pub locals: Option<Scope>,
    pub type_param_scope: Option<Scope>,
    pub closure: Vec<Scope>,
    pub class_name: Option<String>,
    pub class_value: Rc<RefCell<Option<Value>>>,
    pub is_constraint_tuple: bool,
    pub identity: Rc<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstEvaluatorKind {
    TypeAliasValue,
    TypeParamBound,
    TypeParamConstraints,
    TypeParamDefault,
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(i64),
    BigInt(BigInt),
    Float(FloatRef),
    Complex {
        real: f64,
        imag: f64,
    },
    String(String),
    Bytes(Vec<u8>),
    ByteArray(ByteArrayRef),
    MemoryView(MemoryViewRef),
    Bool(bool),
    List(ListRef),
    Tuple(TupleRef),
    Set(SetRef),
    FrozenSet(FrozenSetRef),
    Dict(DictRef),
    OrderedDict(DictRef),
    ScopeDict(Scope),
    DictView {
        kind: DictViewKind,
        entries: DictRef,
    },
    MappingView {
        kind: DictViewKind,
        mapping: Box<Value>,
    },
    MappingProxy {
        entries: DictRef,
    },
    MappingProxyObject {
        mapping: Box<Value>,
    },
    ChainMap {
        maps: Vec<Value>,
    },
    Counter {
        entries: DictRef,
    },
    UserList {
        data: ListRef,
        attrs: DictRef,
    },
    UserDict {
        data: DictRef,
        attrs: DictRef,
    },
    NamedTupleType(NamedTupleTypeRef),
    NamedTuple {
        typ: NamedTupleTypeRef,
        values: TupleRef,
    },
    NamedTupleFieldDescriptor {
        typ: NamedTupleTypeRef,
        index: usize,
    },
    NamedTupleTypeMethod {
        typ: NamedTupleTypeRef,
        name: String,
    },
    NamedTupleInstanceMethod {
        instance: Box<Value>,
        name: String,
    },
    SimpleNamespace {
        fields: DictRef,
    },
    PicklePayload(Box<Value>),
    AstNode {
        kind: String,
        fields: Vec<String>,
        attrs: DictRef,
        identity: Rc<()>,
    },
    CodeObject {
        mode: CodeMode,
        filename: String,
        instructions: Vec<Instruction>,
        line_spans: Vec<CodeLineSpan>,
        varnames: Vec<String>,
        consts: Vec<Value>,
        flags: i64,
        freevars: Vec<String>,
        name: String,
        identity: Rc<()>,
    },
    Cell {
        name: String,
        scope: Scope,
        identity: Rc<()>,
    },
    DeferredTypeParamExpr(DeferredTypeParamExprRef),
    Traceback {
        identity: Rc<()>,
    },
    Slice {
        start: Option<Box<Value>>,
        stop: Option<Box<Value>>,
        step: Option<Box<Value>>,
    },
    Range {
        start: BigInt,
        stop: BigInt,
        step: BigInt,
    },
    RangeIterator {
        current: BigInt,
        stop: BigInt,
        step: BigInt,
    },
    ListIterator {
        items: ListRef,
        index: usize,
        exhausted: bool,
    },
    TupleIterator {
        items: Vec<Value>,
        index: usize,
    },
    TemplateIterator {
        items: Vec<Value>,
        index: usize,
    },
    StringIterator {
        chars: Vec<String>,
        index: usize,
    },
    BytesIterator {
        bytes: Vec<u8>,
        index: usize,
    },
    ByteArrayIterator {
        bytes: ByteArrayRef,
        index: usize,
        exhausted: bool,
    },
    SetIterator {
        items: Vec<Value>,
        index: usize,
        source: Option<SetRef>,
        expected_len: usize,
    },
    DictIterator {
        kind: DictViewKind,
        entries: DictRef,
        index: usize,
        expected_len: usize,
        expected_version: usize,
    },
    ReverseIterator {
        items: Vec<Value>,
        index: usize,
    },
    DictReverseIterator {
        kind: DictViewKind,
        entries: DictRef,
        keys: Vec<Value>,
        index: usize,
        expected_len: usize,
        expected_version: usize,
    },
    EnumerateIterator {
        iterator: Box<Value>,
        index: BigInt,
    },
    ZipIterator {
        iterators: Vec<Value>,
        strict: bool,
    },
    MapIterator {
        function: Box<Value>,
        iterators: Vec<Value>,
        strict: bool,
    },
    FilterIterator {
        function: Box<Value>,
        iterator: Box<Value>,
    },
    CallIterator {
        callable: Box<Value>,
        sentinel: Box<Value>,
        done: bool,
    },
    SequenceIterator {
        object: Box<Value>,
        index: i64,
    },
    SequenceReverseIterator {
        object: Box<Value>,
        index: i64,
    },
    Iterator(Rc<RefCell<Value>>),
    Function {
        name: String,
        type_params: Vec<Value>,
        globals: Scope,
        positional_only: Vec<String>,
        params: Vec<String>,
        defaults: Vec<(String, Value)>,
        vararg: Option<String>,
        keyword_only: Vec<String>,
        keyword_defaults: Vec<(String, Value)>,
        kwarg: Option<String>,
        annotations: Vec<(String, Value)>,
        doc: Rc<RefCell<Value>>,
        attrs: Scope,
        closure: Vec<Scope>,
        body: Vec<Instruction>,
        is_generator: bool,
        is_async: bool,
        first_line: usize,
        line_sequence: Vec<usize>,
        position_columns: Vec<Option<(usize, usize)>>,
        identity: Rc<()>,
        owner_class: Option<Box<Value>>,
    },
    Generator(Rc<RefCell<GeneratorState>>),
    Coroutine(Rc<RefCell<CoroutineState>>),
    CoroutineAwait(Rc<RefCell<CoroutineState>>),
    AwaitIterator(Box<Value>),
    AsyncGenerator(Rc<RefCell<GeneratorState>>),
    AsyncGeneratorNext {
        state: Rc<RefCell<GeneratorState>>,
        send: Box<Value>,
        default: Option<Box<Value>>,
    },
    AsyncGeneratorThrow {
        state: Rc<RefCell<GeneratorState>>,
        exception: Box<Value>,
    },
    AsyncGeneratorClose(Rc<RefCell<GeneratorState>>),
    AsyncGeneratorAthrowMixin {
        typ: Box<Value>,
        val: Box<Value>,
        tb: Box<Value>,
        done: Rc<Cell<bool>>,
    },
    AsyncGeneratorAcloseMixin {
        receiver: Box<Value>,
        done: Rc<Cell<bool>>,
    },
    AnextDefault {
        awaitable: Box<Value>,
        default: Box<Value>,
    },
    Class {
        name: String,
        type_params: Vec<Value>,
        metaclass: Option<Box<Value>>,
        bases: Vec<Value>,
        attrs: Scope,
    },
    TypeParam {
        kind: String,
        name: String,
        bound: Rc<RefCell<Option<Value>>>,
        default: Rc<RefCell<Option<Value>>>,
        infer_variance: bool,
        covariant: bool,
        contravariant: bool,
        identity: Rc<()>,
    },
    TypeAlias {
        name: String,
        type_params: Vec<Value>,
        value: Box<Value>,
    },
    ConstEvaluator {
        kind: ConstEvaluatorKind,
        target: Box<Value>,
    },
    GenericAlias {
        origin: Box<Value>,
        args: Vec<Value>,
    },
    Unpack(Box<Value>),
    Template {
        strings: Vec<String>,
        interpolations: Vec<TemplateInterpolation>,
    },
    TemplateInterpolation(TemplateInterpolation),
    Instance {
        class_name: String,
        fields: Scope,
        class_attrs: Scope,
        class_bases: Vec<Value>,
    },
    Property {
        fget: Option<Box<Value>>,
        fset: Option<Box<Value>>,
        fdel: Option<Box<Value>>,
        doc: Option<Box<Value>>,
    },
    MemberDescriptor {
        name: String,
        owner_name: String,
    },
    StaticMethod {
        function: Box<Value>,
    },
    ClassMethod {
        function: Box<Value>,
    },
    Super {
        class: Box<Value>,
        object: Box<Value>,
        identity: Rc<()>,
    },
    BoundMethod {
        function: Box<Value>,
        receiver: Box<Value>,
        identity: Rc<()>,
    },
    Partial {
        function: Box<Value>,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
        identity: Rc<()>,
    },
    OperatorAttrGetter {
        attrs: Vec<String>,
        identity: Rc<()>,
    },
    OperatorItemGetter {
        items: Vec<Value>,
        identity: Rc<()>,
    },
    OperatorMethodCaller {
        name: String,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
        identity: Rc<()>,
    },
    InspectSignature {
        text: String,
    },
    Module {
        name: String,
        attrs: Scope,
    },
    Exception {
        type_name: String,
        type_hierarchy: Vec<String>,
        type_object: Option<Box<Value>>,
        message: Option<String>,
        args: Vec<Value>,
        attrs: Vec<(String, Value)>,
        exceptions: Option<Vec<Value>>,
        cause: Option<Box<Value>>,
        context: Option<Box<Value>>,
        suppress_context: bool,
        identity: Rc<()>,
    },
    Builtin(String),
    None,
    NotImplemented,
    Ellipsis,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(value) => write!(f, "{value}"),
            Value::BigInt(value) => write!(f, "{value}"),
            Value::Float(value) => write!(f, "{}", format_float_display(**value)),
            Value::Complex { real, imag } => write!(f, "{}", format_complex(*real, *imag)),
            Value::String(value) => write!(f, "{value}"),
            Value::Bytes(value) => write!(f, "{}", repr_bytes(value)),
            Value::ByteArray(value) => write!(f, "bytearray({})", repr_bytes(&value.borrow())),
            Value::MemoryView(view) if view.borrow().released => {
                write!(f, "<released memory at 0x0>")
            }
            Value::MemoryView(_) => write!(f, "<memory at 0x0>"),
            Value::Bool(true) => write!(f, "True"),
            Value::Bool(false) => write!(f, "False"),
            Value::List(items) => {
                let items = items.borrow();
                write!(f, "[{}]", format_list_items(&items))
            }
            Value::Tuple(items) => write!(f, "{}", format_tuple(items)),
            Value::Set(items) => write!(f, "{}", format_set(&items.borrow())),
            Value::FrozenSet(items) => write!(f, "{}", format_frozen_set(items)),
            Value::Dict(entries) => write!(f, "{{{}}}", format_dict(&entries.borrow())),
            Value::OrderedDict(entries) => {
                write!(f, "OrderedDict({{{}}})", format_dict(&entries.borrow()))
            }
            Value::ScopeDict(scope) => write!(f, "{{{}}}", format_scope_dict(scope)),
            Value::DictView { kind, entries } => write!(
                f,
                "{}({})",
                dict_view_type_name(*kind),
                format_dict_view_payload(*kind, entries)
            ),
            Value::MappingView { kind, mapping } => {
                write!(f, "{}({mapping})", dict_view_type_name(*kind))
            }
            Value::MappingProxy { entries } => {
                write!(f, "mappingproxy({{{}}})", format_dict(&entries.borrow()))
            }
            Value::MappingProxyObject { mapping } => write!(f, "mappingproxy({mapping})"),
            Value::ChainMap { maps } => {
                let rendered = maps
                    .iter()
                    .map(|map| map.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "ChainMap({rendered})")
            }
            Value::Counter { entries } => write!(f, "{}", format_counter(&entries.borrow())),
            Value::UserList { data, .. } => {
                let data = data.borrow();
                write!(f, "[{}]", format_list_items(&data))
            }
            Value::UserDict { data, .. } => {
                write!(f, "UserDict({{{}}})", format_dict(&data.borrow()))
            }
            Value::NamedTupleType(typ) => write!(f, "<class '{}'>", typ.name),
            Value::NamedTuple { typ, values } => {
                write!(f, "{}", format_named_tuple(typ, values))
            }
            Value::NamedTupleFieldDescriptor { typ, index } => {
                let field = namedtuple_field_name(typ, *index);
                write!(f, "<namedtuple field '{field}' of '{}'>", typ.name)
            }
            Value::NamedTupleTypeMethod { typ, name } => {
                write!(f, "<bound method {}.{name}>", typ.name)
            }
            Value::NamedTupleInstanceMethod { instance, name } => {
                write!(f, "<bound method {instance}.{name}>")
            }
            Value::SimpleNamespace { fields } => write!(f, "{}", format_simple_namespace(fields)),
            Value::PicklePayload(_) => write!(f, "<pickle payload>"),
            Value::AstNode { kind, .. } => write!(f, "<ast.{kind} object>"),
            Value::CodeObject { filename, .. } => {
                write!(f, "<code object <module>, file \"{filename}\", line 1>")
            }
            Value::Cell { .. } => write!(f, "<cell object>"),
            Value::DeferredTypeParamExpr(_) => write!(f, "<deferred type parameter expression>"),
            Value::Traceback { .. } => write!(f, "<traceback object>"),
            Value::Range { start, stop, step } if step == &BigInt::from(1) => {
                write!(f, "range({start}, {stop})")
            }
            Value::Range { start, stop, step } => write!(f, "range({start}, {stop}, {step})"),
            Value::Slice { start, stop, step } => write!(
                f,
                "slice({}, {}, {})",
                format_slice_part(start),
                format_slice_part(stop),
                format_slice_part(step)
            ),
            Value::RangeIterator { .. } => write!(f, "<range_iterator>"),
            Value::ListIterator { .. } => write!(f, "<list_iterator>"),
            Value::TupleIterator { .. } => write!(f, "<tuple_iterator>"),
            Value::TemplateIterator { .. } => {
                write!(f, "<string.templatelib.TemplateIter object>")
            }
            Value::StringIterator { .. } => write!(f, "<str_iterator>"),
            Value::BytesIterator { .. } => write!(f, "<bytes_iterator>"),
            Value::ByteArrayIterator { .. } => write!(f, "<bytearray_iterator>"),
            Value::SetIterator { .. } => write!(f, "<set_iterator>"),
            Value::DictIterator { .. } => write!(f, "<dict_keyiterator>"),
            Value::ReverseIterator { .. } => write!(f, "<reversed object>"),
            Value::DictReverseIterator { .. } => write!(f, "<dict_reversekeyiterator>"),
            Value::EnumerateIterator { .. } => write!(f, "<enumerate object>"),
            Value::ZipIterator { .. } => write!(f, "<zip object>"),
            Value::MapIterator { .. } => write!(f, "<map object>"),
            Value::FilterIterator { .. } => write!(f, "<filter object>"),
            Value::CallIterator { .. } => write!(f, "<callable_iterator object>"),
            Value::SequenceIterator { .. } => write!(f, "<iterator>"),
            Value::SequenceReverseIterator { .. } => write!(f, "<reversed object>"),
            Value::Iterator(_) => write!(f, "<iterator>"),
            Value::Function { name, .. } => write!(f, "<function {name}>"),
            Value::Generator(state) => write!(f, "<generator object {}>", state.borrow().name),
            Value::Coroutine(state) => write!(f, "<coroutine object {}>", state.borrow().name),
            Value::CoroutineAwait(_) => write!(f, "<coroutine_wrapper object>"),
            Value::AwaitIterator(_) => write!(f, "<await_iterator object>"),
            Value::AsyncGenerator(state) => {
                write!(f, "<async_generator object {}>", state.borrow().name)
            }
            Value::AsyncGeneratorNext { .. } | Value::AsyncGeneratorThrow { .. } => {
                write!(f, "<async_generator_asend object>")
            }
            Value::AsyncGeneratorClose(_) => write!(f, "<async_generator_athrow object>"),
            Value::AsyncGeneratorAthrowMixin { .. } => {
                write!(f, "<coroutine object AsyncGenerator.athrow>")
            }
            Value::AsyncGeneratorAcloseMixin { .. } => {
                write!(f, "<coroutine object AsyncGenerator.aclose>")
            }
            Value::AnextDefault { .. } => write!(f, "<anext_awaitable object>"),
            Value::Class { name, .. } => write!(f, "<class {name}>"),
            Value::TypeParam { name, .. } => write!(f, "{name}"),
            Value::TypeAlias { name, .. } => write!(f, "<type alias {name}>"),
            Value::ConstEvaluator { kind, target } => {
                write!(f, "{}", format_const_evaluator(*kind, target))
            }
            Value::GenericAlias { origin, args } => write!(
                f,
                "{}[{}]",
                format_generic_origin(origin),
                args.iter()
                    .map(format_generic_alias_arg)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Unpack(value) => write!(f, "*{}", format_value_repr(value)),
            Value::Template {
                strings,
                interpolations,
            } => write!(
                f,
                "Template(strings={}, interpolations={})",
                format_string_tuple(strings),
                format_interpolation_tuple(interpolations)
            ),
            Value::TemplateInterpolation(interpolation) => {
                write!(f, "{}", format_template_interpolation(interpolation))
            }
            Value::Instance {
                class_name, fields, ..
            } => {
                if let Some(rendered) = format_int_subclass(fields) {
                    write!(f, "{rendered}")
                } else if let Some(rendered) = format_named_tuple_subclass(class_name, fields) {
                    write!(f, "{rendered}")
                } else {
                    write!(f, "<{class_name} object>")
                }
            }
            Value::Property { .. } => write!(f, "<property object>"),
            Value::MemberDescriptor { name, owner_name } => {
                write!(f, "<member '{name}' of '{owner_name}' objects>")
            }
            Value::StaticMethod { .. } => write!(f, "<staticmethod object>"),
            Value::ClassMethod { .. } => write!(f, "<classmethod object>"),
            Value::Super { .. } => write!(f, "<super object>"),
            Value::BoundMethod {
                function, receiver, ..
            } => {
                write!(f, "{}", format_bound_method(function, receiver))
            }
            Value::Partial { .. } => write!(f, "<functools.partial object>"),
            Value::OperatorAttrGetter { attrs, .. } => {
                write!(f, "{}", format_operator_attrgetter(attrs))
            }
            Value::OperatorItemGetter { items, .. } => {
                write!(f, "{}", format_operator_itemgetter(items))
            }
            Value::OperatorMethodCaller {
                name,
                args,
                keywords,
                ..
            } => {
                write!(f, "{}", format_operator_methodcaller(name, args, keywords))
            }
            Value::InspectSignature { text } => write!(f, "{text}"),
            Value::Module { name, .. } => write!(f, "<module {name}>"),
            Value::Exception {
                message: Some(message),
                exceptions: Some(exceptions),
                ..
            } => write!(
                f,
                "{} ({})",
                message,
                format_subexception_count(exceptions.len())
            ),
            Value::Exception {
                type_name,
                type_hierarchy,
                args,
                message,
                attrs,
                exceptions: None,
                ..
            } => write!(
                f,
                "{}",
                format_exception_display(
                    type_name,
                    type_hierarchy,
                    message.as_deref(),
                    args,
                    attrs
                )
            ),
            Value::Exception {
                message: None,
                exceptions: Some(exceptions),
                ..
            } => write!(f, "({})", format_subexception_count(exceptions.len())),
            Value::Builtin(name) if is_builtin_type_display_name(name) => {
                write!(f, "<class '{}'>", builtin_type_public_name(name))
            }
            Value::Builtin(name) => write!(f, "<builtin {name}>"),
            Value::None => write!(f, "None"),
            Value::NotImplemented => write!(f, "NotImplemented"),
            Value::Ellipsis => write!(f, "Ellipsis"),
        }
    }
}

fn format_list_items(items: &[Value]) -> String {
    items
        .iter()
        .map(format_value_repr)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_operator_attrgetter(attrs: &[String]) -> String {
    let args = attrs
        .iter()
        .map(|attr| repr_string(attr))
        .collect::<Vec<_>>()
        .join(", ");
    format!("operator.attrgetter({args})")
}

fn format_operator_itemgetter(items: &[Value]) -> String {
    format!("operator.itemgetter({})", format_list_items(items))
}

fn format_operator_methodcaller(
    name: &str,
    args: &[Value],
    keywords: &[(String, Value)],
) -> String {
    let mut parts = Vec::with_capacity(1 + args.len() + keywords.len());
    parts.push(repr_string(name));
    parts.extend(args.iter().map(format_value_repr));
    parts.extend(
        keywords
            .iter()
            .map(|(name, value)| format!("{name}={}", format_value_repr(value))),
    );
    format!("operator.methodcaller({})", parts.join(", "))
}

fn format_subexception_count(count: usize) -> String {
    if count == 1 {
        "1 sub-exception".to_string()
    } else {
        format!("{count} sub-exceptions")
    }
}

fn format_exception_display(
    type_name: &str,
    type_hierarchy: &[String],
    message: Option<&str>,
    args: &[Value],
    attrs: &[(String, Value)],
) -> String {
    if type_name == "OSError" || type_hierarchy.iter().any(|name| name == "OSError") {
        return format_os_error_display(args, attrs);
    }

    if type_name == "KeyError" || type_hierarchy.iter().any(|name| name == "KeyError") {
        if let ([value], Some(message)) = (args, message) {
            let rendered = format_value_repr(value);
            if rendered == message {
                return rendered;
            }
        }
    }

    match args {
        [] => message.unwrap_or_default().to_string(),
        [value] => value.to_string(),
        values => format_tuple(values),
    }
}

fn format_os_error_display(args: &[Value], attrs: &[(String, Value)]) -> String {
    if args.len() < 2 {
        return match args {
            [] => String::new(),
            [value] => value.to_string(),
            _ => unreachable!("args.len() is checked above"),
        };
    }

    let errno = exception_attr(attrs, "errno").unwrap_or(&args[0]);
    let strerror = exception_attr(attrs, "strerror").unwrap_or(&args[1]);
    let mut display = format!("[Errno {errno}] {strerror}");

    if let Some(filename) =
        exception_attr(attrs, "filename").filter(|value| !matches!(value, Value::None))
    {
        display.push_str(": ");
        display.push_str(&format_value_repr(filename));

        if let Some(filename2) =
            exception_attr(attrs, "filename2").filter(|value| !matches!(value, Value::None))
        {
            display.push_str(" -> ");
            display.push_str(&format_value_repr(filename2));
        }
    }

    display
}

fn exception_attr<'a>(attrs: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    attrs
        .iter()
        .find_map(|(attr_name, value)| (attr_name == name).then_some(value))
}

fn format_exception_args_repr(args: &[Value]) -> String {
    match args {
        [] => "()".to_string(),
        [value] => format!("({})", format_value_repr(value)),
        values => {
            let rendered = values
                .iter()
                .map(format_value_repr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({rendered})")
        }
    }
}

fn format_bound_method(function: &Value, receiver: &Value) -> String {
    format!(
        "<bound method {} of {}>",
        bound_method_display_name(function),
        format_value_repr(receiver)
    )
}

fn bound_method_display_name(function: &Value) -> String {
    match function {
        Value::Function {
            name,
            owner_class: Some(owner_class),
            ..
        } => match owner_class.as_ref() {
            Value::Class {
                name: owner_name, ..
            } => format!("{owner_name}.{name}"),
            _ => name.clone(),
        },
        Value::Function { name, .. } => name.clone(),
        Value::Builtin(name) => name.clone(),
        _ => "?".to_string(),
    }
}

fn format_value_repr(value: &Value) -> String {
    match value {
        Value::String(value) => repr_string(value),
        Value::Bytes(value) => repr_bytes(value),
        Value::ByteArray(value) => format!("bytearray({})", repr_bytes(&value.borrow())),
        Value::MemoryView(view) if view.borrow().released => "<released memory at 0x0>".to_string(),
        Value::MemoryView(_) => "<memory at 0x0>".to_string(),
        Value::List(items) => {
            let items = items.borrow();
            format!("[{}]", format_list_items(&items))
        }
        Value::Tuple(items) => format_tuple(items),
        Value::Set(items) => format_set(&items.borrow()),
        Value::FrozenSet(items) => format_frozen_set(items),
        Value::Dict(entries) => format!("{{{}}}", format_dict(&entries.borrow())),
        Value::OrderedDict(entries) => {
            format!("OrderedDict({{{}}})", format_dict(&entries.borrow()))
        }
        Value::ScopeDict(scope) => format!("{{{}}}", format_scope_dict(scope)),
        Value::DictView { kind, entries } => {
            format!(
                "{}({})",
                dict_view_type_name(*kind),
                format_dict_view_payload(*kind, entries)
            )
        }
        Value::MappingView { kind, mapping } => {
            format!("{}({mapping})", dict_view_type_name(*kind))
        }
        Value::MappingProxy { entries } => {
            format!("mappingproxy({{{}}})", format_dict(&entries.borrow()))
        }
        Value::MappingProxyObject { mapping } => format!("mappingproxy({mapping})"),
        Value::ChainMap { maps } => {
            let rendered = maps
                .iter()
                .map(format_value_repr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("ChainMap({rendered})")
        }
        Value::Counter { entries } => format_counter(&entries.borrow()),
        Value::UserList { data, .. } => {
            let data = data.borrow();
            format!("[{}]", format_list_items(&data))
        }
        Value::UserDict { data, .. } => {
            format!("UserDict({{{}}})", format_dict(&data.borrow()))
        }
        Value::SimpleNamespace { fields } => format_simple_namespace(fields),
        Value::PicklePayload(_) => "<pickle payload>".to_string(),
        Value::AstNode { kind, .. } => format!("<ast.{kind} object>"),
        Value::CodeObject { filename, .. } => {
            format!("<code object <module>, file \"{filename}\", line 1>")
        }
        Value::Cell { .. } => "<cell object>".to_string(),
        Value::DeferredTypeParamExpr(_) => "<deferred type parameter expression>".to_string(),
        Value::Traceback { .. } => "<traceback object>".to_string(),
        Value::Function { name, .. } => format!("<function {name}>"),
        Value::Generator(state) => format!("<generator object {}>", state.borrow().name),
        Value::Coroutine(state) => format!("<coroutine object {}>", state.borrow().name),
        Value::CoroutineAwait(_) => "<coroutine_wrapper object>".to_string(),
        Value::AwaitIterator(_) => "<await_iterator object>".to_string(),
        Value::AsyncGenerator(state) => format!("<async_generator object {}>", state.borrow().name),
        Value::AsyncGeneratorNext { .. } | Value::AsyncGeneratorThrow { .. } => {
            "<async_generator_asend object>".to_string()
        }
        Value::AsyncGeneratorClose(_) => "<async_generator_athrow object>".to_string(),
        Value::AsyncGeneratorAthrowMixin { .. } => {
            "<coroutine object AsyncGenerator.athrow>".to_string()
        }
        Value::AsyncGeneratorAcloseMixin { .. } => {
            "<coroutine object AsyncGenerator.aclose>".to_string()
        }
        Value::AnextDefault { .. } => "<anext_awaitable object>".to_string(),
        Value::Class { name, .. } => format!("<class {name}>"),
        Value::TypeParam { name, .. } => name.clone(),
        Value::TypeAlias { name, .. } => format!("<type alias {name}>"),
        Value::ConstEvaluator { kind, target } => format_const_evaluator(*kind, target),
        Value::GenericAlias { origin, args } => format!(
            "{}[{}]",
            format_generic_origin(origin),
            args.iter()
                .map(format_generic_alias_arg)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Unpack(value) => format!("*{}", format_value_repr(value)),
        Value::Template {
            strings,
            interpolations,
        } => format!(
            "Template(strings={}, interpolations={})",
            format_string_tuple(strings),
            format_interpolation_tuple(interpolations)
        ),
        Value::TemplateInterpolation(interpolation) => format_template_interpolation(interpolation),
        Value::Instance {
            class_name, fields, ..
        } => format_int_subclass(fields)
            .or_else(|| format_named_tuple_subclass(class_name, fields))
            .unwrap_or_else(|| format!("<{class_name} object>")),
        Value::Property { .. } => "<property object>".to_string(),
        Value::NamedTupleFieldDescriptor { typ, index } => {
            let field = namedtuple_field_name(typ, *index);
            format!("<namedtuple field '{field}' of '{}'>", typ.name)
        }
        Value::MemberDescriptor { name, owner_name } => {
            format!("<member '{name}' of '{owner_name}' objects>")
        }
        Value::StaticMethod { .. } => "<staticmethod object>".to_string(),
        Value::ClassMethod { .. } => "<classmethod object>".to_string(),
        Value::Super { .. } => "<super object>".to_string(),
        Value::BoundMethod {
            function, receiver, ..
        } => format_bound_method(function, receiver),
        Value::Partial { .. } => "<functools.partial object>".to_string(),
        Value::OperatorAttrGetter { attrs, .. } => format_operator_attrgetter(attrs),
        Value::OperatorItemGetter { items, .. } => format_operator_itemgetter(items),
        Value::OperatorMethodCaller {
            name,
            args,
            keywords,
            ..
        } => format_operator_methodcaller(name, args, keywords),
        Value::InspectSignature { text } => format!("<Signature {text}>"),
        Value::Module { name, .. } => format!("<module {name}>"),
        Value::Exception {
            type_name,
            message,
            exceptions: Some(exceptions),
            ..
        } => format!(
            "{type_name}({:?}, [{}])",
            message.clone().unwrap_or_default(),
            exceptions
                .iter()
                .map(format_value_repr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Exception {
            type_name, args, ..
        } => format!("{type_name}{}", format_exception_args_repr(args)),
        value => value.to_string(),
    }
}

fn format_value_repr_with_namespace_seen(value: &Value, active: &mut HashSet<usize>) -> String {
    match value {
        Value::SimpleNamespace { fields } => format_simple_namespace_inner(fields, active),
        _ => format_value_repr(value),
    }
}

fn format_complex(real: f64, imag: f64) -> String {
    if real == 0.0 && !real.is_sign_negative() {
        return format!("{}j", format_complex_part(imag));
    }

    let (sign, imag_abs) = if imag.is_nan() {
        ('+', imag)
    } else if imag.is_sign_negative() {
        ('-', -imag)
    } else {
        ('+', imag)
    };

    format!(
        "({}{}{}j)",
        format_complex_part(real),
        sign,
        format_complex_part(imag_abs)
    )
}

fn format_complex_part(value: f64) -> String {
    if value == 0.0 && value.is_sign_negative() {
        return "-0".to_string();
    }

    if value.is_finite() && value.fract() == 0.0 {
        if value >= i64::MIN as f64 && value <= i64::MAX as f64 {
            return format!("{}", value as i64);
        }
    }

    format_float_display(value)
}

pub(crate) fn format_float_display(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value == f64::INFINITY {
        return "inf".to_string();
    }
    if value == f64::NEG_INFINITY {
        return "-inf".to_string();
    }

    normalize_float_display_exponent(format!("{value:?}"))
}

fn normalize_float_display_exponent(value: String) -> String {
    let Some(index) = value.find(['e', 'E']) else {
        return value;
    };
    let (mantissa, exponent) = value.split_at(index);
    let marker = &exponent[..1];
    let exponent = exponent[1..].parse::<i32>().unwrap_or(0);
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("{mantissa}{marker}{sign}{:02}", exponent.abs())
}

fn is_builtin_type_display_name(name: &str) -> bool {
    matches!(
        name,
        "object"
            | "type"
            | "bool"
            | "int"
            | "float"
            | "complex"
            | "str"
            | "bytes"
            | "bytearray"
            | "memoryview"
            | "list"
            | "tuple"
            | "dict"
            | "set"
            | "frozenset"
            | "range"
            | "slice"
            | "mappingproxy"
            | "ChainMap"
            | "Counter"
            | "OrderedDict"
            | "UserList"
            | "UserDict"
            | "UserString"
            | "SimpleNamespace"
            | "property"
            | "super"
            | "staticmethod"
            | "classmethod"
            | "Generic"
            | "Template"
            | "Interpolation"
            | "TemplateIter"
            | "Hashable"
            | "Iterable"
            | "Iterator"
            | "Generator"
            | "Reversible"
            | "Awaitable"
            | "Coroutine"
            | "AsyncIterable"
            | "AsyncIterator"
            | "AsyncGenerator"
            | "Sized"
            | "Container"
            | "Callable"
            | "Collection"
            | "Buffer"
            | "Sequence"
            | "MutableSequence"
            | "ByteString"
            | "Mapping"
            | "MutableMapping"
            | "MappingView"
            | "KeysView"
            | "ItemsView"
            | "ValuesView"
            | "MutableSet"
            | "NodeVisitor"
            | "NodeTransformer"
            | "NoneType"
    ) || name.starts_with("ast.")
        || name
            .strip_prefix("typing.")
            .is_some_and(|name| matches!(name, "TypeVar" | "TypeVarTuple" | "ParamSpec"))
}

fn builtin_type_public_name(name: &str) -> &str {
    name.strip_prefix("typing.")
        .or_else(|| name.strip_prefix("ast."))
        .unwrap_or(name)
}

fn format_slice_part(value: &Option<Box<Value>>) -> String {
    value
        .as_deref()
        .map(format_value_repr)
        .unwrap_or_else(|| "None".to_string())
}

fn format_generic_origin(origin: &Value) -> String {
    match origin {
        Value::Builtin(name) | Value::Class { name, .. } | Value::TypeParam { name, .. } => {
            name.clone()
        }
        Value::TypeAlias { name, .. } => name.clone(),
        value => format_value_repr(value),
    }
}

fn format_generic_alias_arg(value: &Value) -> String {
    match value {
        Value::Builtin(name) | Value::Class { name, .. } | Value::TypeParam { name, .. } => {
            name.clone()
        }
        Value::TypeAlias { name, .. } => name.clone(),
        Value::Unpack(value) => format!("*{}", format_generic_alias_arg(value)),
        value => format_value_repr(value),
    }
}

fn format_const_evaluator(kind: ConstEvaluatorKind, target: &Value) -> String {
    format!(
        "<constevaluator {}>",
        format_const_evaluator_target(kind, target)
    )
}

fn format_const_evaluator_target(kind: ConstEvaluatorKind, target: &Value) -> String {
    let value = match (kind, target) {
        (ConstEvaluatorKind::TypeAliasValue, Value::TypeAlias { value, .. }) => {
            Some(value.as_ref().clone())
        }
        (ConstEvaluatorKind::TypeParamBound, Value::TypeParam { bound, .. }) => {
            match bound.borrow().as_ref() {
                Some(Value::Tuple(_)) | None => Some(Value::None),
                Some(value) => Some(value.clone()),
            }
        }
        (ConstEvaluatorKind::TypeParamConstraints, Value::TypeParam { bound, .. }) => {
            match bound.borrow().as_ref() {
                Some(Value::Tuple(items)) => Some(Value::Tuple(items.clone())),
                _ => Some(Value::Tuple(Rc::new(Vec::new()))),
            }
        }
        (ConstEvaluatorKind::TypeParamDefault, Value::TypeParam { default, .. }) => {
            default.borrow().clone()
        }
        _ => None,
    };

    value
        .as_ref()
        .map(format_const_evaluator_value)
        .unwrap_or_else(|| "?".to_string())
}

fn format_const_evaluator_value(value: &Value) -> String {
    match value {
        Value::Builtin(name) => {
            format!("<class '{}'>", name.strip_prefix("typing.").unwrap_or(name))
        }
        Value::Tuple(items) => match items.as_slice() {
            [] => "()".to_string(),
            [item] => format!("({},)", format_const_evaluator_value(item)),
            _ => format!(
                "({})",
                items
                    .iter()
                    .map(format_const_evaluator_value)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
        value => format_value_repr(value),
    }
}

fn bool_as_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn simple_namespace_entries_equal(left: &[(Value, Value)], right: &[(Value, Value)]) -> bool {
    code_metadata_namespace_entries_equal(left, right).unwrap_or(left == right)
}

pub(crate) fn code_metadata_namespace_entries_equal(
    left: &[(Value, Value)],
    right: &[(Value, Value)],
) -> Option<bool> {
    let left_consts = code_metadata_consts_field(left)?;
    let right_consts = code_metadata_consts_field(right)?;
    Some(
        string_field_value(left, "co_varnames") == string_field_value(right, "co_varnames")
            && string_field_value(left, "co_flags") == string_field_value(right, "co_flags")
            && string_field_value(left, "co_firstlineno")
                == string_field_value(right, "co_firstlineno")
            && strict_constant_value_equal(left_consts, right_consts),
    )
}

fn code_metadata_consts_field(entries: &[(Value, Value)]) -> Option<&Value> {
    let consts = string_field_value(entries, "co_consts")?;
    if string_field_value(entries, "co_varnames").is_some()
        && string_field_value(entries, "co_flags").is_some()
    {
        Some(consts)
    } else {
        None
    }
}

fn code_object_instructions_equal(left: &[Instruction], right: &[Instruction]) -> bool {
    left == right
        || instructions_without_debug_positions(left) == instructions_without_debug_positions(right)
}

fn string_field_value<'a>(entries: &'a [(Value, Value)], name: &str) -> Option<&'a Value> {
    entries.iter().find_map(|(key, value)| match key {
        Value::String(key) if key == name => Some(value),
        _ => None,
    })
}

fn strict_constant_value_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left == right,
        (Value::BigInt(left), Value::BigInt(right)) => left == right,
        (Value::Number(left), Value::BigInt(right))
        | (Value::BigInt(right), Value::Number(left)) => BigInt::from(*left) == *right,
        (Value::Float(left), Value::Float(right)) => left.to_bits() == right.to_bits(),
        (
            Value::Complex {
                real: left_real,
                imag: left_imag,
            },
            Value::Complex {
                real: right_real,
                imag: right_imag,
            },
        ) => {
            left_real.to_bits() == right_real.to_bits()
                && left_imag.to_bits() == right_imag.to_bits()
        }
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Bytes(left), Value::Bytes(right)) => left == right,
        (Value::None, Value::None) => true,
        (Value::Ellipsis, Value::Ellipsis) => true,
        (Value::Tuple(left), Value::Tuple(right)) => {
            strict_constant_slices_equal(left.as_ref(), right.as_ref())
        }
        (Value::FrozenSet(left), Value::FrozenSet(right)) => {
            strict_constant_sets_equal(left.as_ref(), right.as_ref())
        }
        (
            Value::CodeObject {
                mode: left_mode,
                instructions: left_instructions,
                consts: left_consts,
                varnames: left_varnames,
                flags: left_flags,
                freevars: left_freevars,
                ..
            },
            Value::CodeObject {
                mode: right_mode,
                instructions: right_instructions,
                consts: right_consts,
                varnames: right_varnames,
                flags: right_flags,
                freevars: right_freevars,
                ..
            },
        ) => {
            left_mode == right_mode
                && code_object_instructions_equal(left_instructions, right_instructions)
                && strict_constant_slices_equal(left_consts, right_consts)
                && left_varnames == right_varnames
                && left_flags == right_flags
                && left_freevars == right_freevars
        }
        (
            Value::Cell {
                identity: left_identity,
                ..
            },
            Value::Cell {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        _ => false,
    }
}

fn strict_constant_slices_equal(left: &[Value], right: &[Value]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| strict_constant_value_equal(left, right))
}

fn strict_constant_sets_equal(left: &[Value], right: &[Value]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut matched = vec![false; right.len()];
    'left_items: for left_item in left {
        for (index, right_item) in right.iter().enumerate() {
            if !matched[index] && strict_constant_value_equal(left_item, right_item) {
                matched[index] = true;
                continue 'left_items;
            }
        }
        return false;
    }
    true
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(left), Value::Number(right)) => left == right,
            (Value::BigInt(left), Value::BigInt(right)) => left == right,
            (Value::Number(left), Value::BigInt(right))
            | (Value::BigInt(right), Value::Number(left)) => BigInt::from(*left) == *right,
            (Value::Float(left), Value::Float(right)) => left == right,
            (
                Value::Complex {
                    real: left_real,
                    imag: left_imag,
                },
                Value::Complex {
                    real: right_real,
                    imag: right_imag,
                },
            ) => left_real == right_real && left_imag == right_imag,
            (Value::Number(left), Value::Float(right)) => (*left as f64) == **right,
            (Value::Float(left), Value::Number(right)) => **left == (*right as f64),
            (Value::BigInt(left), Value::Float(right)) => {
                left.to_f64().is_some_and(|left| left == **right)
            }
            (Value::Float(left), Value::BigInt(right)) => {
                right.to_f64().is_some_and(|right| **left == right)
            }
            (Value::Number(left), Value::Complex { real, imag })
            | (Value::Complex { real, imag }, Value::Number(left)) => {
                (*left as f64) == *real && *imag == 0.0
            }
            (Value::BigInt(left), Value::Complex { real, imag })
            | (Value::Complex { real, imag }, Value::BigInt(left)) => {
                left.to_f64().is_some_and(|left| left == *real) && *imag == 0.0
            }
            (Value::Float(left), Value::Complex { real, imag })
            | (Value::Complex { real, imag }, Value::Float(left)) => {
                **left == *real && *imag == 0.0
            }
            (Value::Bool(left), Value::Number(right))
            | (Value::Number(right), Value::Bool(left)) => bool_as_i64(*left) == *right,
            (Value::Bool(left), Value::BigInt(right))
            | (Value::BigInt(right), Value::Bool(left)) => {
                BigInt::from(bool_as_i64(*left)) == *right
            }
            (Value::Bool(left), Value::Float(right)) | (Value::Float(right), Value::Bool(left)) => {
                bool_as_i64(*left) as f64 == **right
            }
            (Value::Bool(left), Value::Complex { real, imag })
            | (Value::Complex { real, imag }, Value::Bool(left)) => {
                bool_as_i64(*left) as f64 == *real && *imag == 0.0
            }
            (Value::String(left), Value::String(right)) => left == right,
            (Value::Bytes(left), Value::Bytes(right)) => left == right,
            (Value::ByteArray(left), Value::ByteArray(right)) => *left.borrow() == *right.borrow(),
            (Value::MemoryView(left), Value::MemoryView(right)) if Rc::ptr_eq(left, right) => true,
            (Value::MemoryView(left), Value::MemoryView(right)) => {
                let left = left.borrow();
                let right = right.borrow();
                match (
                    memory_view_state_bytes(&left),
                    memory_view_state_bytes(&right),
                ) {
                    (Some(left), Some(right)) => left == right,
                    _ => false,
                }
            }
            (Value::MemoryView(left), Value::Bytes(right))
            | (Value::Bytes(right), Value::MemoryView(left)) => {
                let left = left.borrow();
                memory_view_state_bytes(&left).is_some_and(|left| left == *right)
            }
            (Value::MemoryView(left), Value::ByteArray(right))
            | (Value::ByteArray(right), Value::MemoryView(left)) => {
                let left = left.borrow();
                memory_view_state_bytes(&left)
                    .is_some_and(|left| left.as_slice() == right.borrow().bytes())
            }
            (Value::Bytes(left), Value::ByteArray(right))
            | (Value::ByteArray(right), Value::Bytes(left)) => {
                left.as_slice() == right.borrow().bytes()
            }
            (Value::Bool(left), Value::Bool(right)) => left == right,
            (Value::List(left), Value::List(right)) => *left.borrow() == *right.borrow(),
            (Value::UserList { data: left, .. }, Value::UserList { data: right, .. }) => {
                *left.borrow() == *right.borrow()
            }
            (Value::List(left), Value::UserList { data: right, .. })
            | (Value::UserList { data: right, .. }, Value::List(left)) => {
                *left.borrow() == *right.borrow()
            }
            (Value::Tuple(left), Value::Tuple(right)) => left.as_ref() == right.as_ref(),
            (Value::NamedTuple { values: left, .. }, Value::NamedTuple { values: right, .. }) => {
                left.as_ref() == right.as_ref()
            }
            (Value::NamedTuple { values: left, .. }, Value::Tuple(right))
            | (Value::Tuple(right), Value::NamedTuple { values: left, .. }) => {
                left.as_ref() == right.as_ref()
            }
            (
                Value::NamedTupleFieldDescriptor {
                    typ: left_typ,
                    index: left_index,
                },
                Value::NamedTupleFieldDescriptor {
                    typ: right_typ,
                    index: right_index,
                },
            ) => Rc::ptr_eq(left_typ, right_typ) && left_index == right_index,
            (Value::Set(left), Value::Set(right)) => sets_equal(&left.borrow(), &right.borrow()),
            (Value::FrozenSet(left), Value::FrozenSet(right)) => {
                sets_equal(left.as_ref(), right.as_ref())
            }
            (Value::Set(left), Value::FrozenSet(right))
            | (Value::FrozenSet(right), Value::Set(left)) => {
                sets_equal(&left.borrow(), right.as_ref())
            }
            (Value::Dict(left), Value::Dict(right)) => {
                left.borrow().entries == right.borrow().entries
            }
            (Value::OrderedDict(left), Value::OrderedDict(right)) => {
                left.borrow().entries == right.borrow().entries
            }
            (Value::Dict(left), Value::OrderedDict(right))
            | (Value::OrderedDict(left), Value::Dict(right)) => {
                dict_entries_equal(&left.borrow().entries, &right.borrow().entries)
            }
            (Value::UserDict { data: left, .. }, Value::UserDict { data: right, .. }) => {
                left.borrow().entries == right.borrow().entries
            }
            (Value::SimpleNamespace { fields: left }, Value::SimpleNamespace { fields: right }) => {
                simple_namespace_entries_equal(&left.borrow().entries, &right.borrow().entries)
            }
            (Value::PicklePayload(left), Value::PicklePayload(right)) => left == right,
            (
                Value::CodeObject {
                    mode: left_mode,
                    instructions: left_instructions,
                    consts: left_consts,
                    varnames: left_varnames,
                    flags: left_flags,
                    freevars: left_freevars,
                    ..
                },
                Value::CodeObject {
                    mode: right_mode,
                    instructions: right_instructions,
                    consts: right_consts,
                    varnames: right_varnames,
                    flags: right_flags,
                    freevars: right_freevars,
                    ..
                },
            ) => {
                left_mode == right_mode
                    && code_object_instructions_equal(left_instructions, right_instructions)
                    && strict_constant_slices_equal(left_consts, right_consts)
                    && left_varnames == right_varnames
                    && left_flags == right_flags
                    && left_freevars == right_freevars
            }
            (
                Value::Cell {
                    identity: left_identity,
                    ..
                },
                Value::Cell {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (Value::ScopeDict(left), Value::ScopeDict(right)) => Rc::ptr_eq(left, right),
            (
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                },
                Value::DictView {
                    kind: right_kind,
                    entries: right_entries,
                },
            ) if dict_view_is_set_like(*left_kind) && dict_view_is_set_like(*right_kind) => {
                sets_equal(
                    &dict_view_values(*left_kind, left_entries),
                    &dict_view_values(*right_kind, right_entries),
                )
            }
            (
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                },
                Value::Set(right),
            )
            | (
                Value::Set(right),
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                },
            ) if dict_view_is_set_like(*left_kind) => {
                sets_equal(&dict_view_values(*left_kind, left_entries), &right.borrow())
            }
            (
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                },
                Value::FrozenSet(right),
            )
            | (
                Value::FrozenSet(right),
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                },
            ) if dict_view_is_set_like(*left_kind) => {
                sets_equal(&dict_view_values(*left_kind, left_entries), right.as_ref())
            }
            (
                Value::MappingView {
                    kind: left_kind,
                    mapping: left_mapping,
                },
                Value::MappingView {
                    kind: right_kind,
                    mapping: right_mapping,
                },
            ) => left_kind == right_kind && left_mapping == right_mapping,
            (
                Value::MappingProxy {
                    entries: left_entries,
                },
                Value::MappingProxy {
                    entries: right_entries,
                },
            ) => left_entries.borrow().entries == right_entries.borrow().entries,
            (Value::MappingProxy { entries: left }, Value::Dict(right))
            | (Value::Dict(right), Value::MappingProxy { entries: left })
            | (Value::MappingProxy { entries: left }, Value::OrderedDict(right))
            | (Value::OrderedDict(right), Value::MappingProxy { entries: left }) => {
                left.borrow().entries == right.borrow().entries
            }
            (
                Value::MappingProxyObject {
                    mapping: left_mapping,
                },
                Value::MappingProxyObject {
                    mapping: right_mapping,
                },
            ) => left_mapping == right_mapping,
            (Value::ChainMap { maps: left_maps }, Value::ChainMap { maps: right_maps }) => {
                left_maps == right_maps
            }
            (Value::Counter { entries: left }, Value::Counter { entries: right }) => {
                dict_entries_equal(&left.borrow().entries, &right.borrow().entries)
            }
            (Value::Counter { entries: left }, Value::Dict(right))
            | (Value::Dict(right), Value::Counter { entries: left })
            | (Value::Counter { entries: left }, Value::OrderedDict(right))
            | (Value::OrderedDict(right), Value::Counter { entries: left }) => {
                dict_entries_equal(&left.borrow().entries, &right.borrow().entries)
            }
            (
                Value::Slice {
                    start: left_start,
                    stop: left_stop,
                    step: left_step,
                },
                Value::Slice {
                    start: right_start,
                    stop: right_stop,
                    step: right_step,
                },
            ) => left_start == right_start && left_stop == right_stop && left_step == right_step,
            (
                Value::Range {
                    start: left_start,
                    stop: left_stop,
                    step: left_step,
                },
                Value::Range {
                    start: right_start,
                    stop: right_stop,
                    step: right_step,
                },
            ) => left_start == right_start && left_stop == right_stop && left_step == right_step,
            (
                Value::RangeIterator {
                    current: left_current,
                    stop: left_stop,
                    step: left_step,
                },
                Value::RangeIterator {
                    current: right_current,
                    stop: right_stop,
                    step: right_step,
                },
            ) => {
                left_current == right_current && left_stop == right_stop && left_step == right_step
            }
            (
                Value::ListIterator {
                    items: left_items,
                    index: left_index,
                    exhausted: left_exhausted,
                },
                Value::ListIterator {
                    items: right_items,
                    index: right_index,
                    exhausted: right_exhausted,
                },
            ) => {
                *left_items.borrow() == *right_items.borrow()
                    && left_index == right_index
                    && left_exhausted == right_exhausted
            }
            (
                Value::TupleIterator {
                    items: left_items,
                    index: left_index,
                },
                Value::TupleIterator {
                    items: right_items,
                    index: right_index,
                },
            )
            | (
                Value::TemplateIterator {
                    items: left_items,
                    index: left_index,
                },
                Value::TemplateIterator {
                    items: right_items,
                    index: right_index,
                },
            ) => left_items == right_items && left_index == right_index,
            (
                Value::StringIterator {
                    chars: left_chars,
                    index: left_index,
                },
                Value::StringIterator {
                    chars: right_chars,
                    index: right_index,
                },
            ) => left_chars == right_chars && left_index == right_index,
            (
                Value::BytesIterator {
                    bytes: left_bytes,
                    index: left_index,
                },
                Value::BytesIterator {
                    bytes: right_bytes,
                    index: right_index,
                },
            ) => left_bytes == right_bytes && left_index == right_index,
            (
                Value::ByteArrayIterator {
                    bytes: left_bytes,
                    index: left_index,
                    exhausted: left_exhausted,
                },
                Value::ByteArrayIterator {
                    bytes: right_bytes,
                    index: right_index,
                    exhausted: right_exhausted,
                },
            ) => {
                left_bytes.borrow().as_slice() == right_bytes.borrow().as_slice()
                    && left_index == right_index
                    && left_exhausted == right_exhausted
            }
            (
                Value::SetIterator {
                    items: left_items,
                    index: left_index,
                    expected_len: left_expected_len,
                    ..
                },
                Value::SetIterator {
                    items: right_items,
                    index: right_index,
                    expected_len: right_expected_len,
                    ..
                },
            ) => {
                left_items == right_items
                    && left_index == right_index
                    && left_expected_len == right_expected_len
            }
            (
                Value::DictIterator {
                    kind: left_kind,
                    entries: left_entries,
                    index: left_index,
                    expected_len: left_expected_len,
                    expected_version: left_expected_version,
                },
                Value::DictIterator {
                    kind: right_kind,
                    entries: right_entries,
                    index: right_index,
                    expected_len: right_expected_len,
                    expected_version: right_expected_version,
                },
            ) => {
                left_kind == right_kind
                    && Rc::ptr_eq(left_entries, right_entries)
                    && left_index == right_index
                    && left_expected_len == right_expected_len
                    && left_expected_version == right_expected_version
            }
            (
                Value::ReverseIterator {
                    items: left_items,
                    index: left_index,
                },
                Value::ReverseIterator {
                    items: right_items,
                    index: right_index,
                },
            ) => left_items == right_items && left_index == right_index,
            (
                Value::DictReverseIterator {
                    kind: left_kind,
                    entries: left_entries,
                    keys: left_keys,
                    index: left_index,
                    expected_len: left_expected_len,
                    expected_version: left_expected_version,
                },
                Value::DictReverseIterator {
                    kind: right_kind,
                    entries: right_entries,
                    keys: right_keys,
                    index: right_index,
                    expected_len: right_expected_len,
                    expected_version: right_expected_version,
                },
            ) => {
                left_kind == right_kind
                    && Rc::ptr_eq(left_entries, right_entries)
                    && left_keys == right_keys
                    && left_index == right_index
                    && left_expected_len == right_expected_len
                    && left_expected_version == right_expected_version
            }
            (
                Value::EnumerateIterator {
                    iterator: left_iterator,
                    index: left_index,
                },
                Value::EnumerateIterator {
                    iterator: right_iterator,
                    index: right_index,
                },
            ) => left_iterator == right_iterator && left_index == right_index,
            (
                Value::ZipIterator {
                    iterators: left_iterators,
                    strict: left_strict,
                },
                Value::ZipIterator {
                    iterators: right_iterators,
                    strict: right_strict,
                },
            ) => left_iterators == right_iterators && left_strict == right_strict,
            (
                Value::MapIterator {
                    function: left_function,
                    iterators: left_iterators,
                    strict: left_strict,
                },
                Value::MapIterator {
                    function: right_function,
                    iterators: right_iterators,
                    strict: right_strict,
                },
            ) => {
                left_function == right_function
                    && left_iterators == right_iterators
                    && left_strict == right_strict
            }
            (
                Value::FilterIterator {
                    function: left_function,
                    iterator: left_iterator,
                },
                Value::FilterIterator {
                    function: right_function,
                    iterator: right_iterator,
                },
            ) => left_function == right_function && left_iterator == right_iterator,
            (
                Value::CallIterator {
                    callable: left_callable,
                    sentinel: left_sentinel,
                    done: left_done,
                },
                Value::CallIterator {
                    callable: right_callable,
                    sentinel: right_sentinel,
                    done: right_done,
                },
            ) => {
                left_callable == right_callable
                    && left_sentinel == right_sentinel
                    && left_done == right_done
            }
            (
                Value::SequenceIterator {
                    object: left_object,
                    index: left_index,
                },
                Value::SequenceIterator {
                    object: right_object,
                    index: right_index,
                },
            ) => left_object == right_object && left_index == right_index,
            (
                Value::SequenceReverseIterator {
                    object: left_object,
                    index: left_index,
                },
                Value::SequenceReverseIterator {
                    object: right_object,
                    index: right_index,
                },
            ) => left_object == right_object && left_index == right_index,
            (Value::Iterator(left), Value::Iterator(right)) => Rc::ptr_eq(left, right),
            (
                Value::Function {
                    identity: left_identity,
                    ..
                },
                Value::Function {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (Value::Generator(left), Value::Generator(right)) => Rc::ptr_eq(left, right),
            (Value::Coroutine(left), Value::Coroutine(right)) => Rc::ptr_eq(left, right),
            (Value::CoroutineAwait(left), Value::CoroutineAwait(right)) => Rc::ptr_eq(left, right),
            (Value::AwaitIterator(left), Value::AwaitIterator(right)) => left == right,
            (Value::AsyncGenerator(left), Value::AsyncGenerator(right)) => Rc::ptr_eq(left, right),
            (
                Value::AsyncGeneratorNext {
                    state: left_state,
                    send: left_send,
                    default: left_default,
                },
                Value::AsyncGeneratorNext {
                    state: right_state,
                    send: right_send,
                    default: right_default,
                },
            ) => {
                Rc::ptr_eq(left_state, right_state)
                    && left_send == right_send
                    && left_default == right_default
            }
            (
                Value::AsyncGeneratorThrow {
                    state: left_state,
                    exception: left_exception,
                },
                Value::AsyncGeneratorThrow {
                    state: right_state,
                    exception: right_exception,
                },
            ) => Rc::ptr_eq(left_state, right_state) && left_exception == right_exception,
            (Value::AsyncGeneratorClose(left), Value::AsyncGeneratorClose(right)) => {
                Rc::ptr_eq(left, right)
            }
            (
                Value::AsyncGeneratorAthrowMixin {
                    done: left_done, ..
                },
                Value::AsyncGeneratorAthrowMixin {
                    done: right_done, ..
                },
            ) => Rc::ptr_eq(left_done, right_done),
            (
                Value::AsyncGeneratorAcloseMixin {
                    done: left_done, ..
                },
                Value::AsyncGeneratorAcloseMixin {
                    done: right_done, ..
                },
            ) => Rc::ptr_eq(left_done, right_done),
            (
                Value::AnextDefault {
                    awaitable: left_awaitable,
                    default: left_default,
                },
                Value::AnextDefault {
                    awaitable: right_awaitable,
                    default: right_default,
                },
            ) => left_awaitable == right_awaitable && left_default == right_default,
            (
                Value::Class {
                    attrs: left_attrs, ..
                },
                Value::Class {
                    attrs: right_attrs, ..
                },
            ) => Rc::ptr_eq(left_attrs, right_attrs),
            (Value::NamedTupleType(left), Value::NamedTupleType(right)) => {
                Rc::ptr_eq(&left.identity, &right.identity)
            }
            (
                Value::TypeParam {
                    identity: left_identity,
                    ..
                },
                Value::TypeParam {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (Value::DeferredTypeParamExpr(left), Value::DeferredTypeParamExpr(right)) => {
                Rc::ptr_eq(left, right)
            }
            (
                Value::TypeAlias {
                    name: left_name,
                    type_params: left_type_params,
                    value: left_value,
                },
                Value::TypeAlias {
                    name: right_name,
                    type_params: right_type_params,
                    value: right_value,
                },
            ) => {
                left_name == right_name
                    && left_type_params == right_type_params
                    && left_value == right_value
            }
            (
                Value::ConstEvaluator {
                    kind: left_kind,
                    target: left_target,
                },
                Value::ConstEvaluator {
                    kind: right_kind,
                    target: right_target,
                },
            ) => left_kind == right_kind && left_target == right_target,
            (
                Value::GenericAlias {
                    origin: left_origin,
                    args: left_args,
                },
                Value::GenericAlias {
                    origin: right_origin,
                    args: right_args,
                },
            ) => left_origin == right_origin && left_args == right_args,
            (Value::Unpack(left), Value::Unpack(right)) => left == right,
            (
                Value::Template {
                    strings: left_strings,
                    interpolations: left_interpolations,
                },
                Value::Template {
                    strings: right_strings,
                    interpolations: right_interpolations,
                },
            ) => left_strings == right_strings && left_interpolations == right_interpolations,
            (Value::TemplateInterpolation(left), Value::TemplateInterpolation(right)) => {
                left == right
            }
            (
                Value::Instance {
                    fields: left_fields,
                    ..
                },
                Value::Instance {
                    fields: right_fields,
                    ..
                },
            ) => Rc::ptr_eq(left_fields, right_fields),
            (
                Value::Property {
                    fget: left_fget,
                    fset: left_fset,
                    fdel: left_fdel,
                    doc: left_doc,
                },
                Value::Property {
                    fget: right_fget,
                    fset: right_fset,
                    fdel: right_fdel,
                    doc: right_doc,
                },
            ) => {
                left_fget == right_fget
                    && left_fset == right_fset
                    && left_fdel == right_fdel
                    && left_doc == right_doc
            }
            (
                Value::MemberDescriptor {
                    name: left_name,
                    owner_name: left_owner_name,
                },
                Value::MemberDescriptor {
                    name: right_name,
                    owner_name: right_owner_name,
                },
            ) => left_name == right_name && left_owner_name == right_owner_name,
            (
                Value::StaticMethod {
                    function: left_function,
                },
                Value::StaticMethod {
                    function: right_function,
                },
            )
            | (
                Value::ClassMethod {
                    function: left_function,
                },
                Value::ClassMethod {
                    function: right_function,
                },
            ) => left_function == right_function,
            (
                Value::Super {
                    identity: left_identity,
                    ..
                },
                Value::Super {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (
                Value::BoundMethod {
                    function: left_function,
                    receiver: left_receiver,
                    ..
                },
                Value::BoundMethod {
                    function: right_function,
                    receiver: right_receiver,
                    ..
                },
            ) => left_function == right_function && left_receiver == right_receiver,
            (
                Value::AstNode {
                    identity: left_identity,
                    ..
                },
                Value::AstNode {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (
                Value::Traceback {
                    identity: left_identity,
                },
                Value::Traceback {
                    identity: right_identity,
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (
                Value::Module {
                    attrs: left_attrs, ..
                },
                Value::Module {
                    attrs: right_attrs, ..
                },
            ) => Rc::ptr_eq(left_attrs, right_attrs),
            (
                Value::Exception {
                    identity: left_identity,
                    ..
                },
                Value::Exception {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (
                Value::Partial {
                    identity: left_identity,
                    ..
                },
                Value::Partial {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::OperatorAttrGetter {
                    identity: left_identity,
                    ..
                },
                Value::OperatorAttrGetter {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::OperatorItemGetter {
                    identity: left_identity,
                    ..
                },
                Value::OperatorItemGetter {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::OperatorMethodCaller {
                    identity: left_identity,
                    ..
                },
                Value::OperatorMethodCaller {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (
                Value::InspectSignature { text: left_text },
                Value::InspectSignature { text: right_text },
            ) => left_text == right_text,
            (Value::Builtin(left), Value::Builtin(right)) => left == right,
            (Value::None, Value::None) => true,
            (Value::NotImplemented, Value::NotImplemented) => true,
            (Value::Ellipsis, Value::Ellipsis) => true,
            _ => false,
        }
    }
}

fn format_tuple(items: &[Value]) -> String {
    match items {
        [] => "()".to_string(),
        [item] => format!("({},)", format_value_repr(item)),
        _ => format!("({})", format_list_items(items)),
    }
}

fn format_named_tuple(typ: &NamedTupleType, values: &[Value]) -> String {
    format_named_tuple_with_name(&typ.name, &typ.fields, values)
}

fn format_named_tuple_with_name(name: &str, fields: &[String], values: &[Value]) -> String {
    let rendered = fields
        .iter()
        .zip(values.iter())
        .map(|(field, value)| format!("{field}={}", format_value_repr(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{name}({rendered})")
}

fn format_named_tuple_subclass(class_name: &str, fields: &Scope) -> Option<String> {
    let storage = fields
        .borrow()
        .get(NAMED_TUPLE_SUBCLASS_STORAGE_FIELD)
        .cloned()?;
    let Value::NamedTuple { typ, values } = storage else {
        return None;
    };
    Some(format_named_tuple_with_name(
        class_name,
        &typ.fields,
        values.as_ref(),
    ))
}

fn format_int_subclass(fields: &Scope) -> Option<String> {
    match fields.borrow().get(INT_SUBCLASS_STORAGE_FIELD).cloned()? {
        Value::Number(value) => Some(value.to_string()),
        Value::BigInt(value) => Some(value.to_string()),
        _ => None,
    }
}

fn namedtuple_field_name(typ: &NamedTupleType, index: usize) -> &str {
    typ.fields
        .get(index)
        .map(String::as_str)
        .unwrap_or("<unknown>")
}

fn format_string_tuple(items: &[String]) -> String {
    match items {
        [] => "()".to_string(),
        [item] => format!("({},)", repr_string(item)),
        _ => format!(
            "({})",
            items
                .iter()
                .map(|item| repr_string(item))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn format_interpolation_tuple(items: &[TemplateInterpolation]) -> String {
    match items {
        [] => "()".to_string(),
        [item] => format!("({},)", format_template_interpolation(item)),
        _ => format!(
            "({})",
            items
                .iter()
                .map(format_template_interpolation)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn format_template_interpolation(interpolation: &TemplateInterpolation) -> String {
    let conversion = interpolation
        .conversion
        .as_ref()
        .map(|value| repr_string(value))
        .unwrap_or_else(|| "None".to_string());

    format!(
        "Interpolation({}, {}, {}, {})",
        format_template_interpolation_value(&interpolation.value),
        repr_string(&interpolation.expression),
        conversion,
        repr_string(&interpolation.format_spec)
    )
}

fn format_template_interpolation_value(value: &Value) -> String {
    match value {
        Value::String(value) => repr_string(value),
        value => format_value_repr(value),
    }
}

fn repr_bytes(value: &[u8]) -> String {
    let mut result = String::from("b'");
    for byte in value {
        match *byte {
            b'\\' => result.push_str("\\\\"),
            b'\'' => result.push_str("\\'"),
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            b'\t' => result.push_str("\\t"),
            0x20..=0x7e => result.push(*byte as char),
            byte => result.push_str(&format!("\\x{byte:02x}")),
        }
    }
    result.push('\'');
    result
}

fn repr_string(value: &str) -> String {
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut result = String::new();
    result.push(quote);
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            ch if ch == quote => {
                result.push('\\');
                result.push(ch);
            }
            ch => result.push(ch),
        }
    }
    result.push(quote);
    result
}

fn format_set(items: &[Value]) -> String {
    if items.is_empty() {
        "set()".to_string()
    } else {
        format!("{{{}}}", format_list_items(items))
    }
}

fn format_frozen_set(items: &[Value]) -> String {
    if items.is_empty() {
        "frozenset()".to_string()
    } else {
        format!("frozenset({})", format_set(items))
    }
}

fn format_dict_view_payload(kind: DictViewKind, entries: &DictRef) -> String {
    let values = dict_view_values(kind, entries);
    format!("[{}]", format_list_items(&values))
}

fn dict_view_type_name(kind: DictViewKind) -> &'static str {
    match kind {
        DictViewKind::Keys => "dict_keys",
        DictViewKind::Values => "dict_values",
        DictViewKind::Items => "dict_items",
    }
}

fn dict_view_is_set_like(kind: DictViewKind) -> bool {
    matches!(kind, DictViewKind::Keys | DictViewKind::Items)
}

fn format_dict(entries: &[(Value, Value)]) -> String {
    entries
        .iter()
        .map(|(key, value)| format!("{}: {}", format_value_repr(key), format_value_repr(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_counter(entries: &[(Value, Value)]) -> String {
    if entries.is_empty() {
        return "Counter()".to_string();
    }

    let mut indexed = entries.iter().enumerate().collect::<Vec<_>>();
    indexed.sort_by(
        |(left_index, (_, left_value)), (right_index, (_, right_value))| {
            counter_repr_count(right_value)
                .cmp(&counter_repr_count(left_value))
                .then_with(|| left_index.cmp(right_index))
        },
    );
    let rendered = indexed
        .into_iter()
        .map(|(_, (key, value))| {
            format!("{}: {}", format_value_repr(key), format_value_repr(value))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("Counter({{{rendered}}})")
}

fn dict_entries_equal(left: &[(Value, Value)], right: &[(Value, Value)]) -> bool {
    left.len() == right.len()
        && left.iter().all(|(left_key, left_value)| {
            right
                .iter()
                .find(|(right_key, _)| right_key == left_key)
                .is_some_and(|(_, right_value)| right_value == left_value)
        })
}

fn counter_repr_count(value: &Value) -> i128 {
    match value {
        Value::Bool(value) => bool_as_i64(*value) as i128,
        Value::Number(value) => *value as i128,
        Value::BigInt(value) => value.to_i128().unwrap_or_else(|| {
            if value.sign() == num_bigint::Sign::Minus {
                i128::MIN
            } else {
                i128::MAX
            }
        }),
        _ => 0,
    }
}

fn format_scope_dict(scope: &Scope) -> String {
    let entries = scope
        .borrow()
        .iter()
        .map(|(key, value)| (Value::String(key.clone()), value.clone()))
        .collect::<Vec<_>>();
    format_dict(&entries)
}

fn format_simple_namespace(fields: &DictRef) -> String {
    let mut active = HashSet::new();
    format_simple_namespace_inner(fields, &mut active)
}

fn format_simple_namespace_inner(fields: &DictRef, active: &mut HashSet<usize>) -> String {
    let ptr = Rc::as_ptr(fields) as usize;
    if !active.insert(ptr) {
        return "namespace(...)".to_string();
    }

    let fields = fields.borrow();
    let rendered = fields
        .iter()
        .filter_map(|(key, value)| match key {
            Value::String(name) => Some(format!(
                "{name}={}",
                format_value_repr_with_namespace_seen(value, active)
            )),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(", ");
    active.remove(&ptr);
    format!("namespace({rendered})")
}

fn sets_equal(left: &[Value], right: &[Value]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .all(|item| right.iter().any(|other| other == item))
}
