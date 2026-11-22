//! The comparison of the Rust report with the report the python implementation produces.
//!
//! The comparison is structural: it walks two parsed JSON values and never looks at text,
//! so `1e21` and `1e+21`, or `3` and `3.0`, are the same number.

use serde_json::Value;

/// Compare `ours` with `theirs` and return the differences, an empty list where they agree.
///
/// Every difference names the JSON pointer and both values. The order of an array is
/// significant, the order of the keys of an object is not, and two numbers agree when they are
/// the same `f64`. A key present on one side only is a difference, which catches a field the
/// port forgot.
///
/// The number of a value parsed from the text of a fixture is the double that text round trips
/// to, which is what the `float_roundtrip` feature of serde_json makes of it: without it the
/// 17 significant digits python writes for a double parse to a neighbour of the double they
/// came from, and the two sides of the gate would differ on a number both sides got right.
///
/// An entry of `excluded` silences a part of the report and comes in one of two forms:
///
/// - An exact pointer, such as `/models/0/listOfRules`, which silences that pointer and
///   everything below it, and nothing else: a sibling whose pointer merely starts with the same
///   text stays compared. The entry is matched against the pointer of the walk, whose tokens are
///   escaped as RFC 6901 asks, so an entry has to carry the same escaping: a key which holds a
///   slash is `~1` in the entry, a key which holds a tilde is `~0`.
/// - A trailing segment, written `**/derivedUnits`, which silences every pointer whose last
///   segment is exactly that segment, and everything below those. It is the form for a field
///   which sits on every element, where one exact pointer per element would be unreadable.
///
/// An empty entry, or the bare `**/`, names nothing and is skipped rather than silencing the
/// whole report.
pub fn compare(ours: &Value, theirs: &Value, excluded: &[&str]) -> Vec<String> {
    let mut differences = Vec::new();
    walk("", ours, theirs, excluded, &mut differences);
    differences
}

/// The last segment of a pointer, `None` for the pointer of the root, which has none.
fn last_segment(pointer: &str) -> Option<&str> {
    pointer.rsplit_once('/').map(|(_, last)| last)
}

/// Whether `pointer` is silenced by one of the entries of `excluded`.
fn is_excluded(pointer: &str, excluded: &[&str]) -> bool {
    excluded
        .iter()
        .any(|entry| match entry.strip_prefix("**/") {
            Some(segment) => !segment.is_empty() && last_segment(pointer) == Some(segment),
            None => {
                !entry.is_empty()
                    && (pointer == *entry
                        || pointer
                            .strip_prefix(entry)
                            .is_some_and(|rest| rest.starts_with('/')))
            }
        })
}

/// Escape a key as a JSON pointer token (RFC 6901).
fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Record a difference at `pointer`.
fn record(pointer: &str, ours: Option<&Value>, theirs: Option<&Value>, out: &mut Vec<String>) {
    let show = |v: Option<&Value>| v.map_or_else(|| "<absent>".to_string(), Value::to_string);
    let at = if pointer.is_empty() { "/" } else { pointer };
    out.push(format!(
        "{at}: ours {} but python {}",
        show(ours),
        show(theirs)
    ));
}

/// Walk both values in parallel.
fn walk(pointer: &str, ours: &Value, theirs: &Value, excluded: &[&str], out: &mut Vec<String>) {
    if is_excluded(pointer, excluded) {
        return;
    }
    match (ours, theirs) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, va) in a {
                let child = format!("{pointer}/{}", escape(key));
                match b.get(key) {
                    Some(vb) => walk(&child, va, vb, excluded, out),
                    None if is_excluded(&child, excluded) => {}
                    None => record(&child, Some(va), None, out),
                }
            }
            for (key, vb) in b {
                if !a.contains_key(key) {
                    let child = format!("{pointer}/{}", escape(key));
                    if !is_excluded(&child, excluded) {
                        record(&child, None, Some(vb), out);
                    }
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            for i in 0..a.len().max(b.len()) {
                let child = format!("{pointer}/{i}");
                match (a.get(i), b.get(i)) {
                    (Some(va), Some(vb)) => walk(&child, va, vb, excluded, out),
                    (va, vb) => {
                        if !is_excluded(&child, excluded) {
                            record(&child, va, vb, out);
                        }
                    }
                }
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            if a.as_f64() != b.as_f64() {
                record(pointer, Some(ours), Some(theirs), out);
            }
        }
        _ => {
            if ours != theirs {
                record(pointer, Some(ours), Some(theirs), out);
            }
        }
    }
}
