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
pub type BytesRef = Rc<Vec<u8>>;
pub type ByteArrayRef = Rc<RefCell<ByteArrayStorage>>;
pub type MemoryViewRef = Rc<RefCell<MemoryViewState>>;
pub type BytesIORef = Rc<RefCell<BytesIOState>>;
pub type TeeRef = Rc<RefCell<TeeState>>;
pub type GroupByRef = Rc<RefCell<GroupByState>>;
pub type NamedTupleTypeRef = Rc<NamedTupleType>;
pub type DeferredTypeParamExprRef = Rc<DeferredTypeParamExpr>;
pub type MockCallsRef = Rc<RefCell<Vec<Vec<Value>>>>;
pub type MockSideEffectRef = Rc<RefCell<Option<Value>>>;

pub const EXCEPTION_TRACEBACK_ATTR: &str = "\0minipython_traceback";
pub const INT_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_int_storage";
pub const INT_ENUM_MEMBER_NAME_FIELD: &str = "\0minipython_int_enum_member_name";
pub const INT_ENUM_MEMBER_VALUE_FIELD: &str = "\0minipython_int_enum_member_value";
pub const FLOAT_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_float_storage";
pub const COMPLEX_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_complex_storage";
pub const NAMED_TUPLE_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_namedtuple_storage";
pub const TUPLE_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_tuple_storage";
pub const SET_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_set_storage";
pub const FROZEN_SET_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_frozenset_storage";
pub const GENERIC_ALIAS_SUBCLASS_STORAGE_FIELD: &str = "\0minipython_genericalias_storage";

pub fn identity_string_value(value: String) -> Value {
    Value::IdentityString {
        value,
        identity: Rc::new(()),
    }
}

pub fn complex_value(real: f64, imag: f64) -> Value {
    Value::Complex {
        real,
        imag,
        identity: Rc::new(()),
    }
}

#[derive(Debug, Clone)]
pub struct NamedTupleType {
    pub name: String,
    pub fields: Vec<String>,
    pub bases: Vec<Value>,
    pub original_bases: Option<Value>,
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
    pub ndim: usize,
    pub offset: usize,
    pub len: usize,
    pub stride: isize,
    pub readonly: bool,
    pub released: bool,
}

