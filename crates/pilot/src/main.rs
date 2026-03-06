#![allow(dead_code)]
mod agorg;
mod bus;
mod config;
mod db_runtime;
mod governance;
pub mod preflight;
mod serve_ui;
pub mod service_supervisor;
mod shim_runtime;
use agorg::AgorgStore;
use pilot_branch as branch;
use pilot_core::{
    append_audit_event, write_repo_outcomes_artifact, AuditEvent, CommandReport, RepoContext,
    RepoOutcome,
};
use pilot_create as create;
use pilot_heal as heal;
use pilot_know as know;
use pilot_multi as multi;
use pilot_navigate as navigate;
use pilot_oracle as oracle;
use pilot_plan as plan;
use pilot_secure as secure;

use clap::{Args, Parser, Subcommand};
use config::Config;
use miette::{miette, Context, IntoDiagnostic, Result};
use serde::de::DeserializeOwned;
use shim_runtime::bus_shim_command;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "pilot")]
#[command(about = "ArqonPilot: DevSecOps Automation System", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to config file
    #[arg(long, global = true, default_value = ".pilot/config.toml")]
    config: PathBuf,

    /// Emit a machine-readable JSON command report
    #[arg(long, global = true)]
    report_json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize ArqonPilot in the current repository
    Init,
    /// Codebase Oracle commands
    Oracle(OracleArgs),
    /// Autonomous Self-Healing CI
    Heal(HealArgs),
    /// Governed Release Pipeline
    Navigate(NavigateArgs),
    /// Security scanning and dependency maintenance
    Secure(SecureArgs),
    /// Planning and prioritization commands
    Plan(PlanArgs),
    /// Feature and test scaffolding commands
    Create(CreateArgs),
    /// Decision record and knowledge commands
    Know(KnowArgs),
    /// Cross-repo branch lifecycle operations
    Branch(BranchArgs),
    /// Multi-repo registry and operations
    Multi(MultiArgs),
    /// AGOrg/AGO control-plane operations
    Agorg(AgorgArgs),
    /// Managed local Postgres runtime operations
    Db(DbArgs),
    /// ArqonBus standalone operations
    Bus(BusArgs),
    /// Orchestrate all supervised background services (DB + Bus)
    Services(ServicesArgs),
    /// Run Pilot as an ArqonBus command bridge
    Serve(ServeArgs),
    /// Governance and Settings operations
    Settings(SettingsArgs),
    /// Governance Policy operations
    Policy(PolicyArgs),
    /// Tamper-Evident Evidence Verification
    Verify(VerifyArgs),
}

#[derive(Args)]
struct VerifyArgs {
    #[command(subcommand)]
    command: VerifyCommands,
}

#[derive(Subcommand)]
enum VerifyCommands {
    /// Walk the audit.jsonl chain and report integrity
    Chain,
    /// Verify a previously exported evidence bundle JSON
    Bundle {
        #[arg(long, short)]
        path: String,
    },
    /// Verify a single artifact against its .sha256 sidecar
    Artifact {
        #[arg(long, short)]
        path: String,
    },
}

#[derive(Args)]
struct OracleArgs {
    #[command(subcommand)]
    command: OracleCommands,
}

#[derive(Subcommand)]
enum OracleCommands {
    /// Build the Codebase Oracle (Graph + Vectors)
    Scan,
    /// Query the Codebase Oracle
    Query(OracleQueryArgs),
}

#[derive(Args)]
struct OracleQueryArgs {
    /// The query string
    #[arg(long, short)]
    query: String,

    /// Use CLI output mode instead of TUI (default for now)
    #[arg(long)]
    cli: bool,
}

#[derive(Args)]
struct HealArgs {
    /// Path to the test output file (cargo test --message-format=json)
    #[arg(long)]
    log_file: Option<PathBuf>,

    /// Maximum healing attempts (default: 2)
    #[arg(long, default_value = "2")]
    max_attempts: u32,

    /// Target file or crate to heal (optional, heals first failure if not specified)
    #[arg(long, short)]
    target: Option<String>,

    /// Enable verbose output with detailed progress
    #[arg(long, short)]
    verbose: bool,

    /// Build a multi-file repair plan and exit without applying fixes
    #[arg(long)]
    plan_only: bool,

    /// Maximum files to include in multi-file plan
    #[arg(long, default_value = "5")]
    max_files: usize,
}

#[derive(Args)]
struct SecureArgs {
    #[command(subcommand)]
    command: SecureCommands,
}

#[derive(Subcommand)]
enum SecureCommands {
    /// Scan selected repos for dependency vulnerabilities and leaked secrets
    Scan(SecureScanArgs),
    /// Apply or preview dependency maintenance fixes
    Fix(SecureFixArgs),
}

#[derive(Args, Clone)]
struct SecureScanArgs {
    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,
}

#[derive(Args, Clone)]
struct SecureFixArgs {
    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Apply fixes (default behavior is dry-run preview only)
    #[arg(long)]
    apply: bool,
}

#[derive(Args)]
struct PlanArgs {
    #[command(subcommand)]
    command: PlanCommands,
}

#[derive(Subcommand)]
enum PlanCommands {
    /// Ingest issues from JSON or GitHub into local plan cache
    Issues(PlanIssuesArgs),
    /// Score issues by impact/risk/effort
    Score(PlanScoreArgs),
    /// Generate prioritized roadmap markdown
    Roadmap(PlanRoadmapArgs),
}

#[derive(Args, Clone)]
struct PlanIssuesArgs {
    /// Input JSON file with issues array
    #[arg(long)]
    input: Option<PathBuf>,

    /// GitHub repo owner/name (e.g. novelbytelabs/ArqonPilot)
    #[arg(long)]
    github_repo: Option<String>,

    /// Optional output file path (default: ~/.pilot/plan/issues.json)
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Args, Clone)]
struct PlanScoreArgs {
    /// Input issues JSON (default: ~/.pilot/plan/issues.json)
    #[arg(long)]
    input: Option<PathBuf>,

    /// Output scored JSON (default: ~/.pilot/plan/scored.json)
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Args, Clone)]
struct PlanRoadmapArgs {
    /// Input scored JSON (default: ~/.pilot/plan/scored.json)
    #[arg(long)]
    input: Option<PathBuf>,

    /// Output roadmap markdown (default: ~/.pilot/plan/roadmap.md)
    #[arg(long)]
    output: Option<PathBuf>,

    /// Maximum roadmap items
    #[arg(long, default_value = "10")]
    top_n: usize,
}

#[derive(Args)]
struct CreateArgs {
    #[command(subcommand)]
    command: CreateCommands,
}

#[derive(Subcommand)]
enum CreateCommands {
    /// Scaffold a feature module and paired test
    Feature(CreateFeatureArgs),
    /// Scaffold a test skeleton for an existing target
    Tests(CreateTestsArgs),
}

#[derive(Args, Clone)]
struct CreateFeatureArgs {
    /// Feature name
    name: String,

    /// Root directory for scaffold output
    #[arg(long, default_value = ".")]
    output_dir: PathBuf,

    /// Preview without writing files
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Clone)]
struct CreateTestsArgs {
    /// Target module/component name
    target: String,

    /// Root directory for scaffold output
    #[arg(long, default_value = ".")]
    output_dir: PathBuf,

    /// Preview without writing files
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct KnowArgs {
    #[command(subcommand)]
    command: KnowCommands,
}

#[derive(Subcommand)]
enum KnowCommands {
    /// Record an ADR/decision entry
    Record(KnowRecordArgs),
    /// Search recorded decisions
    Query(KnowQueryArgs),
}

#[derive(Args, Clone)]
struct KnowRecordArgs {
    #[arg(long)]
    title: String,
    #[arg(long)]
    context: String,
    #[arg(long)]
    decision: String,
    #[arg(long, default_value = "accepted")]
    status: String,
    #[arg(long = "tag")]
    tags: Vec<String>,
}

#[derive(Args, Clone)]
struct KnowQueryArgs {
    #[arg(long, short)]
    query: String,
    #[arg(long, default_value = "20")]
    limit: usize,
}

#[derive(Args)]
struct NavigateArgs {
    /// Skip pre-flight checks
    #[arg(long)]
    skip_checks: bool,

    /// Dry run (don't create PR)
    #[arg(long)]
    dry_run: bool,

    /// Source branch for the release PR (default: current branch)
    #[arg(long, default_value = "dev")]
    head_branch: String,

    /// Target branch for the release PR (default: main)
    #[arg(long, default_value = "main")]
    base_branch: String,

    /// Run in multi-repo mode using the workspace registry
    #[arg(long)]
    multi: bool,

    /// Multi mode: select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Multi mode: select only repos containing all provided tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Multi mode: optional manifest output path
    #[arg(long)]
    plan_output: Option<PathBuf>,
}

#[derive(Args)]
struct BranchArgs {
    #[command(subcommand)]
    command: BranchCommands,
}

#[derive(Subcommand)]
enum BranchCommands {
    /// Create/reset a branch across selected repositories
    Create(BranchCreateArgs),
    /// Fast-forward sync a branch from origin/base across selected repositories
    Sync(BranchSyncArgs),
    /// Show current branch and cleanliness per selected repository
    Status(MultiFilterArgs),
    /// Prune merged branches across selected repositories
    Prune(BranchPruneArgs),
}

#[derive(Args, Clone)]
struct BranchCreateArgs {
    /// Branch to create/reset
    branch: String,

    /// Base branch to branch from
    #[arg(long, default_value = "main")]
    base_branch: String,

    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Preview without mutating repos
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Clone)]
struct BranchSyncArgs {
    /// Branch to sync
    #[arg(long, default_value = "dev")]
    branch: String,

    /// Base branch used as sync source
    #[arg(long, default_value = "main")]
    base_branch: String,

    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Preview without mutating repos
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Clone)]
struct BranchPruneArgs {
    /// Branch considered as merge base for pruning
    #[arg(long, default_value = "main")]
    base_branch: String,

    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Preview without mutating repos
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct MultiArgs {
    #[command(subcommand)]
    command: MultiCommands,
}

#[derive(Subcommand)]
enum MultiCommands {
    /// Register a repository in the workspace registry
    Register(MultiRegisterArgs),
    /// List registered repositories
    List(MultiFilterArgs),
    /// Show health/status across selected repositories
    Status(MultiFilterArgs),
    /// Run Oracle query across selected repositories
    Query(MultiQueryArgs),
    /// Manage repo dependency graph
    Deps(MultiDepsArgs),
    /// Print selected repos in dependency order
    Order(MultiFilterArgs),
    /// Export dependency DAG and staged execution plan
    Dag(MultiDagArgs),
    /// Apply branch orchestration in dependency-aware stages
    Apply(MultiApplyArgs),
    /// Linked pull request planning
    Prs(MultiPrsArgs),
}

#[derive(Args)]
struct SettingsArgs {
    #[command(subcommand)]
    command: SettingsCommands,
}

#[derive(Subcommand)]
enum SettingsCommands {
    /// Manage branch policies
    Branch {
        /// Show active branch policy json payload
        #[arg(long)]
        show: bool,
    },
}

#[derive(Args)]
struct PolicyArgs {
    #[command(subcommand)]
    command: PolicyCommands,
}

#[derive(Subcommand)]
enum PolicyCommands {
    /// List policy versions for a scope
    List {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        ago_path: Option<String>,
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Get effective policy
    Get {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        ago_path: Option<String>,
    },
    /// Save draft policy
    SetDraft {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        file: PathBuf,
    },
    /// Preview/simulate draft policy
    Preview {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        version: u32,
    },
    /// Approve draft policy
    Approve {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        version: u32,
        #[arg(long)]
        simulation_artifact: PathBuf,
    },
    /// Activate approved policy
    Activate {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        version: u32,
    },
    /// Delete a specific policy version
    Delete {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        ago_path: Option<String>,
        #[arg(long)]
        version: u32,
    },
    /// Resolve policy for a repository
    Resolve {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        repo_path: PathBuf,
    },
    /// Run compliance scan
    Scan {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        group: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    /// Manage policy exceptions
    Exceptions(PolicyExceptionsArgs),
    /// Query policy decisions
    Decisions {
        #[arg(long)]
        kind: String,
        #[arg(long, default_value = "100")]
        limit: usize,
    },
}

#[derive(Args)]
struct PolicyExceptionsArgs {
    #[command(subcommand)]
    command: PolicyExceptionsCommands,
}

#[derive(Subcommand)]
enum PolicyExceptionsCommands {
    /// List active exceptions
    List {
        #[arg(long)]
        kind: String,
    },
    /// Add an exception
    Add(PolicyExceptionsAddArgs),
    /// Delete an exception
    Delete {
        #[arg(long)]
        id: String,
    },
}

#[derive(Args, Clone)]
struct PolicyExceptionsAddArgs {
    #[arg(long)]
    kind: String,
    #[arg(long)]
    ago_path: Option<String>,
    #[arg(long)]
    rule_path: String,
    #[arg(long)]
    operation_scope: String,
    #[arg(long)]
    mode: String,
    #[arg(long)]
    owner: String,
    #[arg(long)]
    ticket: String,
    #[arg(long)]
    reason: String,
    #[arg(long)]
    expires_at: i64,
}

#[derive(Args)]
struct MultiRegisterArgs {
    /// Repo path to register
    #[arg(long, short, default_value = ".")]
    path: PathBuf,

    /// Optional explicit display name
    #[arg(long)]
    name: Option<String>,

    /// Optional group label
    #[arg(long)]
    group: Option<String>,

    /// Tag(s) for scoped selection; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,
}

#[derive(Args, Clone)]
struct MultiFilterArgs {
    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,
}

#[derive(Args, Clone)]
struct MultiQueryArgs {
    /// Query string
    #[arg(long, short)]
    query: String,

    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Max returned matches per repo
    #[arg(long, default_value = "5")]
    per_repo_limit: usize,
}

#[derive(Args)]
struct MultiDepsArgs {
    #[command(subcommand)]
    command: MultiDepsCommands,
}

#[derive(Subcommand)]
enum MultiDepsCommands {
    /// Set dependency edges for a repository
    Set(MultiDepsSetArgs),
}

#[derive(Args, Clone)]
struct MultiDepsSetArgs {
    /// Repository name
    #[arg(long)]
    repo: String,

    /// Repository names this repo depends on; repeatable
    #[arg(long = "depends-on")]
    depends_on: Vec<String>,

    /// Preview without mutating the dependency registry
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct MultiPrsArgs {
    #[command(subcommand)]
    command: MultiPrsCommands,
}

#[derive(Subcommand)]
enum MultiPrsCommands {
    /// Generate a linked-PR manifest in dependency order
    Create(MultiPrsCreateArgs),
}

#[derive(Args, Clone)]
struct MultiPrsCreateArgs {
    /// Source branch for PRs
    #[arg(long, default_value = "dev")]
    head_branch: String,

    /// Target branch for PRs
    #[arg(long, default_value = "main")]
    base_branch: String,

    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Optional output path for the generated manifest JSON
    #[arg(long)]
    output: Option<PathBuf>,

    /// Preview plan only, without writing manifest
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Clone)]
struct MultiDagArgs {
    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Optional output path for DAG JSON report
    #[arg(long)]
    output: Option<PathBuf>,

    /// Preview plan only, without writing a file
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Clone)]
struct MultiApplyArgs {
    /// Feature branch to create in selected repositories
    #[arg(long)]
    branch: String,

