//! Convert a `query::Value` back into a `serde_yaml::Value` for Runestone
//! cell edits. Pairs with `Value::from_yaml`; round-tripping should be
//! lossless for the tagged-scalar and list shapes we actually store.
//!
//! Mappings produce an error — the query evaluator doesn't synthesize
//! mappings, so seeing one on the writeback path means the caller is trying
//! to persist something the rest of the stack can't round-trip cleanly.

use thiserror::Error;

use super::value::Value;

#[derive(Debug, Error)]
pub enum WritebackError {
    #[error("cannot serialize {0} to YAML")]
    Unrepresentable(&'static str),
    #[error("invalid float: {0}")]
    InvalidFloat(f64),
}

/// Convert a query `Value` into the `serde_yaml::Value` shape used in note
/// frontmatter. Dates and datetimes render as ISO 8601 strings, matching the
/// coercion rules `Value::from_yaml` uses on the way in.
pub fn value_to_yaml(v: &Value) -> Result<serde_yaml::Value, WritebackError> {
    match v {
        Value::Null => Ok(serde_yaml::Value::Null),
        Value::Bool(b) => Ok(serde_yaml::Value::Bool(*b)),
        Value::Integer(n) => Ok(serde_yaml::Value::Number((*n).into())),
        Value::Float(f) => {
            let n = serde_yaml::Number::from(*f);
            // serde_yaml encodes NaN/Infinity as special forms most YAML
            // readers reject; refuse them here so we never write a file we
            // can't parse back.
            if !f.is_finite() {
                return Err(WritebackError::InvalidFloat(*f));
            }
            Ok(serde_yaml::Value::Number(n))
        }
        Value::String(s) => Ok(serde_yaml::Value::String(s.clone())),
        Value::Date(d) => Ok(serde_yaml::Value::String(d.format("%Y-%m-%d").to_string())),
        Value::DateTime(dt) => Ok(serde_yaml::Value::String(dt.to_rfc3339())),
        Value::Time(t) => Ok(serde_yaml::Value::String(t.format("%H:%M:%S").to_string())),
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(value_to_yaml(item)?);
            }
            Ok(serde_yaml::Value::Sequence(out))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_scalars() {
        let cases: Vec<serde_yaml::Value> = vec![
            serde_yaml::Value::Null,
            serde_yaml::Value::Bool(true),
            serde_yaml::Value::Number(7.into()),
            serde_yaml::Value::String("hello".to_string()),
        ];
        for input in cases {
            let v = Value::from_yaml(&input);
            let back = value_to_yaml(&v).unwrap();
            assert_eq!(input, back);
        }
    }

    #[test]
    fn round_trips_list_of_strings() {
        let input = serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("a".to_string()),
            serde_yaml::Value::String("b".to_string()),
        ]);
        let v = Value::from_yaml(&input);
        let back = value_to_yaml(&v).unwrap();
        assert_eq!(input, back);
    }

    #[test]
    fn date_becomes_iso_string() {
        let input = serde_yaml::Value::String("2026-04-17".to_string());
        let v = Value::from_yaml(&input);
        let back = value_to_yaml(&v).unwrap();
        assert_eq!(back, serde_yaml::Value::String("2026-04-17".to_string()));
    }

    #[test]
    fn rejects_nan() {
        let v = Value::Float(f64::NAN);
        assert!(value_to_yaml(&v).is_err());
    }
}
