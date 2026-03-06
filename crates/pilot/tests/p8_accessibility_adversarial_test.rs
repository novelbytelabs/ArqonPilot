use std::fs;
use std::path::Path;

/// P8: Zero-Doc UI Accessibility Adversarial Test
/// Tests that UI code meets our strict semantic accessibility rules at the text level.

#[test]
fn test_p8_adversarial_ui_contracts() {
    // 1. Check pilot_ui.js for required accessibility handlers
    let js_path = Path::new("src/pilot_ui.js");
    let js_content = fs::read_to_string(js_path).expect("Failed to read pilot_ui.js");

    // Enforce Error Contract:
    // "On failure paths, assert presence of: role="alert" OR aria-live="assertive",
    // actionable remediation text..."
    assert!(
        js_content.contains("setAttribute('role', 'alert')")
            || js_content.contains("role=\"alert\""),
        "Adversarial failure: pilot_ui.js does not inject role='alert' on failure paths"
    );

    assert!(
        js_content.contains("Mitigation:"),
        "Adversarial failure: Error logic missing actionable 'Mitigation:' remediation hint"
    );

    // 2. Check serve_ui.rs for HTML structure constraints
    let html_path = Path::new("src/serve_ui.rs");
    let html_content = fs::read_to_string(html_path).expect("Failed to read serve_ui.rs");

    // Enforce Semantic Contract:
    // "Replace 'use onclick on non-interactive elements + tabindex' with this rule:
    // Prefer actual <button> elements for interactive workflow chips."
    let span_onclick_re = regex::Regex::new(r"<span[^>]*onclick=").unwrap();
    for (i, line) in html_content.lines().enumerate() {
        if span_onclick_re.is_match(line) {
            assert!(
                line.contains("role=\"button\""),
                "Adversarial failure: Found interactive <span> without role=\"button\" in serve_ui.rs line {}: {}",
                i + 1,
                line.trim()
            );
            assert!(
                line.contains("tabindex=\"0\""),
                "Adversarial failure: Found interactive <span> without tabindex=\"0\" in serve_ui.rs line {}: {}",
                i + 1,
                line.trim()
            );
        }
    }
}