    /// Base branch used to create/reset feature branch
    #[arg(long, default_value = "dev")]
    base_branch: String,

    /// PR base branch used for linked PR planning
    #[arg(long, default_value = "main")]
    pr_base_branch: String,

    /// Select only repos in this group
    #[arg(long)]
    group: Option<String>,

    /// Select only repos that contain all given tags; repeatable
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Max repos to process in one batch within each dependency stage
    #[arg(long, default_value = "2")]
    stage_size: usize,

    /// Continue processing next batches even if one batch has failures
    #[arg(long)]
    continue_on_failure: bool,

    /// Optional output path for linked PR manifest JSON
    #[arg(long)]
    pr_output: Option<PathBuf>,

    /// Apply changes (default is dry-run preview)
    #[arg(long)]
    apply: bool,
}

#[derive(Args, Clone)]
struct ServeArgs {
    /// ArqonBus websocket URL
    #[arg(long, default_value_t = bus::default_ws_url())]
    ws_url: String,

    /// Environment variable that contains JWT for bus authentication
    #[arg(long, default_value_t = bus::default_jwt_env())]
    jwt_env: String,

    /// Bus room for pilot control-plane events
    #[arg(long, default_value_t = bus::default_room())]
    room: String,

    /// Bus channel for incoming pilot commands
    #[arg(long, default_value = "control")]
    channel: String,

    /// Bus channel for outgoing pilot telemetry events
    #[arg(long, default_value = "telemetry")]
    telemetry_channel: String,

    /// Process exactly one command then exit
    #[arg(long)]
    once: bool,

    /// Start local UI control panel bound to this host
    #[arg(long, default_value = "127.0.0.1")]
    ui_host: String,

    /// Start local UI control panel on this port
    #[arg(long)]
    ui_port: Option<u16>,

    /// Auto-start local ArqonBus shim when serving UI and ws-url is local default
    #[arg(long = "ui-auto-start-bus", default_value_t = true, action = clap::ArgAction::Set)]
    ui_auto_start_bus: bool,

    /// UI instance identifier for per-instance AGOrg scope/session isolation
    #[arg(long)]
    ui_instance_id: Option<String>,

    /// Allow mutating operations from UI/API (disabled by default for safety)
    #[arg(long)]
    ui_allow_mutations: bool,

    /// Restrict UI/API to these commands only (repeatable)
    #[arg(long = "ui-allow-command")]
    ui_allow_commands: Vec<String>,
}

#[derive(Args)]
struct AgorgArgs {
    #[command(subcommand)]
    command: AgorgCommands,
}

#[derive(Args)]
struct DbArgs {
    #[command(subcommand)]
    command: DbCommands,
}

#[derive(Args)]
struct BusArgs {
    #[command(subcommand)]
    command: BusCommands,
}

#[derive(Args)]
struct ServicesArgs {
    #[command(subcommand)]
    command: ServicesCommands,
}

#[derive(Subcommand)]
enum DbCommands {
    /// Initialize and start managed local Postgres if needed
    Ensure,
    /// Start managed local Postgres
    Start,
    /// Stop managed local Postgres
    Stop,
    /// Show managed local Postgres status and DSN
    Status,
}

#[derive(Subcommand)]
enum BusCommands {
    /// Start ArqonBus
    Start,
    /// Stop ArqonBus
    Stop,
    /// Restart ArqonBus
    Restart,
    /// Show ArqonBus status
    Status,
}

#[derive(Subcommand)]
enum ServicesCommands {
    /// Start all supervised background services (DB + Bus)
    Start,
    /// Stop all background services
    Stop,
    /// Restart all background services
    Restart,
    /// Show status for all services
    Status,
}

#[derive(Subcommand)]
enum AgorgCommands {
    /// Create an AGOrg record
    Create(AgorgCreateArgs),
    /// Create an AGOrg project and optionally autoscan/import hierarchy
    CreateProject(AgorgCreateProjectArgs),
    /// List AGOrgs
    List,
    /// Show active AGOrg scope
    Show,
    /// Set active AGOrg scope
    Use(AgorgUseArgs),
    /// Update AGOrg metadata
    Update(AgorgUpdateArgs),
    /// Delete AGOrg
    Delete(AgorgDeleteArgs),
    /// Discover AGOrg/AGO hierarchy from root
    Discover(AgorgDiscoverArgs),
    /// Print AGOrg graph/tree
    Tree(AgorgTreeArgs),
    /// Link one AGOrg as child of another (cycle-safe)
    Link(AgorgLinkArgs),
    /// Run AGOrg reconciliation report
    Reconcile(AgorgReconcileArgs),
}

#[derive(Args, Clone)]
struct AgorgCreateArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    master: Option<String>,
    #[arg(long)]
    parent: Option<String>,
    #[arg(long, default_value = "4")]
    scan_depth: usize,
    #[arg(long)]
    default_scope: bool,
}

#[derive(Args, Clone)]
struct AgorgCreateProjectArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    master: Option<String>,
    #[arg(long)]
    parent: Option<String>,
    #[arg(long, default_value = "4")]
    scan_depth: usize,
    #[arg(long)]
    autoscan: bool,
    #[arg(long)]
    import: bool,
    #[arg(long)]
    prune_missing: bool,
    #[arg(long)]
    default_scope: bool,
}

#[derive(Args, Clone)]
struct AgorgUseArgs {
    /// AGOrg UUID or name
    agorg: String,
}

#[derive(Args, Clone)]
struct AgorgUpdateArgs {
    /// AGOrg UUID or name
    agorg: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    root: Option<PathBuf>,
    #[arg(long)]
    master: Option<String>,
    #[arg(long)]
    scan_depth: Option<usize>,
    #[arg(long)]
    default_scope: bool,
}

#[derive(Args, Clone)]
struct AgorgDeleteArgs {
    /// AGOrg UUID or name
    agorg: String,
}

#[derive(Args, Clone)]
struct AgorgDiscoverArgs {
    #[arg(long)]
    root: PathBuf,
    #[arg(long, default_value = "4")]
    depth: usize,
    #[arg(long)]
    import_to: Option<String>,
    #[arg(long)]
    prune_missing: bool,
}

#[derive(Args, Clone)]
struct AgorgTreeArgs {
    /// Optional AGOrg UUID or name for subtree root
    #[arg(long)]
    root: Option<String>,
}

#[derive(Args, Clone)]
struct AgorgLinkArgs {
    /// Parent AGOrg UUID or name
    #[arg(long)]
    parent: String,
    /// Child AGOrg UUID or name
    #[arg(long)]
    child: String,
}

#[derive(Args, Clone)]
struct AgorgReconcileArgs {
    /// Optional AGOrg UUID or name (defaults to active scope)
    #[arg(long)]
    agorg: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let outcome = run_cli(&cli).await;

    match outcome {
        Ok(report) => {
            emit_report(&report, cli.report_json)?;
            Ok(())
        }
        Err(err) => {
            let report = CommandReport::err(command_name(&cli.command), err.to_string());
            let _ = emit_report(&report, cli.report_json);
            Err(err)
        }
    }
}

