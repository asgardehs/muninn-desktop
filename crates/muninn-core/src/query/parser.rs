//! SQL → [`MuninnQuery`] translation.
//!
//! Wraps `sqlparser-rs` and accepts only the narrow subset the evaluator
//! supports. Anything outside that subset — DDL, DML, CTEs, UNION, subqueries,
//! JOINs (deferred to Phase 5), window functions — returns a `ParseError`
//! with a clear message.

use sqlparser::ast::{
    BinaryOperator as SqlBinOp, Expr as SqlExpr, FunctionArg, FunctionArgExpr,
    FunctionArguments, GroupByExpr, OrderByExpr, Query as SqlQuery, Select, SelectItem,
    SetExpr, Statement, TableFactor, UnaryOperator as SqlUnOp, Value as SqlValue,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use thiserror::Error;

use super::ast::{BinaryOp, Expr, MuninnQuery, OrderBy, Projection, SortOrder, UnaryOp};
use super::value::Value;
use super::{MAX_EXPR_DEPTH};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("sql parse error: {0}")]
    Sql(String),
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    #[error("expression too deeply nested (max {0})")]
    TooDeep(usize),
    #[error("invalid literal: {0}")]
    InvalidLiteral(String),
    #[error("FROM clause must name a single type")]
    BadFrom,
}

pub fn parse_query(sql: &str) -> Result<MuninnQuery, ParseError> {
    let dialect = GenericDialect {};
    let statements = Parser::parse_sql(&dialect, sql).map_err(|e| ParseError::Sql(e.to_string()))?;

    if statements.len() != 1 {
        return Err(ParseError::Unsupported(
            "exactly one statement required per query",
        ));
    }

    let query = match statements.into_iter().next().unwrap() {
        Statement::Query(q) => q,
        _ => return Err(ParseError::Unsupported("only SELECT statements are allowed")),
    };

    lower_query(*query)
}

fn lower_query(q: SqlQuery) -> Result<MuninnQuery, ParseError> {
    if q.with.is_some() {
        return Err(ParseError::Unsupported("CTE (WITH) not supported"));
    }
    if q.fetch.is_some() {
        return Err(ParseError::Unsupported("FETCH not supported"));
    }
    if !q.locks.is_empty() {
        return Err(ParseError::Unsupported("FOR UPDATE/SHARE not supported"));
    }

    let select = match *q.body {
        SetExpr::Select(s) => *s,
        SetExpr::Query(_) => return Err(ParseError::Unsupported("nested queries not supported")),
        SetExpr::SetOperation { .. } => {
            return Err(ParseError::Unsupported("UNION/INTERSECT/EXCEPT not supported"));
        }
        SetExpr::Values(_) => return Err(ParseError::Unsupported("VALUES not supported")),
        SetExpr::Insert(_) | SetExpr::Update(_) | SetExpr::Table(_) => {
            return Err(ParseError::Unsupported("only SELECT queries are allowed"));
        }
    };

    let from = lower_from(&select)?;
    let projections = lower_projections(&select)?;
    let filter = select
        .selection
        .as_ref()
        .map(|e| lower_expr(e, 0))
        .transpose()?;
    let group_by = lower_group_by(&select.group_by)?;
    let having = select
        .having
        .as_ref()
        .map(|e| lower_expr(e, 0))
        .transpose()?;

    if select.distinct.is_some() {
        return Err(ParseError::Unsupported("DISTINCT not supported"));
    }
    if select.qualify.is_some() {
        return Err(ParseError::Unsupported("QUALIFY not supported"));
    }
    if !select.named_window.is_empty() {
        return Err(ParseError::Unsupported("named windows not supported"));
    }
    if select.prewhere.is_some() {
        return Err(ParseError::Unsupported("PREWHERE not supported"));
    }

    let order_by = match q.order_by {
        Some(ob) => ob
            .exprs
            .iter()
            .map(lower_order_by)
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };

    let limit = q.limit.as_ref().map(expr_to_usize).transpose()?;
    let offset = q
        .offset
        .as_ref()
        .map(|o| expr_to_usize(&o.value))
        .transpose()?
        .unwrap_or(0);

    Ok(MuninnQuery {
        from,
        projections,
        filter,
        group_by,
        having,
        order_by,
        limit,
        offset,
    })
}

