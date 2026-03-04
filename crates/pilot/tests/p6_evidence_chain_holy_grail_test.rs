use pilot_core::{verify_evidence_bundle, EvidenceArtifact, EvidenceBundleManifest};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_p6_bundle_export_and_verify_holy_grail() {
    // 1. Setup a valid bundle structure
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    
    // Create two dummy artifacts
    let art1_path = root.join("art1.json");
    let art2_path = root.join("art2.json");
    
    fs::write(&art1_path, b"dummy content 1").unwrap();
    fs::write(&art2_path, b"dummy content 2").unwrap();
    
    let sha1 = pilot_core::compute_file_hash(&art1_path).unwrap();
    let sha2 = pilot_core::compute_file_hash(&art2_path).unwrap();

    // 2. Build the Canonical Manifest
    let manifest = EvidenceBundleManifest {
        bundle_id: "test-bundle-123".to_string(),
        created_at: "2026-03-03T12:00:00Z".to_string(),
        scope_id: Some("test-scope".to_string()),
        operator: Some("test-user".to_string()),
        artifacts: vec![
            // Deliberately unsorted here to prove compute_hash() sorts it
            EvidenceArtifact {
                path: "art2.json".to_string(),
                sha256: sha2.clone(),
                size_bytes: 15,
            },
            EvidenceArtifact {
                path: "art1.json".to_string(),
                sha256: sha1.clone(),
                size_bytes: 15,
            },
        ],
        chain_integrity: json!({
            "is_valid": true,
            "audited_events": 5,
            "errors": []
        }),
    };

    let bundle_hash = manifest.compute_hash();

    // 3. Write the final bundle JSON
    let bundle = json!({
        "exported_at_unix": 1234567890,
        "bundle_hash": bundle_hash,
        "manifest": manifest
    });
    
    let bundle_path = root.join("evidence_bundle.json");
    fs::write(&bundle_path, serde_json::to_string_pretty(&bundle).unwrap()).unwrap();

    // 4. Verify the Holy Grail
    let result = verify_evidence_bundle(&bundle_path);
    assert!(result.is_valid, "Valid bundle must pass verification");
    assert_eq!(result.reason_code, "valid");
}
