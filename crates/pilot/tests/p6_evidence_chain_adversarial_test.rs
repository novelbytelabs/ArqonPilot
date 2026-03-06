use pilot_core::{verify_evidence_bundle, EvidenceArtifact, EvidenceBundleManifest};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_p6_verify_tampered_artifact_yields_hash_mismatch() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let art_path = root.join("art1.json");
    fs::write(&art_path, b"original content").unwrap();

    let original_hash = pilot_core::compute_file_hash(&art_path).unwrap();

    let manifest = EvidenceBundleManifest {
        bundle_id: "test".to_string(),
        created_at: "now".to_string(),
        scope_id: None,
        operator: None,
        artifacts: vec![EvidenceArtifact {
            path: "art1.json".to_string(),
            sha256: original_hash,
            size_bytes: 16,
        }],
        chain_integrity: json!({ "is_valid": true }),
    };

    let bundle = json!({
        "bundle_hash": manifest.compute_hash(),
        "manifest": manifest
    });

    let bundle_path = root.join("bundle.json");
    fs::write(&bundle_path, serde_json::to_string(&bundle).unwrap()).unwrap();

    // Tamper the file!
    fs::write(&art_path, b"tampered content").unwrap();

    let result = verify_evidence_bundle(&bundle_path);
    assert!(!result.is_valid);
    assert_eq!(result.reason_code, "hash_mismatch");
    assert_eq!(result.offending_path.unwrap(), "art1.json");
}

#[test]
fn test_p6_verify_missing_artifact_yields_missing_file() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let manifest = EvidenceBundleManifest {
        bundle_id: "test".to_string(),
        created_at: "now".to_string(),
        scope_id: None,
        operator: None,
        artifacts: vec![EvidenceArtifact {
            path: "missing.json".to_string(),
            sha256: "fakehash".to_string(),
            size_bytes: 10,
        }],
        chain_integrity: json!({ "is_valid": true }),
    };

    let bundle = json!({
        "bundle_hash": manifest.compute_hash(),
        "manifest": manifest
    });

    let bundle_path = root.join("bundle.json");
    fs::write(&bundle_path, serde_json::to_string(&bundle).unwrap()).unwrap();

    let result = verify_evidence_bundle(&bundle_path);
    assert!(!result.is_valid);
    assert_eq!(result.reason_code, "missing_file");
    assert_eq!(result.offending_path.unwrap(), "missing.json");
}

#[test]
fn test_p6_verify_corrupted_json_yields_parse_error() {
    let dir = TempDir::new().unwrap();
    let bundle_path = dir.path().join("bundle.json");
    fs::write(&bundle_path, b"{ bad json {").unwrap();

    let result = verify_evidence_bundle(&bundle_path);
    assert!(!result.is_valid);
    assert_eq!(result.reason_code, "parse_error");
}

#[test]
fn test_p6_verify_bad_schema_yields_schema_error() {
    let dir = TempDir::new().unwrap();
    let bundle_path = dir.path().join("bundle.json");
    // Valid JSON, but missing "bundle_hash"
    fs::write(&bundle_path, b"{\"manifest\": {}}").unwrap();

    let result = verify_evidence_bundle(&bundle_path);
    assert!(!result.is_valid);
    assert_eq!(result.reason_code, "schema_error");
}

#[test]
fn test_p6_verify_empty_artifacts_yields_schema_error() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let manifest = EvidenceBundleManifest {
        bundle_id: "test".to_string(),
        created_at: "now".to_string(),
        scope_id: None,
        operator: None,
        artifacts: vec![], // Empty artifacts list
        chain_integrity: json!({ "is_valid": true }),
    };

    let bundle = json!({
        "bundle_hash": manifest.compute_hash(),
        "manifest": manifest
    });

    let bundle_path = root.join("bundle.json");
    fs::write(&bundle_path, serde_json::to_string(&bundle).unwrap()).unwrap();

    let result = verify_evidence_bundle(&bundle_path);
    assert!(!result.is_valid);
    assert_eq!(result.reason_code, "schema_error");
    assert!(result.details.contains("artifacts array is empty"));
}

#[test]
fn test_p6_verify_chain_mismatch() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let art_path = root.join("art1.json");
    fs::write(&art_path, b"content").unwrap();
    let sha = pilot_core::compute_file_hash(&art_path).unwrap();

    let manifest = EvidenceBundleManifest {
        bundle_id: "test".to_string(),
        created_at: "now".to_string(),
        scope_id: None,
        operator: None,
        artifacts: vec![EvidenceArtifact {
            path: "art1.json".to_string(),
            sha256: sha,
            size_bytes: 7,
        }],
        chain_integrity: json!({ "is_valid": false }), // Invalid internal chain
    };

    let bundle = json!({
        "bundle_hash": manifest.compute_hash(),
        "manifest": manifest
    });

    let bundle_path = root.join("bundle.json");
    fs::write(&bundle_path, serde_json::to_string(&bundle).unwrap()).unwrap();

    let result = verify_evidence_bundle(&bundle_path);
    assert!(!result.is_valid);
    assert_eq!(result.reason_code, "chain_mismatch");
}
