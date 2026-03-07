use std::fs;
use std::path::Path;

#[test]
fn test_routine_autoheal_and_codex_escalation_contract() {
    let js_path = Path::new("src/pilot_ui.js");
    let js_content = fs::read_to_string(js_path).expect("Failed to read pilot_ui.js");

    assert!(
        js_content.contains("function routineAutoHealAndRetry()"),
        "Missing auto-heal routine handler"
    );
    assert!(
        js_content.contains("function routineEscalateFailureToCodex()"),
        "Missing Codex escalation handler for routine failures"
    );
    assert!(
        js_content.contains("signature: 'format_parity'")
            && js_content.contains("action: 'cargo-fmt'"),
        "Missing format parity auto-heal playbook"
    );
    assert!(
        js_content.contains("dashRoutineAutoHealRunning"),
        "Missing auto-heal running guard state"
    );
}
