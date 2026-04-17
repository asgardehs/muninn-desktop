//! Query execution against a vault.
//!
//! The evaluator walks notes matching the FROM clause, joins them per the
//! JOIN list (Phase 5), filters with WHERE, optionally GROUP BYs with
//! aggregates, projects SELECT columns, applies ORDER BY, and paginates via
//! LIMIT/OFFSET.
//!
//! A tuple is `Vec<Option<&Note>>`: one binding per alias in the FROM clause.
//! Single-source queries use tuples of length 1. LEFT JOIN bindings are
//! `None` when no right-side match was found — qualified references against a
//! `None` binding resolve to `Value::Null`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use thiserror::Error;

use crate::markdown::{self, Note};
use crate::mdbase::match_type::match_types;
use crate::mdbase::types::TypeDef;

use super::ast::{
    BinaryOp, Expr, FromClause, JoinKind, MuninnQuery, OrderBy, Projection, SortOrder, UnaryOp,
};
use super::functions;
use super::value::Value;
use super::{ANY_TYPE_SOURCE, MAX_RESULT_ROWS};

/// Maximum recursion depth when resolving computed fields.
pub const MAX_COMPUTED_DEPTH: usize = 16;

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("unknown type: {0}")]
    UnknownType(String),
    #[error("unknown alias: {0}")]
    UnknownAlias(String),
    #[error("type mismatch in expression: {0}")]
    TypeMismatch(String),
    #[error("unknown function: {0}")]
    UnknownFunction(String),
    #[error("result set exceeds max rows ({0})")]
    ResultTooLarge(usize),
    #[error("computed field '{0}' recurses too deeply")]
    ComputedTooDeep(String),
    #[error("computed field '{field}' parse error: {source}")]
    ComputedParse {
        field: String,
        #[source]
        source: super::parser::ParseError,
    },
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

/// Cache of parsed computed-field expressions keyed by type name → field
/// name. Built once per `execute` call so each type's `computed:` map only
/// parses its SQL snippets once.
pub(crate) type ComputedCache = HashMap<String, IndexMap<String, Expr>>;

fn build_computed_cache(types: &HashMap<String, TypeDef>) -> Result<ComputedCache, EvalError> {
    let mut cache: ComputedCache = HashMap::new();
    for (tn, td) in types {
        if td.computed.is_empty() {
            continue;
        }
        let mut parsed = IndexMap::new();
        for (field, raw) in &td.computed {
            let expr = super::parser::parse_expr(raw).map_err(|e| EvalError::ComputedParse {
                field: format!("{tn}.{field}"),
                source: e,
            })?;
            parsed.insert(field.clone(), expr);
        }
        cache.insert(tn.clone(), parsed);
    }
    Ok(cache)
}

/// Execute a parsed query against a vault rooted at `vault_root`.
pub fn execute(
    vault_root: &Path,
    types: &HashMap<String, TypeDef>,
    config: Option<&crate::mdbase::config::MdbaseConfig>,
    query: &MuninnQuery,
) -> Result<QueryResultSet, EvalError> {
    let computed = build_computed_cache(types)?;

    // Load per-alias note pools.
    let mut per_alias: Vec<Vec<Note>> = Vec::with_capacity(1 + query.from.joins.len());
    per_alias.push(load_source_rows(
        vault_root,
        types,
        config,
        &query.from.primary.type_name,
    )?);
    for j in &query.from.joins {
        per_alias.push(load_source_rows(
            vault_root,
            types,
            config,
            &j.right.type_name,
        )?);
    }

    let tuples = build_join_tuples(&per_alias, &query.from, types, &computed, vault_root)?;

    let filtered: Vec<Vec<Option<&Note>>> = match &query.filter {
        Some(expr) => {
            let mut out = Vec::with_capacity(tuples.len());
            for t in &tuples {
                let ctx = RowCtx::new(&query.from, t, types, &computed, vault_root);
                if eval_predicate(expr, &ctx)? {
                    out.push(t.clone());
                }
            }
            out
        }
        None => tuples,
    };

    let columns = resolve_columns(&query.projections);

    let grouped = !query.group_by.is_empty()
        || query.having.is_some()
        || projections_have_aggregate(&query.projections);

    let mut rows = if grouped {
        execute_grouped(&filtered, query, types, &computed, vault_root)?
    } else {
        execute_simple(&filtered, query, types, &computed, vault_root)?
    };

    apply_pagination(&mut rows, query.offset, query.limit);

    if rows.len() > MAX_RESULT_ROWS {
        return Err(EvalError::ResultTooLarge(MAX_RESULT_ROWS));
    }

    Ok(QueryResultSet { columns, rows })
}

