//! Query execution against a vault.
//!
//! The evaluator walks notes matching the FROM clause, filters with WHERE,
//! projects SELECT columns, applies ORDER BY, and paginates via LIMIT/OFFSET.
//! Commit 2 of Phase 3 extends this with GROUP BY/HAVING and built-in
//! functions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::markdown::{self, Note};
use crate::mdbase::match_type::match_types;

use super::ast::{BinaryOp, Expr, MuninnQuery, OrderBy, Projection, SortOrder, UnaryOp};
use super::functions;
use super::value::Value;
use super::{ANY_TYPE_SOURCE, MAX_RESULT_ROWS};

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("unknown type: {0}")]
    UnknownType(String),
    #[error("type mismatch in expression: {0}")]
    TypeMismatch(String),
    #[error("unknown function: {0}")]
    UnknownFunction(String),
    #[error("result set exceeds max rows ({0})")]
    ResultTooLarge(usize),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] markdown::ParseError),
    #[error("walk error: {0}")]
    Walk(#[from] walkdir::Error),
}

/// Row identity + its source note path + materialized column values.
#[derive(Debug, Clone)]
pub struct QueryResultRow {
    pub path: PathBuf,
    pub cells: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct QueryResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<QueryResultRow>,
}

/// Execute a parsed query against a vault rooted at `vault_root`.
///
/// Kept standalone (no `&Vault` reference) so the evaluator can be reused by
/// tests and by future Runestone materialization without pulling in the full
/// `Vault` surface.
pub fn execute(
    vault_root: &Path,
    types: &HashMap<String, crate::mdbase::types::TypeDef>,
    config: Option<&crate::mdbase::config::MdbaseConfig>,
    query: &MuninnQuery,
) -> Result<QueryResultSet, EvalError> {
    let source = load_source_rows(vault_root, types, config, &query.from)?;

    let filtered: Vec<Note> = match &query.filter {
        Some(expr) => source
            .into_iter()
            .filter_map(|n| match eval_predicate(expr, &RowCtx::new(&n)) {
                Ok(true) => Some(Ok(n)),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => source,
    };

    let columns = resolve_columns(&query.projections);

    let grouped = !query.group_by.is_empty()
        || query.having.is_some()
        || projections_have_aggregate(&query.projections);

    let mut rows = if grouped {
        execute_grouped(&filtered, query, vault_root)?
    } else {
        execute_simple(&filtered, query, vault_root)?
    };

    apply_pagination(&mut rows, query.offset, query.limit);

    if rows.len() > MAX_RESULT_ROWS {
        return Err(EvalError::ResultTooLarge(MAX_RESULT_ROWS));
    }

    Ok(QueryResultSet { columns, rows })
}

fn execute_simple(
    filtered: &[Note],
    query: &MuninnQuery,
    vault_root: &Path,
) -> Result<Vec<QueryResultRow>, EvalError> {
    let mut rows: Vec<QueryResultRow> = filtered
        .iter()
        .map(|n| project_row(n, &query.projections, vault_root))
        .collect::<Result<Vec<_>, _>>()?;
    let resolved_order: Vec<OrderBy> = query
        .order_by
        .iter()
        .map(|ob| OrderBy {
            expr: resolve_alias(&ob.expr, &query.projections),
            order: ob.order,
        })
        .collect();
    apply_order_by(&mut rows, &resolved_order, filtered, vault_root)?;
    Ok(rows)
}

/// If `expr` is a bare column reference matching a projection alias, swap it
/// for the underlying projection expression. Standard SQL behavior for
/// ORDER BY clauses.
fn resolve_alias(expr: &Expr, projections: &[Projection]) -> Expr {
    if let Expr::Column(name) = expr {
        for p in projections {
            if let Projection::Expr {
                expr: inner,
                alias: Some(a),
            } = p
                && a == name
            {
                return inner.clone();
            }
        }
    }
    expr.clone()
}

/// GROUP BY + aggregate execution path.
///
/// Rows are bucketed by the values of each GROUP BY expression. Non-aggregate
/// projection parts evaluate in the first row of the group; aggregate function
/// calls fold the argument across all rows in the group. HAVING filters whole
/// groups. ORDER BY evaluates against the same group context.
fn execute_grouped(
    filtered: &[Note],
    query: &MuninnQuery,
    vault_root: &Path,
) -> Result<Vec<QueryResultRow>, EvalError> {
    // One big group when GROUP BY is absent but aggregates are present (i.e.
    // `SELECT COUNT(*) FROM note` with no GROUP BY).
    let groups = build_groups(filtered, &query.group_by)?;

    let mut rows: Vec<(Vec<Value>, QueryResultRow)> = Vec::with_capacity(groups.len());
    for group in &groups {
        if group.notes.is_empty() {
            continue;
        }

        if let Some(expr) = &query.having {
            let keep = eval_in_group(expr, group)?.is_truthy();
            if !keep {
                continue;
            }
        }

        let mut cells = Vec::with_capacity(query.projections.len());
        for p in &query.projections {
            match p {
                Projection::Wildcard => {
                    return Err(EvalError::TypeMismatch(
                        "SELECT * is not allowed with GROUP BY / aggregates".to_string(),
                    ));
                }
                Projection::Expr { expr, .. } => {
                    cells.push(eval_in_group(expr, group)?);
                }
            }
        }

        // ORDER BY keys evaluated in the same group context; resolve aliases
        // so `ORDER BY my_alias` works when `my_alias` names a projection.
        let sort_keys: Vec<Value> = query
            .order_by
            .iter()
            .map(|ob| eval_in_group(&resolve_alias(&ob.expr, &query.projections), group))
            .collect::<Result<Vec<_>, _>>()?;

        let first_note = group.notes[0];
        let rel = first_note
            .path
            .strip_prefix(vault_root)
            .unwrap_or(&first_note.path);

        rows.push((
            sort_keys,
            QueryResultRow {
                path: rel.to_path_buf(),
                cells,
            },
        ));
    }

    if !query.order_by.is_empty() {
        rows.sort_by(|a, b| {
            for ((ka, kb), ob) in a.0.iter().zip(b.0.iter()).zip(query.order_by.iter()) {
                let ord = ka.cmp_for_order(kb);
                if ord != std::cmp::Ordering::Equal {
                    return if ob.order == SortOrder::Desc {
                        ord.reverse()
                    } else {
                        ord
                    };
                }
            }
            std::cmp::Ordering::Equal
        });
    }

    Ok(rows.into_iter().map(|(_, r)| r).collect())
}

struct Group<'a> {
    key: Vec<Value>,
    notes: Vec<&'a Note>,
}

fn build_groups<'a>(
    filtered: &'a [Note],
    group_by: &[Expr],
) -> Result<Vec<Group<'a>>, EvalError> {
    if group_by.is_empty() {
        // Single synthetic group holding every filtered row.
        return Ok(vec![Group {
            key: Vec::new(),
            notes: filtered.iter().collect(),
        }]);
    }

    let mut groups: Vec<Group<'a>> = Vec::new();
    for note in filtered {
        let ctx = RowCtx::new(note);
        let key: Vec<Value> = group_by
            .iter()
            .map(|e| eval_expr(e, &ctx))
            .collect::<Result<Vec<_>, _>>()?;

        if let Some(existing) = groups.iter_mut().find(|g| keys_equal(&g.key, &key)) {
            existing.notes.push(note);
        } else {
            groups.push(Group {
                key,
                notes: vec![note],
            });
        }
    }
    Ok(groups)
}

