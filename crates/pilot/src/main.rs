#![allow(dead_code)]
mod agorg;
mod bus;
mod config;
mod db_runtime;
mod serve_ui;
use agorg::AgorgStore;
use db_runtime::PilotDbManager;
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
    /// Run Pilot as an ArqonBus command bridge
    Serve(ServeArgs),
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
}

#[derive(Args, Clone)]
struct AgorgCreateArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    root: PathBuf,
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
    parent: Option<String>,
    #[arg(long, default_value = "4")]
    scan_depth: usize,
    #[arg(long)]
    autoscan: bool,
    #[arg(long)]
    import: bool,
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
                    let entry = registry
                        .register_repo(
                            &args.path,
                            args.name.as_deref(),
                            args.group.as_deref(),
                            &args.tags,
                        )
                        .map_err(|e| miette::miette!("Register failed: {e}"))?;

                    println!(
                        "Registered: {} ({}) group={:?} tags={:?}",
                        entry.name,
                        entry.path.display(),
                        entry.group_name,
                        entry.tags
                    );

                    let report = CommandReport::ok(
                        "multi.register",
                        format!("Registered {}", entry.path.display()),
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
        Commands::Agorg(args) => run_agorg(args).await,
        Commands::Db(args) => run_db(args).await,
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
                    bus: cfg.clone(),
                    allow_mutations: args.ui_allow_mutations,
                    allowed_commands,
                };
                let bridge = tokio::spawn(async move { bus::run_bridge(&bridge_cfg).await });
                let ui = tokio::spawn(async move { serve_ui::run_ui_server(ui_cfg).await });
                let (bridge_res, ui_res) = tokio::try_join!(bridge, ui)
                    .map_err(|e| miette::miette!("Serve tasks failed: {e}"))?;
                bridge_res?;
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

fn persist_mutation_audit(command: &str, dry_run: bool, summary: &str, outcomes: Vec<RepoOutcome>) {
    let repo_count = outcomes.len();
    let failures = outcomes.iter().filter(|o| !o.success).count();
    let artifact_path = write_repo_outcomes_artifact(command, &outcomes)
        .map(|p| p.display().to_string())
        .ok();

    let event = AuditEvent {
        timestamp: String::new(),
        command: command.to_string(),
        dry_run,
        success: failures == 0,
        summary: summary.to_string(),
        repo_count,
        failures,
        artifact_path: artifact_path.clone(),
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
        Commands::Serve(_) => "serve",
    }
}

async fn run_db(args: &DbArgs) -> Result<CommandReport> {
    let manager = PilotDbManager::from_env();
    match args.command {
        DbCommands::Ensure => {
            manager.ensure_ready().await?;
            let status = manager.status().await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&status).into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "db.ensure",
                "Managed DB ensured and ready".to_string(),
            ))
        }
        DbCommands::Start => {
            manager.start().await?;
            let status = manager.status().await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&status).into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "db.start",
                "Managed DB started".to_string(),
            ))
        }
        DbCommands::Stop => {
            manager.stop().await?;
            let status = manager.status().await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&status).into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "db.stop",
                "Managed DB stopped".to_string(),
            ))
        }
        DbCommands::Status => {
            let status = manager.status().await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&status).into_diagnostic()?
            );
            Ok(CommandReport::ok(
                "db.status",
                "Managed DB status".to_string(),
            ))
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
                    store.import_discovery(ag.id, scan).await?;
                    println!("Imported {} discovered candidates", scan.candidates.len());
                } else {
                    let scan = agorg::discover_hierarchy(&args.root, args.scan_depth)?;
                    store.import_discovery(ag.id, &scan).await?;
                    println!("Imported {} discovered candidates", scan.candidates.len());
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
                store.import_discovery(id, &scan).await?;
                println!(
                    "Imported discovery into {} ({} candidates)",
                    id,
                    scan.candidates.len()
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
