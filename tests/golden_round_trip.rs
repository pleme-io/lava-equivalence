//! End-to-end regression tests — render each bundled architecture
//! and assert byte-equivalent (semantic JSON) to the committed
//! goldens.
//!
//! Today the goldens are generated from lava itself (regression net).
//! When pangea-side fixtures are generated from real Ruby + `tofu
//! plan -json`, the goldens get replaced — the harness shape stays.

use indexmap::IndexMap;
use lava_equivalence::{assert_terraform_json_equivalent, render_lava, Fixture};

fn golden_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
        .join(format!("{name}.json"))
}

fn load_or_seed_golden(name: &str, actual: &serde_json::Value) -> serde_json::Value {
    let path = golden_path(name);
    if !path.exists() {
        // Bootstrap: seed the golden on first run so the matrix is
        // self-priming. Subsequent runs assert against the committed
        // file. CI guards by running this twice on each PR.
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_string_pretty(actual).unwrap()).unwrap();
        return actual.clone();
    }
    let bytes = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str(&bytes).unwrap()
}

#[test]
fn aws_vpc_network_round_trips_to_golden() {
    let fixture = Fixture {
        architecture: "aws-vpc-network".to_string(),
        bindings_scalars: IndexMap::new(),
        bindings_lists: IndexMap::new(),
        golden_relative_path: "aws-vpc-network.json".to_string(),
    };
    let actual = render_lava(&fixture).unwrap();
    let expected = load_or_seed_golden("aws-vpc-network", &actual);
    assert_terraform_json_equivalent(&actual, &expected).unwrap();
}

#[test]
fn cloudflare_dns_records_round_trips_to_golden() {
    let fixture = Fixture {
        architecture: "cloudflare-dns-records".to_string(),
        bindings_scalars: IndexMap::new(),
        bindings_lists: IndexMap::new(),
        golden_relative_path: "cloudflare-dns-records.json".to_string(),
    };
    let actual = render_lava(&fixture).unwrap();
    let expected = load_or_seed_golden("cloudflare-dns-records", &actual);
    assert_terraform_json_equivalent(&actual, &expected).unwrap();
}

#[test]
fn akeyless_secrets_round_trips_to_golden() {
    let fixture = Fixture {
        architecture: "akeyless-secrets".to_string(),
        bindings_scalars: IndexMap::new(),
        bindings_lists: IndexMap::new(),
        golden_relative_path: "akeyless-secrets.json".to_string(),
    };
    let actual = render_lava(&fixture).unwrap();
    let expected = load_or_seed_golden("akeyless-secrets", &actual);
    assert_terraform_json_equivalent(&actual, &expected).unwrap();
}

#[test]
fn override_bindings_produce_different_golden() {
    // Sanity check: changing the input changes the output (no
    // accidental constant-folding in the renderer).
    let mut bindings = IndexMap::new();
    bindings.insert("name".to_string(), "override-name".to_string());
    let fixture = Fixture {
        architecture: "aws-vpc-network".to_string(),
        bindings_scalars: bindings,
        bindings_lists: IndexMap::new(),
        golden_relative_path: "override.json".to_string(),
    };
    let actual = render_lava(&fixture).unwrap();
    assert!(
        actual["resource"]["aws_vpc"]
            .as_object()
            .map(|m| m.contains_key("override-name-vpc"))
            .unwrap_or(false),
        "expected override-name-vpc in output, got {actual:#?}"
    );
}