impl Drop for MemoryViewState {
    fn drop(&mut self) {
        if !self.released {
            if let Some(bytearray) = &self.exported_bytearray {
                bytearray.borrow_mut().release_export();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BytesIOState {
    pub buffer: ByteArrayRef,
    pub position: usize,
    pub closed: bool,
    pub attrs: Scope,
}

#[derive(Debug, Clone)]
pub struct TeeState {
    pub iterator: Value,
    pub buffer: Vec<Value>,
    pub exhausted: bool,
}

#[derive(Debug, Clone)]
pub struct GroupByState {
    pub iterator: Value,
    pub key_func: Value,
    pub current_key: Option<Value>,
    pub pending_value: Option<Value>,
    pub lookahead: Option<(Value, Value)>,
    pub active_group_id: usize,
    pub exhausted: bool,
}

#[derive(Debug, Clone)]
pub struct LruCacheState {
    pub entries: Vec<(Value, Value)>,
    pub hits: usize,
    pub misses: usize,
    pub maxsize: Option<usize>,
    pub maxsize_parameter: Value,
    pub typed: bool,
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

impl BytesIOState {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer: Rc::new(RefCell::new(ByteArrayStorage::new(buffer))),
            position: 0,
            closed: false,
            attrs: Rc::new(RefCell::new(HashMap::new())),
        }
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
    pub end_line: i64,
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

pub fn bytes_value(bytes: Vec<u8>) -> Value {
    Value::Bytes(Rc::new(bytes))
}

pub fn memory_view_value(bytes: Vec<u8>, readonly: bool) -> Value {
    let len = bytes.len();
    let obj = bytes_value(bytes.clone());
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
        Value::ByteArray(bytearray) => Some(bytearray.clone()),
        _ => None,
    };
    memory_view_from_parts_with_exported_bytearray(
        bytes,
        obj,
        exported_bytearray,
        offset,
        len,
        stride,
        readonly,
        format,
    )
}

pub fn memory_view_from_parts_with_exported_bytearray(
    bytes: ByteArrayRef,
    obj: Value,
    exported_bytearray: Option<ByteArrayRef>,
    offset: usize,
    len: usize,
    stride: isize,
    readonly: bool,
    format: String,
) -> Value {
    memory_view_from_parts_with_exported_bytearray_and_ndim(
        bytes,
        obj,
        exported_bytearray,
        offset,
        len,
        stride,
        readonly,
        format,
        1,
    )
}

pub fn memory_view_from_parts_with_exported_bytearray_and_ndim(
    bytes: ByteArrayRef,
    obj: Value,
    exported_bytearray: Option<ByteArrayRef>,
    offset: usize,
    len: usize,
    stride: isize,
    readonly: bool,
    format: String,
    ndim: usize,
) -> Value {
    if let Some(bytearray) = &exported_bytearray {
        bytearray.borrow_mut().retain_export();
    }
    Value::MemoryView(Rc::new(RefCell::new(MemoryViewState {
        bytes,
        obj,
        exported_bytearray,
        hash_cache: None,
        format,
        ndim,
        offset,
        len,
        stride,
        readonly,
        released: false,
    })))
}

pub fn bytes_io_value(buffer: Vec<u8>) -> Value {
    Value::BytesIO(Rc::new(RefCell::new(BytesIOState::new(buffer))))
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
    Value::DictView {
        kind,
        entries,
        ordered: false,
        identity: Rc::new(()),
    }
}

pub fn ordered_dict_view_value(kind: DictViewKind, entries: DictRef) -> Value {
    Value::DictView {
        kind,
        entries,
        ordered: true,
        identity: Rc::new(()),
    }
}

pub fn mapping_view_value(kind: DictViewKind, mapping: Value) -> Value {
    Value::MappingView {
        kind,
        mapping: Box::new(mapping),
        identity: Rc::new(()),
    }
}

pub fn mapping_proxy_value(entries: DictRef) -> Value {
    Value::MappingProxy {
        entries,
        identity: Rc::new(()),
    }
}

pub fn frame_locals_proxy_value(locals: Scope) -> Value {
    Value::FrameLocalsProxy {
        locals,
        identity: Rc::new(()),
    }
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
    pub name_value: Value,
    pub qualname_value: Value,
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
    pub running: bool,
    pub is_iterable_coroutine: bool,
    pub first_line: usize,
    pub line_sequence: Vec<usize>,
    pub code_identity: Rc<()>,
    pub frame_fields: Option<DictRef>,
    pub yield_from: Option<Value>,
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
    pub running: bool,
    pub first_line: usize,
    pub line_sequence: Vec<usize>,
    pub code_identity: Rc<()>,
    pub frame_fields: Option<DictRef>,
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
        identity: Rc<()>,
    },
    String(String),
    IdentityString {
        value: String,
        identity: Rc<()>,
    },
    Bytes(BytesRef),
    ByteArray(ByteArrayRef),
    MemoryView(MemoryViewRef),
    BytesIO(BytesIORef),
    Bool(bool),
    List(ListRef),
    Tuple(TupleRef),
    Set(SetRef),
    FrozenSet(FrozenSetRef),
    Dict(DictRef),
    OrderedDict(DictRef),
    DefaultDict {
        entries: DictRef,
        default_factory: Rc<RefCell<Value>>,
    },
    ScopeDict(Scope),
    DictView {
        kind: DictViewKind,
        entries: DictRef,
        ordered: bool,
        identity: Rc<()>,
    },
    MappingView {
        kind: DictViewKind,
        mapping: Box<Value>,
        identity: Rc<()>,
    },
    MappingProxy {
        entries: DictRef,
        identity: Rc<()>,
    },
    MappingProxyObject {
        mapping: Box<Value>,
        identity: Rc<()>,
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
    Deque {
        data: ListRef,
        maxlen: Option<usize>,
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
        public_filename: Box<Value>,
        instructions: Vec<Instruction>,
        line_spans: Vec<CodeLineSpan>,
        varnames: Vec<String>,
        consts: Vec<Value>,
        flags: i64,
        freevars: Vec<String>,
        positional_only: Vec<String>,
        params: Vec<String>,
        vararg: Option<String>,
        keyword_only: Vec<String>,
        kwarg: Option<String>,
        name: String,
        identity: Rc<()>,
    },
    Cell {
        name: String,
        scope: Scope,
        identity: Rc<()>,
    },
    DeferredTypeParamExpr(DeferredTypeParamExprRef),
    Frame {
        fields: DictRef,
    },
    FrameLocalsProxy {
        locals: Scope,
        identity: Rc<()>,
    },
    Traceback {
        identity: Rc<()>,
    },
    Slice {
        start: Option<Box<Value>>,
        stop: Option<Box<Value>>,
        step: Option<Box<Value>>,
        identity: Rc<()>,
    },
    Range {
        start: BigInt,
        stop: BigInt,
        step: BigInt,
        identity: Rc<()>,
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
    ItertoolsCount {
        current: Box<Value>,
        step: Box<Value>,
    },
    ItertoolsRepeat {
        value: Box<Value>,
        remaining: Option<BigInt>,
    },
    ItertoolsCycle {
        iterator: Box<Value>,
        saved: Vec<Value>,
        index: usize,
        exhausted: bool,
    },
    ItertoolsChain {
        iterators: Vec<Value>,
        index: usize,
    },
    ItertoolsChainFromIterable {
        iterator: Box<Value>,
        current: Option<Box<Value>>,
    },
    ItertoolsAccumulate {
        iterator: Box<Value>,
        function: Box<Value>,
        total: Option<Box<Value>>,
        initial: Option<Box<Value>>,
    },
    ItertoolsCompress {
        data: Box<Value>,
        selectors: Box<Value>,
    },
    ItertoolsFilterFalse {
        function: Box<Value>,
        iterator: Box<Value>,
    },
    ItertoolsTakewhile {
        predicate: Box<Value>,
        iterator: Box<Value>,
        done: bool,
    },
    ItertoolsDropwhile {
        predicate: Box<Value>,
        iterator: Box<Value>,
        dropping: bool,
    },
    ItertoolsStarmap {
        function: Box<Value>,
        iterator: Box<Value>,
    },
    ItertoolsZipLongest {
        iterators: Vec<Option<Value>>,
        fillvalue: Box<Value>,
    },
    ItertoolsIslice {
        iterator: Box<Value>,
        position: i64,
        next_position: i64,
        stop: Option<i64>,
        step: i64,
    },
    ItertoolsPairwise {
        iterator: Box<Value>,
        previous: Option<Box<Value>>,
        initialized: bool,
    },
    ItertoolsProduct {
        pools: Vec<Vec<Value>>,
        indices: Vec<usize>,
        first: bool,
        done: bool,
    },
    ItertoolsCombinations {
        pool: Vec<Value>,
        indices: Vec<usize>,
        r: usize,
        first: bool,
        done: bool,
    },
    ItertoolsCombinationsWithReplacement {
        pool: Vec<Value>,
        indices: Vec<usize>,
        r: usize,
        first: bool,
        done: bool,
    },
    ItertoolsPermutations {
        pool: Vec<Value>,
        indices: Vec<usize>,
        cycles: Vec<usize>,
        r: usize,
        first: bool,
        done: bool,
    },
    ItertoolsTee {
        state: TeeRef,
        index: usize,
    },
    ItertoolsBatched {
        iterator: Box<Value>,
        n: usize,
        strict: bool,
    },
    ItertoolsGroupBy {
        state: GroupByRef,
    },
    ItertoolsGroup {
        state: GroupByRef,
        key: Box<Value>,
        group_id: usize,
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
        locals_are_globals: bool,
        first_line: usize,
        line_sequence: Vec<usize>,
        position_columns: Vec<Option<(usize, usize)>>,
        identity: Rc<()>,
        owner_class: Option<Box<Value>>,
    },
    TypesCoroutineFunction {
        function: Box<Value>,
        identity: Rc<()>,
    },
    MagicMock {
        attrs: Scope,
        methods: Scope,
        calls: MockCallsRef,
        side_effect: MockSideEffectRef,
        identity: Rc<()>,
    },
    MockMethod {
        name: String,
        calls: MockCallsRef,
        side_effect: MockSideEffectRef,
        identity: Rc<()>,
    },
    WeakRef {
        target: Box<Value>,
        callback: Option<Box<Value>>,
        identity: Rc<()>,
    },
    WeakProxy {
        target: Box<Value>,
        callable: bool,
        identity: Rc<()>,
    },
    Generator(Rc<RefCell<GeneratorState>>),
    GeneratorWrapper {
        wrapped: Box<Value>,
        exact_generator: bool,
        identity: Rc<()>,
    },
    Coroutine(Rc<RefCell<CoroutineState>>),
    CoroutineAwait(Rc<RefCell<CoroutineState>>),
    AwaitIterator(Box<Value>),
    AsyncGenerator(Rc<RefCell<GeneratorState>>),
    AsyncGeneratorNext {
        state: Rc<RefCell<GeneratorState>>,
        send: Box<Value>,
        default: Option<Box<Value>>,
        identity: Rc<()>,
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
    ParamSpecAccess {
        name: String,
        origin: Box<Value>,
        is_kwargs: bool,
        identity: Rc<()>,
    },
    TypeAlias {
        name: String,
        type_params: Vec<Value>,
        value: Box<Value>,
    },
    ForwardRef {
        arg: String,
    },
    NewType {
        name: String,
        module: String,
        supertype: Box<Value>,
        identity: Rc<()>,
    },
    ConstEvaluator {
        kind: ConstEvaluatorKind,
        target: Box<Value>,
    },
    GenericAlias {
        origin: Box<Value>,
        args: Vec<Value>,
        union_unhashable_count: usize,
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
        doc: Rc<RefCell<Option<Value>>>,
        doc_from_getter: bool,
        name: Rc<RefCell<Option<Value>>>,
        identity: Rc<()>,
    },
    MemberDescriptor {
        name: String,
        owner_name: String,
        identity: Rc<()>,
    },
    StaticMethod {
        function: Box<Value>,
        identity: Rc<()>,
    },
    ClassMethod {
        function: Box<Value>,
        identity: Rc<()>,
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
        keywords: DictRef,
        attrs: Scope,
        identity: Rc<()>,
    },
    PartialMethod {
        function: Box<Value>,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
        attrs: Scope,
        identity: Rc<()>,
    },
    PartialMethodCall {
        function: Box<Value>,
        receiver: Option<Box<Value>>,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
        expects_self_arg: bool,
        identity: Rc<()>,
    },
    LruCacheWrapper {
        function: Box<Value>,
        state: Rc<RefCell<LruCacheState>>,
        attrs: Scope,
        identity: Rc<()>,
    },
    SingleDispatch {
        function: Box<Value>,
        registry: Rc<RefCell<Vec<(Value, Value)>>>,
        attrs: Scope,
        identity: Rc<()>,
    },
    SingleDispatchRegister {
        dispatcher: Box<Value>,
        cls: Box<Value>,
        identity: Rc<()>,
    },
    SingleDispatchMethod {
        dispatcher: Box<Value>,
        func: Box<Value>,
        attrs: Scope,
        identity: Rc<()>,
    },
    SingleDispatchMethodCallable {
        descriptor: Box<Value>,
        receiver: Option<Box<Value>>,
        owner: Box<Value>,
        dispatch_arg_index: usize,
        identity: Rc<()>,
    },
    CachedProperty {
        function: Box<Value>,
        attrname: Rc<RefCell<Option<String>>>,
        attrs: Scope,
        identity: Rc<()>,
    },
    CmpToKey {
        comparator: Box<Value>,
        identity: Rc<()>,
    },
    CmpToKeyObject {
        comparator: Box<Value>,
        object: Rc<RefCell<Value>>,
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
            Value::Complex { real, imag, .. } => write!(f, "{}", format_complex(*real, *imag)),
            Value::String(value) | Value::IdentityString { value, .. } => write!(f, "{value}"),
            Value::Bytes(value) => write!(f, "{}", repr_bytes(value)),
            Value::ByteArray(value) => write!(f, "{}", repr_bytearray(&value.borrow())),
            Value::MemoryView(view) if view.borrow().released => {
                write!(f, "<released memory at 0x0>")
            }
            Value::MemoryView(_) => write!(f, "<memory at 0x0>"),
            Value::BytesIO(_) => write!(f, "<_io.BytesIO object at 0x0>"),
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
            Value::OrderedDict(entries) => write!(f, "{}", format_ordered_dict(&entries.borrow())),
            Value::DefaultDict {
                entries,
                default_factory,
            } => write!(
                f,
                "defaultdict({}, {{{}}})",
                format_value_repr(&default_factory.borrow()),
                format_dict(&entries.borrow())
            ),
            Value::ScopeDict(scope) => write!(f, "{{{}}}", format_scope_dict(scope)),
            Value::DictView {
                kind,
                entries,
                ordered,
                ..
            } => write!(
                f,
                "{}({})",
                dict_view_type_name(*kind, *ordered),
                format_dict_view_payload(*kind, entries)
            ),
            Value::MappingView { kind, mapping, .. } => {
                write!(f, "{}({mapping})", dict_view_type_name(*kind, false))
            }
            Value::MappingProxy { entries, .. } => {
                write!(f, "mappingproxy({{{}}})", format_dict(&entries.borrow()))
            }
            Value::MappingProxyObject { mapping, .. } => write!(f, "mappingproxy({mapping})"),
            Value::ChainMap { maps } => {
                let rendered = maps
                    .iter()
                    .map(format_value_repr)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "ChainMap({rendered})")
            }
            Value::Counter { entries } => write!(f, "{}", format_counter(&entries.borrow())),
            Value::UserList { data, .. } => {
                let data = data.borrow();
                write!(f, "[{}]", format_list_items(&data))
            }
            Value::Deque { data, maxlen } => {
                let data = data.borrow();
                if let Some(maxlen) = maxlen {
                    write!(f, "deque([{}], maxlen={maxlen})", format_list_items(&data))
                } else {
                    write!(f, "deque([{}])", format_list_items(&data))
                }
            }
            Value::UserDict { data, .. } => write!(f, "{{{}}}", format_dict(&data.borrow())),
            Value::NamedTupleType(typ) => {
                write!(f, "<class '{}'>", named_tuple_display_name(typ))
            }
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
            Value::Frame { .. } => write!(f, "<frame object>"),
            Value::FrameLocalsProxy { .. } => write!(f, "<frame locals proxy object>"),
            Value::Traceback { .. } => write!(f, "<traceback object>"),
            Value::Range {
                start, stop, step, ..
            } if step == &BigInt::from(1) => {
                write!(f, "range({start}, {stop})")
            }
            Value::Range {
                start, stop, step, ..
            } => write!(f, "range({start}, {stop}, {step})"),
            Value::Slice {
                start, stop, step, ..
            } => write!(
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
            Value::ItertoolsCount { .. } => write!(f, "count(...)"),
            Value::ItertoolsRepeat { .. } => write!(f, "repeat(...)"),
            Value::ItertoolsCycle { .. } => write!(f, "<itertools.cycle object>"),
            Value::ItertoolsChain { .. } => write!(f, "<itertools.chain object>"),
            Value::ItertoolsChainFromIterable { .. } => {
                write!(f, "<itertools.chain object>")
            }
            Value::ItertoolsAccumulate { .. } => write!(f, "<itertools.accumulate object>"),
            Value::ItertoolsCompress { .. } => write!(f, "<itertools.compress object>"),
            Value::ItertoolsFilterFalse { .. } => write!(f, "<itertools.filterfalse object>"),
            Value::ItertoolsTakewhile { .. } => write!(f, "<itertools.takewhile object>"),
            Value::ItertoolsDropwhile { .. } => write!(f, "<itertools.dropwhile object>"),
            Value::ItertoolsStarmap { .. } => write!(f, "<itertools.starmap object>"),
            Value::ItertoolsZipLongest { .. } => write!(f, "<itertools.zip_longest object>"),
            Value::ItertoolsIslice { .. } => write!(f, "<itertools.islice object>"),
            Value::ItertoolsPairwise { .. } => write!(f, "<itertools.pairwise object>"),
            Value::ItertoolsProduct { .. } => write!(f, "<itertools.product object>"),
            Value::ItertoolsCombinations { .. } => {
                write!(f, "<itertools.combinations object>")
            }
            Value::ItertoolsCombinationsWithReplacement { .. } => {
                write!(f, "<itertools.combinations_with_replacement object>")
            }
            Value::ItertoolsPermutations { .. } => {
                write!(f, "<itertools.permutations object>")
            }
            Value::ItertoolsTee { .. } => write!(f, "<itertools._tee object>"),
            Value::ItertoolsBatched { .. } => write!(f, "<itertools.batched object>"),
            Value::ItertoolsGroupBy { .. } => write!(f, "<itertools.groupby object>"),
            Value::ItertoolsGroup { .. } => write!(f, "<itertools._grouper object>"),
            Value::CallIterator { .. } => write!(f, "<callable_iterator object>"),
            Value::SequenceIterator { .. } => write!(f, "<iterator>"),
            Value::SequenceReverseIterator { .. } => write!(f, "<reversed object>"),
            Value::Iterator(_) => write!(f, "<iterator>"),
            Value::Function { name, .. } => write!(f, "<function {name}>"),
            Value::TypesCoroutineFunction { function, .. } => write!(f, "{function}"),
            Value::MagicMock { .. } => write!(f, "<MagicMock object>"),
            Value::MockMethod { name, .. } => write!(f, "<MagicMock name='{name}'>"),
            Value::WeakRef {
                target, identity, ..
            } => write!(f, "{}", format_weakref_repr(target, identity)),
            Value::WeakProxy {
                target, identity, ..
            } => write!(f, "{}", format_weakproxy_repr(target, identity)),
            Value::Generator(state) => write!(f, "<generator object {}>", state.borrow().name),
            Value::GeneratorWrapper { .. } => write!(f, "<types._GeneratorWrapper object>"),
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
            Value::TypeParam { kind, name, .. } if kind == "TypeVar" || kind == "ParamSpec" => {
                write!(f, "~{name}")
            }
            Value::TypeParam { name, .. } => write!(f, "{name}"),
            Value::ParamSpecAccess { name, .. } => write!(f, "{name}"),
            Value::TypeAlias { name, .. } => write!(f, "<type alias {name}>"),
            Value::ForwardRef { arg } => write!(f, "ForwardRef({})", repr_string(arg)),
            Value::NewType { name, module, .. } => {
                write!(f, "{}", format_new_type_name(module, name))
            }
            Value::ConstEvaluator { kind, target } => {
                write!(f, "{}", format_const_evaluator(*kind, target))
            }
            Value::GenericAlias { origin, args, .. } if is_union_origin(origin) => {
                write!(f, "{}", format_union_args(args))
            }
            Value::GenericAlias { origin, args, .. } => {
                write!(f, "{}", format_generic_alias(origin, args))
            }
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
                } else if let Some(rendered) = format_float_subclass(fields) {
                    write!(f, "{rendered}")
                } else if let Some(rendered) = format_complex_subclass(fields) {
                    write!(f, "{rendered}")
                } else if let Some(rendered) = format_named_tuple_subclass(class_name, fields) {
                    write!(f, "{rendered}")
                } else if let Some(rendered) = format_tuple_subclass(fields) {
                    write!(f, "{rendered}")
                } else if let Some(rendered) = format_set_subclass(class_name, fields) {
                    write!(f, "{rendered}")
                } else if let Some(rendered) = format_frozen_set_subclass(class_name, fields) {
                    write!(f, "{rendered}")
                } else if let Some(rendered) = format_generic_alias_subclass(fields) {
                    write!(f, "{rendered}")
                } else {
                    write!(f, "<{class_name} object>")
                }
            }
            Value::Property { .. } => write!(f, "<property object>"),
            Value::MemberDescriptor {
                name, owner_name, ..
            } => {
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
            Value::Partial {
                function,
                args,
                keywords,
                ..
            } => write!(f, "{}", format_partial(function, args, keywords)),
            Value::PartialMethod {
                function,
                args,
                keywords,
                ..
            } => write!(f, "{}", format_partialmethod(function, args, keywords)),
            Value::PartialMethodCall {
                function,
                receiver,
                args,
                keywords,
                expects_self_arg,
                ..
            } if *expects_self_arg => write!(f, "{}", format_partialmethod_unbound_method()),
            Value::PartialMethodCall {
                function,
                receiver,
                args,
                keywords,
                ..
            } => write!(
                f,
                "{}",
                format_partialmethod_call(function, receiver.as_deref(), args, keywords)
            ),
            Value::LruCacheWrapper { identity, .. } => {
                write!(f, "{}", format_lru_cache_wrapper(identity))
            }
            Value::SingleDispatch {
                function,
                attrs,
                identity,
                ..
            } => write!(f, "{}", format_singledispatch(function, attrs, identity)),
            Value::SingleDispatchRegister { .. } => {
                write!(f, "<function singledispatch register>")
            }
            descriptor @ Value::SingleDispatchMethod { .. } => {
                write!(f, "{}", format_singledispatchmethod(descriptor))
            }
            Value::SingleDispatchMethodCallable {
                descriptor,
                receiver,
                ..
            } => write!(
                f,
                "{}",
                format_singledispatchmethod_callable(descriptor, receiver.as_deref())
            ),
            Value::CachedProperty { identity, .. } => {
                write!(f, "{}", format_cached_property(identity))
            }
            Value::CmpToKey { identity, .. } | Value::CmpToKeyObject { identity, .. } => {
                write!(f, "{}", format_cmp_to_key(identity))
            }
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
            Value::Builtin(name) if is_typing_special_form_name(name) => {
                write!(f, "{name}")
            }
            Value::Builtin(name) if is_deque_maxlen_getset_descriptor(name) => {
                write!(f, "<attribute 'maxlen' of 'collections.deque' objects>")
            }
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

fn format_partial(function: &Value, args: &[Value], keywords: &DictRef) -> String {
    let keyword_entries = keywords.borrow();
    let mut parts = Vec::with_capacity(1 + args.len() + keyword_entries.entries.len());
    parts.push(format_value_repr(function));
    parts.extend(args.iter().map(format_value_repr));
    parts.extend(keyword_entries.entries.iter().map(|(key, value)| {
        let key = match key {
            Value::String(name) | Value::IdentityString { value: name, .. } => name.clone(),
            value => format_value_repr(value),
        };
        format!("{key}={}", format_value_repr(value))
    }));
    format!("functools.partial({})", parts.join(", "))
}

fn format_partialmethod(function: &Value, args: &[Value], keywords: &[(String, Value)]) -> String {
    let mut parts = Vec::with_capacity(1 + args.len() + keywords.len());
    parts.push(format_value_repr(function));
    parts.extend(args.iter().map(format_value_repr));
    parts.extend(
        keywords
            .iter()
            .map(|(name, value)| format!("{name}={}", format_value_repr(value))),
    );
    format!("functools.partialmethod({})", parts.join(", "))
}

fn format_partialmethod_call(
    function: &Value,
    receiver: Option<&Value>,
    args: &[Value],
    keywords: &[(String, Value)],
) -> String {
    let function = match receiver {
        Some(receiver) => format_bound_method(function, receiver),
        None => format_value_repr(function),
    };
    let mut parts = Vec::with_capacity(1 + args.len() + keywords.len());
    parts.push(function);
    parts.extend(args.iter().map(format_value_repr));
    parts.extend(
        keywords
            .iter()
            .map(|(name, value)| format!("{name}={}", format_value_repr(value))),
    );
    format!("functools.partial({})", parts.join(", "))
}

fn format_partialmethod_unbound_method() -> &'static str {
    "<function partialmethod._make_unbound_method.<locals>._method>"
}

fn format_lru_cache_wrapper(identity: &Rc<()>) -> String {
    format!(
        "<functools._lru_cache_wrapper object at 0x{:x}>",
        Rc::as_ptr(identity) as usize
    )
}

fn format_singledispatch(function: &Value, attrs: &Scope, identity: &Rc<()>) -> String {
    let fallback = match function {
        Value::Function { name, .. } => name.as_str(),
        _ => "singledispatch wrapper",
    };
    let name = function_like_name_from_attrs(attrs, fallback);
    format_function_object_repr(&name, identity)
}

fn singledispatchmethod_display_name(descriptor: &Value) -> String {
    let Value::SingleDispatchMethod {
        dispatcher, func, ..
    } = descriptor
    else {
        return "singledispatchmethod".to_string();
    };
    if let Value::SingleDispatch {
        function, attrs, ..
    } = dispatcher.as_ref()
    {
        let fallback = match function.as_ref() {
            Value::Function { name, .. } => name.as_str(),
            _ => "singledispatchmethod",
        };
        return function_like_name_from_attrs(attrs, fallback);
    }
    match func.as_ref() {
        Value::Function { name, .. } => name.clone(),
        _ => "singledispatchmethod".to_string(),
    }
}

fn format_singledispatchmethod(descriptor: &Value) -> String {
    format!(
        "<single dispatch method descriptor {}>",
        singledispatchmethod_display_name(descriptor)
    )
}

fn format_cached_property(identity: &Rc<()>) -> String {
    format!(
        "<functools.cached_property object at 0x{:x}>",
        Rc::as_ptr(identity) as usize
    )
}

fn format_cmp_to_key(identity: &Rc<()>) -> String {
    format!(
        "<functools.KeyWrapper object at 0x{:x}>",
        Rc::as_ptr(identity) as usize
    )
}

fn format_singledispatchmethod_callable(descriptor: &Value, receiver: Option<&Value>) -> String {
    let name = singledispatchmethod_display_name(descriptor);
    match receiver {
        Some(receiver) => format!(
            "<bound single dispatch method {name} of {}>",
            format_value_repr(receiver)
        ),
        None => format!("<single dispatch method {name}>"),
    }
}

fn function_like_name_from_attrs(attrs: &Scope, fallback: &str) -> String {
    let attrs = attrs.borrow();
    for name in ["__qualname__", "__name__"] {
        match attrs.get(name) {
            Some(Value::String(value)) | Some(Value::IdentityString { value, .. }) => {
                return value.clone();
            }
            _ => {}
        }
    }
    fallback.to_string()
}

fn format_function_object_repr(name: &str, identity: &Rc<()>) -> String {
    format!("<function {name} at 0x{:x}>", Rc::as_ptr(identity) as usize)
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
        Value::String(value) | Value::IdentityString { value, .. } => repr_string(value),
        Value::Bytes(value) => repr_bytes(value),
        Value::ByteArray(value) => repr_bytearray(&value.borrow()),
        Value::MemoryView(view) if view.borrow().released => "<released memory at 0x0>".to_string(),
        Value::MemoryView(_) => "<memory at 0x0>".to_string(),
        Value::BytesIO(_) => "<_io.BytesIO object at 0x0>".to_string(),
        Value::List(items) => {
            let items = items.borrow();
            format!("[{}]", format_list_items(&items))
        }
        Value::Tuple(items) => format_tuple(items),
        Value::Set(items) => format_set(&items.borrow()),
        Value::FrozenSet(items) => format_frozen_set(items),
        Value::Dict(entries) => format!("{{{}}}", format_dict(&entries.borrow())),
        Value::OrderedDict(entries) => format_ordered_dict(&entries.borrow()),
        Value::DefaultDict {
            entries,
            default_factory,
        } => format!(
            "defaultdict({}, {{{}}})",
            format_value_repr(&default_factory.borrow()),
            format_dict(&entries.borrow())
        ),
        Value::ScopeDict(scope) => format!("{{{}}}", format_scope_dict(scope)),
        Value::DictView {
            kind,
            entries,
            ordered,
            ..
        } => {
            format!(
                "{}({})",
                dict_view_type_name(*kind, *ordered),
                format_dict_view_payload(*kind, entries)
            )
        }
        Value::MappingView { kind, mapping, .. } => {
            format!("{}({mapping})", dict_view_type_name(*kind, false))
        }
        Value::MappingProxy { entries, .. } => {
            format!("mappingproxy({{{}}})", format_dict(&entries.borrow()))
        }
        Value::MappingProxyObject { mapping, .. } => format!("mappingproxy({mapping})"),
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
        Value::Deque { data, maxlen } => {
            let data = data.borrow();
            if let Some(maxlen) = maxlen {
                format!("deque([{}], maxlen={maxlen})", format_list_items(&data))
            } else {
                format!("deque([{}])", format_list_items(&data))
            }
        }
        Value::UserDict { data, .. } => format!("{{{}}}", format_dict(&data.borrow())),
        Value::SimpleNamespace { fields } => format_simple_namespace(fields),
        Value::PicklePayload(_) => "<pickle payload>".to_string(),
        Value::AstNode { kind, .. } => format!("<ast.{kind} object>"),
        Value::CodeObject { filename, .. } => {
            format!("<code object <module>, file \"{filename}\", line 1>")
        }
        Value::Cell { .. } => "<cell object>".to_string(),
        Value::DeferredTypeParamExpr(_) => "<deferred type parameter expression>".to_string(),
        Value::Frame { .. } => "<frame object>".to_string(),
        Value::FrameLocalsProxy { .. } => "<frame locals proxy object>".to_string(),
        Value::Traceback { .. } => "<traceback object>".to_string(),
        Value::Function { name, .. } => format!("<function {name}>"),
        Value::TypesCoroutineFunction { function, .. } => format_value_repr(function),
        Value::MagicMock { .. } => "<MagicMock object>".to_string(),
        Value::MockMethod { name, .. } => format!("<MagicMock name='{name}'>"),
        Value::WeakRef {
            target, identity, ..
        } => format_weakref_repr(target, identity),
        Value::WeakProxy {
            target, identity, ..
        } => format_weakproxy_repr(target, identity),
        Value::Generator(state) => format!("<generator object {}>", state.borrow().name),
        Value::Iterator(state) => {
            let iterator = state.borrow();
            format_iterator_repr(&iterator).unwrap_or_else(|| "<iterator>".to_string())
        }
        Value::GeneratorWrapper { .. } => "<types._GeneratorWrapper object>".to_string(),
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
        Value::ParamSpecAccess { name, .. } => name.clone(),
        Value::TypeAlias { name, .. } => format!("<type alias {name}>"),
        Value::ForwardRef { arg } => format!("ForwardRef({})", repr_string(arg)),
        Value::Builtin(name) if is_typing_special_form_name(name) => name.clone(),
        Value::NewType { name, module, .. } => format_new_type_name(module, name),
        Value::ConstEvaluator { kind, target } => format_const_evaluator(*kind, target),
        Value::GenericAlias { origin, args, .. } if is_union_origin(origin) => {
            format_union_args(args)
        }
        Value::GenericAlias { origin, args, .. } => format_generic_alias(origin, args),
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
        } => format_int_enum_member_repr(class_name, fields)
            .or_else(|| format_int_subclass(fields))
            .or_else(|| format_float_subclass(fields))
            .or_else(|| format_named_tuple_subclass(class_name, fields))
            .or_else(|| format_tuple_subclass(fields))
            .or_else(|| format_set_subclass(class_name, fields))
            .or_else(|| format_frozen_set_subclass(class_name, fields))
            .or_else(|| format_generic_alias_subclass(fields))
            .unwrap_or_else(|| format!("<{class_name} object>")),
        Value::Property { .. } => "<property object>".to_string(),
        Value::NamedTupleFieldDescriptor { typ, index } => {
            let field = namedtuple_field_name(typ, *index);
            format!("<namedtuple field '{field}' of '{}'>", typ.name)
        }
        Value::Builtin(name) if is_deque_maxlen_getset_descriptor(name) => {
            "<attribute 'maxlen' of 'collections.deque' objects>".to_string()
        }
        Value::MemberDescriptor {
            name, owner_name, ..
        } => {
            format!("<member '{name}' of '{owner_name}' objects>")
        }
        Value::StaticMethod { .. } => "<staticmethod object>".to_string(),
        Value::ClassMethod { .. } => "<classmethod object>".to_string(),
        Value::Super { .. } => "<super object>".to_string(),
        Value::BoundMethod {
            function, receiver, ..
        } => format_bound_method(function, receiver),
        Value::Partial {
            function,
            args,
            keywords,
            ..
        } => format_partial(function, args, keywords),
        Value::PartialMethod {
            function,
            args,
            keywords,
            ..
        } => format_partialmethod(function, args, keywords),
        Value::PartialMethodCall {
            function,
            receiver,
            args,
            keywords,
            expects_self_arg,
            ..
        } if *expects_self_arg => format_partialmethod_unbound_method().to_string(),
        Value::PartialMethodCall {
            function,
            receiver,
            args,
            keywords,
            ..
        } => format_partialmethod_call(function, receiver.as_deref(), args, keywords),
        Value::LruCacheWrapper { identity, .. } => format_lru_cache_wrapper(identity),
        Value::SingleDispatch {
            function,
            attrs,
            identity,
            ..
        } => format_singledispatch(function, attrs, identity),
        Value::SingleDispatchRegister { .. } => "<function singledispatch register>".to_string(),
        descriptor @ Value::SingleDispatchMethod { .. } => format_singledispatchmethod(descriptor),
        Value::SingleDispatchMethodCallable {
            descriptor,
            receiver,
            ..
        } => format_singledispatchmethod_callable(descriptor, receiver.as_deref()),
        Value::CachedProperty { identity, .. } => format_cached_property(identity),
        Value::CmpToKey { identity, .. } | Value::CmpToKeyObject { identity, .. } => {
            format_cmp_to_key(identity)
        }
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

pub(crate) fn format_iterator_repr(iterator: &Value) -> Option<String> {
    match iterator {
        Value::ItertoolsCount { current, step } => {
            let current = format_value_repr(current);
            if itertools_count_step_is_default(step) {
                Some(format!("count({current})"))
            } else {
                Some(format!("count({current}, {})", format_value_repr(step)))
            }
        }
        Value::ItertoolsRepeat { value, remaining } => {
            let value = format_value_repr(value);
            match remaining {
                Some(remaining) => Some(format!("repeat({value}, {remaining})")),
                None => Some(format!("repeat({value})")),
            }
        }
        Value::ItertoolsCycle { .. } => Some(itertools_object_repr("cycle")),
        Value::ItertoolsChain { .. } | Value::ItertoolsChainFromIterable { .. } => {
            Some(itertools_object_repr("chain"))
        }
        Value::ItertoolsAccumulate { .. } => Some(itertools_object_repr("accumulate")),
        Value::ItertoolsCompress { .. } => Some(itertools_object_repr("compress")),
        Value::ItertoolsFilterFalse { .. } => Some(itertools_object_repr("filterfalse")),
        Value::ItertoolsTakewhile { .. } => Some(itertools_object_repr("takewhile")),
        Value::ItertoolsDropwhile { .. } => Some(itertools_object_repr("dropwhile")),
        Value::ItertoolsStarmap { .. } => Some(itertools_object_repr("starmap")),
        Value::ItertoolsZipLongest { .. } => Some(itertools_object_repr("zip_longest")),
        Value::ItertoolsIslice { .. } => Some(itertools_object_repr("islice")),
        Value::ItertoolsPairwise { .. } => Some(itertools_object_repr("pairwise")),
        Value::ItertoolsProduct { .. } => Some(itertools_object_repr("product")),
        Value::ItertoolsCombinations { .. } => Some(itertools_object_repr("combinations")),
        Value::ItertoolsCombinationsWithReplacement { .. } => {
            Some(itertools_object_repr("combinations_with_replacement"))
        }
        Value::ItertoolsPermutations { .. } => Some(itertools_object_repr("permutations")),
        Value::ItertoolsTee { .. } => Some(itertools_object_repr("_tee")),
        Value::ItertoolsBatched { .. } => Some(itertools_object_repr("batched")),
        Value::ItertoolsGroupBy { .. } => Some(itertools_object_repr("groupby")),
        Value::ItertoolsGroup { .. } => Some(itertools_object_repr("_grouper")),
        _ => None,
    }
}

fn itertools_count_step_is_default(step: &Value) -> bool {
    match step {
        Value::Number(value) => *value == 1,
        Value::BigInt(value) => value == &BigInt::from(1),
        _ => false,
    }
}

fn itertools_object_repr(name: &str) -> String {
    format!("<itertools.{name} object at 0x0>")
}

fn format_weakref_repr(target: &Value, identity: &Rc<()>) -> String {
    format_weak_pointer_repr("weakref", target, identity)
}

fn format_weakproxy_repr(target: &Value, identity: &Rc<()>) -> String {
    format_weak_pointer_repr("weakproxy", target, identity)
}

fn format_weak_pointer_repr(kind: &str, target: &Value, identity: &Rc<()>) -> String {
    let ref_addr = Rc::as_ptr(identity) as usize;
    let target_addr = weakref_target_address(target);
    let (target_type, suffix) = weakref_target_repr_type(target);
    let suffix = suffix
        .map(|value| format!(" ({value})"))
        .unwrap_or_default();
    format!("<{kind} at 0x{ref_addr:x}; to '{target_type}' at 0x{target_addr:x}{suffix}>")
}

fn weakref_target_repr_type(target: &Value) -> (&str, Option<String>) {
    match target {
        Value::Instance { class_name, .. } => (class_name.as_str(), None),
        Value::Class { name, .. } => ("type", Some(name.clone())),
        Value::Function { name, .. } => ("function", Some(name.clone())),
        Value::TypesCoroutineFunction { function, .. } => weakref_target_repr_type(function),
        Value::Set(_) => ("set", None),
        Value::FrozenSet(_) => ("frozenset", None),
        Value::MemoryView(_) => ("memoryview", None),
        Value::BytesIO(_) => ("_io.BytesIO", None),
        Value::Builtin(name) if weakref_builtin_type_name(name).is_some() => {
            ("type", weakref_builtin_type_name(name).map(str::to_string))
        }
        Value::Builtin(name) => ("builtin_function_or_method", Some(name.clone())),
        Value::TypeParam { kind, .. } => (kind.as_str(), None),
        Value::ParamSpecAccess { is_kwargs, .. } => {
            if *is_kwargs {
                ("ParamSpecKwargs", None)
            } else {
                ("ParamSpecArgs", None)
            }
        }
        Value::TypeAlias { .. } => ("TypeAliasType", None),
        Value::ForwardRef { .. } => ("ForwardRef", None),
        Value::NewType { .. } => ("NewType", None),
        Value::GenericAlias { .. } => ("GenericAlias", None),
        Value::MagicMock { .. } => ("MagicMock", None),
        Value::MockMethod { .. } => ("MagicMock", None),
        Value::GeneratorWrapper { .. } => ("_GeneratorWrapper", None),
        value => (weakref_fallback_target_type(value), None),
    }
}

fn weakref_builtin_type_name(name: &str) -> Option<&str> {
    match name {
        "object"
        | "type"
        | "int"
        | "bool"
        | "float"
        | "complex"
        | "str"
        | "bytes"
        | "bytearray"
        | "list"
        | "tuple"
        | "dict"
        | "set"
        | "frozenset"
        | "range"
        | "memoryview"
        | "weakref.ReferenceType"
        | "weakref.ProxyType"
        | "weakref.CallableProxyType"
        | "GenericAlias"
        | "UnionType" => Some(name.rsplit('.').next().unwrap_or(name)),
        _ => None,
    }
}

fn weakref_fallback_target_type(value: &Value) -> &'static str {
    match value {
        Value::Module { .. } => "module",
        Value::Cell { .. } => "cell",
        Value::CodeObject { .. } => "code",
        Value::Frame { .. } => "frame",
        Value::FrameLocalsProxy { .. } => "FrameLocalsProxy",
        Value::Traceback { .. } => "traceback",
        Value::Generator(_) => "generator",
        Value::Coroutine(_) => "coroutine",
        Value::AsyncGenerator(_) => "async_generator",
        _ => "object",
    }
}

fn weakref_target_address(value: &Value) -> usize {
    match value {
        Value::List(items) => Rc::as_ptr(items) as usize,
        Value::UserList { data, .. } => Rc::as_ptr(data) as usize,
        Value::Tuple(items) => Rc::as_ptr(items) as usize,
        Value::NamedTuple { values, .. } => Rc::as_ptr(values) as usize,
        Value::ByteArray(bytes) => Rc::as_ptr(bytes) as usize,
        Value::Set(items) => Rc::as_ptr(items) as usize,
        Value::FrozenSet(items) => Rc::as_ptr(items) as usize,
        Value::Dict(entries) | Value::OrderedDict(entries) => Rc::as_ptr(entries) as usize,
        Value::ScopeDict(scope) => Rc::as_ptr(scope) as usize,
        Value::UserDict { data, .. } => Rc::as_ptr(data) as usize,
        Value::SimpleNamespace { fields } => Rc::as_ptr(fields) as usize,
        Value::Class { attrs, .. } => Rc::as_ptr(attrs) as usize,
        Value::Instance { fields, .. } => Rc::as_ptr(fields) as usize,
        Value::Function { identity, .. }
        | Value::TypesCoroutineFunction { identity, .. }
        | Value::MagicMock { identity, .. }
        | Value::MockMethod { identity, .. }
        | Value::GeneratorWrapper { identity, .. }
        | Value::TypeParam { identity, .. }
        | Value::ParamSpecAccess { identity, .. } => Rc::as_ptr(identity) as usize,
        Value::Generator(state) => Rc::as_ptr(state) as usize,
        Value::Coroutine(state) | Value::CoroutineAwait(state) => Rc::as_ptr(state) as usize,
        Value::AsyncGenerator(state) | Value::AsyncGeneratorClose(state) => {
            Rc::as_ptr(state) as usize
        }
        Value::MemoryView(view) => Rc::as_ptr(view) as usize,
        Value::BytesIO(bytes_io) => Rc::as_ptr(bytes_io) as usize,
        value => value as *const Value as usize,
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
            | "io.BytesIO"
            | "list"
            | "tuple"
            | "dict"
            | "set"
            | "frozenset"
            | "range"
            | "slice"
            | "mappingproxy"
            | "dict_keys"
            | "dict_items"
            | "dict_values"
            | "odict_keys"
            | "odict_items"
            | "odict_values"
            | "ChainMap"
            | "Counter"
            | "OrderedDict"
            | "defaultdict"
            | "UserList"
            | "UserDict"
            | "UserString"
            | "SimpleNamespace"
            | "property"
            | "super"
            | "staticmethod"
            | "classmethod"
            | "PyCapsule"
            | "classmethod_descriptor"
            | "DynamicClassAttribute"
            | "FrameLocalsProxy"
            | "getset_descriptor"
            | "lazy_import"
            | "member_descriptor"
            | "method_descriptor"
            | "method-wrapper"
            | "wrapper_descriptor"
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
        || name.strip_prefix("typing.").is_some_and(|name| {
            matches!(
                name,
                "BinaryIO"
                    | "IO"
                    | "NewType"
                    | "ParamSpec"
                    | "TextIO"
                    | "TypeVar"
                    | "TypeVarTuple"
                    | "Protocol"
            )
        })
        || matches!(name, "ParamSpecArgs" | "ParamSpecKwargs")
}

fn is_typing_special_form_name(name: &str) -> bool {
    matches!(name, "typing.Any" | "typing.NoReturn" | "typing.Optional")
}

fn is_deque_maxlen_getset_descriptor(name: &str) -> bool {
    name == "deque.maxlen.getset_descriptor"
}

fn builtin_type_public_name(name: &str) -> &str {
    if name.starts_with("typing.") {
        return name;
    }
    if name == "defaultdict" {
        return "collections.defaultdict";
    }
    name.strip_prefix("ast.").unwrap_or(name)
}

fn format_slice_part(value: &Option<Box<Value>>) -> String {
    value
        .as_deref()
        .map(format_value_repr)
        .unwrap_or_else(|| "None".to_string())
}

fn format_generic_origin(origin: &Value) -> String {
    match origin {
        Value::Builtin(name) => format_generic_builtin_name(name),
        Value::Class { name, .. } | Value::TypeParam { name, .. } => name.clone(),
        Value::TypeAlias { name, .. } => name.clone(),
        value => format_value_repr(value),
    }
}

fn format_generic_alias(origin: &Value, args: &[Value]) -> String {
    format!(
        "{}[{}]",
        format_generic_origin(origin),
        args.iter()
            .map(format_generic_alias_arg)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn format_generic_alias_arg(value: &Value) -> String {
    match value {
        Value::Builtin(name) => format_generic_builtin_name(name),
        Value::Class { name, .. } | Value::TypeParam { name, .. } => name.clone(),
        Value::TypeAlias { name, .. } => name.clone(),
        Value::Unpack(value) => format!("*{}", format_generic_alias_arg(value)),
        value => format_value_repr(value),
    }
}

fn format_generic_builtin_name(name: &str) -> String {
    match name {
        "deque" => "collections.deque".to_string(),
        "OrderedDict" => "collections.OrderedDict".to_string(),
        "defaultdict" => "collections.defaultdict".to_string(),
        _ => name.to_string(),
    }
}

fn format_union_args(args: &[Value]) -> String {
    args.iter()
        .map(format_union_arg)
        .collect::<Vec<_>>()
        .join(" | ")
}

fn format_union_arg(value: &Value) -> String {
    match value {
        Value::Builtin(name) if name == "NoneType" => "None".to_string(),
        Value::Builtin(name) | Value::Class { name, .. } | Value::TypeParam { name, .. } => {
            name.clone()
        }
        Value::TypeAlias { name, .. } => name.clone(),
        Value::NewType { name, module, .. } => format_new_type_name(module, name),
        Value::GenericAlias { origin, args, .. } if is_union_origin(origin) => {
            format_union_args(args)
        }
        Value::Unpack(value) => format!("*{}", format_union_arg(value)),
        value => format_value_repr(value),
    }
}

fn format_new_type_name(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_string()
    } else {
        format!("{module}.{name}")
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
                ..
            },
            Value::Complex {
                real: right_real,
                imag: right_imag,
                ..
            },
        ) => {
            left_real.to_bits() == right_real.to_bits()
                && left_imag.to_bits() == right_imag.to_bits()
        }
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::String(left), Value::String(right))
        | (Value::String(left), Value::IdentityString { value: right, .. })
        | (Value::IdentityString { value: left, .. }, Value::String(right))
        | (Value::IdentityString { value: left, .. }, Value::IdentityString { value: right, .. }) => {
            left == right
        }
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

fn is_union_origin(value: &Value) -> bool {
    matches!(value, Value::Builtin(name) if name == "Union")
}

fn is_literal_origin(value: &Value) -> bool {
    matches!(value, Value::Builtin(name) if name == "typing.Literal")
}

pub fn generic_alias_subclass_alias(value: &Value) -> Option<Value> {
    let Value::Instance { fields, .. } = value else {
        return None;
    };
    match fields
        .borrow()
        .get(GENERIC_ALIAS_SUBCLASS_STORAGE_FIELD)
        .cloned()?
    {
        alias @ Value::GenericAlias { .. } => Some(alias),
        _ => None,
    }
}

fn literal_args_equal(left: &[Value], right: &[Value]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| literal_arg_equal(left, right))
}

fn literal_arg_equal(left: &Value, right: &Value) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right) && left == right
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if let Some(left) = generic_alias_subclass_alias(self) {
            return left == *other;
        }
        if let Some(right) = generic_alias_subclass_alias(other) {
            return *self == right;
        }

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
                    ..
                },
                Value::Complex {
                    real: right_real,
                    imag: right_imag,
                    ..
                },
            ) => left_real == right_real && left_imag == right_imag,
            (Value::Number(left), Value::Float(right)) => {
                float_equals_integer_exact(**right, &BigInt::from(*left))
            }
            (Value::Float(left), Value::Number(right)) => {
                float_equals_integer_exact(**left, &BigInt::from(*right))
            }
            (Value::BigInt(left), Value::Float(right)) => float_equals_integer_exact(**right, left),
            (Value::Float(left), Value::BigInt(right)) => float_equals_integer_exact(**left, right),
            (Value::Number(left), Value::Complex { real, imag, .. })
            | (Value::Complex { real, imag, .. }, Value::Number(left)) => {
                *imag == 0.0 && float_equals_integer_exact(*real, &BigInt::from(*left))
            }
            (Value::BigInt(left), Value::Complex { real, imag, .. })
            | (Value::Complex { real, imag, .. }, Value::BigInt(left)) => {
                *imag == 0.0 && float_equals_integer_exact(*real, left)
            }
            (Value::Float(left), Value::Complex { real, imag, .. })
            | (Value::Complex { real, imag, .. }, Value::Float(left)) => {
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
            (Value::Bool(left), Value::Complex { real, imag, .. })
            | (Value::Complex { real, imag, .. }, Value::Bool(left)) => {
                bool_as_i64(*left) as f64 == *real && *imag == 0.0
            }
            (Value::String(left), Value::String(right))
            | (Value::String(left), Value::IdentityString { value: right, .. })
            | (Value::IdentityString { value: left, .. }, Value::String(right))
            | (
                Value::IdentityString { value: left, .. },
                Value::IdentityString { value: right, .. },
            ) => left == right,
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
                memory_view_state_bytes(&left)
                    .is_some_and(|left| left.as_slice() == right.as_slice())
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
            (Value::BytesIO(left), Value::BytesIO(right)) => Rc::ptr_eq(left, right),
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
            (Value::Frame { fields: left }, Value::Frame { fields: right }) => {
                Rc::ptr_eq(left, right)
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
                Value::FrameLocalsProxy { locals: left, .. },
                Value::FrameLocalsProxy { locals: right, .. },
            ) => Rc::ptr_eq(left, right),
            (
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                    identity: left_identity,
                    ..
                },
                Value::DictView {
                    kind: right_kind,
                    entries: right_entries,
                    identity: right_identity,
                    ..
                },
            ) => {
                if dict_view_is_set_like(*left_kind) && dict_view_is_set_like(*right_kind) {
                    sets_equal(
                        &dict_view_values(*left_kind, left_entries),
                        &dict_view_values(*right_kind, right_entries),
                    )
                } else {
                    left_kind == right_kind && Rc::ptr_eq(left_identity, right_identity)
                }
            }
            (
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                    ..
                },
                Value::Set(right),
            )
            | (
                Value::Set(right),
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                    ..
                },
            ) if dict_view_is_set_like(*left_kind) => {
                sets_equal(&dict_view_values(*left_kind, left_entries), &right.borrow())
            }
            (
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                    ..
                },
                Value::FrozenSet(right),
            )
            | (
                Value::FrozenSet(right),
                Value::DictView {
                    kind: left_kind,
                    entries: left_entries,
                    ..
                },
            ) if dict_view_is_set_like(*left_kind) => {
                sets_equal(&dict_view_values(*left_kind, left_entries), right.as_ref())
            }
            (
                Value::MappingView {
                    kind: left_kind,
                    mapping: left_mapping,
                    identity: left_identity,
                },
                Value::MappingView {
                    kind: right_kind,
                    mapping: right_mapping,
                    identity: right_identity,
                },
            ) => {
                if dict_view_is_set_like(*left_kind) && dict_view_is_set_like(*right_kind) {
                    left_mapping == right_mapping
                } else {
                    left_kind == right_kind && Rc::ptr_eq(left_identity, right_identity)
                }
            }
            (
                Value::MappingProxy {
                    entries: left_entries,
                    ..
                },
                Value::MappingProxy {
                    entries: right_entries,
                    ..
                },
            ) => left_entries.borrow().entries == right_entries.borrow().entries,
            (Value::MappingProxy { entries: left, .. }, Value::Dict(right))
            | (Value::Dict(right), Value::MappingProxy { entries: left, .. })
            | (Value::MappingProxy { entries: left, .. }, Value::OrderedDict(right))
            | (Value::OrderedDict(right), Value::MappingProxy { entries: left, .. }) => {
                left.borrow().entries == right.borrow().entries
            }
            (
                Value::MappingProxyObject {
                    mapping: left_mapping,
                    ..
                },
                Value::MappingProxyObject {
                    mapping: right_mapping,
                    ..
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
                    ..
                },
                Value::Slice {
                    start: right_start,
                    stop: right_stop,
                    step: right_step,
                    ..
                },
            ) => left_start == right_start && left_stop == right_stop && left_step == right_step,
            (
                Value::Range {
                    start: left_start,
                    stop: left_stop,
                    step: left_step,
                    ..
                },
                Value::Range {
                    start: right_start,
                    stop: right_stop,
                    step: right_step,
                    ..
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
                Value::ItertoolsFilterFalse {
                    function: left_function,
                    iterator: left_iterator,
                },
                Value::ItertoolsFilterFalse {
                    function: right_function,
                    iterator: right_iterator,
                },
            ) => left_function == right_function && left_iterator == right_iterator,
            (
                Value::ItertoolsTakewhile {
                    predicate: left_predicate,
                    iterator: left_iterator,
                    done: left_done,
                },
                Value::ItertoolsTakewhile {
                    predicate: right_predicate,
                    iterator: right_iterator,
                    done: right_done,
                },
            ) => {
                left_predicate == right_predicate
                    && left_iterator == right_iterator
                    && left_done == right_done
            }
            (
                Value::ItertoolsDropwhile {
                    predicate: left_predicate,
                    iterator: left_iterator,
                    dropping: left_dropping,
                },
                Value::ItertoolsDropwhile {
                    predicate: right_predicate,
                    iterator: right_iterator,
                    dropping: right_dropping,
                },
            ) => {
                left_predicate == right_predicate
                    && left_iterator == right_iterator
                    && left_dropping == right_dropping
            }
            (
                Value::ItertoolsStarmap {
                    function: left_function,
                    iterator: left_iterator,
                },
                Value::ItertoolsStarmap {
                    function: right_function,
                    iterator: right_iterator,
                },
            ) => left_function == right_function && left_iterator == right_iterator,
            (
                Value::ItertoolsZipLongest {
                    iterators: left_iterators,
                    fillvalue: left_fillvalue,
                },
                Value::ItertoolsZipLongest {
                    iterators: right_iterators,
                    fillvalue: right_fillvalue,
                },
            ) => left_iterators == right_iterators && left_fillvalue == right_fillvalue,
            (
                Value::ItertoolsAccumulate {
                    iterator: left_iterator,
                    function: left_function,
                    total: left_total,
                    initial: left_initial,
                },
                Value::ItertoolsAccumulate {
                    iterator: right_iterator,
                    function: right_function,
                    total: right_total,
                    initial: right_initial,
                },
            ) => {
                left_iterator == right_iterator
                    && left_function == right_function
                    && left_total == right_total
                    && left_initial == right_initial
            }
            (
                Value::ItertoolsCycle {
                    iterator: left_iterator,
                    saved: left_saved,
                    index: left_index,
                    exhausted: left_exhausted,
                },
                Value::ItertoolsCycle {
                    iterator: right_iterator,
                    saved: right_saved,
                    index: right_index,
                    exhausted: right_exhausted,
                },
            ) => {
                left_iterator == right_iterator
                    && left_saved == right_saved
                    && left_index == right_index
                    && left_exhausted == right_exhausted
            }
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
            (
                Value::TypesCoroutineFunction {
                    identity: left_identity,
                    ..
                },
                Value::TypesCoroutineFunction {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (
                Value::MagicMock {
                    identity: left_identity,
                    ..
                },
                Value::MagicMock {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::MockMethod {
                    identity: left_identity,
                    ..
                },
                Value::MockMethod {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::WeakRef {
                    identity: left_identity,
                    ..
                },
                Value::WeakRef {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (
                Value::ParamSpecAccess {
                    identity: left_identity,
                    ..
                },
                Value::ParamSpecAccess {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (Value::Generator(left), Value::Generator(right)) => Rc::ptr_eq(left, right),
            (
                Value::GeneratorWrapper {
                    identity: left_identity,
                    ..
                },
                Value::GeneratorWrapper {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
            (Value::Coroutine(left), Value::Coroutine(right)) => Rc::ptr_eq(left, right),
            (Value::CoroutineAwait(left), Value::CoroutineAwait(right)) => Rc::ptr_eq(left, right),
            (Value::AwaitIterator(left), Value::AwaitIterator(right)) => left == right,
            (Value::AsyncGenerator(left), Value::AsyncGenerator(right)) => Rc::ptr_eq(left, right),
            (
                Value::AsyncGeneratorNext {
                    identity: left_identity,
                    ..
                },
                Value::AsyncGeneratorNext {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
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
                Value::NewType {
                    identity: left_identity,
                    ..
                },
                Value::NewType {
                    identity: right_identity,
                    ..
                },
            ) => Rc::ptr_eq(left_identity, right_identity),
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
            (Value::ForwardRef { arg: left }, Value::ForwardRef { arg: right }) => left == right,
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
                    ..
                },
                Value::GenericAlias {
                    origin: right_origin,
                    args: right_args,
                    ..
                },
            ) if is_union_origin(left_origin) && is_union_origin(right_origin) => {
                left_args.len() == right_args.len()
                    && left_args
                        .iter()
                        .all(|left| right_args.iter().any(|right| right == left))
            }
            (
                Value::GenericAlias {
                    origin: left_origin,
                    args: left_args,
                    ..
                },
                Value::GenericAlias {
                    origin: right_origin,
                    args: right_args,
                    ..
                },
            ) if is_literal_origin(left_origin) && is_literal_origin(right_origin) => {
                literal_args_equal(left_args, right_args)
            }
            (
                Value::GenericAlias {
                    origin: left_origin,
                    args: left_args,
                    ..
                },
                Value::GenericAlias {
                    origin: right_origin,
                    args: right_args,
                    ..
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
                    doc_from_getter: left_doc_from_getter,
                    name: left_name,
                    ..
                },
                Value::Property {
                    fget: right_fget,
                    fset: right_fset,
                    fdel: right_fdel,
                    doc: right_doc,
                    doc_from_getter: right_doc_from_getter,
                    name: right_name,
                    ..
                },
            ) => {
                left_fget == right_fget
                    && left_fset == right_fset
                    && left_fdel == right_fdel
                    && *left_doc.borrow() == *right_doc.borrow()
                    && left_doc_from_getter == right_doc_from_getter
                    && *left_name.borrow() == *right_name.borrow()
            }
            (
                Value::MemberDescriptor {
                    name: left_name,
                    owner_name: left_owner_name,
                    ..
                },
                Value::MemberDescriptor {
                    name: right_name,
                    owner_name: right_owner_name,
                    ..
                },
            ) => left_name == right_name && left_owner_name == right_owner_name,
            (
                Value::StaticMethod {
                    function: left_function,
                    ..
                },
                Value::StaticMethod {
                    function: right_function,
                    ..
                },
            )
            | (
                Value::ClassMethod {
                    function: left_function,
                    ..
                },
                Value::ClassMethod {
                    function: right_function,
                    ..
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
                Value::LruCacheWrapper {
                    identity: left_identity,
                    ..
                },
                Value::LruCacheWrapper {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::SingleDispatch {
                    identity: left_identity,
                    ..
                },
                Value::SingleDispatch {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::SingleDispatchRegister {
                    identity: left_identity,
                    ..
                },
                Value::SingleDispatchRegister {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::SingleDispatchMethod {
                    identity: left_identity,
                    ..
                },
                Value::SingleDispatchMethod {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::SingleDispatchMethodCallable {
                    identity: left_identity,
                    ..
                },
                Value::SingleDispatchMethodCallable {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::PartialMethod {
                    identity: left_identity,
                    ..
                },
                Value::PartialMethod {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::PartialMethodCall {
                    identity: left_identity,
                    ..
                },
                Value::PartialMethodCall {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::CachedProperty {
                    identity: left_identity,
                    ..
                },
                Value::CachedProperty {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::CmpToKey {
                    identity: left_identity,
                    ..
                },
                Value::CmpToKey {
                    identity: right_identity,
                    ..
                },
            )
            | (
                Value::CmpToKeyObject {
                    identity: left_identity,
                    ..
                },
                Value::CmpToKeyObject {
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

fn float_equals_integer_exact(value: f64, integer: &BigInt) -> bool {
    if !value.is_finite() || value.fract() != 0.0 {
        return false;
    }

    let bits = value.to_bits();
    let negative = (bits >> 63) != 0;
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let mantissa_bits = bits & ((1_u64 << 52) - 1);
    if exponent_bits == 0 {
        return mantissa_bits == 0 && integer == &BigInt::from(0);
    }

    let mut converted = BigInt::from(mantissa_bits | (1_u64 << 52));
    let exponent = exponent_bits - 1023 - 52;
    if exponent >= 0 {
        converted <<= exponent as usize;
    } else {
        let shift = (-exponent) as usize;
        let divisor = BigInt::from(1) << shift;
        if (&converted % &divisor) != BigInt::from(0) {
            return false;
        }
        converted >>= shift;
    }

    if negative {
        converted = -converted;
    }
    &converted == integer
}

fn format_tuple(items: &[Value]) -> String {
    match items {
        [] => "()".to_string(),
        [item] => format!("({},)", format_value_repr(item)),
        _ => format!("({})", format_list_items(items)),
    }
}

fn format_named_tuple(typ: &NamedTupleType, values: &[Value]) -> String {
    format_named_tuple_with_name(&named_tuple_display_name(typ), &typ.fields, values)
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

fn named_tuple_display_name(typ: &NamedTupleType) -> String {
    if matches!(&typ.module, Value::String(module) if module == "sys")
        && matches!(
            typ.name.as_str(),
            "version_info" | "float_info" | "hash_info" | "flags"
        )
    {
        format!("sys.{}", typ.name)
    } else {
        typ.name.clone()
    }
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

fn format_tuple_subclass(fields: &Scope) -> Option<String> {
    let storage = fields.borrow().get(TUPLE_SUBCLASS_STORAGE_FIELD).cloned()?;
    let Value::Tuple(items) = storage else {
        return None;
    };
    Some(format_tuple(items.as_ref()))
}

fn format_set_subclass(class_name: &str, fields: &Scope) -> Option<String> {
    let storage = fields.borrow().get(SET_SUBCLASS_STORAGE_FIELD).cloned()?;
    let Value::Set(items) = storage else {
        return None;
    };
    let items = items.borrow();
    Some(format_set_subclass_items(class_name, &items))
}

fn format_frozen_set_subclass(class_name: &str, fields: &Scope) -> Option<String> {
    let storage = fields
        .borrow()
        .get(FROZEN_SET_SUBCLASS_STORAGE_FIELD)
        .cloned()?;
    let Value::FrozenSet(items) = storage else {
        return None;
    };
    Some(format_set_subclass_items(class_name, items.as_ref()))
}

fn format_set_subclass_items(class_name: &str, items: &[Value]) -> String {
    if items.is_empty() {
        format!("{class_name}()")
    } else {
        format!("{class_name}({{{}}})", format_list_items(items))
    }
}

fn format_generic_alias_subclass(fields: &Scope) -> Option<String> {
    let alias = fields
        .borrow()
        .get(GENERIC_ALIAS_SUBCLASS_STORAGE_FIELD)
        .cloned()?;
    match alias {
        Value::GenericAlias { origin, args, .. } if is_union_origin(&origin) => {
            Some(format_union_args(&args))
        }
        Value::GenericAlias { origin, args, .. } => Some(format_generic_alias(&origin, &args)),
        _ => None,
    }
}

fn format_int_subclass(fields: &Scope) -> Option<String> {
    match fields.borrow().get(INT_SUBCLASS_STORAGE_FIELD).cloned()? {
        Value::Number(value) => Some(value.to_string()),
        Value::BigInt(value) => Some(value.to_string()),
        _ => None,
    }
}

fn format_int_enum_member_repr(class_name: &str, fields: &Scope) -> Option<String> {
    let fields_ref = fields.borrow();
    let member_name = match fields_ref.get(INT_ENUM_MEMBER_NAME_FIELD)? {
        Value::String(value) | Value::IdentityString { value, .. } => value.clone(),
        _ => return None,
    };
    let value = fields_ref.get(INT_ENUM_MEMBER_VALUE_FIELD).cloned()?;
    Some(format!(
        "<{class_name}.{member_name}: {}>",
        format_value_repr(&value)
    ))
}

fn format_float_subclass(fields: &Scope) -> Option<String> {
    match fields.borrow().get(FLOAT_SUBCLASS_STORAGE_FIELD).cloned()? {
        Value::Float(value) => Some(format_float_display(*value)),
        _ => None,
    }
}

fn format_complex_subclass(fields: &Scope) -> Option<String> {
    match fields
        .borrow()
        .get(COMPLEX_SUBCLASS_STORAGE_FIELD)
        .cloned()?
    {
        Value::Complex { real, imag, .. } => Some(format_complex(real, imag)),
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
        Value::String(value) | Value::IdentityString { value, .. } => repr_string(value),
        value => format_value_repr(value),
    }
}

fn repr_bytes(value: &[u8]) -> String {
    repr_bytes_inner(value, false)
}

fn repr_bytearray(value: &[u8]) -> String {
    format!("bytearray({})", repr_bytearray_bytes(value))
}

fn repr_bytearray_bytes(value: &[u8]) -> String {
    repr_bytes_inner(value, true)
}

fn repr_bytes_inner(value: &[u8], escape_single_quote_always: bool) -> String {
    let quote = if value.contains(&b'\'') && !value.contains(&b'"') {
        b'"'
    } else {
        b'\''
    };
    let mut result = String::from("b");
    result.push(quote as char);
    for byte in value {
        match *byte {
            b'\\' => result.push_str("\\\\"),
            b'\'' if escape_single_quote_always || quote == b'\'' => result.push_str("\\'"),
            b'\'' => result.push('\''),
            b'"' if quote == b'"' => result.push_str("\\\""),
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            b'\t' => result.push_str("\\t"),
            0x20..=0x7e => result.push(*byte as char),
            byte => result.push_str(&format!("\\x{byte:02x}")),
        }
    }
    result.push(quote as char);
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

fn dict_view_type_name(kind: DictViewKind, ordered: bool) -> &'static str {
    match (kind, ordered) {
        (DictViewKind::Keys, true) => "odict_keys",
        (DictViewKind::Values, true) => "odict_values",
        (DictViewKind::Items, true) => "odict_items",
        (DictViewKind::Keys, false) => "dict_keys",
        (DictViewKind::Values, false) => "dict_values",
        (DictViewKind::Items, false) => "dict_items",
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

fn format_ordered_dict(entries: &[(Value, Value)]) -> String {
    if entries.is_empty() {
        return "OrderedDict()".to_string();
    }

    let items = entries
        .iter()
        .map(|(key, value)| format!("{}: {}", format_value_repr(key), format_value_repr(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("OrderedDict({{{items}}})")
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
