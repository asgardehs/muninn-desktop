//! Typed value used by the query evaluator.
//!
//! Distinct from `serde_yaml::Value` so the evaluator has real SQL semantics:
//! dates and datetimes are tagged, numbers split into integer/float, and
//! coercion rules are explicit. Conversion to/from `serde_yaml::Value` is
//! lossy only for YAML-specific constructs (tagged values, anchors) that the
//! evaluator has no use for.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Date(NaiveDate),
    DateTime(DateTime<Utc>),
    Time(NaiveTime),
    List(Vec<Value>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Date(_) => "date",
            Value::DateTime(_) => "datetime",
            Value::Time(_) => "time",
            Value::List(_) => "list",
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Integer(n) => *n != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Date(_) | Value::DateTime(_) | Value::Time(_) => true,
        }
    }

    /// Convert a `serde_yaml::Value` to a `Value`. Unknown tagged values and
    /// mappings collapse to `Null` — callers that care can check `type_name`.
    pub fn from_yaml(v: &serde_yaml::Value) -> Self {
        match v {
            serde_yaml::Value::Null => Value::Null,
            serde_yaml::Value::Bool(b) => Value::Bool(*b),
            serde_yaml::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_yaml::Value::String(s) => coerce_string(s),
            serde_yaml::Value::Sequence(seq) => {
                Value::List(seq.iter().map(Value::from_yaml).collect())
            }
            serde_yaml::Value::Mapping(_) | serde_yaml::Value::Tagged(_) => Value::Null,
        }
    }

    /// Serialize to JSON for API and CLI output.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Integer(i) => serde_json::Value::Number((*i).into()),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Date(d) => serde_json::Value::String(d.format("%Y-%m-%d").to_string()),
            Value::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
            Value::Time(t) => serde_json::Value::String(t.format("%H:%M:%S").to_string()),
            Value::List(l) => serde_json::Value::Array(l.iter().map(Value::to_json).collect()),
        }
    }

    /// Compare for equality using SQL semantics. `NULL = NULL` is `NULL` (false
    /// for filtering purposes, which callers handle).
    pub fn sql_eq(&self, other: &Value) -> Option<bool> {
        if self.is_null() || other.is_null() {
            return None;
        }
        Some(eq_non_null(self, other))
    }

    /// Order for ORDER BY. `NULL` sorts first (SQL ASC NULLS FIRST default).
    pub fn cmp_for_order(&self, other: &Value) -> Ordering {
        match (self, other) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,
            _ => cmp_non_null(self, other).unwrap_or(Ordering::Equal),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, ""),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Integer(n) => write!(f, "{n}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Date(d) => write!(f, "{}", d.format("%Y-%m-%d")),
            Value::DateTime(dt) => write!(f, "{}", dt.to_rfc3339()),
            Value::Time(t) => write!(f, "{}", t.format("%H:%M:%S")),
            Value::List(l) => {
                let parts: Vec<String> = l.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", parts.join(", "))
            }
        }
    }
}

fn coerce_string(s: &str) -> Value {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Value::Date(d);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Value::DateTime(dt.with_timezone(&Utc));
    }
    Value::String(s.to_string())
}

fn eq_non_null(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Integer(x), Value::Integer(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Integer(x), Value::Float(y)) | (Value::Float(y), Value::Integer(x)) => {
            (*x as f64) == *y
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Date(x), Value::Date(y)) => x == y,
        (Value::DateTime(x), Value::DateTime(y)) => x == y,
        (Value::Time(x), Value::Time(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| eq_non_null(a, b))
        }
        _ => false,
    }
}

fn cmp_non_null(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => Some(x.cmp(y)),
        (Value::Integer(x), Value::Integer(y)) => Some(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
        (Value::Integer(x), Value::Float(y)) => (*x as f64).partial_cmp(y),
        (Value::Float(x), Value::Integer(y)) => x.partial_cmp(&(*y as f64)),
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        (Value::Date(x), Value::Date(y)) => Some(x.cmp(y)),
        (Value::DateTime(x), Value::DateTime(y)) => Some(x.cmp(y)),
        (Value::Time(x), Value::Time(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerces_iso_date_from_string() {
        let v = Value::from_yaml(&serde_yaml::Value::String("2026-04-16".to_string()));
        assert!(matches!(v, Value::Date(_)));
    }

    #[test]
    fn coerces_rfc3339_datetime() {
        let v = Value::from_yaml(&serde_yaml::Value::String(
            "2026-04-16T12:00:00Z".to_string(),
        ));
        assert!(matches!(v, Value::DateTime(_)));
    }

    #[test]
    fn plain_string_stays_string() {
        let v = Value::from_yaml(&serde_yaml::Value::String("hello".to_string()));
        assert!(matches!(v, Value::String(_)));
    }

    #[test]
    fn null_compares_as_less() {
        assert_eq!(Value::Null.cmp_for_order(&Value::Integer(1)), Ordering::Less);
        assert_eq!(Value::Integer(1).cmp_for_order(&Value::Null), Ordering::Greater);
    }

    #[test]
    fn integer_float_mixed_equality() {
        assert_eq!(Value::Integer(3).sql_eq(&Value::Float(3.0)), Some(true));
    }
}
