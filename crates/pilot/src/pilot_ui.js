async function fetchJsonSafe(url, options = {}) {
  try {
    const res = await fetch(url, options);
    const text = await res.text();
    if (!text || text.trim() === '') return { ok: false, error: 'Empty response' };
    try {
      return JSON.parse(text);
    } catch (e) {
      return { ok: false, error: 'Malformed JSON response', raw: text };
    }
  } catch (err) {
    return { ok: false, error: err.message };
  }
}

// Global handler to make elements with role="button" actionable via Enter/Space
document.addEventListener('keydown', function(e) {
  if (e.key === 'Enter' || e.key === ' ') {
    const el = e.target;
    if (el && el.getAttribute('role') === 'button') {
      e.preventDefault();
      el.click();
    }
  }
});

function copyToClipboard(id, btn) {
  const el = document.getElementById(id);
  if (!el) return;
  const text = el.textContent;
  navigator.clipboard.writeText(text).then(() => {
    const orig = btn.textContent;
    btn.textContent = 'COPIED';
    setTimeout(() => { btn.textContent = orig; }, 1500);
  });
}
function clearElement(id) {
  const el = document.getElementById(id);
  if (el) {
    if (id.includes('stream') || id.includes('mirror') || id.includes('logs')) {
      el.textContent = '[]';
    } else {
      el.textContent = '';
    }
  }
}
const out = {
  get textContent() { return ""; },
  set textContent(val) { console.log("Global Response (Hidden):", val); }
};
const liveStream = document.getElementById('live-stream');
const busStatusChip = document.getElementById('bus-status-chip');
const agorgOpenBtn = document.getElementById('agorg-open-btn');
const agorgStatusChip = document.getElementById('agorg-status-chip');
const opDetailMeta = document.getElementById('op-detail-meta');
const opDetailArtifact = document.getElementById('op-detail-artifact');
const opDetail = document.getElementById('op-detail');
const timelineEl = document.getElementById('timeline');
const failedOnlyToggle = document.getElementById('failed-only');
const timelineCommandFilter = document.getElementById('timeline-command-filter');
const timelineTextFilter = document.getElementById('timeline-text-filter');
const streamToggleBtn = document.getElementById('stream-toggle');
const oracleReportSelect = document.getElementById('oracle-report-select');
const oracleReportContent = document.getElementById('oracle-report-content');
const depActionOut = document.getElementById('dep-action-out');
const depActionOutGlobal = {
  get textContent() { return ""; },
  set textContent(val) { console.log("Dep Action Output (Hidden):", val); }
};
const depLogs = document.getElementById('dep-logs');
const depPolicyStatus = document.getElementById('dep-policy-status');
const depHookStatus = document.getElementById('dep-hook-status');
const depDriftStatus = document.getElementById('dep-drift-status');
const agorgRegistryList = document.getElementById('agorg-registry-list');
const agorgActiveDetails = document.getElementById('agorg-active-details');
const agorgActivityLog = document.getElementById('agorg-activity-log');
const multiRegistryChip = document.getElementById('multi-registry-chip');
const multiActionOut = document.getElementById('multi-action-out');
const multiGroupOptions = document.getElementById('multi-group-options');
const multiTagOptions = document.getElementById('multi-tag-options');
const multiOutputHtmlBtn = document.getElementById('multi-output-html');
const multiOutputJsonBtn = document.getElementById('multi-output-json');
const multiListBtn = document.getElementById('multi-list-btn');
const multiStatusBtn = document.getElementById('multi-status-btn');
const multiOrderBtn = document.getElementById('multi-order-btn');
const multiPrPlanBtn = document.getElementById('multi-pr-plan-btn');
const multiMacroLogOut = document.getElementById('multi-macro-log-out');
const multiMacroToggleBtn = document.getElementById('multi-macro-toggle-btn');
const multiScopeModal = document.getElementById('multi-scope-modal');
const multiDagVisual = document.getElementById('multi-dag-visual');
const multiDagVisualScroll = document.getElementById('multi-dag-visual-scroll');
const multiDagVisualEmpty = document.getElementById('multi-dag-visual-empty');
let repoAgoOptionsCache = [];

function logActivity(title, data) {
  if (!agorgActivityLog) return;
  const entry = document.createElement('div');
  entry.style.background = 'rgba(255,255,255,0.05)';
  entry.style.padding = '10px';
  entry.style.borderRadius = '6px';
  entry.style.borderLeft = '3px solid var(--accent)';
  
  const header = document.createElement('div');
  header.style.display = 'flex';
  header.style.justifyContent = 'space-between';
  header.style.marginBottom = '6px';
  
  const titleEl = document.createElement('strong');
  titleEl.textContent = title;
  
  const timeEl = document.createElement('small');
  timeEl.style.color = 'var(--text-muted)';
  timeEl.textContent = new Date().toLocaleTimeString();
  
  header.appendChild(titleEl);
  header.appendChild(timeEl);
  entry.appendChild(header);
  
  const body = document.createElement('div');
  body.style.whiteSpace = 'pre-wrap';
  body.style.fontFamily = 'monospace';
  body.style.fontSize = '0.9em';
  
  let isString = typeof data === 'string';
  let parsed = null;
  if (isString) {
      if (data.trim() === '(results cleared)' || data.trim() === '') return;
      try {
          if (data.trim().match(/^[\{\[]/)) parsed = JSON.parse(data);
      } catch (e) {}
  }
  let obj = parsed || data;

  if (obj && typeof obj === 'object') {
    let summary = [];
    for (const [key, val] of Object.entries(obj)) {
        if (Array.isArray(val)) {
            summary.push(`${key}: [${val.length} items]`);
        } else if (val && typeof val === 'object') {
            summary.push(`${key}: { ... }`);
        } else {
            summary.push(`${key}: ${val}`);
        }
    }
    body.textContent = summary.join('\n');
    
    const details = document.createElement('details');
    details.style.marginTop = '8px';
    const summaryEl = document.createElement('summary');
    summaryEl.textContent = 'Raw JSON';
    summaryEl.style.cursor = 'pointer';
    summaryEl.style.color = 'var(--text-muted)';
    const pre = document.createElement('pre');
    pre.style.marginTop = '4px';
    pre.style.padding = '8px';
    pre.style.background = 'rgba(0,0,0,0.3)';
    pre.style.borderRadius = '4px';
    pre.textContent = typeof data === 'string' ? data : JSON.stringify(data, null, 2);
    details.appendChild(summaryEl);
    details.appendChild(pre);
    body.appendChild(details);
  } else {
    body.textContent = String(data);
  }
  
  entry.appendChild(body);
  agorgActivityLog.appendChild(entry);
  agorgActivityLog.scrollTop = agorgActivityLog.scrollHeight;
}

const agorgOut = {
  get textContent() { return ""; },
  set textContent(val) { if (val) logActivity("System Response", val); },
  isActivityLog: true
};
const agorgDiscoveryOut = {
  get textContent() { return ""; },
  set textContent(val) { if (val) logActivity("Discovery Action", val); },
  isActivityLog: true
};

const agorgDiscoveryReview = document.getElementById('agorg-discovery-review');
const agorgPolicyReportSelect = document.getElementById('agorg-policy-report-select');
const agorgReconcileClass = document.getElementById('agorg-reconcile-class');

const codexOut = document.getElementById('codex-out');
const codexContractsOut = document.getElementById('codex-contracts-out');
const codexContractSelect = document.getElementById('codex-contract-select');
const telemetryMirror = document.getElementById('telemetry-mirror');
const dashStatusOut = document.getElementById('dash-status-out');
const dashPolicyChip = document.getElementById('dash-policy-chip');
const dashHookChip = document.getElementById('dash-hook-chip');
const dashDriftChip = document.getElementById('dash-drift-chip');
const dashBusChip = document.getElementById('dash-bus-chip');
const dashDbChip = document.getElementById('dash-db-chip');
const dashGateChip = document.getElementById('dash-gate-chip');
const dashPushChip = document.getElementById('dash-push-chip');
const dashRoutineScopeChip = document.getElementById('dash-routine-scope-chip');
const dashRoutineMultiChip = document.getElementById('dash-routine-multi-chip');
const dashRoutineGatesChip = document.getElementById('dash-routine-gates-chip');
const dashRoutinePushChip = document.getElementById('dash-routine-push-chip');
const dashRoutineEvidenceChip = document.getElementById('dash-routine-evidence-chip');
const dashRoutineCiDocsChip = document.getElementById('dash-routine-ci-docs-chip');
const dashRoutineCiRustChip = document.getElementById('dash-routine-ci-rust-chip');
const dashRoutineCiUiChip = document.getElementById('dash-routine-ci-ui-chip');
const dashRoutineCiPackagingChip = document.getElementById('dash-routine-ci-packaging-chip');
const dashRoutineProfileSourceChip = document.getElementById('dash-routine-profile-source-chip');
const dashRoutineProfileStepsChip = document.getElementById('dash-routine-profile-steps-chip');
const dashRoutineModeChip = document.getElementById('dash-routine-mode-chip');
const dashRoutineLastResultChip = document.getElementById('dash-routine-last-result-chip');
const dashRoutineStageStatusChip = document.getElementById('dash-routine-stage-status-chip');
const dashRoutineOut = document.getElementById('dash-routine-out');
const dashRoutineRunBtn = document.getElementById('dash-routine-run-btn');
const dashRoutineTimeline = document.getElementById('dash-routine-timeline');
const dashRoutineActions = document.getElementById('dash-routine-actions');
const dashRoutineStagePanel = document.getElementById('dash-routine-stage-panel');
const dashRoutineWorkspaceTitle = document.getElementById('dash-routine-workspace-title');
const dashRoutineWorkspaceSummary = document.getElementById('dash-routine-workspace-summary');
const dashRoutineWorkspaceChipRow = document.getElementById('dash-routine-workspace-chip-row');
const dashRoutineWorkspaceMetrics = document.getElementById('dash-routine-workspace-metrics');
const dashRoutineWorkspaceDetails = document.getElementById('dash-routine-workspace-details');
const dashRoutineWorkspaceArtifacts = document.getElementById('dash-routine-workspace-artifacts');
const dashRoutineWorkspaceNotes = document.getElementById('dash-routine-workspace-notes');
const dashRoutineDagView = document.getElementById('dash-routine-dag-view');
const dashRoutineDagLanes = document.getElementById('dash-routine-dag-lanes');
const dashRoutineDagSummaryChip = document.getElementById('dash-routine-dag-summary-chip');
const dashRoutineScopeSummary = document.getElementById('dash-routine-scope-summary');
const dashRoutinePlanSummary = document.getElementById('dash-routine-plan-summary');
const dashRoutineBranchInput = document.getElementById('dash-routine-branch');
const dashRoutineRemoteInput = document.getElementById('dash-routine-remote');
const dashRoutineCiDynamicList = document.getElementById('dash-routine-ci-dynamic-list');
const dashRoutineCiNotes = document.getElementById('dash-routine-ci-notes');
const dashRoutineCiPolicySummary = document.getElementById('dash-routine-ci-policy-summary');
const dashRoutinePolicyModal = document.getElementById('dash-routine-policy-modal');
const dashRoutinePolicyEditor = document.getElementById('dash-routine-policy-editor');
const dashRoutinePolicyModalStatus = document.getElementById('dash-routine-policy-modal-status');
const dashRoutineLiveStatus = document.getElementById('dash-routine-live-status');
const dashRoutineLiveAlert = document.getElementById('dash-routine-live-alert');
const dashReleaseReadinessChip = document.getElementById('dash-release-readiness-chip');
const dashReleaseCompatChip = document.getElementById('dash-release-compat-chip');
const dashReleaseMigrationChip = document.getElementById('dash-release-migration-chip');
const dashReleasePushChip = document.getElementById('dash-release-push-chip');
const dashReleaseBundleChip = document.getElementById('dash-release-bundle-chip');
const dashReleaseVerifyChip = document.getElementById('dash-release-verify-chip');
const dashReleaseScoreChip = document.getElementById('dash-release-score-chip');
const dashReleaseOut = document.getElementById('dash-release-out');
const dashReleaseRunBtn = document.getElementById('dash-release-run-btn');
const dashOracleChip = document.getElementById('dash-oracle-chip');
const dashHealChip = document.getElementById('dash-heal-chip');
const dashAgorgScoreChip = document.getElementById('dash-agorg-score-chip');
const dashAgorgIssuesChip = document.getElementById('dash-agorg-issues-chip');
const dashAgorgOffpolicyChip = document.getElementById('dash-agorg-offpolicy-chip');
const dashAgorgOverviewOut = document.getElementById('dash-agorg-overview-out');
const dashTempComponentsOut = document.getElementById('dash-temp-components-out');
const dashTempChecklistOut = document.getElementById('dash-temp-checklist-out');
const dashAcceptanceMatrixOut = document.getElementById('dash-acceptance-matrix-out');
const dashAgorgReportSelect = document.getElementById('dash-agorg-report-select');
const dashAgorgReconcileClass = document.getElementById('dash-agorg-reconcile-class');
const dashAgorgPolicyOut = document.getElementById('dash-agorg-policy-out');
const dashAgorgDuplicatesOut = document.getElementById('dash-agorg-duplicates-out');
const dashAgorgDupKindFilter = document.getElementById('dash-agorg-dup-kind-filter');
const dashAgorgFilteredDuplicatesOut = document.getElementById('dash-agorg-filtered-duplicates-out');
const dashAgorgDuplicateDetailOut = document.getElementById('dash-agorg-duplicate-detail-out');
const dashAgorgClassCountsOut = document.getElementById('dash-agorg-class-counts-out');
const dashAgorgParityOut = document.getElementById('dash-agorg-parity-out');
const dashAgorgContractOut = document.getElementById('dash-agorg-contract-out');
const dashAgorgIssueClassFilter = document.getElementById('dash-agorg-issue-class-filter');
const dashAgorgFilteredIssuesOut = document.getElementById('dash-agorg-filtered-issues-out');
const dashAgorgIssueDetailOut = document.getElementById('dash-agorg-issue-detail-out');
const dashOracleScanBtn = document.getElementById('dash-oracle-scan-btn');
const dashOracleQueryBtn = document.getElementById('dash-oracle-query-btn');
const dashHealPlanBtn = document.getElementById('dash-heal-plan-btn');
const dashHealRunBtn = document.getElementById('dash-heal-run-btn');
const multiDagChip = document.getElementById('multi-dag-chip');
const multiApplyChip = document.getElementById('multi-apply-chip');
const multiDagBtn = document.getElementById('multi-dag-btn');
const multiApplyDryBtn = document.getElementById('multi-apply-dry-btn');
const multiApplyExecBtn = document.getElementById('multi-apply-exec-btn');
const branchLogList = document.getElementById('branch-log-list');
const branchLogSummary = document.getElementById('branch-log-summary');
const branchLogLimitInput = document.getElementById('branch-log-limit');
const branchMatrixBody = document.getElementById('branch-matrix-body');
const branchMatrixSummary = document.getElementById('branch-matrix-summary');
const branchMatrixRefreshBtn = document.getElementById('branch-matrix-refresh-btn');
const branchMatrixSourceChip = document.getElementById('branch-matrix-source-chip');
const branchMatrixAdvanced = document.getElementById('branch-matrix-advanced');
const branchPreviewState = document.getElementById('branch-preview-state');
const branchPruneModal = document.getElementById('branch-prune-modal');
const branchPruneConfirmInput = document.getElementById('branch-prune-confirm-input');
const branchDagChip = document.getElementById('branch-dag-chip');
const branchApplyChip = document.getElementById('branch-apply-chip');
const branchDagBtn = document.getElementById('branch-dag-btn');
const branchApplyPreviewBtn = document.getElementById('branch-apply-preview-btn');
const branchApplyExecBtn = document.getElementById('branch-apply-exec-btn');
const branchCreateChip = document.getElementById('branch-create-chip');
const branchSyncChip = document.getElementById('branch-sync-chip');
const branchPruneChip = document.getElementById('branch-prune-chip');
const branchStatusChip = document.getElementById('branch-status-chip');
const settingsStatusOut = document.getElementById('settings-status-out');
const settingsStatusPanel = document.getElementById('settings-status-panel');
const branchCreatePreviewBtn = document.getElementById('branch-create-preview-btn');
const branchCreateExecBtn = document.getElementById('branch-create-exec-btn');
const branchSyncPreviewBtn = document.getElementById('branch-sync-preview-btn');
const branchSyncExecBtn = document.getElementById('branch-sync-exec-btn');
const branchPrunePreviewBtn = document.getElementById('branch-prune-preview-btn');
const branchPruneExecBtn = document.getElementById('branch-prune-exec-btn');
const branchStatusBtn = document.getElementById('branch-status-btn');
const oracleChip = document.getElementById('oracle-chip');
const oracleScanBtn = document.getElementById('oracle-scan-btn');
const oracleQueryBtn = document.getElementById('oracle-query-btn');
const healChip = document.getElementById('heal-chip');
const healPlanBtn = document.getElementById('heal-plan-btn');
const healRunBtn = document.getElementById('heal-run-btn');
const BUS_HEALTH_KEY = 'pilot.bus.health.v1';
const timelineState = new Map();
let selectedOperationId = null;
let auditCache = [];
let streamPaused = false;
let streamHandle = null;
let latestCodexContractId = '';
let currentTab = 'dashboard';
let branchMatrixRows = [];
let branchSelectedRepoIds = new Set();
let currentBranchScope = null;
let branchPreviewTokens = { create: null, sync: null, prune: null };
let branchPreviewData = { create: null, sync: null, prune: null };
let branchLogItems = [];
let agorgDiscoveryCache = null;
let agorgApprovedPaths = new Set();
let restoringUiSession = false;
let uiSessionSaveTimer = null;
let agorgCache = { at: 0, items: [], active: null, recent: [], instanceId: 'unknown' };
let dashAgorgIssueFilterState = { issues: [], classFilter: 'all', selectedIndex: 0 };
let agorgReconcileState = { report: null, dryRunTokenByClass: {} };
const AGORG_CACHE_TTL_MS = 8000;
const BRANCH_LOG_LIMIT_KEY = 'pilot.branch.log.limit.v1';
let agorgDefaultScopeCandidate = null;
let multiOutputMode = 'html';
let multiActionLast = null;
let multiMacroRunning = false;
let multiMacroExpanded = false;
let dashRoutineRunning = false;
let dashRoutineAutoHealRunning = false;
let dashRoutineAutoHealAttempted = false;
let dashRoutineCanResume = false;
let dashRoutineTrace = [];
let dashRoutineSelectedStage = 'resolve';
let dashRoutinePolicySimulationId = '';
let dashRoutinePolicySimulationFingerprint = '';
let routineCiSyncTimer = null;
let routineCiSyncBusy = false;
let routineCiAutoResumeBusy = false;
let codexActionBusy = false;
let codexActionQueue = Promise.resolve();
let routineCodexAutoRunning = false;
let dashRoutineCodexAuto = { active: false, entries: [], summary: '' };
const ROUTINE_HEAL_LOG_KEY = 'pilot.routine.heal.log.v1';
const ROUTINE_HEAL_RECIPE_KEY = 'pilot.routine.heal.recipe.v1';
const ROUTINE_CI_CONTINUATION_KEY = 'pilot.routine.ci.continuation.v1';
const ROUTINE_SAFE_HEAL_ACTIONS = Object.freeze(new Set(['cargo-fmt', 'repair']));
const ROUTINE_STAGE_ORDER = Object.freeze(['resolve', 'plan', 'multi', 'gates', 'push', 'ci', 'evidence', 'reconcile']);
const ROUTINE_STAGE_LABELS = Object.freeze({
  resolve: 'Resolve',
  plan: 'Plan',
  multi: 'Multi',
  gates: 'Gates',
  push: 'Push',
  ci: 'CI',
  evidence: 'Evidence',
  reconcile: 'Reconcile'
});
let dashRoutineWorkspaceState = {
  loaded: null,
  resolveSnapshot: null,
  active: null,
  scope: null,
  stats: null,
  multiSnapshot: null,
  gateDetail: null,
  pushDetail: null,
  pushNoop: false,
  ciDetail: null,
  ciCatalog: null,
  ciSelectedWorkflowKey: '',
  ciInFlight: false,
  healStatus: '',
  evidenceDetail: null,
  lastResult: 'idle',
  lastRunAt: null,
  failure: null
};
const ROUTINE_DEFAULT_PROFILE = Object.freeze({
  step_order: ['scope', 'multi', 'gates', 'push', 'ci', 'evidence'],
  stop_on_fail: true,
  include_push_step: true,
  export_evidence_step: false
});

function activatePanel(tabName, opts = {}) {
  const persist = opts.persist !== false;
  currentTab = tabName;
  for (const t of document.querySelectorAll('.tab')) t.classList.remove('active');
  for (const p of document.querySelectorAll('.panel')) p.classList.remove('active');
  const panel = document.getElementById(tabName);
  if (panel) panel.classList.add('active');
  const tabBtn = document.querySelector('.tab[data-tab="' + tabName + '"]');
  if (tabBtn) tabBtn.classList.add('active');
  if (tabName === 'agorg') {
    // Ensure AGOrg panel always reflects current saved state when opened.
    agorgShowActive();
    agorgList();
    agorgTree();
    agorgLoadPolicyReports();
  }
  if (tabName === 'branch') {
    branchLoadMatrix();
  }
  if (tabName === 'dashboard') {
    unifiedTimelineLoad();
    routineStartCiSyncLoop();
  } else {
    routineStopCiSyncLoop();
  }
  if (tabName === 'settings') {
    settingsRefreshTargetOptions().then(() => {
      settingsReloadPolicyControls();
      settingsLoadExceptions();
    });
  }
  if (tabName === 'multi') {
    multiLoadSelectorOptions();
    multiRefreshRegistry();
  }
  if (['oracle', 'heal', 'dependencies', 'multi'].includes(tabName)) {
    fetchJsonSafe('/api/agorg/active').then(res => {
      const container = document.getElementById(tabName + '-empty-state');
      const hasActiveScope = !!(res && res.ok && res.active && res.active.id);
      if (container) {
        if (!hasActiveScope) {
          container.innerHTML = `
            <div style="background:rgba(255, 215, 0, 0.1); border:1px solid rgba(255, 215, 0, 0.4); padding:12px; border-radius:8px; margin-bottom:16px;">
              <strong>No active AGOrg detected.</strong><br>
              <span style="font-size:0.85em; color:var(--text-muted);">These operations require a target repository context.</span>
              <div style="margin-top:8px;">
                <strong>Next Steps:</strong> Go to the <a href="#" onclick="activatePanel('agorg'); return false;" style="color:var(--accent);text-decoration:underline;">AGOrg tab</a> to select or import a master directory.
              </div>
            </div>`;
          container.style.display = 'block';
        } else {
          container.innerHTML = '';
          container.style.display = 'none';
        }
      }
    });
  }
  if (tabName === 'multi') {
    multiRefreshRegistry();
  }
  if (persist && !restoringUiSession) queueUiSessionSave();
}

for (const btn of document.querySelectorAll('.tab')) {
  btn.addEventListener('click', () => activatePanel(btn.dataset.tab));
}
if (agorgReconcileClass) {
  agorgReconcileClass.addEventListener('change', () => {
    syncReconcileClassControls(agorgReconcileClass.value || '');
    renderReconcileParitySummary();
  });
}
if (dashAgorgReconcileClass) {
  dashAgorgReconcileClass.addEventListener('change', () => {
    syncReconcileClassControls(dashAgorgReconcileClass.value || '');
    renderReconcileParitySummary();
  });
}

function readInputValue(id) {
  const el = document.getElementById(id);
  return el ? String(el.value || '') : '';
}

function readInputChecked(id) {
  const el = document.getElementById(id);
  return !!(el && el.checked);
}

function setVal(id, val) {
  const el = document.getElementById(id);
  if (el) {
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {
      el.value = val;
    } else {
      el.textContent = val;
    }
  }
}

function collectUiSessionState() {
  const subTabs = {};
  document.querySelectorAll('.card .sub-tabs').forEach((container, idx) => {
    const activeBtn = container.querySelector('.sub-tab.active');
    const panelId = activeBtn ? (activeBtn.getAttribute('onclick') || '') : '';
    subTabs['group_' + idx] = panelId;
  });
  return {
    active_tab: currentTab,
    agorg_use_id: document.getElementById('agorg-use-id') ? document.getElementById('agorg-use-id').value : null,
    agorg_master: readInputValue('agorg-master'),
    agorg_root: readInputValue('agorg-root'),
    agorg_name: readInputValue('agorg-name'),
    agorg_depth: readInputValue('agorg-depth'),
    agorg_default: readInputChecked('agorg-default'),
    agorg_prune: readInputChecked('agorg-prune'),
    agorg_profile_name: readInputValue('agorg-profile-name'),
    agorg_pref_default_branch: readInputValue('agorg-pref-default-branch'),
    agorg_pref_release_branch: readInputValue('agorg-pref-release-branch'),
    agorg_pref_auto_prune: readInputChecked('agorg-pref-auto-prune'),
    multi_group: readInputValue('multi-group'),
    multi_tags: readInputValue('multi-tags'),
    multi_branch: readInputValue('multi-apply-branch'),
    multi_stage_size: readInputValue('multi-apply-stage-size'),
    branch_matrix_group: readInputValue('branch-matrix-group'),
    branch_matrix_tags: readInputValue('branch-matrix-tags'),
    branch_matrix_search: readInputValue('branch-matrix-search'),
    branch_matrix_base: readInputValue('branch-matrix-base'),
    routine_group: readInputValue('dash-routine-group'),
    routine_tags: readInputValue('dash-routine-tags'),
    routine_branch: readInputValue('dash-routine-branch'),
    routine_remote: readInputValue('dash-routine-remote'),
    routine_allow_push: readInputChecked('dash-routine-allow-push'),
    routine_export_evidence: readInputChecked('dash-routine-export-evidence'),
    routine_auto_heal: readInputChecked('dash-routine-auto-heal'),
    routine_auto_codex: readInputChecked('dash-routine-auto-codex'),
    branch_matrix_advanced_open: !!(branchMatrixAdvanced && branchMatrixAdvanced.open),
    branch_log_limit: readInputValue('branch-log-limit'),
    codex_contract_id: readInputValue('codex-contract-id'),
    sub_tabs: subTabs
  };
}

function applyUiSessionState(session) {
  if (!session || typeof session !== 'object') return;
  const setVal = (id, value) => {
    if (value === undefined || value === null) return;
    const el = document.getElementById(id);
    if (el) el.value = String(value);
  };
  const setCheck = (id, value) => {
    if (value === undefined || value === null) return;
    const el = document.getElementById(id);
    if (el) el.checked = !!value;
  };
  if (session.active_tab) {
    activatePanel(session.active_tab, { persist: false });
  }
  if (document.getElementById('agorg-use-id')) {
    setVal('agorg-use-id', session.agorg_use_id);
  }
  setVal('agorg-master', session.agorg_master);
  setVal('agorg-name', session.agorg_name);
  setVal('agorg-profile-name', session.agorg_profile_name);
  setVal('agorg-pref-default-branch', session.agorg_pref_default_branch);
  setVal('agorg-pref-release-branch', session.agorg_pref_release_branch);
  setCheck('agorg-pref-auto-prune', session.agorg_pref_auto_prune);
  setVal('multi-group', session.multi_group);
  setVal('multi-tags', session.multi_tags);
  setVal('multi-apply-branch', session.multi_branch);
  setVal('multi-apply-stage-size', session.multi_stage_size);
  setVal('branch-matrix-group', session.branch_matrix_group);
  setVal('branch-matrix-tags', session.branch_matrix_tags);
  setVal('branch-matrix-search', session.branch_matrix_search);
  setVal('branch-matrix-base', session.branch_matrix_base);
  setVal('dash-routine-group', session.routine_group);
  setVal('dash-routine-tags', session.routine_tags);
  setVal('dash-routine-branch', session.routine_branch);
  setVal('dash-routine-remote', session.routine_remote);
  setCheck('dash-routine-allow-push', session.routine_allow_push);
  setCheck('dash-routine-export-evidence', session.routine_export_evidence);
  setCheck('dash-routine-auto-heal', session.routine_auto_heal);
  setCheck('dash-routine-auto-codex', session.routine_auto_codex);
  if (branchMatrixAdvanced && session.branch_matrix_advanced_open !== undefined && session.branch_matrix_advanced_open !== null) {
    branchMatrixAdvanced.open = !!session.branch_matrix_advanced_open;
  }
  setVal('branch-log-limit', session.branch_log_limit);
  setVal('codex-contract-id', session.codex_contract_id);
}

function queueUiSessionSave() {
  if (restoringUiSession) return;
  if (uiSessionSaveTimer) clearTimeout(uiSessionSaveTimer);
  uiSessionSaveTimer = setTimeout(() => {
    uiSessionSaveTimer = null;
    persistUiSession().catch(() => {});
  }, 300);
}

async function persistUiSession() {
  const session = collectUiSessionState();
  await fetch('/api/ui/session', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ session })
  });
}

async function hydrateScopeSnapshot(force = false) {
  const age = Date.now() - agorgCache.at;
  if (!force && age < AGORG_CACHE_TTL_MS && agorgCache.items.length > 0) {
    return agorgCache;
  }
  const data = await fetchJsonSafe('/api/agorg/scope_snapshot');
  if (data && data.ok) {
    agorgCache = {
      at: Date.now(),
      items: Array.isArray(data.agorgs) ? data.agorgs : [],
      active: data.active || null,
      recent: Array.isArray(data.recent_scopes) ? data.recent_scopes : [],
      instanceId: data.instance_id || 'unknown',
      uiSession: data.ui_session || {}
    };
  }
  return agorgCache;
}

// Hero AGOrg Dropdown Logic
async function toggleAgorgDropdown(event) {
  event.stopPropagation();
  const dropdown = document.getElementById('agorg-hero-dropdown-container');
  const isActive = dropdown.classList.contains('active');
  
  // Close all other dropdowns if any
  document.querySelectorAll('.agorg-dropdown').forEach(d => d.classList.remove('active'));
  
  if (!isActive) {
    dropdown.classList.add('active');
    await loadAgorgQuickNav();
  }
}

async function loadAgorgQuickNav() {
  const dropdown = document.getElementById('agorg-hero-dropdown');
  dropdown.innerHTML = '<div class="agorg-drop-header">Loading registered repositories...</div>';
  
  try {
    const snapshot = await hydrateScopeSnapshot();
    const agData = snapshot.items || [];
    const activeId = snapshot.active && snapshot.active.id ? snapshot.active.id : '';
    const recentIds = new Set((snapshot.recent || []).map((r) => r.id));
    
    let html = '<div class="agorg-drop-item" style="font-weight:700;color:#6a7dff;" onclick="activatePanel(\'agorg\'); agorgShowActive();">⚙ Manage AGOrgs / Panel</div>';
    
    if (agData && agData.length > 0) {
      // Filter to only show registered AGOrgs in the quick nav (exclude flat AGOs if they somehow appear here)
      const orgsOnly = agData.filter(ag => !ag.repo_path || (ag.id && !ag.name.startsWith('AGO:'))); // Basic guard, hydrateScopeSnapshot.items usually contains the Orgs
      
      html += '<div class="agorg-drop-header">AGOrgs</div>';
      orgsOnly.forEach(ag => {
        const isActive = ag.id === activeId;
        const badgeText = isActive ? 'ACTIVE' : (recentIds.has(ag.id) ? 'RECENT' : 'ORG');
        
        let badgeStyle = 'font-size:0.6rem; font-weight:700; padding:2px 6px; border-radius:4px;';
        if (isActive) {
          badgeStyle += ' background:#00d1ff; color:#001a33; box-shadow: 0 0 10px rgba(0, 209, 255, 0.6); animation: pulse-blue 2s infinite;';
        } else if (recentIds.has(ag.id)) {
          badgeStyle += ' background:#1c2635; color:#a8b9e3;';
        } else {
          badgeStyle += ' background:#1c2635; color:#6a7dff; opacity:0.7;';
        }

        html += `<div class="agorg-drop-item" onclick="switchAgorgScope('${ag.id}')">
          <span style="${isActive ? 'color:#fff; font-weight:600;' : ''}">${ag.name}</span>
          <span style="${badgeStyle}">${badgeText}</span>
        </div>`;
      });
    }

    dropdown.innerHTML = html || '<div class="agorg-drop-header">No registered repositories found.</div>';
  } catch (err) {
    console.error("QuickNav Error:", err);
    dropdown.innerHTML = `<div class="agorg-drop-header" style="color:#ff6b6b;">Error: ${err.message}</div>`;
  }
}

async function switchAgorgScope(id) {
  const req = { agorg: id };
  const res = await fetch('/api/agorg/use', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  await hydrateScopeSnapshot(true);
  refreshAgorgHeader();
  if (data && data.ok) {
    await refreshPolicyHookDriftChips();
  }
  agorgTree();
  agorgShowActive();
  await loadRepoAgoOptions();

  if (currentTab === 'branch') {
    await branchLoadMatrix();
  }
  if (currentTab === 'settings') {
    await settingsRefreshTargetOptions();
    await settingsReloadPolicyControls();
    await settingsLoadExceptions();
  }
  queueUiSessionSave();
  p5ResetRailState(); // P5: clear orchestration rail state on scope switch
  document.getElementById('agorg-hero-dropdown').classList.remove('active');
}

async function refreshPolicyHookDriftChips() {
  await depRun('policy');
  await depRun('hook-policy');
  await depRun('drift');
}

// Global click to close dropdown
document.addEventListener('click', () => {
  document.querySelectorAll('.agorg-dropdown').forEach(d => d.classList.remove('active'));
});

// Update the initial listener for agorgOpenBtn if it exists
if (agorgOpenBtn) {
  // It's now handled by inline onclick toggleAgorgDropdown(event)
}

function tags(v) { return v.split(',').map(s => s.trim()).filter(Boolean); }
function setButtonBusy(btn, busy, runningLabel) {
  if (!btn) return;
  if (!btn.dataset.defaultLabel) {
    btn.dataset.defaultLabel = btn.textContent || '';
  }
  btn.disabled = !!busy;
  if (busy && runningLabel) {
    btn.textContent = runningLabel;
  } else {
    btn.textContent = btn.dataset.defaultLabel;
  }
}

function setChipState(chip, label, state, suffix) {
  if (!chip) return;
  let level = 'neutral';
  if (state === 'running') level = 'warn';
  if (state === 'success') level = 'ok';
  if (state === 'failed') level = 'fail';
  chip.className = 'chip ' + level;
  const detail = suffix ? (': ' + suffix) : '';
  chip.textContent = label + detail;
  // Add accessibility attributes for screen readers and tooltips
  const tooltipText = getBranchSourceTooltip(suffix);
  chip.setAttribute('title', tooltipText || label + detail + ' - Click for details');
  chip.setAttribute('aria-label', label + ' chip: Current state is ' + (suffix || 'unknown'));

  // Announce the state change to screen readers
  let announcer = document.getElementById('aria-announcer');
  if (!announcer) {
    announcer = document.createElement('div');
    announcer.id = 'aria-announcer';
    announcer.setAttribute('aria-live', 'polite');
    announcer.setAttribute('class', 'sr-only');
    announcer.style.position = 'absolute';
    announcer.style.width = '1px';
    announcer.style.height = '1px';
    announcer.style.padding = '0';
    announcer.style.margin = '-1px';
    announcer.style.overflow = 'hidden';
    announcer.style.clip = 'rect(0, 0, 0, 0)';
    announcer.style.whiteSpace = 'nowrap';
    announcer.style.border = '0';
    document.body.appendChild(announcer);
  }
  announcer.textContent = `${label} status is now ${state} ${suffix || ''}`;
}

// Tooltip explanations for branch source types
function getBranchSourceTooltip(source) {
  const tooltips = {
    'registry': 'Data from local registry',
    'bootstrapped': 'Auto-created from current scope',
    'autodiscovered': 'Imported from discovered AGOrg repositories',
    'empty': 'No branches found. Try adjusting filters or refresh.'
  };
  return tooltips[source] || null;
}

// Display inline error message with role="alert" for accessibility
function showInlineError(message, containerEl = null, nextSteps = null) {
  const targetEl = containerEl || out;
  const errorDiv = document.createElement('div');
  errorDiv.className = 'error-message';
  errorDiv.setAttribute('role', 'alert');
  errorDiv.setAttribute('aria-live', 'assertive');
  let html = '<strong>Error:</strong> ' + message;
  if (nextSteps) {
    html += '<div style="margin-top:6px; font-size:0.85em; background:rgba(255,46,46,0.1); border:1px solid rgba(255,46,46,0.2); padding:6px; border-radius:4px;"><strong>Next Steps:</strong> ' + nextSteps + '</div>';
  } else {
    html += '<br><small>Please try again or adjust your settings.</small>';
  }
  errorDiv.innerHTML = html;
  // Clear previous error and insert new one
  const existing = targetEl.querySelector('.error-message');
  if (existing) existing.remove();
  targetEl.insertBefore(errorDiv, targetEl.firstChild);
  // Also log the error for visibility
  console.error('[Error]', message);
}

function clearInlineError(containerEl = null) {
  const targetEl = containerEl || out;
  if (!targetEl) return;
  const existing = targetEl.querySelector('.error-message');
  if (existing) existing.remove();
}

async function run(command, payload, opts = {}) {
  const label = opts.label || command;
  const chip = opts.chip || null;
  const buttons = Array.isArray(opts.buttons) ? opts.buttons : [];
  const outputEl = opts.outputEl || out;
  const mirrorDashboard = opts.mirrorDashboard !== false;
  payload.schema_version = 1;
  setChipState(chip, label, 'running', 'running');
  for (const b of buttons) setButtonBusy(b, true, opts.runningLabel || null);
  const runningText = JSON.stringify({status: "running", command, payload}, null, 2);
  outputEl.textContent = runningText;
  out.textContent = runningText;
  if (dashStatusOut && mirrorDashboard) {
    dashStatusOut.textContent = out.textContent;
  }
  try {
    const ctl = new AbortController();
    const timeoutMs = Number.isFinite(opts.timeoutMs) ? opts.timeoutMs : 90000;
    const timeoutId = setTimeout(() => ctl.abort(), timeoutMs);
    const res = await fetch('/api/orchestrate/run', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify({ domain: 'command', payload: { command, payload } }),
      signal: ctl.signal
    });
    clearTimeout(timeoutId);
    const data = await res.json();
    const resultText = JSON.stringify(data, null, 2);
    outputEl.textContent = resultText;
    out.textContent = resultText;
    if (dashStatusOut && mirrorDashboard) {
      dashStatusOut.textContent = resultText;
    }
    const ok = !!data.ok;
    setChipState(chip, label, ok ? 'success' : 'failed', ok ? 'success' : 'failed');
    appendLive({ source: 'ui_command', command, ok: !!data.ok, status: res.status });
    loadHistory();
    return data;
  } catch (err) {
    const msg = (err && err.name === 'AbortError')
      ? 'Request timed out waiting for API response. If this is a control-plane command, check server logs for fallback/local execution details.'
      : (err && err.message ? err.message : String(err));
    const payloadErr = { ok: false, error: msg, command };
    const errorText = JSON.stringify(payloadErr, null, 2);
    outputEl.textContent = errorText;
    out.textContent = errorText;
    if (dashStatusOut && mirrorDashboard) dashStatusOut.textContent = errorText;
    setChipState(chip, label, 'failed', 'failed');
    appendLive({ source: 'ui_command', command, ok: false, error: msg });
    return payloadErr;
  } finally {
    for (const b of buttons) setButtonBusy(b, false, null);
  }
}

function appendLive(eventObj) {
  const current = liveStream.textContent.trim();
  let arr = [];
  if (current && current !== '[]') {
    try { arr = JSON.parse(current); } catch (_) { arr = []; }
  }
  arr.push(eventObj);
  if (arr.length > 120) arr = arr.slice(arr.length - 120);
  liveStream.textContent = JSON.stringify(arr, null, 2);
  if (telemetryMirror) {
    const tail = arr.slice(Math.max(0, arr.length - 20));
    telemetryMirror.textContent = JSON.stringify(tail, null, 2);
  }
  ingestTimeline(eventObj);
}

function clearLive() {
  liveStream.textContent = '[]';
  if (telemetryMirror) telemetryMirror.textContent = '[]';
}

function syncTelemetryMirror() {
  if (!telemetryMirror) return;
  const current = liveStream.textContent.trim();
  try {
    const arr = current ? JSON.parse(current) : [];
    const tail = Array.isArray(arr) ? arr.slice(Math.max(0, arr.length - 20)) : [];
    telemetryMirror.textContent = JSON.stringify(tail, null, 2);
  } catch (_) {
    telemetryMirror.textContent = current || '[]';
  }
}

function setBusStatus(connected, note) {
  busStatusChip.textContent = connected ? 'CONNECTED' : 'DISCONNECTED';
  busStatusChip.classList.toggle('connected', connected);
  busStatusChip.classList.toggle('disconnected', !connected);
  if (dashBusChip) {
    setChip(dashBusChip, 'Bus: ' + (connected ? 'RUNNING' : 'STOPPED'), connected ? 'ok' : 'fail');
  }
  try {
    localStorage.setItem(BUS_HEALTH_KEY, JSON.stringify({
      connected,
      note: note || '',
      at: new Date().toISOString()
    }));
  } catch (_) {}
  if (note) {
    opDetailMeta.textContent = note;
  }
}

function setAgorgStatus(label, active) {
  if (!agorgStatusChip) return;
  agorgStatusChip.textContent = label;
  if (active) {
    agorgStatusChip.style.color = '#fff';
  } else {
    agorgStatusChip.style.color = '#a8b9e3';
  }
}

async function refreshAgorgHeader() {
  try {
    const snapshot = await hydrateScopeSnapshot();
    const active = snapshot && snapshot.active ? snapshot.active : null;
    if (active && active.name) {
      const label = active.name + (snapshot.instanceId ? ` (${snapshot.instanceId})` : '');
      setAgorgStatus(label, true);
    } else {
      const label = 'NO ACTIVE' + (snapshot.instanceId ? ` (${snapshot.instanceId})` : '');
      setAgorgStatus(label, false);
    }
  } catch (_) {
    setAgorgStatus('UNAVAILABLE', false);
  }
}

function restoreBusStatus() {
  try {
    const raw = localStorage.getItem(BUS_HEALTH_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (typeof parsed.connected === 'boolean') {
      setBusStatus(parsed.connected, parsed.note || '');
    }
  } catch (_) {}
}

function filteredTimelineItems() {
  const cmdNeedle = String(timelineCommandFilter.value || '').trim().toLowerCase();
  const textNeedle = String(timelineTextFilter.value || '').trim().toLowerCase();
  return Array.from(timelineState.values())
    .sort((a, b) => String(b.updatedAt).localeCompare(String(a.updatedAt)))
    .filter((x) => !failedOnlyToggle.checked || x.phase === 'failed')
    .filter((x) => !cmdNeedle || String(x.command || '').toLowerCase().includes(cmdNeedle))
    .filter((x) => {
      if (!textNeedle) return true;
      const hay = [
        x.opId || '',
        x.command || '',
        ...(x.steps || []).map((s) => s.summary || '')
      ].join(' ').toLowerCase();
      return hay.includes(textNeedle);
    });
}

function exportTimeline() {
  const items = filteredTimelineItems();
  const payload = {
    exported_at: new Date().toISOString(),
    filters: {
      failed_only: !!failedOnlyToggle.checked,
      command_contains: timelineCommandFilter.value || '',
      text_contains: timelineTextFilter.value || ''
    },
    items
  };
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'pilot_timeline_export.json';
  a.click();
  URL.revokeObjectURL(url);
}

function extractTimelineRecord(evt) {
  if (!evt || typeof evt !== 'object') return null;

  if (typeof evt.eventType === 'string' && evt.eventType.startsWith('pilot.op.')) {
    const payload = evt.payload || {};
    const opId = payload.operation_id || payload.operationId;
    if (!opId) return null;
    return {
      opId,
      phase: evt.eventType.replace('pilot.op.', '') || 'progress',
      command: payload.command || 'unknown',
      summary: payload.summary || '',
      at: payload.timestamp || new Date().toISOString()
    };
  }

  if (evt.source === 'ui_command' && typeof evt.command === 'string') {
    const success = !!(evt.response && evt.response.success);
    return {
      opId: (evt.response && evt.response.reply_to) || ('ui-' + Date.now()),
      phase: success ? 'completed' : 'failed',
      command: evt.command,
      summary: evt.error || (evt.response && evt.response.data && evt.response.data.summary) || '',
      at: new Date().toISOString()
    };
  }

  if (typeof evt.source === 'string' && typeof evt.action === 'string') {
    const phase = evt.ok === false ? 'failed' : 'completed';
    const opId = evt.artifact_path
      ? ('artifact:' + String(evt.artifact_path))
      : ((evt.source + ':' + evt.action + ':' + Date.now()));
    const summary = evt.artifact_path
      ? ('artifact=' + evt.artifact_path)
      : (evt.error || evt.message || '');
    return {
      opId,
      phase,
      command: evt.source + '.' + evt.action,
      summary,
      at: new Date().toISOString()
    };
  }

  return null;
}

function ingestTimeline(evt) {
  const rec = extractTimelineRecord(evt);
  if (!rec) return;

  const current = timelineState.get(rec.opId) || {
    opId: rec.opId,
    command: rec.command,
    phase: 'started',
    updatedAt: rec.at,
    steps: [],
    rawEvents: []
  };

  current.command = rec.command || current.command;
  current.phase = rec.phase || current.phase;
  current.updatedAt = rec.at || current.updatedAt;
  current.steps.push({
    phase: rec.phase,
    summary: rec.summary || '',
    at: rec.at || new Date().toISOString()
  });
  current.rawEvents.push(evt);
  if (current.steps.length > 10) current.steps = current.steps.slice(current.steps.length - 10);
  if (current.rawEvents.length > 20) current.rawEvents = current.rawEvents.slice(current.rawEvents.length - 20);

  timelineState.set(rec.opId, current);
  if (!selectedOperationId) selectedOperationId = rec.opId;
  renderTimeline();
  renderOperationDetail();
}

function renderTimeline() {
  timelineEl.innerHTML = '';
  const items = filteredTimelineItems().slice(0, 40);

  if (!items.length) {
    const empty = document.createElement('div');
    empty.className = 'tl-empty';
    empty.setAttribute('role', 'status');
    empty.setAttribute('aria-live', 'polite');
    empty.innerHTML = '<p>Timeline is empty. Branches with activity will appear here.</p>';
    timelineEl.appendChild(empty);
    return;
  }

  for (const item of items) {
    const card = document.createElement('div');
    card.className = 'tl-card';
    if (item.opId === selectedOperationId) {
      card.classList.add('selected');
    }
    card.addEventListener('click', () => {
      selectedOperationId = item.opId;
      renderTimeline();
      renderOperationDetail();
    });

    const head = document.createElement('div');
    head.className = 'tl-head';

    const title = document.createElement('div');
    title.className = 'tl-title';
    title.textContent = item.command + ' (' + item.opId + ')';

    const badge = document.createElement('span');
    const phaseClass = ['started', 'progress', 'completed', 'failed'].includes(item.phase) ? item.phase : 'progress';
    badge.className = 'tl-badge ' + phaseClass;
    badge.textContent = String(item.phase).toUpperCase();

    head.appendChild(title);
    head.appendChild(badge);
    const artifactPath = inferArtifactPath(item);
    if (artifactPath) {
      const artifactBadge = document.createElement('span');
      artifactBadge.className = 'tl-badge progress';
      artifactBadge.style.marginLeft = '8px';
      artifactBadge.textContent = 'ARTIFACT';
      head.appendChild(artifactBadge);
    }
    card.appendChild(head);

    const steps = document.createElement('ul');
    steps.className = 'tl-steps';
    for (const step of item.steps.slice().reverse()) {
      const li = document.createElement('li');
      const msg = step.summary ? ' - ' + step.summary : '';
      li.textContent = '[' + step.at + '] ' + step.phase + msg;
      steps.appendChild(li);
    }
    card.appendChild(steps);

    timelineEl.appendChild(card);
  }
}

function shortCommand(cmd) {
  if (!cmd) return '';
  return cmd.startsWith('pilot.') ? cmd.slice(6) : cmd;
}

function inferArtifactPath(item) {
  const raw = Array.isArray(item && item.rawEvents) ? item.rawEvents : [];
  for (let i = raw.length - 1; i >= 0; i--) {
    const ev = raw[i] || {};
    if (typeof ev.artifact_path === 'string' && ev.artifact_path.trim()) {
      return ev.artifact_path.trim();
    }
    if (ev.response && typeof ev.response.artifact_path === 'string' && ev.response.artifact_path.trim()) {
      return ev.response.artifact_path.trim();
    }
  }
  const cmd = shortCommand(item.command);
  for (let i = auditCache.length - 1; i >= 0; i--) {
    const ev = auditCache[i] || {};
    if (ev.command === cmd && ev.artifact_path) {
      return ev.artifact_path;
    }
  }
  return '';
}

function renderOperationDetail() {
  const item = selectedOperationId ? timelineState.get(selectedOperationId) : null;
  if (!item) {
    opDetailMeta.textContent = 'Select a timeline item';
    opDetailArtifact.textContent = '';
    opDetail.textContent = '[]';
    return;
  }
  opDetailMeta.textContent = item.command + ' | ' + item.opId + ' | phase=' + item.phase;
  const artifact = inferArtifactPath(item);
  opDetailArtifact.textContent = artifact ? ('Artifact: ' + artifact) : 'Artifact: (not resolved)';
  opDetail.textContent = JSON.stringify(item.rawEvents || [], null, 2);
}

function extractArtifactPathFromJsonText(text) {
  if (!text || !text.trim()) return '';
  try {
    const parsed = JSON.parse(text);
    const direct = parsed && typeof parsed.artifact_path === 'string' ? parsed.artifact_path.trim() : '';
    if (direct) return direct;
    const responsePath = parsed && parsed.response && typeof parsed.response.artifact_path === 'string'
      ? parsed.response.artifact_path.trim()
      : '';
    if (responsePath) return responsePath;
    const contractRespPath = parsed && parsed.contract && parsed.contract.execute_response
      && typeof parsed.contract.execute_response.artifact_path === 'string'
      ? parsed.contract.execute_response.artifact_path.trim()
      : '';
    return contractRespPath || '';
  } catch (_) {
    return '';
  }
}

async function openSelectedTimelineArtifact() {
  const item = selectedOperationId ? timelineState.get(selectedOperationId) : null;
  const path = item ? inferArtifactPath(item) : '';
  if (!path) {
    const msg = JSON.stringify({ ok: false, error: 'No artifact linked to selected timeline item.' }, null, 2);
    opDetail.textContent = msg;
    out.textContent = msg;
    return;
  }
  await openReportPath(path, opDetail, out);
}

async function dashAgorgContractOpenArtifact() {
  const path = extractArtifactPathFromJsonText(dashAgorgContractOut ? dashAgorgContractOut.textContent : '');
  if (!path) {
    const msg = JSON.stringify({ ok: false, error: 'No artifact_path found in AGOrg contract output.' }, null, 2);
    if (dashAgorgContractOut) dashAgorgContractOut.textContent = msg;
    out.textContent = msg;
    return;
  }
  await openReportPath(path, dashAgorgContractOut, out);
}

failedOnlyToggle.addEventListener('change', renderTimeline);
timelineCommandFilter.addEventListener('input', renderTimeline);
timelineTextFilter.addEventListener('input', renderTimeline);

function branchScopeFilters() {
  const groupEl = document.getElementById('branch-matrix-group');
  const tagEl = document.getElementById('branch-matrix-tags');
  const group = (groupEl && groupEl.value ? groupEl.value : '').trim();
  const tagRaw = (tagEl && tagEl.value ? tagEl.value : '').trim();
  return {
    group: group || null,
    tags: tags(tagRaw)
  };
}

function branchSelectedIdsArray() {
  return Array.from(branchSelectedRepoIds).map(x => Number(x)).filter(x => Number.isFinite(x));
}

function branchMatrixRequest() {
  const filters = branchScopeFilters();
  return {
    group: filters.group,
    tags: filters.tags,
    search: (document.getElementById('branch-matrix-search').value || '').trim() || null,
    base_branch: (document.getElementById('branch-matrix-base').value || document.getElementById('sync-base').value || 'main').trim() || 'main',
    target_branch: (document.getElementById('branch-name').value || '').trim() || null
  };
}

function refreshBranchPreviewState() {
  if (!branchPreviewState) return;
  const parts = [];
  let blockedByConflicts = false;

  for (const action of ['create', 'sync', 'prune']) {
    if (branchPreviewTokens[action]) {
      const data = branchPreviewData[action];
      let details = `<div style="margin-top: 8px;"><strong>${action}</strong> preview ready.</div>`;

      // Conflict Radar Rendering
      if (data && data.conflicts && data.conflicts.has_conflicts) {
        blockedByConflicts = true;
        details += `<div class="warn" style="margin-top: 4px;">🚨 <strong>Conflicts Detected:</strong> ${data.conflicts.conflict_count} repos. Execution blocked.</div>`;
        const c_list = data.conflicts.results.filter(r => r.has_conflicts).map(r => 
          `<div><strong>${r.repo}</strong>: ${r.conflicting_files.join(', ')}</div>`
        ).join('');
        details += `<div style="font-size: 0.85em; margin-left: 10px; color: var(--text-muted);">${c_list}</div>`;
      }

      // Confirmation Gate Rendering
      if (data && data.confirmation_required && data.confirmation_required.type !== 'None' && data.confirmation_required.type !== 'Standard') {
        const ctype = data.confirmation_required.type;
        const phrase = data.confirmation_required.phrase || 'CONFIRM';
        if (ctype === 'TypedPhrase' || ctype === 'DoubleConfirm') {
           details += `<div style="margin-top: 8px; padding: 8px; border: 1px solid var(--border); border-radius: 4px;">
              <div class="helper" style="color: var(--color-warn);">⚠️ Protected branch policy requires typed confirmation.</div>
              <input type="text" id="branch-confirm-${action}" placeholder="Type '${phrase}' to confirm" style="width: 100%; border-color: var(--color-warn);" onkeyup="checkBranchConfirm('${action}', '${phrase}')" />
            </div>`;
        }
      }
      parts.push(details);
    }
  }

  branchPreviewState.innerHTML = parts.length
    ? parts.join('')
    : 'No active preview token.';

  // Disable exec buttons if blocked by conflicts or needing typed confirm
  const btns = {
    create: document.getElementById('branch-create-exec-btn'),
    sync: document.getElementById('branch-sync-exec-btn'),
    prune: document.getElementById('branch-prune-exec-btn')
  };
  
  for (const [action, btn] of Object.entries(btns)) {
    if (!btn) continue;
    if (blockedByConflicts && branchPreviewTokens[action]) {
       btn.disabled = true;
       btn.title = "Blocked by conflicts";
       continue;
    }
    const data = branchPreviewData[action];
    if (branchPreviewTokens[action] && data && data.confirmation_required) {
      const ctype = data.confirmation_required.type;
      if (ctype === 'TypedPhrase' || ctype === 'DoubleConfirm') {
         btn.disabled = true; // wait for checkBranchConfirm
         btn.title = "Requires typed confirmation";
      } else {
         btn.disabled = false;
         btn.title = "";
      }
    } else {
       btn.disabled = !branchPreviewTokens[action];
       btn.title = branchPreviewTokens[action] ? "" : "Run preview first";
    }
  }
}

window.checkBranchConfirm = function(action, requiredPhrase) {
  const input = document.getElementById(`branch-confirm-${action}`);
  const btn = document.getElementById(`branch-${action}-exec-btn`);
  if (!input || !btn) return;
  if (input.value === requiredPhrase) {
    btn.disabled = false;
    btn.title = "";
  } else {
    btn.disabled = true;
    btn.title = "Requires typed confirmation";
  }
};

function invalidateBranchPreviews(reason) {
  branchPreviewTokens = { create: null, sync: null, prune: null };
  branchPreviewData = { create: null, sync: null, prune: null };
  refreshBranchPreviewState();
  if (reason) {
    branchAddLogEntry({
      title: 'Preview invalidated',
      phase: 'info',
      summary: reason,
      payload: { status: 'preview_invalidated', reason }
    });
  }
}

function getBranchLogLimit() {
  const rawInput = branchLogLimitInput ? parseInt(branchLogLimitInput.value || '50', 10) : 50;
  let limit = Number.isFinite(rawInput) ? rawInput : 50;
  limit = Math.max(1, Math.min(100, limit));
  return limit;
}

function persistBranchLogLimit() {
  const limit = getBranchLogLimit();
  if (branchLogLimitInput) branchLogLimitInput.value = String(limit);
  try { localStorage.setItem(BRANCH_LOG_LIMIT_KEY, String(limit)); } catch (_) {}
  return limit;
}

function restoreBranchLogLimit() {
  try {
    const raw = localStorage.getItem(BRANCH_LOG_LIMIT_KEY);
    if (!raw) return;
    const parsed = parseInt(raw, 10);
    if (!Number.isFinite(parsed)) return;
    const limit = Math.max(1, Math.min(100, parsed));
    if (branchLogLimitInput) branchLogLimitInput.value = String(limit);
  } catch (_) {}
}

function branchRenderLog() {
  if (!branchLogList) return;
  branchLogList.innerHTML = '';
  if (!branchLogItems.length) {
    const empty = document.createElement('div');
    empty.className = 'muted';
    empty.setAttribute('role', 'status');
    empty.setAttribute('aria-live', 'polite');
    empty.innerHTML = '<p>No activity logged yet. Actions will appear here.</p>';
    branchLogList.appendChild(empty);
    if (branchLogSummary) branchLogSummary.textContent = '0 log entries';
    return;
  }
  for (const item of branchLogItems) {
    const entry = document.createElement('div');
    entry.className = 'branch-log-entry ' + (item.ok ? 'ok' : (item.phase === 'running' || item.phase === 'info' ? '' : 'fail'));
    const head = document.createElement('div');
    head.className = 'branch-log-head';
    const title = document.createElement('div');
    title.textContent = `${item.at} | ${item.title} | ${String(item.phase || 'completed').toUpperCase()}`;
    const actions = document.createElement('div');
    actions.className = 'branch-log-actions';
    const copyBtn = document.createElement('button');
    copyBtn.className = 'action-btn';
    copyBtn.textContent = 'COPY JSON';
    copyBtn.addEventListener('click', () => {
      navigator.clipboard.writeText(JSON.stringify(item.payload || {}, null, 2)).catch(() => {});
    });
    actions.appendChild(copyBtn);
    if (item.artifactPath) {
      const artifactBtn = document.createElement('button');
      artifactBtn.className = 'action-btn';
      artifactBtn.textContent = 'OPEN ARTIFACT';
      artifactBtn.addEventListener('click', async () => {
        await openReportPath(item.artifactPath, opDetail, out);
      });
      actions.appendChild(artifactBtn);
    }
    const details = document.createElement('details');
    const sum = document.createElement('summary');
    sum.textContent = 'Show JSON';
    const pre = document.createElement('pre');
    pre.style.maxHeight = '220px';
    pre.textContent = JSON.stringify(item.payload || {}, null, 2);
    details.appendChild(sum);
    details.appendChild(pre);
    const body = document.createElement('div');
    body.className = 'branch-log-body';
    body.textContent = item.summary || '';

    head.appendChild(title);
    head.appendChild(actions);
    entry.appendChild(head);
    entry.appendChild(body);
    entry.appendChild(details);
    branchLogList.appendChild(entry);
  }
  if (branchLogSummary) {
    branchLogSummary.textContent = `${branchLogItems.length} log entries (limit ${getBranchLogLimit()})`;
  }
}

function branchAddLogEntry({ title, phase, ok, summary, payload }) {
  const artifactPath = extractArtifactPathFromJsonText(JSON.stringify(payload || {}));
  branchLogItems.unshift({
    at: new Date().toISOString(),
    title: title || 'Branch',
    phase: phase || (ok ? 'completed' : 'failed'),
    ok: !!ok,
    summary: summary || '',
    payload: payload || {},
    artifactPath: artifactPath || ''
  });
  const limit = persistBranchLogLimit();
  if (branchLogItems.length > limit) branchLogItems = branchLogItems.slice(0, limit);
  branchRenderLog();
}

function branchClearHtmlLog() {
  branchLogItems = [];
  branchRenderLog();
}

function renderBranchMatrix() {
  if (!branchMatrixBody) return;
  branchMatrixBody.innerHTML = '';
  if (!branchMatrixRows.length) {
    const tr = document.createElement('tr');
    const td = document.createElement('td');
    td.colSpan = 10;
    td.setAttribute('role', 'status');
    td.setAttribute('aria-live', 'polite');
    td.innerHTML = '<p>No branches found. Try adjusting filters or refresh the matrix.</p>' +
      '<button class="action-btn" onclick="refreshBranchMatrix()" title="Click to refresh the branch matrix">Refresh Matrix</button>';
    tr.appendChild(td);
    branchMatrixBody.appendChild(tr);
    if (branchMatrixSummary) branchMatrixSummary.textContent = '0 repos shown, 0 selected';
    setChipState(branchMatrixSourceChip, 'Matrix Source', 'warn', 'empty');
    return;
  }
  let selectedVisible = 0;
  for (const row of branchMatrixRows) {
    const tr = document.createElement('tr');
    if (branchSelectedRepoIds.has(row.id)) {
      tr.classList.add('selected');
      selectedVisible += 1;
    }

    const selTd = document.createElement('td');
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = branchSelectedRepoIds.has(row.id);
    cb.addEventListener('change', () => {
      if (cb.checked) branchSelectedRepoIds.add(row.id);
      else branchSelectedRepoIds.delete(row.id);
      invalidateBranchPreviews('selection changed');
      renderBranchMatrix();
    });
    selTd.appendChild(cb);
    tr.appendChild(selTd);

    const cols = [
      row.repo,
      row.group || '-',
      (row.tags || []).join(','),
      row.current_branch || 'unknown',
      row.protected ? 'yes' : 'no',
      row.clean ? 'yes' : 'no',
      row.ahead == null ? '-' : String(row.ahead),
      row.behind == null ? '-' : String(row.behind),
      row.on_target == null ? '-' : (row.on_target ? 'yes' : 'no')
    ];
    for (const value of cols) {
      const td = document.createElement('td');
      td.textContent = value;
      tr.appendChild(td);
    }
    branchMatrixBody.appendChild(tr);
  }
  if (branchMatrixSummary) {
    branchMatrixSummary.textContent = `${branchMatrixRows.length} repos shown, ${branchSelectedRepoIds.size} selected (${selectedVisible} visible)`;
  }
}

function updateBranchMatrixSourceChip(data) {
  const source = String(data && data.source ? data.source : 'unknown').toLowerCase();
  if (source === 'autodiscovered') {
    setChipState(branchMatrixSourceChip, 'Matrix Source', 'warn', 'autodiscovered');
    return;
  }
  if (source === 'bootstrapped') {
    setChipState(branchMatrixSourceChip, 'Matrix Source', 'success', 'bootstrapped');
    return;
  }
  if (source === 'registry') {
    setChipState(branchMatrixSourceChip, 'Matrix Source', 'success', 'registry');
    return;
  }
  setChipState(branchMatrixSourceChip, 'Matrix Source', 'neutral', source);
}

async function branchLoadMatrix() {
  if (branchMatrixRefreshBtn) setButtonBusy(branchMatrixRefreshBtn, true, 'Loading...');
  const payload = branchMatrixRequest();
  branchAddLogEntry({
    title: 'Matrix refresh',
    phase: 'running',
    ok: true,
    summary: 'Loading branch matrix',
    payload: { status: 'running', endpoint: '/api/branch/matrix', payload }
  });
  try {
    const res = await fetch('/api/branch/matrix', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify(payload)
    });
    const data = await res.json();
    if (!data.ok) {
      branchAddLogEntry({
        title: 'Matrix refresh',
        phase: 'failed',
        ok: false,
        summary: data.error || 'Matrix request failed',
        payload: data
      });
      branchMatrixRows = [];
      branchSelectedRepoIds = new Set();
      renderBranchMatrix();
      return;
    }
    const rows = Array.isArray(data.rows) ? data.rows : [];
    const visibleIds = new Set(rows.map(r => r.id));
    branchSelectedRepoIds = new Set(Array.from(branchSelectedRepoIds).filter(id => visibleIds.has(id)));
    branchMatrixRows = rows;
    invalidateBranchPreviews('matrix refreshed');
    branchAddLogEntry({
      title: 'Matrix refresh',
      phase: 'completed',
      ok: true,
      summary: `Loaded ${rows.length} repos${Number(data.autodiscovered || 0) > 0 ? ` (auto-discovered ${Number(data.autodiscovered)} AGOs)` : ''}${Number(data.bootstrapped || 0) > 0 ? ` (bootstrapped ${Number(data.bootstrapped)} from AGOrg)` : ''}`,
      payload: data
    });
    updateBranchMatrixSourceChip(data);
    renderBranchMatrix();
  } catch (err) {
    const msg = { ok: false, error: err && err.message ? err.message : String(err), endpoint: '/api/branch/matrix' };
    branchAddLogEntry({
      title: 'Matrix refresh',
      phase: 'failed',
      ok: false,
      summary: msg.error,
      payload: msg
    });
    setChipState(branchMatrixSourceChip, 'Matrix Source', 'fail', 'error');
    appendLive({ source: 'branch_matrix', error: String(err) });
  } finally {
    if (branchMatrixRefreshBtn) setButtonBusy(branchMatrixRefreshBtn, false, 'Refresh Matrix');
  }

  // Fire P4 loads
  branchUndoJournalLoad();
  branchTimelineLoad();
}

function branchSelectVisible() {
  for (const row of branchMatrixRows) branchSelectedRepoIds.add(row.id);
  invalidateBranchPreviews('selection changed');
  renderBranchMatrix();
}

function branchClearSelection() {
  branchSelectedRepoIds.clear();
  invalidateBranchPreviews('selection changed');
  renderBranchMatrix();
}

function branchCreatePayload(dryRun) {
  const selectors = branchScopeFilters();
  return {
    action: 'create',
    branch: document.getElementById('branch-name').value,
    base_branch: document.getElementById('branch-base').value,
    dry_run: !!dryRun,
    group: selectors.group,
    tags: selectors.tags,
    selected_repo_ids: branchSelectedIdsArray()
  };
}

function branchSyncPayload(dryRun) {
  const selectors = branchScopeFilters();
  return {
    action: 'sync',
    branch: document.getElementById('sync-branch').value,
    base_branch: document.getElementById('sync-base').value,
    dry_run: !!dryRun,
    group: selectors.group,
    tags: selectors.tags,
    selected_repo_ids: branchSelectedIdsArray()
  };
}

function branchPrunePayload(dryRun, confirmPhrase) {
  const selectors = branchScopeFilters();
  const payload = {
    action: 'prune',
    base_branch: document.getElementById('sync-base').value,
    dry_run: !!dryRun,
    group: selectors.group,
    tags: selectors.tags,
    selected_repo_ids: branchSelectedIdsArray()
  };
  if (!dryRun && confirmPhrase) payload.confirm_phrase = confirmPhrase;
  return payload;
}

async function runBranchAction(payload, opts = {}) {
  const label = opts.label || 'Branch';
  const chip = opts.chip || null;
  const buttons = Array.isArray(opts.buttons) ? opts.buttons : [];
  const action = String(payload.action || '').trim().toLowerCase();
  const isMutatingAction = ['create', 'sync', 'prune'].includes(action);
  const isExecute = isMutatingAction && payload.dry_run === false;

  if (isExecute) {
    const token = branchPreviewTokens[action];
    if (!token) {
      const msg = { ok: false, error: 'No preview token for execute. Run Preview first.', action };
      branchAddLogEntry({
        title: `${action} execute blocked`,
        phase: 'failed',
        ok: false,
        summary: msg.error,
        payload: msg
      });
      setChipState(chip, label, 'failed', 'failed');
      return msg;
    }
    payload.preview_token = token;
  }

  setChipState(chip, label, 'running', 'running');
  for (const b of buttons) setButtonBusy(b, true, opts.runningLabel || null);
  branchAddLogEntry({
    title: `${action} ${payload.dry_run ? 'preview' : 'execute'}`,
    phase: 'running',
    ok: true,
    summary: 'Running branch action',
    payload: { status: 'running', endpoint: '/api/branch/run', payload }
  });
  try {
    const res = await fetch('/api/orchestrate/run', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify({ domain: 'branch', payload })
    });
    const data = await res.json();
    const ok = !!data.ok;
    if (ok && isMutatingAction && payload.dry_run === true && data.preview_token) {
      branchPreviewTokens[action] = data.preview_token;
      branchPreviewData[action] = data;
      refreshBranchPreviewState();
    } else if (ok && isExecute) {
      branchPreviewTokens[action] = null;
      branchPreviewData[action] = null;
      refreshBranchPreviewState();
    } else if (!ok && isExecute) {
      branchPreviewTokens[action] = null;
      refreshBranchPreviewState();
    }
    setChipState(chip, label, ok ? 'success' : 'failed', ok ? 'success' : 'failed');
    branchAddLogEntry({
      title: `${action} ${payload.dry_run ? 'preview' : 'execute'}`,
      phase: ok ? 'completed' : 'failed',
      ok,
      summary: ok
        ? (data.failures != null ? `repo_count=${data.repo_count || 0}, failures=${data.failures}` : 'success')
        : (data.error || 'Branch action failed'),
      payload: data
    });
    appendLive({ source: 'branch_control', action: payload.action, ok, status: res.status, dry_run: !!payload.dry_run });
    loadHistory();
    if (ok && (isExecute || action === 'status')) await branchLoadMatrix();
    return data;
  } catch (err) {
    const msg = err && err.message ? err.message : String(err);
    branchAddLogEntry({
      title: `${action} ${payload.dry_run ? 'preview' : 'execute'}`,
      phase: 'failed',
      ok: false,
      summary: msg,
      payload: { ok: false, error: msg, payload }
    });
    setChipState(chip, label, 'failed', 'failed');
    appendLive({ source: 'branch_control', action: payload.action, ok: false, error: msg });
    return { ok: false, error: msg };
  } finally {
    for (const b of buttons) setButtonBusy(b, false, null);
  }
}

async function runBranchBusCommand(command, payload, opts = {}) {
  const label = opts.label || command;
  const chip = opts.chip || null;
  const buttons = Array.isArray(opts.buttons) ? opts.buttons : [];
  setChipState(chip, label, 'running', 'running');
  for (const b of buttons) setButtonBusy(b, true, opts.runningLabel || null);
  branchAddLogEntry({
    title: `${label} run`,
    phase: 'running',
    ok: true,
    summary: `command=${command}`,
    payload: { status: 'running', endpoint: '/api/command', command, payload }
  });
  try {
    const res = await fetch('/api/command', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify({ command, payload })
    });
    const data = await res.json();
    const ok = !!data.ok;
    setChipState(chip, label, ok ? 'success' : 'failed', ok ? 'success' : 'failed');
    branchAddLogEntry({
      title: `${label} run`,
      phase: ok ? 'completed' : 'failed',
      ok,
      summary: ok ? 'success' : (data.error || 'command failed'),
      payload: data
    });
    appendLive({ source: 'branch_control', action: label.toLowerCase(), ok, status: res.status, command });
    loadHistory();
    return data;
  } catch (err) {
    const msg = err && err.message ? err.message : String(err);
    setChipState(chip, label, 'failed', 'failed');
    branchAddLogEntry({
      title: `${label} run`,
      phase: 'failed',
      ok: false,
      summary: msg,
      payload: { ok: false, error: msg, command, payload }
    });
    appendLive({ source: 'branch_control', action: label.toLowerCase(), ok: false, error: msg, command });
    return { ok: false, error: msg };
  } finally {
    for (const b of buttons) setButtonBusy(b, false, null);
  }
}

function branchApplyPayload(apply) {
  const selectors = branchScopeFilters();
  const stageSizeRaw = parseInt(document.getElementById('branch-apply-stage-size').value || '2', 10);
  const stageSize = Number.isFinite(stageSizeRaw) && stageSizeRaw > 0 ? stageSizeRaw : 2;
  return {
    branch: document.getElementById('branch-apply-branch').value || 'feat/pilot-wave13',
    base_branch: document.getElementById('branch-apply-base').value || 'dev',
    pr_base_branch: document.getElementById('branch-apply-pr-base').value || 'main',
    group: selectors.group,
    tags: selectors.tags,
    stage_size: stageSize,
    continue_on_failure: !!document.getElementById('branch-apply-continue').checked,
    apply: !!apply
  };
}

function branchDagPreview() {
  const selectors = branchScopeFilters();
  runBranchBusCommand('pilot.multi.dag', {
    group: selectors.group,
    tags: selectors.tags,
    dry_run: true
  }, {
    label: 'DAG',
    chip: branchDagChip,
    buttons: [branchDagBtn],
    runningLabel: 'Running...'
  });
}

function branchApplyPreview() {
  const payload = branchApplyPayload(false);
  runBranchBusCommand('pilot.multi.apply', payload, {
    label: 'Staged Apply',
    chip: branchApplyChip,
    buttons: [branchApplyPreviewBtn, branchApplyExecBtn],
    runningLabel: 'Running...'
  });
}

function branchApplyExecute() {
  const payload = branchApplyPayload(true);
  runBranchBusCommand('pilot.multi.apply', payload, {
    label: 'Staged Apply',
    chip: branchApplyChip,
    buttons: [branchApplyPreviewBtn, branchApplyExecBtn],
    runningLabel: 'Running...'
  });
}

function branchCreatePreview() {
  runBranchAction(branchCreatePayload(true), {
    label: 'Create',
    chip: branchCreateChip,
    buttons: [branchCreatePreviewBtn, branchCreateExecBtn],
    runningLabel: 'Running...'
  });
}

function branchCreateExecute() {
  runBranchAction(branchCreatePayload(false), {
    label: 'Create',
    chip: branchCreateChip,
    buttons: [branchCreatePreviewBtn, branchCreateExecBtn],
    runningLabel: 'Running...'
  });
}

function branchSyncPreview() {
  runBranchAction(branchSyncPayload(true), {
    label: 'Sync',
    chip: branchSyncChip,
    buttons: [branchSyncPreviewBtn, branchSyncExecBtn],
    runningLabel: 'Running...'
  });
}

function branchSyncExecute() {
  runBranchAction(branchSyncPayload(false), {
    label: 'Sync',
    chip: branchSyncChip,
    buttons: [branchSyncPreviewBtn, branchSyncExecBtn],
    runningLabel: 'Running...'
  });
}

function branchPrunePreview() {
  runBranchAction(branchPrunePayload(true), {
    label: 'Prune',
    chip: branchPruneChip,
    buttons: [branchPrunePreviewBtn, branchPruneExecBtn],
    runningLabel: 'Running...'
  });
}

function branchPruneExecute() {
  if (branchPruneModal) {
    branchPruneModal.classList.add('open');
    if (branchPruneConfirmInput) {
      branchPruneConfirmInput.value = '';
      branchPruneConfirmInput.focus();
    }
    return;
  }
  // fallback if modal is unavailable
  const typed = prompt('Type PRUNE to confirm destructive prune execute');
  if (String(typed || '').trim().toUpperCase() !== 'PRUNE') {
    branchAddLogEntry({
      title: 'prune execute blocked',
      phase: 'failed',
      ok: false,
      summary: 'Prune execute cancelled: confirmation phrase mismatch.',
      payload: {
        ok: false,
        action: 'prune',
        error: 'Prune execute cancelled: confirmation phrase mismatch.'
      }
    });
    return;
  }
  runBranchAction(branchPrunePayload(false, 'PRUNE'), {
    label: 'Prune',
    chip: branchPruneChip,
    buttons: [branchPrunePreviewBtn, branchPruneExecBtn],
    runningLabel: 'Running...'
  });
}

function branchCancelPruneConfirm() {
  if (!branchPruneModal) return;
  branchPruneModal.classList.remove('open');
}

function branchConfirmPruneExecute() {
  const typed = String(branchPruneConfirmInput ? branchPruneConfirmInput.value : '').trim().toUpperCase();
  if (typed !== 'PRUNE') {
    branchAddLogEntry({
      title: 'prune execute blocked',
      phase: 'failed',
      ok: false,
      summary: 'Prune execute blocked: type PRUNE to confirm.',
      payload: {
        ok: false,
        action: 'prune',
        error: 'Prune execute blocked: type PRUNE to confirm.'
      }
    });
    return;
  }
  if (branchPruneModal) branchPruneModal.classList.remove('open');
  runBranchAction(branchPrunePayload(false, 'PRUNE'), {
    label: 'Prune',
    chip: branchPruneChip,
    buttons: [branchPrunePreviewBtn, branchPruneExecBtn],
    runningLabel: 'Running...'
  });
}

function branchStatus() {
  const selectors = branchScopeFilters();
  runBranchAction({
    action: 'status',
    dry_run: true,
    group: selectors.group,
    tags: selectors.tags,
    selected_repo_ids: branchSelectedIdsArray()
  }, {
    label: 'Status',
    chip: branchStatusChip,
    buttons: [branchStatusBtn],
    runningLabel: 'Running...'
  });
}

function oracleScan() {
  run('pilot.oracle.scan', {}, {
    label: 'Oracle',
    chip: oracleChip,
    buttons: [oracleScanBtn, oracleQueryBtn],
    runningLabel: 'Running...'
  });
}

function oracleQuery() {
  run('pilot.oracle.query', {
    query: document.getElementById('oracle-query').value,
    cli: true
  }, {
    label: 'Oracle',
    chip: oracleChip,
    buttons: [oracleScanBtn, oracleQueryBtn],
    runningLabel: 'Running...'
  });
}

function dashOracleScan() {
  run('pilot.oracle.scan', {}, {
    label: 'Oracle',
    chip: dashOracleChip,
    buttons: [dashOracleScanBtn, dashOracleQueryBtn],
    runningLabel: 'Running...'
  });
}

function dashOracleQuery() {
  run('pilot.oracle.query', {
    query: document.getElementById('dash-oracle-query').value,
    cli: true
  }, {
    label: 'Oracle',
    chip: dashOracleChip,
    buttons: [dashOracleScanBtn, dashOracleQueryBtn],
    runningLabel: 'Running...'
  });
}

function healPayload(planOnly) {
  const maxAttemptsRaw = document.getElementById('heal-max-attempts').value;
  const maxFilesRaw = document.getElementById('heal-max-files').value;
  const maxAttempts = parseInt(maxAttemptsRaw || '2', 10);
  const maxFiles = parseInt(maxFilesRaw || '5', 10);
  return {
    log_file: document.getElementById('heal-log-file').value || 'test_output.json',
    target: document.getElementById('heal-target').value || null,
    max_attempts: Number.isFinite(maxAttempts) ? maxAttempts : 2,
    max_files: Number.isFinite(maxFiles) ? maxFiles : 5,
    verbose: !!document.getElementById('heal-verbose').checked,
    plan_only: !!planOnly
  };
}

function healPlan() {
  run('pilot.heal.run', healPayload(true), {
    label: 'Heal',
    chip: healChip,
    buttons: [healPlanBtn, healRunBtn],
    runningLabel: 'Running...'
  });
}

function healRun() {
  run('pilot.heal.run', healPayload(false), {
    label: 'Heal',
    chip: healChip,
    buttons: [healPlanBtn, healRunBtn],
    runningLabel: 'Running...'
  });
}

function dashHealPayload(planOnly) {
  const maxAttemptsRaw = document.getElementById('dash-heal-max-attempts').value;
  const maxFilesRaw = document.getElementById('dash-heal-max-files').value;
  const maxAttempts = parseInt(maxAttemptsRaw || '2', 10);
  const maxFiles = parseInt(maxFilesRaw || '5', 10);
  return {
    log_file: document.getElementById('dash-heal-log-file').value || 'test_output.json',
    target: document.getElementById('dash-heal-target').value || null,
    max_attempts: Number.isFinite(maxAttempts) ? maxAttempts : 2,
    max_files: Number.isFinite(maxFiles) ? maxFiles : 5,
    verbose: false,
    plan_only: !!planOnly
  };
}

function dashHealPlan() {
  run('pilot.heal.run', dashHealPayload(true), {
    label: 'Heal',
    chip: dashHealChip,
    buttons: [dashHealPlanBtn, dashHealRunBtn],
    runningLabel: 'Running...'
  });
}

function dashHealRun() {
  run('pilot.heal.run', dashHealPayload(false), {
    label: 'Heal',
    chip: dashHealChip,
    buttons: [dashHealPlanBtn, dashHealRunBtn],
    runningLabel: 'Running...'
  });
}

async function oracleLoadReports() {
  const res = await fetch('/api/reports?limit=200');
  const data = await res.json();
  const rows = (data && data.reports) ? data.reports : [];
  oracleReportSelect.innerHTML = '';
  if (!rows.length) {
    const opt = document.createElement('option');
    opt.value = '';
    opt.textContent = 'No report files found in ~/.pilot/reports';
    oracleReportSelect.appendChild(opt);
    oracleReportContent.textContent = 'No report files found.';
    return;
  }
  for (const row of rows) {
    const opt = document.createElement('option');
    opt.value = row.path;
    const kb = Math.max(1, Math.round((row.size_bytes || 0) / 1024));
    opt.textContent = row.path + ' (' + kb + ' KB)';
    oracleReportSelect.appendChild(opt);
  }
}

async function oracleViewReport() {
  const path = oracleReportSelect.value;
  if (!path) {
    oracleReportContent.textContent = 'No report selected.';
    return;
  }
  const res = await fetch('/api/report?path=' + encodeURIComponent(path) + '&max_bytes=524288');
  const data = await res.json();
  if (!data || !data.ok) {
    oracleReportContent.textContent = JSON.stringify(data, null, 2);
    return;
  }
  oracleReportContent.textContent = data.content || '';
}

async function depRun(action, options = {}) {
  const normalizedAction = action === 'gate' ? 'prepush-gate' : action;
  const isPreflight = ['policy', 'hook-policy', 'drift'].includes(normalizedAction);
  const req = { action: isPreflight ? 'preflight' : normalizedAction, json: false };
  if (isPreflight) {
    let step = normalizedAction;
    if (step === 'hook-policy') step = 'hook';
    req.preflight_steps = [step];
  }
  if (normalizedAction === 'push') {
    const { branch, remote } = routineReadBranchRemote();
    req.branch = branch;
    req.remote = remote;
  }
  if (options && typeof options === 'object') {
    if (typeof options.branch === 'string' && options.branch.trim() !== '') {
      req.branch = options.branch.trim();
    }
    if (typeof options.remote === 'string' && options.remote.trim() !== '') {
      req.remote = options.remote.trim();
    }
    if (typeof options.label === 'string' && options.label.trim() !== '') {
      req.label = options.label.trim();
    }
    if (typeof options.bundle_path === 'string' && options.bundle_path.trim() !== '') {
      req.bundle_path = options.bundle_path.trim();
    }
    if (Number.isFinite(options.ci_timeout_sec)) {
      req.ci_timeout_sec = Math.max(60, Math.min(7200, Math.floor(options.ci_timeout_sec)));
    }
  }
  const res = await fetch('/api/orchestrate/run', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ domain: 'dependency', payload: req })
  });
  const data = await res.json();
  const inner = (data && data.inner && typeof data.inner === 'object') ? data.inner : data;
  
  if (inner.action === 'preflight' && inner.report && Array.isArray(inner.report.steps)) {
    for (const s of inner.report.steps) {
      const stepName = s.step;
      const passed = s.result.status === 'Pass';
      const suffix = passed ? 'PASS' : 'FAIL';
      const level = passed ? 'ok' : 'fail';
      
      const parsedStruct = { ok: passed };
      if (!passed && s.result.failure_code) {
          parsedStruct.failed_checks = [s.result.failure_code];
      }
      
      if (stepName === 'Policy') {
          setDepStatus(depPolicyStatus, parsedStruct);
          setChip(dashPolicyChip, 'Policy: ' + suffix, level);
      }
      if (stepName === 'Hook') {
          setDepStatus(depHookStatus, parsedStruct);
          setChip(dashHookChip, 'Hook: ' + suffix, level);
      }
      if (stepName === 'Drift') {
          setDepDriftStatus(suffix);
          setChip(dashDriftChip, 'Drift: ' + suffix, level);
      }
      if (stepName === 'Gate') {
          setChip(dashGateChip, 'Gate: ' + suffix, level);
      }
      if (stepName === 'Push') {
          setChip(dashPushChip, 'Push: ' + suffix, level);
      }
      
      if (!passed && s.result.hint) {
          appendLive({ source: 'dashboard', action: stepName, error: s.result.hint });
      }
    }
  }
  if (isPreflight && (!inner || !inner.ok || !(inner.report && Array.isArray(inner.report.steps)))) {
    const err = (inner && inner.error) ? String(inner.error) : ((data && data.error) ? String(data.error) : 'preflight failed');
    if (action === 'policy') {
      setDepStatus(depPolicyStatus, { ok: false, failed_checks: [err] });
      setChip(dashPolicyChip, 'Policy: FAIL', 'fail');
    }
    if (action === 'hook-policy') {
      setDepStatus(depHookStatus, { ok: false, failed_checks: [err] });
      setChip(dashHookChip, 'Hook: FAIL', 'fail');
    }
    if (action === 'drift') {
      setDepDriftStatus('FAIL');
      setChip(dashDriftChip, 'Drift: FAIL', 'fail');
    }
  }

  if (normalizedAction.startsWith('bus-')) {
    const text = String(inner.stdout || '') + '\n' + String(inner.stderr || '');
    if (text.includes('RUNNING')) setBusStatus(true, 'bus shim reported RUNNING');
    if (text.includes('STOPPED')) setBusStatus(false, 'bus shim reported STOPPED');
  }
  if (normalizedAction === 'services-status' || normalizedAction === 'services-start' || normalizedAction === 'services-stop' || normalizedAction === 'services-restart') {
    if (typeof inner.bus_running === 'boolean') {
      setBusStatus(inner.bus_running, inner.bus_running ? 'service manager reported RUNNING' : 'service manager reported STOPPED');
    }
  }
  depActionOut.textContent = JSON.stringify(data, null, 2);
  if (depActionOutGlobal) {
    depActionOutGlobal.textContent = JSON.stringify(data, null, 2);
  }
  if (!isPreflight) {
      updateDashChip(normalizedAction, !!inner.ok, inner);
  }
  depLoadLogs();
  return data;
}

async function depRunWithTimeout(action, options = {}, timeoutMs = 0) {
  const timeout = Number(timeoutMs);
  if (!Number.isFinite(timeout) || timeout <= 0) {
    return depRun(action, options);
  }
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`${action} timed out after ${Math.floor(timeout / 1000)}s`)), timeout);
    depRun(action, options)
      .then((data) => {
        clearTimeout(timer);
        resolve(data);
      })
      .catch((err) => {
        clearTimeout(timer);
        reject(err);
      });
  });
}

function routinePushLikelyNoop(pushRes) {
  const inner = depEnvelopeInner(pushRes) || {};
  const stdout = String(inner?.stdout || pushRes?.stdout || '');
  const summary = inner?.summary && typeof inner.summary === 'object' ? inner.summary : {};
  const merged = `${stdout}\n${JSON.stringify(summary)}`.toLowerCase();
  return merged.includes('everything up-to-date') || merged.includes('everything up to date');
}

function setChip(el, label, level) {
  if (!el) return;
  el.textContent = label;
  el.className = 'chip ' + level;
}

function updateDashChip(action, ok, data) {
  const suffix = ok ? 'PASS' : 'FAIL';
  const level = ok ? 'ok' : 'fail';
  if (action === 'policy') setChip(dashPolicyChip, 'Policy: ' + suffix, level);
  if (action === 'hook-policy') setChip(dashHookChip, 'Hook: ' + suffix, level);
  if (action === 'drift') setChip(dashDriftChip, 'Drift: ' + suffix, level);
  if (action === 'bus-status' || action === 'bus-start' || action === 'bus-stop' || action === 'bus-restart') {
    setChip(dashBusChip, 'Bus: ' + (ok ? 'RUNNING' : 'STOPPED'), ok ? 'ok' : 'fail');
  }
  if (action === 'db-status' || action === 'db-start' || action === 'db-restart') {
    setChip(dashDbChip, 'DB: ' + (ok ? 'RUNNING' : 'STOPPED'), ok ? 'ok' : 'fail');
  }
  if (action === 'db-stop') {
    setChip(dashDbChip, 'DB: ' + (ok ? 'STOPPED' : 'RUNNING'), ok ? 'ok' : 'fail');
  }
  if (action === 'services-status' || action === 'services-start' || action === 'services-stop' || action === 'services-restart') {
    if (typeof data.bus_running === 'boolean') {
      setChip(dashBusChip, 'Bus: ' + (data.bus_running ? 'RUNNING' : 'STOPPED'), data.bus_running ? 'ok' : 'fail');
    }
    if (typeof data.db_running === 'boolean') {
      setChip(dashDbChip, 'DB: ' + (data.db_running ? 'RUNNING' : 'STOPPED'), data.db_running ? 'ok' : 'fail');
    }
  }
  if (action === 'gate') setChip(dashGateChip, 'Gate: ' + suffix, level);
  if (action === 'prepush-gate') setChip(dashGateChip, 'Gate: ' + suffix, level);
  if (action === 'push') setChip(dashPushChip, 'Push: ' + suffix, level);
  if (!ok && data && data.error) {
    appendLive({ source: 'dashboard', action, error: data.error });
  }
}

function setDepStatus(el, parsed) {
  if (!parsed || typeof parsed !== 'object') {
    el.textContent = 'invalid response';
    el.className = 'dep-fail';
    return;
  }
  if (parsed.ok) {
    el.textContent = 'PASS';
    el.className = 'dep-ok';
    return;
  }
  const failed = Array.isArray(parsed.failed_checks) ? parsed.failed_checks.join(', ') : 'unknown';
  el.textContent = 'FAIL: ' + failed;
  el.className = 'dep-fail';
}

function setDepDriftStatus(text) {
  if (!depDriftStatus) return;
  const ok = text === 'PASS';
  depDriftStatus.textContent = text;
  depDriftStatus.className = ok ? 'dep-ok' : 'dep-fail';
}

async function depLoadLogs() {
  const res = await fetch('/api/dependencies/logs');
  const data = await res.json();
  depLogs.textContent = JSON.stringify(data, null, 2);
}

async function agorgCreateProject() {
  const pruneEl = document.getElementById('agorg-prune');
  const req = {
    name: document.getElementById('agorg-name').value.trim(),
    root: document.getElementById('agorg-root').value.trim(),
    master: document.getElementById('agorg-master').value.trim() || null,
    parent: null,
    scan_depth: parseInt(document.getElementById('agorg-depth').value || '4', 10),
    autoscan: true, // Always scan for fleet model
    import: true,   // Default to import for fleet model
    prune_missing: !!(pruneEl && pruneEl.checked),
    default_scope: !!document.getElementById('agorg-default').checked
  };
  const data = await fetchJsonSafe('/api/agorg/create_project', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  agorgOut.textContent = JSON.stringify(data, null, 2);
  if (data.discovery) {
    setDiscoveryCache(data.discovery);
  }
  if (data.ok && data.agorg && data.agorg.master_path) {
    agorgScanMaster(data.agorg.master_path);
  }
  refreshAgorgHeader();
  agorgList();
}

async function agorgScanMaster(path) {
  if (!path) path = document.getElementById('agorg-master').value.trim();
  if (!path) return;
  
  const res = await fetch('/api/agorg/scan_master', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ path })
  });
  const data = await res.json();
  if (data.ok) {
    renderHierarchy(data.items);
  } else {
    agorgOut.textContent = JSON.stringify(data, null, 2);
  }
}

function renderHierarchy(items) {
  const container = document.getElementById('agorg-hierarchy-tree');
  container.innerHTML = '';
  items.forEach(item => {
    const el = document.createElement('div');
    el.className = 'tree-node';
    if (item.kind === 'agorg') el.classList.add('node-agorg');
    else if (item.kind === 'ago') el.classList.add('node-ago');
    
    el.innerHTML = `
      <span class="icon">${item.kind === 'agorg' ? '🏢' : item.kind === 'ago' ? '📦' : '📁'}</span>
      <span class="name">${item.name}</span>
      <span class="status-dot ${item.is_registered ? 'registered' : 'unregistered'}"></span>
    `;
    
    el.onclick = () => {
      document.querySelectorAll('.tree-node').forEach(n => n.classList.remove('selected'));
      el.classList.add('selected');
      // Load into Panel 1
      document.getElementById('agorg-name').value = item.name;
      document.getElementById('agorg-root').value = item.path;
      // If it's none/unregistered, maybe show upgrade button?
      if (!item.is_registered) {
        agorgOut.textContent = `Directory: ${item.name}\nPath: ${item.path}\nStatus: Unregistered\nTip: Click "Import AGOrg" to register this as an Arqon entry.`;
      }
    };
    
    // Drag and Drop (Linkage)
    el.draggable = true;
    el.ondragstart = (e) => {
      e.dataTransfer.setData('text/plain', JSON.stringify(item));
    };
    el.ondragover = (e) => { e.preventDefault(); el.classList.add('drag-over'); };
    el.ondragleave = () => { el.classList.remove('drag-over'); };
    el.ondrop = (e) => {
      e.preventDefault();
      el.classList.remove('drag-over');
      const dragged = JSON.parse(e.dataTransfer.getData('text/plain'));
      if (dragged.path === item.path) return;
      
      if (confirm(`Link "${dragged.name}" as a child of "${item.name}"?`)) {
        agorgEditRelationship(item.path, null, [dragged.name]); // Simplified for now
      }
    };
    
    container.appendChild(el);
  });
}

async function agorgEditRelationship(path, parent, children) {
  const res = await fetch('/api/agorg/edit_relationship', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ path, parent, children })
  });
  const data = await res.json();
  agorgOut.textContent = JSON.stringify(data, null, 2);
}

async function agorgUpgradeAgo(path, name) {
  const res = await fetch('/api/agorg/upgrade_ago', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ path, name })
  });
  const data = await res.json();
  agorgOut.textContent = JSON.stringify(data, null, 2);
}

async function agorgUpdate() {
  const activeRes = await fetchJsonSafe('/api/agorg/active');
  if (!activeRes.ok || !activeRes.active) {
     agorgOut.textContent = "Error: No active AGOrg selected for update";
     return;
  }
  const active = activeRes.active;

  const newName = prompt("New Name:", active.name);
  if (newName === null) return;
  const newRoot = prompt("New Root Path:", active.root_path);
  if (newRoot === null) return;
  const newMaster = prompt("New Master Path (optional):", active.master_path || "");
  if (newMaster === null) return;

  const req = {
    id: active.id,
    name: newName.trim() || active.name,
    root: newRoot.trim() || active.root_path,
    master: newMaster.trim() || null
  };
  
  const res = await fetchJsonSafe('/api/agorg/update', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  agorgOut.textContent = JSON.stringify(res, null, 2);
  if (res.ok) {
    await hydrateScopeSnapshot(true);
    refreshAgorgHeader();
    agorgShowActive();
    agorgList();
  }
}

async function agorgDelete() {
  const activeRes = await fetchJsonSafe('/api/agorg/active');
  if (!activeRes.ok || !activeRes.active) {
     agorgOut.textContent = "Error: No active AGOrg selected for deletion";
     return;
  }
  const active = activeRes.active;

  if (!confirm(`Are you sure you want to delete AGOrg \"${active.name}\" (${active.id})? This will unregister it from Pilot but will NOT delete files on disk.`)) {
    return;
  }

  const res = await fetchJsonSafe('/api/agorg/delete', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ id: active.id })
  });
  
  agorgOut.textContent = JSON.stringify(res, null, 2);
  if (res.ok) {
    await hydrateScopeSnapshot(true);
    refreshAgorgHeader();
    agorgShowActive();
    agorgList();
    agorgTree();
  }
}

async function browseAgorgMaster() {
  const start = document.getElementById('agorg-master').value || '/home';
  const res = await fetch('/api/fs/pick-directory', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ start_dir: start })
  });
  const data = await res.json();
  if (data.ok && data.path) {
    document.getElementById('agorg-master').value = data.path;
    agorgScanMaster(data.path);
    await agorgDiscoverPreview();
  }
}

function clearAgorgResults() {
  agorgOut.textContent = '(results cleared)';
  agorgDiscoveryOut.textContent = '';
}

function renderAgorgRegistry(tree, activeId = '') {
  if (!agorgRegistryList) return;
  agorgRegistryList.innerHTML = '';
  const datalist = document.getElementById('agorg-datalist');
  if (datalist) datalist.innerHTML = '';

  if (!tree || tree.length === 0) {
    agorgRegistryList.innerHTML = '<div style="padding:10px; color:#4e6ba6; font-size:0.8rem;">No registered AGOrgs found.</div>';
    return;
  }

  const renderNode = (node, depth = 0) => {
    const agorg = node.agorg;
    if (!agorg) return;

    const el = document.createElement('div');
    el.className = 'agorg-reg-item' + (depth > 0 ? ' agorg-tree-node' : '');
    el.dataset.agorgId = agorg.id;
    const paddingLeft = 12 + (depth * 24);
    
    // Style for normal vs active
    let baseStyle = `display:flex; align-items:center; justify-content:space-between; padding:10px 12px 10px ${paddingLeft}px; border-bottom:1px solid #1c2635; cursor:pointer; font-size:0.85rem;`;
    if (depth > 0) {
      baseStyle += ` background:rgba(255,255,255,${Math.min(0.05, depth * 0.02)});`;
    }
    el.style = baseStyle;

    el.innerHTML = `
      <div style="display:flex; align-items:center;">
        <span class="agorg-icon" style="margin-right:12px; display:inline-flex; width:1.2rem; font-size:1.1rem;">🏢</span>
        <span class="agorg-name" style="font-weight:600;">${agorg.name}</span>
      </div>
      <span class="agorg-badge" style="font-size:0.65rem; font-weight:700; padding:2px 4px; border-radius:3px; background:#1c2635; color:#a8b9e3;">AGOrg</span>
    `;
    el.onclick = () => switchAgorgScope(agorg.id);
    agorgRegistryList.appendChild(el);

    if (datalist) {
       const opt = document.createElement('option');
       opt.value = agorg.id;
       opt.textContent = agorg.name;
       datalist.appendChild(opt);
    }

    // Render associated AGOs nested under this ORG
    (node.agos || []).forEach(ago => {
       const agoEl = document.createElement('div');
       agoEl.className = 'agorg-reg-item' + (depth > 0 ? ' agorg-tree-node' : '');
       const agoPaddingLeft = paddingLeft + 24;
       // AGOs are non-interactive listings under the parent ORG
       agoEl.style = `display:flex; align-items:center; justify-content:space-between; padding:8px 12px 8px ${agoPaddingLeft}px; border-bottom:1px solid #1c2635; cursor:default; font-size:0.82rem; background:rgba(0,0,0,0.1);`;
       agoEl.innerHTML = `
         <div style="display:flex; align-items:center;">
           <span style="margin-right:10px; display:inline-flex; width:1.1rem; opacity:0.8;">🤖</span>
           <span style="font-weight:500; color:#b8c8ef;">${ago.name}</span>
         </div>
         <span style="font-size:0.6rem; font-weight:700; padding:1px 4px; border-radius:3px; background:#1c2635; color:#6a7dff; opacity:0.8; cursor:default;">AGO</span>
       `;
       agorgRegistryList.appendChild(agoEl);
    });

    // Recurse into children
    (node.child_agorgs || []).forEach(child => renderNode(child, depth + 1));
  };

  tree.forEach(root => {
    // root is an AgorgTreeNode, so it has .agorg
    if (root.agorg && root.agorg.master_path) {
      const h = document.createElement('div');
      h.style = 'padding:8px 12px; background:#1c2635; color:#a8b9e3; font-size:0.75rem; font-weight:700; display:flex; align-items:center; gap:8px;';
      h.innerHTML = `<span>📁 ORG (Master):</span> <span style="font-family:monospace; opacity:0.8;">${root.agorg.master_path}</span>`;
      agorgRegistryList.appendChild(h);
    }
    renderNode(root);
  });
}

async function agorgList() {
  if (agorgRegistryList) {
    agorgRegistryList.innerHTML = '<div style="padding:10px; color:#4e6ba6; font-size:0.8rem;">Loading backend properties...</div>';
  }
  try {
    const treeData = await fetchJsonSafe('/api/agorg/tree');
    const snapshot = await hydrateScopeSnapshot(true);
    const activeId = snapshot.active ? snapshot.active.id : '';

    if (treeData && treeData.ok && treeData.tree) {
       renderAgorgRegistry(treeData.tree, activeId);
    } else {
       // Fallback to flat if tree fails
       renderAgorgRegistry(snapshot.items || [], activeId);
    }

    if (agorgOut) agorgOut.textContent = JSON.stringify(treeData, null, 2);

  } catch (err) {
    const msg = (err && err.message) ? err.message : String(err);
    if (agorgOut) agorgOut.textContent = JSON.stringify({ ok: false, error: msg }, null, 2);
    if (agorgRegistryList) {
      agorgRegistryList.innerHTML = `<div style="padding:10px; color:#ff6b6b; font-size:0.8rem;">Load Failed: ${msg}</div>`;
    }
  }
}

async function agorgShowActive() {
  if (agorgActiveDetails) {
    agorgActiveDetails.innerHTML = '<em>Loading active scope...</em>';
  }
  const snapshot = await hydrateScopeSnapshot(true);
  const data = { ok: true, active: snapshot.active };
  const text = JSON.stringify(data, null, 2);
  agorgOut.textContent = text;
  
  if (data.ok && data.active) {
    if (agorgActiveDetails) {
      agorgActiveDetails.innerHTML = `
        <div style="color:#fff; font-weight:700; margin-bottom:4px; font-size:1.05rem;">${data.active.name}</div>
        <div style="margin-bottom:2px;"><strong>ID:</strong> <span style="font-family:monospace; color:#6a7dff;">${data.active.id}</span></div>
        <div style="margin-bottom:2px;"><strong>Root:</strong> <span style="font-family:monospace;">${data.active.root_path}</span></div>
        <div style="margin-bottom:2px;"><strong>Master (ORG):</strong> <span style="font-family:monospace;">${data.active.master_path || 'None'}</span></div>
      `;
    }
    
    // Update selection highlight decoupled from DOM rebuilds
    const activeId = data.active.id;
    document.querySelectorAll('.agorg-reg-item').forEach(el => {
      // Ignore AGO child nodes
      if (!el.dataset.agorgId) return;
      
      if (el.dataset.agorgId === activeId) {
        el.classList.add('active-node');
        const badge = el.querySelector('.agorg-badge');
        if (badge) badge.textContent = 'ACTIVE';
      } else {
        el.classList.remove('active-node');
        const badge = el.querySelector('.agorg-badge');
        if (badge) badge.textContent = 'ORG';
      }
    });
  } else {
    if (agorgActiveDetails) {
      agorgActiveDetails.innerHTML = `<em>No active scope set</em>`;
    }
    document.querySelectorAll('.agorg-reg-item').forEach(el => {
       el.classList.remove('active-node');
       const badge = el.querySelector('.agorg-badge');
       if (badge) badge.textContent = 'ORG';
    });
  }
  refreshAgorgHeader();
}

async function agorgRefreshActive() {
  await hydrateScopeSnapshot(true);
  await agorgShowActive();
  await agorgList();
  await agorgTree();
}

async function agorgMacroImportDiscover() {
  const onboardTab = document.querySelector('.sub-tab[onclick*="agorg-onboarding-panel"]');
  if (onboardTab) {
    activateSubPanel('agorg-onboarding-panel', onboardTab);
  }
  // Small delay to ensure panel is active before browser opens
  setTimeout(() => {
    browseAgorgMaster();
  }, 50);
}

async function agorgMacroCreateNew() {
  const onboardTab = document.querySelector('.sub-tab[onclick*="agorg-onboarding-panel"]');
  if (onboardTab) {
    activateSubPanel('agorg-onboarding-panel', onboardTab);
  }
  // Expand creation options and focus the master path field
  const details = document.getElementById('agorg-creation-details');
  if (details) {
    details.open = true;
  }
  const masterInput = document.getElementById('agorg-master');
  if (masterInput) {
    masterInput.focus();
    masterInput.select();
  }
}

async function agorgOpenEditModal() {
  const snapshot = await hydrateScopeSnapshot();
  if (!snapshot || !snapshot.active) {
    agorgOut.textContent = "Error: No active scope to edit.";
    return;
  }
  const active = snapshot.active;

  setVal('agorg-edit-id', snapshot.active.id);
  setVal('agorg-edit-name', snapshot.active.name);
  setVal('agorg-edit-root', snapshot.active.root_path);
  setVal('agorg-edit-master', snapshot.active.master_path || '');
  document.getElementById('agorg-edit-modal').classList.add('active');
}

async function agorgRemoveSelected() {
  const snapshot = await hydrateScopeSnapshot();
  if (!snapshot || !snapshot.active) {
    alert("No active AGOrg selected to remove.");
    return;
  }
  const id = snapshot.active.id;
  const name = snapshot.active.name;
  if (!confirm(`Are you sure you want to REMOVE AGOrg '${name}' (${id}) from the registry?`)) return;

  try {
    const res = await fetch('/api/agorg/delete', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ id: id })
    });
    const data = await res.json();
    if (data.ok) {
       logActivity("Remove AGOrg", `Successfully removed '${name}'`);
       
       const nextSnapshot = await hydrateScopeSnapshot(true);
       if (nextSnapshot && nextSnapshot.agorgs && nextSnapshot.agorgs.length > 0) {
           // Auto-fallback to the first available AGOrg
           const nextId = nextSnapshot.agorgs[0].id;
           await switchAgorgScope(nextId);
       } else {
           await agorgRefreshActive();
       }
    } else {
       alert("Failed to remove AGOrg: " + (data.error || "Unknown error"));
    }
  } catch (err) {
    alert("Error removing AGOrg: " + err.message);
  }
}

function agorgCloseEditModal() {
  document.getElementById('agorg-edit-modal').classList.remove('active');
}

async function agorgSaveEditModal() {
  const id = document.getElementById('agorg-edit-id').textContent;
  const req = {
    id: id,
    name: document.getElementById('agorg-edit-name').value.trim(),
    root: document.getElementById('agorg-edit-root').value.trim(),
    master: document.getElementById('agorg-edit-master').value.trim()
  };

  const res = await fetchJsonSafe('/api/agorg/update', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });

  agorgOut.textContent = JSON.stringify(res, null, 2);
  if (res.ok) {
    agorgCloseEditModal();
    await agorgRefreshActive();
  }
}

async function browseAgorgEditRoot() {
  const start = document.getElementById('agorg-edit-root').value || '/home';
  const res = await fetchJsonSafe('/api/fs/pick-directory', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ start_dir: start })
  });
  if (res.ok && res.path) {
    document.getElementById('agorg-edit-root').value = res.path;
  }
}

async function browseAgorgEditMaster() {
  const start = document.getElementById('agorg-edit-master').value || '/home';
  const res = await fetchJsonSafe('/api/fs/pick-directory', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ start_dir: start })
  });
  if (res.ok && res.path) {
    document.getElementById('agorg-edit-master').value = res.path;
  }
}

async function agorgUse() {
  const el = document.getElementById('agorg-use-id');
  const req = { agorg: el ? el.value.trim() : '' };
  if (!req.agorg) return;
  const data = await fetchJsonSafe('/api/agorg/use', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const text = JSON.stringify(data, null, 2);
  agorgOut.textContent = text;
  await hydrateScopeSnapshot(true);
  refreshAgorgHeader();
  if (data.ok) {
    await refreshPolicyHookDriftChips();
    agorgList();
    agorgShowActive();
    agorgTree();
    await loadRepoAgoOptions();
    if (currentTab === 'branch') {
      await branchLoadMatrix();
    }
    if (currentTab === 'settings') {
      await settingsRefreshTargetOptions();
      await settingsReloadPolicyControls();
      await settingsLoadExceptions();
    }
    queueUiSessionSave();
  }
  p5ResetRailState(); // P5: clear orchestration rail state on scope switch
}

async function agorgLoadPreferences() {
  const data = await fetchJsonSafe('/api/agorg/preferences');
  if (!data.ok) {
    return;
  }
  const settings = data.settings || {};
  const profile = settings.profile || {};
  const prefs = settings.preferences || {};
  const profileNameEl = document.getElementById('agorg-profile-name');
  const defaultBranchEl = document.getElementById('agorg-pref-default-branch');
  const releaseBranchEl = document.getElementById('agorg-pref-release-branch');
  const autoPruneEl = document.getElementById('agorg-pref-auto-prune');
  if (profileNameEl) profileNameEl.value = profile.name || '';
  if (defaultBranchEl) defaultBranchEl.value = prefs.default_branch || '';
  if (releaseBranchEl) releaseBranchEl.value = prefs.release_branch || '';
  if (autoPruneEl) autoPruneEl.checked = !!prefs.auto_prune;
}

async function agorgSavePreferences() {
  const req = {
    merge: true,
    preferences: {
      profile: {
        name: readInputValue('agorg-profile-name').trim()
      },
      preferences: {
        default_branch: readInputValue('agorg-pref-default-branch').trim(),
        release_branch: readInputValue('agorg-pref-release-branch').trim(),
        auto_prune: !!readInputChecked('agorg-pref-auto-prune')
      }
    }
  };
  const data = await fetchJsonSafe('/api/agorg/preferences', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const text = JSON.stringify(data, null, 2);
  agorgOut.textContent = text;
  out.textContent = text;
  if (data.ok) queueUiSessionSave();
}

async function agorgLink() {
  const req = {
    parent: document.getElementById('agorg-link-parent').value.trim(),
    child: document.getElementById('agorg-link-child').value.trim()
  };
  const res = await fetch('/api/agorg/link', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  const text = JSON.stringify(data, null, 2);
  agorgOut.textContent = text;
  out.textContent = text;
}

async function agorgDiscover() {
  const discoverPrune = document.getElementById('agorg-discover-prune');
  const req = {
    root: document.getElementById('agorg-discover-root').value.trim(),
    depth: parseInt(document.getElementById('agorg-discover-depth').value || '4', 10),
    import_to: document.getElementById('agorg-discover-import-to').value.trim() || null,
    prune_missing: !!(discoverPrune && discoverPrune.checked)
  };
  const res = await fetch('/api/agorg/discover', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  const text = JSON.stringify(data, null, 2);
  agorgDiscoveryOut.textContent = text;
  out.textContent = text;
  if (data.ok && data.discovery) {
    setDiscoveryCache(data.discovery);
  }
}

function setDiscoveryCache(discovery) {
  agorgDiscoveryCache = discovery;
  const candidates = Array.isArray(discovery.candidates) ? discovery.candidates : [];
  agorgApprovedPaths = new Set(
    candidates.filter(c => c.kind === 'ago').map(c => c.path)
  );
  renderAgorgDiscoveryReview();
}

function renderAgorgDiscoveryReview() {
  if (!agorgDiscoveryReview) return;
  const candidates = Array.isArray(agorgDiscoveryCache.candidates) ? agorgDiscoveryCache.candidates : [];
  if (!candidates.length) {
    agorgDiscoveryReview.innerHTML = '<div class="tl-empty">Run Discover Preview to populate candidates.</div>';
    return;
  }
  const selectableCount = candidates.filter(c => c.kind === 'ago' || c.kind === 'folder').length;
  const approvedCount = candidates.filter(c => (c.kind === 'ago' || c.kind === 'folder') && agorgApprovedPaths.has(c.path)).length;
  const rows = candidates.map((c) => {
    const selectable = c.kind === 'ago' || c.kind === 'folder';
    const disabled = selectable ? '' : 'disabled';
    const checked = selectable && agorgApprovedPaths.has(c.path) ? 'checked' : '';
    const isDefault = agorgDefaultScopeCandidate === c.path;
    const defaultChecked = isDefault ? 'checked' : '';
    
    let icon = '📄';
    let kindTag = 'OTHER';
    let chipClass = 'neutral';
    if (c.kind === 'agorg') { icon = '🏢'; kindTag = 'AGOrg'; chipClass = 'warn'; }
    else if (c.kind === 'ago') { icon = '📦'; kindTag = 'AGO'; chipClass = 'ok'; }
    else if (c.kind === 'folder') { icon = '📁'; kindTag = 'DIR'; chipClass = 'neutral'; }

    const designationHtml = isDefault 
      ? `<span class="chip" style="background:rgba(0,245,255,0.15); color:var(--accent); border-color:rgba(0,245,255,0.5); box-shadow:0 0 10px rgba(0,245,255,0.3);" title="Default AGOrg scope" aria-label="AGOrg scope: default">AGOrg</span>`
      : `<span class="chip ${chipClass}" title="${kindTag} designation" aria-label="${kindTag} chip: ${c.kind} type">${kindTag}</span>`;

    return `<div style="display:grid;grid-template-columns:30px 30px 90px 1fr;gap:8px;align-items:center;padding:8px 4px;border-bottom:1px solid rgba(0,245,255,0.15); transition:background 0.2s; ${checked ? 'background:rgba(0,245,255,0.05);' : ''}">
      <div title="Set as AGOrg Scope">
        <input type="radio" name="agorg-default-radio" ${defaultChecked} ${disabled} onchange="agorgSetDefaultCandidate('${encodeURIComponent(c.path)}')"/>
      </div>
      <div title="Include in Import">
        <input type="checkbox" ${checked} ${disabled} onchange="agorgToggleCandidate('${encodeURIComponent(c.path)}', this.checked)" />
      </div>
      ${designationHtml}
      <div style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:'JetBrains Mono',monospace;" title="${c.path}">
        <span style="color:var(--text);font-weight:600;"><span style="margin-right:6px;">${icon}</span>${c.name}</span> 
        <span style="color:var(--dim);font-size:0.85em;margin-left:8px;">${c.path}</span>
      </div>
    </div>`;
  }).join('');
  agorgDiscoveryReview.innerHTML = `
    <div style="background:rgba(0,245,255,0.05); padding:10px; border-radius:6px; border:1px solid rgba(0,245,255,0.2); margin-bottom:12px; font-size:0.9em; line-height:1.4;">
      <div style="color:var(--accent); font-weight:bold; margin-bottom:4px;">Instructions:</div>
      <ul style="margin:0; padding-left:18px; color:var(--text-muted);">
        <li>Select a <b>radio button</b> to designate the <b>AGOrg</b> (this folder becomes the root).</li>
        <li>Select <b>checkboxes</b> for the <b>AGOs</b> (sub-projects/replicated repositories).</li>
        <li>Click the <b>IMPORT APPROVED</b> button.</li>
      </ul>
    </div>
    <div style="padding:6px 4px; border-bottom: 2px solid rgba(0,245,255,0.3); margin-bottom: 8px;">
      <div style="display:grid; grid-template-columns:30px 30px 90px 1fr; gap:8px; align-items:center; width:100%;">
        <div style="font-size:0.65rem; color:var(--accent); font-weight:bold; text-align:center;" title="Designate this folder as the AGOrg root">AGOrg</div>
        <div style="font-size:0.65rem; color:var(--accent); font-weight:bold; text-align:center;" title="Include as a child project">AGO</div>
        <div style="font-size:0.65rem; color:var(--accent); font-weight:bold; padding-left:4px;">KIND</div>
        <div style="font-size:0.65rem; color:var(--accent); font-weight:bold;">
          PROJECT PATH / IDENTITY
          ${agorgDefaultScopeCandidate ? `<span style="float:right; color:var(--accent); font-weight:bold;">AGOrg Scope Selected</span>` : `<span style="float:right; color:#ff4d4d; font-weight:bold;">⚠️ No AGOrg Scope Selected</span>`}
        </div>
      </div>
    </div>
    ${rows}
  `;
}

function agorgSetDefaultCandidate(encodedPath) {
  agorgDefaultScopeCandidate = decodeURIComponent(encodedPath);
  renderAgorgDiscoveryReview();
}

function agorgToggleCandidate(encodedPath, approved) {
  const path = decodeURIComponent(encodedPath);
  if (approved) agorgApprovedPaths.add(path);
  else agorgApprovedPaths.delete(path);
  renderAgorgDiscoveryReview();
}

function agorgSelectAllReview(approve) {
  const candidates = Array.isArray(agorgDiscoveryCache.candidates) ? agorgDiscoveryCache.candidates : [];
  if (!candidates.length) return;
  agorgApprovedPaths = new Set(
    approve ? candidates.filter(c => c.kind === 'ago' || c.kind === 'folder').map(c => c.path) : []
  );
  renderAgorgDiscoveryReview();
}

async function getActiveAgorgId() {
  const active = await fetchJsonSafe('/api/agorg/active');
  if (!active.ok || !active.active.id) return null;
  return active.active.id;
}

async function agorgDiscoverPreview() {
  const root = document.getElementById('agorg-master').value.trim();
  const depth = 4; // default hardcode
  logActivity("Discovering Candidates...", `Root=${root} Depth=${depth}`);
  const data = await fetchJsonSafe('/api/agorg/discover', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ root, depth })
  });
  logActivity("Discovery Preview", data);
  if (data.ok && data.discovery) {
    setDiscoveryCache(data.discovery);

    // agorgDefaultScopeCandidate = root; // REMOVED: Do not select it as the default AGOrg by default

    const candidates = Array.isArray(data.discovery.candidates) ? data.discovery.candidates : [];
    agorgApprovedPaths = new Set(candidates.filter(c => c.kind === 'ago' || c.kind === 'folder').map(c => c.path));
    renderAgorgDiscoveryReview();
  }
}

async function agorgCreateNewFolder() {
  const root = document.getElementById('agorg-master').value.trim();
  if (!root) {
    showInlineError("Please select or enter a master directory first.", out);
    return;
  }
  const folderName = prompt("Enter new folder name:");
  if (!folderName || !folderName.trim()) return;

  const path = root.replace(/\/+$/, '') + "/" + folderName.trim();
  logActivity("Creating Folder", path);
  const data = await fetchJsonSafe('/api/fs/create-dir', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ path })
  });

  if (data.ok) {
    logActivity("Folder Created", path);
    agorgDiscoverPreview();
  } else {
    logActivity("Folder Creation Failed", data);
    showInlineError("Failed to create folder: " + (data.error || "Unknown error"), out);
  }
}

async function agorgImportApproved() {
  try {
    const masterVal = document.getElementById('agorg-master').value.trim();
    
    if (!masterVal) {
      logActivity("Import Approved", { ok: false, error: 'Directory is required.' });
      return;
    }
    if (!agorgDefaultScopeCandidate) {
      showInlineError("You must select a directory to serve as the AGOrg for this group. Use the radio buttons in the Discovery Review panel to designate one.", out);
      logActivity("Import Refused", "No AGOrg candidate selected. User must select one via radio button.");
      return;
    }
    
    // The AGOrg name comes from the default directory name
    const agorgName = agorgDefaultScopeCandidate.split('/').filter(Boolean).pop() || 'AGOrg';
    
    // Separate AGOrg candidate from AGO candidates
    const candidates = Array.isArray(agorgDiscoveryCache.candidates) ? agorgDiscoveryCache.candidates : [];
    const agoCandidates = candidates.filter(c => 
      c.path !== agorgDefaultScopeCandidate && agorgApprovedPaths.has(c.path)
    );
    
    // 1. Create the AGOrg from the default directory
    const createReq = {
      name: agorgName,
      root: agorgDefaultScopeCandidate,
      master: masterVal,
      parent: null,
      scan_depth: 4,
      autoscan: false,
      import: false,
      prune_missing: false,
      default_scope: true
    };
    
    logActivity("Creating AGOrg", `Creating AGOrg "${agorgName}" from ${agorgDefaultScopeCandidate}...`);
    const createRes = await fetchJsonSafe('/api/agorg/create_project', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify(createReq)
    });
    
    if (!createRes.ok || !createRes.agorg) {
      logActivity("AGOrg Creation Failed", createRes);
      return;
    }
    
    const activeId = createRes.agorg.id;
    logActivity("AGOrg Created", `ID: ${activeId}\nImporting ${agoCandidates.length} AGO candidates...`);
    
    // 2. Import the checked AGO candidates (NOT the default directory — that IS the AGOrg)
    //    Tag each candidate with the AGOrg name so the backend can write proper pyproject.toml
    const taggedCandidates = agoCandidates.map(c => ({
      ...c,
      parent_hint: agorgName
    }));
    
    const req = {
      agorg: activeId,
      root: masterVal,
      depth: 4,
      candidates: taggedCandidates,
      prune_missing: false,
      default_scope_path: null,
      agorg_name: agorgName
    };
    
    const data = await fetchJsonSafe('/api/agorg/import_selected', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify(req)
    });
    
    logActivity("Import Selected Candidates", data);
    
    if (data.ok) {
      await agorgTree();
      await agorgList();
      await agorgShowActive();
    }
  } catch (err) {
    logActivity("Import Failed (JS Error)", err.message + '\n' + err.stack);
  }
}

async function agorgResetDb() {
  if (!confirm('This will DELETE all AGOrg and AGO records from the database. Are you sure?')) return;
  logActivity("Reset Database", "Resetting database...");
  const data = await fetchJsonSafe('/api/agorg/reset', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: '{}'
  });
  logActivity("Reset Database Result", data);
  if (data.ok) {
    agorgDiscoveryCache = null;
    agorgApprovedPaths = new Set();
    agorgDefaultScopeCandidate = null;
    renderAgorgDiscoveryReview();
    await agorgTree();
    await agorgList();
    await refreshAgorgHeader();
  }
}

async function agorgReconcile() {
  const activeId = await getActiveAgorgId();
  if (!activeId) {
    logActivity("Policy Report", { ok: false, error: 'No active AGOrg scope selected' });
    return;
  }
  const cls = selectedReconcileClass();
  syncReconcileClassControls(cls);
  const body = cls ? { agorg: activeId, issue_class: cls } : { agorg: activeId };
  const data = await fetchJsonSafe('/api/agorg/policy_report', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(body)
  });
  logActivity("Governance & Policy Report", data);
  if (data && data.ok) {
    agorgReconcileState.report = data.report || null;
    appendLive({ source: 'agorg_policy', action: 'report', artifact_path: data.artifact_path || '' });
    if (dashAgorgDuplicatesOut) dashAgorgDuplicatesOut.textContent = renderDuplicateResolutionText(data.report);
    setDashAgorgDuplicateStateFromReport(data.report);
    if (dashAgorgClassCountsOut) dashAgorgClassCountsOut.textContent = renderClassCountsText(data.report);
    setDashAgorgIssueStateFromReport(data.report);
    await agorgLoadPolicyReports();
    // Render governance issues if present
    const govIssues = (data.report || {}).governance_issues || [];
    const conflictTraces = (data.report || {}).conflict_traces || [];
    if (govIssues.length > 0) {
      const severityIcon = { error: '🔴', warning: '🟡', info: '🔵' };
      let govText = `── Governance Issues (${govIssues.length}) ──\n`;
      govIssues.forEach(gi => {
        govText += `${severityIcon[gi.severity] || '⚪'} [${gi.issue_type}] ${gi.policy_kind} → ${gi.ago_path}\n`;
        govText += `   ${gi.message}\n`;
        govText += `   ↳ Remediation: ${gi.remediation}\n`;
      });
      logActivity("Governance Reconcile Issues", govText);
    }
    if (conflictTraces.length > 0) {
      let traceText = `── Conflict Traces (${conflictTraces.length}) ──\n`;
      conflictTraces.forEach(ct => {
        traceText += `${ct.policy_kind} → ${ct.ago_path} (winner: ${ct.resolved_source})\n`;
        (ct.chain || []).forEach(step => {
          traceText += `   ${step.is_winner ? '★' : '·'} ${step.agorg_name} (depth ${step.depth})${step.has_override ? ' [OVERRIDE]' : ''}${step.has_fleet_policy ? ' [FLEET]' : ''}\n`;
        });
      });
      logActivity("Policy Conflict Traces", traceText);
    }
  }
}

async function agorgReconcileApply() {
  const activeId = await getActiveAgorgId();
  if (!activeId) {
    logActivity("Reconcile Apply", { ok: false, error: 'No active AGOrg scope selected' });
    return;
  }
  const cls = selectedReconcileClass();
  syncReconcileClassControls(cls);
  const tokenClass = cls || 'all';

  logActivity("Reconcile Background Dry Run", "Running dry run to fetch token...");
  const dryBody = cls ? { agorg: activeId, dry_run: true, issue_class: cls } : { agorg: activeId, dry_run: true };
  const dryRes = await fetchJsonSafe('/api/agorg/reconcile_apply', { method: 'POST', headers: {'content-type':'application/json'}, body: JSON.stringify(dryBody) });
  
  if (!dryRes.ok || !dryRes.dry_run_token) {
     logActivity("Dry Run Failed", dryRes);
     return;
  }
  const dryRunToken = dryRes.dry_run_token;
  const requestId = (typeof crypto !== 'undefined' && crypto.randomUUID) ? crypto.randomUUID() : `req-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;

  const body = {
    idempotency_key: requestId,
    agorg: activeId,
    dry_run: false,
    dry_run_token: dryRunToken,
    issue_class: cls
  };

  logActivity("Applying Reconciliation", body);
  const data = await fetchJsonSafe('/api/agorg/reconcile_apply', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(body)
  });
  logActivity("Reconciliation Result", data);
  
  if (dashAgorgDuplicatesOut && data && data.before) {
    dashAgorgDuplicatesOut.textContent = renderDuplicateResolutionText(data.before);
  }
  if (data && data.before) {
    setDashAgorgDuplicateStateFromReport(data.before);
  }
  if (dashAgorgClassCountsOut && data && data.before) {
    dashAgorgClassCountsOut.textContent = renderClassCountsText(data.before);
  }
  if (data && data.before) {
    setDashAgorgIssueStateFromReport(data.before);
  }
  appendLive({ source: 'agorg_policy', action: 'apply', ok: !!data.ok, pruned: data.pruned || 0 });
  if (data && data.ok) {
    await agorgTree();
    await agorgList();
    await agorgShowActive();
  }
}

function renderPolicyReportSelect(selectEl, rows) {
  if (!selectEl) return;
  const items = Array.isArray(rows) ? rows : [];
  selectEl.innerHTML = '';
  if (!items.length) {
    const opt = document.createElement('option');
    opt.value = '';
    opt.textContent = 'No policy artifacts yet';
    selectEl.appendChild(opt);
    return;
  }
  for (const row of items) {
    const path = String(row.path || '');
    const file = String(row.name || (path.split('/').pop() || path));
    const opt = document.createElement('option');
    opt.value = path;
    opt.textContent = file;
    selectEl.appendChild(opt);
  }
}

function renderDuplicateResolutionText(report) {
  const rows = Array.isArray(report && report.duplicate_resolutions)
    ? report.duplicate_resolutions
    : [];
  if (!rows.length) return 'No duplicate merge candidates.';
  const header = `Duplicate merge candidates: ${rows.length}`;
  const body = rows
    .map((r, i) => {
      const losers = Array.isArray(r.loser_repo_paths) ? r.loser_repo_paths.join(', ') : '';
      return `${i + 1}. [${r.kind}] key=${r.key}\n   winner=${r.winner_repo_path}\n   losers=${losers}`;
    })
    .join('\n');
  return `${header}\n${body}`;
}

function duplicateSummaryLine(entry, idx) {
  const kind = String(entry && entry.kind ? entry.kind : 'unknown');
  const key = String(entry && entry.key ? entry.key : 'unknown');
  const winner = String(entry && entry.winner_repo_path ? entry.winner_repo_path : '');
  const loserCount = Array.isArray(entry && entry.loser_repo_paths) ? entry.loser_repo_paths.length : 0;
  return `${idx + 1}. [${kind}] key=${key} :: winner=${winner} :: losers=${loserCount}`;
}

function filterDuplicatesByKind(rows, kindFilter) {
  if (kindFilter === 'all') return rows;
  return rows.filter((it) => String(it && it.kind ? it.kind : '') === kindFilter);
}

function renderDuplicateDetail(entry) {
  if (!entry) return 'No duplicate candidate selected.';
  const losers = Array.isArray(entry.loser_repo_paths) ? entry.loser_repo_paths : [];
  const detail = {
    kind: entry.kind || 'unknown',
    key: entry.key || 'unknown',
    winner_repo_path: entry.winner_repo_path || '',
    loser_repo_paths: losers,
    planned_prune_count: losers.length,
    recommended_action: losers.length
      ? 'Dry-run reconcile first, then apply to prune loser_repo_paths.'
      : 'No losers detected; no prune action required.'
  };
  return JSON.stringify(detail, null, 2);
}

function setDashAgorgDuplicateStateFromReport(report, preserveSelection = false) {
  const rows = Array.isArray(report && report.duplicate_resolutions)
    ? report.duplicate_resolutions
    : [];
  const kindFilter = dashAgorgDupKindFilter && dashAgorgDupKindFilter.value ? dashAgorgDupKindFilter.value : 'all';
  dashAgorgDuplicateFilterState = {
    rows,
    kindFilter,
    selectedIndex: preserveSelection ? dashAgorgDuplicateFilterState.selectedIndex : 0
  };
  renderDashAgorgDuplicateView();
}

function renderDashAgorgDuplicateView() {
  const filter = dashAgorgDuplicateFilterState.kindFilter || 'all';
  const rows = filterDuplicatesByKind(dashAgorgDuplicateFilterState.rows || [], filter);
  const idxMax = Math.max(rows.length - 1, 0);
  if (dashAgorgDuplicateFilterState.selectedIndex > idxMax) dashAgorgDuplicateFilterState.selectedIndex = idxMax;
  if (dashAgorgDuplicateFilterState.selectedIndex < 0) dashAgorgDuplicateFilterState.selectedIndex = 0;
  if (dashAgorgFilteredDuplicatesOut) {
    dashAgorgFilteredDuplicatesOut.textContent = rows.length
      ? rows.map((it, idx) => duplicateSummaryLine(it, idx)).join('\n')
      : 'No duplicate candidates for current filter.';
  }
  const selected = rows.length ? rows[dashAgorgDuplicateFilterState.selectedIndex] : null;
  if (dashAgorgDuplicateDetailOut) {
    dashAgorgDuplicateDetailOut.textContent = renderDuplicateDetail(selected);
  }
}

function dashAgorgApplyDuplicateFilter() {
  dashAgorgDuplicateFilterState.kindFilter = dashAgorgDupKindFilter && dashAgorgDupKindFilter.value ? dashAgorgDupKindFilter.value : 'all';
  dashAgorgDuplicateFilterState.selectedIndex = 0;
  renderDashAgorgDuplicateView();
}

function dashAgorgPrevDuplicate() {
  dashAgorgDuplicateFilterState.selectedIndex = Math.max(dashAgorgDuplicateFilterState.selectedIndex - 1, 0);
  renderDashAgorgDuplicateView();
}

function dashAgorgNextDuplicate() {
  const rows = filterDuplicatesByKind(dashAgorgDuplicateFilterState.rows || [], dashAgorgDuplicateFilterState.kindFilter || 'all');
  dashAgorgDuplicateFilterState.selectedIndex = Math.min(
    dashAgorgDuplicateFilterState.selectedIndex + 1,
    Math.max(rows.length - 1, 0)
  );
  renderDashAgorgDuplicateView();
}

function renderClassCountsText(report) {
  const counts = (report && report.class_counts) || {};
  const entries = Object.entries(counts);
  if (!entries.length) return 'No issue classes found.';
  return entries
    .sort((a, b) => String(a[0]).localeCompare(String(b[0])))
    .map(([k, v]) => `${k}: ${v}`)
    .join('\n');
}

function selectedReconcileClass() {
  const primary = agorgReconcileClass && agorgReconcileClass.value ? String(agorgReconcileClass.value).trim() : '';
  const dash = dashAgorgReconcileClass && dashAgorgReconcileClass.value ? String(dashAgorgReconcileClass.value).trim() : '';
  return primary || dash || '';
}

function syncReconcileClassControls(value) {
  if (agorgReconcileClass) agorgReconcileClass.value = value || '';
  if (dashAgorgReconcileClass) dashAgorgReconcileClass.value = value || '';
}

function issueSummaryLine(issue, idx) {
  const cls = String(issue.issue_class || 'unknown');
  const code = String(issue.code || 'unknown');
  const repo = String(issue.repo_name || issue.repo_path || 'repo');
  return `${idx + 1}. [${cls}] ${code} :: ${repo}`;
}

function filterIssuesByClass(issues, classFilter) {
  if (classFilter === 'all') return issues;
  return issues.filter((it) => String(it.issue_class || '') === classFilter);
}

function renderIssueDetail(issue) {
  if (!issue) return 'No issue selected.';
  return JSON.stringify(issue, null, 2);
}

function setDashAgorgIssueStateFromReport(report, preserveSelection = false) {
  const issues = Array.isArray(report && report.issues) ? report.issues : [];
  const classFilter = dashAgorgIssueClassFilter && dashAgorgIssueClassFilter.value ? dashAgorgIssueClassFilter.value : 'all';
  dashAgorgIssueFilterState = {
    issues,
    classFilter,
    selectedIndex: preserveSelection ? dashAgorgIssueFilterState.selectedIndex : 0
  };
  renderDashAgorgIssueView();
}

function renderDashAgorgIssueView() {
  const filter = dashAgorgIssueFilterState.classFilter || 'all';
  const rows = filterIssuesByClass(dashAgorgIssueFilterState.issues || [], filter);
  const idxMax = Math.max(rows.length - 1, 0);
  if (dashAgorgIssueFilterState.selectedIndex > idxMax) dashAgorgIssueFilterState.selectedIndex = idxMax;
  if (dashAgorgIssueFilterState.selectedIndex < 0) dashAgorgIssueFilterState.selectedIndex = 0;
  if (dashAgorgFilteredIssuesOut) {
    dashAgorgFilteredIssuesOut.textContent = rows.length
      ? rows.map((it, idx) => issueSummaryLine(it, idx)).join('\n')
      : 'No issues match current class filter.';
  }
  const selected = rows.length ? rows[dashAgorgIssueFilterState.selectedIndex] : null;
  if (dashAgorgIssueDetailOut) {
    dashAgorgIssueDetailOut.textContent = renderIssueDetail(selected);
  }
}

function dashAgorgApplyIssueClassFilter() {
  dashAgorgIssueFilterState.classFilter = dashAgorgIssueClassFilter && dashAgorgIssueClassFilter.value ? dashAgorgIssueClassFilter.value : 'all';
  dashAgorgIssueFilterState.selectedIndex = 0;
  renderDashAgorgIssueView();
}

function dashAgorgPrevIssue() {
  dashAgorgIssueFilterState.selectedIndex = Math.max(dashAgorgIssueFilterState.selectedIndex - 1, 0);
  renderDashAgorgIssueView();
}

function dashAgorgNextIssue() {
  const rows = filterIssuesByClass(dashAgorgIssueFilterState.issues || [], dashAgorgIssueFilterState.classFilter || 'all');
  dashAgorgIssueFilterState.selectedIndex = Math.min(dashAgorgIssueFilterState.selectedIndex + 1, Math.max(rows.length - 1, 0));
  renderDashAgorgIssueView();
}

async function openReportPath(path, targetEl, fallbackEl) {
  if (!path) {
    const msg = JSON.stringify({ ok: false, error: 'No report selected' }, null, 2);
    if (targetEl) targetEl.textContent = msg;
    if (fallbackEl) fallbackEl.textContent = msg;
    return;
  }
  const data = await fetchJsonSafe('/api/report/read?path=' + encodeURIComponent(path) + '&max_bytes=200000');
  const text = JSON.stringify(data, null, 2);
  if (targetEl) targetEl.textContent = text;
  if (fallbackEl) fallbackEl.textContent = text;
}

async function agorgLoadPolicyReports() {
  const data = await fetchJsonSafe('/api/agorg/policy_reports?limit=50');
  if (!data || !data.ok) {
    const text = JSON.stringify(data, null, 2);
    agorgOut.textContent = text;
    out.textContent = text;
    return;
  }
  renderPolicyReportSelect(agorgPolicyReportSelect, data.reports);
  renderPolicyReportSelect(dashAgorgReportSelect, data.reports);
}

async function agorgOpenPolicyReport() {
  const selected = agorgPolicyReportSelect && agorgPolicyReportSelect.value ? agorgPolicyReportSelect.value : '';
  await openReportPath(selected, agorgOut, out);
}

async function agorgTree() {
  const el = document.getElementById('agorg-use-id');
  const root = el ? el.value.trim() : '';
  const query = root ? ('?root=' + encodeURIComponent(root)) : '';
  const res = await fetch('/api/agorg/tree' + query);
  const data = await res.json();
  const text = JSON.stringify(data, null, 2);
  agorgDiscoveryOut.textContent = text;
  out.textContent = text;
}

function activateSubPanel(panelId, btn) {
  const parent = btn.parentElement;
  Array.from(parent.querySelectorAll('.sub-tab')).forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  const card = parent.parentElement;
  Array.from(card.querySelectorAll('.sub-panel')).forEach(p => p.classList.remove('active'));
  document.getElementById(panelId).classList.add('active');
  if (!restoringUiSession) queueUiSessionSave();
}

async function pickDirectory(startDir) {
  const res = await fetch('/api/fs/pick-directory', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ start_dir: startDir || null })
  });
  return res.json();
}

async function browseAgorgRoot() {
  const input = document.getElementById('agorg-root');
  const nameInput = document.getElementById('agorg-name');
  const data = await pickDirectory(input.value);
  if (data && data.ok && data.path) {
    input.value = data.path;
    if (!nameInput.value.trim()) {
      const cleanPath = data.path.replace(/\/$/, '');
      const parts = cleanPath.split('/');
      nameInput.value = parts[parts.length - 1] || 'Arqon';
    }
  } else {
    const text = JSON.stringify(data, null, 2);
    agorgOut.textContent = text;
    out.textContent = text;
  }
}

async function browseAgorgCreateDest() {
  const input = document.getElementById('agorg-create-dest');
  const data = await pickDirectory(input.value);
  if (data && data.ok && data.path) {
    input.value = data.path;
  }
}

async function agorgBatchCreate() {
  const masterPath = document.getElementById('agorg-master').value.trim();
  if (!masterPath) {
    showInlineError("Master Directory path is required for batch creation.", out);
    return;
  }

  // Derive destination and name from the unified master path
  const parts = masterPath.split(/[/\\]/).filter(Boolean);
  const name = parts.pop() || 'NewOrg';
  const destination = masterPath.substring(0, masterPath.lastIndexOf(name)).replace(/[/\\]$/, '') || '/';

  const req = {
    destination: destination,
    name: name,
    siblings: document.getElementById('agorg-create-siblings').value.split('\n').map(s => s.trim()).filter(s => !!s),
    use_git: !!document.getElementById('agorg-create-git').checked
  };

  logActivity("Batch Creating Collective", req);
  const res = await fetch('/api/agorg/batch-create', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  logActivity("Batch Create Result", data);

  if (data.ok) {
    agorgList();
    agorgTree();
  } else {
    showInlineError(data.error || "Batch creation failed.", out);
  }
}

async function browseDiscoverRoot() {
  const input = document.getElementById('agorg-discover-root');
  const data = await pickDirectory(input.value);
  if (data && data.ok && data.path) {
    input.value = data.path;
  } else {
    const text = JSON.stringify(data, null, 2);
    agorgOut.textContent = text;
    out.textContent = text;
  }
}

async function browseAgorgParent() {
  const input = document.getElementById('agorg-parent');
  const data = await pickDirectory(input.value);
  if (data && data.ok && data.path) {
    input.value = data.path;
  } else {
    const text = JSON.stringify(data, null, 2);
    agorgOut.textContent = text;
    out.textContent = text;
  }
}

async function browseRepoPath() {
  const pathIn = document.getElementById('repo-path');
  const nameIn = document.getElementById('repo-name');
  const data = await pickDirectory(pathIn.value);
  if (data && data.ok && data.path) {
    pathIn.value = data.path;
    if (nameIn) {
      const match = repoAgoOptionsCache.find((ago) => ago.path === data.path);
      if (match) {
        nameIn.value = match.name;
      } else {
        nameIn.value = '';
      }
    }
  }
}

function collectAgosFromTree(nodes, out = []) {
  if (!Array.isArray(nodes)) return out;
  for (const node of nodes) {
    if (Array.isArray(node.agos)) {
      for (const ago of node.agos) {
        if (!ago || !ago.name || !ago.repo_path) continue;
        out.push({ name: String(ago.name), path: String(ago.repo_path) });
      }
    }
    if (Array.isArray(node.child_agorgs)) {
      collectAgosFromTree(node.child_agorgs, out);
    }
  }
  return out;
}

function renderRepoAgoOptions(options) {
  const sel = document.getElementById('repo-name');
  if (!sel) return;
  const current = sel.value || '';
  sel.innerHTML = '<option value="">Select AGO from active AGOrg…</option>';
  for (const ago of options) {
    const opt = document.createElement('option');
    opt.value = ago.name;
    opt.textContent = `${ago.name} - ${ago.path}`;
    sel.appendChild(opt);
  }
  if (current && options.some((x) => x.name === current)) {
    sel.value = current;
  }
}

function repoSelectAgo() {
  const sel = document.getElementById('repo-name');
  const pathIn = document.getElementById('repo-path');
  if (!sel || !pathIn) return;
  const picked = (sel.value || '').trim();
  if (!picked) return;
  const match = repoAgoOptionsCache.find((ago) => ago.name === picked);
  if (match) {
    pathIn.value = match.path;
  }
}

async function loadRepoAgoOptions() {
  const sel = document.getElementById('repo-name');
  if (!sel) return;
  repoAgoOptionsCache = [];
  renderRepoAgoOptions([]);
  const res = await fetchJsonSafe('/api/agorg/repo_options');
  if (!res || !res.ok || !Array.isArray(res.items)) {
    return;
  }
  const all = res.items
    .filter((x) => x && x.name && x.path)
    .map((x) => ({ name: String(x.name), path: String(x.path) }));
  const unique = new Map();
  for (const ago of all) {
    if (!unique.has(ago.path)) unique.set(ago.path, ago);
  }
  repoAgoOptionsCache = Array.from(unique.values()).sort((a, b) => a.name.localeCompare(b.name));
  renderRepoAgoOptions(repoAgoOptionsCache);
}

async function multiRegister() {
  const outputEl = document.getElementById('multi-register-out');
  const actionsEl = document.getElementById('multi-register-actions');
  const selectedAgoName = (document.getElementById('repo-name').value || '').trim();
  const selectedAgo = selectedAgoName
    ? repoAgoOptionsCache.find((ago) => ago.name === selectedAgoName)
    : null;
  const pathValue = (document.getElementById('repo-path').value || '').trim();
  if (!pathValue) {
    showInlineError(outputEl, 'Missing path', 'Select an AGO name or browse a repo path first.');
    if (outputEl) outputEl.style.display = 'block';
    return;
  }
  if (outputEl) outputEl.style.display = 'block';
  if (actionsEl) actionsEl.style.display = 'flex';
  const res = await run('pilot.multi.register', {
    path: pathValue,
    name: (selectedAgo ? selectedAgo.name : selectedAgoName) || null,
    group: document.getElementById('repo-group').value || null,
    tags: tags(document.getElementById('repo-tags').value)
  }, { outputEl: outputEl });
  if (res && res.ok) {
    multiRefreshRegistry();
    multiLoadSelectorOptions();
  }
}

function copyMultiRegister() {
  const outputEl = document.getElementById('multi-register-out');
  if (outputEl && outputEl.textContent) {
    navigator.clipboard.writeText(outputEl.textContent).then(() => {
      msg('Copied to clipboard');
    }).catch(err => {
      console.error('Failed to copy: ', err);
      msg('Copy failed');
    });
  }
}

function clearMultiRegister() {
  const outputEl = document.getElementById('multi-register-out');
  const actionsEl = document.getElementById('multi-register-actions');
  if (outputEl) {
    outputEl.textContent = '';
    outputEl.style.display = 'none';
  }
  if (actionsEl) actionsEl.style.display = 'none';
}

function multiScopeSelector() {
  const group = (document.getElementById('multi-group').value || '').trim();
  const selectedTags = tags(document.getElementById('multi-tags').value);
  if (!group && selectedTags.length === 0) {
    const message = {
      ok: false,
      error: 'Scope required: set Group or Tags before running Multi actions.',
      hint: 'Example: group=core OR tags=apply-pilot',
      source: 'multi_ui_guard'
    };
    multiActionLast = { command: 'multi.scope', data: message };
    multiRenderActionOutput();
    return null;
  }
  return {
    group: group || null,
    tags: selectedTags
  };
}

function multiHasScope() {
  const group = (document.getElementById('multi-group').value || '').trim();
  const selectedTags = tags(document.getElementById('multi-tags').value);
  return !!group || selectedTags.length > 0;
}

function openMultiScopeModal() {
  if (!multiScopeModal) return;
  const modalGroup = document.getElementById('multi-scope-modal-group');
  const modalTags = document.getElementById('multi-scope-modal-tags');
  const groupInput = document.getElementById('multi-group');
  const tagsInput = document.getElementById('multi-tags');
  if (modalGroup && groupInput) modalGroup.value = groupInput.value || '';
  if (modalTags && tagsInput) modalTags.value = tagsInput.value || '';
  if (modalGroup) {
    modalGroup.disabled = false;
    modalGroup.readOnly = false;
  }
  if (modalTags) {
    modalTags.disabled = false;
    modalTags.readOnly = false;
  }
  multiScopeModal.classList.add('active');
  setTimeout(() => {
    if (modalGroup) {
      modalGroup.focus();
      modalGroup.select();
    }
  }, 0);
}

function closeMultiScopeModal() {
  if (!multiScopeModal) return;
  multiScopeModal.classList.remove('active');
}

function applyMultiScopeModal() {
  const modalGroup = document.getElementById('multi-scope-modal-group');
  const modalTags = document.getElementById('multi-scope-modal-tags');
  const groupInput = document.getElementById('multi-group');
  const tagsInput = document.getElementById('multi-tags');
  const groupValue = (modalGroup?.value || '').trim();
  const tagsValue = (modalTags?.value || '').trim();
  if (groupInput) groupInput.value = groupValue;
  if (tagsInput) tagsInput.value = tagsValue;
  if (!groupValue && !tagsValue) {
    showInlineError(
      multiActionOut,
      'Scope required',
      'Enter Group, Tags, or both before continuing.'
    );
    if (modalGroup) modalGroup.focus();
    return;
  }
  closeMultiScopeModal();
  multiRefreshRegistry();
  if (groupInput) groupInput.focus();
}

function multiSetOutputMode(mode) {
  multiOutputMode = mode === 'json' ? 'json' : 'html';
  if (multiOutputHtmlBtn) multiOutputHtmlBtn.classList.toggle('active', multiOutputMode === 'html');
  if (multiOutputJsonBtn) multiOutputJsonBtn.classList.toggle('active', multiOutputMode === 'json');
  multiRenderActionOutput();
}

function toggleMultiMacroLog(forceExpand = null) {
  if (!multiMacroLogOut || !multiMacroToggleBtn) return;
  if (forceExpand === true) multiMacroExpanded = true;
  else if (forceExpand === false) multiMacroExpanded = false;
  else multiMacroExpanded = !multiMacroExpanded;
  multiMacroLogOut.style.display = multiMacroExpanded ? 'block' : 'none';
  multiMacroToggleBtn.textContent = multiMacroExpanded
    ? '🔼 Collapse Macro Telemetry'
    : '🔽 Expand Macro Telemetry';
}

function appendMultiMacroTelemetry(label, data) {
  if (!multiMacroLogOut) return;
  const now = new Date().toISOString();
  const ok = !!(data && data.ok);
  const status = ok ? 'PASS' : 'FAIL';
  const formatted = formatMultiOutput(`pilot.multi.${label.toLowerCase().replace(/\s+/g, '_')}`, data);
  const summary = formatted
    .split('\n')
    .find((line) => line.startsWith('Summary: '))
    ?.replace('Summary: ', '') || 'No summary available';
  const error = data?.error || data?.inner?.error || '';
  const snap = data?.multi_snapshot || null;
  const repoCount = Array.isArray(snap?.repos) ? snap.repos.length : null;
  const statusCount = Array.isArray(snap?.statuses) ? snap.statuses.length : null;
  const orderCount = Array.isArray(snap?.order) ? snap.order.length : null;
  const stageCount = Array.isArray(snap?.dag?.stages) ? snap.dag.stages.length : null;
  const block = [
    `=== ${label} @ ${now} ===`,
    `Result: ${status}`,
    `Summary: ${summary}`,
    repoCount !== null ? `Repos: ${repoCount}` : null,
    statusCount !== null ? `Statuses: ${statusCount}` : null,
    orderCount !== null ? `Order length: ${orderCount}` : null,
    stageCount !== null ? `DAG stages: ${stageCount}` : null,
    error ? `Error: ${error}` : null,
    ''
  ].filter(Boolean).join('\n');
  const current = multiMacroLogOut.textContent && multiMacroLogOut.textContent !== 'Macro telemetry ready.'
    ? `${multiMacroLogOut.textContent}\n`
    : '';
  multiMacroLogOut.textContent = `${current}${block}`;
}

function focusResultsForA11y(el) {
  if (!el) return;
  el.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  try {
    el.focus({ preventScroll: true });
  } catch (_) {
    el.focus();
  }
}

function renderMultiDagVisual(snapshot) {
  if (!multiDagVisual || !multiDagVisualEmpty || !multiDagVisualScroll) return;
  const stages = Array.isArray(snapshot?.dag?.stages) ? snapshot.dag.stages : [];
  const rawEdges = Array.isArray(snapshot?.dag?.edges) ? snapshot.dag.edges : [];
  if (stages.length === 0) {
    multiDagVisual.innerHTML = '';
    multiDagVisualScroll.style.display = 'none';
    multiDagVisualEmpty.style.display = 'block';
    multiDagVisualEmpty.textContent = 'No DAG stages available for current selector.';
    return;
  }

  const stageGapX = 270;
  const nodeW = 190;
  const nodeH = 52;
  const nodeGapY = 74;
  const marginX = 48;
  const marginY = 38;
  const maxRows = Math.max(...stages.map((s) => (Array.isArray(s) ? s.length : 0)));
  const width = Math.max(980, marginX * 2 + stageGapX * Math.max(1, stages.length - 1) + nodeW);
  const height = Math.max(280, marginY * 2 + nodeGapY * Math.max(1, maxRows - 1) + nodeH);

  multiDagVisual.setAttribute('viewBox', `0 0 ${width} ${height}`);
  multiDagVisual.setAttribute('width', String(width));
  multiDagVisual.setAttribute('height', String(height));
  multiDagVisual.innerHTML = '';

  const NS = 'http://www.w3.org/2000/svg';
  const mk = (name, attrs = {}) => {
    const el = document.createElementNS(NS, name);
    Object.entries(attrs).forEach(([k, v]) => el.setAttribute(k, String(v)));
    return el;
  };

  const defs = mk('defs');
  const edgeGrad = mk('linearGradient', { id: 'multiEdgeGrad', x1: '0%', y1: '0%', x2: '100%', y2: '0%' });
  edgeGrad.appendChild(mk('stop', { offset: '0%', 'stop-color': '#00f5ff', 'stop-opacity': '0.8' }));
  edgeGrad.appendChild(mk('stop', { offset: '100%', 'stop-color': '#6a7dff', 'stop-opacity': '0.85' }));
  const nodeGrad = mk('linearGradient', { id: 'multiNodeGrad', x1: '0%', y1: '0%', x2: '100%', y2: '100%' });
  nodeGrad.appendChild(mk('stop', { offset: '0%', 'stop-color': '#0c2230' }));
  nodeGrad.appendChild(mk('stop', { offset: '100%', 'stop-color': '#171d36' }));
  const glow = mk('filter', { id: 'multiGlow', x: '-50%', y: '-50%', width: '200%', height: '200%' });
  glow.appendChild(mk('feGaussianBlur', { stdDeviation: '2.2', result: 'coloredBlur' }));
  const merge = mk('feMerge');
  merge.appendChild(mk('feMergeNode', { in: 'coloredBlur' }));
  merge.appendChild(mk('feMergeNode', { in: 'SourceGraphic' }));
  glow.appendChild(merge);
  defs.appendChild(edgeGrad);
  defs.appendChild(nodeGrad);
  defs.appendChild(glow);
  multiDagVisual.appendChild(defs);

  const bg = mk('rect', { x: 0, y: 0, width, height, rx: 14, fill: 'rgba(5,8,14,0.95)' });
  multiDagVisual.appendChild(bg);

  // stage lanes
  stages.forEach((_, stageIdx) => {
    const x = marginX + stageIdx * stageGapX - 14;
    const lane = mk('rect', {
      x,
      y: 12,
      width: nodeW + 28,
      height: height - 24,
      rx: 12,
      fill: stageIdx % 2 === 0 ? 'rgba(0,245,255,0.05)' : 'rgba(106,125,255,0.05)',
      stroke: 'rgba(255,255,255,0.06)',
      'stroke-width': 1
    });
    multiDagVisual.appendChild(lane);
    const title = mk('text', {
      x: x + 12,
      y: 28,
      fill: '#8be9ff',
      'font-size': 12,
      'font-family': 'JetBrains Mono, monospace',
      'letter-spacing': '0.06em'
    });
    title.textContent = `STAGE ${stageIdx + 1}`;
    multiDagVisual.appendChild(title);
  });

  const nodePos = new Map();
  stages.forEach((stage, stageIdx) => {
    const stageNodes = Array.isArray(stage) ? stage : [];
    stageNodes.forEach((name, rowIdx) => {
      const x = marginX + stageIdx * stageGapX;
      const y = marginY + rowIdx * nodeGapY;
      nodePos.set(String(name), { x, y });
    });
  });

  const edges = rawEdges
    .map((e) => ({ from: String(e.depends_on || ''), to: String(e.repo || '') }))
    .filter((e) => nodePos.has(e.from) && nodePos.has(e.to));
  edges.forEach((e) => {
    const from = nodePos.get(e.from);
    const to = nodePos.get(e.to);
    const x1 = from.x + nodeW;
    const y1 = from.y + nodeH / 2;
    const x2 = to.x;
    const y2 = to.y + nodeH / 2;
    const c1x = x1 + 56;
    const c2x = x2 - 56;
    const path = mk('path', {
      d: `M ${x1} ${y1} C ${c1x} ${y1}, ${c2x} ${y2}, ${x2} ${y2}`,
      fill: 'none',
      stroke: 'url(#multiEdgeGrad)',
      'stroke-width': 2.2,
      opacity: 0.94,
      filter: 'url(#multiGlow)'
    });
    multiDagVisual.appendChild(path);
  });

  stages.forEach((stage, stageIdx) => {
    const stageNodes = Array.isArray(stage) ? stage : [];
    stageNodes.forEach((name, rowIdx) => {
      const x = marginX + stageIdx * stageGapX;
      const y = marginY + rowIdx * nodeGapY;
      const card = mk('rect', {
        x, y, width: nodeW, height: nodeH, rx: 10,
        fill: 'url(#multiNodeGrad)',
        stroke: 'rgba(0,245,255,0.72)',
        'stroke-width': 1.3,
        filter: 'url(#multiGlow)'
      });
      multiDagVisual.appendChild(card);
      const txt = mk('text', {
        x: x + 12, y: y + 22, fill: '#d7edff',
        'font-size': 13, 'font-family': 'JetBrains Mono, monospace'
      });
      txt.textContent = String(name);
      multiDagVisual.appendChild(txt);
      const tag = mk('text', {
        x: x + 12, y: y + 40, fill: '#85b4ff',
        'font-size': 10, 'font-family': 'JetBrains Mono, monospace',
        'letter-spacing': '0.05em'
      });
      tag.textContent = `LAYER ${stageIdx + 1}`;
      multiDagVisual.appendChild(tag);
    });
  });

  multiDagVisualEmpty.style.display = 'none';
  multiDagVisualScroll.style.display = 'block';
}

function multiRenderActionOutput() {
  if (!multiActionOut) return;
  if (!multiActionLast) {
    multiActionOut.textContent = 'ready';
    return;
  }
  const { command, data } = multiActionLast;
  if (multiOutputMode === 'json') {
    multiActionOut.textContent = JSON.stringify(data, null, 2);
    renderMultiDagVisual(data?.multi_snapshot || null);
    return;
  }
  multiActionOut.textContent = formatMultiOutput(command, data);
  renderMultiDagVisual(data?.multi_snapshot || null);
}

async function fetchMultiSnapshot(scope) {
  const params = new URLSearchParams();
  const group = (scope?.group || '').trim();
  const tagList = Array.isArray(scope?.tags) ? scope.tags : [];
  if (group) params.set('group', group);
  if (tagList.length > 0) params.set('tags', tagList.join(','));
  const url = params.toString() ? `/api/multi/snapshot?${params.toString()}` : '/api/multi/snapshot';
  const snap = await fetchJsonSafe(url);
  return snap && typeof snap === 'object' ? snap : null;
}

function renderMultiSnapshot(snapshot) {
  if (!snapshot || snapshot.ok === false) return '';
  const repos = Array.isArray(snapshot.repos) ? snapshot.repos : [];
  const statuses = Array.isArray(snapshot.statuses) ? snapshot.statuses : [];
  const order = Array.isArray(snapshot.order) ? snapshot.order : [];
  const dagStages = Array.isArray(snapshot?.dag?.stages) ? snapshot.dag.stages : [];

  const lines = [];
  lines.push('');
  lines.push(`Repos (${repos.length}):`);
  if (repos.length === 0) {
    lines.push('  - none');
  } else {
    repos.forEach((r) => lines.push(`  - ${r.name} | ${r.path}`));
  }
  lines.push('');
  lines.push(`Status (${statuses.length}):`);
  if (statuses.length === 0) {
    lines.push('  - none');
  } else {
    statuses.forEach((s) => {
      const clean = s.clean === null || s.clean === undefined ? 'unknown' : String(s.clean);
      lines.push(`  - ${s.name}: exists=${s.exists} git=${s.git_repo} clean=${clean} pilot=${s.pilot_initialized} oracle=${s.oracle_ready}`);
    });
  }
  lines.push('');
  lines.push(`Dependency order (${order.length}):`);
  if (order.length === 0) {
    lines.push('  - none');
  } else {
    order.forEach((r, i) => lines.push(`  ${i + 1}. ${r.name}`));
  }
  lines.push('');
  lines.push(`DAG stages (${dagStages.length}):`);
  if (dagStages.length === 0) {
    lines.push('  - none');
  } else {
    dagStages.forEach((stage, i) => lines.push(`  Stage ${i + 1}: ${Array.isArray(stage) ? stage.join(', ') : String(stage)}`));
  }
  return lines.join('\n');
}

function formatMultiOutput(command, data) {
  const envSummary = data?.summary || '';
  const inner = data?.inner?.response?.response || null;
  const innerSummary = inner?.summary || '';
  const ok = !!(data?.ok);
  const status = ok ? 'SUCCESS' : 'FAILED';
  const lines = [];
  lines.push(`Command: ${command}`);
  lines.push(`Status: ${status}`);
  if (innerSummary) lines.push(`Summary: ${innerSummary}`);
  else if (envSummary) lines.push(`Summary: ${envSummary}`);
  if (!ok) {
    lines.push(`Error: ${data?.error || data?.inner?.error || 'Unknown error'}`);
  }
  const snapshotText = renderMultiSnapshot(data?.multi_snapshot);
  if (snapshotText) lines.push(snapshotText);
  return lines.join('\n');
}

async function runMultiCommand(command, payload, opts = {}) {
  const outputEl = opts.outputEl || multiActionOut || out;
  const data = await run(command, payload, {
    ...opts,
    outputEl
  });
  if (data && data.ok && /^pilot\.multi\.(list|status|order|dag|prs\.create)$/.test(command)) {
    try {
      const scope = {
        group: payload?.group || '',
        tags: Array.isArray(payload?.tags) ? payload.tags : []
      };
      const snapshot = await fetchMultiSnapshot(scope);
      if (snapshot) data.multi_snapshot = snapshot;
    } catch (_) {
      // Keep command result even if snapshot enrichment fails.
    }
  }
  multiActionLast = { command, data };
  multiRenderActionOutput();
  if (opts.focusResultOnComplete) {
    focusResultsForA11y(outputEl);
  }
  return data;
}

function multiList() {
  const scope = multiScopeSelector();
  if (!scope) return;
  return runMultiCommand('pilot.multi.list', scope, { outputEl: multiActionOut });
}

function multiRefreshRegistry() {
  if (!multiRegistryChip) return;
  const group = (document.getElementById('multi-group')?.value || '').trim();
  const selectedTags = tags(document.getElementById('multi-tags')?.value || '');
  let url = '/api/multi/registry_stats';
  const params = [];
  if (group) params.push(`group=${encodeURIComponent(group)}`);
  if (selectedTags.length > 0) params.push(`tags=${encodeURIComponent(selectedTags.join(','))}`);
  if (params.length) url += `?${params.join('&')}`;
  fetchJsonSafe(url).then((data) => {
    if (!data || data.ok === false) {
      multiRegistryChip.textContent = 'Registry: error';
      multiRegistryChip.className = 'chip warn';
      multiRegistryChip.title = data?.error || 'Failed to load registry stats';
      return;
    }
    const filtered = Number.isFinite(data.filtered_count) ? data.filtered_count : 0;
    const total = Number.isFinite(data.in_scope_total) ? data.in_scope_total : 0;
    multiRegistryChip.textContent = `Registry: selected ${filtered} / scope ${total}`;
    multiRegistryChip.className = filtered > 0 ? 'chip success' : 'chip warn';
    multiRegistryChip.title = `Filtered in-scope repos: ${filtered} | In-scope total: ${total} | Global total: ${data.total_registered ?? 0}`;
  }).catch((err) => {
    multiRegistryChip.textContent = 'Registry: error';
    multiRegistryChip.className = 'chip warn';
    multiRegistryChip.title = err?.message || 'Failed to load registry stats';
  });
}

async function multiSelectorStats() {
  const group = (document.getElementById('multi-group')?.value || '').trim();
  const selectedTags = tags(document.getElementById('multi-tags')?.value || '');
  let url = '/api/multi/registry_stats';
  const params = [];
  if (group) params.push(`group=${encodeURIComponent(group)}`);
  if (selectedTags.length > 0) params.push(`tags=${encodeURIComponent(selectedTags.join(','))}`);
  if (params.length) url += `?${params.join('&')}`;
  const data = await fetchJsonSafe(url);
  if (!data || data.ok === false) {
    return { ok: false, error: data?.error || 'Failed to load registry stats' };
  }
  return {
    ok: true,
    filtered: Number(data.filtered_count || 0),
    inScope: Number(data.in_scope_total || 0),
    total: Number(data.total_registered || 0)
  };
}

function multiLoadSelectorOptions() {
  fetchJsonSafe('/api/multi/selectors').then((data) => {
    if (!data || data.ok === false) return;
    if (multiGroupOptions) {
      multiGroupOptions.innerHTML = '';
      (Array.isArray(data.groups) ? data.groups : []).forEach((g) => {
        const opt = document.createElement('option');
        opt.value = g;
        multiGroupOptions.appendChild(opt);
      });
    }
    if (multiTagOptions) {
      multiTagOptions.innerHTML = '';
      (Array.isArray(data.tags) ? data.tags : []).forEach((t) => {
        const opt = document.createElement('option');
        opt.value = t;
        multiTagOptions.appendChild(opt);
      });
    }
  }).catch(() => {});
}
function multiStatus() {
  const scope = multiScopeSelector();
  if (!scope) return;
  return runMultiCommand('pilot.multi.status', scope, { outputEl: multiActionOut });
}
function multiOrder() {
  const scope = multiScopeSelector();
  if (!scope) return;
  return runMultiCommand('pilot.multi.order', scope, { outputEl: multiActionOut });
}
function multiDag() {
  const scope = multiScopeSelector();
  if (!scope) return;
  return runMultiCommand('pilot.multi.dag', {
    ...scope,
    dry_run: true
  }, {
    label: 'DAG',
    chip: multiDagChip,
    buttons: [multiDagBtn],
    runningLabel: 'DAG running...',
    outputEl: multiActionOut
  });
}
function multiPrsCreate() {
  const scope = multiScopeSelector();
  if (!scope) return;
  return runMultiCommand('pilot.multi.prs.create', {
    ...scope,
    dry_run: true,
    head_branch: 'dev',
    base_branch: 'main'
  }, { outputEl: multiActionOut });
}

function multiApplyPayload(apply) {
  const scope = multiScopeSelector();
  if (!scope) return null;
  const stageSizeRaw = parseInt(document.getElementById('multi-apply-stage-size').value || '2', 10);
  const stageSize = Number.isFinite(stageSizeRaw) && stageSizeRaw > 0 ? stageSizeRaw : 2;
  return {
    branch: document.getElementById('multi-apply-branch').value || 'feat/pilot-wave13',
    base_branch: document.getElementById('multi-apply-base').value || 'dev',
    pr_base_branch: document.getElementById('multi-apply-pr-base').value || 'main',
    ...scope,
    stage_size: stageSize,
    continue_on_failure: !!document.getElementById('multi-apply-continue').checked,
    apply: !!apply
  };
}

function multiApplyDryRun() {
  const payload = multiApplyPayload(false);
  if (!payload) return;
  return runMultiCommand('pilot.multi.apply', payload, {
    label: 'Staged Apply',
    chip: multiApplyChip,
    buttons: [multiApplyDryBtn, multiApplyExecBtn],
    runningLabel: 'Running...',
    outputEl: multiActionOut
  });
}

function multiApplyExecute() {
  const payload = multiApplyPayload(true);
  if (!payload) return;
  return runMultiCommand('pilot.multi.apply', payload, {
    label: 'Staged Apply',
    chip: multiApplyChip,
    buttons: [multiApplyDryBtn, multiApplyExecBtn],
    runningLabel: 'Running...',
    outputEl: multiActionOut
  });
}

async function multiMacroListStatusOrder() {
  if (multiMacroRunning) return;
  if (!multiHasScope()) {
    openMultiScopeModal();
    return;
  }
  multiMacroRunning = true;
  try {
    toggleMultiMacroLog(true);
    let passCount = 0;
    let failCount = 0;
    const steps = [
      { btn: multiListBtn, fn: multiList, name: 'List' },
      { btn: multiStatusBtn, fn: multiStatus, name: 'Status' },
      { btn: multiOrderBtn, fn: multiOrder, name: 'Order' }
    ];
    for (const step of steps) {
      if (step.btn) {
        step.btn.focus();
      }
      const result = await step.fn();
      appendMultiMacroTelemetry(step.name, result);
      if (result && result.ok) passCount += 1;
      else failCount += 1;
    }
    if (multiMacroLogOut) {
      multiMacroLogOut.textContent += `\nMacro Summary: ${passCount} passed, ${failCount} failed`;
      focusResultsForA11y(multiMacroLogOut);
    }
  } finally {
    multiMacroRunning = false;
  }
}

async function multiMacroDagPrPlan() {
  if (multiMacroRunning) return;
  if (!multiHasScope()) {
    openMultiScopeModal();
    return;
  }
  multiMacroRunning = true;
  try {
    toggleMultiMacroLog(true);
    let passCount = 0;
    let failCount = 0;
    const steps = [
      { btn: multiDagBtn, fn: multiDag, name: 'DAG' },
      { btn: multiPrPlanBtn, fn: multiPrsCreate, name: 'PR Plan' }
    ];
    for (const step of steps) {
      if (step.btn) step.btn.focus();
      const result = await step.fn();
      appendMultiMacroTelemetry(step.name, result);
      if (result && result.ok) passCount += 1;
      else failCount += 1;
    }
    if (multiMacroLogOut) {
      multiMacroLogOut.textContent += `\nMacro Summary: ${passCount} passed, ${failCount} failed`;
      focusResultsForA11y(multiMacroLogOut);
    }
  } finally {
    multiMacroRunning = false;
  }
}

async function multiMacroFleetFlow() {
  if (multiMacroRunning) return;
  if (!multiHasScope()) {
    openMultiScopeModal();
    return;
  }
  multiMacroRunning = true;
  try {
    const steps = [
      { btn: multiListBtn, fn: multiList, name: 'List' },
      { btn: multiStatusBtn, fn: multiStatus, name: 'Status' },
      { btn: multiOrderBtn, fn: multiOrder, name: 'Order' },
      { btn: multiDagBtn, fn: multiDag, name: 'DAG' },
      { btn: multiPrPlanBtn, fn: multiPrsCreate, name: 'PR Plan' }
    ];
    for (const step of steps) {
      if (step.btn) step.btn.focus();
      await step.fn();
    }
  } finally {
    multiMacroRunning = false;
  }
}

function dashBranchCreate() {
  run('pilot.branch.create', {
    branch: document.getElementById('dash-branch-name').value,
    base_branch: document.getElementById('dash-branch-base').value || 'main',
    group: document.getElementById('dash-branch-group').value || null,
    tags: tags(document.getElementById('dash-branch-tags').value),
    dry_run: true
  });
}

function dashRunPolicy() { depRun('policy'); }
function dashRunHookPolicy() { depRun('hook-policy'); }
function dashRunDrift() { depRun('drift'); }
function dashRunGate() { depRun('gate'); }
function dashRunRepair() { depRun('repair'); }
function dashStartBus() { depRun('bus-start'); }
function dashStopBus() { depRun('bus-stop'); }
function dashRestartBus() { depRun('bus-restart'); }
function dashBusStatus() { depRun('bus-status'); }
function dashDbStatus() { depRun('db-status'); }
function dashDbStart() { depRun('db-start'); }
function dashDbStop() { depRun('db-stop'); }
function dashDbRestart() { depRun('db-restart'); }
function dashServicesStatus() { depRun('services-status'); }
function dashServicesStart() { depRun('services-start'); }
function dashServicesStop() { depRun('services-stop'); }
function dashServicesRestart() { depRun('services-restart'); }
function dashRunPush() { depRun('push'); }

function normalizeRoutineProfile(profile) {
  const fallback = { ...ROUTINE_DEFAULT_PROFILE };
  if (!profile || typeof profile !== 'object') return fallback;
  const validSteps = new Set(['scope', 'multi', 'gates', 'push', 'ci', 'evidence']);
  const incoming = Array.isArray(profile.step_order) ? profile.step_order : [];
  const seen = new Set();
  const ordered = [];
  incoming.forEach((step) => {
    const normalized = String(step || '').trim().toLowerCase();
    if (!validSteps.has(normalized) || seen.has(normalized)) return;
    seen.add(normalized);
    ordered.push(normalized);
  });
  if (!ordered.length) ordered.push(...fallback.step_order);
  return {
    step_order: ordered,
    stop_on_fail: profile.stop_on_fail !== false,
    include_push_step: !!profile.include_push_step,
    export_evidence_step: profile.export_evidence_step !== false
  };
}

function routineProfileDiff(profile) {
  const diffs = [];
  const fallback = ROUTINE_DEFAULT_PROFILE;
  if (JSON.stringify(profile.step_order) !== JSON.stringify(fallback.step_order)) {
    diffs.push(`step_order=${profile.step_order.join('>')}`);
  }
  if (profile.stop_on_fail !== fallback.stop_on_fail) {
    diffs.push(`stop_on_fail=${profile.stop_on_fail}`);
  }
  if (profile.include_push_step !== fallback.include_push_step) {
    diffs.push(`include_push_step=${profile.include_push_step}`);
  }
  if (profile.export_evidence_step !== fallback.export_evidence_step) {
    diffs.push(`export_evidence_step=${profile.export_evidence_step}`);
  }
  return diffs;
}

async function loadRoutinePolicyProfile() {
  const res = await fetchJsonSafe('/api/settings/policy/operator_routine');
  if (!res || res.ok === false || !res.policy_json || typeof res.policy_json !== 'object') {
    return {
      ok: false,
      source: 'Built-in default',
      version: 0,
      status: 'fallback',
      profile: { ...ROUTINE_DEFAULT_PROFILE },
      policy_json: { kind: 'operator_routine', version: 1, post_commit_profile: { ...ROUTINE_DEFAULT_PROFILE } },
      diff: []
    };
  }
  const profile = normalizeRoutineProfile(res.policy_json.post_commit_profile);
  return {
    ok: true,
    source: String(res.source || 'Unknown'),
    version: Number.isFinite(res.version) ? res.version : 0,
    status: String(res.status || 'unknown'),
    profile,
    policy_json: { ...res.policy_json, post_commit_profile: profile },
    diff: routineProfileDiff(profile)
  };
}

function routineApplyPolicyProfile(loaded) {
  const profile = loaded?.profile ? loaded.profile : { ...ROUTINE_DEFAULT_PROFILE };
  const pushToggle = document.getElementById('dash-routine-allow-push');
  const evidenceToggle = document.getElementById('dash-routine-export-evidence');
  if (pushToggle) pushToggle.checked = !!profile.include_push_step;
  if (evidenceToggle) evidenceToggle.checked = !!profile.export_evidence_step;
  const sourceText = `${loaded?.source || 'Built-in default'} v${loaded?.version ?? 0} [${loaded?.status || 'fallback'}]`;
  const stepsText = profile.step_order.join(' -> ');
  const sourceLevel = loaded?.ok ? 'ok' : 'warn';
  routineSetChip(dashRoutineProfileSourceChip, `Profile: ${sourceText}`, sourceLevel);
  routineSetChip(dashRoutineProfileStepsChip, `Steps: ${stepsText}`, 'neutral');
  dashRoutineWorkspaceState.loaded = loaded;
  routineSetStageState('plan', 'Ready', loaded?.ok ? 'ok' : 'warn');
  routineUpdateModeChip();
  routineRefreshPlanPreview();
  routineRenderWorkspace();
}

function routineSetChip(chip, label, level) {
  if (!chip) return;
  chip.textContent = label;
  chip.className = `chip ${level}`;
}

function routineAnnounceStatus(message) {
  if (dashRoutineLiveStatus) dashRoutineLiveStatus.textContent = String(message || '');
}

function routineAnnounceAlert(message) {
  if (dashRoutineLiveAlert) dashRoutineLiveAlert.textContent = String(message || '');
}

function routineStageStateEl(stage) {
  return document.getElementById(`dash-routine-stage-${stage}-state`);
}

function routineStageTabEl(stage) {
  return document.getElementById(`dash-routine-stage-${stage}-tab`);
}

function routineReadBranchRemote() {
  const branch = (dashRoutineBranchInput?.value || document.getElementById('dash-push-branch')?.value || 'main').trim() || 'main';
  const remote = (dashRoutineRemoteInput?.value || document.getElementById('dash-push-remote')?.value || 'origin').trim() || 'origin';
  return { branch, remote };
}

function routineSyncPushControls() {
  const legacyBranch = document.getElementById('dash-push-branch');
  const legacyRemote = document.getElementById('dash-push-remote');
  const { branch, remote } = routineReadBranchRemote();
  if (dashRoutineBranchInput && dashRoutineBranchInput.value !== branch) dashRoutineBranchInput.value = branch;
  if (dashRoutineRemoteInput && dashRoutineRemoteInput.value !== remote) dashRoutineRemoteInput.value = remote;
  if (legacyBranch && legacyBranch.value !== branch) legacyBranch.value = branch;
  if (legacyRemote && legacyRemote.value !== remote) legacyRemote.value = remote;
}

function routineActiveProfile() {
  return dashRoutineWorkspaceState.loaded?.profile || { ...ROUTINE_DEFAULT_PROFILE };
}

function routineUpdateModeChip() {
  const allowPush = !!document.getElementById('dash-routine-allow-push')?.checked;
  const exportEvidence = !!document.getElementById('dash-routine-export-evidence')?.checked;
  const autoHeal = !!document.getElementById('dash-routine-auto-heal')?.checked;
  const autoCodex = !!document.getElementById('dash-routine-auto-codex')?.checked;
  const mode = allowPush ? 'mutating' : 'safe';
  const suffix = `${exportEvidence ? ' + evidence' : ''}${autoHeal ? ' + auto-heal' : ''}${autoCodex ? ' + auto-codex' : ''}`;
  routineSetChip(dashRoutineModeChip, `Mode: ${mode}${suffix}`, allowPush ? 'warn' : 'neutral');
}

function routineAutoHealEnabled() {
  return !!document.getElementById('dash-routine-auto-heal')?.checked;
}

function routineAutoCodexEnabled() {
  return !!document.getElementById('dash-routine-auto-codex')?.checked;
}

function routineSetLastResult(status, label = '') {
  const normalized = String(status || 'idle').toLowerCase();
  const level = normalized === 'success' ? 'ok' : (normalized === 'failed' ? 'fail' : (normalized === 'running' ? 'warn' : 'neutral'));
  const text = label || `Last Result: ${normalized}`;
  routineSetChip(dashRoutineLastResultChip, text, level);
}

function routineSummaryText() {
  const active = dashRoutineWorkspaceState.active;
  const stats = dashRoutineWorkspaceState.stats;
  if (active && stats && Number.isFinite(stats.filtered)) {
    return `${active.name || active.id || 'AGOrg'} | ${stats.filtered}/${stats.inScope || stats.filtered} repos`;
  }
  return 'Awaiting resolve stage';
}

function routineRefreshPlanPreview() {
  const profile = routineActiveProfile();
  const scopeSummary = routineSummaryText();
  const stepList = Array.isArray(profile.step_order) && profile.step_order.length
    ? profile.step_order.map((step) => ROUTINE_STAGE_LABELS[step === 'scope' ? 'resolve' : step] || step).join(' -> ')
    : 'Resolve to compute plan';
  if (dashRoutineScopeSummary) dashRoutineScopeSummary.value = scopeSummary;
  if (dashRoutinePlanSummary) dashRoutinePlanSummary.value = stepList;
}

async function routineLoadMultiSnapshot() {
  const scope = parseMultiScopeFromRoutine();
  const params = new URLSearchParams();
  if (scope.group) params.set('group', scope.group);
  if (Array.isArray(scope.tags) && scope.tags.length) params.set('tags', scope.tags.join(','));
  const suffix = params.toString() ? `?${params.toString()}` : '';
  const snapshot = await fetchJsonSafe(`/api/multi/snapshot${suffix}`);
  if (snapshot && snapshot.ok) {
    dashRoutineWorkspaceState.multiSnapshot = snapshot;
    if (dashRoutineSelectedStage === 'multi') routineRenderWorkspace();
  }
  return snapshot;
}

async function routineLoadResolveSnapshot() {
  const scope = parseMultiScopeFromRoutine();
  const { branch, remote } = routineReadBranchRemote();
  const params = new URLSearchParams();
  if (scope.group) params.set('group', scope.group);
  if (Array.isArray(scope.tags) && scope.tags.length) params.set('tags', scope.tags.join(','));
  params.set('branch', branch);
  params.set('remote', remote);
  const snapshot = await fetchJsonSafe(`/api/dashboard/routine/resolve?${params.toString()}`);
  if (snapshot && snapshot.ok) {
    dashRoutineWorkspaceState.resolveSnapshot = snapshot;
    dashRoutineWorkspaceState.active = snapshot.active_scope || null;
    dashRoutineWorkspaceState.scope = {
      group: snapshot.selector?.group || null,
      tags: Array.isArray(snapshot.selector?.tags) ? snapshot.selector.tags : []
    };
    dashRoutineWorkspaceState.stats = {
      filtered: Number(snapshot.cohort?.filtered_count || 0),
      inScope: Number(snapshot.cohort?.in_scope_total || 0)
    };
    if (snapshot.resolved_policy) {
      dashRoutineWorkspaceState.loaded = {
        ok: true,
        source: snapshot.resolved_policy.source || 'Unknown',
        version: Number.isFinite(snapshot.resolved_policy.version) ? snapshot.resolved_policy.version : 0,
        status: snapshot.resolved_policy.status || 'unknown',
        profile: normalizeRoutineProfile(snapshot.resolved_policy.profile),
        diff: routineProfileDiff(normalizeRoutineProfile(snapshot.resolved_policy.profile))
      };
    }
    routineRefreshPlanPreview();
    if (dashRoutineSelectedStage === 'resolve' || dashRoutineSelectedStage === 'plan') {
      routineRenderWorkspace();
    }
  }
  return snapshot;
}

function routineEscapeHtml(value) {
  return String(value || '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

function routineRenderDag(snapshot) {
  if (!dashRoutineDagView || !dashRoutineDagLanes || !dashRoutineDagSummaryChip) return;
  const dag = snapshot?.dag;
  const stages = Array.isArray(dag?.stages) ? dag.stages : [];
  const edges = Array.isArray(dag?.edges) ? dag.edges : [];
  const statuses = Array.isArray(snapshot?.statuses) ? snapshot.statuses : [];
  const statusByName = new Map(statuses.map((status) => [String(status.name || ''), status]));
  if (!stages.length) {
    dashRoutineDagView.classList.remove('active');
    dashRoutineDagLanes.innerHTML = '<div class="routine-dag-empty">Run Multi to materialize dependency topology.</div>';
    routineSetChip(dashRoutineDagSummaryChip, 'DAG: pending', 'neutral');
    return;
  }
  dashRoutineDagView.classList.add('active');
  routineSetChip(dashRoutineDagSummaryChip, `DAG: ${stages.length} stage${stages.length === 1 ? '' : 's'} / ${edges.length} edge${edges.length === 1 ? '' : 's'}`, 'ok');
  const laneHtml = stages.map((stageRepos, idx) => {
    const nodes = Array.isArray(stageRepos) ? stageRepos.map((repoName) => {
      const status = statusByName.get(String(repoName)) || {};
      const readiness = status.clean === false ? 'dirty' : (status.clean === true ? 'clean' : 'unknown');
      const init = status.pilot_initialized === false ? 'uninitialized' : (status.pilot_initialized === true ? 'pilot-ready' : 'init-unknown');
      return `
        <div class="routine-dag-node">
          <div class="routine-dag-node-name">${routineEscapeHtml(repoName)}</div>
          <div class="routine-dag-node-meta">${routineEscapeHtml(`${readiness} | ${init}`)}</div>
        </div>
      `;
    }).join('') : '';
    return `
      <div class="routine-dag-lane">
        <div class="routine-dag-lane-title">Execution Stage ${idx + 1}</div>
        <div class="routine-dag-nodes">${nodes || '<div class="routine-dag-empty">No repos in this stage.</div>'}</div>
      </div>
    `;
  }).join('');
  dashRoutineDagLanes.innerHTML = laneHtml;
}

function routineWorkspaceViewModel(stage) {
  const loaded = dashRoutineWorkspaceState.loaded;
  const resolveSnapshot = dashRoutineWorkspaceState.resolveSnapshot;
  const profile = routineActiveProfile();
  const { branch, remote } = routineReadBranchRemote();
  const scope = dashRoutineWorkspaceState.scope;
  const active = dashRoutineWorkspaceState.active;
  const stats = dashRoutineWorkspaceState.stats;
  const multiSnapshot = dashRoutineWorkspaceState.multiSnapshot;
  const gateDetail = dashRoutineWorkspaceState.gateDetail;
  const pushDetail = dashRoutineWorkspaceState.pushDetail;
  const ciDetail = dashRoutineWorkspaceState.ciDetail;
  const evidenceDetail = dashRoutineWorkspaceState.evidenceDetail;
  const failure = dashRoutineWorkspaceState.failure;
  const traceTail = dashRoutineTrace.slice(-3).map((entry) => `${entry.stage}: ${entry.summary}`);
  const baseNotes = traceTail.length ? traceTail.join('\n') : 'No stage activity recorded yet.';
  if (stage === 'resolve') {
    const cohort = resolveSnapshot?.cohort || {};
    const guard = resolveSnapshot?.guard_summary || {};
    return {
      title: 'Resolve',
      summary: 'Active AGOrg, cohort selector, and repo visibility are resolved here before any mutation path begins.',
      chips: [
        { label: active ? `AGOrg: ${active.name || active.id}` : 'AGOrg: unresolved', level: active ? 'ok' : 'neutral' },
        { label: stats ? `Repos: ${stats.filtered}/${stats.inScope || stats.filtered}` : 'Repos: unresolved', level: stats ? 'ok' : 'neutral' },
        { label: scope?.group ? `Group: ${scope.group}` : 'Group: any', level: scope?.group ? 'ok' : 'neutral' },
        { label: `Guard: ${guard.blocked ? 'blocked' : 'ready'}`, level: guard.blocked ? 'fail' : 'ok' }
      ],
      metrics: [
        ['Branch', branch],
        ['Remote', remote],
        ['Tags', Array.isArray(scope?.tags) && scope.tags.length ? scope.tags.join(', ') : 'none'],
        ['Clean / Dirty', `${cohort.clean_count || 0} / ${cohort.dirty_count || 0}`],
        ['Pilot / Oracle Ready', `${cohort.pilot_initialized_count || 0} / ${cohort.oracle_ready_count || 0}`]
      ],
      details: [
        active ? `Active AGOrg: ${active.name || active.id}` : 'No active AGOrg selected yet.',
        stats ? `Matched ${stats.filtered} repos within ${stats.inScope || stats.filtered} in-scope repos.` : 'Cohort statistics have not been loaded.',
        scope?.group || (scope?.tags && scope.tags.length) ? `Selector: group=${scope?.group || '-'} tags=${(scope?.tags || []).join(',') || '-'}` : 'Selector is waiting for group or tags input.',
        guard.blocked ? `Push guard is currently blocked with ${guard.violation_count || 0} violation(s).` : 'Push guard currently resolves without blocking violations.'
      ],
      artifacts: [
        'Resolve uses /api/dashboard/routine/resolve.',
        active ? 'Scope is ready for Plan and Multi.' : 'Open AGOrg tab if scope selection is missing.'
      ],
      notes: resolveSnapshot ? JSON.stringify(resolveSnapshot, null, 2) : baseNotes
    };
  }
  if (stage === 'plan') {
    const guard = resolveSnapshot?.guard_summary || {};
    return {
      title: 'Plan',
      summary: 'The plan stage resolves the operator_routine policy, mutation toggles, and exact execution order before the deck runs.',
      chips: [
        { label: `Policy: ${loaded?.source || 'Built-in default'}`, level: loaded?.ok ? 'ok' : 'warn' },
        { label: `Stop On Fail: ${profile.stop_on_fail ? 'yes' : 'no'}`, level: profile.stop_on_fail ? 'ok' : 'warn' },
        { label: `Push Enabled: ${document.getElementById('dash-routine-allow-push')?.checked ? 'yes' : 'no'}`, level: document.getElementById('dash-routine-allow-push')?.checked ? 'warn' : 'neutral' },
        { label: `Guard Violations: ${guard.violation_count || 0}`, level: guard.violation_count ? 'fail' : 'ok' }
      ],
      metrics: [
        ['Step Count', String(profile.step_order.length)],
        ['Policy Version', String(loaded?.version ?? 0)],
        ['Evidence', document.getElementById('dash-routine-export-evidence')?.checked ? 'enabled' : 'disabled']
      ],
      details: [
        `Resolved step order: ${profile.step_order.join(' -> ')}`,
        loaded?.diff?.length ? `Policy diff from built-in default: ${loaded.diff.join(', ')}` : 'Policy matches the built-in default profile.',
        `Push target: ${remote}/${branch}`,
        guard.warning_count ? `Guard warnings: ${guard.warning_count}` : 'No guard warnings in current routine context.'
      ],
      artifacts: [
        'Plan summary is derived from the resolved operator_routine policy.',
        'Quick Edit Policy can simulate changes without leaving Dashboard.'
      ],
      notes: resolveSnapshot ? JSON.stringify(resolveSnapshot.plan || resolveSnapshot, null, 2) : baseNotes
    };
  }
  if (stage === 'multi') {
    const repoCount = Array.isArray(multiSnapshot?.repos) ? multiSnapshot.repos.length : 0;
    return {
      title: 'Multi',
      summary: 'Multi preview materializes the cohort, order, DAG preview, and pull request plan for the selected repo set.',
      chips: [
        { label: `Repos: ${repoCount || 0}`, level: repoCount ? 'ok' : 'neutral' },
        { label: `DAG: ${repoCount ? 'ready' : 'pending'}`, level: repoCount ? 'ok' : 'neutral' },
        { label: 'Mode: preview', level: 'neutral' }
      ],
      metrics: [
        ['Repos', String(repoCount || 0)],
        ['Selector Group', scope?.group || '-'],
        ['Selector Tags', Array.isArray(scope?.tags) && scope.tags.length ? scope.tags.join(', ') : '-']
      ],
      details: repoCount
        ? (multiSnapshot.repos || []).slice(0, 6).map((repo) => `${repo.name || repo.repo_name || repo.path || 'repo'} | ${repo.path || 'path unavailable'}`)
        : ['Run Multi to materialize cohort topology and ordering.'],
      artifacts: [
        repoCount ? 'Preview snapshot is available in the stage notes.' : 'No DAG snapshot yet.',
        'This stage reuses the current Multi registry and DAG preview commands.'
      ],
      notes: multiSnapshot ? JSON.stringify(multiSnapshot, null, 2) : baseNotes
    };
  }
  if (stage === 'gates') {
    const steps = gateDetail?.report?.steps || [];
    return {
      title: 'Gates',
      summary: 'Governance checks remain first-class: policy, hook policy, drift, and pre-push gating stay visible as distinct verdicts.',
      chips: steps.length
        ? steps.map((step) => ({ label: `${step.step}: ${step.result?.status || 'unknown'}`, level: String(step.result?.status || '').toLowerCase() === 'pass' ? 'ok' : 'fail' }))
        : [{ label: 'Guard matrix: pending', level: 'neutral' }],
      metrics: [
        ['Checks', String(steps.length || 4)],
        ['Failures', String(steps.filter((step) => String(step.result?.status || '') !== 'Pass').length)],
        ['Mode', profile.stop_on_fail ? 'strict' : 'continue-on-fail']
      ],
      details: steps.length
        ? steps.map((step) => `${step.step}: ${step.result?.status || 'unknown'}${step.result?.failure_code ? ` (${step.result.failure_code})` : ''}`)
        : ['Run Gates to get policy and pre-push verdicts.'],
      artifacts: [
        'Blocked is treated as governed behavior, not hidden failure.',
        failure?.stage === 'Gates' ? `Latest remediation: ${failure.remediation || 'inspect Dashboard gate controls'}` : 'Remediation appears here if a guard blocks the path.'
      ],
      notes: gateDetail ? JSON.stringify(gateDetail, null, 2) : baseNotes
    };
  }
  if (stage === 'push') {
    return {
      title: 'Push',
      summary: 'Push remains explicit, policy-gated, and operator-visible with owned branch and remote controls inside the deck.',
      chips: [
        { label: `Branch: ${branch}`, level: 'neutral' },
        { label: `Remote: ${remote}`, level: 'neutral' },
        { label: `Allowed: ${document.getElementById('dash-routine-allow-push')?.checked ? 'yes' : 'no'}`, level: document.getElementById('dash-routine-allow-push')?.checked ? 'warn' : 'neutral' }
      ],
      metrics: [
        ['Branch', branch],
        ['Remote', remote],
        ['Guarded', 'yes']
      ],
      details: [
        `Push target resolves to ${remote}/${branch}.`,
        pushDetail?.error ? `Latest push error: ${pushDetail.error}` : 'No push result yet.',
        'Legacy dashboard push controls are synchronized from this deck.'
      ],
      artifacts: [
        failure?.stage === 'Push' ? `Latest remediation: ${failure.remediation || 'inspect push diagnostics'}` : 'Push result artifacts appear after execution.',
        'Push still flows through dependency orchestrate/run and operator_routine guard evaluation.'
      ],
      notes: pushDetail ? JSON.stringify(pushDetail, null, 2) : baseNotes
    };
  }
  if (stage === 'ci') {
    const summary = ciDetail?.summary || {};
    const catalog = dashRoutineWorkspaceState.ciCatalog || {};
    const workflowCount = Array.isArray(catalog.workflows) ? catalog.workflows.length : 0;
    const gapCount = Array.isArray(catalog.missing) ? catalog.missing.length : 0;
    const selectedWorkflow = (Array.isArray(catalog.workflows) ? catalog.workflows : []).find((wf) => wf.key === dashRoutineWorkspaceState.ciSelectedWorkflowKey)
      || (Array.isArray(catalog.workflows) ? catalog.workflows[0] : null);
    return {
      title: 'Continuous Integration',
      summary: 'CI observability is dynamic: whatever GitHub Actions are configured and required by current policy should surface here.',
      chips: [
        { label: `Docs: ${summary.docs_state || 'idle'}`, level: routineCiChipLevel(summary.docs_state) },
        { label: `PyPI: ${summary.pypi_state || 'idle'}`, level: routineCiChipLevel(summary.pypi_state) },
        { label: `Rust: ${summary.rust_state || 'idle'}`, level: routineCiChipLevel(summary.rust_state) },
        { label: `UI: ${summary.ui_smoke_state || 'idle'}`, level: routineCiChipLevel(summary.ui_smoke_state) },
        { label: `Workflows: ${workflowCount}`, level: workflowCount ? 'ok' : 'neutral' },
        { label: `Gaps: ${gapCount}`, level: gapCount ? 'warn' : 'ok' }
      ],
      metrics: [
        ['Workflow', selectedWorkflow?.workflow_name || summary.workflow || 'n/a'],
        ['Branch', branch],
        ['Run URL', summary.run_url ? 'available' : 'pending']
      ],
      details: [
        summary.run_url ? `Run URL: ${summary.run_url}` : 'Run URL not available yet.',
        `Overall state: ${summary.overall_state || 'idle'}`,
        `Discovered workflows: ${workflowCount}`,
        gapCount ? `Coverage gaps: ${gapCount}` : 'No required CI coverage gaps detected.'
      ],
      artifacts: [
        summary.run_url || 'No run URL yet.',
        selectedWorkflow?.workflow_path || 'No workflow selected.',
        'CI detail payload is captured in workspace notes.'
      ],
      notes: dashRoutineCiNotes?.textContent || (ciDetail ? JSON.stringify(ciDetail, null, 2) : baseNotes)
    };
  }
  if (stage === 'evidence') {
    return {
      title: 'Evidence',
      summary: 'Evidence export should leave the deck with artifact paths and integrity hints, not only a raw JSON blob.',
      chips: [
        { label: `Enabled: ${document.getElementById('dash-routine-export-evidence')?.checked ? 'yes' : 'no'}`, level: document.getElementById('dash-routine-export-evidence')?.checked ? 'warn' : 'neutral' },
        { label: evidenceDetail?.path ? 'Artifact: ready' : 'Artifact: pending', level: evidenceDetail?.path ? 'ok' : 'neutral' },
        { label: 'Integrity: explicit', level: 'neutral' }
      ],
      metrics: [
        ['Artifact Path', evidenceDetail?.path || '-'],
        ['Export', evidenceDetail?.ok ? 'success' : 'pending'],
        ['Signed', 'follow-up verify']
      ],
      details: [
        evidenceDetail?.path ? `Exported to ${evidenceDetail.path}` : 'No evidence bundle has been exported yet.',
        evidenceDetail?.error ? `Latest error: ${evidenceDetail.error}` : 'Export runs through /api/evidence/export.',
        'Use Evidence Integrity Verification below the dashboard for cryptographic verification.'
      ],
      artifacts: [
        evidenceDetail?.path || 'No artifact path yet.',
        'Transcript and workspace notes preserve the raw evidence response for audit.'
      ],
      notes: evidenceDetail ? JSON.stringify(evidenceDetail, null, 2) : baseNotes
    };
  }
  return {
    title: 'Reconcile',
    summary: 'Reconcile compresses the run into operator language: what changed, what passed, what blocked, and what should happen next.',
    chips: [
      { label: `Last Result: ${dashRoutineWorkspaceState.lastResult}`, level: dashRoutineWorkspaceState.lastResult === 'success' ? 'ok' : (dashRoutineWorkspaceState.lastResult === 'failed' ? 'fail' : 'neutral') },
      { label: `Timeline Entries: ${dashRoutineTrace.length}`, level: dashRoutineTrace.length ? 'ok' : 'neutral' },
      { label: `Last Run: ${dashRoutineWorkspaceState.lastRunAt ? 'captured' : 'none'}`, level: dashRoutineWorkspaceState.lastRunAt ? 'ok' : 'neutral' }
    ],
    metrics: [
      ['Result', dashRoutineWorkspaceState.lastResult],
      ['Last Run', dashRoutineWorkspaceState.lastRunAt || '-'],
      ['Steps Recorded', String(dashRoutineTrace.length)]
    ],
    details: [
      dashRoutineWorkspaceState.lastResult === 'success' ? 'Routine completed without terminal failure.' : 'Routine has not completed successfully yet.',
      failure ? `Last blocking stage: ${failure.stage}` : 'No blocking stage recorded.',
      failure?.remediation ? `Next action: ${failure.remediation}` : 'Next action will appear when a stage blocks.',
      dashRoutineWorkspaceState.healStatus ? 'Auto-heal transcript captured in action panel.' : 'No auto-heal run recorded yet.'
    ],
    artifacts: [
      evidenceDetail?.path || 'No evidence artifact recorded.',
      ciDetail?.summary?.run_url || 'No CI run URL recorded.'
    ],
    notes: baseNotes
  };
}

function routineRenderWorkspace() {
  const stage = dashRoutineSelectedStage;
  const vm = routineWorkspaceViewModel(stage);
  if (dashRoutineWorkspaceTitle) dashRoutineWorkspaceTitle.textContent = vm.title;
  if (dashRoutineWorkspaceSummary) dashRoutineWorkspaceSummary.textContent = vm.summary;
  if (dashRoutineStageStatusChip) routineSetChip(dashRoutineStageStatusChip, `Workspace: ${vm.title}`, 'neutral');
  if (dashRoutineWorkspaceChipRow) {
    dashRoutineWorkspaceChipRow.innerHTML = (vm.chips || []).map((chip) => `<span class="chip ${chip.level || 'neutral'}">${routineEscapeHtml(chip.label)}</span>`).join('');
  }
  if (dashRoutineWorkspaceMetrics) {
    dashRoutineWorkspaceMetrics.innerHTML = (vm.metrics || []).map(([label, value]) => `
      <div class="routine-metric">
        <span class="metric-label">${routineEscapeHtml(label)}</span>
        <span class="metric-value">${routineEscapeHtml(value)}</span>
      </div>
    `).join('');
  }
  if (dashRoutineWorkspaceDetails) {
    dashRoutineWorkspaceDetails.innerHTML = (vm.details || []).map((item) => `<li>${routineEscapeHtml(item)}</li>`).join('');
  }
  if (dashRoutineWorkspaceArtifacts) {
    dashRoutineWorkspaceArtifacts.innerHTML = (vm.artifacts || []).map((item) => `<li>${routineEscapeHtml(item)}</li>`).join('');
  }
  if (dashRoutineWorkspaceNotes) dashRoutineWorkspaceNotes.textContent = vm.notes || 'Routine workspace ready.';
  if (stage === 'multi' && dashRoutineWorkspaceState.multiSnapshot) {
    routineRenderDag(dashRoutineWorkspaceState.multiSnapshot);
  } else {
    routineRenderDag(null);
  }
}

function routineUpdateStageTabs() {
  ROUTINE_STAGE_ORDER.forEach((stage) => {
    const tab = routineStageTabEl(stage);
    const stateEl = routineStageStateEl(stage);
    if (!tab || !stateEl) return;
    tab.setAttribute('aria-selected', stage === dashRoutineSelectedStage ? 'true' : 'false');
    if (dashRoutineStagePanel) {
      if (stage === dashRoutineSelectedStage) {
        dashRoutineStagePanel.setAttribute('aria-labelledby', tab.id);
      }
    }
    const fallback = stage === 'plan' ? 'preview' : 'idle';
    const text = stateEl.textContent || fallback;
    let level = 'neutral';
    const lower = text.toLowerCase();
    if (lower.includes('pass') || lower.includes('ready') || lower.includes('captured') || lower.includes('success')) level = 'ok';
    else if (lower.includes('fail') || lower.includes('blocked')) level = 'fail';
    else if (lower.includes('running') || lower.includes('preview')) level = 'warn';
    tab.dataset.level = level;
  });
}

function routineSetStageState(stage, text, level = 'neutral') {
  const stateEl = routineStageStateEl(stage);
  if (stateEl) stateEl.textContent = text;
  const tab = routineStageTabEl(stage);
  if (tab) tab.dataset.level = level;
  routineUpdateStageTabs();
}

function routineSelectStage(stage) {
  if (!ROUTINE_STAGE_ORDER.includes(stage)) return;
  dashRoutineSelectedStage = stage;
  routineUpdateStageTabs();
  routineRenderWorkspace();
  routineAnnounceStatus(`Routine workspace moved to ${ROUTINE_STAGE_LABELS[stage] || stage}.`);
}

function routineRecordFailure(entry) {
  dashRoutineWorkspaceState.failure = entry ? {
    stage: entry.stage,
    summary: entry.summary,
    remediation: entry.remediation,
    detail: entry.detail
  } : null;
}

async function routineLoadCiCatalog() {
  const { branch } = routineReadBranchRemote();
  const params = new URLSearchParams({ branch });
  const res = await fetchJsonSafe(`/api/dashboard/ci/catalog?${params.toString()}`);
  if (isErrorResponse(res)) {
    dashRoutineWorkspaceState.ciCatalog = null;
    return res;
  }
  dashRoutineWorkspaceState.ciCatalog = res;
  const workflows = Array.isArray(res.workflows) ? res.workflows : [];
  const current = dashRoutineWorkspaceState.ciSelectedWorkflowKey;
  if (!current || !workflows.some((wf) => wf.key === current)) {
    const preferred = workflows.find((wf) => wf.required_by_policy)
      || workflows.find((wf) => (wf.key || '').toLowerCase() === 'ci.yml')
      || workflows[0];
    dashRoutineWorkspaceState.ciSelectedWorkflowKey = preferred?.key || '';
  }
  return res;
}

function routineCiWorkflowState(summary = {}, workflow = {}) {
  const normalize = (raw) => routineNormalizeCiState(raw, summary);
  const key = String(workflow.key || '').toLowerCase();
  const name = String(workflow.workflow_name || '').toLowerCase();
  if (key === 'docs.yml' || key === 'docs.yaml' || name.includes('docs')) {
    const docsBuild = normalize(summary.docs_build_state);
    const docsDeploy = normalize(summary.docs_deploy_state);
    const docsState = normalize(summary.docs_state);
    const docsJobStates = [docsBuild, docsDeploy];
    if (docsJobStates.some((s) => s === 'fail')) return 'fail';
    if (docsJobStates.some((s) => s === 'running' || s === 'queued' || s === 'in_progress')) return 'running';
    if (docsJobStates.some((s) => s === 'pass' || s === 'success' || s === 'completed')) return 'pass';
    return docsState;
  }
  if (key === 'ci.yml' || key === 'ci.yaml' || name.includes('ci')) {
    return normalize(summary.overall_state);
  }
  if (key === 'pypi.yml' || key === 'pypi.yaml' || name.includes('pypi')) {
    return normalize(summary.pypi_state);
  }
  return 'idle';
}

function routineCiJobState(summary = {}, workflow = {}, job = {}) {
  const normalize = (raw) => routineNormalizeCiState(raw, summary);
  const workflowKey = String(workflow.key || '').toLowerCase();
  const jobId = String(job.id || '').toLowerCase();
  if ((workflowKey === 'ci.yml' || workflowKey === 'ci.yaml') && jobId === 'rust') {
    return normalize(summary.rust_state);
  }
  if ((workflowKey === 'ci.yml' || workflowKey === 'ci.yaml') && jobId === 'ui-smoke') {
    return normalize(summary.ui_smoke_state);
  }
  if ((workflowKey === 'ci.yml' || workflowKey === 'ci.yaml') && jobId === 'packaging-parity') {
    return normalize(summary.packaging_parity_state);
  }
  if ((workflowKey === 'docs.yml' || workflowKey === 'docs.yaml') && jobId === 'build') {
    const docsBuild = normalize(summary.docs_build_state);
    if (docsBuild !== 'idle') return docsBuild;
    return normalize(summary.docs_state);
  }
  if ((workflowKey === 'docs.yml' || workflowKey === 'docs.yaml') && (jobId === 'deploy' || jobId === 'github-pages' || jobId === 'pages')) {
    const docsDeploy = normalize(summary.docs_deploy_state);
    if (docsDeploy !== 'idle') return docsDeploy;
    return normalize(summary.docs_state);
  }
  if (workflowKey === 'pypi.yml' || workflowKey === 'pypi.yaml') {
    if (jobId === 'build-and-publish') {
      const pypiJobState = normalize(summary.pypi_build_and_publish_state);
      if (pypiJobState !== 'idle') return pypiJobState;
      return normalize(summary.pypi_state);
    }
    return normalize(summary.pypi_state);
  }
  return 'idle';
}

function routineWorkflowRunUrl(summary = {}, workflow = {}) {
  const key = String(workflow.key || '').toLowerCase();
  if (key === 'ci.yml' || key === 'ci.yaml') return String(summary.run_url || '');
  if (key === 'docs.yml' || key === 'docs.yaml') return String(summary.docs_run_url || '');
  if (key === 'pypi.yml' || key === 'pypi.yaml') return String(summary.pypi_run_url || '');
  return '';
}

function routineSelectCiWorkflow(workflowKey) {
  dashRoutineWorkspaceState.ciSelectedWorkflowKey = String(workflowKey || '');
  routineRenderCiObservatory(
    dashRoutineWorkspaceState.ciDetail?.summary || {},
    dashRoutineWorkspaceState.ciDetail || null
  );
  if (dashRoutineSelectedStage === 'ci') routineRenderWorkspace();
}

function routineRenderCiObservatory(summary = {}, detail = null) {
  const catalog = dashRoutineWorkspaceState.ciCatalog || {};
  const workflows = Array.isArray(catalog.workflows) ? catalog.workflows : [];
  const missing = Array.isArray(catalog.missing) ? catalog.missing : [];
  const selectedKey = dashRoutineWorkspaceState.ciSelectedWorkflowKey || workflows[0]?.key || '';
  const selectedWorkflow = workflows.find((wf) => wf.key === selectedKey) || workflows[0] || null;

  if (dashRoutineCiDynamicList) {
    if (!workflows.length && !missing.length) {
      dashRoutineCiDynamicList.innerHTML = `
        <div class="routine-ci-item">
          <div class="routine-ci-item-header">
            <span class="routine-ci-item-label">No workflow data loaded</span>
            <span class="chip neutral">Idle</span>
          </div>
          <div class="routine-ci-item-copy">Refresh CI to populate live GitHub Actions state for the selected branch.</div>
        </div>
      `;
    } else {
      const workflowHtml = workflows.map((workflow) => {
        const workflowState = routineCiWorkflowState(summary, workflow);
        const selected = selectedWorkflow && workflow.key === selectedWorkflow.key;
        const jobRow = Array.isArray(workflow.jobs) && workflow.jobs.length
          ? workflow.jobs.map((job) => {
            const jobState = routineCiJobState(summary, workflow, job);
            return `<span class="chip ${routineCiChipLevel(jobState)}">${routineEscapeHtml(job.label || job.id)}: ${routineEscapeHtml(jobState)}</span>`;
          }).join(' ')
          : '<span class="chip neutral">No jobs parsed</span>';
        return `
          <button type="button" class="routine-ci-item state-${routineEscapeHtml(workflowState)}${selected ? ' selected' : ''}" onclick="routineSelectCiWorkflow('${routineEscapeHtml(workflow.key)}')" aria-pressed="${selected ? 'true' : 'false'}">
            <div class="routine-ci-item-header">
              <span class="routine-ci-item-label">${routineEscapeHtml(workflow.workflow_name || workflow.key)}</span>
              <span class="chip ${routineCiChipLevel(workflowState)}">${routineEscapeHtml(workflowState)}</span>
            </div>
            <div class="routine-ci-item-meta">${routineEscapeHtml(workflow.workflow_path || workflow.key)} | triggers: ${routineEscapeHtml((workflow.trigger_events || []).join(', ') || 'manual')}</div>
            <div class="routine-ci-item-copy">${routineEscapeHtml(workflow.policy_reason || 'Dynamic workflow discovery')}</div>
            <div class="chip-row" style="margin:8px 0 0;">${jobRow}</div>
          </button>
        `;
      }).join('');
      const missingHtml = missing.map((gap) => `
        <div class="routine-ci-item routine-ci-missing">
          <div class="routine-ci-item-header">
            <span class="routine-ci-item-label">${routineEscapeHtml(gap.label || gap.id)}</span>
            <span class="chip ${gap.severity === 'high' ? 'fail' : 'warn'}">${routineEscapeHtml(gap.severity || 'warn')}</span>
          </div>
          <div class="routine-ci-item-meta">${routineEscapeHtml(gap.kind || 'gap')}${gap.workflow_key ? ` | ${routineEscapeHtml(gap.workflow_key)}` : ''}</div>
          <div class="routine-ci-item-copy">${routineEscapeHtml(gap.remediation || 'Restore the missing CI contract element.')}</div>
        </div>
      `).join('');
      dashRoutineCiDynamicList.innerHTML = workflowHtml + missingHtml;
    }
  }

  if (dashRoutineCiPolicySummary) {
    const basis = catalog.policy_basis || {};
    const summaryBits = [
      `Policy: ${basis.source || 'unknown'} v${basis.version ?? '-'}`,
      `CI Stage: ${basis.ci_stage_enabled ? 'enabled' : 'disabled'}`,
      `Required workflows: ${catalog.summary?.required_workflow_count ?? 0}`,
      `Coverage gaps: ${catalog.summary?.gap_count ?? 0}`,
      `Warnings: ${catalog.summary?.warning_count ?? 0}`
    ];
    dashRoutineCiPolicySummary.textContent = summaryBits.join(' | ');
  }

  if (dashRoutineCiNotes) {
    if (selectedWorkflow) {
      const workflowState = routineCiWorkflowState(summary, selectedWorkflow);
      const jobLines = (selectedWorkflow.jobs || []).map((job) => {
        const jobState = routineCiJobState(summary, selectedWorkflow, job);
        return `- ${job.label || job.id}: ${jobState}${job.required_by_policy ? ' [required]' : ''}`;
      });
      const lines = [
        `Workflow: ${selectedWorkflow.workflow_name || selectedWorkflow.key}`,
        `File: ${selectedWorkflow.workflow_path || selectedWorkflow.key}`,
        `Status: ${workflowState}`,
        `Triggers: ${(selectedWorkflow.trigger_events || []).join(', ') || 'manual'}`,
        `Required by policy: ${selectedWorkflow.required_by_policy ? 'yes' : 'no'}`,
        `Reason: ${selectedWorkflow.policy_reason || 'n/a'}`,
        routineWorkflowRunUrl(summary, selectedWorkflow) ? `Run URL: ${routineWorkflowRunUrl(summary, selectedWorkflow)}` : '',
        jobLines.length ? 'Jobs:' : '',
        ...jobLines
      ].filter(Boolean);
      if (missing.length) {
        lines.push('', 'Coverage Gaps:');
        missing.forEach((gap) => {
          lines.push(`- ${gap.label || gap.id}: ${gap.remediation || 'restore the missing contract element'}`);
        });
      }
      const warnings = Array.isArray(catalog.warnings) ? catalog.warnings : [];
      if (warnings.length) {
        lines.push('', 'Catalog Warnings:');
        warnings.forEach((warn) => lines.push(`- ${warn}`));
      }
      if (detail && selectedWorkflow.key && String(selectedWorkflow.key).toLowerCase() === 'ci.yml') {
        lines.push('', `Latest CI detail captured for branch ${summary.branch || routineReadBranchRemote().branch}.`);
      }
      dashRoutineCiNotes.textContent = lines.join('\n');
    } else if (detail) {
      dashRoutineCiNotes.textContent = JSON.stringify(detail, null, 2);
    } else if (summary && Object.keys(summary).length) {
      dashRoutineCiNotes.textContent = JSON.stringify(summary, null, 2);
    } else {
      dashRoutineCiNotes.textContent = 'CI observatory ready.';
    }
  }
}

function routinePolicyModalSetStatus(value) {
  if (dashRoutinePolicyModalStatus) {
    dashRoutinePolicyModalStatus.textContent = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
  }
}

function policyGetDisplayName(policyJson, fallback = '') {
  if (!policyJson || typeof policyJson !== 'object' || Array.isArray(policyJson)) return String(fallback || '');
  const direct = String(policyJson.display_name || '').trim();
  if (direct) return direct;
  return String(fallback || '');
}

function policyApplyDisplayName(policyJson, name) {
  if (!policyJson || typeof policyJson !== 'object' || Array.isArray(policyJson)) return policyJson;
  const normalized = String(name || '').trim();
  if (normalized) {
    policyJson.display_name = normalized;
  } else if (Object.prototype.hasOwnProperty.call(policyJson, 'display_name')) {
    delete policyJson.display_name;
  }
  return policyJson;
}

function policyNormalizeForFingerprint(value) {
  if (Array.isArray(value)) return value.map((item) => policyNormalizeForFingerprint(item));
  if (value && typeof value === 'object') {
    const out = {};
    Object.keys(value).sort().forEach((key) => {
      out[key] = policyNormalizeForFingerprint(value[key]);
    });
    return out;
  }
  return value;
}

function policyFingerprint(value) {
  try {
    return JSON.stringify(policyNormalizeForFingerprint(value ?? null));
  } catch (_) {
    return '';
  }
}

function policySourceClass(source, isOverride = false) {
  if (isOverride) return 'ago';
  const normalized = String(source || '').toLowerCase();
  if (normalized.includes('default')) return 'default';
  return 'agorg';
}

function policyDiffLines(base, next, path = '') {
  const lines = [];
  const currentPath = path || '(root)';
  if (Array.isArray(base) || Array.isArray(next)) {
    const left = Array.isArray(base) ? base : [];
    const right = Array.isArray(next) ? next : [];
    if (JSON.stringify(left) !== JSON.stringify(right)) {
      lines.push(`~ ${currentPath}: [${left.length}] -> [${right.length}]`);
    }
    return lines;
  }
  const baseObj = (base && typeof base === 'object') ? base : {};
  const nextObj = (next && typeof next === 'object') ? next : {};
  const keys = Array.from(new Set([...Object.keys(baseObj), ...Object.keys(nextObj)])).sort();
  keys.forEach((key) => {
    const childPath = path ? `${path}.${key}` : key;
    const hasBase = Object.prototype.hasOwnProperty.call(baseObj, key);
    const hasNext = Object.prototype.hasOwnProperty.call(nextObj, key);
    if (!hasBase && hasNext) {
      lines.push(`+ ${childPath}`);
      return;
    }
    if (hasBase && !hasNext) {
      lines.push(`- ${childPath}`);
      return;
    }
    const left = baseObj[key];
    const right = nextObj[key];
    const bothObjects = left && right
      && typeof left === 'object'
      && typeof right === 'object'
      && !Array.isArray(left)
      && !Array.isArray(right);
    if (bothObjects) {
      lines.push(...policyDiffLines(left, right, childPath));
      return;
    }
    if (JSON.stringify(left) !== JSON.stringify(right)) {
      lines.push(`~ ${childPath}`);
    }
  });
  return lines;
}

function policyPathRead(root, path) {
  let node = root;
  for (const part of path) {
    if (node == null) return undefined;
    node = node[part];
  }
  return node;
}

function policyPathWrite(root, path, value) {
  if (!path.length) return;
  let node = root;
  for (let i = 0; i < path.length - 1; i += 1) {
    const key = path[i];
    const nextKey = path[i + 1];
    if (typeof node[key] !== 'object' || node[key] === null) {
      node[key] = Number.isInteger(nextKey) ? [] : {};
    }
    node = node[key];
  }
  node[path[path.length - 1]] = value;
}

function policyParsePath(pathStr) {
  const out = [];
  const raw = String(pathStr || '').trim();
  if (!raw) return out;
  const parts = raw.split('.');
  for (const part of parts) {
    if (!part) continue;
    const m = part.match(/^(.+)\[(\d+)\]$/);
    if (m) {
      out.push(m[1]);
      out.push(Number.parseInt(m[2], 10));
    } else {
      out.push(part);
    }
  }
  return out;
}

function policyFormInferArrayKind(arr) {
  if (!Array.isArray(arr)) return 'string';
  for (const v of arr) {
    if (typeof v === 'number') return 'number';
    if (typeof v === 'boolean') return 'boolean';
    if (typeof v === 'string') return 'string';
  }
  return 'string';
}

function policyFormEncodeArray(arr) {
  if (!Array.isArray(arr) || !arr.length) return '';
  return arr.map((v) => String(v)).join(', ');
}

function policyFormDecodeArray(text, kind) {
  const parts = String(text || '')
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
  if (kind === 'number') {
    return parts.map((s) => Number.parseFloat(s)).filter((n) => Number.isFinite(n));
  }
  if (kind === 'boolean') {
    return parts
      .map((s) => s.toLowerCase())
      .filter((s) => s === 'true' || s === 'false')
      .map((s) => s === 'true');
  }
  return parts;
}

function policyEnumOptionsFor(path, label) {
  const p = String(path || '').toLowerCase();
  const l = String(label || '').toLowerCase();
  if (p.endsWith('.level') || l === 'level') return ['off', 'info', 'warn', 'block', 'auto-fix'];
  if (p.endsWith('.kind') || l === 'kind') {
    return ['operator_routine', 'branch', 'dependency', 'release', 'security', 'quality', 'runtime'];
  }
  if (p.endsWith('.confirmation_type') || p.endsWith('.prune_confirmation_type')) {
    return ['none', 'standard', 'typed_phrase', 'double_confirm'];
  }
  if (p.endsWith('.default_scope')) return ['local', 'dryrun', 'full'];
  return null;
}

function policyRenderFormNode(value, path, label, depth = 0) {
  const pathAttr = routineEscapeHtml(path);
  const rawPath = String(path || '');
  const safeLabel = routineEscapeHtml(label || '(root)');
  if (Array.isArray(value)) {
    const simple = value.every((v) => ['string', 'number', 'boolean'].includes(typeof v));
    if (simple) {
      const kind = policyFormInferArrayKind(value);
      return `
        <div class="routine-control" style="margin-left:${depth * 10}px;">
          <label>${safeLabel}</label>
          <input type="text" data-policy-form-path="${pathAttr}" data-policy-form-kind="array" data-policy-array-kind="${kind}" value="${routineEscapeHtml(policyFormEncodeArray(value))}" />
          <div class="helper">Comma-separated ${kind} values</div>
        </div>
      `;
    }
    return `
      <div class="routine-control" style="margin-left:${depth * 10}px;">
        <label>${safeLabel}</label>
        <textarea data-policy-form-path="${pathAttr}" data-policy-form-kind="json" style="min-height:90px;">${routineEscapeHtml(JSON.stringify(value, null, 2))}</textarea>
      </div>
    `;
  }
  if (value && typeof value === 'object') {
    const rows = Object.keys(value).map((key) => {
      const childPath = path ? `${path}.${key}` : key;
      return policyRenderFormNode(value[key], childPath, key, depth + 1);
    }).join('');
    return `
      <div class="routine-detail-box" style="margin-left:${depth * 10}px;">
        <h5>${safeLabel}</h5>
        ${rows || '<div class="helper">Empty object</div>'}
      </div>
    `;
  }
  if (typeof value === 'boolean') {
    return `
      <div class="routine-control" style="margin-left:${depth * 10}px;display:flex;align-items:center;justify-content:flex-start;gap:10px;">
        <label style="display:flex;align-items:center;gap:8px;margin:0;">
          <input style="width:auto;margin:0;" type="checkbox" data-policy-form-path="${pathAttr}" data-policy-form-kind="boolean" ${value ? 'checked' : ''} />
          <span style="font-size:0.72rem;letter-spacing:0.14em;color:var(--dim);text-transform:uppercase;">${safeLabel}</span>
          <span class="helper" style="margin:0;">${value ? 'TRUE' : 'FALSE'}</span>
        </label>
      </div>
    `;
  }
  if (typeof value === 'number') {
    return `
      <div class="routine-control" style="margin-left:${depth * 10}px;">
        <label>${safeLabel}</label>
        <input type="number" step="any" data-policy-form-path="${pathAttr}" data-policy-form-kind="number" value="${value}" />
      </div>
    `;
  }
  if (typeof value === 'string') {
    const enumOptions = policyEnumOptionsFor(rawPath, label);
    if (Array.isArray(enumOptions) && enumOptions.length) {
      const selected = String(value || '').toLowerCase();
      const options = enumOptions.map((opt) => `<option value="${opt}"${selected === opt ? ' selected' : ''}>${opt}</option>`).join('');
      return `
      <div class="routine-control" style="margin-left:${depth * 10}px;">
        <label>${safeLabel}</label>
        <select data-policy-form-path="${pathAttr}" data-policy-form-kind="string">
          ${options}
        </select>
      </div>
    `;
    }
  }
  return `
    <div class="routine-control" style="margin-left:${depth * 10}px;">
      <label>${safeLabel}</label>
      <input type="text" data-policy-form-path="${pathAttr}" data-policy-form-kind="string" value="${routineEscapeHtml(String(value ?? ''))}" />
    </div>
  `;
}

function policyRenderFormInto(containerEl, policyJson) {
  if (!containerEl) return;
  if (!policyJson || typeof policyJson !== 'object' || Array.isArray(policyJson)) {
    containerEl.innerHTML = '<div class="helper">Valid policy JSON object required to render form.</div>';
    return;
  }
  const html = Object.keys(policyJson).map((key) => policyRenderFormNode(policyJson[key], key, key, 0)).join('');
  containerEl.innerHTML = html || '<div class="helper">Policy object is empty.</div>';
}

function policySyncFormChangeToEditor(event, editorEl, nameEl, statusSetter) {
  const target = event?.target;
  if (!target || !target.dataset || !target.dataset.policyFormPath || !editorEl) return;
  let policyJson;
  try {
    policyJson = JSON.parse(editorEl.value || '{}');
  } catch (err) {
    if (statusSetter) statusSetter(`Invalid JSON in editor: ${err.message}`);
    return;
  }
  const path = policyParsePath(target.dataset.policyFormPath || '');
  const kind = String(target.dataset.policyFormKind || 'string');
  let nextValue = '';
  if (kind === 'boolean') {
    nextValue = !!target.checked;
  } else if (kind === 'number') {
    const n = Number.parseFloat(String(target.value || ''));
    nextValue = Number.isFinite(n) ? n : 0;
  } else if (kind === 'array') {
    nextValue = policyFormDecodeArray(target.value, target.dataset.policyArrayKind || 'string');
  } else if (kind === 'json') {
    try {
      nextValue = JSON.parse(String(target.value || '[]'));
    } catch (err) {
      if (statusSetter) statusSetter(`Invalid JSON value for ${target.dataset.policyFormPath}: ${err.message}`);
      return;
    }
  } else {
    nextValue = String(target.value || '');
  }
  policyPathWrite(policyJson, path, nextValue);
  policyApplyDisplayName(policyJson, nameEl ? nameEl.value : '');
  editorEl.value = JSON.stringify(policyJson, null, 2);
}

function routinePolicyDraftValidate(policyJson) {
  const errors = [];
  const warnings = [];
  if (!policyJson || typeof policyJson !== 'object' || Array.isArray(policyJson)) {
    errors.push('Policy draft must be a JSON object.');
    return { ok: false, errors, warnings, normalizedPolicy: null, profile: null };
  }
  const rawProfile = policyJson.post_commit_profile;
  if (!rawProfile || typeof rawProfile !== 'object' || Array.isArray(rawProfile)) {
    errors.push('Draft must include object key: post_commit_profile.');
    return { ok: false, errors, warnings, normalizedPolicy: null, profile: null };
  }
  const profile = normalizeRoutineProfile(rawProfile);
  if (!Array.isArray(rawProfile.step_order) || rawProfile.step_order.length === 0) {
    warnings.push('step_order was empty or invalid; normalized to default sequence.');
  }
  if (!profile.step_order.includes('push') && profile.include_push_step) {
    warnings.push('include_push_step=true while push is absent from step_order.');
  }
  if (profile.step_order.includes('push') && !profile.include_push_step) {
    warnings.push('push is present in step_order but include_push_step=false.');
  }
  const normalizedPolicy = {
    ...policyJson,
    post_commit_profile: profile
  };
  return { ok: true, errors, warnings, normalizedPolicy, profile };
}

function routinePolicyDraftStatusSummary(validation, loaded) {
  if (!validation || !validation.ok) {
    return `Policy draft invalid:\n${(validation?.errors || ['unknown error']).map((e) => `- ${e}`).join('\n')}`;
  }
  const profile = validation.profile;
  const baseline = loaded?.profile ? normalizeRoutineProfile(loaded.profile) : ROUTINE_DEFAULT_PROFILE;
  const diff = routineProfileDiff(profile);
  const changed = JSON.stringify(profile) !== JSON.stringify(baseline);
  const lines = [
    `Draft profile step_order: ${profile.step_order.join(' -> ')}`,
    `Draft toggles: stop_on_fail=${profile.stop_on_fail}, include_push_step=${profile.include_push_step}, export_evidence_step=${profile.export_evidence_step}`,
    changed ? `Draft diff vs loaded profile: ${diff.join(', ') || 'none'}` : 'Draft matches loaded profile.'
  ];
  if (validation.warnings.length) {
    lines.push('Warnings:');
    validation.warnings.forEach((warn) => lines.push(`- ${warn}`));
  }
  return lines.join('\n');
}

function policyChecklistHtml(items) {
  const rows = (items || []).map((item) => {
    const ok = !!item.ok;
    return `<li style="color:${ok ? '#10B981' : '#F87171'};">${ok ? 'PASS' : 'FAIL'}: ${routineEscapeHtml(item.label || '')}</li>`;
  }).join('');
  return `<ul style="margin:0; padding-left:18px;">${rows || '<li style="color:var(--muted);">No checks.</li>'}</ul>`;
}

function policyDiffHtml(lines, emptyLabel) {
  if (!Array.isArray(lines) || !lines.length) {
    return `<div class="helper">${routineEscapeHtml(emptyLabel || 'No differences detected.')}</div>`;
  }
  const capped = lines.slice(0, 60);
  const more = lines.length > capped.length ? `<div class="helper" style="margin-top:6px;">+${lines.length - capped.length} more</div>` : '';
  return `<pre style="margin:0; max-height:160px; overflow:auto; font-size:0.72rem;">${routineEscapeHtml(capped.join('\n'))}</pre>${more}`;
}

function routinePolicyModalRefreshInsights(policyJson = null, loaded = null) {
  const sourceEl = document.getElementById('dash-routine-policy-source');
  const diffEl = document.getElementById('dash-routine-policy-diff');
  const checklistEl = document.getElementById('dash-routine-policy-checklist');
  const parsed = policyJson || (() => {
    try {
      return JSON.parse(document.getElementById('dash-routine-policy-editor')?.value || '{}');
    } catch (_) {
      return null;
    }
  })();
  const activeLoaded = loaded || dashRoutineWorkspaceState.loaded || null;
  if (sourceEl) {
    const source = activeLoaded?.source || 'Unknown';
    const status = activeLoaded?.status || 'unknown';
    const version = activeLoaded?.version ?? 0;
    const scope = dashRoutineWorkspaceState.scope?.id || dashRoutineWorkspaceState.active?.id || 'unknown';
    const sourceClass = policySourceClass(source, source.toLowerCase().includes('override'));
    sourceEl.innerHTML = `
      <div class="inheritance-trace">
        Source: <span class="source-pill ${sourceClass}">${routineEscapeHtml(source)}</span>
        <div style="font-size:0.7rem; margin-top:4px;">Version: ${routineEscapeHtml(String(version))} | Status: ${routineEscapeHtml(String(status))}</div>
        <div style="font-size:0.7rem; margin-top:4px;">Scope: ${routineEscapeHtml(String(scope))} | Kind: operator_routine</div>
      </div>
    `;
  }
  const validation = parsed ? routinePolicyDraftValidate(parsed) : { ok: false, errors: ['Invalid JSON draft.'] };
  const normalized = validation.ok ? validation.normalizedPolicy : null;
  const baseline = activeLoaded?.policy_json || {};
  const lines = normalized ? policyDiffLines(baseline, normalized) : [];
  if (diffEl) diffEl.innerHTML = policyDiffHtml(lines, 'Draft matches loaded policy.');
  if (checklistEl) {
    const draftFingerprint = normalized ? policyFingerprint(normalized) : '';
    const checklist = [
      { label: 'Draft JSON is valid for operator_routine', ok: !!validation.ok },
      { label: 'Simulation evidence is present', ok: !!dashRoutinePolicySimulationId },
      { label: 'Draft unchanged since last simulation', ok: !!dashRoutinePolicySimulationId && !!dashRoutinePolicySimulationFingerprint && draftFingerprint === dashRoutinePolicySimulationFingerprint },
      { label: 'Draft differs from loaded policy', ok: lines.length > 0 }
    ];
    checklistEl.innerHTML = policyChecklistHtml(checklist);
  }
}

function settingsPolicyRefreshInsights(policyJson = null) {
  const contextEl = document.getElementById('settings-policy-context');
  const diffEl = document.getElementById('settings-policy-diff');
  const checklistEl = document.getElementById('settings-policy-checklist');
  const kind = document.getElementById('settings-policy-kind')?.value || 'branch';
  const target = settingsTargetValue();
  const parsed = policyJson || (() => {
    try {
      return JSON.parse(settingsPolicyEditorEl()?.value || '{}');
    } catch (_) {
      return null;
    }
  })();
  const meta = settingsLoadedPolicyMeta || {};
  if (contextEl) {
    const source = meta.source || 'Unknown';
    const version = meta.version ?? '?';
    const status = meta.status || 'unknown';
    const sourceClass = policySourceClass(source, !!meta.is_override);
    contextEl.innerHTML = `
      <div class="inheritance-trace">
        Source: <span class="source-pill ${sourceClass}">${routineEscapeHtml(source)}</span>
        ${meta.is_override ? '<span class="override-tag">AGO Override</span>' : ''}
        <div style="font-size:0.7rem; margin-top:4px;">Version: ${routineEscapeHtml(String(version))} | Status: ${routineEscapeHtml(String(status))}</div>
        <div style="font-size:0.7rem; margin-top:4px;">Kind: ${routineEscapeHtml(kind)} | Target: ${routineEscapeHtml(target || 'AGOrg')}</div>
      </div>
    `;
  }
  const baseline = settingsLoadedPolicyJson || {};
  const lines = parsed ? policyDiffLines(baseline, parsed) : [];
  if (diffEl) diffEl.innerHTML = policyDiffHtml(lines, 'Draft matches loaded policy.');
  if (checklistEl) {
    const fp = parsed ? policyFingerprint(parsed) : '';
    const checklist = [
      { label: 'Draft JSON parses successfully', ok: !!parsed },
      { label: 'Simulation evidence is present', ok: !!settingsActiveSimulationId },
      { label: 'Draft unchanged since last simulation', ok: !!settingsActiveSimulationId && !!settingsLastSimulatedFingerprint && fp === settingsLastSimulatedFingerprint },
      { label: 'Draft differs from loaded policy', ok: lines.length > 0 }
    ];
    checklistEl.innerHTML = policyChecklistHtml(checklist);
  }
}

function routinePolicyModalOpen() {
  if (!dashRoutinePolicyModal) return;
  dashRoutinePolicyModal.classList.add('active');
  dashRoutinePolicyModal.setAttribute('aria-hidden', 'false');
  routinePolicyModalLoad();
}

function routinePolicyModalClose() {
  if (!dashRoutinePolicyModal) return;
  dashRoutinePolicyModal.classList.remove('active');
  dashRoutinePolicyModal.setAttribute('aria-hidden', 'true');
}

function routinePolicyModalViewRaw(showRaw) {
  const editor = document.getElementById('dash-routine-policy-editor');
  const form = document.getElementById('dash-routine-policy-form');
  if (editor) editor.style.display = showRaw ? 'block' : 'none';
  if (form) form.style.display = showRaw ? 'none' : 'block';
}

function routinePolicyModalSyncNameToEditor() {
  const editor = document.getElementById('dash-routine-policy-editor');
  const nameEl = document.getElementById('dash-routine-policy-name');
  if (!editor || !nameEl) return;
  try {
    const parsed = JSON.parse(editor.value || '{}');
    policyApplyDisplayName(parsed, nameEl.value);
    editor.value = JSON.stringify(parsed, null, 2);
  } catch (_) {}
}

function routinePolicyModalRenderForm() {
  const editor = document.getElementById('dash-routine-policy-editor');
  const form = document.getElementById('dash-routine-policy-form');
  const nameEl = document.getElementById('dash-routine-policy-name');
  if (!editor || !form) return;
  let parsed;
  try {
    parsed = JSON.parse(editor.value || '{}');
  } catch (err) {
    routinePolicyModalSetStatus(`Invalid JSON: ${err.message}`);
    return;
  }
  if (nameEl) nameEl.value = policyGetDisplayName(parsed, 'Operator Routine');
  policyRenderFormInto(form, parsed);
  routinePolicyModalRefreshInsights(parsed);
}

function routinePolicyModalOnFormInput(event) {
  policySyncFormChangeToEditor(
    event,
    document.getElementById('dash-routine-policy-editor'),
    document.getElementById('dash-routine-policy-name'),
    routinePolicyModalSetStatus
  );
  routinePolicyModalRenderForm();
}

async function routinePolicyModalLoad() {
  const loaded = await loadRoutinePolicyProfile();
  const editor = document.getElementById('dash-routine-policy-editor');
  const nameEl = document.getElementById('dash-routine-policy-name');
  const form = document.getElementById('dash-routine-policy-form');
  if (editor) editor.value = JSON.stringify(loaded.policy_json || { post_commit_profile: loaded.profile }, null, 2);
  if (nameEl) nameEl.value = policyGetDisplayName(loaded.policy_json || {}, 'Operator Routine');
  if (form && !form.dataset.bound) {
    form.addEventListener('input', routinePolicyModalOnFormInput);
    form.addEventListener('change', routinePolicyModalOnFormInput);
    form.dataset.bound = '1';
  }
  if (editor && !editor.dataset.boundPolicy) {
    editor.addEventListener('input', () => routinePolicyModalRefreshInsights());
    editor.dataset.boundPolicy = '1';
  }
  routinePolicyModalRenderForm();
  routinePolicyModalViewRaw(false);
  dashRoutinePolicySimulationId = '';
  dashRoutinePolicySimulationFingerprint = '';
  routinePolicyModalRefreshInsights(loaded.policy_json || null, loaded);
  routinePolicyModalSetStatus(`Loaded operator_routine policy (${loaded.source} v${loaded.version} [${loaded.status}])`);
}

async function routinePolicyModalLoadLatestActive() {
  const versionsRes = await fetchJsonSafe('/api/settings/policy/operator_routine/versions?limit=50');
  if (isErrorResponse(versionsRes)) {
    routinePolicyModalSetStatus(`Failed to list versions:\n${JSON.stringify(versionsRes, null, 2)}`);
    return;
  }
  const activeItem = (Array.isArray(versionsRes.items) ? versionsRes.items : []).find((item) => {
    return String(item.status || '').toLowerCase() === 'active';
  });
  if (!activeItem || !Number.isFinite(activeItem.version)) {
    routinePolicyModalSetStatus('No active operator_routine version found.');
    return;
  }
  const res = await fetchJsonSafe('/api/settings/policy/operator_routine/load_version', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ago_path: null, version: activeItem.version })
  });
  if (isErrorResponse(res)) {
    routinePolicyModalSetStatus(`Failed to load active version:\n${JSON.stringify(res, null, 2)}`);
    return;
  }
  const editor = document.getElementById('dash-routine-policy-editor');
  const nameEl = document.getElementById('dash-routine-policy-name');
  const loadedPolicy = res.policy_json || {};
  if (editor) editor.value = JSON.stringify(loadedPolicy, null, 2);
  if (nameEl) nameEl.value = policyGetDisplayName(res.policy_json || {}, 'Operator Routine');
  const loadedProfile = normalizeRoutineProfile((loadedPolicy.post_commit_profile || {}));
  dashRoutineWorkspaceState.loaded = {
    ok: true,
    source: String(res.source || 'AGOrg'),
    version: Number.isFinite(res.version) ? res.version : Number(activeItem.version || 0),
    status: String(res.status || 'active'),
    profile: loadedProfile,
    policy_json: { ...loadedPolicy, post_commit_profile: loadedProfile },
    diff: routineProfileDiff(loadedProfile)
  };
  dashRoutinePolicySimulationId = '';
  dashRoutinePolicySimulationFingerprint = '';
  routinePolicyModalRenderForm();
  routinePolicyModalViewRaw(false);
  routinePolicyModalSetStatus(`Loaded latest active operator_routine v${activeItem.version}.`);
}

async function routinePolicyModalSimulate() {
  if (!dashRoutinePolicyEditor) return;
  routinePolicyModalSyncNameToEditor();
  let policyJson;
  try {
    policyJson = JSON.parse(dashRoutinePolicyEditor.value);
  } catch (err) {
    routinePolicyModalSetStatus(`Invalid JSON: ${err.message}`);
    return;
  }
  const loaded = dashRoutineWorkspaceState.loaded || await loadRoutinePolicyProfile();
  const validation = routinePolicyDraftValidate(policyJson);
  if (!validation.ok) {
    routinePolicyModalSetStatus(routinePolicyDraftStatusSummary(validation, loaded));
    return;
  }
  const draftSummary = routinePolicyDraftStatusSummary(validation, loaded);
  const res = await fetchJsonSafe('/api/settings/policy/operator_routine/simulate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ago_path: null, policy_json: validation.normalizedPolicy })
  });
  if (isErrorResponse(res)) {
    routinePolicyModalSetStatus(`${draftSummary}\n\nSimulation failed:\n${JSON.stringify(res, null, 2)}`);
    routinePolicyModalRefreshInsights(validation.normalizedPolicy, loaded);
    return;
  }
  dashRoutinePolicySimulationId = res.evidence_id || '';
  dashRoutinePolicySimulationFingerprint = policyFingerprint(validation.normalizedPolicy);
  routinePolicyModalRefreshInsights(validation.normalizedPolicy, loaded);
  routinePolicyModalSetStatus(`${draftSummary}\n\nSimulation evidence: ${dashRoutinePolicySimulationId || 'missing'}`);
}

async function routinePolicyModalActivate() {
  if (!dashRoutinePolicySimulationId) {
    routinePolicyModalSetStatus('Simulation evidence is required before activation.');
    return;
  }
  if (!dashRoutinePolicyEditor) return;
  routinePolicyModalSyncNameToEditor();
  let policyJson;
  try {
    policyJson = JSON.parse(dashRoutinePolicyEditor.value);
  } catch (err) {
    routinePolicyModalSetStatus(`Invalid JSON: ${err.message}`);
    return;
  }
  const loaded = dashRoutineWorkspaceState.loaded || await loadRoutinePolicyProfile();
  const validation = routinePolicyDraftValidate(policyJson);
  if (!validation.ok) {
    routinePolicyModalSetStatus(routinePolicyDraftStatusSummary(validation, loaded));
    return;
  }
  const currentPolicyFingerprint = policyFingerprint(validation.normalizedPolicy);
  if (!dashRoutinePolicySimulationFingerprint || currentPolicyFingerprint !== dashRoutinePolicySimulationFingerprint) {
    routinePolicyModalSetStatus(
      `${routinePolicyDraftStatusSummary(validation, loaded)}\n\nDraft changed since last simulation. Re-run Simulate before Activate.`
    );
    routinePolicyModalRefreshInsights(validation.normalizedPolicy, loaded);
    return;
  }
  const res = await fetchJsonSafe('/api/settings/policy/operator_routine/activate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ago_path: null, simulation_evidence_id: dashRoutinePolicySimulationId })
  });
  if (isErrorResponse(res)) {
    routinePolicyModalSetStatus(res);
    routinePolicyModalRefreshInsights(validation.normalizedPolicy, loaded);
    return;
  }
  dashRoutinePolicySimulationId = '';
  dashRoutinePolicySimulationFingerprint = '';
  routinePolicyModalRefreshInsights(validation.normalizedPolicy, loaded);
  routinePolicyModalSetStatus(`Activation succeeded:\n${JSON.stringify(res, null, 2)}`);
  await dashLoadRoutine();
}

function routineCiChipLevel(state) {
  const normalized = String(state || '').toLowerCase();
  if (normalized === 'pass' || normalized === 'passed' || normalized === 'success' || normalized === 'succeeded' || normalized === 'completed' || normalized === 'ok') return 'ok';
  if (normalized === 'running' || normalized === 'in_progress' || normalized === 'queued') return 'warn';
  if (normalized === 'fail' || normalized === 'failure' || normalized === 'timed_out' || normalized === 'cancelled') return 'fail';
  return 'neutral';
}

function routineCiSummaryInFlight(summary = null) {
  if (!summary || typeof summary !== 'object') return false;
  const overallState = String(summary.overall_state || '').toLowerCase();
  return overallState === 'running' || overallState === 'queued' || overallState === 'in_progress' || overallState === 'pending';
}

function routineCiIsTerminalState(state) {
  const s = String(state || '').toLowerCase();
  return s === 'pass' || s === 'success' || s === 'completed' || s === 'ok' || s === 'fail' || s === 'failure' || s === 'cancelled' || s === 'timed_out';
}

function routineCiMergeStickySummary(prevSummary = null, nextSummary = null) {
  if (!nextSummary || typeof nextSummary !== 'object') return nextSummary;
  if (!prevSummary || typeof prevSummary !== 'object') return nextSummary;
  const merged = { ...nextSummary };

  const mergeWorkflowSticky = (prefix, runIdKey, runUrlKey, stateKeys) => {
    const prevRunId = String(prevSummary[runIdKey] || '').toLowerCase();
    const nextRunId = String(merged[runIdKey] || '').toLowerCase();
    const sameRun = prevRunId && nextRunId && prevRunId === nextRunId;
    const nextUnknownRun = !nextRunId || nextRunId === 'unknown';
    if ((sameRun || nextUnknownRun) && (!merged[runUrlKey] || String(merged[runUrlKey]).toLowerCase() === 'unknown')) {
      if (prevSummary[runUrlKey]) merged[runUrlKey] = prevSummary[runUrlKey];
    }
    if (nextUnknownRun && prevRunId && prevRunId !== 'unknown') {
      merged[runIdKey] = prevSummary[runIdKey];
    }
    if (!(sameRun || nextUnknownRun)) return;
    stateKeys.forEach((key) => {
      const nextState = routineNormalizeCiState(merged[key], merged);
      const prevState = routineNormalizeCiState(prevSummary[key], prevSummary);
      if ((nextState === 'idle' || nextState === 'unknown') && routineCiIsTerminalState(prevState)) {
        merged[key] = prevSummary[key];
      }
    });
  };

  mergeWorkflowSticky('docs', 'docs_run_id', 'docs_run_url', ['docs_state', 'docs_build_state', 'docs_deploy_state']);
  mergeWorkflowSticky('pypi', 'pypi_run_id', 'pypi_run_url', ['pypi_state', 'pypi_build_and_publish_state']);
  mergeWorkflowSticky('ci', 'ci_run_id', 'run_url', ['overall_state', 'rust_state', 'ui_smoke_state', 'packaging_parity_state']);

  return merged;
}

function routineNormalizeCiState(raw, summary = null) {
  let s = String(raw || 'idle').toLowerCase();
  if (s === 'unknown' || s === 'null' || s === '') s = 'idle';
  const workspaceInFlight = !!dashRoutineWorkspaceState.ciInFlight;
  const summaryInFlight = routineCiSummaryInFlight(summary);
  if (workspaceInFlight && summaryInFlight) {
    if (s === 'pass' || s === 'passed' || s === 'success' || s === 'succeeded' || s === 'completed' || s === 'ok') s = 'running';
  }
  return s;
}

function routineSetCiJobChips(summary = {}) {
  const docsState = routineNormalizeCiState(summary.docs_state, summary);
  const rustState = routineNormalizeCiState(summary.rust_state, summary);
  const uiState = routineNormalizeCiState(summary.ui_smoke_state, summary);
  const packagingState = routineNormalizeCiState(summary.packaging_parity_state, summary);
  routineSetChip(dashRoutineCiDocsChip, `Docs: ${docsState}`, routineCiChipLevel(docsState));
  routineSetChip(dashRoutineCiRustChip, `Rust: ${rustState}`, routineCiChipLevel(rustState));
  routineSetChip(dashRoutineCiUiChip, `UI Smoke: ${uiState}`, routineCiChipLevel(uiState));
  routineSetChip(dashRoutineCiPackagingChip, `Packaging: ${packagingState}`, routineCiChipLevel(packagingState));
}

async function routineSyncCiFromGitHub(opts = {}) {
  const force = !!opts.force;
  if (routineCiSyncBusy) return;
  if (!force && currentTab !== 'dashboard') return;
  routineCiSyncBusy = true;
  try {
    routineSyncPushControls();
    const catalogRes = await routineLoadCiCatalog();
    if (isErrorResponse(catalogRes)) {
      routineRenderCiObservatory({}, catalogRes);
      return;
    }
    const ciBranch = routineReadBranchRemote().branch;
    const statusRes = await depRun('ci-status', { branch: ciBranch });
    const statusInner = depEnvelopeInner(statusRes);
    const statusSummary = statusInner && statusInner.summary && typeof statusInner.summary === 'object'
      ? statusInner.summary
      : null;
    if (!statusSummary) return;
    const mergedSummary = routineCiMergeStickySummary(dashRoutineWorkspaceState.ciDetail?.summary || null, statusSummary);
    routineSetCiJobChips(mergedSummary);
    dashRoutineWorkspaceState.ciDetail = statusInner
      ? { ...statusInner, summary: mergedSummary }
      : { ...(statusRes || {}), summary: mergedSummary };
    routineRenderCiObservatory(mergedSummary, dashRoutineWorkspaceState.ciDetail);
    if (dashRoutineSelectedStage === 'ci') routineRenderWorkspace();
    routineMaybeAutoContinueFromCi(statusSummary);
  } finally {
    routineCiSyncBusy = false;
  }
}

function routineStopCiSyncLoop() {
  if (routineCiSyncTimer) {
    clearInterval(routineCiSyncTimer);
    routineCiSyncTimer = null;
  }
}

function routineStartCiSyncLoop() {
  routineStopCiSyncLoop();
  routineCiSyncTimer = setInterval(() => {
    routineSyncCiFromGitHub().catch(() => {});
  }, 15000);
  routineSyncCiFromGitHub({ force: true }).catch(() => {});
}

async function dashRefreshCiStatus() {
  const btn = document.querySelector('button[onclick="dashRefreshCiStatus()"]');
  if (btn) {
    btn.disabled = true;
    btn.textContent = '...';
  }
  try {
    await routineSyncCiFromGitHub({ force: true });
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.textContent = 'Refresh CI';
    }
  }
}

async function dashRefreshCdStatus() {
  const btn = document.querySelector('button[onclick="dashRefreshCdStatus()"]');
  if (btn) {
    btn.disabled = true;
    btn.textContent = '...';
  }
  try {
    routineRefreshPlanPreview();
    await routineLoadResolveSnapshot();
    await routineLoadMultiSnapshot();
    await routineLoadCiCatalog();
    routineRenderCiObservatory(
      dashRoutineWorkspaceState.ciDetail?.summary || {},
      dashRoutineWorkspaceState.ciDetail || null
    );
    routineRenderWorkspace();
    await new Promise(r => setTimeout(r, 800)); 
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.textContent = 'Refresh';
    }
  }
}

async function dashLoadRoutine() {
  const btn = document.querySelector('button[onclick="dashLoadRoutine()"]');
  if (btn) {
    btn.disabled = true;
    btn.textContent = 'Loading...';
  }
  try {
    const loaded = await loadRoutinePolicyProfile();
    routineApplyPolicyProfile(loaded);
    await routineLoadResolveSnapshot();
    await routineLoadCiCatalog();
    const detail = document.getElementById('dash-routine-policy-detail');
    if (detail) {
      detail.textContent = JSON.stringify(loaded.profile, null, 2);
    }
    routineRenderCiObservatory(
      dashRoutineWorkspaceState.ciDetail?.summary || {},
      dashRoutineWorkspaceState.ciDetail || null
    );
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.textContent = 'Load Routine';
    }
  }
}

async function dashRefreshRoutinePolicy() {
  const btn = document.querySelector('button[onclick="dashRefreshRoutinePolicy()"]');
  if (btn) {
    btn.disabled = true;
    btn.textContent = 'Refreshing...';
  }
  try {
    const loaded = await loadRoutinePolicyProfile();
    routineApplyPolicyProfile(loaded);
    const view = document.getElementById('dash-routine-policy-view');
    const detail = document.getElementById('dash-routine-policy-detail');
    if (detail && view && view.style.display !== 'none') {
      detail.textContent = JSON.stringify(loaded.profile, null, 2);
    }
    routineAnnounceStatus('Operator routine policy refreshed (' + loaded.source + ' v ' + loaded.version + ' [' + loaded.status + ']).');
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.textContent = 'Refresh Policy';
    }
  }
}

function dashToggleRoutinePolicyView() {
  const view = document.getElementById('dash-routine-policy-view');
  const detail = document.getElementById('dash-routine-policy-detail');
  if (!view || !detail) return;
  
  if (view.style.display === 'none') {
    if (!detail.textContent) {
      loadRoutinePolicyProfile().then(loaded => {
        detail.textContent = JSON.stringify(loaded.profile, null, 2);
      });
    }
    view.style.display = 'block';
  } else {
    view.style.display = 'none';
  }
}

function routineResetChips() {
  routineSetChip(dashRoutineScopeChip, 'Scope: idle', 'neutral');
  routineSetChip(dashRoutineMultiChip, 'Multi: idle', 'neutral');
  routineSetChip(dashRoutineGatesChip, 'Gates: idle', 'neutral');
  routineSetChip(dashRoutinePushChip, 'Push: idle', 'neutral');
  routineSetChip(dashRoutineEvidenceChip, 'Evidence: idle', 'neutral');
  dashRoutineWorkspaceState.ciInFlight = false;
  routineSetCiJobChips({});
  dashRoutineWorkspaceState.ciCatalog = null;
  dashRoutineWorkspaceState.ciSelectedWorkflowKey = '';
  dashRoutineWorkspaceState.healStatus = '';
  dashRoutineCodexAuto = { active: false, entries: [], summary: '' };
  dashRoutineWorkspaceState.pushNoop = false;
  dashRoutineAutoHealAttempted = false;
  dashRoutineCanResume = false;
  routineRenderCiObservatory({});
  routineSetChip(dashRoutineProfileSourceChip, 'Profile: loading', 'neutral');
  routineSetChip(dashRoutineProfileStepsChip, 'Steps: -', 'neutral');
  routineSetLastResult('idle', 'Last Result: idle');
  ROUTINE_STAGE_ORDER.forEach((stage) => {
    const defaultText = stage === 'plan' ? 'Preview' : 'Idle';
    routineSetStageState(stage, defaultText, stage === 'plan' ? 'warn' : 'neutral');
  });
}

function routineResetTimeline() {
  dashRoutineTrace = [];
  routineRenderTimeline();
  routineRenderActions();
}

function routineRecord(stage, status, summary, detail = '', remediation = '', actionId = '', startedAt = 0) {
  const finishedAt = Date.now();
  const durationMs = startedAt > 0 ? Math.max(0, finishedAt - startedAt) : 0;
  const entry = {
    ts: new Date(finishedAt).toISOString(),
    stage,
    status,
    summary,
    detail,
    remediation,
    actionId,
    durationMs
  };
  dashRoutineTrace.push(entry);
  if (status === 'fail') routineRecordFailure(entry);
  if (status === 'pass' && stage === 'Reconcile') routineRecordFailure(null);
  dashRoutineWorkspaceState.lastRunAt = entry.ts;
  routineRenderTimeline();
  routineRenderActions();
  if (dashRoutineSelectedStage === 'reconcile' || dashRoutineSelectedStage === routineStageKey(String(stage || '').toLowerCase())) {
    routineRenderWorkspace();
  }
}

function routineRenderTimeline() {
  if (!dashRoutineTimeline) return;
  if (!dashRoutineTrace.length) {
    dashRoutineTimeline.innerHTML = '<div class="tl-empty">No routine run yet.</div>';
    return;
  }
  const stageProgressKey = (entry) => {
    const stage = String(entry.stage || '').trim();
    const summary = String(entry.summary || '');
    const m = summary.match(/\((\d+\/\d+)\)/);
    if (!m) return '';
    return `${stage}|${m[1]}`;
  };
  const resolvedProgressKeys = new Set();
  for (const entry of dashRoutineTrace) {
    if (entry.status === 'pass' || entry.status === 'fail') {
      const key = stageProgressKey(entry);
      if (key) resolvedProgressKeys.add(key);
    }
  }
  const visible = dashRoutineTrace.filter((entry) => {
    if (entry.status !== 'running') return true;
    const key = stageProgressKey(entry);
    return !key || !resolvedProgressKeys.has(key);
  });

  const rows = visible.map((e) => {
    const stateClass = e.status === 'pass' ? 'completed' : (e.status === 'fail' ? 'failed' : 'running');
    const stageClass = String(e.stage || '').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
    const duration = e.durationMs > 0 ? `${e.durationMs}ms` : '-';
    const safeDetail = String(e.detail || '').replaceAll('<', '&lt;');
    const safeRemediation = String(e.remediation || '').replaceAll('<', '&lt;');
    return `
      <div class="tl-item ${stateClass} ${stageClass ? `stage-${stageClass}` : ''}">
        <div class="tl-head">
          <span><b>${e.stage}</b> (${duration})</span>
          <span class="chip ${e.status === 'pass' ? 'ok' : (e.status === 'fail' ? 'fail' : 'warn')}">${String(e.status || '').toUpperCase()}</span>
        </div>
        <div class="tl-body">${e.summary || ''}</div>
        ${safeRemediation ? `<div class="muted" style="margin-top:6px;">Remediation: ${safeRemediation}</div>` : ''}
        ${safeDetail ? `<details style="margin-top:6px;"><summary>Step details</summary><pre style="margin-top:6px;">${safeDetail}</pre></details>` : ''}
      </div>
    `;
  });
  dashRoutineTimeline.innerHTML = rows.join('\n');
}

function routineRenderActions() {
  if (!dashRoutineActions) return;
  const fail = [...dashRoutineTrace].reverse().find((e) => e.status === 'fail' && e.actionId);
  if (!fail) {
    dashRoutineActions.innerHTML = '';
    return;
  }
  let label = 'Open Relevant Panel';
  if (fail.actionId === 'open_agorg') label = 'Open AGOrg Panel';
  if (fail.actionId === 'open_multi') label = 'Open Multi Panel';
  if (fail.actionId === 'open_dashboard_gate') label = 'Open Dashboard Gates';
  if (fail.actionId === 'open_push') label = 'Open Push Controls';
  if (fail.actionId === 'open_ci') label = 'Open CI Summary';
  const classified = routineClassifyFailureForHeal(fail);
  const healBtn = classified.playbook.length
    ? `<button class="btn" onclick="routineAutoHealAndRetry()" ${dashRoutineAutoHealRunning ? 'disabled' : ''}>${dashRoutineAutoHealRunning ? 'Auto-Heal Running...' : 'Auto-Heal + Verify'}</button>`
    : '';
  const resumeBtn = dashRoutineCanResume
    ? `<button class="btn" onclick="dashResumePostCommitRoutine()" ${dashRoutineRunning ? 'disabled' : ''}>Resume from Failed Stage</button>`
    : '';
  const statusBlock = dashRoutineWorkspaceState.healStatus
    ? `<pre style="margin:6px 0 0; max-height:160px; overflow:auto; font-size:0.72rem;">${routineEscapeHtml(dashRoutineWorkspaceState.healStatus)}</pre>`
    : '';
  const codexAutoBlock = routineRenderCodexAutoStatus();
  const healStats = routineHealLogStats();
  dashRoutineActions.innerHTML = `
    <div class="row" style="gap:8px; align-items:center; flex-wrap:wrap;">
      <button class="btn secondary" onclick="routineRunAction('${fail.actionId}')">${label}</button>
      ${healBtn}
      ${resumeBtn}
      <button class="btn secondary" onclick="routineEscalateFailureToCodex()">Escalate to Codex</button>
      <button class="btn secondary" onclick="routineShowHealLog()">Heal Log (${healStats.total})</button>
      <button class="btn secondary" onclick="routineShowHealRecipes()">Recipes (${healStats.learned})</button>
      <button class="btn secondary" onclick="routineClearHealLog()">Clear Heal Log</button>
      <button class="btn secondary" onclick="routineClearHealRecipes()">Clear Recipes</button>
    </div>
    ${statusBlock}
    ${codexAutoBlock}
  `;
}

function routineCodexAutoRecord(label, status, detail = '') {
  const entry = {
    ts: new Date().toISOString(),
    label: String(label || ''),
    status: String(status || 'neutral'),
    detail: String(detail || '')
  };
  dashRoutineCodexAuto.entries.push(entry);
  if (dashRoutineCodexAuto.entries.length > 18) {
    dashRoutineCodexAuto.entries = dashRoutineCodexAuto.entries.slice(-18);
  }
  routineRenderActions();
  routineRenderWorkspace();
}

function routineCodexAutoStart(context = '') {
  dashRoutineCodexAuto = { active: true, entries: [], summary: '' };
  routineCodexAutoRecord('Codex Auto Start', 'running', context);
}

function routineCodexAutoFinish(ok, summary = '') {
  dashRoutineCodexAuto.active = false;
  dashRoutineCodexAuto.summary = String(summary || '');
  routineCodexAutoRecord('Codex Auto Finish', ok ? 'pass' : 'fail', summary);
}

function routineRenderCodexAutoStatus() {
  const hasEntries = Array.isArray(dashRoutineCodexAuto.entries) && dashRoutineCodexAuto.entries.length > 0;
  const hasSummary = !!String(dashRoutineCodexAuto.summary || '').trim();
  if (!hasEntries && !hasSummary) return '';
  const rows = (dashRoutineCodexAuto.entries || []).map((e) => {
    const level = e.status === 'pass' ? 'ok' : (e.status === 'fail' ? 'fail' : (e.status === 'running' ? 'warn' : 'neutral'));
    const safeLabel = routineEscapeHtml(e.label || '');
    const safeDetail = routineEscapeHtml(e.detail || '');
    return `<div class="tl-head" style="margin-top:4px;"><span>${safeLabel}</span><span class="chip ${level}">${String(e.status || '').toUpperCase()}</span></div>${safeDetail ? `<div class="muted" style="margin-top:3px;">${safeDetail}</div>` : ''}`;
  }).join('');
  const summary = hasSummary ? `<div class="muted" style="margin-top:6px;">${routineEscapeHtml(dashRoutineCodexAuto.summary)}</div>` : '';
  return `<div role="status" aria-live="polite" style="margin-top:8px; padding:8px; border:1px solid var(--border); border-radius:6px; background: rgba(3,12,20,0.45);"><b>Codex Auto Progress</b>${rows}${summary}</div>`;
}

function routineLatestFailureEntry() {
  return [...dashRoutineTrace].reverse().find((e) => e.status === 'fail') || null;
}

function routineFailureStageToStep(entry) {
  const stage = String(entry?.stage || '').toLowerCase().trim();
  if (stage === 'scope') return 'scope';
  if (stage === 'multi') return 'multi';
  if (stage === 'gates') return 'gates';
  if (stage === 'push') return 'push';
  if (stage === 'ci') return 'ci';
  if (stage === 'evidence') return 'evidence';
  return '';
}

function routineParseFailureDetail(entry) {
  if (!entry || !entry.detail) return null;
  try {
    return JSON.parse(entry.detail);
  } catch (_) {
    return null;
  }
}

function routineIsCiAuthErrorPayload(detail) {
  const inner = detail && detail.inner && typeof detail.inner === 'object' ? detail.inner : detail;
  const out = String(inner?.stdout || '');
  const err = String(inner?.stderr || '');
  const merged = `${out}\n${err}`.toLowerCase();
  const exitCode = Number(inner?.exit_code);
  return (
    merged.includes('gh authentication')
    || merged.includes('gh auth login')
    || (Number.isFinite(exitCode) && exitCode === 6)
  );
}

function routineClassifyFailureForHeal(entry) {
  const parsed = routineParseFailureDetail(entry);
  const inner = parsed && parsed.inner && typeof parsed.inner === 'object' ? parsed.inner : parsed;
  const action = String(inner?.action || '').toLowerCase();
  const out = String(inner?.stdout || '');
  const err = String(inner?.stderr || '');
  const merged = `${out}\n${err}`.toLowerCase();

  const base = {
    signature: 'unknown',
    confidence: 'low',
    playbook: [],
    verifyAction: '',
    summary: 'No known safe remediation matched.'
  };

  const learned = routineMatchLearnedHealRecipe(entry);
  if (learned) {
    return {
      signature: `learned:${learned.signature || learned.fingerprint || 'recipe'}`,
      confidence: learned.confidence || 'medium',
      playbook: Array.isArray(learned.playbook) ? learned.playbook : [],
      verifyAction: learned.verifyAction || '',
      summary: `Using learned remediation recipe (${learned.source || 'local-history'}).`
    };
  }

  if (entry?.stage === 'Gates' && action === 'prepush-gate') {
    if (merged.includes('diff in') && merged.includes('[cargo-fmt]')) {
      return {
        signature: 'format_parity',
        confidence: 'high',
        playbook: [{ action: 'cargo-fmt', label: 'Apply formatter' }],
        verifyAction: 'gate',
        summary: 'Detected format parity failure in pre-push gate.'
      };
    }
    if (merged.includes('repair_lock_182') || merged.includes('lockfile compatibility') || merged.includes('edition2024')) {
      return {
        signature: 'lock_drift',
        confidence: 'medium',
        playbook: [{ action: 'repair', label: 'Repair frozen lock lane' }],
        verifyAction: 'gate',
        summary: 'Detected lock drift signal in pre-push gate.'
      };
    }
    return {
      signature: 'gate_failure_generic',
      confidence: 'medium',
      playbook: [],
      verifyAction: 'gate',
      summary: 'Gate failure requires operator/Codex investigation.'
    };
  }

  if (entry?.stage === 'Push') {
    return {
      signature: 'push_failure_generic',
      confidence: 'low',
      playbook: [],
      verifyAction: 'push',
      summary: 'Push failure may require manual branch/remote/policy intervention.'
    };
  }

  if (entry?.stage === 'CI') {
    if (routineIsCiAuthErrorPayload(parsed || entry?.detail || null) || merged.includes('not installed')) {
      return {
        signature: 'ci_cli_auth',
        confidence: 'high',
        playbook: [],
        verifyAction: 'ci-status',
        summary: 'Detected CI observer environment/auth issue; run gh auth login in pilot runtime context.'
      };
    }
    return {
      signature: 'ci_failure_generic',
      confidence: 'low',
      playbook: [],
      verifyAction: 'ci-status',
      summary: 'CI failure is not in safe local auto-heal registry.'
    };
  }

  return base;
}

function routineNormalizeFailureText(value) {
  let text = String(value || '').toLowerCase();
  text = text.replace(/[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}/g, '<uuid>');
  text = text.replace(/\d{4}-\d{2}-\d{2}t\d{2}:\d{2}:\d{2}(?:\.\d+)?z/g, '<ts>');
  text = text.replace(/:\d{2,6}/g, ':#');
  text = text.replace(/\b\d{3,}\b/g, '#');
  text = text.replace(/\s+/g, ' ').trim();
  return text.slice(0, 320);
}

function routineFailureFingerprint(entry) {
  if (!entry || typeof entry !== 'object') return '';
  const parsed = routineParseFailureDetail(entry);
  const inner = parsed && parsed.inner && typeof parsed.inner === 'object' ? parsed.inner : parsed;
  const merged = `${String(inner?.stdout || '')}\n${String(inner?.stderr || '')}`;
  const key = {
    stage: String(entry.stage || '').toLowerCase(),
    action: String(inner?.action || '').toLowerCase(),
    summary: routineNormalizeFailureText(entry.summary || ''),
    remediation: routineNormalizeFailureText(entry.remediation || ''),
    detail: routineNormalizeFailureText(merged)
  };
  return JSON.stringify(key);
}

function routineHealRecipeRead() {
  try {
    const raw = window.localStorage.getItem(ROUTINE_HEAL_RECIPE_KEY);
    const items = raw ? JSON.parse(raw) : [];
    return Array.isArray(items) ? items : [];
  } catch (_) {
    return [];
  }
}

function routineHealRecipeWrite(items) {
  try {
    const next = Array.isArray(items) ? items.slice(-200) : [];
    window.localStorage.setItem(ROUTINE_HEAL_RECIPE_KEY, JSON.stringify(next));
  } catch (_) {}
}

function routineIsSafeHealPlaybook(playbook) {
  if (!Array.isArray(playbook) || !playbook.length) return false;
  return playbook.every((step) => ROUTINE_SAFE_HEAL_ACTIONS.has(String(step?.action || '').trim()));
}

function routineLearnSuccessfulHeal(entry, classified) {
  if (!entry || !classified || !routineIsSafeHealPlaybook(classified.playbook)) return;
  const fingerprint = routineFailureFingerprint(entry);
  if (!fingerprint) return;
  const items = routineHealRecipeRead();
  const next = Array.isArray(items) ? items : [];
  const recipe = {
    ts: new Date().toISOString(),
    fingerprint,
    stage: String(entry.stage || ''),
    signature: String(classified.signature || 'unknown'),
    confidence: String(classified.confidence || 'medium'),
    verifyAction: String(classified.verifyAction || ''),
    playbook: classified.playbook.map((step) => ({
      action: String(step.action || ''),
      label: String(step.label || step.action || '')
    })),
    source: 'auto-heal-success'
  };
  const existingIdx = next.findIndex((item) => item && item.fingerprint === fingerprint);
  if (existingIdx >= 0) next[existingIdx] = recipe;
  else next.push(recipe);
  routineHealRecipeWrite(next);
}

function routineMatchLearnedHealRecipe(entry) {
  const fingerprint = routineFailureFingerprint(entry);
  if (!fingerprint) return null;
  const items = routineHealRecipeRead();
  const hit = items.find((item) => item && item.fingerprint === fingerprint);
  if (!hit || !routineIsSafeHealPlaybook(hit.playbook)) return null;
  return hit;
}

function routineHealLogAppend(record) {
  try {
    const raw = window.localStorage.getItem(ROUTINE_HEAL_LOG_KEY);
    const items = raw ? JSON.parse(raw) : [];
    const next = Array.isArray(items) ? items : [];
    next.push(record);
    window.localStorage.setItem(ROUTINE_HEAL_LOG_KEY, JSON.stringify(next.slice(-200)));
  } catch (_) {}
}

function routineHealLogRead() {
  try {
    const raw = window.localStorage.getItem(ROUTINE_HEAL_LOG_KEY);
    const items = raw ? JSON.parse(raw) : [];
    return Array.isArray(items) ? items : [];
  } catch (_) {
    return [];
  }
}

function routineHealLogStats() {
  const items = routineHealLogRead();
  let success = 0;
  let failed = 0;
  for (const item of items) {
    if (item && item.ok) success += 1;
    else failed += 1;
  }
  const learned = routineHealRecipeRead().length;
  return { total: items.length, success, failed, learned };
}

function routineShowHealLog() {
  const items = routineHealLogRead();
  if (!items.length) {
    dashRoutineWorkspaceState.healStatus = 'Heal log is empty.';
    routineRenderActions();
    routineAnnounceStatus('Heal log is empty.');
    return;
  }
  const lines = ['Heal Log (latest first):'];
  const recent = [...items].reverse().slice(0, 20);
  recent.forEach((item) => {
    lines.push(`- ${item.ts || 'unknown'} | ${item.signature || 'unknown'} | ${item.ok ? 'PASS' : 'FAIL'} | stage=${item.stage || '-'} | confidence=${item.confidence || 'n/a'}`);
  });
  const recipes = routineHealRecipeRead();
  if (recipes.length) {
    lines.push('', `Learned recipes: ${recipes.length}`);
    [...recipes].reverse().slice(0, 10).forEach((item) => {
      const actions = (item.playbook || []).map((step) => step.action).join(' -> ') || '-';
      lines.push(`- ${item.ts || 'unknown'} | stage=${item.stage || '-'} | ${item.signature || 'unknown'} | ${actions}`);
    });
  }
  dashRoutineWorkspaceState.healStatus = lines.join('\n');
  routineRenderActions();
  routineAnnounceStatus(`Heal log loaded (${items.length} records).`);
}

function routineShowHealRecipes() {
  const recipes = routineHealRecipeRead();
  if (!recipes.length) {
    dashRoutineWorkspaceState.healStatus = 'No learned recipes yet.';
    routineRenderActions();
    routineAnnounceStatus('No learned recipes available.');
    return;
  }
  const lines = [`Learned Recipes (${recipes.length})`];
  [...recipes].reverse().slice(0, 20).forEach((item) => {
    const actions = (item.playbook || []).map((step) => step.action).join(' -> ') || '-';
    const verifyAction = item.verifyAction || '-';
    lines.push(`- ${item.ts || 'unknown'} | stage=${item.stage || '-'} | ${item.signature || 'unknown'} | fix=${actions} | verify=${verifyAction}`);
  });
  dashRoutineWorkspaceState.healStatus = lines.join('\n');
  routineRenderActions();
  routineAnnounceStatus(`Loaded ${recipes.length} learned heal recipes.`);
}

function routineClearHealLog() {
  try {
    window.localStorage.removeItem(ROUTINE_HEAL_LOG_KEY);
  } catch (_) {}
  dashRoutineWorkspaceState.healStatus = 'Heal log cleared.';
  routineRenderActions();
  routineAnnounceStatus('Heal log cleared.');
}

function routineClearHealRecipes() {
  try {
    window.localStorage.removeItem(ROUTINE_HEAL_RECIPE_KEY);
  } catch (_) {}
  dashRoutineWorkspaceState.healStatus = 'Learned heal recipes cleared.';
  routineRenderActions();
  routineAnnounceStatus('Learned heal recipes cleared.');
}

async function routineAutoHealAndRetry(options = {}) {
  if (dashRoutineAutoHealRunning) return { attempted: false, ok: false, escalated: false };
  const fromAuto = !!options.fromAuto;
  const fail = routineLatestFailureEntry();
  if (!fail) return { attempted: false, ok: false, escalated: false };
  const classified = routineClassifyFailureForHeal(fail);
  if (!classified.playbook.length) {
    dashRoutineWorkspaceState.healStatus = `Auto-heal skipped: ${classified.summary}`;
    routineHealLogAppend({
      ts: new Date().toISOString(),
      signature: classified.signature,
      confidence: classified.confidence,
      ok: false,
      stage: fail.stage,
      summary: fail.summary,
      reason: 'no_safe_playbook'
    });
    routineRenderActions();
    const codexAuto = routineAutoCodexEnabled();
    await routineEscalateFailureToCodex({ autoRun: codexAuto, autoResume: codexAuto });
    return { attempted: false, ok: false, escalated: true, codexAuto };
  }

  dashRoutineAutoHealAttempted = true;
  dashRoutineAutoHealRunning = true;
  dashRoutineWorkspaceState.healStatus = `${fromAuto ? 'Automatic' : 'Manual'} auto-heal started for signature=${classified.signature} (${classified.confidence}).`;
  routineRenderActions();
  routineAnnounceStatus(`Auto-heal started for ${classified.signature}.`);

  let ok = true;
  const lines = [`Auto-heal signature: ${classified.signature}`, `Summary: ${classified.summary}`];
  try {
    for (const step of classified.playbook) {
      lines.push(`Running fix step: ${step.action}`);
      const res = await depRun(step.action);
      const pass = isDepStepPass(step.action, res);
      lines.push(`- ${step.action}: ${pass ? 'PASS' : 'FAIL'}`);
      if (!pass) {
        ok = false;
        lines.push(`- detail: ${JSON.stringify(depEnvelopeInner(res) || res || {}, null, 2)}`);
        break;
      }
    }

    if (ok && classified.verifyAction) {
      lines.push(`Verifying failed stage via: ${classified.verifyAction}`);
      const verifyRes = await depRun(classified.verifyAction);
      const verifyPass = isDepStepPass(classified.verifyAction, verifyRes);
      lines.push(`- verify ${classified.verifyAction}: ${verifyPass ? 'PASS' : 'FAIL'}`);
      ok = verifyPass;
      if (!verifyPass) lines.push(`- detail: ${JSON.stringify(depEnvelopeInner(verifyRes) || verifyRes || {}, null, 2)}`);
      if (classified.verifyAction === 'gate') {
        dashRoutineWorkspaceState.gateDetail = depEnvelopeInner(verifyRes);
      } else if (classified.verifyAction === 'push') {
        dashRoutineWorkspaceState.pushDetail = depEnvelopeInner(verifyRes);
      } else if (classified.verifyAction === 'ci-status') {
        dashRoutineWorkspaceState.ciDetail = depEnvelopeInner(verifyRes);
      }
    }
  } finally {
    dashRoutineAutoHealRunning = false;
  }

  const timestamp = new Date().toISOString();
  if (ok) {
    lines.push('Auto-heal result: SUCCESS');
    lines.push('Next: retry the routine or continue from failed stage.');
    routineLearnSuccessfulHeal(fail, classified);
    dashRoutineCanResume = true;
  } else {
    lines.push('Auto-heal result: FAILED');
    lines.push('Escalating to Codex with incident context.');
  }
  dashRoutineWorkspaceState.healStatus = lines.join('\n');
  routineHealLogAppend({
    ts: timestamp,
    signature: classified.signature,
    confidence: classified.confidence,
    ok,
    stage: fail.stage,
    summary: fail.summary
  });
  routineRenderActions();
  routineRenderWorkspace();
  if (!ok) {
    routineAnnounceAlert('Auto-heal failed. Escalating to Codex.');
    const codexAuto = routineAutoCodexEnabled();
    await routineEscalateFailureToCodex({ autoRun: codexAuto, autoResume: codexAuto });
    return { attempted: true, ok: false, escalated: true, codexAuto };
  }
  routineAnnounceStatus('Auto-heal completed successfully.');
  return { attempted: true, ok: true, escalated: false };
}

async function dashResumePostCommitRoutine() {
  if (dashRoutineRunning) return;
  const fail = routineLatestFailureEntry();
  const resumeFromStep = routineFailureStageToStep(fail);
  if (!resumeFromStep) {
    dashRoutineWorkspaceState.healStatus = 'Resume unavailable: no resumable failed stage in trace.';
    routineRenderActions();
    routineAnnounceAlert('Resume unavailable: no resumable failed stage.');
    return;
  }
  await dashRunPostCommitRoutine({ resumeFromStep });
}

function routineBuildCodexEscalationPacket(fail, options = {}) {
  if (!fail) return null;
  const classified = routineClassifyFailureForHeal(fail);
  const parsed = routineParseFailureDetail(fail);
  const branchRemote = routineReadBranchRemote();
  const branch = options.branch || branchRemote.branch;
  const remote = options.remote || branchRemote.remote;
  const secondPass = !!options.secondPass;
  const intent = `Resolve routine failure (${fail.stage}): ${classified.signature}`;
  const payload = {
    stage: fail.stage,
    summary: fail.summary,
    remediation: fail.remediation || '',
    signature: classified.signature,
    confidence: classified.confidence,
    action_id: fail.actionId || '',
    detail: parsed || fail.detail || '',
    trace_tail: dashRoutineTrace.slice(-8)
  };
  let cmd = 'pilot.multi.status';
  let verify = 'pilot.multi.status';
  if (fail.stage === 'CI' && secondPass) {
    cmd = 'pilot.dependency.ci-trigger';
    verify = 'pilot.dependency.ci-watch';
    payload.branch = branch;
    payload.ci_timeout_sec = 1800;
    payload.summary = `CI remediation pass (trigger + watch): ${fail.summary || classified.summary}`;
  } else if (fail.stage === 'CI') {
    cmd = 'pilot.dependency.ci-status';
    verify = 'pilot.dependency.ci-status';
    payload.branch = branch;
    payload.ci_timeout_sec = 1800;
  } else if (fail.stage === 'Gates') {
    cmd = 'pilot.dependency.gate';
    verify = 'pilot.dependency.gate';
  } else if (fail.stage === 'Push') {
    cmd = 'pilot.dependency.push';
    verify = 'pilot.dependency.push';
    payload.branch = branch;
    payload.remote = remote;
  }
  const expected = secondPass
    ? `CI remediation pass triggers and verifies a fresh successful workflow run for ${branch}.`
    : `Root cause identified and minimal safe remediation applied; failed stage (${fail.stage}) verifies PASS.`;
  const rollback = 'If mutation is unsafe, revert to last clean branch state and rerun in preview mode.';
  return { intent, cmd, payload, expected, rollback, verify };
}

function routineApplyCodexPacket(packet) {
  if (!packet) return;
  activatePanel('codex');
  const setVal = (id, value) => {
    const el = document.getElementById(id);
    if (el) el.value = value;
  };
  setVal('codex-intent', packet.intent);
  setVal('codex-command', packet.cmd);
  setVal('codex-payload', JSON.stringify(packet.payload, null, 2));
  setVal('codex-expected', packet.expected);
  setVal('codex-rollback', packet.rollback);
  setVal('codex-verify', packet.verify);
  if (codexOut) {
    codexOut.textContent = `Codex escalation packet prepared.\n\n${JSON.stringify(packet.payload, null, 2)}`;
  }
}

function routineCodexIsHealthyForStage(fail, contract) {
  if (!contract) return false;
  const command = String(contract.command || '').trim();
  const action = command.startsWith('pilot.dependency.')
    ? command.slice('pilot.dependency.'.length)
    : '';
  const response = (contract.verify_response && typeof contract.verify_response === 'object')
    ? contract.verify_response
    : ((contract.execute_response && typeof contract.execute_response === 'object') ? contract.execute_response : null);
  if (!response) return false;

  if (action === 'ci-status') {
    const summary = response.summary && typeof response.summary === 'object' ? response.summary : {};
    const overallState = String(summary.overall_state || '').toLowerCase();
    const conclusion = String(summary.overall_conclusion || '').toLowerCase();
    if (overallState === 'fail' || conclusion === 'failure') return false;
    if (overallState === 'success' || overallState === 'pass' || conclusion === 'success') return true;
    return false;
  }
  if (action === 'ci-watch') {
    const summary = response.summary && typeof response.summary === 'object' ? response.summary : {};
    const result = String(summary.result || '').toUpperCase();
    return !!response.ok && (result === '' || result === 'SUCCESS');
  }
  if (action) return isDepStepPass(action, response);
  if (String(fail?.stage || '') === 'CI') return false;
  return !!response.ok;
}

async function routineRunCodexLifecycleForFailure(fail, options = {}) {
  if (!fail) return { ok: false, reason: 'no_failure' };
  const classified = routineClassifyFailureForHeal(fail);
  if (classified.signature === 'ci_cli_auth') {
    routineCodexAutoStart(`Stage=${fail.stage} | observer-auth blocked`);
    routineCodexAutoRecord('Pass 1: preview', 'fail', 'Skipped: gh auth is not configured in pilot runtime context.');
    routineCodexAutoFinish(false, 'Skipped auto-remediation: run gh auth login, then rerun routine.');
    return { ok: true, healthy: false, attempted: 0, skipped: true, reason: 'ci_cli_auth' };
  }
  if (routineCodexAutoRunning) return { ok: false, reason: 'busy' };
  routineCodexAutoRunning = true;
  const allowSecondPass = options.allowSecondPass !== false;
  routineCodexAutoStart(`Stage=${fail.stage}${allowSecondPass && String(fail.stage || '') === 'CI' ? ' | second-pass enabled' : ''}`);
  try {
    const runCycle = async (packet, passLabel) => {
      routineCodexAutoRecord(`${passLabel}: preview`, 'running');
      routineApplyCodexPacket(packet);
      const preview = await codexRun('preview');
      if (!preview || !preview.ok) {
        routineCodexAutoRecord(`${passLabel}: preview`, 'fail', preview?.error || 'preview failed');
        return { ok: false, phase: 'preview', preview };
      }
      routineCodexAutoRecord(`${passLabel}: preview`, 'pass');
      routineCodexAutoRecord(`${passLabel}: approve`, 'running');
      const approve = await codexRun('approve');
      if (!approve || !approve.ok) {
        routineCodexAutoRecord(`${passLabel}: approve`, 'fail', approve?.error || 'approve failed');
        return { ok: false, phase: 'approve', approve };
      }
      routineCodexAutoRecord(`${passLabel}: approve`, 'pass');
      routineCodexAutoRecord(`${passLabel}: execute`, 'running');
      const execute = await codexRun('execute');
      if (!execute || !execute.ok) {
        routineCodexAutoRecord(`${passLabel}: execute`, 'fail', execute?.error || 'execute failed');
        return { ok: false, phase: 'execute', execute };
      }
      routineCodexAutoRecord(`${passLabel}: execute`, 'pass');
      routineCodexAutoRecord(`${passLabel}: reconcile`, 'running');
      const reconcile = await codexRun('reconcile');
      if (!reconcile || !reconcile.ok) {
        routineCodexAutoRecord(`${passLabel}: reconcile`, 'fail', reconcile?.error || 'reconcile failed');
        return { ok: false, phase: 'reconcile', reconcile };
      }
      routineCodexAutoRecord(`${passLabel}: reconcile`, 'pass');
      const contractId = String(
        reconcile?.contract?.contract_id
        || execute?.contract?.contract_id
        || approve?.contract?.contract_id
        || preview?.contract?.contract_id
        || ''
      );
      let contract = reconcile?.contract || null;
      if (contractId) {
        const fetched = await codexFetchContract(contractId);
        if (fetched && fetched.contract) contract = fetched.contract;
      }
      const healthy = routineCodexIsHealthyForStage(fail, contract);
      routineCodexAutoRecord(`${passLabel}: verify`, healthy ? 'pass' : 'fail', healthy ? 'Verification passed.' : 'Verification still failing.');
      return { ok: true, healthy, contract, contractId };
    };

    const primary = await runCycle(routineBuildCodexEscalationPacket(fail), 'Pass 1');
    if (!primary.ok) {
      routineCodexAutoFinish(false, 'Primary Codex pass failed.');
      return { ok: false, phase: primary.phase, attempted: 1 };
    }
    if (primary.healthy) {
      routineCodexAutoFinish(true, 'Primary Codex pass verified success.');
      return { ok: true, healthy: true, contract: primary.contract, attempted: 1 };
    }
    if (!(allowSecondPass && String(fail.stage || '') === 'CI')) {
      routineCodexAutoFinish(false, 'Codex pass completed but verification still failing.');
      return { ok: true, healthy: false, contract: primary.contract, attempted: 1 };
    }
    const secondary = await runCycle(routineBuildCodexEscalationPacket(fail, { secondPass: true }), 'Pass 2');
    if (!secondary.ok) {
      routineCodexAutoFinish(false, 'Second Codex pass failed.');
      return { ok: false, phase: secondary.phase, attempted: 2 };
    }
    routineCodexAutoFinish(!!secondary.healthy, secondary.healthy ? 'Second Codex pass verified success.' : 'Second Codex pass still unresolved.');
    return { ok: true, healthy: !!secondary.healthy, contract: secondary.contract, attempted: 2 };
  } finally {
    routineCodexAutoRunning = false;
  }
}

async function routineEscalateFailureToCodex() {
  const options = (arguments && arguments[0] && typeof arguments[0] === 'object') ? arguments[0] : {};
  const fail = options.failureEntry || routineLatestFailureEntry();
  if (!fail) return { ok: false, reason: 'no_failure' };
  const packet = routineBuildCodexEscalationPacket(fail, options);
  if (!packet) return { ok: false, reason: 'no_packet' };
  routineApplyCodexPacket(packet);
  if (!options.autoRun) {
    try {
      await codexRun('preview');
    } catch (_) {
      // Keep packet populated even if preview call fails.
    }
    return { ok: true, prepared: true, automated: false };
  }

  const outcome = await routineRunCodexLifecycleForFailure(fail, {
    allowSecondPass: options.allowSecondPass !== false
  });
  const resumeFromStep = routineFailureStageToStep(fail);
  if (outcome.ok && outcome.healthy && options.autoResume && resumeFromStep && !dashRoutineRunning) {
    dashRoutineCanResume = true;
    const prior = String(dashRoutineWorkspaceState.healStatus || '');
    const line = `Codex auto-remediation: SUCCESS (${outcome.attempted || 1} pass${(outcome.attempted || 1) > 1 ? 'es' : ''}).`;
    dashRoutineWorkspaceState.healStatus = prior ? `${prior}\n${line}` : line;
    routineRenderActions();
    routineRenderWorkspace();
    routineCodexAutoRecord('Resume', 'running', `Resuming routine from ${resumeFromStep}.`);
    setTimeout(() => {
      dashRunPostCommitRoutine({ resumeFromStep, autoResumeDepth: 1 });
    }, 60);
  }
  return {
    ok: !!outcome.ok,
    prepared: true,
    automated: true,
    healthy: !!outcome.healthy,
    attempts: outcome.attempted || 0
  };
}

function routineRunAction(actionId) {
  if (actionId === 'open_agorg') {
    activatePanel('agorg');
    return;
  }
  if (actionId === 'open_multi') {
    activatePanel('multi');
    const groupInput = document.getElementById('multi-group');
    if (groupInput) groupInput.focus();
    return;
  }
  if (actionId === 'open_dashboard_gate') {
    activatePanel('dashboard');
    const btn = document.querySelector('button[onclick="dashRunGate()"]');
    if (btn) btn.focus();
    return;
  }
  if (actionId === 'open_push') {
    activatePanel('dashboard');
    const btn = document.querySelector('button[onclick="dashRunPush()"]');
    if (btn) btn.focus();
    return;
  }
  if (actionId === 'open_ci') {
    activatePanel('dashboard');
    if (dashRoutineOut) dashRoutineOut.focus();
  }
}

function parseMultiScopeFromRoutine() {
  const routineGroup = (document.getElementById('dash-routine-group')?.value || '').trim();
  const routineTags = tags(document.getElementById('dash-routine-tags')?.value || '');
  return {
    group: routineGroup || null,
    tags: routineTags
  };
}

function routineStageKey(stepName) {
  if (stepName === 'scope') return 'resolve';
  return String(stepName || '').toLowerCase();
}

function isDepStepPass(action, data) {
  const inner = (data && data.inner && typeof data.inner === 'object') ? data.inner : data;
  if (!inner) return false;
  if (['policy', 'hook-policy', 'drift', 'gate'].includes(action)) {
    const report = inner.report;
    if (!report || !Array.isArray(report.steps)) return !!inner.ok;
    const expected = action === 'hook-policy' ? 'Hook' : action.charAt(0).toUpperCase() + action.slice(1).replace('-', ' ');
    const step = report.steps.find((s) => String(s.step || '') === expected);
    if (!step || !step.result) return !!inner.ok;
    return String(step.result.status || '') === 'Pass';
  }
  return !!inner.ok;
}

function routineWriteSummary(lines) {
  if (!dashRoutineOut) return;
  const text = lines.join('\n');
  dashRoutineOut.textContent = text;
  // Guard against rare interrupted async flows leaving the button stuck in RUNNING...
  if (text.includes('Result: ')) {
    dashRoutineRunning = false;
    if (dashRoutineRunBtn) {
      dashRoutineRunBtn.disabled = false;
      dashRoutineRunBtn.textContent = 'Run Post-Commit Routine';
    }
  }
  focusResultsForA11y(dashRoutineOut);
  routineRenderWorkspace();
}

function routineAppendSummaryLine(line) {
  if (!dashRoutineOut) return;
  const prior = String(dashRoutineOut.textContent || '').trimEnd();
  dashRoutineOut.textContent = prior ? `${prior}\n${line}` : line;
  focusResultsForA11y(dashRoutineOut);
}

function routineReadCiContinuation() {
  try {
    const raw = window.localStorage.getItem(ROUTINE_CI_CONTINUATION_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return null;
    return parsed;
  } catch (_) {
    return null;
  }
}

function routineWriteCiContinuation(payload) {
  try {
    window.localStorage.setItem(ROUTINE_CI_CONTINUATION_KEY, JSON.stringify(payload || {}));
  } catch (_) {}
}

function routineClearCiContinuation() {
  try {
    window.localStorage.removeItem(ROUTINE_CI_CONTINUATION_KEY);
  } catch (_) {}
}

function routineStepIsTerminalPass(state) {
  const s = String(state || '').toLowerCase();
  return s === 'pass' || s === 'passed' || s === 'success' || s === 'succeeded' || s === 'completed' || s === 'ok';
}

function routineMaybeAutoContinueFromCi(summary = null) {
  if (!summary || typeof summary !== 'object') return;
  if (dashRoutineRunning || routineCiAutoResumeBusy) return;
  const pending = routineReadCiContinuation();
  if (!pending || typeof pending !== 'object') return;
  const nextStep = String(pending.next_step || '').toLowerCase();
  if (!nextStep) return;
  const summaryBranch = String(summary.branch || '').trim();
  if (pending.branch && summaryBranch && pending.branch !== summaryBranch) return;
  const summaryRunId = String(summary.ci_run_id || summary.run_id || '').trim();
  if (pending.ci_run_id && summaryRunId && pending.ci_run_id !== summaryRunId) return;
  if (!routineStepIsTerminalPass(summary.overall_state)) return;

  routineCiAutoResumeBusy = true;
  routineClearCiContinuation();
  const stageLabel = ROUTINE_STAGE_LABELS[routineStageKey(nextStep)] || nextStep;
  routineRecord(
    'CI',
    'pass',
    `Auto-resume verified CI completion and will continue at ${stageLabel}.`,
    JSON.stringify({ pending, summary }, null, 2),
    '',
    '',
    Date.now()
  );
  routineAppendSummaryLine(`Auto-Resume: CI completed; resuming from ${nextStep}.`);
  setTimeout(() => {
    dashRunPostCommitRoutine({ resumeFromStep: nextStep, resumeReason: 'ci-continuation' })
      .catch(() => {})
      .finally(() => { routineCiAutoResumeBusy = false; });
  }, 120);
}

async function dashRunPostCommitRoutine(options = {}) {
  if (dashRoutineRunning) return;
  dashRoutineRunning = true;
  dashRoutineAutoHealAttempted = false;
  const requestedResumeFrom = String(options?.resumeFromStep || '').toLowerCase().trim();
  const autoResumeDepth = Number(options?.autoResumeDepth || 0);
  const resumeReason = String(options?.resumeReason || '').trim();
  const previousWorkspaceState = dashRoutineWorkspaceState || {};
  let queuedAutoResumeStep = '';
  if (dashRoutineRunBtn) {
    dashRoutineRunBtn.disabled = true;
    dashRoutineRunBtn.textContent = 'Running...';
  }
  routineSyncPushControls();
  routineResetChips();
  routineResetTimeline();
  dashRoutineWorkspaceState = {
    loaded: null,
    resolveSnapshot: null,
    active: requestedResumeFrom ? (previousWorkspaceState.active || null) : null,
    scope: requestedResumeFrom ? (previousWorkspaceState.scope || parseMultiScopeFromRoutine()) : parseMultiScopeFromRoutine(),
    stats: requestedResumeFrom ? (previousWorkspaceState.stats || null) : null,
    multiSnapshot: requestedResumeFrom ? (previousWorkspaceState.multiSnapshot || null) : null,
    gateDetail: requestedResumeFrom ? (previousWorkspaceState.gateDetail || null) : null,
    pushDetail: requestedResumeFrom ? (previousWorkspaceState.pushDetail || null) : null,
    pushNoop: requestedResumeFrom ? !!previousWorkspaceState.pushNoop : false,
    ciDetail: requestedResumeFrom ? (previousWorkspaceState.ciDetail || null) : null,
    ciCatalog: previousWorkspaceState.ciCatalog || null,
    ciSelectedWorkflowKey: previousWorkspaceState.ciSelectedWorkflowKey || '',
    ciInFlight: false,
    healStatus: '',
    evidenceDetail: null,
    lastResult: 'running',
    lastRunAt: new Date().toISOString(),
    failure: null
  };
  routineSetLastResult('running', 'Last Result: running');
  routineSelectStage('resolve');
  if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
  routineAnnounceStatus('Post-Commit Routine started.');
  const lines = [];
  lines.push('Post-Commit Routine');
  lines.push(`Started: ${new Date().toISOString()}`);
  if (resumeReason) lines.push(`Resume Reason: ${resumeReason}`);
  try {
    const loaded = await loadRoutinePolicyProfile();
    routineApplyPolicyProfile(loaded);
    await routineLoadResolveSnapshot();
    const profile = loaded.profile;
    const profileDiff = routineProfileDiff(profile);
    lines.push(`Profile: ${loaded.source} v${loaded.version} [${loaded.status}]`);
    lines.push(`Step Order: ${profile.step_order.join(' -> ')}`);
    if (profileDiff.length) {
      lines.push(`Profile Diff: ${profileDiff.join(', ')}`);
    }

    const stepsToRun = profile.step_order.slice();
    routineClearCiContinuation();
    const includeSet = new Set(stepsToRun);
    const resumeIndex = requestedResumeFrom ? stepsToRun.indexOf(requestedResumeFrom) : -1;
    const ciIndex = stepsToRun.indexOf('ci');
    const nextStepAfterCi = ciIndex >= 0 && ciIndex + 1 < stepsToRun.length ? stepsToRun[ciIndex + 1] : '';
    const effectiveSteps = resumeIndex >= 0 ? stepsToRun.slice(resumeIndex) : stepsToRun;
    if (requestedResumeFrom) {
      if (resumeIndex >= 0) {
        lines.push(`Resume: starting from ${requestedResumeFrom}; skipped steps: ${stepsToRun.slice(0, resumeIndex).join(', ') || 'none'}`);
      } else {
        lines.push(`Resume: requested step "${requestedResumeFrom}" not in profile; running full routine.`);
      }
    }
    if (!includeSet.has('scope')) routineSetChip(dashRoutineScopeChip, 'Scope: off', 'neutral');
    if (!includeSet.has('multi')) routineSetChip(dashRoutineMultiChip, 'Multi: off', 'neutral');
    if (!includeSet.has('gates')) routineSetChip(dashRoutineGatesChip, 'Gates: off', 'neutral');
    if (!includeSet.has('push')) routineSetChip(dashRoutinePushChip, 'Push: off', 'neutral');
    if (!includeSet.has('evidence')) routineSetChip(dashRoutineEvidenceChip, 'Evidence: off', 'neutral');
    if (!includeSet.has('scope')) routineSetStageState('resolve', 'Off', 'neutral');
    if (!includeSet.has('multi')) routineSetStageState('multi', 'Off', 'neutral');
    if (!includeSet.has('gates')) routineSetStageState('gates', 'Off', 'neutral');
    if (!includeSet.has('push')) routineSetStageState('push', 'Off', 'neutral');
    if (!includeSet.has('ci')) routineSetStageState('ci', 'Off', 'neutral');
    if (!includeSet.has('evidence')) routineSetStageState('evidence', 'Off', 'neutral');
    routineSetStageState('plan', 'Ready', 'ok');
    routineRefreshPlanPreview();
    routineRenderWorkspace();

    const context = {
      active: dashRoutineWorkspaceState.active || null,
      scope: dashRoutineWorkspaceState.scope || null,
      stats: dashRoutineWorkspaceState.stats || null
    };
    let failed = false;

    for (const stepName of effectiveSteps) {
      const stageStart = Date.now();
      const stageKey = routineStageKey(stepName);
      routineSelectStage(stageKey);
      if (stepName === 'scope') {
        routineSetChip(dashRoutineScopeChip, 'Scope: running', 'warn');
        routineSetStageState('resolve', 'Running', 'warn');
        const active = await fetchJsonSafe('/api/agorg/active');
        if (!active || !active.ok || !active.active || !active.active.id) {
          routineSetChip(dashRoutineScopeChip, 'Scope: fail', 'fail');
          routineSetStageState('resolve', 'Blocked', 'fail');
          lines.push('Scope: FAIL (no active AGOrg)');
          lines.push('Remediation: open AGOrg tab and select active scope.');
          routineRecord('Scope', 'fail', 'No active AGOrg scope selected.', JSON.stringify(active || {}, null, 2), 'Open AGOrg tab and select active scope.', 'open_agorg', stageStart);
          routineAnnounceAlert('Routine blocked: no active AGOrg scope selected.');
          if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        const scope = parseMultiScopeFromRoutine();
        if (!scope.group && (!Array.isArray(scope.tags) || scope.tags.length === 0)) {
          routineSetChip(dashRoutineScopeChip, 'Scope: fail', 'fail');
          routineSetStageState('resolve', 'Blocked', 'fail');
          lines.push('Scope: FAIL (Group/Tags required)');
          lines.push('Remediation: set Group or Tags in the Post-Commit Routine card.');
          routineRecord('Scope', 'fail', 'Group/Tags selector missing.', JSON.stringify(scope, null, 2), 'Set Group or Tags in routine selector.', 'open_multi', stageStart);
          routineAnnounceAlert('Routine blocked: group or tags selector missing.');
          if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        const multiGroupInput = document.getElementById('multi-group');
        const multiTagsInput = document.getElementById('multi-tags');
        if (multiGroupInput) multiGroupInput.value = scope.group || '';
        if (multiTagsInput) multiTagsInput.value = (scope.tags || []).join(',');
        const stats = await multiSelectorStats();
        if (!stats.ok || stats.filtered <= 0) {
          routineSetChip(dashRoutineScopeChip, 'Scope: fail', 'fail');
          routineSetStageState('resolve', 'Blocked', 'fail');
          lines.push(`Scope: FAIL (${stats.error || 'no repos matched selector'})`);
          lines.push('Remediation: register AGO(s), then adjust Group/Tags selector.');
          routineRecord('Scope', 'fail', 'No repos matched scope selector.', JSON.stringify(stats, null, 2), 'Adjust Group/Tags to match registered AGOs.', 'open_multi', stageStart);
          routineAnnounceAlert('Routine blocked: no repos matched the current selector.');
          if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        context.active = active.active;
        context.scope = scope;
        context.stats = stats;
        dashRoutineWorkspaceState.active = active.active;
        dashRoutineWorkspaceState.scope = scope;
        dashRoutineWorkspaceState.stats = stats;
        routineSetChip(dashRoutineScopeChip, `Scope: pass (${stats.filtered})`, 'ok');
        routineSetStageState('resolve', 'Ready', 'ok');
        routineRecord('Scope', 'pass', `Matched ${stats.filtered}/${stats.inScope} repos in active AGOrg.`, JSON.stringify({ scope, stats, agorg: active.active }, null, 2), '', '', stageStart);
        lines.push(`Scope: PASS (AGOrg=${active.active.name}, matched=${stats.filtered}/${stats.inScope})`);
        routineRefreshPlanPreview();
        continue;
      }

      if (stepName === 'multi') {
        routineSetChip(dashRoutineMultiChip, 'Multi: running', 'warn');
        routineSetStageState('multi', 'Running', 'warn');
        if (!context.scope) {
          routineSetChip(dashRoutineMultiChip, 'Multi: fail', 'fail');
          routineSetStageState('multi', 'Blocked', 'fail');
          lines.push('Multi: FAIL (scope stage not satisfied)');
          routineRecord('Multi', 'fail', 'Scope step must pass before multi flow.', '', 'Run Scope stage first or keep default step order.', 'open_multi', stageStart);
          routineAnnounceAlert('Routine blocked: Multi cannot run before Resolve passes.');
          if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        const multiSteps = [
          { cmd: 'pilot.multi.list', payload: context.scope },
          { cmd: 'pilot.multi.status', payload: context.scope },
          { cmd: 'pilot.multi.order', payload: context.scope },
          { cmd: 'pilot.multi.dag', payload: { ...context.scope, dry_run: true } },
          { cmd: 'pilot.multi.prs.create', payload: { ...context.scope, dry_run: true, head_branch: 'dev', base_branch: 'main' } }
        ];
        let multiOk = true;
        for (const step of multiSteps) {
          const result = await runMultiCommand(step.cmd, step.payload, { outputEl: multiActionOut });
          if (!result || !result.ok) {
            multiOk = false;
            lines.push(`Multi: FAIL (${step.cmd})`);
            lines.push(`Reason: ${result?.error || result?.inner?.error || 'unknown'}`);
            routineRecord('Multi', 'fail', `Step failed: ${step.cmd}`, JSON.stringify(result || {}, null, 2), 'Open Multi tab and verify selector + registry.', 'open_multi', stageStart);
            break;
          }
        }
        if (!multiOk) {
          routineSetChip(dashRoutineMultiChip, 'Multi: fail', 'fail');
          routineSetStageState('multi', 'Fail', 'fail');
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        let snap = multiActionLast && multiActionLast.data ? multiActionLast.data.multi_snapshot : null;
        if (!snap) {
          const snapshotRes = await routineLoadMultiSnapshot();
          if (snapshotRes && snapshotRes.ok) snap = snapshotRes;
        }
        const repoCount = Array.isArray(snap?.repos) ? snap.repos.length : 0;
        dashRoutineWorkspaceState.multiSnapshot = snap;
        routineSetChip(dashRoutineMultiChip, `Multi: pass (${repoCount})`, 'ok');
        routineSetStageState('multi', `Ready (${repoCount})`, 'ok');
        routineRecord('Multi', 'pass', `Multi preview flow completed for ${repoCount} repos.`, JSON.stringify(snap || {}, null, 2), '', '', stageStart);
        lines.push(`Multi: PASS (repos=${repoCount})`);
        continue;
      }

      if (stepName === 'gates') {
        routineSetChip(dashRoutineGatesChip, 'Gates: running', 'warn');
        routineSetStageState('gates', 'Running', 'warn');
        const gateActions = ['policy', 'hook-policy', 'drift', 'gate'];
        const gateLabelMap = {
          'policy': 'Policy',
          'hook-policy': 'Hook Policy',
          'drift': 'Drift',
          'gate': 'Gate'
        };
        let gatesOk = true;
        for (let i = 0; i < gateActions.length; i += 1) {
          const action = gateActions[i];
          const stepNo = i + 1;
          const gateLabel = gateLabelMap[action] || action;
          routineSetChip(dashRoutineGatesChip, `Gates: ${stepNo}/${gateActions.length} (${gateLabel})`, 'warn');
          routineSetStageState('gates', `${stepNo}/${gateActions.length}`, 'warn');
          routineRecord('Gates', 'running', `Running ${gateLabel} (${stepNo}/${gateActions.length})...`, '', '', '', stageStart);
          const data = await depRun(action);
          if (!isDepStepPass(action, data)) {
            gatesOk = false;
            dashRoutineWorkspaceState.gateDetail = depEnvelopeInner(data);
            routineSetChip(dashRoutineGatesChip, `Gates: fail (${gateLabel})`, 'fail');
            routineSetStageState('gates', 'Blocked', 'fail');
            lines.push(`Gates: FAIL at ${action}`);
            lines.push('Remediation: inspect Dashboard gate output and retry.');
            routineRecord('Gates', 'fail', `${gateLabel} failed (${stepNo}/${gateActions.length}).`, JSON.stringify(data || {}, null, 2), 'Open Dashboard and run gate controls individually.', 'open_dashboard_gate', stageStart);
            routineAnnounceAlert(`Routine blocked: ${gateLabel} failed.`);
            if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
            break;
          }
          dashRoutineWorkspaceState.gateDetail = depEnvelopeInner(data);
          routineRecord('Gates', 'pass', `${gateLabel} passed (${stepNo}/${gateActions.length}).`, '', '', '', stageStart);
        }
        if (!gatesOk) {
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        routineSetChip(dashRoutineGatesChip, 'Gates: pass', 'ok');
        routineSetStageState('gates', 'Ready', 'ok');
        routineRecord('Gates', 'pass', 'Policy/Hook/Drift/Gate all passed.', '', '', '', stageStart);
        lines.push('Gates: PASS');
        continue;
      }

      if (stepName === 'push') {
        const allowPush = !!document.getElementById('dash-routine-allow-push')?.checked;
        if (!allowPush) {
          routineSetChip(dashRoutinePushChip, 'Push: blocked', 'warn');
          routineSetStageState('push', 'Blocked', 'fail');
          routineRecord('Push', 'warn', 'Push step skipped by toggle.', '', '', '', stageStart);
          lines.push('Push: BLOCKED (allow push step is disabled)');
          continue;
        }
        routineSetChip(dashRoutinePushChip, 'Push: running', 'warn');
        routineSetStageState('push', 'Running', 'warn');
        routineRecord('Push', 'running', 'Executing push-safe pipeline...', '', '', '', stageStart);
        const { branch, remote } = routineReadBranchRemote();
        let pushRes;
        try {
          pushRes = await depRunWithTimeout('push', { branch, remote }, 15 * 60 * 1000);
        } catch (err) {
          pushRes = {
            ok: false,
            error: err?.message || 'push request timed out',
            inner: {
              ok: false,
              action: 'push',
              error: err?.message || 'push request timed out'
            }
          };
        }
        dashRoutineWorkspaceState.pushDetail = depEnvelopeInner(pushRes);
        if (!isDepStepPass('push', pushRes)) {
          const pushError = String(pushRes?.error || pushRes?.inner?.error || 'unknown');
          const pushTimedOut = pushError.toLowerCase().includes('timed out');
          routineSetChip(dashRoutinePushChip, 'Push: fail', 'fail');
          routineSetStageState('push', pushTimedOut ? 'Timed out' : 'Fail', 'fail');
          lines.push(`Push: FAIL (${pushError})`);
          routineRecord(
            'Push',
            'fail',
            pushTimedOut ? 'Push timed out.' : 'Push safe failed.',
            JSON.stringify(pushRes || {}, null, 2),
            pushTimedOut ? 'Push timed out. Check credentials/network and resume from Push.' : 'Open push controls and inspect gate/push diagnostics.',
            'open_push',
            stageStart
          );
          routineAnnounceAlert(pushTimedOut ? 'Routine blocked: push timed out.' : 'Routine blocked: push failed.');
          if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        dashRoutineWorkspaceState.pushNoop = routinePushLikelyNoop(pushRes);
        routineSetChip(dashRoutinePushChip, 'Push: pass', 'ok');
        routineSetStageState('push', 'Ready', 'ok');
        routineRecord('Push', 'pass', 'Push safe passed.', JSON.stringify(pushRes || {}, null, 2), '', '', stageStart);
        lines.push(`Push: PASS${dashRoutineWorkspaceState.pushNoop ? ' (no new remote update)' : ''}`);
        continue;
      }

      if (stepName === 'ci') {
        if (dashRoutineWorkspaceState.pushNoop) {
          routineSetStageState('ci', 'Triggering', 'warn');
          routineRecord('CI', 'running', 'Push had no remote update; triggering CI workflow_dispatch fallback.', '', '', '', stageStart);
          const ciBranch = routineReadBranchRemote().branch;
          const triggerRes = await depRun('ci-trigger', { branch: ciBranch });
          if (!isDepStepPass('ci-trigger', triggerRes)) {
            dashRoutineWorkspaceState.ciInFlight = false;
            routineClearCiContinuation();
            routineSetStageState('ci', 'Blocked', 'fail');
            routineSetCiJobChips({});
            routineRenderCiObservatory({});
            routineRecord('CI', 'fail', 'CI trigger fallback failed.', JSON.stringify(triggerRes || {}, null, 2), 'Enable workflow_dispatch in ci.yml and verify gh auth/permissions.', 'open_ci', stageStart);
            lines.push('CI: FAIL (push was up-to-date and CI fallback trigger failed)');
            routineAnnounceAlert('Routine blocked: CI trigger fallback failed.');
            failed = true;
            if (profile.stop_on_fail) break;
            continue;
          }
          lines.push('CI: TRIGGERED (workflow_dispatch fallback after no-op push)');
        }
        await routineLoadCiCatalog();
        dashRoutineWorkspaceState.ciInFlight = true;
        if (nextStepAfterCi) {
          routineWriteCiContinuation({
            branch: routineReadBranchRemote().branch,
            ci_run_id: '',
            next_step: nextStepAfterCi,
            recorded_at: new Date().toISOString(),
            reason: 'ci-stage-inflight'
          });
        }
        routineSetStageState('ci', 'Running', 'warn');
        routineSetCiJobChips({
          docs_state: 'running',
          rust_state: 'running',
          ui_smoke_state: 'running',
          packaging_parity_state: 'running'
        });
        routineRenderCiObservatory({
          docs_state: 'running',
          rust_state: 'running',
          ui_smoke_state: 'running',
          packaging_parity_state: 'running'
        });
        const ciBranch = routineReadBranchRemote().branch;
        let ciPollDone = false;
        let ciPollBusy = false;
        const pullCiStatus = async () => {
          if (ciPollDone || ciPollBusy) return;
          ciPollBusy = true;
          try {
            const statusRes = await depRun('ci-status', { branch: ciBranch });
            const statusInner = depEnvelopeInner(statusRes);
            const statusSummary = statusInner && statusInner.summary && typeof statusInner.summary === 'object'
              ? statusInner.summary
              : null;
            if (statusSummary) {
              const mergedSummary = routineCiMergeStickySummary(dashRoutineWorkspaceState.ciDetail?.summary || null, statusSummary);
              routineSetCiJobChips(mergedSummary);
              dashRoutineWorkspaceState.ciDetail = statusInner
                ? { ...statusInner, summary: mergedSummary }
                : { ...(statusRes || {}), summary: mergedSummary };
              routineRenderCiObservatory(mergedSummary, dashRoutineWorkspaceState.ciDetail);
            }
          } finally {
            ciPollBusy = false;
          }
        };
        await pullCiStatus();
        const ciPollTimer = setInterval(pullCiStatus, 7000);
        const ci = await depRun('ci-watch', {
          branch: ciBranch,
          ci_timeout_sec: 1800
        });
        ciPollDone = true;
        clearInterval(ciPollTimer);
        const ciInner = depEnvelopeInner(ci);
        dashRoutineWorkspaceState.ciDetail = ciInner || ci;
        if (!ciInner || !ciInner.ok) {
          const ciAuthFailure = routineIsCiAuthErrorPayload(ciInner || ci || null);
          const likelyCause = String(ciInner?.summary?.likely_cause || '').toLowerCase();
          const noFreshRun = likelyCause === 'no_fresh_run_detected';
          dashRoutineWorkspaceState.ciInFlight = false;
          routineClearCiContinuation();
          routineSetStageState('ci', 'Fail', 'fail');
          lines.push(ciAuthFailure
            ? 'CI: BLOCKED (local gh auth unavailable; GitHub workflow may still be healthy)'
            : `CI: FAIL (${ciInner?.error || ci?.error || 'watch failed'})`);
          routineRecord(
            'CI',
            'fail',
            ciAuthFailure
              ? 'CI observer authentication failed in local pilot runtime (gh auth not configured).'
              : (noFreshRun ? 'No fresh CI run was detected for this routine window.' : 'GitHub Actions watch failed.'),
            JSON.stringify(ciInner || ci || {}, null, 2),
            ciAuthFailure
              ? 'Run gh auth login in the same shell/runtime context as pilot serve, then rerun CI/watch.'
              : noFreshRun
              ? 'No new workflow run was triggered. Confirm a new commit was pushed, then retry Push/CI.'
              : 'Inspect run URL and failing jobs in CI summary.',
            'open_ci',
            stageStart
          );
          routineAnnounceAlert(
            ciAuthFailure
              ? 'Routine blocked: local gh auth is missing for CI observation.'
              : (noFreshRun ? 'Routine blocked: no fresh CI run detected.' : 'Routine blocked: CI watch failed.')
          );
          if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        const ciWatchSummary = (ciInner.summary && typeof ciInner.summary === 'object') ? ciInner.summary : {};
        const ciStatusAfterWatch = await depRun('ci-status', { branch: ciBranch });
        const ciStatusInner = depEnvelopeInner(ciStatusAfterWatch);
        const ciStatusSummary = (ciStatusInner && ciStatusInner.summary && typeof ciStatusInner.summary === 'object')
          ? ciStatusInner.summary
          : null;
        const ciSummaryRaw = ciStatusSummary
          ? {
            ...ciStatusSummary,
            run_id: ciWatchSummary.run_id || ciStatusSummary.run_id || ciStatusSummary.ci_run_id || '',
            run_url: ciWatchSummary.run_url || ciStatusSummary.run_url || '',
            workflow: ciWatchSummary.workflow || ciStatusSummary.workflow || 'workflow',
            status: ciWatchSummary.status || ciStatusSummary.status || '',
            conclusion: ciWatchSummary.conclusion || ciStatusSummary.conclusion || ''
          }
          : ciWatchSummary;
        const ciSummary = routineCiMergeStickySummary(dashRoutineWorkspaceState.ciDetail?.summary || null, ciSummaryRaw);
        if (nextStepAfterCi) {
          routineWriteCiContinuation({
            branch: ciSummary.branch || routineReadBranchRemote().branch,
            ci_run_id: ciSummary.run_id || ciSummary.ci_run_id || '',
            next_step: nextStepAfterCi,
            recorded_at: new Date().toISOString(),
            reason: 'ci-stage-completed'
          });
        }
        dashRoutineWorkspaceState.ciDetail = ciStatusSummary
          ? { ...(ciStatusInner || {}), watch_summary: ciWatchSummary, summary: ciSummary }
          : (ciInner || ci);
        dashRoutineWorkspaceState.ciInFlight = false;
        routineSetCiJobChips(ciSummary);
        const runUrl = ciSummary.run_url || '';
        routineRenderCiObservatory(ciSummary, ciInner);
        routineSetStageState('ci', runUrl ? 'Observed' : 'Ready', 'ok');
        routineRecord('CI', 'pass', 'GitHub Actions run completed successfully.', JSON.stringify(ciInner, null, 2), '', '', stageStart);
        lines.push(`CI: PASS (${ciSummary.workflow || 'workflow'})`);
        if (runUrl) lines.push(`CI Run: ${runUrl}`);
        if (nextStepAfterCi) lines.push(`CI Continuation: armed for ${nextStepAfterCi}`);
        continue;
      }

      if (stepName === 'evidence') {
        routineClearCiContinuation();
        const exportEvidence = !!document.getElementById('dash-routine-export-evidence')?.checked;
        if (!exportEvidence) {
          routineSetChip(dashRoutineEvidenceChip, 'Evidence: skipped', 'warn');
          routineSetStageState('evidence', 'Skipped', 'warn');
          routineRecord('Evidence', 'warn', 'Evidence export skipped by toggle.', '', '', '', stageStart);
          lines.push('Evidence: SKIPPED (toggle disabled)');
          continue;
        }
        routineSetChip(dashRoutineEvidenceChip, 'Evidence: running', 'warn');
        routineSetStageState('evidence', 'Running', 'warn');
        const ev = await dashExportEvidence();
        dashRoutineWorkspaceState.evidenceDetail = ev;
        if (!ev || !ev.ok) {
          routineSetChip(dashRoutineEvidenceChip, 'Evidence: fail', 'fail');
          routineSetStageState('evidence', 'Fail', 'fail');
          lines.push(`Evidence: FAIL (${ev?.error || 'export failed'})`);
          routineRecord('Evidence', 'fail', 'Evidence export failed.', JSON.stringify(ev || {}, null, 2), 'Use Export Evidence button directly and inspect output.', 'open_dashboard_gate', stageStart);
          routineAnnounceAlert('Routine blocked: evidence export failed.');
          if (dashRoutineStagePanel) dashRoutineStagePanel.focus();
          failed = true;
          if (profile.stop_on_fail) break;
          continue;
        }
        routineSetChip(dashRoutineEvidenceChip, 'Evidence: pass', 'ok');
        routineSetStageState('evidence', 'Ready', 'ok');
        routineRecord('Evidence', 'pass', `Evidence exported: ${ev.path || 'artifact generated'}.`, JSON.stringify(ev, null, 2), '', '', stageStart);
        lines.push(`Evidence: PASS (${ev.path || 'artifact exported'})`);
      }
    }

    routineSelectStage('reconcile');
    routineSetStageState('reconcile', failed ? 'Failed' : 'Success', failed ? 'fail' : 'ok');
    if (failed) {
      lines.push('Result: FAILED');
      dashRoutineWorkspaceState.lastResult = 'failed';
      routineSetLastResult('failed', 'Last Result: failed');
      routineAnnounceAlert('Post-Commit Routine failed.');
      if (routineAutoHealEnabled() && !dashRoutineAutoHealAttempted) {
        lines.push('Auto-Heal: attempting known-safe remediation.');
        const healResult = await routineAutoHealAndRetry({ fromAuto: true });
        if (healResult?.ok) {
          lines.push('Auto-Heal: SUCCESS (verification passed).');
          failed = false;
          dashRoutineWorkspaceState.lastResult = 'recovered';
          routineSetLastResult('success', 'Last Result: recovered');
          routineSetStageState('reconcile', 'Recovered', 'ok');
          routineAnnounceStatus('Post-Commit Routine recovered via auto-heal.');
          const failureForResume = dashRoutineWorkspaceState.failure || routineLatestFailureEntry();
          const autoResumeStep = routineFailureStageToStep(failureForResume);
          if (autoResumeStep && autoResumeDepth < 1) {
            queuedAutoResumeStep = autoResumeStep;
            lines.push(`Auto-Resume: queued from ${autoResumeStep}.`);
          }
        } else if (healResult?.escalated) {
          lines.push('Auto-Heal: failed or not applicable. Escalated to Codex.');
        } else {
          lines.push('Auto-Heal: no safe playbook matched.');
        }
      }
      if (failed && routineAutoCodexEnabled()) {
        const failureForCodex = dashRoutineWorkspaceState.failure || routineLatestFailureEntry();
        const codexClassified = routineClassifyFailureForHeal(failureForCodex);
        if (codexClassified.signature === 'ci_cli_auth') {
          lines.push('Codex Auto: skipped (ci_cli_auth requires local gh auth login first).');
        } else {
          lines.push('Codex Auto: escalating and running guided remediation.');
          const codexResult = await routineEscalateFailureToCodex({ autoRun: true, autoResume: true });
          if (codexResult?.ok && codexResult?.healthy) {
            lines.push(`Codex Auto: SUCCESS (${codexResult.attempts || 1} pass${(codexResult.attempts || 1) > 1 ? 'es' : ''}).`);
            failed = false;
            dashRoutineWorkspaceState.lastResult = 'recovered';
            routineSetLastResult('success', 'Last Result: recovered');
            routineSetStageState('reconcile', 'Recovered', 'ok');
            routineAnnounceStatus('Post-Commit Routine recovered via Codex automation.');
          } else {
            lines.push('Codex Auto: unresolved; review Codex panel output.');
          }
        }
      }
      if (!profile.stop_on_fail) {
        lines.push('Mode: Continue-on-fail (profile stop_on_fail=false)');
      }
    } else {
      lines.push('Result: SUCCESS');
      dashRoutineWorkspaceState.lastResult = 'success';
      routineSetLastResult('success', 'Last Result: success');
      routineRecordFailure(null);
      routineClearCiContinuation();
      routineAnnounceStatus('Post-Commit Routine completed successfully.');
    }
    const finalFailure = failed ? (dashRoutineWorkspaceState.failure || routineLatestFailureEntry()) : null;
    const finalClassified = finalFailure ? routineClassifyFailureForHeal(finalFailure) : null;
    const reconcileFailSummary = finalClassified?.signature === 'ci_cli_auth'
      ? 'Routine blocked by local gh auth for CI observation; GitHub workflow state may differ.'
      : 'Routine ended with blocking failure.';
    routineRecord('Reconcile', failed ? 'fail' : 'pass', failed ? reconcileFailSummary : 'Routine completed successfully.', '', dashRoutineWorkspaceState.failure?.remediation || '', '', 0);
    dashRoutineWorkspaceState.lastRunAt = new Date().toISOString();
    routineWriteSummary(lines);
  } finally {
    dashRoutineRunning = false;
    if (dashRoutineRunBtn) {
      dashRoutineRunBtn.disabled = false;
      dashRoutineRunBtn.textContent = 'Run Post-Commit Routine';
    }
    routineRenderWorkspace();
    if (dashRoutineOut) dashRoutineOut.focus();
    if (!queuedAutoResumeStep) routineClearCiContinuation();
    if (queuedAutoResumeStep) {
      setTimeout(() => {
        dashRunPostCommitRoutine({ resumeFromStep: queuedAutoResumeStep, autoResumeDepth: autoResumeDepth + 1 });
      }, 60);
    }
  }
}

function dashReleaseSetChip(chip, label, level) {
  if (!chip) return;
  chip.textContent = label;
  chip.className = `chip ${level}`;
}

function dashReleaseResetChips() {
  dashReleaseSetChip(dashReleaseReadinessChip, 'Readiness: idle', 'neutral');
  dashReleaseSetChip(dashReleaseCompatChip, 'Compat: idle', 'neutral');
  dashReleaseSetChip(dashReleaseMigrationChip, 'Migration: idle', 'neutral');
  dashReleaseSetChip(dashReleasePushChip, 'Publish: idle', 'neutral');
  dashReleaseSetChip(dashReleaseBundleChip, 'Bundle: idle', 'neutral');
  dashReleaseSetChip(dashReleaseVerifyChip, 'Verify: idle', 'neutral');
  dashReleaseSetChip(dashReleaseScoreChip, 'Score: -', 'neutral');
}

function dashReleaseActionToChip(action) {
  if (action === 'release-readiness') return dashReleaseReadinessChip;
  if (action === 'release-compat-matrix') return dashReleaseCompatChip;
  if (action === 'release-migration-smoke') return dashReleaseMigrationChip;
  if (action === 'push') return dashReleasePushChip;
  if (action === 'release-collect-evidence') return dashReleaseBundleChip;
  if (action === 'release-verify-bundle') return dashReleaseVerifyChip;
  return null;
}

function dashReleaseActionLabel(action) {
  if (action === 'release-readiness') return 'Readiness';
  if (action === 'release-compat-matrix') return 'Compat';
  if (action === 'release-migration-smoke') return 'Migration';
  if (action === 'push') return 'Publish';
  if (action === 'release-collect-evidence') return 'Bundle';
  if (action === 'release-verify-bundle') return 'Verify';
  return action;
}

function depEnvelopeInner(data) {
  return (data && data.inner && typeof data.inner === 'object') ? data.inner : data;
}

function dashReleaseWrite(lines) {
  if (!dashReleaseOut) return;
  dashReleaseOut.textContent = lines.join('\n');
  focusResultsForA11y(dashReleaseOut);
}

async function dashReleaseRunStep(action) {
  const line = [`Release Step: ${dashReleaseActionLabel(action)}`];
  const chip = dashReleaseActionToChip(action);
  if (chip) dashReleaseSetChip(chip, `${dashReleaseActionLabel(action)}: running`, 'warn');
  const label = (document.getElementById('dash-release-label')?.value || 'alpha-local').trim();
  const bundlePath = (document.getElementById('dash-release-bundle-path')?.value || '').trim();
  const opts = {};
  if (action === 'release-collect-evidence') opts.label = label;
  if (action === 'release-verify-bundle') opts.bundle_path = bundlePath;
  const data = await depRun(action, opts);
  const inner = depEnvelopeInner(data);
  const ok = !!inner?.ok;
  if (chip) dashReleaseSetChip(chip, `${dashReleaseActionLabel(action)}: ${ok ? 'pass' : 'fail'}`, ok ? 'ok' : 'fail');
  if (action === 'release-collect-evidence' && inner?.artifact_path) {
    const bundleInput = document.getElementById('dash-release-bundle-path');
    if (bundleInput) bundleInput.value = inner.artifact_path;
  }
  line.push(`Status: ${ok ? 'PASS' : 'FAIL'}`);
  line.push(`Summary: ${inner?.summary?.result || inner?.summary || inner?.error || 'n/a'}`);
  if (inner?.artifact_path) line.push(`Artifact: ${inner.artifact_path}`);
  dashReleaseWrite(line);
}

async function dashRunReleaseRoutine() {
  dashReleaseResetChips();
  if (dashReleaseRunBtn) {
    dashReleaseRunBtn.disabled = true;
    dashReleaseRunBtn.textContent = 'Running...';
  }
  try {
  const lines = [];
  lines.push('Release Routine');
  lines.push(`Started: ${new Date().toISOString()}`);
  const allowPush = !!document.getElementById('dash-release-allow-push')?.checked;
  const label = (document.getElementById('dash-release-label')?.value || 'alpha-local').trim();
  const steps = [
    { action: 'release-readiness', required: true },
    { action: 'release-compat-matrix', required: true },
    { action: 'release-migration-smoke', required: true },
    { action: 'prepush-gate', required: true, chip: dashReleasePushChip, label: 'Publish Gate' },
    { action: 'push', required: allowPush, chip: dashReleasePushChip, label: 'Publish Push' },
    { action: 'release-collect-evidence', required: true },
    { action: 'release-verify-bundle', required: true }
  ];
  let passed = 0;
  let requiredTotal = steps.filter((s) => s.required).length;
  let failed = false;

  for (const step of steps) {
    const action = step.action;
    const labelText = step.label || dashReleaseActionLabel(action);
    const chip = step.chip || dashReleaseActionToChip(action);
    if (!step.required) {
      if (chip) dashReleaseSetChip(chip, `${labelText}: skipped`, 'neutral');
      lines.push(`- ${labelText}: SKIPPED (toggle disabled)`);
      continue;
    }
    if (chip) dashReleaseSetChip(chip, `${labelText}: running`, 'warn');
    const opts = {};
    if (action === 'release-collect-evidence') opts.label = label;
    if (action === 'release-verify-bundle') {
      const bundlePath = (document.getElementById('dash-release-bundle-path')?.value || '').trim();
      opts.bundle_path = bundlePath;
    }
    const data = await depRun(action, opts);
    const inner = depEnvelopeInner(data);
    const ok = !!inner?.ok;
    if (action === 'release-collect-evidence' && inner?.artifact_path) {
      const bundleInput = document.getElementById('dash-release-bundle-path');
      if (bundleInput) bundleInput.value = inner.artifact_path;
    }
    if (!ok) {
      failed = true;
      if (chip) dashReleaseSetChip(chip, `${labelText}: fail`, 'fail');
      lines.push(`- ${labelText}: FAIL`);
      lines.push(`  reason: ${inner?.error || data?.error || 'unknown error'}`);
      break;
    }
    passed += 1;
    if (chip) dashReleaseSetChip(chip, `${labelText}: pass`, 'ok');
    lines.push(`- ${labelText}: PASS`);
    if (inner?.artifact_path) lines.push(`  artifact: ${inner.artifact_path}`);
  }

  if (!failed) {
    const signed = await dashExportEvidence();
    const signedOk = !!signed?.ok;
    if (signedOk) {
      passed += 1;
      requiredTotal += 1;
      lines.push(`- Signed Evidence Export: PASS`);
      lines.push(`  artifact: ${signed.path || 'generated'}`);
    } else {
      requiredTotal += 1;
      failed = true;
      lines.push(`- Signed Evidence Export: FAIL`);
      lines.push(`  reason: ${signed?.error || 'export failed'}`);
    }
  }

  const score = requiredTotal > 0 ? Math.round((passed / requiredTotal) * 100) : 0;
  dashReleaseSetChip(dashReleaseScoreChip, `Score: ${score}%`, failed ? 'fail' : 'ok');
  lines.push(``);
  lines.push(`Release Readiness Score: ${score}% (${passed}/${requiredTotal})`);
  lines.push(`Result: ${failed ? 'FAILED' : 'SUCCESS'}`);
  dashReleaseWrite(lines);
  } finally {
  if (dashReleaseRunBtn) {
    dashReleaseRunBtn.disabled = false;
    dashReleaseRunBtn.textContent = 'Run Release Routine';
  }
  }
}

function dashWorkflowHint(kind) {
  const guides = {
    health: {
      tab: 'dashboard',
      title: 'Workflow Hint: Health Path',
      text: 'Guided path (no macro execution): 1) Status 2) Bus Status 3) Oracle Query 4) Heal Plan 5) Heal Run'
    },
    branch: {
      tab: 'branch',
      title: 'Workflow Hint: Branch Path',
      text: 'Guided path (no macro execution): 1) Branch Preview 2) Multi Status 3) DAG Preview 4) Staged Apply Preview/Execute'
    },
    push: {
      tab: 'dashboard',
      title: 'Workflow Hint: Push Path',
      text: 'Guided path (no macro execution): 1) Push Safe 2) Timeline Verify (check latest events + artifacts).'
    }
  };
  const guide = guides[kind] || guides.health;
  activatePanel(guide.tab);
  const msg = { ok: true, mode: 'hint_only', workflow: kind, message: guide.text };
  if (dashStatusOut) {
    dashStatusOut.textContent = JSON.stringify(msg, null, 2);
  }
  logActivity(guide.title, msg);
}

function dashAgorgContractPayload() {
  const cls = readInputValue('dash-agorg-contract-class');
  const dryRun = !!(document.getElementById('dash-agorg-contract-dry-run') && document.getElementById('dash-agorg-contract-dry-run').checked);
  const payload = { schema_version: 1, dry_run: dryRun };
  if (cls) payload.issue_class = cls;
  return payload;
}

function dashAgorgContractCommand() {
  return readInputValue('dash-agorg-contract-command') || 'api.agorg.reconcile_apply';
}

async function dashAgorgContractPreview() {
  const req = {
    mode: 'preview',
    intent: readInputValue('dash-agorg-contract-intent') || 'AGOrg policy action',
    command: dashAgorgContractCommand(),
    payload: dashAgorgContractPayload(),
    expected_effect: 'Resolve selected AGOrg policy drift class with governed execution',
    rollback_strategy: 'Use AGOrg reconcile dry-run before apply; restore from report artifact if needed',
    verify_command: 'api.agorg.policy_report'
  };
  const data = await fetchJsonSafe('/api/codex/action', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAgorgContractOut) dashAgorgContractOut.textContent = text;
  out.textContent = text;
  if (data && data.contract && data.contract.contract_id) {
    setVal('dash-agorg-contract-id', data.contract.contract_id);
  }
}

async function dashAgorgContractApprove() {
  const req = {
    mode: 'approve',
    contract_id: readInputValue('dash-agorg-contract-id'),
    expected_effect: 'Approved AGOrg policy action contract',
    verify_command: 'api.agorg.policy_report'
  };
  const data = await fetchJsonSafe('/api/codex/action', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAgorgContractOut) dashAgorgContractOut.textContent = text;
  out.textContent = text;
}

async function dashAgorgContractExecute() {
  const req = {
    mode: 'execute',
    contract_id: readInputValue('dash-agorg-contract-id')
  };
  const data = await fetchJsonSafe('/api/codex/action', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAgorgContractOut) dashAgorgContractOut.textContent = text;
  out.textContent = text;
  if (data && data.response && data.response.report) {
    setDashAgorgIssueStateFromReport(data.response.report);
    setAgorgIssueStateFromReport(data.response.report);
    if (dashAgorgClassCountsOut) dashAgorgClassCountsOut.textContent = renderClassCountsText(data.response.report);
  }
}

async function dashAgorgContractReconcile() {
  const req = {
    mode: 'reconcile',
    contract_id: readInputValue('dash-agorg-contract-id'),
    reconcile_notes: 'Dashboard AGOrg contract reconcile completed'
  };
  const data = await fetchJsonSafe('/api/codex/action', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAgorgContractOut) dashAgorgContractOut.textContent = text;
  out.textContent = text;
  await dashAgorgOverviewRefresh();
}

async function dashAgorgOverviewRefresh() {
  const data = await fetchJsonSafe('/api/agorg/dashboard_overview', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({})
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAgorgOverviewOut) dashAgorgOverviewOut.textContent = text;
  if (data && data.ok) {
    const score = Number(data.score || 0);
    setChip(dashAgorgScoreChip, `Score: ${score}`, score >= 80 ? 'ok' : (score >= 50 ? 'warn' : 'fail'));
    setChip(dashAgorgIssuesChip, `Issues: ${Number(data.unresolved_issues || 0)}`, Number(data.unresolved_issues || 0) === 0 ? 'ok' : 'warn');
    setChip(dashAgorgOffpolicyChip, `Off-policy: ${Number(data.off_policy || 0)}`, Number(data.off_policy || 0) === 0 ? 'ok' : 'fail');
    if (data.report) {
      agorgReconcileState.report = data.report;
      setDashAgorgIssueStateFromReport(data.report, true);
      setAgorgIssueStateFromReport(data.report, true);
      if (dashAgorgClassCountsOut) dashAgorgClassCountsOut.textContent = renderClassCountsText(data.report);
      if (agorgClassCountsOut) agorgClassCountsOut.textContent = renderClassCountsText(data.report);
      renderReconcileParitySummary();
    }
  }
}

async function dashRefreshTemporaryComponents() {
  const data = await fetchJsonSafe('/api/system/temporary_components');
  const text = JSON.stringify(data, null, 2);
  if (dashTempComponentsOut) dashTempComponentsOut.textContent = text;
  if (data && data.ok) {
    appendLive({
      source: 'dashboard',
      action: 'temporary-components-refresh',
      ok: true,
      count: Number(data.count || 0)
    });
  }
}

async function dashRunTemporaryChecklist() {
  const data = await fetchJsonSafe('/api/system/temporary_components/checklist');
  const text = JSON.stringify(data, null, 2);
  if (dashTempChecklistOut) dashTempChecklistOut.textContent = text;
  if (data && data.ok) {
    const pass = !!data.overall_pass;
    appendLive({
      source: 'dashboard',
      action: 'temporary-components-checklist',
      ok: pass,
      summary: pass ? 'all required checklist items passed' : 'one or more required checklist items failed'
    });
  }
}

async function dashExportTemporaryComponents() {
  const data = await fetchJsonSafe('/api/system/temporary_components/export', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({})
  });
  const text = JSON.stringify(data, null, 2);
  if (dashTempComponentsOut) dashTempComponentsOut.textContent = text;
  out.textContent = text;
  appendLive({
    source: 'dashboard',
    action: 'temporary-components-export',
    ok: !!(data && data.ok),
    artifact_path: (data && data.path) ? data.path : ''
  });
}

async function dashRunAcceptanceMatrix() {
  const wave = readInputValue('dash-accept-wave') || 'I';
  const profile = readInputValue('dash-accept-profile') || 'quick';
  const data = await fetchJsonSafe('/api/system/acceptance_matrix/run', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ wave, profile })
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAcceptanceMatrixOut) dashAcceptanceMatrixOut.textContent = text;
  out.textContent = text;
  appendLive({
    source: 'dashboard',
    action: 'acceptance-matrix-run',
    ok: !!(data && data.ok),
    artifact_path: (data && data.artifact_path) ? data.artifact_path : ''
  });
}

async function openDashAcceptanceArtifact() {
  const path = extractArtifactPathFromJsonText(dashAcceptanceMatrixOut ? dashAcceptanceMatrixOut.textContent : '');
  if (!path) {
    const msg = JSON.stringify({ ok: false, error: 'No artifact_path found in acceptance matrix output.' }, null, 2);
    if (dashAcceptanceMatrixOut) dashAcceptanceMatrixOut.textContent = msg;
    out.textContent = msg;
    return;
  }
  await openReportPath(path, dashAcceptanceMatrixOut, out);
}

async function dashAgorgPolicyReport() {
  const cls = selectedReconcileClass();
  syncReconcileClassControls(cls);
  const body = cls ? { issue_class: cls } : {};
  const data = await fetchJsonSafe('/api/agorg/policy_report', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(body)
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAgorgPolicyOut) dashAgorgPolicyOut.textContent = text;
  out.textContent = text;
  if (dashStatusOut) dashStatusOut.textContent = text;
  if (data && data.ok) {
    agorgReconcileState.report = data.report || null;
    const offPolicy = (data.report && Number(data.report.off_policy_count)) || 0;
    setChip(dashDriftChip, 'Drift: ' + (offPolicy > 0 ? 'FAIL' : 'PASS'), offPolicy > 0 ? 'fail' : 'ok');
    appendLive({ source: 'dashboard', action: 'agorg-policy-report', ok: true, artifact_path: data.artifact_path || '' });
    if (dashAgorgDuplicatesOut) dashAgorgDuplicatesOut.textContent = renderDuplicateResolutionText(data.report);
    if (agorgDuplicatePreviewOut) agorgDuplicatePreviewOut.textContent = renderDuplicateResolutionText(data.report);
    setDashAgorgDuplicateStateFromReport(data.report);
    setAgorgDuplicateStateFromReport(data.report);
    if (dashAgorgClassCountsOut) dashAgorgClassCountsOut.textContent = renderClassCountsText(data.report);
    if (agorgClassCountsOut) agorgClassCountsOut.textContent = renderClassCountsText(data.report);
    setDashAgorgIssueStateFromReport(data.report);
    setAgorgIssueStateFromReport(data.report);
    await agorgLoadPolicyReports();
    renderReconcileParitySummary();
  }
}

async function dashAgorgReconcileDryRun() {
  const cls = selectedReconcileClass();
  syncReconcileClassControls(cls);
  const body = cls ? { dry_run: true, issue_class: cls } : { dry_run: true };
  const data = await fetchJsonSafe('/api/agorg/reconcile_apply', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(body)
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAgorgPolicyOut) dashAgorgPolicyOut.textContent = text;
  out.textContent = text;
  if (dashStatusOut) dashStatusOut.textContent = text;
  if (dashAgorgDuplicatesOut && data && data.report) {
    dashAgorgDuplicatesOut.textContent = renderDuplicateResolutionText(data.report);
  }
  if (agorgDuplicatePreviewOut && data && data.report) {
    agorgDuplicatePreviewOut.textContent = renderDuplicateResolutionText(data.report);
  }
  if (data && data.report) {
    setDashAgorgDuplicateStateFromReport(data.report);
    setAgorgDuplicateStateFromReport(data.report);
  }
  if (dashAgorgClassCountsOut && data && data.report) {
    dashAgorgClassCountsOut.textContent = renderClassCountsText(data.report);
  }
  if (agorgClassCountsOut && data && data.report) {
    agorgClassCountsOut.textContent = renderClassCountsText(data.report);
  }
  if (data && data.report) {
    agorgReconcileState.dryRun = data;
    const tokenClass = cls || 'all';
    if (data.dry_run_token) agorgReconcileState.dryRunTokenByClass[tokenClass] = data.dry_run_token;
    setDashAgorgIssueStateFromReport(data.report);
    setAgorgIssueStateFromReport(data.report);
    renderReconcileParitySummary();
  }
  appendLive({
    source: 'dashboard',
    action: 'agorg-reconcile-dry-run',
    ok: !!data.ok,
    planned_prune_count: data.planned_prune_count || 0
  });
}

async function dashAgorgReconcileApply() {
  const cls = selectedReconcileClass();
  syncReconcileClassControls(cls);
  const tokenClass = cls || 'all';
  const dryRunToken = agorgReconcileState.dryRunTokenByClass[tokenClass] || '';
  if (!dryRunToken) {
    const msg = JSON.stringify({ ok: false, error: `Run Reconcile Dry Run first for issue_class='${tokenClass}'` }, null, 2);
    if (dashAgorgPolicyOut) dashAgorgPolicyOut.textContent = msg;
    out.textContent = msg;
    if (dashStatusOut) dashStatusOut.textContent = msg;
    return;
  }
  const body = cls
    ? { dry_run: false, issue_class: cls, dry_run_token: dryRunToken }
    : { dry_run: false, dry_run_token: dryRunToken };
  const data = await fetchJsonSafe('/api/agorg/reconcile_apply', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(body)
  });
  const text = JSON.stringify(data, null, 2);
  if (dashAgorgPolicyOut) dashAgorgPolicyOut.textContent = text;
  out.textContent = text;
  if (dashStatusOut) dashStatusOut.textContent = text;
  if (dashAgorgDuplicatesOut && data && data.before) {
    dashAgorgDuplicatesOut.textContent = renderDuplicateResolutionText(data.before);
  }
  if (agorgDuplicatePreviewOut && data && data.before) {
    agorgDuplicatePreviewOut.textContent = renderDuplicateResolutionText(data.before);
  }
  if (data && data.before) {
    setDashAgorgDuplicateStateFromReport(data.before);
    setAgorgDuplicateStateFromReport(data.before);
  }
  if (dashAgorgClassCountsOut && data && data.before) {
    dashAgorgClassCountsOut.textContent = renderClassCountsText(data.before);
  }
  if (agorgClassCountsOut && data && data.before) {
    agorgClassCountsOut.textContent = renderClassCountsText(data.before);
  }
  if (data && data.before) {
    agorgReconcileState.apply = data;
    setDashAgorgIssueStateFromReport(data.before);
    setAgorgIssueStateFromReport(data.before);
    renderReconcileParitySummary();
  }
  appendLive({ source: 'dashboard', action: 'agorg-reconcile-apply', ok: !!data.ok, pruned: data.pruned || 0 });
  if (data && data.ok) {
    await agorgTree();
    await agorgList();
    await agorgShowActive();
  }
}

async function dashAgorgPolicyReports() {
  await agorgLoadPolicyReports();
  const selected = dashAgorgReportSelect && dashAgorgReportSelect.value ? dashAgorgReportSelect.value : '';
  const payload = { ok: true, selected_artifact: selected || null };
  if (dashAgorgPolicyOut) dashAgorgPolicyOut.textContent = JSON.stringify(payload, null, 2);
}

async function dashAgorgPolicyOpen() {
  const selected = dashAgorgReportSelect && dashAgorgReportSelect.value ? dashAgorgReportSelect.value : '';
  await openReportPath(selected, dashAgorgPolicyOut, out);
}

async function dashExportEvidence() {
  const res = await fetch('/api/evidence/export', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({})
  });
  const data = await res.json();
  const text = JSON.stringify(data, null, 2);
  out.textContent = text;
  if (dashStatusOut) dashStatusOut.textContent = text;
  appendLive({ source: 'dashboard', action: 'evidence-export', ok: !!data.ok, path: data.path || '' });
  return data;
}

function codexPayloadFromUi() {
  const raw = document.getElementById('codex-payload').value.trim();
  if (!raw) return {};
  return JSON.parse(raw);
}

async function codexRun(mode) {
  codexActionQueue = codexActionQueue.then(() => codexRunNow(mode), () => codexRunNow(mode));
  return codexActionQueue;
}

async function codexRunNow(mode) {
  codexActionBusy = true;
  let payload;
  try {
    payload = codexPayloadFromUi();
  } catch (e) {
    const msg = 'Invalid JSON payload: ' + e.message;
    codexOut.textContent = msg;
    out.textContent = msg;
    codexActionBusy = false;
    return null;
  }
  const req = {
    contract_id: document.getElementById('codex-contract-id').value.trim(),
    intent: document.getElementById('codex-intent').value.trim(),
    command: document.getElementById('codex-command').value.trim(),
    payload,
    mode,
    expected_effect: document.getElementById('codex-expected').value.trim(),
    rollback_strategy: document.getElementById('codex-rollback').value.trim(),
    verify_command: document.getElementById('codex-verify').value.trim(),
    reconcile_notes: document.getElementById('codex-reconcile-notes').value.trim()
  };
  try {
    const res = await fetch('/api/codex/action', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify(req)
    });
    const raw = await res.text();
    let data;
    try {
      data = raw ? JSON.parse(raw) : {};
    } catch (_) {
      data = { ok: false, error: `Non-JSON response (${res.status})`, raw };
    }
    if (!res.ok && !data.error) {
      data.error = `HTTP ${res.status}`;
      data.ok = false;
    }
    const text = JSON.stringify(data, null, 2);
    codexOut.textContent = text;
    out.textContent = text;
    if (dashStatusOut) dashStatusOut.textContent = text;
    if (data && data.contract && data.contract.contract_id) {
      latestCodexContractId = data.contract.contract_id;
      document.getElementById('codex-contract-id').value = latestCodexContractId;
    }
    appendLive({ source: 'codex_ui', mode, command: req.command, ok: !!data.ok });
    if (mode === 'execute' || mode === 'reconcile' || mode === 'approve') await loadHistory();
    if (mode === 'execute' || mode === 'reconcile' || mode === 'approve' || mode === 'preview') await codexLoadContracts();
    if (data && data.contract && data.contract.contract_id) await codexLoadSelectedContract();
    return data;
  } catch (err) {
    const msg = {
      ok: false,
      error: err?.message || 'Codex action request failed',
      mode,
      request: req
    };
    const text = JSON.stringify(msg, null, 2);
    codexOut.textContent = text;
    out.textContent = text;
    if (dashStatusOut) dashStatusOut.textContent = text;
    appendLive({ source: 'codex_ui', mode, command: req.command, ok: false, error: msg.error });
    return msg;
  } finally {
    codexActionBusy = false;
  }
}

function codexPreview() { codexRun('preview'); }
function codexApprove() {
  if (!document.getElementById('codex-contract-id').value.trim() && latestCodexContractId) {
    document.getElementById('codex-contract-id').value = latestCodexContractId;
  }
  codexRun('approve');
}
async function codexExecute() { return codexRun('execute'); }
async function codexReconcile() {
  if (!document.getElementById('codex-contract-id').value.trim() && latestCodexContractId) {
    document.getElementById('codex-contract-id').value = latestCodexContractId;
  }
  const activeId = document.getElementById('codex-contract-id').value.trim();
  if (activeId) {
    const selected = await codexFetchContract(activeId);
    const status = (selected && selected.contract && selected.contract.status)
      ? String(selected.contract.status).toLowerCase()
      : '';
    if (status === 'approved') {
      await codexRun('execute');
    }
  }
  return codexRun('reconcile');
}

async function codexLoadContracts() {
  const status = document.getElementById('codex-contract-filter').value.trim();
  const qs = new URLSearchParams({ limit: '100' });
  if (status) qs.set('status', status);
  const res = await fetch('/api/codex/contracts?' + qs.toString());
  const data = await res.json();
  const items = (data && data.contracts) ? data.contracts : [];
  codexContractSelect.innerHTML = '';
  for (const c of items) {
    const opt = document.createElement('option');
    opt.value = c.contract_id;
    opt.textContent = `${c.contract_id} | ${c.status} | ${c.command}`;
    codexContractSelect.appendChild(opt);
  }
  if (items.length > 0) {
    latestCodexContractId = items[0].contract_id;
  }
  codexContractsOut.textContent = JSON.stringify(items, null, 2);
}

async function codexLoadSelectedContract() {
  const id = codexContractSelect.value || document.getElementById('codex-contract-id').value.trim();
  if (!id) {
    codexContractsOut.textContent = 'No contract selected.';
    return;
  }
  const res = await fetch('/api/codex/contract?contract_id=' + encodeURIComponent(id));
  const data = await res.json();
  if (data && data.contract) {
    const c = data.contract;
    document.getElementById('codex-contract-id').value = c.contract_id || '';
    document.getElementById('codex-intent').value = c.intent || '';
    document.getElementById('codex-command').value = c.command || '';
    document.getElementById('codex-payload').value = JSON.stringify(c.payload_original || {}, null, 2);
    document.getElementById('codex-expected').value = c.expected_effect || '';
    document.getElementById('codex-rollback').value = c.rollback_strategy || '';
    document.getElementById('codex-verify').value = c.verify_command || '';
    latestCodexContractId = c.contract_id || latestCodexContractId;
  }
  codexContractsOut.textContent = JSON.stringify(data, null, 2);
}

async function codexFetchContract(contractId) {
  if (!contractId) return null;
  const res = await fetch('/api/codex/contract?contract_id=' + encodeURIComponent(contractId));
  return res.json();
}

async function codexRetryFailedContract() {
  await codexLoadSelectedContract();
  await codexRun('approve');
  await codexRun('execute');
}

async function loadHistory() {
  const res = await fetch('/api/history');
  const data = await res.json();
  auditCache = (data && data.events) ? data.events : [];
  renderOperationDetail();
}


function attachStream() {
  streamHandle = new EventSource('/api/stream');
  streamHandle.onopen = () => {
    setBusStatus(true);
  };
  streamHandle.addEventListener('pilot_event', (evt) => {
    if (streamPaused) return;
    try {
      const parsed = JSON.parse(evt.data);
      if (parsed && parsed.source === 'bus_listener' && parsed.error) {
        setBusStatus(false, parsed.error);
      } else {
        setBusStatus(true);
      }
      if (parsed && parsed.source === 'dependency_action' && parsed.action) {
        updateDashChip(parsed.action, !!parsed.success, parsed);
      }
      appendLive(parsed);
    } catch (_) {
      appendLive({ raw: evt.data });
    }
  });
  streamHandle.onerror = () => {
    setBusStatus(false, 'stream disconnected, retrying...');
    appendLive({ source: 'ui', warning: 'stream disconnected, retrying...' });
  };
}

function toggleStream() {
  streamPaused = !streamPaused;
  streamToggleBtn.textContent = streamPaused ? 'Resume Stream' : 'Pause Stream';
  appendLive({ source: 'ui', info: streamPaused ? 'stream paused' : 'stream resumed' });
}

async function restoreUiSession() {
  restoringUiSession = true;
  try {
    const snapshot = await hydrateScopeSnapshot(true);
    if (snapshot.uiSession && typeof snapshot.uiSession === 'object') {
      applyUiSessionState(snapshot.uiSession);
    }
  } finally {
    restoringUiSession = false;
  }
}

async function bootUi() {
  attachStream();
  multiSetOutputMode('html');
  restoreBusStatus();
  await restoreUiSession();
  await loadHistory();
  await oracleLoadReports();
  await depLoadLogs();
  await refreshPolicyHookDriftChips();
  await depRun('services-status');
  await refreshAgorgHeader();
  await dashRefreshTemporaryComponents();
  await dashRunTemporaryChecklist();
  await agorgLoadPolicyReports();
  await loadRepoAgoOptions();
  await codexLoadContracts();
  if (currentTab === 'dashboard') routineStartCiSyncLoop();
  setInterval(loadHistory, 30000);
  document.querySelectorAll('input, textarea, select').forEach((el) => {
    el.addEventListener('change', () => queueUiSessionSave());
  });
  ['branch-matrix-group', 'branch-matrix-tags', 'branch-matrix-search', 'branch-matrix-base']
    .forEach((id) => {
      const el = document.getElementById(id);
      if (!el) return;
      el.addEventListener('change', () => {
        if (currentTab === 'branch') branchLoadMatrix();
      });
    });
  ['multi-group', 'multi-tags'].forEach((id) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('change', () => {
      if (currentTab === 'multi') multiRefreshRegistry();
    });
    el.addEventListener('input', () => {
      if (currentTab === 'multi') multiRefreshRegistry();
    });
  });
  if (branchMatrixAdvanced) {
    branchMatrixAdvanced.addEventListener('toggle', () => queueUiSessionSave());
  }
  if (branchLogLimitInput) {
    branchLogLimitInput.addEventListener('change', () => {
      persistBranchLogLimit();
      if (branchLogItems.length > getBranchLogLimit()) {
        branchLogItems = branchLogItems.slice(0, getBranchLogLimit());
      }
      branchRenderLog();
    });
  }
  ['branch-name', 'branch-base', 'sync-branch', 'sync-base',
   'branch-apply-branch', 'branch-apply-base', 'branch-apply-pr-base', 'branch-apply-stage-size', 'branch-apply-continue']
    .forEach((id) => {
      const el = document.getElementById(id);
      if (!el) return;
      el.addEventListener('change', () => invalidateBranchPreviews(`input changed: ${id}`));
    });
  if (branchPruneModal) {
    branchPruneModal.addEventListener('click', (evt) => {
      if (evt.target === branchPruneModal) branchCancelPruneConfirm();
    });
  }
  if (multiScopeModal) {
    multiScopeModal.addEventListener('click', (evt) => {
      if (evt.target === multiScopeModal) closeMultiScopeModal();
    });
  }
  if (dashRoutinePolicyModal) {
    dashRoutinePolicyModal.addEventListener('click', (evt) => {
      if (evt.target === dashRoutinePolicyModal) routinePolicyModalClose();
    });
  }
  ['dash-routine-group', 'dash-routine-tags', 'dash-routine-branch', 'dash-routine-remote'].forEach((id) => {
    const el = document.getElementById(id);
    if (!el) return;
    const handler = () => {
      if (id === 'dash-routine-branch' || id === 'dash-routine-remote') routineSyncPushControls();
      dashRoutineWorkspaceState.scope = parseMultiScopeFromRoutine();
      routineRefreshPlanPreview();
      routineRenderWorkspace();
      queueUiSessionSave();
    };
    el.addEventListener('change', handler);
    el.addEventListener('input', handler);
  });
  ['dash-routine-allow-push', 'dash-routine-export-evidence', 'dash-routine-auto-heal', 'dash-routine-auto-codex'].forEach((id) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('change', () => {
      routineUpdateModeChip();
      routineRefreshPlanPreview();
      routineRenderWorkspace();
      queueUiSessionSave();
    });
  });
  ROUTINE_STAGE_ORDER.forEach((stage) => {
    const tab = routineStageTabEl(stage);
    if (!tab) return;
    tab.addEventListener('keydown', (evt) => {
      const currentIdx = ROUTINE_STAGE_ORDER.indexOf(stage);
      if (evt.key === 'ArrowRight' || evt.key === 'ArrowLeft') {
        evt.preventDefault();
        const delta = evt.key === 'ArrowRight' ? 1 : -1;
        const nextIdx = (currentIdx + delta + ROUTINE_STAGE_ORDER.length) % ROUTINE_STAGE_ORDER.length;
        const nextStage = ROUTINE_STAGE_ORDER[nextIdx];
        routineSelectStage(nextStage);
        routineStageTabEl(nextStage)?.focus();
        return;
      }
      if (evt.key === 'Home') {
        evt.preventDefault();
        routineSelectStage(ROUTINE_STAGE_ORDER[0]);
        routineStageTabEl(ROUTINE_STAGE_ORDER[0])?.focus();
        return;
      }
      if (evt.key === 'End') {
        evt.preventDefault();
        const lastStage = ROUTINE_STAGE_ORDER[ROUTINE_STAGE_ORDER.length - 1];
        routineSelectStage(lastStage);
        routineStageTabEl(lastStage)?.focus();
        return;
      }
      if (evt.key === 'Enter' || evt.key === ' ') {
        evt.preventDefault();
        routineSelectStage(stage);
      }
    });
  });
  restoreBranchLogLimit();
  refreshBranchPreviewState();
  branchRenderLog();
  routineSyncPushControls();
  routineResetChips();
  routineRefreshPlanPreview();
  routineRenderWorkspace();
  routineUpdateStageTabs();
  await dashLoadRoutine();
}

/* ==========================================================================
 * SETTINGS & GOVERNANCE UI
 * ========================================================================== */

let settingsActiveSimulationId = "";
let settingsLastSimulatedFingerprint = "";
let settingsLoadedPolicyJson = null;
let settingsLoadedPolicyMeta = null;

function settingsSetStatus(data, level = 'info') {
  if (!settingsStatusOut) return;
  if (level !== 'error') {
    clearInlineError(settingsStatusPanel);
  }
  
  if (data && typeof data === 'object' && data.ok) {
     if (data.details && Array.isArray(data.details)) {
        settingsStatusOut.innerHTML = renderComplianceTable(data);
     } else if (data.resolved_policy) {
        settingsStatusOut.innerHTML = renderResolvedPolicy(data);
     } else {
        settingsStatusOut.textContent = JSON.stringify(data, null, 2);
     }
  } else {
    const rendered = typeof data === 'string' ? data : JSON.stringify(data, null, 2);
    settingsStatusOut.textContent = rendered;
  }

  if (settingsStatusPanel) {
    settingsStatusPanel.classList.remove('ok', 'warn', 'fail');
    if (level === 'success') settingsStatusPanel.classList.add('ok');
    if (level === 'warn') settingsStatusPanel.classList.add('warn');
    if (level === 'error') settingsStatusPanel.classList.add('fail');
  }
}

function renderComplianceTable(data) {
  let html = `
    <div style="margin-bottom:10px; font-weight:600; color:var(--primary);">
      Compliance Scan: ${data.kind} (${data.status})
    </div>
    <table class="gov-table">
      <thead>
        <tr>
          <th>Repo / Path</th>
          <th>Source</th>
          <th>Status</th>
          <th>Issues</th>
        </tr>
      </thead>
      <tbody>
  `;
  
  data.details.forEach(repo => {
    const sourceClass = repo.is_override ? 'ago' : (repo.policy_source === 'Default' ? 'default' : 'agorg');
    const sourceLabel = repo.policy_source + (repo.is_override ? ' (Override)' : '');
    const statusColor = repo.blocked ? 'var(--rose)' : (repo.violations > 0 ? 'var(--accent)' : '#10B981');
    
    html += `
      <tr>
        <td>
          <div style="font-weight:500;">${repo.repo}</div>
          <div style="font-size:0.7rem; color:var(--muted);">${repo.path}</div>
        </td>
        <td>
          <span class="source-pill ${sourceClass}">${sourceLabel}</span>
        </td>
        <td style="color:${statusColor}; font-weight:600;">
          ${repo.blocked ? 'BLOCKED' : (repo.violations > 0 ? 'VIOLATIONS' : 'PASS')}
        </td>
        <td>
          ${repo.violations}V / ${repo.warnings}W
        </td>
      </tr>
    `;
  });
  
  html += `</tbody></table>`;
  return html;
}

function renderResolvedPolicy(data) {
  const sourceClass = data.is_override ? 'ago' : (data.source === 'Default' ? 'default' : 'agorg');
  return `
    <div style="margin-bottom:15px;">
      <div style="font-weight:600; color:var(--primary); margin-bottom:4px;">Resolved ${data.kind} Policy</div>
      <div class="inheritance-trace">
        Source: <span class="source-pill ${sourceClass}">${data.source}</span>
        ${data.is_override ? '<span class="override-tag">Local Override</span>' : ''}
        <div style="font-size:0.7rem; margin-top:4px;">Version: ${data.version} | Status: ${data.status}</div>
      </div>
    </div>
    <div style="background:rgba(0,0,0,0.2); padding:10px; border-radius:4px; font-family:'JetBrains Mono',monospace; font-size:0.8rem; border:1px solid var(--border);">
      <pre style="margin:0;">${JSON.stringify(data.resolved_policy, null, 2)}</pre>
    </div>
  `;
}

function settingsShowError(message, context = null) {
  const payload = context ? `${message}\n${JSON.stringify(context, null, 2)}` : message;
  settingsSetStatus(payload, 'error');
  if (settingsStatusPanel) {
    showInlineError(message, settingsStatusPanel);
  }
  logActivity('Settings Error', context || { error: message });
}

function isErrorResponse(res) {
  return !res || res.ok === false || typeof res.error === 'string';
}

function settingsTargetValue() {
  const el = document.getElementById('settings-policy-target');
  return el ? (el.value || '').trim() : '';
}

function settingsPolicyEditorEl() {
  return document.getElementById('settings-policy-editor');
}

function settingsPolicyNameEl() {
  return document.getElementById('settings-policy-name');
}

function settingsPolicyFormEl() {
  return document.getElementById('settings-policy-form');
}

function settingsPolicyViewRaw(showRaw) {
  const editor = settingsPolicyEditorEl();
  const form = settingsPolicyFormEl();
  if (editor) editor.style.display = showRaw ? 'block' : 'none';
  if (form) form.style.display = showRaw ? 'none' : 'block';
}

function settingsPolicySyncNameToEditor() {
  const editor = settingsPolicyEditorEl();
  const nameEl = settingsPolicyNameEl();
  if (!editor || !nameEl) return;
  try {
    const parsed = JSON.parse(editor.value || '{}');
    policyApplyDisplayName(parsed, nameEl.value);
    editor.value = JSON.stringify(parsed, null, 2);
  } catch (_) {}
}

function settingsPolicyRenderForm() {
  const editor = settingsPolicyEditorEl();
  const nameEl = settingsPolicyNameEl();
  const form = settingsPolicyFormEl();
  if (!editor || !form) return;
  let parsed;
  try {
    parsed = JSON.parse(editor.value || '{}');
  } catch (err) {
    settingsShowError(`Invalid JSON: ${err.message}`);
    return;
  }
  if (nameEl) nameEl.value = policyGetDisplayName(parsed, 'Policy');
  policyRenderFormInto(form, parsed);
  settingsPolicyRefreshInsights(parsed);
}

function settingsPolicyOnFormInput(event) {
  policySyncFormChangeToEditor(
    event,
    settingsPolicyEditorEl(),
    settingsPolicyNameEl(),
    (msg) => settingsSetStatus(msg, 'warn')
  );
  settingsPolicyRenderForm();
}

function settingsPolicyApplyName() {
  settingsPolicySyncNameToEditor();
  settingsPolicyRenderForm();
}

async function settingsRefreshTargetOptions() {
  const targetEl = document.getElementById('settings-policy-target');
  if (!targetEl) return;
  const prev = settingsTargetValue();
  targetEl.innerHTML = '<option value="">AGOrg level (no AGO override)</option>';

  const active = await fetchJsonSafe('/api/agorg/active');
  if (!active.ok || !active.active || !active.active.id) {
    return;
  }

  const matrix = await fetchJsonSafe('/api/branch/matrix', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({
      group: null,
      tags: [],
      search: null,
      base_branch: 'main',
      target_branch: null
    })
  });
  if (!matrix.ok || !Array.isArray(matrix.rows)) {
    return;
  }

  const seen = new Set();
  matrix.rows.forEach(row => {
    const path = (row && row.path) ? String(row.path).trim() : '';
    if (!path || seen.has(path)) return;
    seen.add(path);
    const label = row.repo ? `${row.repo} - ${path}` : path;
    const opt = document.createElement('option');
    opt.value = path;
    opt.textContent = label;
    targetEl.appendChild(opt);
  });

  if (prev && seen.has(prev)) {
    targetEl.value = prev;
  } else {
    targetEl.value = '';
  }
}

async function settingsReloadPolicyControls() {
  await settingsLoadPolicy();
  await settingsLoadPolicyVersions();
}

function settingsSelectedVersion() {
  const list = document.getElementById('settings-policy-versions');
  if (!list || !list.value) return null;
  const parsed = Number.parseInt(list.value, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

async function settingsLoadPolicyVersions() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue();
  const list = document.getElementById('settings-policy-versions');
  if (!list) return;
  list.innerHTML = '<option>Loading...</option>';

  let url = `/api/settings/policy/${kind}/versions?limit=50`;
  if (target) url += `&ago_path=${encodeURIComponent(target)}`;
  const res = await fetchJsonSafe(url);
  if (isErrorResponse(res)) {
    list.innerHTML = '';
    settingsShowError('Failed to load policy versions: ' + (res.error || 'unknown error'), res);
    return;
  }
  const items = Array.isArray(res.items) ? res.items : [];
  if (!items.length) {
    list.innerHTML = '<option value="">No saved versions yet</option>';
    return;
  }
  list.innerHTML = '';
  items.forEach(item => {
    const opt = document.createElement('option');
    opt.value = String(item.version);
    opt.dataset.status = String(item.status || '');
    const updatedBy = item.updated_by || 'unknown';
    const status = item.status || 'unknown';
    opt.textContent = `v${item.version} [${status}] by ${updatedBy}`;
    list.appendChild(opt);
  });
  // Default to newest version to avoid ambiguous "no selection" flows.
  if (list.options.length > 0) {
    list.selectedIndex = 0;
  }
}

async function settingsLoadSelectedPolicyVersion() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const version = settingsSelectedVersion();
  const target = settingsTargetValue() || '';
  if (version === null) {
    const list = document.getElementById('settings-policy-versions');
    if (list && list.options.length > 0) list.selectedIndex = 0;
  }
  const resolvedVersion = settingsSelectedVersion();
  if (resolvedVersion === null) {
    settingsShowError('Select a policy version first.');
    return;
  }
  const res = await fetchJsonSafe(`/api/settings/policy/${kind}/load_version`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      ago_path: target === '' ? null : target,
      version: resolvedVersion
    })
  });
  if (isErrorResponse(res)) {
    settingsShowError('Failed to load selected version: ' + (res.error || 'unknown error'), res);
    return;
  }
  const editor = document.getElementById('settings-policy-editor');
  const loadedJson = res.policy_json || {};
  editor.value = JSON.stringify(loadedJson, null, 2);
  settingsLoadedPolicyJson = JSON.parse(JSON.stringify(loadedJson));
  settingsLoadedPolicyMeta = {
    source: res.source || 'Unknown',
    version: res.version ?? '?',
    status: res.status || 'unknown',
    is_override: !!res.is_override
  };
  settingsActiveSimulationId = "";
  settingsLastSimulatedFingerprint = "";
  const nameEl = settingsPolicyNameEl();
  if (nameEl) nameEl.value = policyGetDisplayName(res.policy_json || {}, `${kind} policy`);
  const form = settingsPolicyFormEl();
  if (form && !form.dataset.bound) {
    form.addEventListener('input', settingsPolicyOnFormInput);
    form.addEventListener('change', settingsPolicyOnFormInput);
    form.dataset.bound = '1';
  }
  if (editor && !editor.dataset.boundPolicy) {
    editor.addEventListener('input', () => settingsPolicyRefreshInsights());
    editor.dataset.boundPolicy = '1';
  }
  settingsPolicyRenderForm();
  settingsPolicyViewRaw(false);
  settingsSetStatus(`Loaded ${kind} v${resolvedVersion} into editor`, 'success');
}

async function settingsDeleteSelectedPolicyVersion() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const version = settingsSelectedVersion();
  const target = settingsTargetValue() || '';
  const confirm = (document.getElementById('settings-policy-delete-confirm')?.value || '').trim();
  if (version === null) {
    settingsShowError('Select a policy version first.');
    return;
  }
  const res = await fetchJsonSafe(`/api/settings/policy/${kind}/delete_version`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      ago_path: target === '' ? null : target,
      version,
      confirm
    })
  });
  if (isErrorResponse(res)) {
    settingsShowError('Failed to delete selected version: ' + (res.error || 'unknown error'), res);
    return;
  }
  settingsSetStatus(`Deleted ${kind} v${version}`, 'success');
  const confirmEl = document.getElementById('settings-policy-delete-confirm');
  if (confirmEl) confirmEl.value = '';
  await settingsReloadPolicyControls();
}

async function settingsLoadPolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue();
  const editor = document.getElementById('settings-policy-editor');
  settingsSetStatus(`Loading ${kind} policy...`, 'info');
  
  let url = `/api/settings/policy/${kind}`;
  if (target) url += `?ago_path=${encodeURIComponent(target)}`;
  
  const res = await fetchJsonSafe(url);
  if (isErrorResponse(res)) {
    if (res.error === "Policy not found") {
       const fallbackJson = { note: "No policy specific to this scope. Draft here to create one." };
       editor.value = JSON.stringify(fallbackJson, null, 2);
       settingsLoadedPolicyJson = {};
       settingsLoadedPolicyMeta = {
         source: 'Fallback/Inherited',
         version: 0,
         status: 'missing',
         is_override: false
       };
       settingsActiveSimulationId = "";
       settingsLastSimulatedFingerprint = "";
       settingsPolicyRefreshInsights(fallbackJson);
       settingsSetStatus("No policy found for this target. Editor is in fallback/inherit mode.", 'warn');
    } else {
       settingsShowError('Error loading policy: ' + (res.error || 'unknown error'), res);
    }
    return;
  }
  if (!Object.prototype.hasOwnProperty.call(res, 'policy_json')) {
    settingsShowError('Error loading policy: malformed response payload', res);
    return;
  }
  editor.value = JSON.stringify(res.policy_json, null, 2);
  settingsLoadedPolicyJson = JSON.parse(JSON.stringify(res.policy_json || {}));
  settingsLoadedPolicyMeta = {
    source: res.source || 'Unknown',
    version: Object.prototype.hasOwnProperty.call(res, 'version') ? res.version : '?',
    status: res.status || 'unknown',
    is_override: !!res.is_override
  };
  settingsActiveSimulationId = "";
  settingsLastSimulatedFingerprint = "";
  const nameEl = settingsPolicyNameEl();
  if (nameEl) nameEl.value = policyGetDisplayName(res.policy_json || {}, `${kind} policy`);
  const form = settingsPolicyFormEl();
  if (form && !form.dataset.bound) {
    form.addEventListener('input', settingsPolicyOnFormInput);
    form.addEventListener('change', settingsPolicyOnFormInput);
    form.dataset.bound = '1';
  }
  if (editor && !editor.dataset.boundPolicy) {
    editor.addEventListener('input', () => settingsPolicyRefreshInsights());
    editor.dataset.boundPolicy = '1';
  }
  settingsPolicyRenderForm();
  settingsPolicyViewRaw(false);
  const source = res.source || 'Unknown';
  const version = Object.prototype.hasOwnProperty.call(res, 'version') ? res.version : '?';
  const status = res.status || 'unknown';
  settingsSetStatus(`Loaded ${kind} policy (${source}) v${version} [${status}]`, 'success');
  logActivity('Loaded Policy', `Kind: ${kind}\nID: ${res.id}\nStatus: ${res.status}`);
  settingsPolicyRefreshInsights(res.policy_json || {});
}

async function settingsDraftPolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue() || "";
  const editor = document.getElementById('settings-policy-editor');
  
  settingsPolicySyncNameToEditor();
  let policyJson;
  try {
    policyJson = JSON.parse(editor.value);
  } catch(e) {
    settingsShowError("Invalid JSON in editor");
    return;
  }

  const res = await fetchJsonSafe(`/api/settings/policy/${kind}/draft`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
       ago_path: target === "" ? null : target,
       policy_json: policyJson
    })
  });
  
  if (isErrorResponse(res)) {
    settingsShowError('Failed to save draft: ' + (res.error || 'unknown error'), res);
    return;
  }
  settingsSetStatus(res, 'success');
  settingsActiveSimulationId = "";
  settingsLastSimulatedFingerprint = "";
  settingsReloadPolicyControls();
}

async function settingsSimulatePolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue() || "";
  const editor = document.getElementById('settings-policy-editor');
  
  settingsPolicySyncNameToEditor();
  let policyJson;
  try {
    policyJson = JSON.parse(editor.value);
  } catch(e) {
    settingsShowError("Invalid JSON in editor");
    return;
  }

  const res = await fetchJsonSafe(`/api/settings/policy/${kind}/simulate`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
       ago_path: target === "" ? null : target,
       policy_json: policyJson
    })
  });
  
  if (isErrorResponse(res)) {
    logActivity('Simulation Failed', res.error);
    settingsShowError('Simulation failed: ' + (res.error || 'unknown error'), res);
    return;
  }
  
  settingsActiveSimulationId = res.evidence_id;
  settingsLastSimulatedFingerprint = policyFingerprint(policyJson);
  settingsSetStatus(res, 'success');
  settingsPolicyRefreshInsights(policyJson);
  logActivity('Policy Simulation', JSON.stringify(res, null, 2));
}

async function settingsActivatePolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue() || "";
  
  if (!settingsActiveSimulationId) {
    settingsShowError("Must successfully simulate policy first!");
    return;
  }
  let policyJson;
  try {
    policyJson = JSON.parse(settingsPolicyEditorEl()?.value || '{}');
  } catch(e) {
    settingsShowError("Invalid JSON in editor");
    return;
  }
  const fingerprint = policyFingerprint(policyJson);
  if (!settingsLastSimulatedFingerprint || fingerprint !== settingsLastSimulatedFingerprint) {
    settingsShowError("Draft changed since last simulation. Re-run Simulate before Activate.");
    settingsPolicyRefreshInsights(policyJson);
    return;
  }

  const res = await fetchJsonSafe(`/api/settings/policy/${kind}/activate`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
       ago_path: target === "" ? null : target,
       simulation_evidence_id: settingsActiveSimulationId
    })
  });

  if (isErrorResponse(res)) {
    settingsShowError('Activation failed: ' + (res.error || 'unknown error'), res);
    return;
  }
  
  settingsSetStatus(res, 'success');
  settingsActiveSimulationId = "";
  settingsLastSimulatedFingerprint = "";
  settingsReloadPolicyControls();
}

async function settingsLoadLatestActiveVersion() {
  const list = document.getElementById('settings-policy-versions');
  if (!list || !list.options.length) {
    settingsShowError('No policy versions available.');
    return;
  }
  let selectedIndex = -1;
  for (let i = 0; i < list.options.length; i += 1) {
    if ((list.options[i].dataset.status || '').toLowerCase() === 'active') {
      selectedIndex = i;
      break;
    }
  }
  if (selectedIndex < 0) {
    settingsShowError('No active policy version found in list.');
    return;
  }
  list.selectedIndex = selectedIndex;
  await settingsLoadSelectedPolicyVersion();
}

async function settingsLoadExceptions() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue() || "";
  
  let url = `/api/settings/exceptions/${kind}`;
  if (target) url += `?ago_path=${encodeURIComponent(target)}`;
  
  const res = await fetchJsonSafe(url);
  const list = document.getElementById('settings-exceptions-list');
  list.innerHTML = "";
  if (isErrorResponse(res)) {
    settingsShowError('Failed to load exceptions: ' + (res.error || 'unknown error'), res);
    return;
  }
  
  const items = Array.isArray(res) ? res : (Array.isArray(res.exceptions) ? res.exceptions : null);
  if (!items) {
    settingsShowError('Failed to load exceptions: malformed response payload', res);
    return;
  }
  items.forEach(exc => {
      const opt = document.createElement("option");
      opt.value = exc.id;
      opt.textContent = `[${exc.rule_path}] by ${exc.owner} - ${exc.reason} (Expires: ${new Date(exc.expires_at).toLocaleString()})`;
      list.appendChild(opt);
  });
  settingsSetStatus({ ok: true, loaded: items.length }, 'success');
}

async function settingsDeleteException() {
  const list = document.getElementById('settings-exceptions-list');
  if(!list.value) {
     settingsShowError("Select an exception to revoke.");
     return;
  }
  const res = await fetchJsonSafe(`/api/settings/exceptions/delete/${list.value}`, { method: 'POST' });
  if(isErrorResponse(res)) {
      settingsShowError("Failed to delete: " + (res.error || 'unknown error'), res);
      return;
  }
  settingsSetStatus(res, 'success');
  settingsLoadExceptions();
}

async function settingsAddException() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue() || "";
  
  const rule = document.getElementById('settings-exc-rule').value.trim();
  const reason = document.getElementById('settings-exc-reason').value.trim();
  const ticket = document.getElementById('settings-exc-ticket').value.trim() || "";
  const owner = document.getElementById('settings-exc-owner').value.trim();
  const expires = document.getElementById('settings-exc-expires').value;
  
  if(!rule || !reason || !owner || !expires) {
     settingsShowError("Rule, Reason, Owner and Expiration are all required");
     return;
  }
  
  const unix = Math.floor(new Date(expires).getTime() / 1000);
  
  const res = await fetchJsonSafe(`/api/settings/exceptions/${kind}`, {
     method: 'POST',
     headers: {'Content-Type': 'application/json'},
     body: JSON.stringify({
        ago_path: target === "" ? null : target,
        rule_path: rule,
        reason: reason,
        ticket_ref: ticket === "" ? null : ticket,
        owner: owner,
        expires_at_unix: unix
     })
  });
  
  if(isErrorResponse(res)) {
     settingsShowError("Failed to add exception: " + (res.error || 'unknown error'), res);
     return;
  }
  
  settingsSetStatus(res, 'success');
  document.getElementById('settings-exc-rule').value = "";
  document.getElementById('settings-exc-reason').value = "";
  document.getElementById('settings-exc-ticket').value = "";
  settingsLoadExceptions();
}

async function settingsComplianceScan() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue() || "";
  
  settingsSetStatus("Running compliance scan...", "info");
  
  const res = await fetchJsonSafe(`/api/settings/compliance_scan`, {
     method: 'POST',
     headers: {'Content-Type': 'application/json'},
     body: JSON.stringify({
        ago_path: target === "" ? null : target,
        kind: kind
     })
  });
  
  if(isErrorResponse(res)) {
     settingsShowError("Scan failed: " + (res.error || 'unknown error'), res);
     return;
  }
  
  settingsSetStatus(res, 'success');
}

async function settingsExploreDecisions() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const limit = 50;
  
  settingsSetStatus("Loading recent decisions...", "info");
  
  const res = await fetchJsonSafe(`/api/settings/decisions?kind=${kind}&limit=${limit}`);
  
  if(isErrorResponse(res)) {
     settingsShowError("Failed to fetch decisions: " + (res.error || 'unknown error'), res);
     return;
  }
  
  if (res.decisions && res.decisions.length === 0) {
     settingsSetStatus("No decisions found for this policy kind.", "warn");
     return;
  }
  settingsSetStatus(res, 'success');
}

async function settingsResolvePolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = settingsTargetValue();
  
  if(!target) {
      settingsShowError("Target (repo path) is required to resolve policy");
      return;
  }
  
  settingsSetStatus(`Resolving policy for ${target}...`, "info");
  
  const res = await fetchJsonSafe(`/api/settings/policy/resolve`, {
     method: 'POST',
     headers: {'Content-Type': 'application/json'},
     body: JSON.stringify({
        repo_path: target,
        kind: kind
     })
  });
  
  if(isErrorResponse(res)) {
     settingsShowError("Resolve failed: " + (res.error || 'unknown error'), res);
     return;
  }
  
  settingsSetStatus(res, 'success');
}


// ─────────────────────────────────────────────────────────────────────────────
// P4: Conflict Radar — explicit pre-sync/pre-merge conflict detection panel
// ─────────────────────────────────────────────────────────────────────────────

async function branchConflictRadarRun() {
  const branch = (document.getElementById('branch-radar-input').value || '').trim();
  const base = (document.getElementById('branch-radar-base').value || 'main').trim();
  const chip = document.getElementById('branch-radar-chip');
  const results = document.getElementById('branch-radar-results');
  if (!results) return;

  if (!branch) {
    if (chip) setChipState(chip, 'Conflict Radar', 'failed', 'branch required');
    results.innerHTML = `<div class="warn">Branch name is required.</div>`;
    return;
  }

  if (chip) setChipState(chip, 'Conflict Radar', 'running', 'scanning...');
  results.innerHTML = `<div class="muted">Scanning for conflicts…</div>`;

  try {
    const res = await fetch('/api/branch/conflict-radar', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ branch, base_branch: base })
    });
    const data = await res.json();

    if (!data.ok) {
      if (chip) setChipState(chip, 'Conflict Radar', 'failed', 'error');
      results.innerHTML = `<div class="warn">Conflict radar failed: ${data.error || 'unknown error'}</div>`;
      return;
    }

    const hasConflicts = data.has_conflicts;
    const statusColor = hasConflicts ? 'var(--color-failed)' : 'var(--color-ok)';
    const statusLabel = hasConflicts
      ? `⚠️ ${data.conflict_count} repo(s) have conflicts`
      : `✅ No conflicts detected`;

    if (chip) setChipState(chip, 'Conflict Radar', hasConflicts ? 'failed' : 'success', `${data.conflict_count} conflicts`);

    const rows = (data.results || []).map(r => {
      const conflictBadge = r.has_conflicts
        ? `<span style="color:var(--color-failed)">⚠️ ${r.conflicting_files.length || 0} file(s)</span>`
        : `<span style="color:var(--color-ok)">✅ clean</span>`;
      const files = r.has_conflicts && r.conflicting_files.length
        ? `<div style="font-size:0.8em;color:var(--text-muted);margin-top:4px;">${r.conflicting_files.slice(0, 5).map(f => `• ${f}`).join('<br>')}</div>`
        : '';
      const errBadge = r.error ? `<span style="color:var(--color-warn)">⚠ ${r.error}</span>` : '';
      return `
        <tr>
          <td style="font-family:var(--font-mono)">${r.repo}</td>
          <td>${conflictBadge}${files}</td>
          <td>${r.ahead || '?'} ↑ / ${r.behind || '?'} ↓</td>
          <td>${errBadge}</td>
        </tr>`;
    }).join('');

    results.innerHTML = `
      <div style="margin-bottom:8px;font-weight:600;color:${statusColor}">${statusLabel}</div>
      <div style="font-size:0.85em;color:var(--text-muted);margin-bottom:8px;">
        ${data.repo_count} repos scanned · branch: <code>${branch}</code> vs <code>${base}</code>
      </div>
      ${rows ? `<table style="width:100%;border-collapse:collapse;font-size:0.9em;">
        <thead><tr>
          <th style="text-align:left;padding:4px 8px;border-bottom:1px solid var(--border)">Repo</th>
          <th style="text-align:left;padding:4px 8px;border-bottom:1px solid var(--border)">Status</th>
          <th style="text-align:left;padding:4px 8px;border-bottom:1px solid var(--border)">Ahead/Behind</th>
          <th style="text-align:left;padding:4px 8px;border-bottom:1px solid var(--border)">Error</th>
        </tr></thead>
        <tbody>${rows}</tbody>
      </table>` : '<div class="muted">No repo results returned.</div>'}`;

  } catch (err) {
    if (chip) setChipState(chip, 'Conflict Radar', 'failed', 'error');
    results.innerHTML = `<div class="warn">Conflict radar error: ${err.message}</div>`;
  }
}

async function branchTimelineLoad(offset = 0) {
  const btn = document.getElementById('branch-timeline-refresh-btn');
  const list = document.getElementById('branch-timeline-list');
  if (!list) return;
  if (btn) setButtonBusy(btn, true, 'Loading...');

  try {
    const res = await fetch(`/api/branch/timeline?offset=${offset}&limit=50`);
    const data = await res.json();
    if (!data.ok) {
      list.innerHTML = `<div class="warn">Failed to load timeline: ${data.error || 'Unknown error'}</div>`;
      return;
    }

    if (!data.events || data.events.length === 0) {
      list.innerHTML = `<div class="muted">No timeline events found for this scope.</div>`;
      return;
    }

    list.innerHTML = data.events.map((ev, idx) => {
      const isOk = ev.success;
      const statusIcon = isOk ? '✅' : '❌';
      const action = ev.action.toUpperCase();
      const undoBadge = ev.undo_entry_ids && ev.undo_entry_ids.length > 0
        ? `<span class="badge" style="background:var(--color-warn);color:#000;font-size:0.75em;margin-left:8px;">undoable</span>`
        : '';
      const dryRunBadge = ev.dry_run ? `<span class="badge neutral" style="font-size:0.75em;margin-left:8px;">dry-run</span>` : '';
      const repos = Array.isArray(ev.repos) ? ev.repos.join(', ') : 'N/A';
      const detailStr = (ev.details && ev.details.response_summary && ev.details.response_summary.error)
        ? `<div style="color:var(--color-failed); font-size: 0.85em; margin-top: 4px;">Error: ${ev.details.response_summary.error}</div>`
        : '';
      // Per-event drill-down: truncate at 2000 chars to prevent oversized render (G-007)
      const detailJson = JSON.stringify(ev.details || {}, null, 2);
      const detailTruncated = detailJson.length > 2000
        ? detailJson.slice(0, 2000) + '\n... [truncated]'
        : detailJson;
      const drillId = `btl-detail-${offset}-${idx}`;

      return `
        <div style="border-bottom: 1px solid var(--border); padding: 8px 0; font-family: var(--font-mono);">
          <div style="cursor:pointer;" onclick="const el=document.getElementById('${drillId}');el.style.display=el.style.display==='none'?'block':'none';">
             <span style="color:var(--text-muted); font-size:0.85em;">[${new Date(ev.timestamp).toLocaleString()}]</span>
             ${statusIcon} <strong>${action}</strong> on <em>${ev.branch || ev.domain || ''}</em>
             ${dryRunBadge} ${undoBadge}
             <span style="color:var(--text-muted);font-size:0.75em;margin-left:8px;">▸ details</span>
          </div>
          <div style="font-size: 0.9em; margin-top: 4px;">
            <span style="color:var(--text-muted);">Repos (${ev.repo_count}):</span> ${repos}
          </div>
          ${detailStr}
          <pre id="${drillId}" style="display:none;background:rgba(0,0,0,0.3);padding:8px;border-radius:4px;font-size:0.78em;overflow-x:auto;white-space:pre-wrap;margin-top:6px;">${detailTruncated.replace(/</g,'&lt;').replace(/>/g,'&gt;')}</pre>
        </div>
      `;
    }).join('');

  } catch (err) {
    list.innerHTML = `<div class="warn">Failed to load timeline: ${err.message}</div>`;
  } finally {
    if (btn) setButtonBusy(btn, false, 'Refresh Timeline');
  }
}

async function branchUndoJournalLoad() {
  const chip = document.getElementById('branch-undo-chip');
  const tbody = document.getElementById('branch-undo-body');
  if (!tbody) return;

  setChipState(chip, 'Undo Journal', 'running', 'loading');

  try {
    const res = await fetch(`/api/branch/undo-journal?limit=20`);
    const data = await res.json();
    if (!data.ok) {
      setChipState(chip, 'Undo Journal', 'failed', 'failed');
      tbody.innerHTML = `<tr><td colspan="7" class="warn">Failed to load journal: ${data.error}</td></tr>`;
      return;
    }

    if (!data.entries || data.entries.length === 0) {
      setChipState(chip, 'Undo Journal', 'success', 'empty');
      tbody.innerHTML = `<tr><td colspan="7" class="muted">No undo entries found.</td></tr>`;
      return;
    }

    setChipState(chip, 'Undo Journal', 'success', `${data.entries.length} entries`);

    tbody.innerHTML = data.entries.map(ev => {
      const time = new Date(ev.timestamp).toLocaleString();
      const isUndone = ev.undone;
      const statusClass = isUndone ? 'muted' : '';
      const statusText = isUndone ? 'Reverted' : 'Active';
      const actionHtml = isUndone 
        ? `<button class="btn secondary" disabled>Undone</button>`
        : `<button class="btn secondary" onclick="branchUndoExecute('${ev.id}', true)">Preview Undo</button>`;

      return `
        <tr class="${statusClass}">
          <td>${time}</td>
          <td>${ev.action}</td>
          <td>${ev.repo}</td>
          <td>${ev.branch_name}</td>
          <td style="font-family: monospace; font-size: 0.85em;">${ev.prior_ref.substring(0, 8)}</td>
          <td>${statusText}</td>
          <td>${actionHtml}</td>
        </tr>
      `;
    }).join('');

  } catch (err) {
    setChipState(chip, 'Undo Journal', 'failed', 'error');
    tbody.innerHTML = `<tr><td colspan="7" class="warn">Failed to load journal: ${err.message}</td></tr>`;
  }
}

let pendingUndoId = null;

async function branchUndoExecute(id, dryRun) {
  const chip = document.getElementById('branch-undo-chip');
  const errorContainer = document.getElementById('branch-undo-body').parentElement;
  
  if (!dryRun && pendingUndoId !== id) {
     showInlineError("Please preview before executing.", errorContainer);
     return;
  }
  
  if (dryRun) {
     const confirmPrompt = confirm("Previewing undo. After clicking OK, a real 'Execute Undo' button will replace this if the preview is successful.");
     if (!confirmPrompt) return;
  } else {
     const confirmPrompt = confirm("Are you sure you want to revert this branch mutation? This is a destructive operation.");
     if (!confirmPrompt) return;
  }

  setChipState(chip, 'Undo', 'running', dryRun ? 'previewing' : 'executing');

  try {
    const res = await fetch('/api/branch/undo', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify({ entry_id: id, dry_run: dryRun })
    });
    const data = await res.json();
    if (!data.ok) {
       setChipState(chip, 'Undo', 'failed', 'failed');
       showInlineError(`Undo failed:\n${JSON.stringify(data.outcome, null, 2)}`, errorContainer);
       return;
    }

    if (dryRun) {
       pendingUndoId = id;
       setChipState(chip, 'Undo', 'success', 'preview ok');
       // Morph the button for the specific row (quick hack since we don't hold the row reference strictly)
       branchUndoJournalLoad().then(() => {
          setTimeout(() => {
             const rows = document.getElementById('branch-undo-body').querySelectorAll('tr');
             rows.forEach(r => {
                const btn = r.querySelector('button');
                if (btn && btn.getAttribute('onclick') === `branchUndoExecute('${id}', true)`) {
                   btn.textContent = "EXECUTE UNDO";
                   btn.className = "btn"; // Make it primary
                   btn.setAttribute('onclick', `branchUndoExecute('${id}', false)`);
                }
             });
          }, 100);
       });
       logActivity("Undo Preview Successful", { ref: data.outcome.new_ref || data.outcome.prior_ref, message: data.outcome.message });
    } else {
       pendingUndoId = null;
       setChipState(chip, 'Undo', 'success', 'reverted');
       branchUndoJournalLoad();
       branchTimelineLoad();
       logActivity("Undo Successfully Executed", { entry_id: id });
    }

  } catch (err) {
    setChipState(chip, 'Undo', 'failed', 'error');
    showInlineError(`Undo failed: ${err.message}`, errorContainer);
  }
}

async function unifiedTimelineLoad() {
  const domainNode = document.getElementById('dash-timeline-domain');
  if(!domainNode) return;
  const domain = domainNode.value;
  let q = '?limit=50';
  if (domain) q += '&domain=' + encodeURIComponent(domain);
  try {
    const res = await fetch('/api/orchestrate/timeline' + q);
    const data = await res.json();
    const out = document.getElementById('dash-timeline-out');
    if (data.events && data.events.length > 0) {
      const lines = data.events.map(e => `[${new Date(e.timestamp).toLocaleTimeString()}] ${e.domain.toUpperCase()}: ${e.action} (dry_run=${e.dry_run}) -> success=${e.success} (${e.summary})`);
      out.textContent = lines.join('\n');
    } else {
      out.textContent = 'No events found in timeline.';
    }
  } catch(e) {
    const out = document.getElementById('dash-timeline-out');
    if(out) out.textContent = 'Error loading timeline: ' + e;
  }
}

async function dashVerifyEvidence() {
  const pathNode = document.getElementById('dash-verify-path');
  const out = document.getElementById('dash-verify-out');
  const path = pathNode ? pathNode.value.trim() : '';
  
  if (!path) {
    out.setAttribute('role', 'alert');
    out.innerHTML = '<span class="term-err">ERROR: Path cannot be empty</span><br><span class="term-dim">Mitigation: Please provide a valid file path to an evidence bundle.</span>';
    return;
  }

  const req = { path };
  out.innerHTML = '<span class="term-sys">Verifying...</span>';
  
  try {
    const res = await fetch('/api/evidence/verify', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify(req)
    });
    const data = await res.json();
    
    if (data.is_valid) {
      out.innerHTML = `<span class="term-ok">✓ BUNDLE SIGNATURE OK</span>
<span class="term-sys">Path:</span> ${path}`;
      return;
    }

    // Map strict taxonomy to actionable UI
    let hint = '';
    if (data.reason_code === 'missing_file') {
      hint = 'Mitigation: Ensure the bundle and all referenced artifacts exist at the correct paths.';
    } else if (data.reason_code === 'hash_mismatch') {
      hint = 'Mitigation: A file was changed after export. Review the offending path for tampering.';
    } else if (data.reason_code === 'schema_error') {
      hint = 'Mitigation: The manifest JSON is structurally invalid or missing required keys.';
    } else if (data.reason_code === 'parse_error') {
      hint = 'Mitigation: The bundle file is corrupt and cannot be parsed as JSON.';
    } else if (data.reason_code === 'chain_mismatch') {
      hint = 'Mitigation: The internal chain integrity state was recorded as invalid at export time.';
    } else {
      hint = 'Mitigation: Unknown error occurred.';
    }

    out.setAttribute('role', 'alert');
    out.innerHTML = `<span class="term-err">✗ VERIFICATION FAILED</span>
<span class="term-sys">Reason Code:</span> <span class="term-err">${data.reason_code}</span>
<span class="term-sys">Details:</span>     ${data.details}` + 
(data.offending_path ? `\n<span class="term-sys">Offending:</span>   <span class="term-warn">${data.offending_path}</span>` : '') +
`\n\n<span class="term-dim">${hint}</span>`;

  } catch (err) {
    out.setAttribute('role', 'alert');
    out.innerHTML = `<span class="term-err">✗ Verification request failed: ${err}</span><br><span class="term-dim">Mitigation: Verify the Pilot backend is running and accessible.</span>`;
  }
}

// Global keydown handler for spans acting as buttons
document.addEventListener('keydown', function(event) {
  // Check if the focused element is a span with role="button"
  if (document.activeElement && document.activeElement.tagName === 'SPAN' && document.activeElement.getAttribute('role') === 'button') {
    // If Enter or Space is pressed, trigger the click event
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault(); // Prevent default scroll behavior for Space
      document.activeElement.click();
    }
  }
});

function startHealthHeartbeat() {
  setInterval(async () => {
    try {
      const res = await fetchJsonSafe('/api/health');
      if (res) {
        // Support both legacy flat shape and current nested shape.
        const busRunning =
          typeof res.bus_running === 'boolean'
            ? res.bus_running
            : !!(res.bus && res.bus.running);
        
        let busNoteSuffix = '';
        if (res.bus && res.bus.note && res.bus.note.trim() !== '') {
            busNoteSuffix = res.bus.note;
        }
        
        const dbRunning =
          typeof res.db_running === 'boolean'
            ? res.db_running
            : !!(res.db && res.db.running);
            
        let dbState = dbRunning ? 'RUNNING' : 'STOPPED';
        let dbNote = '';
        if (res.db) {
            if (res.db.state) dbState = res.db.state;
            if (res.db.note) dbNote = res.db.note;
        }

        const latency = res.bus && typeof res.bus.latency_ms === 'number' ? `${res.bus.latency_ms}ms` : '';
        const meta = res.uptime_secs ? `Uptime: ${res.uptime_secs}s` : '';
        const healthParts = [meta, latency];
        if (busNoteSuffix) healthParts.push(`Bus: ${busNoteSuffix}`);
        const note = healthParts.filter(Boolean).join(' | ') || 'OK';
        
        setBusStatus(busRunning, note);
        if (dashDbChip) {
            let label = 'DB: ' + dbState;
            if (dbNote) {
                // Ensure note shows up in UI
                dashDbChip.setAttribute('title', dbNote);
                if (dbState !== 'RUNNING') label += ' (Error)';
            }
            setChipState(dashDbChip, label, dbRunning ? 'success' : 'failed');
        }
      } else {
        setBusStatus(false, 'Heartbeat Failed');
        if (dashDbChip) setChipState(dashDbChip, 'DB: UNKNOWN', 'failed');
      }
    } catch (e) {
      setBusStatus(false, e.message);
      if (dashDbChip) setChipState(dashDbChip, 'DB: ERROR', 'failed');
    }
  }, 5000);
}

// -----------------------------------------------------------------------------
// OVERRIDE REGISTRY & FLEET GOVERNANCE HEALTH SCAN
// -----------------------------------------------------------------------------

async function settingsLoadOverrides() {
  const kind = document.getElementById('settings-override-kind').value;
  try {
    const res = await fetch(`/api/settings/overrides/${encodeURIComponent(kind)}`);
    const data = await res.json();
    const tbody = document.querySelector('#settings-overrides-table tbody');
    if (!data.ok) {
      tbody.innerHTML = `<tr><td colspan="5" style="color:var(--rose)">Error: ${data.message || data.error}</td></tr>`;
      return;
    }
    
    if (!data.overrides || data.overrides.length === 0) {
      tbody.innerHTML = `<tr><td colspan="5" style="text-align:center; color:var(--muted)">No overrides active for ${kind}</td></tr>`;
      return;
    }

    tbody.innerHTML = '';
    for (const ov of data.overrides) {
      const tr = document.createElement('tr');
      
      const tdTarget = document.createElement('td');
      tdTarget.textContent = ov.ago_path;
      tr.appendChild(tdTarget);

      const tdOwner = document.createElement('td');
      tdOwner.textContent = ov.owner_identity;
      tr.appendChild(tdOwner);

      const tdReason = document.createElement('td');
      tdReason.textContent = ov.reason;
      if (ov.ticket_ref) {
        tdReason.textContent += ` (${ov.ticket_ref})`;
      }
      tr.appendChild(tdReason);

      const tdExpires = document.createElement('td');
      if (ov.expires_at) {
        tdExpires.textContent = new Date(ov.expires_at).toLocaleString();
      } else {
        tdExpires.textContent = "Never";
        tdExpires.style.color = 'var(--muted)';
      }
      tr.appendChild(tdExpires);

      const tdActions = document.createElement('td');
      const btnRevoke = document.createElement('button');
      btnRevoke.className = 'btn secondary';
      btnRevoke.style.padding = '4px 8px';
      btnRevoke.textContent = 'Revoke';
      btnRevoke.onclick = () => settingsRevokeOverride(kind, ov.ago_path);
      tdActions.appendChild(btnRevoke);
      tr.appendChild(tdActions);

      tbody.appendChild(tr);
    }
  } catch (err) {
    console.error("Failed to load overrides", err);
  }
}

async function settingsCreateOverride() {
  const kind = document.getElementById('settings-override-kind').value;
  const target = document.getElementById('settings-override-target').value.trim();
  const reason = document.getElementById('settings-override-reason').value.trim();
  const ticket_ref = document.getElementById('settings-override-ticket').value.trim();
  const expiresRaw = document.getElementById('settings-override-expires').value;
  const policyStr = document.getElementById('settings-override-json').value.trim();

  if (!target || !reason || !policyStr) {
    alert("Target AGO, Reason, and Policy JSON are required.");
    return;
  }

  let policy_json;
  try {
    policy_json = JSON.parse(policyStr);
  } catch (e) {
    alert("Invalid Policy JSON: " + e.message);
    return;
  }

  let expires_at = null;
  if (expiresRaw) {
    expires_at = new Date(expiresRaw).toISOString();
  }

  const payload = {
    ago_path: target,
    reason: reason,
    policy_json: policy_json
  };
  
  if (ticket_ref) payload.ticket_ref = ticket_ref;
  if (expires_at) payload.expires_at = expires_at;

  try {
    const res = await fetch(`/api/settings/overrides/${encodeURIComponent(kind)}`, {
      method: 'POST',
      headers: {'content-type': 'application/json'},
      body: JSON.stringify(payload)
    });
    const data = await res.json();
    if (data.ok) {
      // clear form
      document.getElementById('settings-override-target').value = '';
      document.getElementById('settings-override-reason').value = '';
      document.getElementById('settings-override-ticket').value = '';
      document.getElementById('settings-override-expires').value = '';
      settingsLoadOverrides();
    } else {
      alert("Error: " + (data.message || data.error || "Failed to create override"));
    }
  } catch (err) {
    alert("Error: " + err.message);
  }
}

async function settingsRevokeOverride(kind, agoPath) {
  if (!confirm(`Are you sure you want to revoke the ${kind} override for ${agoPath}?`)) {
    return;
  }
  
  try {
    const res = await fetch(`/api/settings/overrides/delete/${encodeURIComponent(kind)}/${encodeURIComponent(agoPath)}`, {
      method: 'POST'
    });
    const data = await res.json();
    if (data.ok) {
      settingsLoadOverrides();
    } else {
      alert("Error revoking override: " + (data.message || data.error));
    }
  } catch (err) {
    alert("Error: " + err.message);
  }
}

async function settingsResolveTrace() {
  const kind = document.getElementById('settings-override-kind').value;
  const target = document.getElementById('settings-override-target').value.trim();
  if (!target) {
    alert("Please enter a Target AGO path to resolve its trace.");
    return;
  }

  try {
    const res = await fetch(`/api/settings/policy/resolve_trace`, {
      method: 'POST',
      headers: {'content-type': 'application/json'},
      body: JSON.stringify({ policy_kind: kind, ago_path: target })
    });
    const data = await res.json();
    const out = document.getElementById('settings-status-out');
    if (data.ok) {
      out.textContent = `TRACE FOR ${kind.toUpperCase()} @ ${target}:\n` + JSON.stringify(data.trace, null, 2);
    } else {
      out.textContent = "Error: " + (data.message || data.error);
    }
  } catch (err) {
    document.getElementById('settings-status-out').textContent = "Error: " + err.message;
  }
}

async function settingsGovernanceScan() {
  const tbody = document.querySelector('#settings-gov-scan-table tbody');
  tbody.innerHTML = `<tr><td colspan="9" style="text-align:center; color:var(--muted)">Scanning fleet...</td></tr>`;
  
  const totChip = document.getElementById('gov-scan-total-chip');
  const violChip = document.getElementById('gov-scan-violations-chip');
  setChipState(totChip, "Scanned: running...", "running");
  setChipState(violChip, "Violations: computing...", "running");

  try {
    const res = await fetch(`/api/settings/governance_scan`, { method: 'POST' });
    const data = await res.json();
    
    if (!data.ok) {
      tbody.innerHTML = `<tr><td colspan="9" style="color:var(--rose)">Error: ${data.message || data.error}</td></tr>`;
      setChipState(totChip, "Scanned: Error", "failed");
      setChipState(violChip, "Violations: Error", "failed");
      return;
    }

    const rep = data.report;
    setChipState(totChip, `Scanned: ${rep.agos_scanned}`, "success");
    setChipState(violChip, `Violations: ${rep.total_violations}`, rep.total_violations > 0 ? "failed" : "success");

    if (!rep.ago_statuses || rep.ago_statuses.length === 0) {
      tbody.innerHTML = `<tr><td colspan="9" style="text-align:center; color:var(--muted)">No AGOs returned by scan.</td></tr>`;
      return;
    }

    tbody.innerHTML = '';
    for (const st of rep.ago_statuses) {
      const tr = document.createElement('tr');
      
      const tdTarget = document.createElement('td');
      tdTarget.textContent = st.ago_path;
      tr.appendChild(tdTarget);

      const tdOverall = document.createElement('td');
      const spanOverall = document.createElement('span');
      spanOverall.className = 'chip ' + (st.overall_status === 'compliant' ? 'ok' : (st.overall_status === 'violation' ? 'fail' : 'warn'));
      spanOverall.textContent = st.overall_status.toUpperCase();
      tdOverall.appendChild(spanOverall);
      tr.appendChild(tdOverall);

      const families = ['branch', 'dependency', 'release', 'security', 'quality', 'runtime'];
      for (const fam of families) {
        const td = document.createElement('td');
        const ev = st.evaluations[fam];
        if (ev) {
          if (ev.blocked || (ev.violations && ev.violations.length > 0)) {
             td.textContent = '❌ Fail';
             td.style.color = 'var(--rose)';
          } else if (ev.warnings && ev.warnings.length > 0) {
             td.textContent = '⚠️ Warn';
             td.style.color = 'var(--accent)';
          } else {
             td.textContent = '✅ Pass';
             td.style.color = 'var(--primary)';
          }
        } else {
          td.textContent = '-';
          td.style.color = 'var(--dim)';
        }
        tr.appendChild(td);
      }

      const tdOverrides = document.createElement('td');
      if (st.is_overridden) {
         tdOverrides.textContent = "Has Overrides";
         tdOverrides.style.color = 'var(--accent)';
      } else {
         tdOverrides.textContent = 'None';
         tdOverrides.style.color = 'var(--muted)';
      }
      tr.appendChild(tdOverrides);

      tbody.appendChild(tr);
    }

  } catch (err) {
    tbody.innerHTML = `<tr><td colspan="9" style="color:var(--rose)">Exception during scan: ${err.message}</td></tr>`;
    setChipState(totChip, "Scanned: Exception", "failed");
    setChipState(violChip, "Violations: Exception", "failed");
  }
}

async function settingsExportGovernanceReport() {
  const activeId = await getActiveAgorgId();
  if (!activeId) {
    logActivity("Export Governance", { ok: false, error: 'No active AGOrg scope selected' });
    return;
  }
  
  try {
    const data = await fetchJsonSafe('/api/agorg/policy_report', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify({ agorg: activeId })
    });
    
    if (data && data.ok) {
      logActivity("Governance Report Exported", { ok: true, artifact_path: data.artifact_path });
      appendLive({ source: 'agorg_policy', action: 'export', artifact_path: data.artifact_path });
      
      // If we have an output region in settings, show it there too
      const out = document.getElementById('settings-status-out');
      if (out) {
        out.textContent = `Governance report successfully persisted to:\n${data.artifact_path}\n\nYou can access this file directly on the host system.`;
      }
    } else {
      logActivity("Export Governance Failed", data);
    }
  } catch (err) {
    logActivity("Export Governance Error", { ok: false, error: err.message });
  }
}

// P5: Cross-Tab Command Graph Orchestration State
let p5RailState = {
  active_operation_id: null,
  active_scope_id: null
};

function p5ResetRailState() {
  p5RailState.active_operation_id = null;
  p5RailState.active_scope_id = null;
  // Reset all chips in the rail strip to neutral
  const strip = document.getElementById('p5-rail-strip');
  if (strip) {
    strip.querySelectorAll('.chip').forEach(chip => {
      chip.className = 'chip neutral';
    });
  }
}

async function p5OrchestrateStep(domain, action, dryRun) {
  // If no active scope, or if scope changed, reset rail state to fresh lineage
  const currentScope = window.AGORG_ACTIVE_ID || 'none';
  if (p5RailState.active_scope_id !== currentScope) {
    p5ResetRailState();
    p5RailState.active_scope_id = currentScope;
  }

  // Derive chip ID based on action
  let chipId = 'p5-chip-status';
  if (action === 'status') chipId = 'p5-chip-status';
  else if (action === 'bus-status') chipId = 'p5-chip-bus';
  else if (action === 'heal.plan') chipId = 'p5-chip-heal-plan';
  else if (action === 'heal.run') chipId = 'p5-chip-heal-run';
  else if (action === 'push.safe') chipId = 'p5-chip-push';
  else if (action === 'matrix') chipId = 'p5-chip-branch';
  else if (action === 'multi.status') chipId = 'p5-chip-multi';
  else if (action === 'dag.evaluate') chipId = 'p5-chip-dag';
  else if (action === 'reconcile') chipId = 'p5-chip-apply';
  
  const chip = document.getElementById(chipId);
  if (chip) chip.className = 'chip warn'; // visually running

  const parseTagsCsv = (s) => (s || '')
    .split(',')
    .map(v => v.trim())
    .filter(Boolean);
  const payload = { action, dry_run: dryRun };
  if (domain === 'dependency' && action === 'push') {
    const { branch, remote } = routineReadBranchRemote();
    payload.branch = branch;
    payload.remote = remote;
  }
  if (domain === 'command') {
    const group = (document.getElementById('multi-group').value || document.getElementById('branch-matrix-group').value || '').trim();
    const tags = parseTagsCsv((document.getElementById('multi-tags').value || document.getElementById('branch-matrix-tags').value || ''));
    if (group) payload.group = group;
    if (tags.length) payload.tags = tags;
    if (action === 'multi.apply') {
      payload.branch = (document.getElementById('branch-apply-branch').value || '').trim() || 'feat/pilot-wave13';
      payload.base_branch = (document.getElementById('branch-apply-base').value || '').trim() || 'dev';
      payload.pr_base_branch = (document.getElementById('branch-apply-pr-base').value || '').trim() || 'main';
      const stageSizeRaw = (document.getElementById('branch-apply-stage-size').value || '2').trim();
      const stageSize = Number.parseInt(stageSizeRaw, 10);
      if (Number.isFinite(stageSize) && stageSize > 0) payload.stage_size = stageSize;
      payload.continue_on_failure = !!document.getElementById('branch-apply-continue').checked;
    }
  }

  try {
    const res = await fetch('/api/orchestrate/run', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        domain,
        payload
      })
    });
    const data = await res.json();
    
    // Save operation_id for UI linkage / lineage continuity (generated by server)
    if (data.operation_id) {
      p5RailState.active_operation_id = data.operation_id;
    }

    if (chip) {
      if (data.stage === 'preview') {
        chip.className = 'chip accent'; // preview uses accent color
      } else {
        chip.className = data.ok ? 'chip ok' : 'chip fail';
      }
    }
    
    // Refresh unified timeline if in view
    unifiedTimelineLoad();

  } catch (err) {
    console.error("P5 Orchestrate error", err);
    if (chip) chip.className = 'chip fail';
  }
}

bootUi();
startHealthHeartbeat();
