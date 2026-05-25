//! lava-equivalence — byte-equivalence harness for the lava suite.
//!
//! For each bundled architecture, render via lava-architectures +
//! diff against a typed golden fixture. Today the goldens are
//! committed under `tests/goldens/<name>.json`; the harness gives
//! regression-detection on the renderer. Once pangea-architectures
//! fixtures are generated from real Ruby + `tofu plan -json` output,
//! they slot in as drop-in golden replacements — the harness shape
//! stays the same.
//!
//! ## Typed surface
//!
//! - [`Fixture`] — architecture name + input bindings + path to the
//!   golden JSON.
//! - [`render_lava`] — load the bundled `.tlisp`, evaluate, render
//!   terraform.json.
//! - [`assert_terraform_json_equivalent`] — semantic JSON equality
//!   with sorted-key normalization, surfaces typed
//!   [`EquivalenceMismatch`] on diff.
//! - [`EquivalenceError`] — wraps every failure mode (Io / Eval /
//!   Render / Mismatch) in one typed enum.

#![allow(clippy::module_name_repetitions)]

use indexmap::IndexMap;
use lava_architectures::load_bundled;
use lava_eval::InputBindings;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One architecture × one input bag × one golden JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub architecture: String,
    pub bindings_scalars: IndexMap<String, String>,
    pub bindings_lists: IndexMap<String, Vec<String>>,
    pub golden_relative_path: String,
}

impl Fixture {
    /// Build the InputBindings flowing into lava_eval from the
    /// fixture's scalar + list bags.
    #[must_use]
    pub fn to_input_bindings(&self) -> InputBindings {
        let mut b = InputBindings::new();
        for (k, v) in &self.bindings_scalars {
            b.set_str(k.clone(), v.clone());
        }
        for (k, v) in &self.bindings_lists {
            b.set_list(k.clone(), v.clone());
        }
        b
    }
}

/// Render a bundled architecture by name. Returns the terraform.json
/// shape magma applies.
///
/// Uses lava-architectures' `load_bundled` so the architecture source
/// path is the lava-architectures crate's own CARGO_MANIFEST_DIR —
/// the harness works without consumers tracking source-tree paths.
///
/// # Errors
/// Parse/eval failures, render failures all surface as the
/// corresponding typed [`EquivalenceError`] variants.
pub fn render_lava(fixture: &Fixture) -> Result<serde_json::Value, EquivalenceError> {
    let bindings = fixture.to_input_bindings();
    let arch = load_bundled(&fixture.architecture, &bindings)
        .map_err(|e| EquivalenceError::Eval(e.to_string()))?;
    arch.render_terraform_json()
        .map_err(|e| EquivalenceError::Render(e.to_string()))
}

/// Assert that two terraform.json shapes are semantically equivalent.
///
/// Both inputs go through key-sorted normalization first; whitespace
/// and key order are ignored — only the resulting JSON tree matters
/// (which is what terraform / magma actually consume).
///
/// # Errors
/// Returns [`EquivalenceError::Mismatch`] carrying the first diverging
/// JSON pointer + the actual + expected values.
pub fn assert_terraform_json_equivalent(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
) -> Result<(), EquivalenceError> {
    let a = normalize(actual);
    let e = normalize(expected);
    if let Some(mismatch) = first_divergence(&a, &e, String::new()) {
        return Err(EquivalenceError::Mismatch(mismatch));
    }
    Ok(())
}

/// Recursive sort-keys normalizer. JSON objects become sorted-key
/// objects so two semantically-equal inputs always serialize the same
/// way under canonicalization.
fn normalize(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let mut sorted: Vec<(String, serde_json::Value)> = map
                .iter()
                .map(|(k, val)| (k.clone(), normalize(val)))
                .collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = serde_json::Map::new();
            for (k, val) in sorted {
                out.insert(k, val);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(normalize).collect())
        }
        other => other.clone(),
    }
}