async fn run_cli(cli: &Cli) -> Result<CommandReport> {
    let _config = if let Commands::Init = &cli.command {
        Config::default()
    } else {
        Config::load_from_file(&cli.config).unwrap_or_default()
    };

    match &cli.command {
        Commands::Init => {
            handle_init(&cli.config)?;
            let report = CommandReport::ok("init", "Initialized .pilot/config.toml");
            persist_mutation_audit(
                "init",
                false,
                &report.summary,
                vec![RepoOutcome {
                    repo: "current-repo".to_string(),
                    path: cli.config.display().to_string(),
                    success: true,
                    message: "Config initialized".to_string(),
                }],
            );
            Ok(report)
        }
        Commands::Oracle(args) => match &args.command {
            OracleCommands::Scan => {
                let root = std::env::current_dir().into_diagnostic()?;
                let ctx = RepoContext::new(root.clone());
                oracle::scan_codebase(&ctx.root)
                    .await
                    .map_err(|e| miette::miette!("{:?}", e))?;
                Ok(CommandReport::ok(
                    "oracle.scan",
                    format!("Scanned codebase at {}", ctx.root.display()),
                ))
            }
            OracleCommands::Query(args) => {
                let root = std::env::current_dir().into_diagnostic()?;
                let ctx = RepoContext::new(root.clone());
                let db_path = ctx.root.join(".pilot/graph.db");
                let vector_path = ctx.root.join(".pilot/vectors.lance");

                let mut engine = oracle::query::QueryEngine::new(
                    db_path.to_str().unwrap(),
                    vector_path.to_str().unwrap(),
                )
                .await
                .map_err(|e| miette::miette!("{:?}", e))?;

                let results = engine
                    .query(&args.query)
                    .await
                    .map_err(|e| miette::miette!("{:?}", e))?;

                let result_count = results.len();
                for res in results {
                    println!("[{}] {} (Score: {:.3})", res.path, res.name, res.score);
                }

                Ok(CommandReport::ok(
                    "oracle.query",
                    format!("Returned {} results", result_count),
                ))
            }
        },
        Commands::Heal(args) => {
            let root = std::env::current_dir().into_diagnostic()?;
            let ctx = RepoContext::new(root.clone());
            println!("Starting self-healing pipeline...");

            let log_path = args
                .log_file
                .clone()
                .unwrap_or_else(|| PathBuf::from("test_output.json"));
            if !log_path.exists() {
                let msg = format!(
                    "No test log file found at {:?}. Run: cargo test --message-format=json > test_output.json",
                    log_path
                );
                println!("{}", msg);
                return Ok(CommandReport::ok("heal", msg));
            }

            use heal::parser_rust::RustLogParser;
            use heal::r#loop::HealingLoop;
            use oracle::OracleStore;

            let failure = RustLogParser::parse_file(&log_path)
                .map_err(|e| miette::miette!("{:?}", e))?
                .ok_or_else(|| miette::miette!("No test failures found in log"))?;

            println!("Detected failure in: {}", failure.file_path);

            let db_path = ctx.root.join(".pilot/graph.db");
            let store = OracleStore::open(db_path.to_str().unwrap())
                .map_err(|e| miette::miette!("{:?}", e))?;

            if args.plan_only {
                let plan = heal::plan::build_multifile_repair_plan(
                    &ctx.root,
                    &store,
                    &failure,
                    args.max_files,
                )
                .map_err(|e| miette::miette!("{:?}", e))?;
                println!("Primary file: {}", plan.primary_file);
                println!("Candidate files:");
                for (idx, file) in plan.candidate_files.iter().enumerate() {
                    println!("  {}. {}", idx + 1, file);
                }
                if !plan.related_signatures.is_empty() {
                    println!("Related signatures:");
                    for sig in &plan.related_signatures {
                        println!("  - {}", sig);
                    }
                }
                return Ok(CommandReport::ok(
                    "heal.plan",
                    format!(
                        "Generated multi-file repair plan with {} candidates",
                        plan.candidate_files.len()
                    ),
                ));
            }

            let mut healing_loop = HealingLoop::new(store, ctx.root, args.max_attempts)
                .map_err(|e| miette::miette!("{:?}", e))?;
            let outcome = healing_loop
                .run(&failure)
                .map_err(|e| miette::miette!("{:?}", e))?;

            println!("Heal outcome: {:?}", outcome);
            Ok(CommandReport::ok("heal", format!("Outcome: {:?}", outcome)))
        }
        Commands::Secure(args) => match &args.command {
            SecureCommands::Scan(args) => {
                let repos = resolve_secure_targets(args.group.clone(), args.tags.clone())?;
                let mut total_findings = 0usize;
                for repo in &repos {
                    let report = secure::scan_repo(repo)
                        .map_err(|e| miette::miette!("Secure scan failed: {e}"))?;
                    println!("Repo: {}", report.repo_path.display());
                    if report.findings.is_empty() {
                        println!("  - no findings");
                    }
                    for f in &report.findings {
                        println!(
                            "  - [{}:{}] {} {}",
                            f.category, f.severity, f.rule, f.message
                        );
                    }
                    total_findings += report.findings.len();
                }

                Ok(CommandReport::ok(
                    "secure.scan",
                    format!(
                        "Scanned {} repos and found {} findings",
                        repos.len(),
                        total_findings
                    ),
                ))
            }
            SecureCommands::Fix(args) => {
                let repos = resolve_secure_targets(args.group.clone(), args.tags.clone())?;
                let dry_run = !args.apply;
                let mut actions_total = 0usize;
                let mut failures = 0usize;
                let mut outcomes: Vec<RepoOutcome> = Vec::new();
                for repo in &repos {
                    let actions = secure::fix_repo(repo, dry_run)
                        .map_err(|e| miette::miette!("Secure fix failed: {e}"))?;
                    println!("Repo: {}", repo.display());
                    let mut repo_ok = true;
                    let mut messages = Vec::new();
                    for a in &actions {
                        println!(
                            "  - {} | applied={} | ok={} | {}",
                            a.command, a.applied, a.success, a.message
                        );
                        if !a.success {
                            failures += 1;
                            repo_ok = false;
                        }
                        messages.push(format!("{}: {}", a.command, a.message));
                    }
                    actions_total += actions.len();
                    outcomes.push(RepoOutcome {
                        repo: repo
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("repo")
                            .to_string(),
                        path: repo.display().to_string(),
                        success: repo_ok,
                        message: messages.join(" | "),
                    });
                }

                let report = CommandReport::ok(
                    "secure.fix",
                    format!(
                        "{} mode across {} repos: {} actions, {} failures",
                        if dry_run { "dry-run" } else { "apply" },
                        repos.len(),
                        actions_total,
                        failures
                    ),
                );
                persist_mutation_audit("secure.fix", dry_run, &report.summary, outcomes);
                Ok(report)
            }
        },
        Commands::Plan(args) => match &args.command {
            PlanCommands::Issues(args) => {
                let out = args
                    .output
                    .clone()
                    .unwrap_or_else(|| plan::default_plan_dir().join("issues.json"));

                let issues = if let Some(input) = &args.input {
                    plan::load_issues_from_file(input)
                        .map_err(|e| miette::miette!("Failed loading input issues: {e}"))?
                } else if let Some(repo) = &args.github_repo {
                    let (owner, name) = repo.split_once('/').ok_or_else(|| {
                        miette::miette!("--github-repo must be in owner/repo format")
                    })?;
                    let token = std::env::var("GITHUB_TOKEN").ok();
                    plan::fetch_issues_from_github(owner, name, token.as_deref())
                        .map_err(|e| miette::miette!("Failed fetching GitHub issues: {e}"))?
                } else {
                    return Err(miette::miette!(
                        "Provide either --input or --github-repo for plan issues"
                    ));
                };

                plan::write_issues(&out, &issues)
                    .map_err(|e| miette::miette!("Failed writing issues cache: {e}"))?;
                println!("Cached {} issues at {}", issues.len(), out.display());
                let report =
                    CommandReport::ok("plan.issues", format!("Cached {} issues", issues.len()));
                persist_mutation_audit(
                    "plan.issues",
                    false,
                    &report.summary,
                    vec![RepoOutcome {
                        repo: "plan-cache".to_string(),
                        path: out.display().to_string(),
                        success: true,
                        message: format!("Cached {} issues", issues.len()),
                    }],
                );
                Ok(report)
            }
            PlanCommands::Score(args) => {
                let input = args
                    .input
                    .clone()
                    .unwrap_or_else(|| plan::default_plan_dir().join("issues.json"));
                let output = args
                    .output
                    .clone()
                    .unwrap_or_else(|| plan::default_plan_dir().join("scored.json"));
                let issues = plan::load_issues_from_file(&input)
                    .map_err(|e| miette::miette!("Failed loading issues: {e}"))?;
                let scored = plan::score_issues(issues);
                plan::write_scored(&output, &scored)
                    .map_err(|e| miette::miette!("Failed writing scored issues: {e}"))?;
                println!(
                    "Wrote {} scored items to {}",
                    scored.len(),
                    output.display()
                );
                let report =
                    CommandReport::ok("plan.score", format!("Scored {} issues", scored.len()));
                persist_mutation_audit(
                    "plan.score",
                    false,
                    &report.summary,
                    vec![RepoOutcome {
                        repo: "plan-cache".to_string(),
                        path: output.display().to_string(),
                        success: true,
                        message: format!("Scored {} issues", scored.len()),
                    }],
                );
                Ok(report)
            }
            PlanCommands::Roadmap(args) => {
                let input = args
                    .input
                    .clone()
                    .unwrap_or_else(|| plan::default_plan_dir().join("scored.json"));
                let output = args
                    .output
                    .clone()
                    .unwrap_or_else(|| plan::default_plan_dir().join("roadmap.md"));
                let scored = plan::load_scored(&input)
                    .map_err(|e| miette::miette!("Failed loading scored issues: {e}"))?;
                let roadmap = plan::build_roadmap(scored, args.top_n);
                plan::write_roadmap_markdown(&output, &roadmap)
                    .map_err(|e| miette::miette!("Failed writing roadmap: {e}"))?;
                println!(
                    "Wrote roadmap with {} items to {}",
                    roadmap.items.len(),
                    output.display()
                );
                let report = CommandReport::ok(
                    "plan.roadmap",
                    format!("Generated roadmap with {} items", roadmap.items.len()),
                );
                persist_mutation_audit(
                    "plan.roadmap",
                    false,
                    &report.summary,
                    vec![RepoOutcome {
                        repo: "plan-cache".to_string(),
                        path: output.display().to_string(),
                        success: true,
                        message: format!("Roadmap items {}", roadmap.items.len()),
                    }],
                );
                Ok(report)
            }
        },
        Commands::Create(args) => match &args.command {
            CreateCommands::Feature(args) => {
                let actions = create::scaffold_feature(&args.output_dir, &args.name, args.dry_run)
                    .map_err(|e| miette::miette!("Create feature failed: {e}"))?;
                for action in &actions {
                    println!(
                        "{} | created={} | {}",
                        action.path.display(),
                        action.created,
                        action.message
                    );
                }
                let report = CommandReport::ok(
                    "create.feature",
                    format!("Processed {} scaffold actions", actions.len()),
                );
                let outcomes: Vec<RepoOutcome> = actions
                    .iter()
                    .map(|a| RepoOutcome {
                        repo: args.name.clone(),
                        path: a.path.display().to_string(),
                        success: a.created
                            || a.message.contains("DRY RUN")
                            || a.message.contains("exists"),
                        message: a.message.clone(),
                    })
                    .collect();
                persist_mutation_audit("create.feature", args.dry_run, &report.summary, outcomes);
                Ok(report)
            }
            CreateCommands::Tests(args) => {
                let action = create::scaffold_tests(&args.output_dir, &args.target, args.dry_run)
                    .map_err(|e| miette::miette!("Create tests failed: {e}"))?;
                println!(
                    "{} | created={} | {}",
                    action.path.display(),
                    action.created,
                    action.message
                );
                let report = CommandReport::ok("create.tests", "Generated test scaffold");
                persist_mutation_audit(
                    "create.tests",
                    args.dry_run,
                    &report.summary,
                    vec![RepoOutcome {
                        repo: args.target.clone(),
                        path: action.path.display().to_string(),
                        success: action.created
                            || action.message.contains("DRY RUN")
                            || action.message.contains("exists"),
                        message: action.message,
                    }],
                );
                Ok(report)
            }
        },
        Commands::Know(args) => {
            let db_path = know::KnowStore::default_db_path();
            let store = know::KnowStore::open(&db_path)
                .map_err(|e| miette::miette!("Failed to open know store: {e}"))?;
            match &args.command {
                KnowCommands::Record(args) => {
                    let id = store
                        .record(
                            &args.title,
                            &args.context,
                            &args.decision,
                            &args.status,
                            &args.tags,
                        )
                        .map_err(|e| miette::miette!("Failed recording decision: {e}"))?;
                    println!("Recorded decision {} in {}", id, db_path.display());
                    let report =
                        CommandReport::ok("know.record", format!("Recorded decision {}", id));
                    persist_mutation_audit(
                        "know.record",
                        false,
                        &report.summary,
                        vec![RepoOutcome {
                            repo: "pilot-know".to_string(),
                            path: db_path.display().to_string(),
                            success: true,
                            message: format!("Decision {} recorded", id),
                        }],
                    );
                    Ok(report)
                }
                KnowCommands::Query(args) => {
                    let records = store
                        .query(&args.query, args.limit)
                        .map_err(|e| miette::miette!("Failed querying decisions: {e}"))?;
                    for rec in &records {
                        println!(
                            "[{}] {} | status={} | tags={}",
                            rec.id,
                            rec.title,
                            rec.status,
                            rec.tags.join(",")
                        );
                    }
                    Ok(CommandReport::ok(
                        "know.query",
                        format!("Returned {} decision records", records.len()),
                    ))
                }
            }
        }
        Commands::Navigate(args) => {
            let report = if args.multi {
                run_navigate_multi(args)?
            } else {
                run_navigate_single(args)?
            };
            let outcomes = if args.multi {
                resolve_multi_outcomes(args.group.clone(), args.tags.clone())
            } else {
                vec![RepoOutcome {
                    repo: std::env::current_dir()
                        .ok()
                        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "current-repo".to_string()),
                    path: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    success: true,
                    message: report.summary.clone(),
                }]
            };
            persist_mutation_audit(
                if args.multi {
                    "navigate.multi"
                } else {
                    "navigate"
                },
                args.dry_run,
                &report.summary,
                outcomes,
            );
            Ok(report)
        }
        Commands::Branch(args) => {
            let db_path = multi::MultiRegistry::default_db_path();
            let registry = multi::MultiRegistry::open(&db_path)
                .map_err(|e| miette::miette!("Failed to open multi registry: {e}"))?;

            match &args.command {
                BranchCommands::Create(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let repos = registry
                        .dependency_order(&filter)
                        .map_err(|e| miette::miette!("Branch create selection failed: {e}"))?;
                    let outcomes = branch::create_branch(
                        &repos,
                        &args.branch,
                        &args.base_branch,
                        args.dry_run,
                    );

                    for o in &outcomes {
                        println!("{} | {} | ok={} | {}", o.repo, o.path, o.success, o.message);
                    }

                    let failures = outcomes.iter().filter(|o| !o.success).count();
                    let report = CommandReport::ok(
                        "branch.create",
                        format!("Processed {} repos ({} failed)", outcomes.len(), failures),
                    );
                    persist_mutation_audit(
                        "branch.create",
                        args.dry_run,
                        &report.summary,
                        outcomes
                            .iter()
                            .map(|o| RepoOutcome {
                                repo: o.repo.clone(),
                                path: o.path.clone(),
                                success: o.success,
                                message: o.message.clone(),
                            })
                            .collect(),
                    );
                    Ok(report)
                }
                BranchCommands::Sync(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let repos = registry
                        .dependency_order(&filter)
                        .map_err(|e| miette::miette!("Branch sync selection failed: {e}"))?;
                    let outcomes =
                        branch::sync_branch(&repos, &args.branch, &args.base_branch, args.dry_run);

                    for o in &outcomes {
                        println!("{} | {} | ok={} | {}", o.repo, o.path, o.success, o.message);
                    }

                    let failures = outcomes.iter().filter(|o| !o.success).count();
                    let report = CommandReport::ok(
                        "branch.sync",
                        format!("Processed {} repos ({} failed)", outcomes.len(), failures),
                    );
                    persist_mutation_audit(
                        "branch.sync",
                        args.dry_run,
                        &report.summary,
                        outcomes
                            .iter()
                            .map(|o| RepoOutcome {
                                repo: o.repo.clone(),
                                path: o.path.clone(),
                                success: o.success,
                                message: o.message.clone(),
                            })
                            .collect(),
                    );
                    Ok(report)
                }
                BranchCommands::Status(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let repos = registry
                        .list_repos(&filter)
                        .map_err(|e| miette::miette!("Branch status selection failed: {e}"))?;
                    let statuses = branch::branch_status(&repos);
                    for s in &statuses {
                        println!(
                            "{} | {} | branch={} | clean={}",
                            s.repo, s.path, s.current_branch, s.clean
                        );
                    }

                    Ok(CommandReport::ok(
                        "branch.status",
                        format!("Reported branch status for {} repos", statuses.len()),
                    ))
                }
                BranchCommands::Prune(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let repos = registry
                        .list_repos(&filter)
                        .map_err(|e| miette::miette!("Branch prune selection failed: {e}"))?;
                    let outcomes = branch::prune_branches(&repos, &args.base_branch, args.dry_run)
                        .map_err(|e| miette::miette!("Prune failed: {e}"))?;

                    for o in &outcomes {
                        println!("{} | {} | ok={} | {}", o.repo, o.path, o.success, o.message);
                    }

                    let failures = outcomes.iter().filter(|o| !o.success).count();
                    let report = CommandReport::ok(
                        "branch.prune",
                        format!("Processed {} repos ({} failed)", outcomes.len(), failures),
                    );
                    persist_mutation_audit(
                        "branch.prune",
                        args.dry_run,
                        &report.summary,
                        outcomes
                            .iter()
                            .map(|o| RepoOutcome {
                                repo: o.repo.clone(),
                                path: o.path.clone(),
                                success: o.success,
                                message: o.message.clone(),
                            })
                            .collect(),
                    );
                    Ok(report)
                }
            }
        }
        Commands::Multi(args) => {
            let db_path = multi::MultiRegistry::default_db_path();
            let registry = multi::MultiRegistry::open(&db_path)
                .map_err(|e| miette::miette!("Failed to open multi registry: {e}"))?;

            match &args.command {
                MultiCommands::Register(args) => {
                    let canonical = args
                        .path
                        .canonicalize()
                        .map_err(|e| miette::miette!("Register failed: {}", e))?;
                    let desired_name = args.name.clone().unwrap_or_else(|| {
                        canonical
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "repo".to_string())
                    });
                    let desired_group = args.group.clone();
                    let mut desired_tags = args.tags.clone();
                    desired_tags.sort();
                    desired_tags.dedup();

                    let existing = registry
                        .list_repos(&multi::RepoFilter::default())
                        .map_err(|e| miette::miette!("Register failed: {}", e))?
                        .into_iter()
                        .find(|r| r.path == canonical);

                    let (entry, already_registered) = if let Some(curr) = existing {
                        let mut current_tags = curr.tags.clone();
                        current_tags.sort();
                        current_tags.dedup();
                        let unchanged = curr.name == desired_name
                            && curr.group_name == desired_group
                            && current_tags == desired_tags;
                        if unchanged {
                            (curr, true)
                        } else {
                            (
                                registry
                                    .register_repo(
                                        &args.path,
                                        args.name.as_deref(),
                                        args.group.as_deref(),
                                        &args.tags,
                                    )
                                    .map_err(|e| miette::miette!("Register failed: {e}"))?,
                                false,
                            )
                        }
                    } else {
                        (
                            registry
                                .register_repo(
                                    &args.path,
                                    args.name.as_deref(),
                                    args.group.as_deref(),
                                    &args.tags,
                                )
                                .map_err(|e| miette::miette!("Register failed: {e}"))?,
                            false,
                        )
                    };

                    if already_registered {
                        println!(
                            "Already registered: {} ({}) group={:?} tags={:?}",
                            entry.name,
                            entry.path.display(),
                            entry.group_name,
                            entry.tags
                        );
                    } else {
                        println!(
                            "Registered: {} ({}) group={:?} tags={:?}",
                            entry.name,
                            entry.path.display(),
                            entry.group_name,
                            entry.tags
                        );
                    }

                    let report = CommandReport::ok(
                        "multi.register",
                        if already_registered {
                            format!("Already registered {}", entry.path.display())
                        } else {
                            format!("Registered {}", entry.path.display())
                        },
                    );
                    persist_mutation_audit(
                        "multi.register",
                        false,
                        &report.summary,
                        vec![RepoOutcome {
                            repo: entry.name.clone(),
                            path: entry.path.display().to_string(),
                            success: true,
                            message: format!(
                                "group={:?} tags={}",
                                entry.group_name,
                                entry.tags.join(",")
                            ),
                        }],
                    );
                    Ok(report)
                }
                MultiCommands::List(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let repos = registry
                        .list_repos(&filter)
                        .map_err(|e| miette::miette!("List failed: {e}"))?;

                    for repo in &repos {
                        println!(
                            "{} | {} | group={:?} | tags={}",
                            repo.name,
                            repo.path.display(),
                            repo.group_name,
                            repo.tags.join(",")
                        );
                    }

                    Ok(CommandReport::ok(
                        "multi.list",
                        format!("Listed {} repositories", repos.len()),
                    ))
                }
                MultiCommands::Status(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let statuses = registry
                        .status_repos(&filter)
                        .map_err(|e| miette::miette!("Status failed: {e}"))?;

                    for s in &statuses {
                        println!(
                            "{} | exists={} git_repo={} git_clean={:?} pilot_init={} oracle_ready={}",
                            s.repo.path.display(),
                            s.path_exists,
                            s.is_git_repo,
                            s.git_clean,
                            s.pilot_initialized,
                            s.oracle_ready
                        );
                    }

                    Ok(CommandReport::ok(
                        "multi.status",
                        format!("Reported status for {} repositories", statuses.len()),
                    ))
                }
                MultiCommands::Query(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let fanout = registry
                        .query_across_repos(&filter, &args.query, args.per_repo_limit)
                        .await
                        .map_err(|e| miette::miette!("Multi query failed: {e}"))?;

                    let mut total_hits = 0usize;
                    for item in &fanout {
                        if let Some(err) = &item.error {
                            println!(
                                "[{}] {} | ERROR: {}",
                                item.repo,
                                item.repo_path.display(),
                                err
                            );
                            continue;
                        }

                        println!("[{}] {}", item.repo, item.repo_path.display());
                        for r in &item.results {
                            println!("  - {} :: {} (score {:.3})", r.path, r.name, r.score);
                        }
                        total_hits += item.results.len();
                    }

                    Ok(CommandReport::ok(
                        "multi.query",
                        format!(
                            "Fanout complete across {} repos, {} total hits",
                            fanout.len(),
                            total_hits
                        ),
                    ))
                }
                MultiCommands::Deps(args) => match &args.command {
                    MultiDepsCommands::Set(args) => {
                        if args.dry_run {
                            println!(
                                "[DRY RUN] Would set dependencies for '{}' => [{}]",
                                args.repo,
                                args.depends_on.join(", ")
                            );
                            let report = CommandReport::ok(
                                "multi.deps.set",
                                "Dry-run dependency update planned",
                            );
                            persist_mutation_audit(
                                "multi.deps.set",
                                true,
                                &report.summary,
                                vec![RepoOutcome {
                                    repo: args.repo.clone(),
                                    path: "~/.pilot/workspace.db".to_string(),
                                    success: true,
                                    message: format!(
                                        "Would set dependencies => [{}]",
                                        args.depends_on.join(", ")
                                    ),
                                }],
                            );
                            return Ok(report);
                        }

                        registry
                            .set_dependencies(&args.repo, &args.depends_on)
                            .map_err(|e| miette::miette!("Set deps failed: {e}"))?;
                        println!(
                            "Updated dependencies for '{}' => [{}]",
                            args.repo,
                            args.depends_on.join(", ")
                        );
                        let report = CommandReport::ok(
                            "multi.deps.set",
                            format!("Updated dependencies for {}", args.repo),
                        );
                        persist_mutation_audit(
                            "multi.deps.set",
                            false,
                            &report.summary,
                            vec![RepoOutcome {
                                repo: args.repo.clone(),
                                path: "~/.pilot/workspace.db".to_string(),
                                success: true,
                                message: format!(
                                    "Dependencies set => [{}]",
                                    args.depends_on.join(", ")
                                ),
                            }],
                        );
                        Ok(report)
                    }
                },
                MultiCommands::Order(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let ordered = registry
                        .dependency_order(&filter)
                        .map_err(|e| miette::miette!("Order failed: {e}"))?;

                    for (idx, repo) in ordered.iter().enumerate() {
                        println!(
                            "{}. {} | {} | group={:?} | tags={}",
                            idx + 1,
                            repo.name,
                            repo.path.display(),
                            repo.group_name,
                            repo.tags.join(",")
                        );
                    }

                    Ok(CommandReport::ok(
                        "multi.order",
                        format!(
                            "Computed dependency order for {} repositories",
                            ordered.len()
                        ),
                    ))
                }
                MultiCommands::Dag(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let dag = registry
                        .dependency_dag_report(&filter)
                        .map_err(|e| miette::miette!("DAG build failed: {e}"))?;

                    println!(
                        "Dependency DAG: repos={} edges={} stages={}",
                        dag.repos.len(),
                        dag.edges.len(),
                        dag.stages.len()
                    );
                    for (idx, stage) in dag.stages.iter().enumerate() {
                        println!("Stage {}: {}", idx + 1, stage.join(", "));
                    }

                    if args.dry_run {
                        return Ok(CommandReport::ok(
                            "multi.dag",
                            format!(
                                "Dry-run DAG generated (repos={}, edges={}, stages={})",
                                dag.repos.len(),
                                dag.edges.len(),
                                dag.stages.len()
                            ),
                        ));
                    }

                    let out_path = registry
                        .write_dependency_dag_report(&filter, args.output.as_deref())
                        .map_err(|e| miette::miette!("DAG write failed: {e}"))?;
                    println!("DAG manifest: {}", out_path.display());

                    Ok(CommandReport::ok(
                        "multi.dag",
                        format!("Dependency DAG written to {}", out_path.display()),
                    ))
                }
                MultiCommands::Apply(args) => {
                    let filter = to_filter(args.group.clone(), args.tags.clone());
                    let mut stages = registry
                        .dependency_stages(&filter)
                        .map_err(|e| miette::miette!("Stage plan failed: {e}"))?;
                    if stages.is_empty() {
                        return Err(miette::miette!("No repositories match selected group/tags"));
                    }

                    let stage_size = args.stage_size.max(1);
                    let dry_run = !args.apply;
                    let mut outcomes: Vec<branch::BranchOutcome> = Vec::new();
                    let mut failed_batches = 0usize;
                    let mut stage_count = 0usize;

                    println!(
                        "{} multi apply for branch '{}' from '{}' (stage_size={})",
                        if dry_run { "[DRY RUN]" } else { "[APPLY]" },
                        args.branch,
                        args.base_branch,
                        stage_size
                    );

                    for (stage_idx, stage_repos) in stages.iter_mut().enumerate() {
                        stage_count += 1;
                        stage_repos.sort_by(|a, b| a.name.cmp(&b.name));
                        println!(
                            "Stage {} ({} repos): {}",
                            stage_idx + 1,
                            stage_repos.len(),
                            stage_repos
                                .iter()
                                .map(|r| r.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );

                        for batch in stage_repos.chunks(stage_size) {
                            let batch_repos: Vec<_> = batch.to_vec();
                            let batch_names = batch_repos
                                .iter()
                                .map(|r| r.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                            println!("  -> Batch: {}", batch_names);
                            let batch_outcomes = branch::create_branch(
                                &batch_repos,
                                &args.branch,
                                &args.base_branch,
                                dry_run,
                            );
                            let batch_failed = batch_outcomes.iter().any(|o| !o.success);
                            for o in &batch_outcomes {
                                println!(
                                    "     {} | {} | ok={} | {}",
                                    o.repo, o.path, o.success, o.message
                                );
                            }
                            outcomes.extend(batch_outcomes);
                            if batch_failed {
                                failed_batches += 1;
                                if !args.continue_on_failure {
                                    let report = CommandReport::err(
                                        "multi.apply",
                                        format!(
                                            "Stopped at stage {} due to batch failure (use --continue-on-failure to proceed)",
                                            stage_idx + 1
                                        ),
                                    );
                                    persist_mutation_audit(
                                        "multi.apply",
                                        dry_run,
                                        &report.summary,
                                        outcomes
                                            .iter()
                                            .map(|o| RepoOutcome {
                                                repo: o.repo.clone(),
                                                path: o.path.clone(),
                                                success: o.success,
                                                message: o.message.clone(),
                                            })
                                            .collect(),
                                    );
                                    return Ok(report);
                                }
                            }
                        }
                    }

                    let pr_manifest = if dry_run {
                        None
                    } else {
                        Some(
                            registry
                                .generate_linked_pr_plan(
                                    &filter,
                                    &args.branch,
                                    &args.pr_base_branch,
                                    args.pr_output.as_deref(),
                                )
                                .map_err(|e| miette::miette!("Linked PR manifest failed: {e}"))?,
                        )
                    };

                    if let Some(path) = pr_manifest.as_ref() {
                        println!("Linked PR manifest: {}", path.display());
                    } else {
                        println!(
                            "[DRY RUN] Linked PR plan would use head='{}' base='{}'",
                            args.branch, args.pr_base_branch
                        );
                    }

                    let failed = outcomes.iter().filter(|o| !o.success).count();
                    let success = outcomes.len().saturating_sub(failed);
                    let summary = format!(
                        "Staged apply completed: stages={} repos={} ok={} failed={} failed_batches={}",
                        stage_count,
                        outcomes.len(),
                        success,
                        failed,
                        failed_batches
                    );
                    let report = if failed == 0 {
                        CommandReport::ok("multi.apply", summary)
                    } else {
                        CommandReport::err("multi.apply", summary)
                    };
                    persist_mutation_audit(
                        "multi.apply",
                        dry_run,
                        &report.summary,
                        outcomes
                            .iter()
                            .map(|o| RepoOutcome {
                                repo: o.repo.clone(),
                                path: o.path.clone(),
                                success: o.success,
                                message: o.message.clone(),
                            })
                            .collect(),
                    );
                    Ok(report)
                }
                MultiCommands::Prs(args) => match &args.command {
                    MultiPrsCommands::Create(args) => {
                        let filter = to_filter(args.group.clone(), args.tags.clone());
                        if args.dry_run {
                            let ordered = registry
                                .dependency_order(&filter)
                                .map_err(|e| miette::miette!("PR plan failed: {e}"))?;
                            println!(
                                "[DRY RUN] Planned linked PR order (head={}, base={}):",
                                args.head_branch, args.base_branch
                            );
                            for (idx, repo) in ordered.iter().enumerate() {
                                println!("{}. {} | {}", idx + 1, repo.name, repo.path.display());
                            }
                            let report = CommandReport::ok(
                                "multi.prs.create",
                                format!("Dry-run planned {} repos for linked PRs", ordered.len()),
                            );
                            persist_mutation_audit(
                                "multi.prs.create",
                                true,
                                &report.summary,
                                ordered
                                    .iter()
                                    .map(|r| RepoOutcome {
                                        repo: r.name.clone(),
                                        path: r.path.display().to_string(),
                                        success: true,
                                        message: format!(
                                            "Planned linked PR head={} base={}",
                                            args.head_branch, args.base_branch
                                        ),
                                    })
                                    .collect(),
                            );
                            return Ok(report);
                        }

                        let manifest = registry
                            .generate_linked_pr_plan(
                                &filter,
                                &args.head_branch,
                                &args.base_branch,
                                args.output.as_deref(),
                            )
                            .map_err(|e| miette::miette!("PR plan generation failed: {e}"))?;
                        println!("Linked PR manifest: {}", manifest.display());
                        let report = CommandReport::ok(
                            "multi.prs.create",
                            format!("Generated linked PR manifest at {}", manifest.display()),
                        );
                        let outcomes =
                            resolve_multi_outcomes(args.group.clone(), args.tags.clone())
                                .into_iter()
                                .map(|mut o| {
                                    o.message =
                                        format!("Linked PR manifest: {}", manifest.display());
                                    o
                                })
                                .collect();
                        persist_mutation_audit(
                            "multi.prs.create",
                            false,
                            &report.summary,
                            outcomes,
                        );
                        Ok(report)
                    }
                },
            }
        }
        Commands::Agorg(args) => run_agorg(&args).await,
        Commands::Settings(args) => run_settings(args).await,
        Commands::Db(args) => run_db(&args).await,
        Commands::Bus(args) => run_bus(&args).await,
        Commands::Services(args) => run_services(&args).await,
        Commands::Serve(args) => {
            let cfg = bus::BusBridgeConfig {
                ws_url: args.ws_url.clone(),
                room: args.room.clone(),
                channel: args.channel.clone(),
                telemetry_channel: args.telemetry_channel.clone(),
                jwt_env: args.jwt_env.clone(),
                once: args.once,
            };
            if let Some(port) = args.ui_port {
                if args.ui_auto_start_bus {
                    maybe_autostart_local_bus_shim(&cfg.ws_url).await;
                }
                let bridge_cfg = cfg.clone();
                let allowed_commands = if args.ui_allow_commands.is_empty() {
                    None
                } else {
                    Some(
                        args.ui_allow_commands
                            .iter()
                            .cloned()
                            .collect::<HashSet<_>>(),
                    )
                };
                let ui_cfg = serve_ui::UiConfig {
                    host: args.ui_host.clone(),
                    port,
                    instance_id: args
                        .ui_instance_id
                        .clone()
                        .unwrap_or_else(|| format!("ui-{}", port)),
                    bus: cfg.clone(),
                    allow_mutations: args.ui_allow_mutations,
                    allowed_commands,
                };
                let bridge: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
                    let mut backoff_secs = 1u64;
                    loop {
                        match bus::run_bridge(&bridge_cfg).await {
                            Ok(()) if bridge_cfg.once => return Ok(()),
                            Ok(()) => {
                                eprintln!(
                                    "[pilot serve] bus bridge exited cleanly; reconnecting in {backoff_secs}s"
                                );
                            }
                            Err(err) => {
                                eprintln!(
                                    "[pilot serve] bus bridge unavailable: {err}; retrying in {backoff_secs}s"
                                );
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(10);
                    }
                });
                let ui = tokio::spawn(async move { serve_ui::run_ui_server(ui_cfg).await });
                let ui_res = ui
                    .await
                    .map_err(|e| miette::miette!("UI task failed: {e}"))?;
                bridge.abort();
                ui_res?;
            } else {
                bus::run_bridge(&cfg).await?;
            }
            Ok(CommandReport::ok(
                "serve",
                format!(
                    "Bridge exited (ws={}, room={}, channel={})",
                    cfg.ws_url, cfg.room, cfg.channel
                ),
            ))
        }
        Commands::Policy(args) => run_policy(args).await,
        Commands::Verify(args) => run_verify(args).await,
    }
}