fn build_join_tuples<'a>(
    per_alias: &'a [Vec<Note>],
    from: &FromClause,
    types: &HashMap<String, TypeDef>,
    computed: &ComputedCache,
    vault_root: &Path,
) -> Result<Vec<Vec<Option<&'a Note>>>, EvalError> {
    let alias_count = 1 + from.joins.len();

    // Seed with one tuple per primary row; pad with Nones for later aliases
    // so RowCtx indexing stays uniform while we evaluate each ON predicate.
    let mut tuples: Vec<Vec<Option<&Note>>> = per_alias[0]
        .iter()
        .map(|n| {
            let mut v: Vec<Option<&Note>> = Vec::with_capacity(alias_count);
            v.push(Some(n));
            for _ in 1..alias_count {
                v.push(None);
            }
            v
        })
        .collect();

    for (j_idx, join) in from.joins.iter().enumerate() {
        let right_alias_idx = j_idx + 1;
        let right_notes = &per_alias[right_alias_idx];
        let mut next: Vec<Vec<Option<&Note>>> = Vec::new();

        for t in &tuples {
            let mut matched = false;
            for rn in right_notes {
                let mut candidate = t.clone();
                candidate[right_alias_idx] = Some(rn);
                let ctx = RowCtx::new(from, &candidate, types, computed, vault_root);
                if eval_predicate(&join.on, &ctx)? {
                    next.push(candidate);
                    matched = true;
                }
            }
            if !matched && join.kind == JoinKind::Left {
                let mut left = t.clone();
                left[right_alias_idx] = None;
                next.push(left);
            }
        }

        tuples = next;
    }

    Ok(tuples)
}

fn execute_simple(
    filtered: &[Vec<Option<&Note>>],
    query: &MuninnQuery,
    types: &HashMap<String, TypeDef>,
    computed: &ComputedCache,
    vault_root: &Path,
) -> Result<Vec<QueryResultRow>, EvalError> {
    let mut rows: Vec<QueryResultRow> = Vec::with_capacity(filtered.len());
    for t in filtered {
        let ctx = RowCtx::new(&query.from, t, types, computed, vault_root);
        rows.push(project_row(&ctx, &query.projections, vault_root)?);
    }

    let resolved_order: Vec<OrderBy> = query
        .order_by
        .iter()
        .map(|ob| OrderBy {
            expr: resolve_alias(&ob.expr, &query.projections),
            order: ob.order,
        })
        .collect();

    apply_order_by(
        &mut rows,
        filtered,
        &resolved_order,
        &query.from,
        types,
        computed,
        vault_root,
    )?;
    Ok(rows)
}

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

