use std::fs;
use std::path::Path;

#[test]
fn test_routine_autoheal_and_codex_escalation_contract() {
    let js_path = Path::new("src/pilot_ui.js");
    let js_content = fs::read_to_string(js_path).expect("Failed to read pilot_ui.js");

    assert!(
        js_content.contains("function routineAutoHealAndRetry("),
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
    assert!(
        js_content.contains("dash-routine-auto-heal")
            && js_content.contains("function routineAutoHealEnabled()"),
        "Missing auto-heal toggle plumbing"
    );
    assert!(
        js_content.contains("function routineShowHealLog()")
            && js_content.contains("function routineShowHealRecipes()")
            && js_content.contains("function routineClearHealLog()")
            && js_content.contains("pilot.routine.heal.log.v1")
            && js_content.contains("pilot.routine.heal.recipe.v1")
            && js_content.contains("function routineLearnSuccessfulHeal(")
            && js_content.contains("function routineMatchLearnedHealRecipe("),
        "Missing learning-loop heal log controls"
    );
    assert!(
        js_content.contains("function depRunWithTimeout(")
            && js_content
                .contains("routineRecord('Push', 'running', 'Executing push-safe pipeline...'")
            && js_content.contains("depRunWithTimeout('push', { branch, remote }, 15 * 60 * 1000)"),
        "Missing push-stage anti-stall timeout guard"
    );
    assert!(
        js_content.contains("let dashRoutineCanResume = false;")
            && js_content.contains("function dashResumePostCommitRoutine()")
            && js_content.contains("Resume from Failed Stage")
            && js_content.contains("options?.resumeFromStep")
            && js_content.contains("Auto-Resume: queued from")
            && js_content.contains("autoResumeDepth"),
        "Missing resume-from-failed-stage routine flow"
    );
    assert!(
        js_content.contains("function routinePushLikelyNoop(")
            && js_content.contains("everything up-to-date")
            && js_content.contains("depRun('ci-trigger'")
            && js_content.contains("CI: TRIGGERED (workflow_dispatch fallback after no-op push)"),
        "Missing no-op push CI-trigger fallback"
    );

    let push_script = fs::read_to_string(Path::new("../../scripts/push_main.sh"))
        .expect("Failed to read push_main.sh");
    assert!(
        push_script.contains("GIT_TERMINAL_PROMPT=0")
            && push_script.contains("GCM_INTERACTIVE=Never")
            && push_script.contains("BatchMode=yes"),
        "Missing non-interactive push guardrails in push_main.sh"
    );
}