async fn run_verify(args: &VerifyArgs) -> Result<CommandReport> {
    match &args.command {
        VerifyCommands::Chain => {
            let res = pilot_core::verify_audit_chain();
            if res.is_valid {
                println!(
                    "Audit chain is valid. {} events verified.",
                    res.audited_events
                );
                Ok(CommandReport::ok("verify.chain", "valid"))
            } else {
                for e in &res.errors {
                    eprintln!("ERROR: {}", e);
                }
                Err(miette::miette!(
                    "Audit chain verification failed with {} errors",
                    res.errors.len()
                ))
            }
        }
        VerifyCommands::Bundle { path } => {
            let p = PathBuf::from(&path);
            let result = pilot_core::verify_evidence_bundle(&p);

            if result.is_valid {
                println!("Bundle signature OK.");
                Ok(CommandReport::ok("verify.bundle", "valid"))
            } else {
                let off = result.offending_path.unwrap_or_else(|| "N/A".to_string());
                let err_msg = format!(
                    "Bundle verification failed. Reason: {} | Path: {} | Details: {}",
                    result.reason_code, off, result.details
                );
                eprintln!("ERROR: {}", err_msg);
                Err(miette::miette!("{}", err_msg))
            }
        }
        VerifyCommands::Artifact { path } => {
            let p = PathBuf::from(&path);
            if pilot_core::verify_artifact_hash(&p) {
                println!("Artifact signature OK: {}", path);
                Ok(CommandReport::ok("verify.artifact", "valid"))
            } else {
                Err(miette::miette!(
                    "Artifact signature mismatch or sidecar missing: {}",
                    path
                ))
            }
        }
    }
}

