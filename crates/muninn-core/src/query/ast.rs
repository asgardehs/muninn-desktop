//! Internal query representation.
//!
//! Narrower than `sqlparser`'s AST: only the SELECT shape we support, with
//! clauses already lifted into typed fields. The parser maps `sqlparser::Query`
//! → [`MuninnQuery`], rejecting unsupported constructs along the way.

use super::value::Value;

#[derive(Debug, Clone)]
pub struct MuninnQuery {
    /// Type names resolved from the `FROM` clause. Phase 3 supports a single
    /// source (type name or the synthetic `note` source). Multi-source FROM
    /// lands in Phase 5 with JOINs.
    pub from: String,
    pub projections: Vec<Projection>,
    pub filter: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderBy>,
    pub limit: Option<usize>,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub enum Projection {
    /// `SELECT *` — all frontmatter fields plus the synthetic `path` column.
    Wildcard,
    /// `SELECT expr [AS alias]`.
    Expr { expr: Expr, alias: Option<String> },
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub expr: Expr,
    pub order: SortOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Expression AST for WHERE, HAVING, SELECT, and ORDER BY.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal value (parsed from SQL literals).
    Literal(Value),
    /// Column reference — resolves against frontmatter or synthetic columns
    /// (`path`, `title`, `tags`).
    Column(String),
    /// Unary logical/arithmetic operator.
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    /// Binary logical/comparison/arithmetic operator.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Function call (built-ins registered in `functions.rs`).
    Function {
        name: String,
        args: Vec<Expr>,
    },
    /// `expr IN (v1, v2, ...)`.
    In {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
    /// `expr BETWEEN low AND high`.
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        negated: bool,
    },
    /// `expr IS NULL` / `expr IS NOT NULL`.
    IsNull { expr: Box<Expr>, negated: bool },
    /// `expr LIKE pattern` (SQL wildcards: `%` and `_`).
    Like {
        expr: Box<Expr>,
        pattern: Box<Expr>,
        negated: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
}
