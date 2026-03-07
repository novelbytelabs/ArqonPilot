use std::fs;
use std::path::Path;

#[test]
fn test_ci_observatory_inflight_pass_suppression_contract() {
    let js_path = Path::new("src/pilot_ui.js");
    let js_content = fs::read_to_string(js_path).expect("Failed to read pilot_ui.js");

    assert!(
        js_content.contains("function routineCiSummaryInFlight(summary = null)"),
        "Missing CI summary in-flight detector"
    );
    assert!(
        js_content.contains("dashRoutineWorkspaceState.ciInFlight")
            && js_content.contains("routineCiSummaryInFlight(summary)"),
        "Missing combined in-flight guard (workspace + summary)"
    );
    assert!(
        js_content.contains("s === 'pass'")
            && js_content.contains("s === 'success'")
            && js_content.contains("s === 'completed'")
            && js_content.contains("s = 'running'"),
        "Missing pass->running coercion while CI is in-flight"
    );
    assert!(
        js_content.contains("routineNormalizeCiState(raw, summary)")
            && js_content.contains("routineNormalizeCiState(summary.overall_state, summary)")
            || (js_content.contains("routineCiWorkflowState(summary = {}, workflow = {})")
                && js_content.contains("routineNormalizeCiState(raw, summary)")),
        "Workflow state rendering must normalize through in-flight guard"
    );
}