fn to_filter(group: Option<String>, tags: Vec<String>) -> multi::RepoFilter {
    multi::RepoFilter { group, tags }
}

fn resolve_secure_targets(group: Option<String>, tags: Vec<String>) -> Result<Vec<PathBuf>> {
    let has_filter = group.is_some() || !tags.is_empty();
    let filter = to_filter(group, tags);
    let db_path = multi::MultiRegistry::default_db_path();

    if let Ok(registry) = multi::MultiRegistry::open(&db_path) {
        let repos = registry
            .list_repos(&filter)
            .map_err(|e| miette::miette!("Secure target selection failed: {e}"))?;
        if !repos.is_empty() {
            return Ok(repos.into_iter().map(|r| r.path).collect());
        }
    }

    if has_filter {
        return Err(miette::miette!(
            "No registered repositories match the selected group/tags"
        ));
    }

    Ok(vec![std::env::current_dir().into_diagnostic()?])
}

fn resolve_multi_outcomes(group: Option<String>, tags: Vec<String>) -> Vec<RepoOutcome> {
    let db_path = multi::MultiRegistry::default_db_path();
    let filter = to_filter(group, tags);
    let repos = multi::MultiRegistry::open(&db_path)
        .and_then(|r| r.list_repos(&filter))
        .unwrap_or_default();
    repos
        .into_iter()
        .map(|r| RepoOutcome {
            repo: r.name,
            path: r.path.display().to_string(),
            success: true,
            message: "Selected for operation".to_string(),
        })
        .collect()
}

async fn maybe_autostart_local_bus_shim(ws_url: &str) {
    let is_default_local_ws =
        ws_url.starts_with("ws://127.0.0.1:9100") || ws_url.starts_with("ws://localhost:9100");
    if !is_default_local_ws {
        return;
    }
    let script = PathBuf::from("./scripts/arqonbus_shim.sh");
    if !script.exists() {
        eprintln!(
            "[pilot serve] local ws-url detected but {} not found; skipping auto-start",
            script.display()
        );
        return;
    }
    let cmd = bus_shim_command("start");
    match tokio::process::Command::new("bash")
        .arg("-lc")
        .arg(cmd)
        .output()
        .await
    {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let line = stdout.trim();
            if !line.is_empty() {
                eprintln!("[pilot serve] bus auto-start: {line}");
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!(
                "[pilot serve] bus auto-start failed (non-fatal): {}",
                stderr.trim()
            );
        }
        Err(err) => {
            eprintln!("[pilot serve] bus auto-start failed (non-fatal): {}", err);
        }
    }
}

fn persist_mutation_audit(command: &str, dry_run: bool, summary: &str, outcomes: Vec<RepoOutcome>) {
    let repo_count = outcomes.len();
    let failures = outcomes.iter().filter(|o| !o.success).count();
    let artifact_path = write_repo_outcomes_artifact(command, &outcomes)
        .map(|p| p.display().to_string())
        .ok();

    let repos: Vec<String> = outcomes.iter().map(|o| o.repo.clone()).collect();
    let event = AuditEvent {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: String::new(),
        scope_id: None, // Scope is implicit or not available here
        domain: "command".to_string(),
        action: command.to_string(),
        dry_run,
        success: failures == 0,
        summary: summary.to_string(),
        repo_count,
        failures,
        artifact_path: artifact_path.clone(),
        repos,
        content_hash: None,
        prev_hash: None,
        details: serde_json::json!({}),
    };

    if let Ok(audit_path) = append_audit_event(event) {
        if let Some(artifact) = artifact_path {
            println!(
                "Audit recorded: {} | outcomes: {}",
                audit_path.display(),
                artifact
            );
        } else {
            println!("Audit recorded: {}", audit_path.display());
        }
    } else {
        eprintln!("Warning: failed to write audit log for {}", command);
    }
}

fn run_navigate_single(args: &NavigateArgs) -> Result<CommandReport> {
    let root = std::env::current_dir().into_diagnostic()?;
    let ctx = RepoContext::new(root.clone());
    println!("Starting release pipeline...");

    if !args.skip_checks {
        let checker = navigate::ConstitutionCheck::new(ctx.root.clone());
        if !checker.run_all().map_err(|e| miette::miette!("{:?}", e))? {
            println!("Constitution checks failed. Use --skip-checks to override.");
            std::process::exit(1);
        }
    }

    let parser = navigate::CommitParser::new(ctx.root.clone());
    let commits = parser
        .get_commits_since_last_tag()
        .map_err(|e| miette::miette!("{:?}", e))?;

    let current_version = navigate::SemVer::from_cargo_toml(&ctx.root.join("Cargo.toml"))?;
    let next_version = navigate::calculate_next_version(&current_version, &commits);
    let changelog = navigate::generate_changelog(&next_version, &commits);

    println!("Next version: v{}", next_version);
    println!("\nChangelog:\n{}", changelog);

    if args.dry_run {
        println!(
            "\n[DRY RUN] Would update root Cargo.toml to v{}",
            next_version
        );
        println!("[DRY RUN] Would create release PR");
        Ok(CommandReport::ok(
            "navigate",
            format!("Dry run complete: next version v{}", next_version),
        ))
    } else {
        next_version.write_to_cargo_toml(&ctx.root.join("Cargo.toml"))?;
        println!("[SUCCESS] Updated root Cargo.toml to v{}", next_version);

        use navigate::git::parse_git_remote;
        use navigate::github::GitHubClient;

        let repo_info = parse_git_remote(&ctx.root).map_err(|e| miette::miette!("{:?}", e))?;

        let client = GitHubClient::new(&repo_info.owner, &repo_info.repo)
            .map_err(|e| miette::miette!("{:?}", e))?;
        let title = format!("chore: release v{}", next_version);
        let body = format!("## Release v{}\n\n{}", next_version, changelog);

        let url = client
            .create_release_pr(&title, &body, &args.head_branch, &args.base_branch)
            .map_err(|e| miette::miette!("{:?}", e))?;

        println!("\n[SUCCESS] Created Release PR: {}", url);
        Ok(CommandReport::ok(
            "navigate",
            format!("Created release PR for v{}", next_version),
        ))
    }
}

fn run_navigate_multi(args: &NavigateArgs) -> Result<CommandReport> {
    let db_path = multi::MultiRegistry::default_db_path();
    let registry = multi::MultiRegistry::open(&db_path)
        .map_err(|e| miette::miette!("Failed to open multi registry: {e}"))?;
    let filter = to_filter(args.group.clone(), args.tags.clone());

    if args.dry_run {
        let ordered = registry
            .dependency_order(&filter)
            .map_err(|e| miette::miette!("Multi navigate planning failed: {e}"))?;
        println!(
            "[DRY RUN] Coordinated release order (head={}, base={}):",
            args.head_branch, args.base_branch
        );
        for (idx, repo) in ordered.iter().enumerate() {
            println!("{}. {} | {}", idx + 1, repo.name, repo.path.display());
        }
        return Ok(CommandReport::ok(
            "navigate.multi",
            format!(
                "Dry-run planned coordinated release for {} repos",
                ordered.len()
            ),
        ));
    }

    let manifest = registry
        .generate_linked_pr_plan(
            &filter,
            &args.head_branch,
            &args.base_branch,
            args.plan_output.as_deref(),
        )
        .map_err(|e| miette::miette!("Multi navigate manifest generation failed: {e}"))?;

    println!("Coordinated release manifest: {}", manifest.display());
    Ok(CommandReport::ok(
        "navigate.multi",
        format!(
            "Prepared coordinated release manifest at {}",
            manifest.display()
        ),
    ))
}

