//! Built-in SQL functions.
//!
//! Split into scalar functions (evaluated once per row) and aggregates
//! (folded over a group of rows). Names are matched case-insensitively.

use chrono::{Datelike, Duration, Utc};

use super::eval::EvalError;
use super::value::Value;

/// True if `name` names an aggregate function. Aggregates trigger group-mode
/// evaluation in `eval::execute`.
pub fn is_aggregate(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "COUNT" | "SUM" | "AVG" | "MIN" | "MAX"
    )
}

/// Call a scalar built-in. Returns `UnknownFunction` if the name isn't
/// recognized (the caller may route to aggregates or user functions).
pub fn call_scalar(name: &str, args: &[Value]) -> Result<Value, EvalError> {
    match name.to_ascii_uppercase().as_str() {
        "TODAY" => {
            arity(name, args, 0)?;
            Ok(Value::Date(Utc::now().date_naive()))
        }
        "NOW" => {
            arity(name, args, 0)?;
            Ok(Value::DateTime(Utc::now()))
        }
        "LENGTH" => {
            arity(name, args, 1)?;
            Ok(match &args[0] {
                Value::String(s) => Value::Integer(s.chars().count() as i64),
                Value::List(l) => Value::Integer(l.len() as i64),
                Value::Null => Value::Null,
                other => {
                    return Err(EvalError::TypeMismatch(format!(
                        "LENGTH expects string or list, got {}",
                        other.type_name()
                    )));
                }
            })
        }
        "LOWER" => {
            arity(name, args, 1)?;
            Ok(match &args[0] {
                Value::String(s) => Value::String(s.to_lowercase()),
                Value::Null => Value::Null,
                other => {
                    return Err(EvalError::TypeMismatch(format!(
                        "LOWER expects string, got {}",
                        other.type_name()
                    )));
                }
            })
        }
        "UPPER" => {
            arity(name, args, 1)?;
            Ok(match &args[0] {
                Value::String(s) => Value::String(s.to_uppercase()),
                Value::Null => Value::Null,
                other => {
                    return Err(EvalError::TypeMismatch(format!(
                        "UPPER expects string, got {}",
                        other.type_name()
                    )));
                }
            })
        }
        "COALESCE" => {
            if args.is_empty() {
                return Err(EvalError::TypeMismatch(
                    "COALESCE requires at least one argument".to_string(),
                ));
            }
            Ok(args
                .iter()
                .find(|v| !v.is_null())
                .cloned()
                .unwrap_or(Value::Null))
        }
        "DATE_ADD" => {
            arity(name, args, 2)?;
            let days = match &args[1] {
                Value::Integer(n) => *n,
                other => {
                    return Err(EvalError::TypeMismatch(format!(
                        "DATE_ADD days must be integer, got {}",
                        other.type_name()
                    )));
                }
            };
            match &args[0] {
                Value::Date(d) => Ok(Value::Date(*d + Duration::days(days))),
                Value::DateTime(dt) => Ok(Value::DateTime(*dt + Duration::days(days))),
                Value::Null => Ok(Value::Null),
                other => Err(EvalError::TypeMismatch(format!(
                    "DATE_ADD expects date/datetime, got {}",
                    other.type_name()
                ))),
            }
        }
        "EXISTS" => {
            // Muninn's schema-less take on EXISTS: true when the argument is
            // present and non-empty. Useful for `WHERE EXISTS(due_date)`.
            arity(name, args, 1)?;
            Ok(Value::Bool(match &args[0] {
                Value::Null => false,
                Value::String(s) => !s.is_empty(),
                Value::List(l) => !l.is_empty(),
                _ => true,
            }))
        }
        "YEAR" => {
            arity(name, args, 1)?;
            Ok(match &args[0] {
                Value::Date(d) => Value::Integer(d.year() as i64),
                Value::DateTime(dt) => Value::Integer(dt.year() as i64),
                Value::Null => Value::Null,
                other => {
                    return Err(EvalError::TypeMismatch(format!(
                        "YEAR expects date/datetime, got {}",
                        other.type_name()
                    )));
                }
            })
        }
        _ => Err(EvalError::UnknownFunction(name.to_string())),
    }
}

/// Fold an aggregate function over a sequence of per-row values.
///
/// Per-row `values` already reflect the argument expression evaluated for
/// each row in the group. COUNT(*) callers must pass a `values` slice with
/// one entry per group row (value itself ignored).
pub fn fold_aggregate(
    name: &str,
    values: &[Value],
    is_count_star: bool,
) -> Result<Value, EvalError> {
    match name.to_ascii_uppercase().as_str() {
        "COUNT" => {
            if is_count_star {
                Ok(Value::Integer(values.len() as i64))
            } else {
                Ok(Value::Integer(
                    values.iter().filter(|v| !v.is_null()).count() as i64,
                ))
            }
        }
        "SUM" => sum_numeric(values),
        "AVG" => {
            let nums: Vec<f64> = values.iter().filter_map(as_f64).collect();
            if nums.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(Value::Float(nums.iter().sum::<f64>() / nums.len() as f64))
            }
        }
        "MIN" => fold_minmax(values, true),
        "MAX" => fold_minmax(values, false),
        _ => Err(EvalError::UnknownFunction(name.to_string())),
    }
}