/// Walk two values in parallel; return the first JSON-pointer path
/// where they diverge.
fn first_divergence(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    path: String,
) -> Option<EquivalenceMismatch> {
    match (actual, expected) {
        (serde_json::Value::Object(a), serde_json::Value::Object(e)) => {
            // Compare key set, then per-key recurse.
            let mut keys: Vec<&String> = a.keys().chain(e.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let ap = format!("{path}/{k}");
                match (a.get(k), e.get(k)) {
                    (Some(av), Some(ev)) => {
                        if let Some(m) = first_divergence(av, ev, ap) {
                            return Some(m);
                        }
                    }
                    (None, Some(ev)) => {
                        return Some(EquivalenceMismatch {
                            pointer: ap,
                            actual: serde_json::Value::Null,
                            expected: ev.clone(),
                        });
                    }
                    (Some(av), None) => {
                        return Some(EquivalenceMismatch {
                            pointer: ap,
                            actual: av.clone(),
                            expected: serde_json::Value::Null,
                        });
                    }
                    (None, None) => unreachable!(),
                }
            }
            None
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(e)) => {
            if a.len() != e.len() {
                return Some(EquivalenceMismatch {
                    pointer: format!("{path}/length"),
                    actual: serde_json::Value::Number(a.len().into()),
                    expected: serde_json::Value::Number(e.len().into()),
                });
            }
            for (i, (av, ev)) in a.iter().zip(e).enumerate() {
                if let Some(m) = first_divergence(av, ev, format!("{path}/{i}")) {
                    return Some(m);
                }
            }
            None
        }
        (av, ev) if av == ev => None,
        (av, ev) => Some(EquivalenceMismatch {
            pointer: if path.is_empty() {
                "/".to_string()
            } else {
                path
            },
            actual: av.clone(),
            expected: ev.clone(),
        }),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquivalenceMismatch {
    /// JSON-pointer-ish path to the divergence.
    pub pointer: String,
    pub actual: serde_json::Value,
    pub expected: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum EquivalenceError {
    #[error("io reading {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("eval: {0}")]
    Eval(String),
    #[error("render: {0}")]
    Render(String),
    #[error("mismatch at `{}`: actual={} expected={}", .0.pointer, .0.actual, .0.expected)]
    Mismatch(EquivalenceMismatch),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equivalent_objects_pass_regardless_of_key_order() {
        let a: serde_json::Value = serde_json::from_str(r#"{"b": 2, "a": 1}"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
        assert!(assert_terraform_json_equivalent(&a, &b).is_ok());
    }

    #[test]
    fn array_order_matters_for_terraform_lists() {
        // terraform list outputs depend on order — DO NOT sort arrays.
        let a: serde_json::Value = serde_json::from_str(r#"[1,2,3]"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(r#"[3,2,1]"#).unwrap();
        let err = assert_terraform_json_equivalent(&a, &b).unwrap_err();
        assert!(matches!(err, EquivalenceError::Mismatch(_)));
    }

    #[test]
    fn array_length_diff_is_surfaced_at_length_pointer() {
        let a: serde_json::Value = serde_json::from_str(r#"[1,2,3]"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(r#"[1,2]"#).unwrap();
        let err = assert_terraform_json_equivalent(&a, &b).unwrap_err();
        let EquivalenceError::Mismatch(m) = err else {
            panic!("expected mismatch");
        };
        assert!(m.pointer.ends_with("/length"));
    }

    #[test]
    fn missing_key_surfaces_at_that_pointer() {
        let a: serde_json::Value =
            serde_json::from_str(r#"{"resource": {"aws_vpc": {}}}"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(
            r#"{"resource": {"aws_vpc": {}, "aws_subnet": {}}}"#,
        )
        .unwrap();
        let err = assert_terraform_json_equivalent(&a, &b).unwrap_err();
        let EquivalenceError::Mismatch(m) = err else {
            panic!("expected mismatch");
        };
        assert_eq!(m.pointer, "/resource/aws_subnet");
    }

    #[test]
    fn fixture_to_input_bindings_round_trips_scalar_and_list() {
        let mut scalars = IndexMap::new();
        scalars.insert("cidr".to_string(), "10.0.0.0/16".to_string());
        let mut lists = IndexMap::new();
        lists.insert(
            "azs".to_string(),
            vec!["a".to_string(), "b".to_string()],
        );
        let f = Fixture {
            architecture: "x".into(),
            bindings_scalars: scalars,
            bindings_lists: lists,
            golden_relative_path: "x.json".into(),
        };
        // Round-trip via serde to prove the wire shape works.
        let json = serde_json::to_string(&f).unwrap();
        let parsed: Fixture = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bindings_scalars["cidr"], "10.0.0.0/16");
        assert_eq!(parsed.bindings_lists["azs"], vec!["a", "b"]);
        let _b = f.to_input_bindings(); // just confirms it doesn't panic
    }
}