fn lower_from(select: &Select) -> Result<String, ParseError> {
    if select.from.len() != 1 {
        return Err(ParseError::Unsupported(
            "multi-table FROM and JOINs arrive in Phase 5",
        ));
    }
    let twj = &select.from[0];
    if !twj.joins.is_empty() {
        return Err(ParseError::Unsupported(
            "JOINs are not yet supported (Phase 5)",
        ));
    }
    match &twj.relation {
        TableFactor::Table { name, .. } => {
            if name.0.len() != 1 {
                return Err(ParseError::BadFrom);
            }
            Ok(name.0[0].value.clone())
        }
        _ => Err(ParseError::BadFrom),
    }
}

fn lower_projections(select: &Select) -> Result<Vec<Projection>, ParseError> {
    let mut out = Vec::with_capacity(select.projection.len());
    for item in &select.projection {
        match item {
            SelectItem::Wildcard(_) => out.push(Projection::Wildcard),
            SelectItem::UnnamedExpr(e) => out.push(Projection::Expr {
                expr: lower_expr(e, 0)?,
                alias: None,
            }),
            SelectItem::ExprWithAlias { expr, alias } => out.push(Projection::Expr {
                expr: lower_expr(expr, 0)?,
                alias: Some(alias.value.clone()),
            }),
            SelectItem::QualifiedWildcard(_, _) => {
                return Err(ParseError::Unsupported("qualified wildcard not supported"));
            }
        }
    }
    Ok(out)
}

fn lower_group_by(g: &GroupByExpr) -> Result<Vec<Expr>, ParseError> {
    match g {
        GroupByExpr::All(_) => Err(ParseError::Unsupported("GROUP BY ALL not supported")),
        GroupByExpr::Expressions(exprs, modifiers) => {
            if !modifiers.is_empty() {
                return Err(ParseError::Unsupported(
                    "GROUP BY modifiers (ROLLUP/CUBE) not supported",
                ));
            }
            exprs.iter().map(|e| lower_expr(e, 0)).collect()
        }
    }
}

fn lower_order_by(o: &OrderByExpr) -> Result<OrderBy, ParseError> {
    let expr = lower_expr(&o.expr, 0)?;
    let order = match o.asc {
        Some(false) => SortOrder::Desc,
        _ => SortOrder::Asc,
    };
    Ok(OrderBy { expr, order })
}