fn execute_grouped(
    filtered: &[Vec<Option<&Note>>],
    query: &MuninnQuery,
    types: &HashMap<String, TypeDef>,
    computed: &ComputedCache,
    vault_root: &Path,
) -> Result<Vec<QueryResultRow>, EvalError> {
    let groups = build_groups(
        filtered,
        &query.group_by,
        &query.from,
        types,
        computed,
        vault_root,
    )?;

    let mut rows: Vec<(Vec<Value>, QueryResultRow)> = Vec::with_capacity(groups.len());
    for group in &groups {
        if group.tuples.is_empty() {
            continue;
        }

        if let Some(expr) = &query.having {
            let keep =
                eval_in_group(expr, group, &query.from, types, computed, vault_root)?.is_truthy();
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
                    cells.push(eval_in_group(
                        expr,
                        group,
                        &query.from,
                        types,
                        computed,
                        vault_root,
                    )?);
                }
            }
        }

        let sort_keys: Vec<Value> = query
            .order_by
            .iter()
            .map(|ob| {
                eval_in_group(
                    &resolve_alias(&ob.expr, &query.projections),
                    group,
                    &query.from,
                    types,
                    computed,
                    vault_root,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let first_tuple = group.tuples[0];
        let first_note = first_tuple[0].expect("primary binding always present");
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
    tuples: Vec<&'a [Option<&'a Note>]>,
}

fn build_groups<'a>(
    filtered: &'a [Vec<Option<&'a Note>>],
    group_by: &[Expr],
    from: &FromClause,
    types: &HashMap<String, TypeDef>,
    computed: &ComputedCache,
    vault_root: &Path,
) -> Result<Vec<Group<'a>>, EvalError> {
    if group_by.is_empty() {
        return Ok(vec![Group {
            key: Vec::new(),
            tuples: filtered.iter().map(|t| t.as_slice()).collect(),
        }]);
    }

    let mut groups: Vec<Group<'a>> = Vec::new();
    for tuple in filtered {
        let ctx = RowCtx::new(from, tuple, types, computed, vault_root);
        let key: Vec<Value> = group_by
            .iter()
            .map(|e| eval_expr(e, &ctx))
            .collect::<Result<Vec<_>, _>>()?;

        if let Some(existing) = groups.iter_mut().find(|g| keys_equal(&g.key, &key)) {
            existing.tuples.push(tuple.as_slice());
        } else {
            groups.push(Group {
                key,
                tuples: vec![tuple.as_slice()],
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
        Expr::Like { expr, pattern, .. } => expr_has_aggregate(expr) || expr_has_aggregate(pattern),
        Expr::Literal(_) | Expr::Column(_) | Expr::QualifiedColumn { .. } => false,
    }
}

fn eval_in_group(
    expr: &Expr,
    group: &Group<'_>,
    from: &FromClause,
    types: &HashMap<String, TypeDef>,
    computed: &ComputedCache,
    vault_root: &Path,
) -> Result<Value, EvalError> {
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
                group.tuples.iter().map(|_| Value::Null).collect()
            } else {
                group
                    .tuples
                    .iter()
                    .map(|t| {
                        let ctx = RowCtx::new(from, t, types, computed, vault_root);
                        eval_expr(&args[0], &ctx)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            functions::fold_aggregate(name, &per_row, is_star)
        }
        Expr::Function { name, args } => {
            let values: Vec<Value> = args
                .iter()
                .map(|a| eval_in_group(a, group, from, types, computed, vault_root))
                .collect::<Result<Vec<_>, _>>()?;
            functions::call_scalar(name, &values)
        }
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Column(_) | Expr::QualifiedColumn { .. } => {
            let first = group
                .tuples
                .first()
                .ok_or_else(|| EvalError::TypeMismatch("empty group".to_string()))?;
            let ctx = RowCtx::new(from, first, types, computed, vault_root);
            eval_expr(expr, &ctx)
        }
        Expr::Unary { op, expr } => {
            let v = eval_in_group(expr, group, from, types, computed, vault_root)?;
            apply_unary(*op, &v)
        }
        Expr::Binary { op, left, right } => {
            let l = eval_in_group(left, group, from, types, computed, vault_root)?;
            let r = eval_in_group(right, group, from, types, computed, vault_root)?;
            eval_binary(*op, &l, &r)
        }
        Expr::In {
            expr,
            list,
            negated,
        } => {
            let v = eval_in_group(expr, group, from, types, computed, vault_root)?;
            let mut any_match = false;
            for item in list {
                let rhs = eval_in_group(item, group, from, types, computed, vault_root)?;
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
            let v = eval_in_group(expr, group, from, types, computed, vault_root)?;
            let lo = eval_in_group(low, group, from, types, computed, vault_root)?;
            let hi = eval_in_group(high, group, from, types, computed, vault_root)?;
            let in_range = v.cmp_for_order(&lo) != std::cmp::Ordering::Less
                && v.cmp_for_order(&hi) != std::cmp::Ordering::Greater;
            Ok(Value::Bool(if *negated { !in_range } else { in_range }))
        }
        Expr::IsNull { expr, negated } => {
            let v = eval_in_group(expr, group, from, types, computed, vault_root)?;
            let b = v.is_null();
            Ok(Value::Bool(if *negated { !b } else { b }))
        }
        Expr::Like {
            expr,
            pattern,
            negated,
        } => {
            let v = eval_in_group(expr, group, from, types, computed, vault_root)?;
            let p = eval_in_group(pattern, group, from, types, computed, vault_root)?;
            apply_like(&v, &p, *negated)
        }
    }
}

fn load_source_rows(
    vault_root: &Path,
    types: &HashMap<String, TypeDef>,
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
            // Always descend into the root — its name may legitimately start
            // with `.` (e.g. test tempdirs, vaults tucked under dotdirs).
            if e.depth() == 0 {
                return true;
            }
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

/// Row context passed to expression evaluation. Holds one optional `&Note`
/// binding per alias in the query's FROM clause; unqualified column
/// references resolve on the primary (index 0) binding, qualified references
/// (`alias.column`) look up by alias.
pub(crate) struct RowCtx<'a> {
    pub from: &'a FromClause,
    pub bindings: &'a [Option<&'a Note>],
    pub types: &'a HashMap<String, TypeDef>,
    pub computed: &'a ComputedCache,
    pub vault_root: &'a Path,
    pub depth: usize,
}

impl<'a> RowCtx<'a> {
    pub fn new(
        from: &'a FromClause,
        bindings: &'a [Option<&'a Note>],
        types: &'a HashMap<String, TypeDef>,
        computed: &'a ComputedCache,
        vault_root: &'a Path,
    ) -> Self {
        Self {
            from,
            bindings,
            types,
            computed,
            vault_root,
            depth: 0,
        }
    }

    pub fn primary_note(&self) -> Option<&'a Note> {
        self.bindings.first().copied().flatten()
    }

    pub fn resolve(&self, column: &str) -> Result<Value, EvalError> {
        match self.primary_note() {
            Some(n) => resolve_on(n, &self.from.primary.type_name, self, column),
            None => Ok(Value::Null),
        }
    }

    fn rel_path(&self, note: &Note) -> String {
        note.path
            .strip_prefix(self.vault_root)
            .unwrap_or(&note.path)
            .display()
            .to_string()
    }

    pub fn resolve_qualified(&self, alias: &str, column: &str) -> Result<Value, EvalError> {
        let aliases = self.from.aliases();
        let idx = aliases
            .iter()
            .position(|a| *a == alias)
            .ok_or_else(|| EvalError::UnknownAlias(alias.to_string()))?;

        let type_name = if idx == 0 {
            &self.from.primary.type_name
        } else {
            &self.from.joins[idx - 1].right.type_name
        };

        match self.bindings.get(idx).and_then(|b| *b) {
            Some(n) => resolve_on(n, type_name, self, column),
            None => Ok(Value::Null),
        }
    }
}