fn emit_report(report: &CommandReport, as_json: bool) -> Result<()> {
    if as_json {
        println!(
            "{}",
            serde_json::to_string(report)
                .into_diagnostic()
                .context("Failed to serialize command report")?
        );
    }
    Ok(())
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Init => "init",
        Commands::Oracle(OracleArgs {
            command: OracleCommands::Scan,
        }) => "oracle.scan",
        Commands::Oracle(OracleArgs {
            command: OracleCommands::Query(_),
        }) => "oracle.query",
        Commands::Heal(_) => "heal",
        Commands::Secure(SecureArgs {
            command: SecureCommands::Scan(_),
        }) => "secure.scan",
        Commands::Secure(SecureArgs {
            command: SecureCommands::Fix(_),
        }) => "secure.fix",
        Commands::Plan(PlanArgs {
            command: PlanCommands::Issues(_),
        }) => "plan.issues",
        Commands::Plan(PlanArgs {
            command: PlanCommands::Score(_),
        }) => "plan.score",
        Commands::Plan(PlanArgs {
            command: PlanCommands::Roadmap(_),
        }) => "plan.roadmap",
        Commands::Create(CreateArgs {
            command: CreateCommands::Feature(_),
        }) => "create.feature",
        Commands::Create(CreateArgs {
            command: CreateCommands::Tests(_),
        }) => "create.tests",
        Commands::Know(KnowArgs {
            command: KnowCommands::Record(_),
        }) => "know.record",
        Commands::Know(KnowArgs {
            command: KnowCommands::Query(_),
        }) => "know.query",
        Commands::Navigate(_) => "navigate",
        Commands::Branch(BranchArgs {
            command: BranchCommands::Create(_),
        }) => "branch.create",
        Commands::Branch(BranchArgs {
            command: BranchCommands::Sync(_),
        }) => "branch.sync",
        Commands::Branch(BranchArgs {
            command: BranchCommands::Status(_),
        }) => "branch.status",
        Commands::Branch(BranchArgs {
            command: BranchCommands::Prune(_),
        }) => "branch.prune",
        Commands::Multi(MultiArgs {
            command: MultiCommands::Register(_),
        }) => "multi.register",
        Commands::Multi(MultiArgs {
            command: MultiCommands::List(_),
        }) => "multi.list",
        Commands::Multi(MultiArgs {
            command: MultiCommands::Status(_),
        }) => "multi.status",
        Commands::Multi(MultiArgs {
            command: MultiCommands::Query(_),
        }) => "multi.query",
        Commands::Multi(MultiArgs {
            command:
                MultiCommands::Deps(MultiDepsArgs {
                    command: MultiDepsCommands::Set(_),
                }),
        }) => "multi.deps.set",
        Commands::Multi(MultiArgs {
            command: MultiCommands::Order(_),
        }) => "multi.order",
        Commands::Multi(MultiArgs {
            command: MultiCommands::Dag(_),
        }) => "multi.dag",
        Commands::Multi(MultiArgs {
            command: MultiCommands::Apply(_),
        }) => "multi.apply",
        Commands::Multi(MultiArgs {
            command:
                MultiCommands::Prs(MultiPrsArgs {
                    command: MultiPrsCommands::Create(_),
                }),
        }) => "multi.prs.create",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Create(_),
        }) => "agorg.create",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::CreateProject(_),
        }) => "agorg.create_project",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::List,
        }) => "agorg.list",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Show,
        }) => "agorg.show",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Use(_),
        }) => "agorg.use",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Update(_),
        }) => "agorg.update",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Delete(_),
        }) => "agorg.delete",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Discover(_),
        }) => "agorg.discover",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Tree(_),
        }) => "agorg.tree",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Link(_),
        }) => "agorg.link",
        Commands::Agorg(AgorgArgs {
            command: AgorgCommands::Reconcile(_),
        }) => "agorg.reconcile",
        Commands::Db(DbArgs {
            command: DbCommands::Ensure,
        }) => "db.ensure",
        Commands::Db(DbArgs {
            command: DbCommands::Start,
        }) => "db.start",
        Commands::Db(DbArgs {
            command: DbCommands::Stop,
        }) => "db.stop",
        Commands::Db(DbArgs {
            command: DbCommands::Status,
        }) => "db.status",
        Commands::Bus(BusArgs {
            command: BusCommands::Start,
        }) => "bus.start",
        Commands::Bus(BusArgs {
            command: BusCommands::Stop,
        }) => "bus.stop",
        Commands::Bus(BusArgs {
            command: BusCommands::Restart,
        }) => "bus.restart",
        Commands::Bus(BusArgs {
            command: BusCommands::Status,
        }) => "bus.status",
        Commands::Services(ServicesArgs {
            command: ServicesCommands::Start,
        }) => "services.start",
        Commands::Services(ServicesArgs {
            command: ServicesCommands::Stop,
        }) => "services.stop",
        Commands::Services(ServicesArgs {
            command: ServicesCommands::Restart,
        }) => "services.restart",
        Commands::Services(ServicesArgs {
            command: ServicesCommands::Status,
        }) => "services.status",
        Commands::Serve(_) => "serve",
        Commands::Settings(_) => "settings.branch",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::List { .. },
        }) => "policy.list",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::Get { .. },
        }) => "policy.get",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::SetDraft { .. },
        }) => "policy.set_draft",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::Preview { .. },
        }) => "policy.preview",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::Approve { .. },
        }) => "policy.approve",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::Activate { .. },
        }) => "policy.activate",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::Delete { .. },
        }) => "policy.delete",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::Resolve { .. },
        }) => "policy.resolve",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::Scan { .. },
        }) => "policy.scan",
        Commands::Policy(PolicyArgs {
            command: PolicyCommands::Decisions { .. },
        }) => "policy.decisions",
        Commands::Verify(_) => "verify",
        Commands::Policy(PolicyArgs {
            command:
                PolicyCommands::Exceptions(PolicyExceptionsArgs {
                    command: PolicyExceptionsCommands::List { .. },
                }),
        }) => "policy.exceptions.list",
        Commands::Policy(PolicyArgs {
            command:
                PolicyCommands::Exceptions(PolicyExceptionsArgs {
                    command: PolicyExceptionsCommands::Add(_),
                }),
        }) => "policy.exceptions.add",
        Commands::Policy(PolicyArgs {
            command:
                PolicyCommands::Exceptions(PolicyExceptionsArgs {
                    command: PolicyExceptionsCommands::Delete { .. },
                }),
        }) => "policy.exceptions.delete",
    }
}

async fn run_db(args: &DbArgs) -> Result<CommandReport> {
    let store = AgorgStore::from_instance("default".to_string());
    match args.command {
        DbCommands::Ensure | DbCommands::Start => {
            println!("Ensuring managed PostgreSQL is running...");
            match store.ensure_managed_db().await {
                Ok(Some(status)) => {
                    println!("Managed DB is running!");
                    println!("DSN: {}", status.dsn);
                    Ok(CommandReport::ok(
                        "db.ensure",
                        "Managed Postgres is running",
                    ))
                }
                Ok(None) => {
                    println!("Managed DB starting is disabled (PILOT_AGORG_DATABASE_URL is set)");
                    Ok(CommandReport::ok(
                        "db.ensure",
                        "Managed Postgres explicitly disabled",
                    ))
                }
                Err(e) => Err(miette!("Failed to ensure database: {}", e)),
            }
        }
        DbCommands::Stop => match store.stop_managed_db().await {
            Ok(Some(_)) => {
                println!("Managed PostgreSQL stopped.");
                Ok(CommandReport::ok(
                    "db.stop",
                    "Managed Postgres stopped gracefully",
                ))
            }
            Ok(None) => {
                println!("Managed PostgreSQL was not running.");
                Ok(CommandReport::ok("db.stop", "Managed Postgres not running"))
            }
            Err(e) => Err(miette!("Failed to stop database: {}", e)),
        },
        DbCommands::Status => match store.managed_db_status().await {
            Ok(Some(status)) => {
                println!(
                    "Status: {}",
                    if status.running { "RUNNING" } else { "STOPPED" }
                );
                if status.running {
                    println!("DSN: {}", status.dsn);
                }
                Ok(CommandReport::ok(
                    "db.status",
                    "Managed Postgres status retrieved",
                ))
            }
            Ok(None) => {
                println!("Status: DISABLED (PILOT_AGORG_DATABASE_URL is set)");
                Ok(CommandReport::ok(
                    "db.status",
                    "Managed Postgres explicitly disabled",
                ))
            }
            Err(e) => Err(miette!("Failed to check database status: {}", e)),
        },
    }
}

async fn run_bus(args: &BusArgs) -> Result<CommandReport> {
    let action = match args.command {
        BusCommands::Start => "start",
        BusCommands::Stop => "stop",
        BusCommands::Restart => "restart",
        BusCommands::Status => "status",
    };

    let cmd = shim_runtime::bus_shim_command(action);
    let (code, out, err) = serve_ui::run_local_script(&cmd)
        .await
        .map_err(|e| miette!("Failed to execute bus shim: {}", e))?;

    println!("{}", out);
    if !err.is_empty() {
        eprintln!("{}", err);
    }

    if code != 0 {
        Err(miette!("Bus {} failed with code {}", action, code))
    } else {
        Ok(CommandReport::ok(
            &format!("bus.{}", action),
            &format!("Bus {} ok", action),
        ))
    }
}

async fn run_services(args: &ServicesArgs) -> Result<CommandReport> {
    let policy = crate::service_supervisor::RetryPolicy::default();
    let store = AgorgStore::from_instance("default".to_string());

    match args.command {
        ServicesCommands::Stop => {
            println!("Stopping ArqonBus...");
            let cmd = shim_runtime::bus_shim_command("stop");
            let _ = serve_ui::run_local_script(&cmd).await;

            println!("Stopping Managed DB...");
            let _ = store.stop_managed_db().await;

            println!("Services stopped.");
            Ok(CommandReport::ok("services.stop", "Services stopped"))
        }
        ServicesCommands::Start => {
            println!("Starting Managed DB...");
            crate::service_supervisor::supervised_start(
                "Managed Database",
                || {
                    let s = store.clone();
                    async move {
                        s.ensure_managed_db().await.map_err(|e| e.to_string())?;
                        ensure_db_stable(&s).await.map_err(|e| e.to_string())
                    }
                },
                policy.clone(),
            )
            .await?;

            println!("Starting ArqonBus...");
            crate::service_supervisor::supervised_start(
                "ArqonBus",
                || async {
                    let cmd = shim_runtime::bus_shim_command("start");
                    let (code, out, err) = serve_ui::run_local_script(&cmd)
                        .await
                        .map_err(|e| e.to_string())?;
                    if code != 0 {
                        return Err(err);
                    }
                    if !serve_ui::bus_shim_running(&out, &err) {
                        return Err("Not running".into());
                    }
                    ensure_bus_stable().await.map_err(|e| e.to_string())?;
                    Ok((code, out, err))
                },
                policy,
            )
            .await?;

            println!("Services started successfully.");
            Ok(CommandReport::ok("services.start", "Services started"))
        }
        ServicesCommands::Restart => {
            println!("Stopping ArqonBus...");
            let _ = serve_ui::run_local_script(&shim_runtime::bus_shim_command("stop")).await;
            println!("Stopping Managed DB...");
            let _ = store.stop_managed_db().await;

            println!("Restarting Managed DB...");
            crate::service_supervisor::supervised_start(
                "Managed Database",
                || {
                    let s = store.clone();
                    async move {
                        s.ensure_managed_db().await.map_err(|e| e.to_string())?;
                        ensure_db_stable(&s).await.map_err(|e| e.to_string())
                    }
                },
                policy.clone(),
            )
            .await?;

            println!("Restarting ArqonBus...");
            crate::service_supervisor::supervised_start(
                "ArqonBus",
                || async {
                    let cmd = shim_runtime::bus_shim_command("start");
                    let (code, out, err) = serve_ui::run_local_script(&cmd)
                        .await
                        .map_err(|e| e.to_string())?;
                    if code != 0 {
                        return Err(err);
                    }
                    if !serve_ui::bus_shim_running(&out, &err) {
                        return Err("Not running".into());
                    }
                    ensure_bus_stable().await.map_err(|e| e.to_string())?;
                    Ok((code, out, err))
                },
                policy,
            )
            .await?;

            println!("Services restarted successfully.");
            Ok(CommandReport::ok("services.restart", "Services restarted"))
        }
        ServicesCommands::Status => {
            let db = store.managed_db_status().await.unwrap_or(None);
            let cmd = shim_runtime::bus_shim_command("status");
            let bus = serve_ui::run_local_script(&cmd).await;

            println!("--- DB Status ---");
            if let Some(st) = db {
                println!("Running: {}", st.running);
                if st.running {
                    println!("DSN: {}", st.dsn);
                }
            } else {
                println!("DISABLED (PILOT_AGORG_DATABASE_URL is set)");
            }

            println!("\n--- Bus Status ---");
            if let Ok((_code, out, err)) = bus {
                print!("{}", out);
                if !err.is_empty() {
                    print!("{}", err);
                }
            } else {
                println!("Status check failed");
            }
            Ok(CommandReport::ok("services.status", "Status retrieved"))
        }
    }
}

async fn ensure_db_stable(store: &AgorgStore) -> Result<()> {
    // Catch quick-crash cases where startup succeeds but DB exits moments later.
    for _ in 0..5 {
        if let Ok(Some(status)) = store.managed_db_status().await {
            if status.running {
                // First successful check - DB is stable
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
    }
    Err(miette!("Managed DB failed stability probe (not running)"))
}

async fn ensure_bus_stable() -> Result<()> {
    // Catch quick-crash bus processes and force supervisor retry.
    for _ in 0..5 {
        let cmd = shim_runtime::bus_shim_command("status");
        let (code, out, err) = serve_ui::run_local_script(&cmd)
            .await
            .map_err(|e| miette!("Bus stability probe failed: {}", e))?;
        if code != 0 || !serve_ui::bus_shim_running(&out, &err) {
            return Err(miette!(
                "ArqonBus failed stability probe: code={} out='{}' err='{}'",
                code,
                out.trim(),
                err.trim()
            ));
        }
        // First successful check - bus is stable
        return Ok(());
    }
    Ok(())
}

async fn run_settings(args: &SettingsArgs) -> Result<CommandReport> {
    match &args.command {
        SettingsCommands::Branch { show } => {
            if *show {
                let agorg_store = AgorgStore::from_env();
                agorg_store.initialize().await?;
                let gov_store = governance::store::GovernanceStore::new(agorg_store.dsn());

                match agorg_store.get_active_agorg().await {
                    Ok(Some(active)) => match gov_store.get_policy(active.id, "branch").await {
                        Ok(Some(pol)) => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&pol.policy_json).into_diagnostic()?
                            );
                        }
                        Ok(None) => {
                            let default_policy = governance::model::BranchPolicy::default();
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&default_policy).into_diagnostic()?
                            );
                        }
                        Err(e) => return Err(miette::miette!("Error loading policy: {}", e)),
                    },
                    Ok(None) => {
                        let default_policy = governance::model::BranchPolicy::default();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&default_policy).into_diagnostic()?
                        );
                    }
                    Err(e) => return Err(miette::miette!("Error loading active AGOrg: {}", e)),
                }
                Ok(CommandReport::ok(
                    "settings.branch",
                    "Printed active branch policy".to_string(),
                ))
            } else {
                Err(miette::miette!("Usage: pilot settings branch --show"))
            }
        }
    }
}

