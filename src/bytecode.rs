use crate::value::Value;

pub type Register = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum CallArgRegister {
    Value(Register),
    Unpack(Register),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallKeywordRegister {
    Named(String, Register),
    Unpack(Register),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePartRegister {
    Literal(String),
    Interpolation {
        value: Register,
        expression: String,
        conversion: Option<FormatConversion>,
        format_spec: Option<Register>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptHandler {
    pub type_names: Option<Vec<String>>,
    pub type_register: Option<Register>,
    pub name: Option<String>,
    pub name_binding: Option<ExceptHandlerNameBinding>,
    pub target: usize,
    pub is_star: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptHandlerNameBinding {
    Local,
    Global,
    Nonlocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatConversion {
    Str,
    Repr,
    Ascii,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    #[allow(dead_code)]
    Noop,
    LoadConst {
        dst: Register,
        value: Value,
    },
    Move {
        dst: Register,
        src: Register,
    },
    FormatValue {
        dst: Register,
        src: Register,
        conversion: Option<FormatConversion>,
        format_spec: Option<Register>,
    },
    BuildTemplate {
        dst: Register,
        parts: Vec<TemplatePartRegister>,
    },
    BuildUnpack {
        dst: Register,
        value: Register,
    },
    LoadName {
        dst: Register,
        name: String,
    },
    LoadLocal {
        dst: Register,
        name: String,
    },
    LoadGlobal {
        dst: Register,
        name: String,
    },
    LoadNonlocal {
        dst: Register,
        name: String,
    },
    StoreName {
        name: String,
        src: Register,
    },
    StoreGlobal {
        name: String,
        src: Register,
    },
    StoreNonlocal {
        name: String,
        src: Register,
    },
    StoreOuterName {
        name: String,
        src: Register,
    },
    StoreAnnotation {
        name: String,
        annotation: Register,
    },
    DeleteName {
        name: String,
    },
    DeleteGlobal {
        name: String,
    },
    DeleteNonlocal {
        name: String,
    },
    ImportModule {
        dst: Register,
        name: String,
        return_root: bool,
        level: usize,
    },
    ImportFrom {
        dst: Register,
        module: Register,
        name: String,
    },
    ImportStar {
        module: Register,
    },
    LoadAttribute {
        dst: Register,
        object: Register,
        name: String,
    },
    LoadContextManagerExit {
        dst: Register,
        manager: Register,
        is_async: bool,
    },
    LoadContextManagerEnter {
        dst: Register,
        manager: Register,
        is_async: bool,
    },
    StoreAttribute {
        object: Register,
        name: String,
        src: Register,
    },
    DeleteAttribute {
        object: Register,
        name: String,
    },
    StoreSubscript {
        object: Register,
        index: Register,
        src: Register,
    },
    DeleteSubscript {
        object: Register,
        index: Register,
    },
    StoreSlice {
        object: Register,
        start: Option<Register>,
        stop: Option<Register>,
        step: Option<Register>,
        src: Register,
    },
    DeleteSlice {
        object: Register,
        start: Option<Register>,
        stop: Option<Register>,
        step: Option<Register>,
    },
    MatchMapping {
        dst: Register,
        src: Register,
    },
    MatchMappingKeys {
        dst: Register,
        mapping: Register,
        keys: Vec<Register>,
    },
    LoadMappingRest {
        dst: Register,
        src: Register,
        keys: Vec<Register>,
    },
    MatchClass {
        dst: Register,
        subject: Register,
        class: Register,
        positional_count: usize,
        keyword_names: Vec<String>,
    },
    LoadMatchClassPositional {
        dst: Register,
        found: Register,
        subject: Register,
        class: Register,
        index: usize,
    },
    LoadMatchAttribute {
        dst: Register,
        found: Register,
        object: Register,
        name: String,
    },
    MatchSequence {
        dst: Register,
        src: Register,
        min_len: usize,
        exact: bool,
    },
    LoadSequenceRest {
        dst: Register,
        src: Register,
        start: usize,
        suffix: usize,
    },
    UnpackSequence {
        src: Register,
        dst: Vec<Register>,
    },
    UnpackSequenceEx {
        src: Register,
        before: Vec<Register>,
        rest: Register,
        after: Vec<Register>,
    },
    Add {
        dst: Register,
        left: Register,
        right: Register,
    },
    InPlaceAdd {
        dst: Register,
        left: Register,
        right: Register,
    },
    Subtract {
        dst: Register,
        left: Register,
        right: Register,
    },
    InPlaceSubtract {
        dst: Register,
        left: Register,
        right: Register,
    },
    Multiply {
        dst: Register,
        left: Register,
        right: Register,
    },
    InPlaceMultiply {
        dst: Register,
        left: Register,
        right: Register,
    },
    MatrixMultiply {
        dst: Register,
        left: Register,
        right: Register,
    },
    InPlaceMatrixMultiply {
        dst: Register,
        left: Register,
        right: Register,
    },
    TrueDivide {
        dst: Register,
        left: Register,
        right: Register,
    },
    FloorDivide {
        dst: Register,
        left: Register,
        right: Register,
    },
    InPlaceFloorDivide {
        dst: Register,
        left: Register,
        right: Register,
    },
    Modulo {
        dst: Register,
        left: Register,
        right: Register,
    },
    Power {
        dst: Register,
        left: Register,
        right: Register,
    },
    BitOr {
        dst: Register,
        left: Register,
        right: Register,
    },
    InPlaceBitOr {
        dst: Register,
        left: Register,
        right: Register,
    },
    BitXor {
        dst: Register,
        left: Register,
        right: Register,
    },
    InPlaceBitXor {
        dst: Register,
        left: Register,
        right: Register,
    },
    BitAnd {
        dst: Register,
        left: Register,
        right: Register,
    },
    InPlaceBitAnd {
        dst: Register,
        left: Register,
        right: Register,
    },
    LeftShift {
        dst: Register,
        left: Register,
        right: Register,
    },
    RightShift {
        dst: Register,
        left: Register,
        right: Register,
    },
    Equal {
        dst: Register,
        left: Register,
        right: Register,
    },
    NotEqual {
        dst: Register,
        left: Register,
        right: Register,
    },
    Less {
        dst: Register,
        left: Register,
        right: Register,
    },
    LessEqual {
        dst: Register,
        left: Register,
        right: Register,
    },
    Greater {
        dst: Register,
        left: Register,
        right: Register,
    },
    GreaterEqual {
        dst: Register,
        left: Register,
        right: Register,
    },
    Contains {
        dst: Register,
        needle: Register,
        haystack: Register,
    },
    Is {
        dst: Register,
        left: Register,
        right: Register,
    },
    Not {
        dst: Register,
        src: Register,
    },
    Positive {
        dst: Register,
        src: Register,
    },
    Negate {
        dst: Register,
        src: Register,
    },
    Invert {
        dst: Register,
        src: Register,
    },
    JumpIfFalse {
        condition: Register,
        target: usize,
    },
    Jump {
        target: usize,
    },
    Call {
        dst: Register,
        callee: Register,
        args: Vec<Register>,
    },
    CallKeyword {
        dst: Register,
        callee: Register,
        args: Vec<Register>,
        keywords: Vec<(String, Register)>,
    },
    CallUnpack {
        dst: Register,
        callee: Register,
        args: Vec<CallArgRegister>,
        keywords: Vec<CallKeywordRegister>,
    },
    MakeTypeParam {
        dst: Register,
        kind: String,
        name: String,
        bound: Option<Register>,
        default: Option<Register>,
    },
    UpdateTypeParam {
        target: Register,
        bound: Option<Register>,
        default: Option<Register>,
    },
    MakeDeferredTypeParamExpr {
        dst: Register,
        body: Vec<Instruction>,
        type_params: Vec<Register>,
        class_name: Option<String>,
        is_constraint_tuple: bool,
    },
    MakeTypeAlias {
        dst: Register,
        name: String,
        type_params: Vec<Register>,
        value: Register,
    },
    MakeFunction {
        dst: Register,
        name: String,
        type_params: Vec<Register>,
        closure_bindings: Vec<(String, Register)>,
        positional_only: Vec<String>,
        params: Vec<String>,
        defaults: Vec<(String, Register)>,
        vararg: Option<String>,
        keyword_only: Vec<String>,
        keyword_defaults: Vec<(String, Register)>,
        kwarg: Option<String>,
        annotations: Vec<(String, Register)>,
        docstring: Option<String>,
        body: Vec<Instruction>,
        is_generator: bool,
        is_async: bool,
        first_line: usize,
        line_sequence: Vec<usize>,
        position_columns: Vec<Option<(usize, usize)>>,
    },
    Await {
        dst: Register,
        src: Register,
    },
    AwaitContextManager {
        dst: Register,
        src: Register,
        is_exit: bool,
    },
    MakeClass {
        dst: Register,
        name: String,
        type_params: Vec<Register>,
        bases: Vec<CallArgRegister>,
        keywords: Vec<CallKeywordRegister>,
        static_attributes: Vec<String>,
        docstring: Option<String>,
        body: Vec<Instruction>,
    },
    Return {
        src: Option<Register>,
    },
    ImplicitReturn,
    Yield {
        src: Option<Register>,
        resume_dst: Option<Register>,
    },
    Assert {
        condition: Register,
        message: Option<Register>,
    },
    SetupExcept {
        handlers: Vec<ExceptHandler>,
    },
    PopExcept,
    ClearException,
    Raise {
        src: Option<Register>,
        cause: Option<Register>,
    },
    LoadCurrentException {
        type_dst: Register,
        value_dst: Register,
        traceback_dst: Register,
    },
    BuildList {
        dst: Register,
        items: Vec<Register>,
    },
    ListAppend {
        list: Register,
        item: Register,
    },
    ListExtend {
        list: Register,
        iterable: Register,
    },
    BuildSet {
        dst: Register,
        items: Vec<Register>,
    },
    BuildFrozenSet {
        dst: Register,
        items: Vec<Register>,
    },
    SetAdd {
        set: Register,
        item: Register,
    },
    SetUpdate {
        set: Register,
        iterable: Register,
    },
    BuildTuple {
        dst: Register,
        items: Vec<Register>,
    },
    BuildTupleFromList {
        dst: Register,
        list: Register,
    },
    BuildSlice {
        dst: Register,
        start: Option<Register>,
        stop: Option<Register>,
        step: Option<Register>,
    },
    BuildDict {
        dst: Register,
        entries: Vec<(Register, Register)>,
    },
    DictSetItem {
        dict: Register,
        key: Register,
        value: Register,
    },
    DictUpdate {
        dict: Register,
        src: Register,
    },
    LoadSubscript {
        dst: Register,
        object: Register,
        index: Register,
    },
    LoadSlice {
        dst: Register,
        object: Register,
        start: Option<Register>,
        stop: Option<Register>,
        step: Option<Register>,
    },
    GetIter {
        dst: Register,
        src: Register,
    },
    GetAsyncIter {
        dst: Register,
        src: Register,
    },
    ForIter {
        iterator: Register,
        dst: Register,
        target: usize,
    },
    AsyncForIter {
        iterator: Register,
        dst: Register,
        target: usize,
    },
    ForIterValue {
        iterator: Register,
        dst: Register,
        completion: Register,
        track_yield_from: bool,
        target: usize,
    },
    Pop {
        src: Register,
    },
    Display {
        src: Register,
    },
    Halt,
}

pub(crate) fn instructions_without_debug_positions(
    instructions: &[Instruction],
) -> Vec<Instruction> {
    instructions
        .iter()
        .map(instruction_without_debug_positions)
        .collect()
}

fn instruction_without_debug_positions(instruction: &Instruction) -> Instruction {
    let mut instruction = instruction.clone();
    match &mut instruction {
        Instruction::MakeDeferredTypeParamExpr { body, .. }
        | Instruction::MakeClass { body, .. } => {
            *body = instructions_without_debug_positions(body);
        }
        Instruction::MakeFunction {
            body,
            first_line,
            line_sequence,
            position_columns,
            ..
        } => {
            *body = instructions_without_debug_positions(body);
            *first_line = 1;
            *line_sequence = vec![1];
            position_columns.clear();
        }
        _ => {}
    }
    instruction
}
