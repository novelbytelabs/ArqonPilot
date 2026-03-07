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

#[test]
fn test_ci_watch_requires_fresh_branch_run_contract() {
    let script_path = Path::new("../../scripts/gh_actions_watch_latest.sh");
    let script =
        fs::read_to_string(script_path).expect("Failed to read gh_actions_watch_latest.sh");

    assert!(
        script.contains("LOOKBACK_SEC=900")
            && script.contains("window_start_iso")
            && script.contains("fresh_candidate_ids")
            && script.contains("fresh_candidate_ids_by_sha")
            && script.contains("no_fresh_run_detected"),
        "CI watch must enforce fresh-run window and avoid stale run attachment"
    );
}

#[test]
fn test_ci_observatory_terminal_refresh_after_watch_contract() {
    let js_path = Path::new("src/pilot_ui.js");
    let js_content = fs::read_to_string(js_path).expect("Failed to read pilot_ui.js");

    assert!(
        js_content.contains(
            "const ciStatusAfterWatch = await depRun('ci-status', { branch: ciBranch });"
        ) && js_content.contains("watch_summary: ciWatchSummary")
            && js_content.contains("routineSetCiJobChips(ciSummary);"),
        "CI observatory must refresh terminal per-job summary after ci-watch completes"
    );
}

#[test]
fn test_ci_status_prefers_workflow_specific_lookup_contract() {
    let script_path = Path::new("../../scripts/gh_actions_status_latest.sh");
    let script = fs::read_to_string(script_path).expect("Failed to read gh_actions_status_latest.sh");

    assert!(
        script.contains("--workflow docs.yml")
            && script.contains("--workflow ci.yml")
            && script.contains("--workflow pypi.yml")
            && script.contains("Prefer deterministic workflow-specific lookups first."),
        "CI status must query workflow-specific latest runs for docs, ci, and pypi before mixed fallback scan"
    );
}

#[test]
fn test_ci_observatory_maps_pypi_and_docs_job_states_contract() {
    let js_path = Path::new("src/pilot_ui.js");
    let js_content = fs::read_to_string(js_path).expect("Failed to read pilot_ui.js");

    assert!(
        js_content.contains("summary.pypi_state")
            && js_content.contains("summary.pypi_build_and_publish_state")
            && js_content.contains("summary.docs_build_state")
            && js_content.contains("summary.docs_deploy_state"),
        "CI observatory must map docs and pypi workflow/job states from ci-status summary"
    );
}

#[test]
fn test_ci_observatory_dashboard_auto_sync_loop_contract() {
    let js_path = Path::new("src/pilot_ui.js");
    let js_content = fs::read_to_string(js_path).expect("Failed to read pilot_ui.js");

    assert!(
        js_content.contains("function routineSyncCiFromGitHub(opts = {})")
            && js_content.contains("function routineStartCiSyncLoop()")
            && js_content.contains("function routineStopCiSyncLoop()")
            && js_content.contains("setInterval(() =>")
            && js_content.contains("15000"),
        "Dashboard CI observatory must keep a periodic auto-sync loop for live Docs/CI/PyPI status parity"
    );
}