fn lower_expr(e: &SqlExpr, depth: usize) -> Result<Expr, ParseError> {
    if depth > MAX_EXPR_DEPTH {
        return Err(ParseError::TooDeep(MAX_EXPR_DEPTH));
    }
    let d = depth + 1;
    match e {
        SqlExpr::Identifier(id) => Ok(Expr::Column(id.value.clone())),
        SqlExpr::CompoundIdentifier(parts) => {
            // Phase 3: `table.column` collapses to just `column`. Runestone
            // dotted relation access (Phase 5) will route through here.
            let last = parts
                .last()
                .ok_or(ParseError::Unsupported("empty compound identifier"))?;
            Ok(Expr::Column(last.value.clone()))
        }
        SqlExpr::Value(v) => Ok(Expr::Literal(lower_literal(v)?)),
        SqlExpr::TypedString { data_type: _, value } => Ok(Expr::Literal(
            super::value::Value::from_yaml(&serde_yaml::Value::String(value.clone())),
        )),
        SqlExpr::Nested(inner) => lower_expr(inner, d),
        SqlExpr::UnaryOp { op, expr } => {
            let op = match op {
                SqlUnOp::Not => UnaryOp::Not,
                SqlUnOp::Minus => UnaryOp::Neg,
                SqlUnOp::Plus => return lower_expr(expr, d),
                _ => return Err(ParseError::Unsupported("unsupported unary operator")),
            };
            Ok(Expr::Unary {
                op,
                expr: Box::new(lower_expr(expr, d)?),
            })
        }
        SqlExpr::BinaryOp { left, op, right } => {
            let op = lower_binop(op)?;
            Ok(Expr::Binary {
                op,
                left: Box::new(lower_expr(left, d)?),
                right: Box::new(lower_expr(right, d)?),
            })
        }
        SqlExpr::IsNull(inner) => Ok(Expr::IsNull {
            expr: Box::new(lower_expr(inner, d)?),
            negated: false,
        }),
        SqlExpr::IsNotNull(inner) => Ok(Expr::IsNull {
            expr: Box::new(lower_expr(inner, d)?),
            negated: true,
        }),
        SqlExpr::InList { expr, list, negated } => {
            let list = list
                .iter()
                .map(|x| lower_expr(x, d))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::In {
                expr: Box::new(lower_expr(expr, d)?),
                list,
                negated: *negated,
            })
        }
        SqlExpr::Between {
            expr,
            low,
            high,
            negated,
        } => Ok(Expr::Between {
            expr: Box::new(lower_expr(expr, d)?),
            low: Box::new(lower_expr(low, d)?),
            high: Box::new(lower_expr(high, d)?),
            negated: *negated,
        }),
        SqlExpr::Like {
            negated,
            any,
            expr,
            pattern,
            escape_char: _,
        } => {
            if *any {
                return Err(ParseError::Unsupported("LIKE ANY not supported"));
            }
            Ok(Expr::Like {
                expr: Box::new(lower_expr(expr, d)?),
                pattern: Box::new(lower_expr(pattern, d)?),
                negated: *negated,
            })
        }
        SqlExpr::Function(func) => lower_function(func, d),
        SqlExpr::InSubquery { .. } => Err(ParseError::Unsupported("subqueries not supported")),
        _ => Err(ParseError::Unsupported("unsupported expression")),
    }
}

fn lower_function(f: &sqlparser::ast::Function, depth: usize) -> Result<Expr, ParseError> {
    if f.over.is_some() {
        return Err(ParseError::Unsupported("window functions not supported"));
    }
    if f.filter.is_some() {
        return Err(ParseError::Unsupported("FILTER clause not supported"));
    }
    if !matches!(f.parameters, FunctionArguments::None) {
        return Err(ParseError::Unsupported(
            "function parameters (ClickHouse syntax) not supported",
        ));
    }

    let name = f
        .name
        .0
        .iter()
        .map(|i| i.value.clone())
        .collect::<Vec<_>>()
        .join(".");

    let args = match &f.args {
        FunctionArguments::None => Vec::new(),
        FunctionArguments::Subquery(_) => {
            return Err(ParseError::Unsupported("subquery arguments not supported"));
        }
        FunctionArguments::List(list) => {
            let mut out = Vec::with_capacity(list.args.len());
            for a in &list.args {
                let expr = match a {
                    FunctionArg::Named { .. } => {
                        return Err(ParseError::Unsupported("named arguments not supported"));
                    }
                    FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => lower_expr(e, depth)?,
                    FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => {
                        // COUNT(*) → COUNT with a single "__wildcard__" marker.
                        Expr::Column("*".to_string())
                    }
                    FunctionArg::Unnamed(FunctionArgExpr::QualifiedWildcard(_)) => {
                        return Err(ParseError::Unsupported(
                            "qualified wildcard in function args not supported",
                        ));
                    }
                };
                out.push(expr);
            }
            out
        }
    };

    Ok(Expr::Function { name, args })
}