fn arity(name: &str, args: &[Value], expected: usize) -> Result<(), EvalError> {
    if args.len() != expected {
        return Err(EvalError::TypeMismatch(format!(
            "{} expects {} argument(s), got {}",
            name.to_ascii_uppercase(),
            expected,
            args.len()
        )));
    }
    Ok(())
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn sum_numeric(values: &[Value]) -> Result<Value, EvalError> {
    let mut any_float = false;
    let mut int_sum: i64 = 0;
    let mut float_sum: f64 = 0.0;
    let mut any_value = false;
    for v in values {
        match v {
            Value::Null => continue,
            Value::Integer(n) => {
                any_value = true;
                if any_float {
                    float_sum += *n as f64;
                } else {
                    int_sum = int_sum.saturating_add(*n);
                }
            }
            Value::Float(f) => {
                any_value = true;
                if !any_float {
                    float_sum = int_sum as f64;
                    any_float = true;
                }
                float_sum += f;
            }
            other => {
                return Err(EvalError::TypeMismatch(format!(
                    "SUM expects numeric values, got {}",
                    other.type_name()
                )));
            }
        }
    }
    if !any_value {
        return Ok(Value::Null);
    }
    Ok(if any_float {
        Value::Float(float_sum)
    } else {
        Value::Integer(int_sum)
    })
}

fn fold_minmax(values: &[Value], is_min: bool) -> Result<Value, EvalError> {
    let mut best: Option<Value> = None;
    for v in values {
        if v.is_null() {
            continue;
        }
        match &best {
            None => best = Some(v.clone()),
            Some(current) => {
                let ord = v.cmp_for_order(current);
                let replace = if is_min {
                    ord == std::cmp::Ordering::Less
                } else {
                    ord == std::cmp::Ordering::Greater
                };
                if replace {
                    best = Some(v.clone());
                }
            }
        }
    }
    Ok(best.unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_of_string() {
        let v = call_scalar("LENGTH", &[Value::String("hello".into())]).unwrap();
        assert!(matches!(v, Value::Integer(5)));
    }

    #[test]
    fn lower_upper_null_pass_through() {
        assert!(matches!(
            call_scalar("LOWER", &[Value::Null]).unwrap(),
            Value::Null
        ));
        assert!(matches!(
            call_scalar("UPPER", &[Value::Null]).unwrap(),
            Value::Null
        ));
    }

    #[test]
    fn coalesce_first_non_null() {
        let v = call_scalar(
            "COALESCE",
            &[Value::Null, Value::Null, Value::String("x".into())],
        )
        .unwrap();
        assert!(matches!(v, Value::String(ref s) if s == "x"));
    }

    #[test]
    fn date_add_positive() {
        let base = chrono::NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        let v = call_scalar("DATE_ADD", &[Value::Date(base), Value::Integer(7)]).unwrap();
        match v {
            Value::Date(d) => assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 4, 23).unwrap()),
            _ => panic!("expected Date"),
        }
    }

    #[test]
    fn exists_empty_string_false() {
        let v = call_scalar("EXISTS", &[Value::String(String::new())]).unwrap();
        assert!(matches!(v, Value::Bool(false)));
    }

    #[test]
    fn count_non_null() {
        let v = fold_aggregate(
            "COUNT",
            &[Value::Integer(1), Value::Null, Value::Integer(3)],
            false,
        )
        .unwrap();
        assert!(matches!(v, Value::Integer(2)));
    }

    #[test]
    fn count_star_counts_rows() {
        let v = fold_aggregate("COUNT", &[Value::Null, Value::Null], true).unwrap();
        assert!(matches!(v, Value::Integer(2)));
    }

    #[test]
    fn sum_mixed_types() {
        let v = fold_aggregate(
            "SUM",
            &[Value::Integer(1), Value::Float(2.5), Value::Integer(3)],
            false,
        )
        .unwrap();
        match v {
            Value::Float(f) => assert!((f - 6.5).abs() < 1e-9),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn avg_of_empty_is_null() {
        let v = fold_aggregate("AVG", &[], false).unwrap();
        assert!(matches!(v, Value::Null));
    }

    #[test]
    fn min_skips_null() {
        let v = fold_aggregate(
            "MIN",
            &[Value::Null, Value::Integer(3), Value::Integer(1)],
            false,
        )
        .unwrap();
        assert!(matches!(v, Value::Integer(1)));
    }
}
