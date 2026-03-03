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
let branchPreviewTokens = { create: null, sync: null, prune: null };
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
  }
  if (tabName === 'branch') {
    branchLoadMatrix();
  }
  if (tabName === 'settings') {
    settingsLoadPolicy();
    settingsLoadExceptions();
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

function collectUiSessionState() {
  const subTabs = {};
  document.querySelectorAll('.card .sub-tabs').forEach((container, idx) => {
    const activeBtn = container.querySelector('.sub-tab.active');
    const panelId = activeBtn ? (activeBtn.getAttribute('onclick') || '') : '';
    subTabs['group_' + idx] = panelId;
  });
  return {
    active_tab: currentTab,
    agorg_use_id: readInputValue('agorg-use-id'),
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
  setVal('agorg-use-id', session.agorg_use_id);
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
      html += '<div class="agorg-drop-header">AGOrgs</div>';
      agData.forEach(ag => {
        const badge = ag.id === activeId ? 'ACTIVE' : (recentIds.has(ag.id) ? 'RECENT' : 'ORG');
        html += `<div class="agorg-drop-item" onclick="switchAgorgScope('${ag.id}')">
          <span>${ag.name}</span>
          <span class="type">${badge}</span>
        </div>`;
      });
    }

    // Attempt to list AGOs if available in the database
    const treeRes = await fetch('/api/agorg/tree');
    const treeDataRaw = treeRes.ok ? await treeRes.json() : { ok: false, tree: [] };
    const treeData = treeDataRaw && treeDataRaw.ok && Array.isArray(treeDataRaw.tree)
      ? treeDataRaw.tree
      : [];
    if (treeData.length > 0) {
      html += '<div class="agorg-drop-header">Sibling AGOs (Active Tree)</div>';
      const agos = [];
      const walk = (node) => {
        (node.agos || []).forEach(a => agos.push(a));
        (node.child_agorgs || []).forEach(walk);
      };
      treeData.forEach(walk);
      // Remove duplicates by ID
      const seen = new Set();
      const uniqueAgos = agos.filter(a => {
        if (seen.has(a.id)) return false;
        seen.add(a.id);
        return true;
      });
      uniqueAgos.forEach(ago => {
        html += `<div class="agorg-drop-item" onclick="switchAgorgScope('${ago.id}')">
          <span>${ago.name}</span>
          <span class="type">AGO</span>
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
  if (currentTab === 'agorg') {
    agorgTree();
    agorgShowActive();
  }
  if (currentTab === 'branch') {
    await branchLoadMatrix();
  }
  queueUiSessionSave();
  document.getElementById('agorg-hero-dropdown').classList.remove('active');
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
function showInlineError(message, containerEl = null) {
  const targetEl = containerEl || out;
  const errorDiv = document.createElement('div');
  errorDiv.className = 'error-message';
  errorDiv.setAttribute('role', 'alert');
  errorDiv.setAttribute('aria-live', 'assertive');
  errorDiv.innerHTML = '<strong>Error:</strong> ' + message + '<br><small>Please try again or adjust your settings.</small>';
  // Clear previous error and insert new one
  const existing = targetEl.querySelector('.error-message');
  if (existing) existing.remove();
  targetEl.insertBefore(errorDiv, targetEl.firstChild);
  // Also log the error for visibility
  console.error('[Error]', message);
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
    const timeoutId = setTimeout(() => ctl.abort(), 25000);
    const res = await fetch('/api/command', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify({ command, payload }),
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
      ? 'Request timed out. Check ArqonBus bridge health and try again.'
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
  const group = (document.getElementById('branch-matrix-group')?.value || '').trim();
  const tagRaw = (document.getElementById('branch-matrix-tags')?.value || '').trim();
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
    search: (document.getElementById('branch-matrix-search')?.value || '').trim() || null,
    base_branch: (document.getElementById('branch-matrix-base')?.value || document.getElementById('sync-base').value || 'main').trim() || 'main',
    target_branch: (document.getElementById('branch-name').value || '').trim() || null
  };
}

function refreshBranchPreviewState() {
  if (!branchPreviewState) return;
  const parts = [];
  for (const action of ['create', 'sync', 'prune']) {
    if (branchPreviewTokens[action]) parts.push(`${action}=ready`);
  }
  branchPreviewState.textContent = parts.length
    ? ('Preview tokens ready: ' + parts.join(', '))
    : 'No active preview token.';
}

function invalidateBranchPreviews(reason) {
  branchPreviewTokens = { create: null, sync: null, prune: null };
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
  } finally {
    if (branchMatrixRefreshBtn) setButtonBusy(branchMatrixRefreshBtn, false, null);
  }
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
    const res = await fetch('/api/branch/run', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify(payload)
    });
    const data = await res.json();
    const ok = !!data.ok;
    if (ok && isMutatingAction && payload.dry_run === true && data.preview_token) {
      branchPreviewTokens[action] = data.preview_token;
      refreshBranchPreviewState();
    } else if (ok && isExecute) {
      branchPreviewTokens[action] = null;
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

async function depRun(action) {
  const isPreflight = ['policy', 'hook-policy', 'drift', 'gate', 'push'].includes(action);
  const req = { action: isPreflight ? 'preflight' : action, json: false };
  if (isPreflight) {
    let step = action;
    if (step === 'hook-policy') step = 'hook';
    req.preflight_steps = [step];
  }
  if (action === 'push') {
    req.branch = document.getElementById('dash-push-branch').value || 'main';
    req.remote = document.getElementById('dash-push-remote').value || 'origin';
  }
  const res = await fetch('/api/dependencies/run', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  
  if (data.action === 'preflight' && data.report && Array.isArray(data.report.steps)) {
    for (const s of data.report.steps) {
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

  if (action.startsWith('bus-')) {
    const text = String(data.stdout || '') + '\n' + String(data.stderr || '');
    if (text.includes('RUNNING')) setBusStatus(true, 'bus shim reported RUNNING');
    if (text.includes('STOPPED')) setBusStatus(false, 'bus shim reported STOPPED');
  }
  if (action === 'services-status' || action === 'services-start' || action === 'services-stop' || action === 'services-restart') {
    if (typeof data.bus_running === 'boolean') {
      setBusStatus(data.bus_running, data.bus_running ? 'service manager reported RUNNING' : 'service manager reported STOPPED');
    }
  }
  depActionOut.textContent = JSON.stringify(data, null, 2);
  if (depActionOutGlobal) {
    depActionOutGlobal.textContent = JSON.stringify(data, null, 2);
  }
  if (!isPreflight) {
      updateDashChip(action, !!data.ok, data);
  }
  depLoadLogs();
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
  const req = {
    id: null, // Need to track selected ID
    name: document.getElementById('agorg-name').value.trim(),
    root: document.getElementById('agorg-root').value.trim(),
    master: document.getElementById('agorg-master').value.trim()
  };
  // Finding the ID from some state or active
  const activeRes = await fetch('/api/agorg/active');
  const activeData = await activeRes.json();
  if (!activeRes.ok || !activeData.active) {
     agorgOut.textContent = "Error: No active AGOrg selected for update";
     return;
  }
  req.id = activeData.active.id;
  
  const res = await fetch('/api/agorg/update', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  agorgOut.textContent = JSON.stringify(data, null, 2);
  refreshAgorgHeader();
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

function renderAgorgRegistry(agorgs) {
  if (!agorgRegistryList) return;
  agorgRegistryList.innerHTML = '';
  if (!agorgs || agorgs.length === 0) {
    agorgRegistryList.innerHTML = '<div style="padding:10px; color:#4e6ba6; font-size:0.8rem;">No registered AGOrgs found.</div>';
    return;
  }

  // Group by master_path
  const grouped = {};
  agorgs.forEach(ag => {
     const path = ag.master_path || 'Independent Orgs';
     if (!grouped[path]) grouped[path] = [];
     grouped[path].push(ag);
  });

  for (const [path, nodes] of Object.entries(grouped)) {
     if (path !== 'Independent Orgs') {
       const h = document.createElement('div');
       h.style = 'padding:8px 12px; background:#1c2635; color:#a8b9e3; font-size:0.75rem; font-weight:700; display:flex; align-items:center; gap:8px;';
       h.innerHTML = `<span>📁 Master:</span> <span style="font-family:monospace; opacity:0.8;">${path}</span>`;
       agorgRegistryList.appendChild(h);
     }
     
     nodes.forEach(node => {
        const el = document.createElement('div');
        el.className = 'agorg-reg-item' + (path !== 'Independent Orgs' ? ' agorg-tree-node' : '');
        el.style = 'display:flex; align-items:center; justify-content:space-between; padding:8px 12px; border-bottom:1px solid #1c2635; cursor:pointer; font-size:0.85rem;';
        el.innerHTML = `
          <div style="display:flex; align-items:center;">
            <span style="margin-right:8px; display:inline-flex; width:1.2rem;">🏢</span>
            <span style="font-weight:600;">${node.name}</span>
          </div>
          <span style="font-size:0.65rem; font-weight:700; padding:2px 4px; border-radius:3px; background:#1c2635; color:#a8b9e3;">ORG</span>
        `;
        el.onclick = () => switchAgorgScope(node.id);
        agorgRegistryList.appendChild(el);
     });
  }
}

async function agorgList() {
  if (agorgRegistryList) {
    agorgRegistryList.innerHTML = '<div style="padding:10px; color:#4e6ba6; font-size:0.8rem;">Loading backend properties...</div>';
  }
  try {
    const snapshot = await hydrateScopeSnapshot(true);
    const data = { ok: true, agorgs: snapshot.items || [] };
    const text = JSON.stringify(data, null, 2);
    if (agorgOut) agorgOut.textContent = text;
    renderAgorgRegistry(data.agorgs);

    // Attempt to mix in AGOs from the active tree to the registry view.
    const treeData = await fetchJsonSafe('/api/agorg/tree');
    if (treeData.ok && treeData.tree && treeData.tree.length > 0) {
      const agos = [];
      const walk = (node) => {
        (node.agos || []).forEach(a => agos.push(a));
        (node.child_agorgs || []).forEach(walk);
      };
      treeData.tree.forEach(walk);

      const seen = new Set();
      const uniqueAgos = agos.filter(a => {
        if (seen.has(a.id)) return false;
        seen.add(a.id);
        return true;
      });

      uniqueAgos.forEach(ago => {
        const el = document.createElement('div');
        el.className = 'agorg-reg-item agorg-tree-node';
        el.style = 'display:flex; align-items:center; justify-content:space-between; padding:8px 12px; border-bottom:1px solid #1c2635; cursor:pointer; font-size:0.85rem; margin-left:16px; border-left:1px solid #1c2635;';
        el.innerHTML = `
          <div style="display:flex; align-items:center;">
            <span style="margin-right:8px; display:inline-flex; width:1.2rem;">🤖</span>
            <span style="font-weight:600;">${ago.name}</span>
          </div>
          <span style="font-size:0.65rem; font-weight:700; padding:2px 4px; border-radius:3px; background:#1c2635; color:#6a7dff;">AGO</span>
        `;
        if (agorgRegistryList) agorgRegistryList.appendChild(el);
      });
    }
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
        <div style="margin-bottom:2px;"><strong>Master:</strong> <span style="font-family:monospace;">${data.active.master_path || 'None'}</span></div>
      `;
    }
    // Highlight active in registry
    Array.from(document.querySelectorAll('.agorg-reg-item')).forEach(el => el.classList.remove('active'));
  } else {
    if (agorgActiveDetails) {
      agorgActiveDetails.innerHTML = `<em>No active scope set</em>`;
    }
  }
  refreshAgorgHeader();
  await agorgLoadPreferences();
}

async function agorgUse() {
  const req = { agorg: document.getElementById('agorg-use-id').value.trim() };
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
    agorgList();
    agorgShowActive();
    agorgTree();
    if (currentTab === 'branch') {
      await branchLoadMatrix();
    }
    queueUiSessionSave();
  }
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
  const candidates = Array.isArray(discovery?.candidates) ? discovery.candidates : [];
  agorgApprovedPaths = new Set(
    candidates.filter(c => c.kind === 'ago').map(c => c.path)
  );
  renderAgorgDiscoveryReview();
}

function renderAgorgDiscoveryReview() {
  if (!agorgDiscoveryReview) return;
  const candidates = Array.isArray(agorgDiscoveryCache?.candidates) ? agorgDiscoveryCache.candidates : [];
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
    if (c.kind === 'agorg') { icon = '🏢'; kindTag = 'ORG'; chipClass = 'warn'; }
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
    <div style="padding:6px 4px;color:#a8b9e3;font-size:0.82rem;display:flex;justify-content:space-between;">
      <span>Approved ${approvedCount}/${selectableCount} candidates</span>
      ${agorgDefaultScopeCandidate ? `<span style="color:var(--accent);font-weight:600;">AGOrg Scope Selected</span>` : `<span style="color:#ff4d4d;font-weight:600;">⚠️ No AGOrg Scope Selected</span>`}
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
  const candidates = Array.isArray(agorgDiscoveryCache?.candidates) ? agorgDiscoveryCache.candidates : [];
  if (!candidates.length) return;
  agorgApprovedPaths = new Set(
    approve ? candidates.filter(c => c.kind === 'ago' || c.kind === 'folder').map(c => c.path) : []
  );
  renderAgorgDiscoveryReview();
}

async function getActiveAgorgId() {
  const active = await fetchJsonSafe('/api/agorg/active');
  if (!active?.ok || !active.active?.id) return null;
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
    const candidates = Array.isArray(agorgDiscoveryCache?.candidates) ? agorgDiscoveryCache.candidates : [];
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
  const root = document.getElementById('agorg-use-id').value.trim();
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
  const req = {
    destination: document.getElementById('agorg-create-dest').value.trim(),
    name: document.getElementById('agorg-create-name').value.trim(),
    siblings: document.getElementById('agorg-create-siblings').value.split('\n').map(s => s.trim()).filter(s => !!s),
    use_git: !!document.getElementById('agorg-create-git').checked
  };
  if (!req.destination || !req.name) {
    showInlineError("Destination and Name are required.", out);
    return;
  }
  const res = await fetch('/api/agorg/batch-create', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  const text = JSON.stringify(data, null, 2);
  agorgOut.textContent = text;
  out.textContent = text;
  if (data.ok) {
    agorgList();
    agorgTree();
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

function multiRegister() {
  run('pilot.multi.register', {
    path: document.getElementById('repo-path').value,
    name: document.getElementById('repo-name').value || null,
    group: document.getElementById('repo-group').value || null,
    tags: tags(document.getElementById('repo-tags').value)
  });
}
function multiList() {
  run('pilot.multi.list', { group: document.getElementById('multi-group').value || null, tags: tags(document.getElementById('multi-tags').value) });
}
function multiStatus() {
  run('pilot.multi.status', { group: document.getElementById('multi-group').value || null, tags: tags(document.getElementById('multi-tags').value) });
}
function multiOrder() {
  run('pilot.multi.order', { group: document.getElementById('multi-group').value || null, tags: tags(document.getElementById('multi-tags').value) });
}
function multiDag() {
  run('pilot.multi.dag', {
    group: document.getElementById('multi-group').value || null,
    tags: tags(document.getElementById('multi-tags').value),
    dry_run: true
  }, {
    label: 'DAG',
    chip: multiDagChip,
    buttons: [multiDagBtn],
    runningLabel: 'DAG running...'
  });
}
function multiPrsCreate() {
  run('pilot.multi.prs.create', {
    group: document.getElementById('multi-group').value || null,
    tags: tags(document.getElementById('multi-tags').value),
    dry_run: true,
    head_branch: 'dev',
    base_branch: 'main'
  });
}

function multiApplyPayload(apply) {
  const stageSizeRaw = parseInt(document.getElementById('multi-apply-stage-size').value || '2', 10);
  const stageSize = Number.isFinite(stageSizeRaw) && stageSizeRaw > 0 ? stageSizeRaw : 2;
  return {
    branch: document.getElementById('multi-apply-branch').value || 'feat/pilot-wave13',
    base_branch: document.getElementById('multi-apply-base').value || 'dev',
    pr_base_branch: document.getElementById('multi-apply-pr-base').value || 'main',
    group: document.getElementById('multi-group').value || null,
    tags: tags(document.getElementById('multi-tags').value),
    stage_size: stageSize,
    continue_on_failure: !!document.getElementById('multi-apply-continue').checked,
    apply: !!apply
  };
}

function multiApplyDryRun() {
  const payload = multiApplyPayload(false);
  run('pilot.multi.apply', payload, {
    label: 'Staged Apply',
    chip: multiApplyChip,
    buttons: [multiApplyDryBtn, multiApplyExecBtn],
    runningLabel: 'Running...'
  });
}

function multiApplyExecute() {
  const payload = multiApplyPayload(true);
  run('pilot.multi.apply', payload, {
    label: 'Staged Apply',
    chip: multiApplyChip,
    buttons: [multiApplyDryBtn, multiApplyExecBtn],
    runningLabel: 'Running...'
  });
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
}

function codexPayloadFromUi() {
  const raw = document.getElementById('codex-payload').value.trim();
  if (!raw) return {};
  return JSON.parse(raw);
}

async function codexRun(mode) {
  let payload;
  try {
    payload = codexPayloadFromUi();
  } catch (e) {
    const msg = 'Invalid JSON payload: ' + e.message;
    codexOut.textContent = msg;
    out.textContent = msg;
    return;
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
  const res = await fetch('/api/codex/action', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  const text = JSON.stringify(data, null, 2);
  codexOut.textContent = text;
  out.textContent = text;
  if (dashStatusOut) dashStatusOut.textContent = text;
  if (data && data.contract && data.contract.contract_id) {
    latestCodexContractId = data.contract.contract_id;
    document.getElementById('codex-contract-id').value = latestCodexContractId;
  }
  appendLive({ source: 'codex_ui', mode, command: req.command, ok: !!data.ok });
  if (mode === 'execute' || mode === 'reconcile' || mode === 'approve') loadHistory();
  if (mode === 'execute' || mode === 'reconcile' || mode === 'approve' || mode === 'preview') codexLoadContracts();
}

function codexPreview() { codexRun('preview'); }
function codexApprove() {
  if (!document.getElementById('codex-contract-id').value.trim() && latestCodexContractId) {
    document.getElementById('codex-contract-id').value = latestCodexContractId;
  }
  codexRun('approve');
}
function codexExecute() { codexRun('execute'); }
function codexReconcile() {
  if (!document.getElementById('codex-contract-id').value.trim() && latestCodexContractId) {
    document.getElementById('codex-contract-id').value = latestCodexContractId;
  }
  codexRun('reconcile');
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
  restoreBusStatus();
  await restoreUiSession();
  await loadHistory();
  await oracleLoadReports();
  await depLoadLogs();
  await depRun('policy');
  await depRun('hook-policy');
  await depRun('drift');
  await depRun('services-status');
  await refreshAgorgHeader();
  await dashRefreshTemporaryComponents();
  await dashRunTemporaryChecklist();
  await agorgLoadPolicyReports();
  await codexLoadContracts();
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
  restoreBranchLogLimit();
  refreshBranchPreviewState();
  branchRenderLog();
}

/* ==========================================================================
 * SETTINGS & GOVERNANCE UI
 * ========================================================================== */

let settingsActiveSimulationId = "";

function settingsSetStatus(data, level = 'info') {
  if (!settingsStatusOut) return;
  const rendered = typeof data === 'string' ? data : JSON.stringify(data, null, 2);
  settingsStatusOut.textContent = rendered;
  if (settingsStatusPanel) {
    settingsStatusPanel.classList.remove('ok', 'warn', 'fail');
    if (level === 'success') settingsStatusPanel.classList.add('ok');
    if (level === 'warn') settingsStatusPanel.classList.add('warn');
    if (level === 'error') settingsStatusPanel.classList.add('fail');
  }
}

function settingsShowError(message, context = null) {
  const payload = context ? `${message}\n${JSON.stringify(context, null, 2)}` : message;
  settingsSetStatus(payload, 'error');
  if (settingsStatusPanel) {
    showInlineError(message, settingsStatusPanel);
  }
  logActivity('Settings Error', context || { error: message });
}

async function settingsLoadPolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = document.getElementById('settings-policy-target').value.trim();
  const editor = document.getElementById('settings-policy-editor');
  
  let url = `/api/settings/policy/${kind}`;
  if (target) url += `?ago_path=${encodeURIComponent(target)}`;
  
  const res = await fetchJsonSafe(url);
  if (!res.ok) {
    if (res.error === "Policy not found") {
       editor.value = "{\n  // No policy specific to this scope. Will fallback/inherit.\n}";
       settingsSetStatus("No policy found for this target. Editor is in fallback/inherit mode.", 'warn');
    } else {
       settingsShowError('Error loading policy: ' + (res.error || 'unknown error'), res);
    }
    return;
  }
  editor.value = JSON.stringify(res.policy_json, null, 2);
  settingsSetStatus(res, 'success');
  logActivity('Loaded Policy', `Kind: ${kind}\nID: ${res.id}\nStatus: ${res.status}`);
  settingsActiveSimulationId = "";
}

async function settingsDraftPolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = document.getElementById('settings-policy-target').value.trim() || "";
  const editor = document.getElementById('settings-policy-editor');
  
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
  
  if (!res.ok) {
    settingsShowError('Failed to save draft: ' + (res.error || 'unknown error'), res);
    return;
  }
  settingsSetStatus(res, 'success');
  settingsActiveSimulationId = "";
  settingsLoadPolicy();
}

async function settingsSimulatePolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = document.getElementById('settings-policy-target').value.trim() || "";
  const editor = document.getElementById('settings-policy-editor');
  
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
  
  if (!res.ok) {
    logActivity('Simulation Failed', res.error);
    settingsShowError('Simulation failed: ' + (res.error || 'unknown error'), res);
    return;
  }
  
  settingsActiveSimulationId = res.evidence_id;
  settingsSetStatus(res, 'success');
  logActivity('Policy Simulation', JSON.stringify(res, null, 2));
}

async function settingsActivatePolicy() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = document.getElementById('settings-policy-target').value.trim() || "";
  
  if (!settingsActiveSimulationId) {
    settingsShowError("Must successfully simulate policy first!");
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

  if (!res.ok) {
    settingsShowError('Activation failed: ' + (res.error || 'unknown error'), res);
    return;
  }
  
  settingsSetStatus(res, 'success');
  settingsActiveSimulationId = "";
  settingsLoadPolicy();
}

async function settingsLoadExceptions() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = document.getElementById('settings-policy-target').value.trim() || "";
  
  let url = `/api/settings/exceptions/${kind}`;
  if (target) url += `?ago_path=${encodeURIComponent(target)}`;
  
  const res = await fetchJsonSafe(url);
  const list = document.getElementById('settings-exceptions-list');
  list.innerHTML = "";
  if (!res.ok) {
    if(!res.error) res.error = res;
    settingsShowError('Failed to load exceptions', res);
    return;
  }
  
  if(Array.isArray(res)) {
      res.forEach(exc => {
          const opt = document.createElement("option");
          opt.value = exc.id;
          opt.textContent = `[${exc.rule_path}] by ${exc.owner} - ${exc.reason} (Expires: ${new Date(exc.expires_at).toLocaleString()})`;
          list.appendChild(opt);
      });
      settingsSetStatus({ ok: true, loaded: res.length }, 'success');
  }
}

async function settingsDeleteException() {
  const list = document.getElementById('settings-exceptions-list');
  if(!list.value) {
     settingsShowError("Select an exception to revoke.");
     return;
  }
  const res = await fetchJsonSafe(`/api/settings/exceptions/delete/${list.value}`, { method: 'POST' });
  if(!res.ok) {
      settingsShowError("Failed to delete: " + (res.error || 'unknown error'), res);
      return;
  }
  settingsSetStatus(res, 'success');
  settingsLoadExceptions();
}

async function settingsAddException() {
  const kind = document.getElementById('settings-policy-kind').value || 'branch';
  const target = document.getElementById('settings-policy-target').value.trim() || "";
  
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
  
  if(!res.ok) {
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
  const target = document.getElementById('settings-policy-target').value.trim() || "";
  
  settingsSetStatus("Running compliance scan...", "info");
  
  const res = await fetchJsonSafe(`/api/settings/compliance_scan`, {
     method: 'POST',
     headers: {'Content-Type': 'application/json'},
     body: JSON.stringify({
        ago_path: target === "" ? null : target,
        kind: kind
     })
  });
  
  if(!res.ok) {
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
  
  if(!res.ok) {
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
  const target = document.getElementById('settings-policy-target').value.trim();
  
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
  
  if(!res.ok) {
     settingsShowError("Resolve failed: " + (res.error || 'unknown error'), res);
     return;
  }
  
  settingsSetStatus(res, 'success');
}

bootUi();
