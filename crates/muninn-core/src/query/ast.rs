//! Internal query representation.
//!
//! Narrower than `sqlparser`'s AST: only the SELECT shape we support, with
//! clauses already lifted into typed fields. The parser maps `sqlparser::Query`
//! → [`MuninnQuery`], rejecting unsupported constructs along the way.

use super::value::Value;

#[derive(Debug, Clone)]
pub struct MuninnQuery {
    /// Type names resolved from the `FROM` clause. Phase 5 introduces JOINs
    /// and aliases; a bare `FROM t` parses as a `FromClause` with one entry
    /// whose alias equals the type name.
    pub from: FromClause,
    pub projections: Vec<Projection>,
    pub filter: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderBy>,
    pub limit: Option<usize>,
    pub offset: usize,
}

/// `FROM` clause: a primary table plus zero or more joined tables. Aliases
/// are always populated — a bare `FROM task` yields `alias = "task"`.
#[derive(Debug, Clone)]
pub struct FromClause {
    pub primary: TableRef,
    pub joins: Vec<Join>,
}

impl FromClause {
    /// Convenience for building a single-source FROM (tests, internal callers).
    pub fn single(type_name: impl Into<String>) -> Self {
        let name = type_name.into();
        Self {
            primary: TableRef {
                type_name: name.clone(),
                alias: name,
            },
            joins: Vec::new(),
        }
    }

    /// Every alias in the FROM clause, primary first.
    pub fn aliases(&self) -> Vec<&str> {
        let mut out = Vec::with_capacity(1 + self.joins.len());
        out.push(self.primary.alias.as_str());
        for j in &self.joins {
            out.push(j.right.alias.as_str());
        }
        out
    }

    /// Resolve an alias to its type name. Useful for the evaluator to pick
    /// the right type when multiple aliases reference the same type
    /// (self-joins).
    pub fn type_of_alias(&self, alias: &str) -> Option<&str> {
        if self.primary.alias == alias {
            return Some(&self.primary.type_name);
        }
        self.joins
            .iter()
            .find(|j| j.right.alias == alias)
            .map(|j| j.right.type_name.as_str())
    }
}

/// A single table reference inside FROM — the type plus an alias. The alias
/// is what qualified column references (`a.title`) bind against.
#[derive(Debug, Clone)]
pub struct TableRef {
    pub type_name: String,
    pub alias: String,
}

#[derive(Debug, Clone)]
pub struct Join {
    pub kind: JoinKind,
    pub right: TableRef,
    pub on: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    /// Standard inner join: only tuples satisfying `ON` survive.
    Inner,
    /// Left outer: every primary row appears at least once; right-side
    /// bindings are NULL when no match.
    Left,
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
    /// Unqualified column — resolves against the primary alias's frontmatter
    /// and synthetic columns (`path`, `title`, `tags`).
    Column(String),
    /// Qualified reference (`alias.column`) — routed to the named alias's
    /// binding in a joined row.
    QualifiedColumn { table: String, column: String },
    /// Unary logical/arithmetic operator.
    Unary { op: UnaryOp, expr: Box<Expr> },
    /// Binary logical/comparison/arithmetic operator.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Function call (built-ins registered in `functions.rs`).
    Function { name: String, args: Vec<Expr> },
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