fn resolve_on(
    note: &Note,
    type_name: &str,
    ctx: &RowCtx<'_>,
    column: &str,
) -> Result<Value, EvalError> {
    match column {
        "path" => return Ok(Value::String(ctx.rel_path(note))),
        "title" => return Ok(Value::String(note.title.clone())),
        "tags" => {
            return Ok(Value::List(
                note.tags.iter().map(|t| Value::String(t.clone())).collect(),
            ));
        }
        _ => {}
    }
    if let Some(v) = note.frontmatter.get(column) {
        return Ok(Value::from_yaml(v));
    }
    // Computed fields on this binding's type.
    if let Some(type_computed) = ctx.computed.get(type_name)
        && let Some(expr) = type_computed.get(column)
    {
        if ctx.depth >= MAX_COMPUTED_DEPTH {
            return Err(EvalError::ComputedTooDeep(column.to_string()));
        }
        let sub_from = FromClause::single(type_name);
        let sub_bindings: Vec<Option<&Note>> = vec![Some(note)];
        let sub_ctx = RowCtx {
            from: &sub_from,
            bindings: &sub_bindings,
            types: ctx.types,
            computed: ctx.computed,
            vault_root: ctx.vault_root,
            depth: ctx.depth + 1,
        };
        return eval_expr(expr, &sub_ctx);
    }
    Ok(Value::Null)
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
        Expr::QualifiedColumn { table, column } => format!("{table}.{column}"),
        Expr::Function { name, .. } => name.to_uppercase(),
        _ => "_expr".to_string(),
    }
}