fn keys_equal(a: &[Value], b: &[Value]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| {
        // NULL group keys match each other in GROUP BY semantics.
        if x.is_null() && y.is_null() {
            return true;
        }
        x.sql_eq(y).unwrap_or(false)
    })
}

fn projections_have_aggregate(projs: &[Projection]) -> bool {
    projs.iter().any(|p| match p {
        Projection::Wildcard => false,
        Projection::Expr { expr, .. } => expr_has_aggregate(expr),
    })
}

fn expr_has_aggregate(expr: &Expr) -> bool {
    match expr {
        Expr::Function { name, args } => {
            functions::is_aggregate(name) || args.iter().any(expr_has_aggregate)
        }
        Expr::Unary { expr, .. } => expr_has_aggregate(expr),
        Expr::Binary { left, right, .. } => expr_has_aggregate(left) || expr_has_aggregate(right),
        Expr::In { expr, list, .. } => {
            expr_has_aggregate(expr) || list.iter().any(expr_has_aggregate)
        }
        Expr::Between {
            expr, low, high, ..
        } => expr_has_aggregate(expr) || expr_has_aggregate(low) || expr_has_aggregate(high),
        Expr::IsNull { expr, .. } => expr_has_aggregate(expr),
        Expr::Like { expr, pattern, .. } => {
            expr_has_aggregate(expr) || expr_has_aggregate(pattern)
        }
        Expr::Literal(_) | Expr::Column(_) => false,
    }
}