async fn run_agorg(args: &AgorgArgs) -> Result<CommandReport> {
    let store = AgorgStore::from_env();
    store.initialize().await?;
    match &args.command {
        AgorgCommands::Create(args) => {
            let parent = resolve_agorg_ref_optional(&store, args.parent.as_deref()).await?;
            let ag = store
                .create_agorg(
                    &args.name,
                    &args.root,
                    args.master.as_deref(),
                    parent,
                    args.scan_depth as i32,
                    args.default_scope,
                )
                .await?;
            println!(
                "Created AGOrg: {} ({}) root={} default_scope={}",
                ag.name, ag.id, ag.root_path, ag.default_scope
            );
            Ok(CommandReport::ok(
                "agorg.create",
                format!("Created AGOrg {}", ag.name),
            ))
        }
        AgorgCommands::CreateProject(args) => {
            let parent = resolve_agorg_ref_optional(&store, args.parent.as_deref()).await?;
            let (ag, discovered) = store
                .create_project(
                    &args.name,
                    &args.root,
                    args.master.as_deref(),
                    parent,
                    args.scan_depth,
                    args.autoscan,
                    args.default_scope,
                )
                .await?;
            println!(
                "Created AGOrg project: {} ({}) root={}",
                ag.name, ag.id, ag.root_path
            );
            if args.import {
                if let Some(scan) = discovered.as_ref() {
                    let summary = store
                        .import_discovery_with_options(ag.id, scan, args.prune_missing)
                        .await?;
                    println!(
                        "Imported discovery into {} (upserted={}, pruned={}, final={})",
                        ag.id, summary.upserted, summary.pruned, summary.final_count
                    );
                } else {
                    let scan = agorg::discover_hierarchy(&args.root, args.scan_depth)?;
                    let summary = store
                        .import_discovery_with_options(ag.id, &scan, args.prune_missing)
                        .await?;
                    println!(
                        "Imported discovery into {} (upserted={}, pruned={}, final={})",
                        ag.id, summary.upserted, summary.pruned, summary.final_count
                    );
                }
            }
            Ok(CommandReport::ok(
                "agorg.create_project",
                format!("Created AGOrg project {}", ag.name),
            ))
        }
        AgorgCommands::List => {
            let list = store.list_agorgs().await?;
            for item in &list {
                println!(
                    "{} | {} | root={} | parent={:?} | default={} | depth={}",
                    item.id,
                    item.name,
                    item.root_path,
                    item.parent_agorg_id,
                    item.default_scope,
                    item.scan_depth
                );
            }
            Ok(CommandReport::ok(
                "agorg.list",
                format!("Listed {} AGOrgs", list.len()),
            ))
        }
        AgorgCommands::Show => {
            let active = store.get_active_agorg().await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&active).into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "agorg.show",
                if active.is_some() {
                    "Active AGOrg found".to_string()
                } else {
                    "No active AGOrg set".to_string()
                },
            ))
        }
        AgorgCommands::Use(args) => {
            let id = resolve_agorg_ref(&store, &args.agorg).await?;
            store.set_active_agorg(id).await?;
            println!("Active AGOrg set to {}", id);
            Ok(CommandReport::ok(
                "agorg.use",
                format!("Active AGOrg set {}", id),
            ))
        }
        AgorgCommands::Update(args) => {
            let id = resolve_agorg_ref(&store, &args.agorg).await?;
            let ag = store
                .update_agorg(
                    id,
                    args.name.clone(),
                    args.root.clone(),
                    args.master.clone(),
                    args.scan_depth.map(|d| d as i32),
                    if args.default_scope { Some(true) } else { None },
                )
                .await?;
            println!(
                "Updated AGOrg: {} ({}) root={} default_scope={} depth={}",
                ag.name, ag.id, ag.root_path, ag.default_scope, ag.scan_depth
            );
            Ok(CommandReport::ok(
                "agorg.update",
                format!("Updated AGOrg {}", ag.id),
            ))
        }
        AgorgCommands::Delete(args) => {
            let id = resolve_agorg_ref(&store, &args.agorg).await?;
            let deleted = store.delete_agorg(id).await?;
            println!("Deleted {} AGOrg rows", deleted);
            Ok(CommandReport::ok(
                "agorg.delete",
                format!("Deleted {} row(s)", deleted),
            ))
        }
        AgorgCommands::Discover(args) => {
            let scan = agorg::discover_hierarchy(&args.root, args.depth)?;
            if let Some(target) = &args.import_to {
                let id = resolve_agorg_ref(&store, target).await?;
                let summary = store
                    .import_discovery_with_options(id, &scan, args.prune_missing)
                    .await?;
                println!(
                    "Imported discovery into {} (upserted={}, pruned={}, final={})",
                    id, summary.upserted, summary.pruned, summary.final_count
                );
            }
            println!("{}", serde_json::to_string_pretty(&scan).into_diagnostic()?);
            Ok(CommandReport::ok(
                "agorg.discover",
                format!("Discovered {} candidates", scan.candidates.len()),
            ))
        }
        AgorgCommands::Tree(args) => {
            let root = match &args.root {
                Some(v) => Some(resolve_agorg_ref(&store, v).await?),
                None => None,
            };
            let tree = store.tree(root).await?;
            println!("{}", serde_json::to_string_pretty(&tree).into_diagnostic()?);
            Ok(CommandReport::ok(
                "agorg.tree",
                format!("Rendered {} root tree nodes", tree.len()),
            ))
        }
        AgorgCommands::Link(args) => {
            let parent = resolve_agorg_ref(&store, &args.parent).await?;
            let child = resolve_agorg_ref(&store, &args.child).await?;
            store.link_agorgs(parent, child).await?;
            println!("Linked AGOrg {} -> {}", parent, child);
            Ok(CommandReport::ok(
                "agorg.link",
                format!("Linked {} -> {}", parent, child),
            ))
        }
        AgorgCommands::Reconcile(args) => {
            let id = match args.agorg.as_deref() {
                Some(v) => resolve_agorg_ref(&store, v).await?,
                None => {
                    store
                        .get_active_agorg()
                        .await?
                        .ok_or_else(|| {
                            miette!("No active AGOrg; pass --agorg or set active scope")
                        })?
                        .id
                }
            };
            let report = store.reconcile_agorg(id).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report).into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "agorg.reconcile",
                format!(
                    "Reconciled AGOrg {} (issues={}, off_policy={})",
                    report.agorg_id, report.issue_count, report.off_policy_count
                ),
            ))
        }
    }
}

async fn resolve_agorg_ref(store: &AgorgStore, input: &str) -> Result<uuid::Uuid> {
    if let Ok(id) = uuid::Uuid::parse_str(input) {
        if store.get_agorg(id).await?.is_some() {
            return Ok(id);
        }
        return Err(miette!("AGOrg UUID {} not found", id));
    }
    let canonical_input = fs::canonicalize(input)
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| input.to_string());
    let list = store.list_agorgs().await?;
    let mut found = list
        .into_iter()
        .filter(|a| {
            a.name.eq_ignore_ascii_case(input)
                || a.root_path == input
                || a.root_path == canonical_input
        })
        .collect::<Vec<_>>();
    if found.is_empty() {
        return Err(miette!(
            "AGOrg '{}' not found (expected UUID, name, or root path)",
            input
        ));
    }
    if found.len() > 1 {
        return Err(miette!(
            "AGOrg name '{}' is ambiguous; use UUID instead",
            input
        ));
    }
    Ok(found.remove(0).id)
}

fn validate_policy_payload(kind: &str, payload: &serde_json::Value) -> Result<()> {
    match kind {
        "branch" => {
            let _: governance::model::BranchPolicy = parse_policy_for_kind(kind, payload)?;
        }
        "dependency" => {
            let _: governance::model::DependencyPolicy = parse_policy_for_kind(kind, payload)?;
        }
        "release" => {
            let _: governance::model::ReleasePolicy = parse_policy_for_kind(kind, payload)?;
        }
        "security" => {
            let _: governance::model::SecurityPolicy = parse_policy_for_kind(kind, payload)?;
        }
        "quality" => {
            let _: governance::model::QualityPolicy = parse_policy_for_kind(kind, payload)?;
        }
        "runtime" => {
            let _: governance::model::RuntimePolicy = parse_policy_for_kind(kind, payload)?;
        }
        "operator_routine" => {
            let _: governance::model::OperatorRoutinePolicy = parse_policy_for_kind(kind, payload)?;
        }
        other => {
            return Err(miette!(
                "Unsupported policy kind '{}'. Supported kinds: branch, dependency, release, security, quality, runtime, operator_routine",
                other
            ));
        }
    }
    Ok(())
}

fn parse_policy_for_kind<T>(kind: &str, payload: &serde_json::Value) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value::<T>(payload.clone()).map_err(|e| {
        miette!(
            "Invalid {} policy payload schema: {}. Use `pilot policy get --kind {}` to inspect the expected shape.",
            kind,
            e,
            kind
        )
    })
}

async fn resolve_agorg_ref_optional(
    store: &AgorgStore,
    input: Option<&str>,
) -> Result<Option<uuid::Uuid>> {
    if let Some(v) = input {
        Ok(Some(resolve_agorg_ref(store, v).await?))
    } else {
        Ok(None)
    }
}

fn canonicalize_path_lossy(path: &Path) -> String {
    fs::canonicalize(path)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn path_in_any_root(path: &str, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| {
        let root_s = canonicalize_path_lossy(root);
        path == root_s || path.starts_with(&(root_s + "/"))
    })
}

fn now_stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

fn handle_init(config_path: &Path) -> Result<()> {
    if config_path.exists() {
        println!("Config file already exists at {:?}", config_path);
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .into_diagnostic()
            .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
    }

    let default_config = Config::default();
    let toml_string = toml::to_string_pretty(&default_config)
        .into_diagnostic()
        .context("Failed to serialize default config")?;

    fs::write(config_path, toml_string)
        .into_diagnostic()
        .with_context(|| format!("Failed to write config file to {:?}", config_path))?;

    println!("Initialized ArqonPilot configuration at {:?}", config_path);
    Ok(())
}