fn project_row(
    ctx: &RowCtx<'_>,
    projs: &[Projection],
    vault_root: &Path,
) -> Result<QueryResultRow, EvalError> {
    let mut cells = Vec::new();
    for p in projs {
        match p {
            Projection::Wildcard => {
                let pairs: Vec<String> = match ctx.primary_note() {
                    Some(n) => n
                        .frontmatter
                        .iter()
                        .map(|(k, v)| format!("{k}={}", Value::from_yaml(v)))
                        .collect(),
                    None => Vec::new(),
                };
                cells.push(Value::String(pairs.join(", ")));
            }
            Projection::Expr { expr, .. } => {
                cells.push(eval_expr(expr, ctx)?);
            }
        }
    }
    let rel = match ctx.primary_note() {
        Some(n) => n
            .path
            .strip_prefix(vault_root)
            .unwrap_or(&n.path)
            .to_path_buf(),
        None => PathBuf::new(),
    };
    Ok(QueryResultRow { path: rel, cells })
}

fn apply_order_by(
    rows: &mut [QueryResultRow],
    tuples: &[Vec<Option<&Note>>],
    order_by: &[OrderBy],
    from: &FromClause,
    types: &HashMap<String, TypeDef>,
    computed: &ComputedCache,
    vault_root: &Path,
) -> Result<(), EvalError> {
    if order_by.is_empty() {
        return Ok(());
    }

    let mut keyed: Vec<(Vec<Value>, &QueryResultRow)> = rows
        .iter()
        .zip(tuples.iter())
        .map(|(row, tuple)| {
            let ctx = RowCtx::new(from, tuple, types, computed, vault_root);
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

fn eval_predicate(expr: &Expr, ctx: &RowCtx<'_>) -> Result<bool, EvalError> {
    let v = eval_expr(expr, ctx)?;
    Ok(v.is_truthy())
}

pub(crate) fn eval_expr(expr: &Expr, ctx: &RowCtx<'_>) -> Result<Value, EvalError> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Column(name) => ctx.resolve(name),
        Expr::QualifiedColumn { table, column } => ctx.resolve_qualified(table, column),
        Expr::Unary { op, expr } => {
            let v = eval_expr(expr, ctx)?;
            apply_unary(*op, &v)
        }
        Expr::Binary { op, left, right } => {
            let l = eval_expr(left, ctx)?;
            let r = eval_expr(right, ctx)?;
            eval_binary(*op, &l, &r)
        }
        Expr::In {
            expr,
            list,
            negated,
        } => {
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
            apply_like(&v, &p, *negated)
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

fn apply_unary(op: UnaryOp, v: &Value) -> Result<Value, EvalError> {
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

fn apply_like(v: &Value, p: &Value, negated: bool) -> Result<Value, EvalError> {
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Null => return Ok(Value::Bool(false)),
        other => {
            return Err(EvalError::TypeMismatch(format!(
                "LIKE left operand must be string, got {}",
                other.type_name()
            )));
        }
    };
    let pat = match p {
        Value::String(s) => s.clone(),
        other => {
            return Err(EvalError::TypeMismatch(format!(
                "LIKE pattern must be string, got {}",
                other.type_name()
            )));
        }
    };
    let matched = like_match(&s, &pat);
    Ok(Value::Bool(if negated { !matched } else { matched }))
}

fn eval_binary(op: BinaryOp, l: &Value, r: &Value) -> Result<Value, EvalError> {
    match op {
        BinaryOp::And => Ok(Value::Bool(l.is_truthy() && r.is_truthy())),
        BinaryOp::Or => Ok(Value::Bool(l.is_truthy() || r.is_truthy())),
        BinaryOp::Eq => Ok(Value::Bool(l.sql_eq(r).unwrap_or(false))),
        BinaryOp::NotEq => Ok(Value::Bool(!l.sql_eq(r).unwrap_or(true))),
        BinaryOp::Lt => Ok(Value::Bool(l.cmp_for_order(r) == std::cmp::Ordering::Less)),
        BinaryOp::LtEq => Ok(Value::Bool(
            l.cmp_for_order(r) != std::cmp::Ordering::Greater,
        )),
        BinaryOp::Gt => Ok(Value::Bool(
            l.cmp_for_order(r) == std::cmp::Ordering::Greater,
        )),
        BinaryOp::GtEq => Ok(Value::Bool(l.cmp_for_order(r) != std::cmp::Ordering::Less)),
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => arith(op, l, r)
            .ok_or_else(|| {
                EvalError::TypeMismatch(format!(
                    "cannot apply {:?} to {}, {}",
                    op,
                    l.type_name(),
                    r.type_name()
                ))
            }),
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