fn eval_in_group(expr: &Expr, group: &Group<'_>) -> Result<Value, EvalError> {
    match expr {
        Expr::Function { name, args } if functions::is_aggregate(name) => {
            if args.len() != 1 {
                return Err(EvalError::TypeMismatch(format!(
                    "{} expects exactly one argument",
                    name.to_ascii_uppercase()
                )));
            }
            let is_star = matches!(&args[0], Expr::Column(s) if s == "*");
            let per_row: Vec<Value> = if is_star {
                group.notes.iter().map(|_| Value::Null).collect()
            } else {
                group
                    .notes
                    .iter()
                    .map(|n| eval_expr(&args[0], &RowCtx::new(n)))
                    .collect::<Result<Vec<_>, _>>()?
            };
            functions::fold_aggregate(name, &per_row, is_star)
        }
        Expr::Function { name, args } => {
            // Non-aggregate function call inside group context — fall back to
            // scalar evaluation using the first row.
            let values: Vec<Value> = args
                .iter()
                .map(|a| eval_in_group(a, group))
                .collect::<Result<Vec<_>, _>>()?;
            functions::call_scalar(name, &values)
        }
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Column(name) => Ok(first_row_ctx(group)?.resolve(name)),
        Expr::Unary { op, expr } => {
            let v = eval_in_group(expr, group)?;
            match op {
                UnaryOp::Not => Ok(Value::Bool(!v.is_truthy())),
                UnaryOp::Neg => match v {
                    Value::Integer(n) => Ok(Value::Integer(-n)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    Value::Null => Ok(Value::Null),
                    other => Err(EvalError::TypeMismatch(format!(
                        "cannot negate {}",
                        other.type_name()
                    ))),
                },
            }
        }
        Expr::Binary { op, left, right } => {
            let l = eval_in_group(left, group)?;
            let r = eval_in_group(right, group)?;
            eval_binary(*op, &l, &r)
        }
        Expr::In { expr, list, negated } => {
            let v = eval_in_group(expr, group)?;
            let mut any_match = false;
            for item in list {
                let rhs = eval_in_group(item, group)?;
                if v.sql_eq(&rhs).unwrap_or(false) {
                    any_match = true;
                    break;
                }
            }
            Ok(Value::Bool(if *negated { !any_match } else { any_match }))
        }
        Expr::Between {
            expr,
            low,
            high,
            negated,
        } => {
            let v = eval_in_group(expr, group)?;
            let lo = eval_in_group(low, group)?;
            let hi = eval_in_group(high, group)?;
            let in_range = v.cmp_for_order(&lo) != std::cmp::Ordering::Less
                && v.cmp_for_order(&hi) != std::cmp::Ordering::Greater;
            Ok(Value::Bool(if *negated { !in_range } else { in_range }))
        }
        Expr::IsNull { expr, negated } => {
            let v = eval_in_group(expr, group)?;
            let b = v.is_null();
            Ok(Value::Bool(if *negated { !b } else { b }))
        }
        Expr::Like {
            expr,
            pattern,
            negated,
        } => {
            let v = eval_in_group(expr, group)?;
            let p = eval_in_group(pattern, group)?;
            let s = match v {
                Value::String(s) => s,
                Value::Null => return Ok(Value::Bool(false)),
                other => {
                    return Err(EvalError::TypeMismatch(format!(
                        "LIKE left operand must be string, got {}",
                        other.type_name()
                    )));
                }
            };
            let pat = match p {
                Value::String(s) => s,
                other => {
                    return Err(EvalError::TypeMismatch(format!(
                        "LIKE pattern must be string, got {}",
                        other.type_name()
                    )));
                }
            };
            let matched = like_match(&s, &pat);
            Ok(Value::Bool(if *negated { !matched } else { matched }))
        }
    }
}

fn first_row_ctx<'a>(group: &'a Group<'a>) -> Result<RowCtx<'a>, EvalError> {
    let note = group
        .notes
        .first()
        .copied()
        .ok_or_else(|| EvalError::TypeMismatch("empty group".to_string()))?;
    Ok(RowCtx::new(note))
}

fn load_source_rows(
    vault_root: &Path,
    types: &HashMap<String, crate::mdbase::types::TypeDef>,
    config: Option<&crate::mdbase::config::MdbaseConfig>,
    source: &str,
) -> Result<Vec<Note>, EvalError> {
    let is_any = source == ANY_TYPE_SOURCE;
    if !is_any && !types.contains_key(source) {
        return Err(EvalError::UnknownType(source.to_string()));
    }

    let paths = list_note_paths(vault_root)?;
    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let note = match markdown::parse_document(&path, &content) {
            Ok(n) => n,
            Err(_) => continue,
        };
        if is_any {
            out.push(note);
            continue;
        }
        let rel = path.strip_prefix(vault_root).unwrap_or(&path);
        let matched = match_types(rel, &note.frontmatter, types, config);
        if matched.iter().any(|t| t.name == source) {
            out.push(note);
        }
    }
    Ok(out)
}

