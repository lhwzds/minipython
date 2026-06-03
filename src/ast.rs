#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Pass,
    Expr(Expr),
    Assign {
        targets: Vec<Target>,
        value: Expr,
        type_comment: Option<String>,
    },
    AnnAssign {
        target: Target,
        annotation: Expr,
        value: Option<Expr>,
        simple: bool,
    },
    TypeAlias {
        name: String,
        type_params: Vec<TypeParam>,
        value: Expr,
    },
    AugAssign {
        target: Target,
        op: BinaryOp,
        value: Expr,
    },
    Delete {
        target: Target,
    },
    FunctionDef {
        name: String,
        type_params: Vec<TypeParam>,
        params: FunctionParams,
        body: Vec<Stmt>,
        decorators: Vec<Expr>,
        returns: Option<Expr>,
        type_comment: Option<String>,
    },
    AsyncFunctionDef {
        name: String,
        type_params: Vec<TypeParam>,
        params: FunctionParams,
        body: Vec<Stmt>,
        decorators: Vec<Expr>,
        returns: Option<Expr>,
        type_comment: Option<String>,
    },
    ClassDef {
        name: String,
        type_params: Vec<TypeParam>,
        bases: Vec<CallArg>,
        keywords: Vec<CallKeyword>,
        body: Vec<Stmt>,
        decorators: Vec<Expr>,
    },
    Import {
        is_lazy: bool,
        aliases: Vec<ImportAlias>,
    },
    ImportFrom {
        is_lazy: bool,
        module: Option<String>,
        level: usize,
        targets: ImportFromTargets,
    },
    Return(Option<Expr>),
    Global(Vec<String>),
    Nonlocal(Vec<String>),
    Assert {
        condition: Expr,
        message: Option<Expr>,
    },
    Raise {
        value: Option<Expr>,
        cause: Option<Expr>,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    Match {
        subject: Expr,
        cases: Vec<MatchCase>,
    },
    Try {
        body: Vec<Stmt>,
        handlers: Vec<ExceptHandler>,
        else_body: Vec<Stmt>,
        finally_body: Vec<Stmt>,
    },
    TryStar {
        body: Vec<Stmt>,
        handlers: Vec<ExceptHandler>,
        else_body: Vec<Stmt>,
        finally_body: Vec<Stmt>,
    },
    With {
        items: Vec<WithItem>,
        body: Vec<Stmt>,
        type_comment: Option<String>,
    },
    AsyncWith {
        items: Vec<WithItem>,
        body: Vec<Stmt>,
        type_comment: Option<String>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    For {
        target: Target,
        iter: Expr,
        body: Vec<Stmt>,
        else_body: Vec<Stmt>,
        type_comment: Option<String>,
    },
    AsyncFor {
        target: Target,
        iter: Expr,
        body: Vec<Stmt>,
        else_body: Vec<Stmt>,
        type_comment: Option<String>,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Name(String),
    Attribute {
        object: Box<Expr>,
        name: String,
    },
    Subscript {
        object: Box<Expr>,
        index: Expr,
    },
    Slice {
        object: Box<Expr>,
        start: Option<Expr>,
        stop: Option<Expr>,
        step: Option<Expr>,
    },
    Starred(Box<Target>),
    Tuple(Vec<Target>),
    List(Vec<Target>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptHandler {
    pub type_expr: Option<Expr>,
    pub name: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithItem {
    pub context_expr: Expr,
    pub optional_vars: Option<Target>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAlias {
    pub name: String,
    pub asname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportFromTargets {
    Star,
    Aliases(Vec<ImportAlias>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Literal(Expr),
    Singleton(Expr),
    Value(Expr),
    Capture(String),
    Wildcard,
    Or(Vec<Pattern>),
    Sequence(Vec<Pattern>),
    Mapping {
        entries: Vec<(Expr, Pattern)>,
        rest: Option<String>,
    },
    Class {
        class: Expr,
        positional: Vec<Pattern>,
        keywords: Vec<(String, Pattern)>,
    },
    Star(Option<String>),
    As {
        pattern: Box<Pattern>,
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComprehensionClause {
    pub is_async: bool,
    pub target: Target,
    pub iter: Expr,
    pub ifs: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Number(i64),
    BigInt(String),
    Float(String),
    Imaginary(String),
    String(String),
    Bytes(Vec<u8>),
    JoinedString(Vec<FStringPart>),
    TemplateString(Vec<TemplateStringPart>),
    Bool(bool),
    None,
    Ellipsis,
    Name(String),
    Attribute {
        object: Box<Expr>,
        name: String,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Comparison {
        left: Box<Expr>,
        op: ComparisonOp,
        right: Box<Expr>,
    },
    ChainedComparison {
        left: Box<Expr>,
        comparisons: Vec<(ComparisonOp, Expr)>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Logical {
        left: Box<Expr>,
        op: LogicalOp,
        right: Box<Expr>,
    },
    IfExpression {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    NamedExpr {
        name: String,
        value: Box<Expr>,
    },
    Yield {
        value: Option<Box<Expr>>,
    },
    YieldFrom(Box<Expr>),
    Await(Box<Expr>),
    Starred(Box<Expr>),
    List(Vec<Expr>),
    ListComp {
        element: Box<Expr>,
        clauses: Vec<ComprehensionClause>,
    },
    Set(Vec<Expr>),
    FrozenSet(Vec<Expr>),
    SetComp {
        element: Box<Expr>,
        clauses: Vec<ComprehensionClause>,
    },
    GeneratorComp {
        element: Box<Expr>,
        clauses: Vec<ComprehensionClause>,
    },
    Tuple(Vec<Expr>),
    Dict(Vec<DictItem>),
    DictComp {
        key: Box<Expr>,
        value: Box<Expr>,
        clauses: Vec<ComprehensionClause>,
    },
    DictUnpackComp {
        value: Box<Expr>,
        clauses: Vec<ComprehensionClause>,
    },
    Subscript {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    SliceLiteral {
        start: Option<Box<Expr>>,
        stop: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
    },
    Slice {
        object: Box<Expr>,
        start: Option<Box<Expr>>,
        stop: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    KeywordCall {
        callee: Box<Expr>,
        args: Vec<Expr>,
        keywords: Vec<(String, Expr)>,
    },
    UnpackCall {
        callee: Box<Expr>,
        args: Vec<CallArg>,
        keywords: Vec<CallKeyword>,
    },
    Lambda {
        params: FunctionParams,
        body: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FStringPart {
    Literal(String),
    Formatted {
        value: Box<Expr>,
        conversion: Option<FStringConversion>,
        format_spec: Option<Vec<FStringPart>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FStringConversion {
    Str,
    Repr,
    Ascii,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateStringPart {
    Literal(String),
    Interpolation {
        value: Box<Expr>,
        expression: String,
        conversion: Option<FStringConversion>,
        format_spec: Option<Vec<FStringPart>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub arg_types: Vec<Expr>,
    pub returns: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeParamKind {
    TypeVar,
    TypeVarTuple,
    ParamSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    pub kind: TypeParamKind,
    pub name: String,
    pub bound: Option<Expr>,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub annotation: Option<Expr>,
    pub default: Option<Expr>,
    pub type_comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionParams {
    pub positional_only: Vec<Param>,
    pub positional: Vec<Param>,
    pub vararg: Option<String>,
    pub vararg_annotation: Option<Box<Expr>>,
    pub vararg_type_comment: Option<String>,
    pub keyword_only: Vec<Param>,
    pub kwarg: Option<String>,
    pub kwarg_annotation: Option<Box<Expr>>,
    pub kwarg_type_comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallArg {
    Expr(Expr),
    Unpack(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallKeyword {
    Named(String, Expr),
    Unpack(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictItem {
    Entry { key: Expr, value: Expr },
    Unpack(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    MatrixMultiply,
    TrueDivide,
    FloorDivide,
    Modulo,
    Power,
    BitOr,
    BitXor,
    BitAnd,
    LeftShift,
    RightShift,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    In,
    NotIn,
    Is,
    IsNot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Positive,
    Negative,
    Invert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}
