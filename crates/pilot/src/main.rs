#![allow(dead_code)]
mod config;
use pilot_branch as branch;
use pilot_core::{CommandReport, RepoContext};
use pilot_heal as heal;
use pilot_multi as multi;
use pilot_navigate as navigate;
use pilot_oracle as oracle;

use clap::{Args, Parser, Subcommand};
use config::Config;
use miette::{Context, IntoDiagnostic, Result};
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
    /// Cross-repo branch lifecycle operations
    Branch(BranchArgs),
    /// Multi-repo registry and operations
    Multi(MultiArgs),
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
            Ok(CommandReport::ok("init", "Initialized .pilot/config.toml"))
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

            let mut healing_loop = HealingLoop::new(store, ctx.root, args.max_attempts)
                .map_err(|e| miette::miette!("{:?}", e))?;
            let outcome = healing_loop
                .run(&failure)
                .map_err(|e| miette::miette!("{:?}", e))?;

            println!("Heal outcome: {:?}", outcome);
            Ok(CommandReport::ok("heal", format!("Outcome: {:?}", outcome)))
        }
        Commands::Navigate(args) => {
            if args.multi {
                run_navigate_multi(args)
            } else {
                run_navigate_single(args)
            }
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
                    Ok(CommandReport::ok(
                        "branch.create",
                        format!("Processed {} repos ({} failed)", outcomes.len(), failures),
                    ))
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
                    Ok(CommandReport::ok(
                        "branch.sync",
                        format!("Processed {} repos ({} failed)", outcomes.len(), failures),
                    ))
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
                    Ok(CommandReport::ok(
                        "branch.prune",
                        format!("Processed {} repos ({} failed)", outcomes.len(), failures),
                    ))
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

                    Ok(CommandReport::ok(
                        "multi.register",
                        format!("Registered {}", entry.path.display()),
                    ))
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
                            return Ok(CommandReport::ok(
                                "multi.deps.set",
                                "Dry-run dependency update planned",
                            ));
                        }

                        registry
                            .set_dependencies(&args.repo, &args.depends_on)
                            .map_err(|e| miette::miette!("Set deps failed: {e}"))?;
                        println!(
                            "Updated dependencies for '{}' => [{}]",
                            args.repo,
                            args.depends_on.join(", ")
                        );
                        Ok(CommandReport::ok(
                            "multi.deps.set",
                            format!("Updated dependencies for {}", args.repo),
                        ))
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
                            return Ok(CommandReport::ok(
                                "multi.prs.create",
                                format!("Dry-run planned {} repos for linked PRs", ordered.len()),
                            ));
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
                        Ok(CommandReport::ok(
                            "multi.prs.create",
                            format!("Generated linked PR manifest at {}", manifest.display()),
                        ))
                    }
                },
            }
        }
    }
}

fn to_filter(group: Option<String>, tags: Vec<String>) -> multi::RepoFilter {
    multi::RepoFilter { group, tags }
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
            command:
                MultiCommands::Prs(MultiPrsArgs {
                    command: MultiPrsCommands::Create(_),
                }),
        }) => "multi.prs.create",
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