fn list_note_paths(vault_root: &Path) -> Result<Vec<PathBuf>, EvalError> {
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(vault_root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            !name.starts_with('.') && name != "_attachments"
        })
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("md") {
                let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if fname != "_index.md" {
                    paths.push(p.to_path_buf());
                }
            }
        }
    }
    Ok(paths)
}

/// Row context passed to expression evaluation: exposes a note's frontmatter,
/// synthetic columns (`path`, `title`, `tags`), and relative path for output.
pub(crate) struct RowCtx<'a> {
    pub note: &'a Note,
}

impl<'a> RowCtx<'a> {
    pub fn new(note: &'a Note) -> Self {
        RowCtx { note }
    }

    pub fn resolve(&self, name: &str) -> Value {
        match name {
            "path" => Value::String(self.note.path.display().to_string()),
            "title" => Value::String(self.note.title.clone()),
            "tags" => Value::List(
                self.note
                    .tags
                    .iter()
                    .map(|t| Value::String(t.clone()))
                    .collect(),
            ),
            other => match self.note.frontmatter.get(other) {
                Some(v) => Value::from_yaml(v),
                None => Value::Null,
            },
        }
    }
}

fn resolve_columns(projs: &[Projection]) -> Vec<String> {
    let mut out = Vec::new();
    for p in projs {
        match p {
            Projection::Wildcard => out.push("*".to_string()),
            Projection::Expr { expr, alias } => {
                out.push(alias.clone().unwrap_or_else(|| expr_label(expr)));
            }
        }
    }
    out
}

fn expr_label(e: &Expr) -> String {
    match e {
        Expr::Column(name) => name.clone(),
        Expr::Function { name, .. } => name.to_uppercase(),
        _ => "_expr".to_string(),
    }
}

fn project_row(
    note: &Note,
    projs: &[Projection],
    vault_root: &Path,
) -> Result<QueryResultRow, EvalError> {
    let ctx = RowCtx::new(note);
    let mut cells = Vec::new();
    for p in projs {
        match p {
            Projection::Wildcard => {
                // For `SELECT *`, emit a single aggregated cell holding the
                // frontmatter. CLI/API callers can flatten as they see fit.
                // For Phase 3 we render it as a compact string.
                let pairs: Vec<String> = note
                    .frontmatter
                    .iter()
                    .map(|(k, v)| format!("{k}={}", Value::from_yaml(v)))
                    .collect();
                cells.push(Value::String(pairs.join(", ")));
            }
            Projection::Expr { expr, .. } => {
                cells.push(eval_expr(expr, &ctx)?);
            }
        }
    }
    let rel = note.path.strip_prefix(vault_root).unwrap_or(&note.path);
    Ok(QueryResultRow {
        path: rel.to_path_buf(),
        cells,
    })
}