fn lower_binop(op: &SqlBinOp) -> Result<BinaryOp, ParseError> {
    Ok(match op {
        SqlBinOp::Plus => BinaryOp::Add,
        SqlBinOp::Minus => BinaryOp::Sub,
        SqlBinOp::Multiply => BinaryOp::Mul,
        SqlBinOp::Divide => BinaryOp::Div,
        SqlBinOp::Eq => BinaryOp::Eq,
        SqlBinOp::NotEq => BinaryOp::NotEq,
        SqlBinOp::Lt => BinaryOp::Lt,
        SqlBinOp::LtEq => BinaryOp::LtEq,
        SqlBinOp::Gt => BinaryOp::Gt,
        SqlBinOp::GtEq => BinaryOp::GtEq,
        SqlBinOp::And => BinaryOp::And,
        SqlBinOp::Or => BinaryOp::Or,
        _ => return Err(ParseError::Unsupported("unsupported binary operator")),
    })
}

fn lower_literal(v: &SqlValue) -> Result<Value, ParseError> {
    match v {
        SqlValue::Number(s, _) => {
            if let Ok(i) = s.parse::<i64>() {
                Ok(Value::Integer(i))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Value::Float(f))
            } else {
                Err(ParseError::InvalidLiteral(s.clone()))
            }
        }
        SqlValue::SingleQuotedString(s)
        | SqlValue::DoubleQuotedString(s)
        | SqlValue::TripleSingleQuotedString(s)
        | SqlValue::TripleDoubleQuotedString(s)
        | SqlValue::NationalStringLiteral(s) => Ok(Value::from_yaml(&serde_yaml::Value::String(s.clone()))),
        SqlValue::Boolean(b) => Ok(Value::Bool(*b)),
        SqlValue::Null => Ok(Value::Null),
        _ => Err(ParseError::Unsupported("unsupported literal form")),
    }
}

fn expr_to_usize(e: &SqlExpr) -> Result<usize, ParseError> {
    match e {
        SqlExpr::Value(SqlValue::Number(s, _)) => s
            .parse::<usize>()
            .map_err(|_| ParseError::InvalidLiteral(s.clone())),
        _ => Err(ParseError::Unsupported(
            "LIMIT/OFFSET must be a literal integer",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_insert() {
        let err = parse_query("INSERT INTO note VALUES (1)").unwrap_err();
        assert!(matches!(err, ParseError::Unsupported(_) | ParseError::Sql(_)));
    }

    #[test]
    fn rejects_join() {
        let err = parse_query("SELECT * FROM note JOIN ref ON note.id = ref.id").unwrap_err();
        assert!(matches!(err, ParseError::Unsupported(_)));
    }

    #[test]
    fn rejects_subquery() {
        let err = parse_query("SELECT * FROM note WHERE id IN (SELECT id FROM ref)").unwrap_err();
        assert!(matches!(err, ParseError::Unsupported(_)));
    }

    #[test]
    fn parses_simple_select() {
        let q = parse_query("SELECT title, status FROM note WHERE status = 'active'").unwrap();
        assert_eq!(q.from, "note");
        assert_eq!(q.projections.len(), 2);
        assert!(q.filter.is_some());
    }

    #[test]
    fn parses_wildcard() {
        let q = parse_query("SELECT * FROM note").unwrap();
        assert!(matches!(q.projections[0], Projection::Wildcard));
    }

    #[test]
    fn parses_order_limit_offset() {
        let q = parse_query("SELECT title FROM note ORDER BY created DESC LIMIT 10 OFFSET 5").unwrap();
        assert_eq!(q.order_by.len(), 1);
        assert_eq!(q.order_by[0].order, SortOrder::Desc);
        assert_eq!(q.limit, Some(10));
        assert_eq!(q.offset, 5);
    }

    #[test]
    fn parses_in_and_between() {
        let q = parse_query(
            "SELECT title FROM note WHERE status IN ('active', 'pending') AND priority BETWEEN 1 AND 5",
        )
        .unwrap();
        assert!(q.filter.is_some());
    }
}
