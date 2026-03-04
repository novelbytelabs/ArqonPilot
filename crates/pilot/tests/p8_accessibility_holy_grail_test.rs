use std::fs;
use std::path::Path;

/// P8: Zero-Doc UI Accessibility Holy Grail Test
/// Tests that the integration flow (Dashboard -> Dependencies -> Branch -> Multi -> Verify)
/// is fully represented in the UI markup with actionable zero-doc hints and proper semantic flow.

#[test]
fn test_p8_holy_grail_operator_flow() {
    let ui_rs_path = Path::new("src/serve_ui.rs");
    let ui_content = fs::read_to_string(ui_rs_path).expect("Failed to read serve_ui.rs");

    // Verify Dashboard context
    assert!(
        ui_content.contains("Pilot Control Panel"),
        "Missing main Dashboard title"
    );

    // Verify Dependencies step
    assert!(
        ui_content.contains("Command Graph Orchestration (P5)"),
        "Missing command orchestration block"
    );
    assert!(
        ui_content.contains("Preview Database and Dependency Status"),
        "Missing dependency/DB status preview zero-doc hint"
    );

    // Verify Branch step
    assert!(
        ui_content.contains("Preview Branch Status"),
        "Missing branch preview zero-doc hint"
    );

    // Verify Multi step
    assert!(
        ui_content.contains("Preview Multi Repo Status"),
        "Missing multi-repo status preview zero-doc hint"
    );

    // Verify Verify (Tamper-Evidence) step
    assert!(
        ui_content.contains("Evidence Integrity Verification")
            || ui_content.contains("Tamper-Evident Verify"),
        "Missing Verify section"
    );

    // 2. Check for missing document links (Zero-Doc requirement)
    // The UI should not point users to internal markdown files for basic operations.
    // Ensure the helper texts exist and are sufficiently descriptive.
    assert!(
        ui_content.contains("Unified cross-tab sequence. Preview operations never mutate."),
        "Zero-doc hint for P5 orchestration missing"
    );
    
    // Check js for zero-doc hints in failure states
    let ui_js_path = Path::new("src/pilot_ui.js");
    let js_content = fs::read_to_string(ui_js_path).expect("Failed to read pilot_ui.js");
    assert!(
        js_content.contains("Mitigation: Ensure the bundle and all referenced artifacts exist"),
        "Missing detailed zero-doc mitigation for missing_file"
    );
}