fn apply_order_by(
    rows: &mut [QueryResultRow],
    order_by: &[OrderBy],
    notes: &[Note],
    _vault_root: &Path,
) -> Result<(), EvalError> {
    if order_by.is_empty() {
        return Ok(());
    }

    // Precompute sort keys for each note so ordering doesn't re-evaluate
    // expressions per comparison.
    let mut keyed: Vec<(Vec<Value>, &QueryResultRow)> = rows
        .iter()
        .zip(notes.iter())
        .map(|(row, note)| {
            let ctx = RowCtx::new(note);
            let keys = order_by
                .iter()
                .map(|o| eval_expr(&o.expr, &ctx))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((keys, row))
        })
        .collect::<Result<Vec<_>, EvalError>>()?;

    keyed.sort_by(|a, b| {
        for ((ka, kb), ob) in a.0.iter().zip(b.0.iter()).zip(order_by.iter()) {
            let ord = ka.cmp_for_order(kb);
            if ord != std::cmp::Ordering::Equal {
                return if ob.order == SortOrder::Desc {
                    ord.reverse()
                } else {
                    ord
                };
            }
        }
        std::cmp::Ordering::Equal
    });

    let sorted: Vec<QueryResultRow> = keyed.into_iter().map(|(_, r)| r.clone()).collect();
    for (slot, new_row) in rows.iter_mut().zip(sorted.into_iter()) {
        *slot = new_row;
    }
    Ok(())
}

fn apply_pagination(rows: &mut Vec<QueryResultRow>, offset: usize, limit: Option<usize>) {
    if offset > 0 {
        if offset >= rows.len() {
            rows.clear();
            return;
        }
        rows.drain(..offset);
    }
    if let Some(n) = limit
        && rows.len() > n
    {
        rows.truncate(n);
    }
}

/// Evaluate a predicate expression. NULL → false for filtering (standard SQL:
/// `WHERE x` filters out NULL rows).
fn eval_predicate(expr: &Expr, ctx: &RowCtx<'_>) -> Result<bool, EvalError> {
    let v = eval_expr(expr, ctx)?;
    Ok(v.is_truthy())
}

pub(crate) fn eval_expr(expr: &Expr, ctx: &RowCtx<'_>) -> Result<Value, EvalError> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Column(name) => Ok(ctx.resolve(name)),
        Expr::Unary { op, expr } => {
            let v = eval_expr(expr, ctx)?;
            match op {
                UnaryOp::Not => Ok(Value::Bool(!v.is_truthy())),
                UnaryOp::Neg => match v {
                    Value::Integer(n) => Ok(Value::Integer(-n)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    Value::Null => Ok(Value::Null),
                    other => Err(EvalError::TypeMismatch(format!(
                        "cannot negate {}",
                        other.type_name()
                    ))),
                },
            }
        }
        Expr::Binary { op, left, right } => {
            let l = eval_expr(left, ctx)?;
            let r = eval_expr(right, ctx)?;
            eval_binary(*op, &l, &r)
        }
        Expr::In { expr, list, negated } => {
            let v = eval_expr(expr, ctx)?;
            let mut any_match = false;
            for item in list {
                let rhs = eval_expr(item, ctx)?;
                if v.sql_eq(&rhs).unwrap_or(false) {
                    any_match = true;
                    break;
                }
            }
            Ok(Value::Bool(if *negated { !any_match } else { any_match }))
        }
        Expr::Between {
            expr,
            low,
            high,
            negated,
        } => {
            let v = eval_expr(expr, ctx)?;
            let lo = eval_expr(low, ctx)?;
            let hi = eval_expr(high, ctx)?;
            let in_range = v.cmp_for_order(&lo) != std::cmp::Ordering::Less
                && v.cmp_for_order(&hi) != std::cmp::Ordering::Greater;
            Ok(Value::Bool(if *negated { !in_range } else { in_range }))
        }
        Expr::IsNull { expr, negated } => {
            let v = eval_expr(expr, ctx)?;
            let b = v.is_null();
            Ok(Value::Bool(if *negated { !b } else { b }))
        }
        Expr::Like {
            expr,
            pattern,
            negated,
        } => {
            let v = eval_expr(expr, ctx)?;
            let p = eval_expr(pattern, ctx)?;
            let s = match v {
                Value::String(s) => s,
                Value::Null => return Ok(Value::Bool(false)),
                other => return Err(EvalError::TypeMismatch(format!(
                    "LIKE left operand must be string, got {}",
                    other.type_name()
                ))),
            };
            let pat = match p {
                Value::String(s) => s,
                other => return Err(EvalError::TypeMismatch(format!(
                    "LIKE pattern must be string, got {}",
                    other.type_name()
                ))),
            };
            let matched = like_match(&s, &pat);
            Ok(Value::Bool(if *negated { !matched } else { matched }))
        }
        Expr::Function { name, args } => {
            if functions::is_aggregate(name) {
                return Err(EvalError::TypeMismatch(format!(
                    "aggregate {} not allowed outside GROUP BY / aggregate SELECT",
                    name.to_ascii_uppercase()
                )));
            }
            let values: Vec<Value> = args
                .iter()
                .map(|a| eval_expr(a, ctx))
                .collect::<Result<_, _>>()?;
            functions::call_scalar(name, &values)
        }
    }
}