async fn run_policy(args: &PolicyArgs) -> Result<CommandReport> {
    let agorg_store = AgorgStore::from_env();
    agorg_store.initialize().await?;
    let active = agorg_store
        .get_active_agorg()
        .await?
        .ok_or_else(|| miette::miette!("No active AGOrg"))?;
    let gov_store = governance::store::GovernanceStore::new(agorg_store.dsn());

    match &args.command {
        PolicyCommands::List {
            kind,
            ago_path,
            limit,
        } => {
            let records = gov_store
                .list_policy_versions(active.id, ago_path.as_deref(), kind, *limit)
                .await?;
            let out: Vec<_> = records
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "agorg_id": r.agorg_id,
                        "ago_path": r.ago_path,
                        "policy_kind": r.policy_kind,
                        "version": r.version,
                        "status": r.status,
                        "updated_at": r.updated_at,
                        "updated_by": r.updated_by
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "kind": kind,
                    "ago_path": ago_path,
                    "count": out.len(),
                    "items": out
                }))
                .into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "policy.list",
                format!("Listed {} policy version(s)", out.len()),
            ))
        }
        PolicyCommands::Get { kind, ago_path } => {
            let record = if let Some(path) = ago_path {
                gov_store
                    .get_ago_policy_override(active.id, path, kind)
                    .await?
            } else {
                gov_store.get_policy(active.id, kind).await?
            };
            if let Some(r) = record {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&r.policy_json).into_diagnostic()?
                );
            } else {
                println!("No {} policy found (ago_path: {:?})", kind, ago_path);
            }
            Ok(CommandReport::ok(
                "policy.get",
                format!("Fetched {} policy", kind),
            ))
        }
        PolicyCommands::SetDraft { kind, file } => {
            let content = std::fs::read_to_string(file).into_diagnostic()?;
            let json: serde_json::Value = serde_json::from_str(&content).into_diagnostic()?;
            validate_policy_payload(kind, &json)?;
            let saved = gov_store
                .save_policy(active.id, None, kind, &json, "draft", "pilot-cli")
                .await?;
            println!("Saved draft policy {} version {}", kind, saved.version);
            let report = CommandReport::ok(
                "policy.set_draft",
                format!("Saved draft v{}", saved.version),
            );
            persist_mutation_audit("policy.set_draft", false, &report.summary, vec![]);
            Ok(report)
        }
        PolicyCommands::Preview { kind, version } => {
            if kind != "branch"
                && kind != "dependency"
                && kind != "release"
                && kind != "security"
                && kind != "quality"
                && kind != "runtime"
                && kind != "operator_routine"
            {
                return Err(miette::miette!(
                    "Preview currently supports branch, dependency, release, security, quality, runtime, operator_routine policies"
                ));
            }
            let record = gov_store
                .get_policy_by_version(active.id, None, kind, *version as i32)
                .await?
                .ok_or_else(|| miette::miette!("No {} policy version {} found", kind, version))?;

            let roots = vec![std::path::PathBuf::from(&active.root_path)];
            let registry = multi::MultiRegistry::open(&multi::MultiRegistry::default_db_path())
                .map_err(|e| miette::miette!("Failed to open multi registry: {}", e))?;
            let mut repos = registry
                .list_repos(&multi::RepoFilter::default())
                .map_err(|e| miette::miette!("Failed listing repos for preview: {}", e))?;
            repos.retain(|repo| {
                let path = canonicalize_path_lossy(&repo.path);
                path_in_any_root(&path, &roots)
            });
            let exceptions = gov_store.get_effective_exceptions(active.id, kind).await?;
            let statuses = branch::branch_status(&repos);
            let mut violations = 0usize;
            let mut warnings = 0usize;
            let mut blocked = 0usize;
            let mut evaluations: Vec<serde_json::Value> = Vec::with_capacity(statuses.len());
            for st in &statuses {
                let path = std::path::Path::new(&st.path);
                let ago_path = path
                    .strip_prefix(std::path::Path::new(&active.root_path))
                    .unwrap_or(path)
                    .display()
                    .to_string();
                let source_name = "Preview";
                let source_id = Some(active.id);
                let eval = match kind.as_str() {
                    "branch" => {
                        let policy: governance::model::BranchPolicy =
                            parse_policy_for_kind(kind, &record.policy_json)?;
                        governance::eval::evaluate_branch_policy(
                            &policy,
                            "create",
                            &st.current_branch,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "dependency" => {
                        let policy: governance::model::DependencyPolicy =
                            parse_policy_for_kind(kind, &record.policy_json)?;
                        governance::eval::evaluate_dependency_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "release" => {
                        let policy: governance::model::ReleasePolicy =
                            parse_policy_for_kind(kind, &record.policy_json)?;
                        governance::eval::evaluate_release_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "security" => {
                        let policy: governance::model::SecurityPolicy =
                            parse_policy_for_kind(kind, &record.policy_json)?;
                        governance::eval::evaluate_security_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "quality" => {
                        let policy: governance::model::QualityPolicy =
                            parse_policy_for_kind(kind, &record.policy_json)?;
                        governance::eval::evaluate_quality_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "runtime" => {
                        let policy: governance::model::RuntimePolicy =
                            parse_policy_for_kind(kind, &record.policy_json)?;
                        governance::eval::evaluate_runtime_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "operator_routine" => {
                        let policy: governance::model::OperatorRoutinePolicy =
                            parse_policy_for_kind(kind, &record.policy_json)?;
                        let clean = std::process::Command::new("git")
                            .arg("-C")
                            .arg(path)
                            .arg("status")
                            .arg("--porcelain")
                            .output()
                            .ok()
                            .map(|o| o.status.success() && o.stdout.is_empty())
                            .unwrap_or(false);
                        let ctx = governance::eval::OperatorRoutineContext {
                            action: "push".to_string(),
                            has_active_scope: true,
                            repo_registered: true,
                            current_branch: Some(st.current_branch.clone()),
                            repo_clean: Some(clean),
                            completed_steps: vec![],
                        };
                        governance::eval::evaluate_operator_routine_policy(
                            &policy,
                            &ctx,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    _ => governance::model::PolicyEvalReport::default(),
                };

                violations += eval.violations.len();
                warnings += eval.warnings.len();
                if eval.blocked {
                    blocked += 1;
                }
                evaluations.push(serde_json::json!({
                    "repo": st.repo,
                    "path": st.path,
                    "branch": st.current_branch,
                    "blocked": eval.blocked,
                    "violations": eval.violations.len(),
                    "warnings": eval.warnings.len()
                }));
            }

            let report_json = serde_json::json!({
                "ok": true,
                "kind": kind,
                "version": version,
                "status": if blocked > 0 { "blocked" } else { "pass" },
                "evaluated_branches": evaluations.len(),
                "violations": violations,
                "warnings": warnings,
                "blocked": blocked,
                "evaluations": evaluations,
            });
            let reports_root = std::env::var("PILOT_REPORTS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(home).join(".pilot").join("reports")
                });
            std::fs::create_dir_all(&reports_root).into_diagnostic()?;
            let out_file = reports_root.join(format!(
                "policy_preview_{}_v{}_{}.json",
                kind,
                version,
                now_stamp()
            ));
            std::fs::write(
                &out_file,
                serde_json::to_string_pretty(&report_json).into_diagnostic()?,
            )
            .into_diagnostic()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report_json).into_diagnostic()?
            );
            println!("Artifact: {}", out_file.display());
            Ok(CommandReport::ok(
                "policy.preview",
                format!(
                    "Previewed {} v{} (artifact: {})",
                    kind,
                    version,
                    out_file.display()
                ),
            ))
        }
        PolicyCommands::Approve {
            kind,
            version,
            simulation_artifact,
        } => {
            if !simulation_artifact.exists() {
                return Err(miette::miette!(
                    "Simulation artifact not found at {}",
                    simulation_artifact.display()
                ));
            }
            let current = gov_store
                .get_policy_by_version(active.id, None, kind, *version as i32)
                .await?
                .ok_or_else(|| {
                    miette::miette!("No {} policy v{} found to approve", kind, version)
                })?;
            gov_store
                .update_policy_status(current.id, "approved", "pilot-cli")
                .await?
                .ok_or_else(|| miette::miette!("Failed to update approved status"))?;
            println!(
                "Approved {} v{} using artifact {}",
                kind,
                version,
                simulation_artifact.display()
            );
            let report = CommandReport::ok("policy.approve", format!("Approved v{}", version));
            persist_mutation_audit("policy.approve", false, &report.summary, vec![]);
            Ok(report)
        }
        PolicyCommands::Activate { kind, version } => {
            let current = gov_store
                .get_policy_by_version(active.id, None, kind, *version as i32)
                .await?
                .ok_or_else(|| {
                    miette::miette!("No {} policy v{} found to activate", kind, version)
                })?;
            gov_store
                .update_policy_status(current.id, "active", "pilot-cli")
                .await?
                .ok_or_else(|| miette::miette!("Failed to update active status"))?;
            println!("Activated {} v{}", kind, version);
            let report = CommandReport::ok("policy.activate", format!("Activated v{}", version));
            persist_mutation_audit("policy.activate", false, &report.summary, vec![]);
            Ok(report)
        }
        PolicyCommands::Delete {
            kind,
            ago_path,
            version,
        } => {
            let deleted = gov_store
                .delete_policy_version(active.id, ago_path.as_deref(), kind, *version as i32)
                .await?;
            if deleted == 0 {
                return Err(miette::miette!(
                    "No {} policy version {} found for ago_path={:?}",
                    kind,
                    version,
                    ago_path
                ));
            }
            println!(
                "Deleted {} policy version {} (ago_path={:?})",
                kind, version, ago_path
            );
            let report =
                CommandReport::ok("policy.delete", format!("Deleted {} v{}", kind, version));
            persist_mutation_audit("policy.delete", false, &report.summary, vec![]);
            Ok(report)
        }
        PolicyCommands::Resolve { kind, repo_path } => {
            let canonical = canonicalize_path_lossy(repo_path);
            let override_record = gov_store
                .get_ago_policy_override(active.id, &canonical, kind)
                .await?;
            let (source, policy_json) = if let Some(r) = override_record {
                ("ago", r.policy_json)
            } else {
                match gov_store.get_policy(active.id, kind).await? {
                    Some(r) => ("agorg", r.policy_json),
                    None => {
                        let fallback_val = match kind.as_str() {
                            "branch" => {
                                serde_json::to_value(governance::model::BranchPolicy::default())
                            }
                            "dependency" => {
                                serde_json::to_value(governance::model::DependencyPolicy::default())
                            }
                            "release" => {
                                serde_json::to_value(governance::model::ReleasePolicy::default())
                            }
                            "security" => {
                                serde_json::to_value(governance::model::SecurityPolicy::default())
                            }
                            "quality" => {
                                serde_json::to_value(governance::model::QualityPolicy::default())
                            }
                            "runtime" => {
                                serde_json::to_value(governance::model::RuntimePolicy::default())
                            }
                            "operator_routine" => serde_json::to_value(
                                governance::model::OperatorRoutinePolicy::default(),
                            ),
                            _ => serde_json::to_value(governance::model::BranchPolicy::default()),
                        }
                        .unwrap_or(serde_json::json!({}));
                        ("fallback_default", fallback_val)
                    }
                }
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "kind": kind,
                    "repo_path": canonical,
                    "source": source,
                    "policy": policy_json
                }))
                .into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "policy.resolve",
                "Resolved policy".to_string(),
            ))
        }
        PolicyCommands::Scan { kind, group, tags } => {
            if kind != "branch"
                && kind != "dependency"
                && kind != "release"
                && kind != "security"
                && kind != "quality"
                && kind != "runtime"
                && kind != "operator_routine"
            {
                return Err(miette::miette!("Scan currently supports branch, dependency, release, security, quality, runtime, operator_routine policies"));
            }
            let record = gov_store
                .get_policy(active.id, kind)
                .await?
                .map(|r| r.policy_json)
                .unwrap_or_else(|| match kind.as_str() {
                    "branch" => {
                        serde_json::to_value(governance::model::BranchPolicy::default()).unwrap()
                    }
                    "dependency" => {
                        serde_json::to_value(governance::model::DependencyPolicy::default())
                            .unwrap()
                    }
                    "release" => {
                        serde_json::to_value(governance::model::ReleasePolicy::default()).unwrap()
                    }
                    "security" => {
                        serde_json::to_value(governance::model::SecurityPolicy::default()).unwrap()
                    }
                    "quality" => {
                        serde_json::to_value(governance::model::QualityPolicy::default()).unwrap()
                    }
                    "runtime" => {
                        serde_json::to_value(governance::model::RuntimePolicy::default()).unwrap()
                    }
                    "operator_routine" => {
                        serde_json::to_value(governance::model::OperatorRoutinePolicy::default())
                            .unwrap()
                    }
                    _ => serde_json::json!({}),
                });
            let filter = multi::RepoFilter {
                group: group.clone(),
                tags: tags.clone(),
            };
            let registry = multi::MultiRegistry::open(&multi::MultiRegistry::default_db_path())
                .map_err(|e| miette::miette!("Failed to open multi registry: {}", e))?;
            let mut repos = registry
                .list_repos(&filter)
                .map_err(|e| miette::miette!("Failed listing repos for scan: {}", e))?;
            let roots = vec![std::path::PathBuf::from(&active.root_path)];
            repos.retain(|repo| {
                let path = canonicalize_path_lossy(&repo.path);
                path_in_any_root(&path, &roots)
            });
            let exceptions = gov_store.get_effective_exceptions(active.id, kind).await?;
            let statuses = branch::branch_status(&repos);
            let mut issues = 0usize;
            let mut off_policy = 0usize;
            let mut details: Vec<serde_json::Value> = Vec::with_capacity(statuses.len());
            for st in &statuses {
                let path = std::path::Path::new(&st.path);
                let ago_path = path
                    .strip_prefix(std::path::Path::new(&active.root_path))
                    .unwrap_or(path)
                    .display()
                    .to_string();
                let source_name = "Scan";
                let source_id = Some(active.id);

                let eval = match kind.as_str() {
                    "branch" => {
                        let policy: governance::model::BranchPolicy =
                            parse_policy_for_kind(kind, &record)?;
                        governance::eval::evaluate_branch_policy(
                            &policy,
                            "create",
                            &st.current_branch,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "dependency" => {
                        let policy: governance::model::DependencyPolicy =
                            parse_policy_for_kind(kind, &record)?;
                        governance::eval::evaluate_dependency_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "release" => {
                        let policy: governance::model::ReleasePolicy =
                            parse_policy_for_kind(kind, &record)?;
                        governance::eval::evaluate_release_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "security" => {
                        let policy: governance::model::SecurityPolicy =
                            parse_policy_for_kind(kind, &record)?;
                        governance::eval::evaluate_security_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "quality" => {
                        let policy: governance::model::QualityPolicy =
                            parse_policy_for_kind(kind, &record)?;
                        governance::eval::evaluate_quality_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "runtime" => {
                        let policy: governance::model::RuntimePolicy =
                            parse_policy_for_kind(kind, &record)?;
                        governance::eval::evaluate_runtime_policy(
                            &policy,
                            path,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    "operator_routine" => {
                        let policy: governance::model::OperatorRoutinePolicy =
                            parse_policy_for_kind(kind, &record)?;
                        let clean = std::process::Command::new("git")
                            .arg("-C")
                            .arg(path)
                            .arg("status")
                            .arg("--porcelain")
                            .output()
                            .ok()
                            .map(|o| o.status.success() && o.stdout.is_empty())
                            .unwrap_or(false);
                        let ctx = governance::eval::OperatorRoutineContext {
                            action: "push".to_string(),
                            has_active_scope: true,
                            repo_registered: true,
                            current_branch: Some(st.current_branch.clone()),
                            repo_clean: Some(clean),
                            completed_steps: vec![],
                        };
                        governance::eval::evaluate_operator_routine_policy(
                            &policy,
                            &ctx,
                            &exceptions,
                            &ago_path,
                            source_name,
                            source_id,
                        )
                    }
                    _ => governance::model::PolicyEvalReport::default(),
                };

                let issue_count = eval.violations.len() + eval.warnings.len();
                issues += issue_count;
                if issue_count > 0 {
                    off_policy += 1;
                }
                details.push(serde_json::json!({
                    "repo": st.repo,
                    "path": st.path,
                    "branch": st.current_branch,
                    "blocked": eval.blocked,
                    "violations": eval.violations.len(),
                    "warnings": eval.warnings.len()
                }));
            }
            let summary = serde_json::json!({
                "kind": kind,
                "scanned": details.len(),
                "issues": issues,
                "off_policy": off_policy,
                "details": details
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "policy.scan",
                "Scan completed".to_string(),
            ))
        }
        PolicyCommands::Decisions { kind, limit } => {
            let decisions = gov_store.get_decisions(active.id, kind, *limit).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&decisions).into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "policy.decisions",
                format!("Returned {} decisions", decisions.len()),
            ))
        }
        PolicyCommands::Exceptions(ex) => match &ex.command {
            PolicyExceptionsCommands::List { kind } => {
                let list = gov_store.get_exceptions(active.id, kind).await?;
                for e in list {
                    println!(
                        "{} | {} | {} | expr: {}",
                        e.id, e.rule_path, e.owner, e.expires_at
                    );
                }
                Ok(CommandReport::ok(
                    "policy.exceptions.list",
                    "Listed exceptions".to_string(),
                ))
            }
            PolicyExceptionsCommands::Add(add) => {
                let ago_path_val = add.ago_path.clone().unwrap_or_else(|| "".to_string());
                let e = governance::model::PolicyException {
                    id: uuid::Uuid::new_v4(),
                    agorg_id: active.id,
                    ago_path: if ago_path_val.is_empty() {
                        None
                    } else {
                        Some(ago_path_val)
                    },
                    policy_kind: add.kind.clone(),
                    rule_path: add.rule_path.clone(),
                    reason: add.reason.clone(),
                    ticket_ref: Some(add.ticket.clone()),
                    owner: add.owner.clone(),
                    expires_at: chrono::DateTime::from_timestamp(add.expires_at, 0)
                        .unwrap_or_else(chrono::Utc::now),
                    created_at: chrono::Utc::now(),
                };
                gov_store.add_exception(e).await?;
                println!("Added exception for {}/{}", add.kind, add.rule_path);
                let report =
                    CommandReport::ok("policy.exceptions.add", "Added exception".to_string());
                persist_mutation_audit("policy.exceptions.add", false, &report.summary, vec![]);
                Ok(report)
            }
            PolicyExceptionsCommands::Delete { id } => {
                let u = uuid::Uuid::parse_str(id).into_diagnostic()?;
                gov_store.delete_exception(u).await?;
                println!("Deleted exception {}", id);
                let report =
                    CommandReport::ok("policy.exceptions.delete", "Deleted exception".to_string());
                persist_mutation_audit("policy.exceptions.delete", false, &report.summary, vec![]);
                Ok(report)
            }
        },
    }
}