fn eval_binary(op: BinaryOp, l: &Value, r: &Value) -> Result<Value, EvalError> {
    match op {
        BinaryOp::And => Ok(Value::Bool(l.is_truthy() && r.is_truthy())),
        BinaryOp::Or => Ok(Value::Bool(l.is_truthy() || r.is_truthy())),
        BinaryOp::Eq => Ok(Value::Bool(l.sql_eq(r).unwrap_or(false))),
        BinaryOp::NotEq => Ok(Value::Bool(!l.sql_eq(r).unwrap_or(true))),
        BinaryOp::Lt => Ok(Value::Bool(l.cmp_for_order(r) == std::cmp::Ordering::Less)),
        BinaryOp::LtEq => Ok(Value::Bool(l.cmp_for_order(r) != std::cmp::Ordering::Greater)),
        BinaryOp::Gt => Ok(Value::Bool(l.cmp_for_order(r) == std::cmp::Ordering::Greater)),
        BinaryOp::GtEq => Ok(Value::Bool(l.cmp_for_order(r) != std::cmp::Ordering::Less)),
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
            arith(op, l, r).ok_or_else(|| {
                EvalError::TypeMismatch(format!(
                    "cannot apply {:?} to {}, {}",
                    op,
                    l.type_name(),
                    r.type_name()
                ))
            })
        }
    }
}

fn arith(op: BinaryOp, l: &Value, r: &Value) -> Option<Value> {
    match (l, r) {
        (Value::Integer(a), Value::Integer(b)) => match op {
            BinaryOp::Add => Some(Value::Integer(a + b)),
            BinaryOp::Sub => Some(Value::Integer(a - b)),
            BinaryOp::Mul => Some(Value::Integer(a * b)),
            BinaryOp::Div if *b != 0 => Some(Value::Integer(a / b)),
            _ => None,
        },
        (Value::Float(a), Value::Float(b)) => Some(match op {
            BinaryOp::Add => Value::Float(a + b),
            BinaryOp::Sub => Value::Float(a - b),
            BinaryOp::Mul => Value::Float(a * b),
            BinaryOp::Div => Value::Float(a / b),
            _ => return None,
        }),
        (Value::Integer(a), Value::Float(b)) | (Value::Float(b), Value::Integer(a)) => {
            let a = *a as f64;
            Some(match op {
                BinaryOp::Add => Value::Float(a + b),
                BinaryOp::Sub => Value::Float(a - b),
                BinaryOp::Mul => Value::Float(a * b),
                BinaryOp::Div => Value::Float(a / b),
                _ => return None,
            })
        }
        _ => None,
    }
}

/// SQL LIKE matching: `%` = any sequence, `_` = any single character.
/// Escape via `\`.
fn like_match(s: &str, pat: &str) -> bool {
    let mut regex = String::with_capacity(pat.len() + 4);
    regex.push('^');
    let mut chars = pat.chars();
    while let Some(c) = chars.next() {
        match c {
            '%' => regex.push_str(".*"),
            '_' => regex.push('.'),
            '\\' => {
                if let Some(esc) = chars.next() {
                    regex.push_str(&regex::escape(&esc.to_string()));
                }
            }
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }
    regex.push('$');
    match regex::Regex::new(&regex) {
        Ok(re) => re.is_match(s),
        Err(_) => false,
    }
}
