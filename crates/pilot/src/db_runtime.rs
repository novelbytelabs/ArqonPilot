use miette::{miette, Context, IntoDiagnostic, Result};
use serde::Serialize;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use tokio::process::Command;

const DEFAULT_DB_NAME: &str = "pilot_local";
const DEFAULT_PORT: u16 = 9132;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DbMode {
    Tcp,
    UnixSocket,
}

#[derive(Debug, Clone)]
pub struct PilotDbManager {
    pub root_dir: PathBuf,
    pub data_dir: PathBuf,
    pub run_dir: PathBuf,
    pub log_file: PathBuf,
    pub db_name: String,
    pub user: String,
    pub socket_dir: PathBuf,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct DbStatus {
    pub initialized: bool,
    pub running: bool,
    pub error_note: Option<String>,
    pub mode: String,
    pub endpoint: String,
    pub dsn: String,
    pub data_dir: String,
    pub log_file: String,
}

impl PilotDbManager {
    fn db_mode(&self) -> DbMode {
        match std::env::var("PILOT_DB_MODE")
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("unix") | Some("unix_socket") | Some("socket") => DbMode::UnixSocket,
            Some("tcp") => DbMode::Tcp,
            _ => DbMode::Tcp,
        }
    }

    pub fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let pilot_home = std::env::var("PILOT_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(home).join(".arqon").join("pilot"));
        let root_dir = pilot_home.join("db");
        let data_dir = root_dir.join("data");
        let run_dir = pilot_home.join("run");
        let log_file = std::env::var("PILOT_DB_LOG_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| run_dir.join("postgres.log"));
        let socket_dir = run_dir.clone();
        let db_name =
            std::env::var("PILOT_DB_NAME").unwrap_or_else(|_| DEFAULT_DB_NAME.to_string());
        let user = std::env::var("PILOT_DB_USER")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "postgres".to_string());
        let port = std::env::var("PILOT_DB_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);
        Self {
            root_dir,
            data_dir,
            run_dir,
            log_file,
            db_name,
            user,
            socket_dir,
            port,
        }
    }

    pub fn endpoint_mode(&self) -> &'static str {
        match self.db_mode() {
            DbMode::Tcp => "tcp",
            DbMode::UnixSocket => "unix_socket",
        }
    }

    pub fn target_dsn(&self) -> String {
        match self.db_mode() {
            DbMode::Tcp => {
                format!(
                    "host=127.0.0.1 port={} user={} dbname={}",
                    self.port, self.user, self.db_name
                )
            }
            DbMode::UnixSocket => {
                format!(
                    "host={} port={} user={} dbname={}",
                    self.socket_dir.display(),
                    self.port,
                    self.user,
                    self.db_name
                )
            }
        }
    }

    pub fn maintenance_dsn(&self) -> String {
        match self.db_mode() {
            DbMode::Tcp => {
                format!(
                    "host=127.0.0.1 port={} user={} dbname=postgres",
                    self.port, self.user
                )
            }
            DbMode::UnixSocket => {
                format!(
                    "host={} port={} user={} dbname=postgres",
                    self.socket_dir.display(),
                    self.port,
                    self.user
                )
            }
        }
    }

    pub async fn ensure_ready(&self) -> Result<()> {
        self.ensure_layout()?;
        self.ensure_initialized().await?;
        self.ensure_running().await?;
        Ok(())
    }

    pub async fn ensure_initialized(&self) -> Result<()> {
        self.ensure_layout()?;
        if self.data_dir.join("PG_VERSION").exists() {
            return Ok(());
        }
        self.run_initdb().await
    }

    pub async fn ensure_running(&self) -> Result<()> {
        self.ensure_layout()?;
        if self.is_running().await? {
            return Ok(());
        }
        self.start().await
    }

    pub async fn start(&self) -> Result<()> {
        self.ensure_initialized().await?;
        let pg_ctl = resolve_postgres_bin("pg_ctl")?;
        let log_file = self.select_log_file()?;
        let mut cmd = Command::new(pg_ctl);
        cmd.arg("-D")
            .arg(&self.data_dir)
            .arg("-l")
            .arg(&log_file)
            .arg("start")
            .arg("-w")
            .arg("-t")
            .arg("20");
        let opts = self.postgres_opts();
        if !opts.is_empty() {
            cmd.arg("-o").arg(opts);
        }
        run_checked(
            cmd,
            "Failed to start managed Postgres. Ensure `pg_ctl` exists and log path is writable.",
        )
        .await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        if !self.data_dir.join("PG_VERSION").exists() {
            return Ok(());
        }
        if !self.is_running().await? {
            return Ok(());
        }
        let pg_ctl = resolve_postgres_bin("pg_ctl")?;
        let mut cmd = Command::new(pg_ctl);
        cmd.arg("-D")
            .arg(&self.data_dir)
            .arg("stop")
            .arg("-m")
            .arg("fast")
            .arg("-w")
            .arg("-t")
            .arg("20");
        run_checked(cmd, "Failed to stop managed Postgres").await?;
        Ok(())
    }

    pub async fn status(&self) -> Result<DbStatus> {
        self.ensure_layout()?;
        let initialized = self.data_dir.join("PG_VERSION").exists();
        let log_file = self
            .select_log_file()
            .unwrap_or_else(|_| self.log_file.clone());
        let running_res = if initialized {
            self.is_running().await
        } else {
            Ok(false)
        };
        let running = running_res.as_ref().copied().unwrap_or(false);
        let error_note = running_res.err().map(|e| e.to_string());
        let endpoint = match self.db_mode() {
            DbMode::Tcp => format!("127.0.0.1:{}", self.port),
            DbMode::UnixSocket => format!("{}/.s.PGSQL.{}", self.socket_dir.display(), self.port),
        };
        Ok(DbStatus {
            initialized,
            running,
            error_note,
            mode: self.endpoint_mode().to_string(),
            endpoint,
            dsn: self.target_dsn(),
            data_dir: self.data_dir.display().to_string(),
            log_file: log_file.display().to_string(),
        })
    }

    fn ensure_layout(&self) -> Result<()> {
        fs::create_dir_all(&self.root_dir).into_diagnostic()?;
        fs::create_dir_all(&self.run_dir).into_diagnostic()?;
        if let Some(parent) = self.log_file.parent() {
            fs::create_dir_all(parent).into_diagnostic()?;
        }
        Ok(())
    }

    fn select_log_file(&self) -> Result<PathBuf> {
        // Deterministic log path to avoid stale/fallback ambiguity.
        // Default always stays in the managed run dir unless explicitly overridden.
        let selected = if std::env::var("PILOT_DB_LOG_FILE").is_ok() {
            self.log_file.clone()
        } else {
            self.run_dir.join("postgres.log")
        };
        Ok(selected)
    }

    async fn run_initdb(&self) -> Result<()> {
        let initdb = resolve_postgres_bin("initdb")?;
        let mut cmd = Command::new(initdb);
        cmd.arg("-D")
            .arg(&self.data_dir)
            .arg("-U")
            .arg(&self.user)
            .arg("-A")
            .arg("trust")
            .arg("--no-locale");
        run_checked(
            cmd,
            "Failed to initialize managed Postgres. Ensure `initdb` exists.",
        )
        .await?;
        Ok(())
    }

    async fn is_running(&self) -> Result<bool> {
        let pg_ctl = match resolve_postgres_bin("pg_ctl") {
            Ok(path) => path,
            Err(_) => return Ok(false),
        };
        let mut cmd = Command::new(pg_ctl);
        cmd.arg("-D").arg(&self.data_dir).arg("status");
        let output = cmd.output().await.into_diagnostic()?;
        Ok(output.status.success())
    }

    fn postgres_opts(&self) -> String {
        match self.db_mode() {
            DbMode::Tcp => {
                // Keep local-only TCP with deterministic high port.
                // Force socket directory away from /var/run/postgresql so non-root users can start cleanly.
                format!(
                    "-k {} -p {} -c listen_addresses=127.0.0.1",
                    self.socket_dir.display(),
                    self.resolve_port()
                )
            }
            DbMode::UnixSocket => {
                format!(
                    "-k {} -p {} -c listen_addresses='' -c unix_socket_permissions=0700",
                    self.socket_dir.display(),
                    self.port
                )
            }
        }
    }

    fn resolve_port(&self) -> u16 {
        if !cfg!(windows) {
            return self.port;
        }
        if is_port_available(self.port) {
            return self.port;
        }
        for port in self.port.saturating_add(1)..=self.port.saturating_add(32) {
            if is_port_available(port) {
                return port;
            }
        }
        self.port
    }
}

fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

async fn run_checked(mut cmd: Command, context: &str) -> Result<()> {
    let debug_cmd = format!(
        "{} {}",
        cmd.as_std().get_program().to_string_lossy(),
        cmd.as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let output = cmd
        .output()
        .await
        .into_diagnostic()
        .with_context(|| format!("{context} (failed to execute child process)"))?;

    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let tail_stderr = truncate_tail(stderr.trim(), 800);
    let tail_stdout = truncate_tail(stdout.trim(), 800);

    // Try to find if there's a log file we can dump
    let mut log_dump = String::new();
    if let Some(arg_idx) = cmd.as_std().get_args().position(|a| a == "-l") {
        if let Some(log_path) = cmd.as_std().get_args().nth(arg_idx + 1) {
            if let Ok(content) = std::fs::read_to_string(log_path) {
                log_dump = format!("\n--- PG LOG TAIL ---\n{}", truncate_tail(&content, 2000));
            }
        }
    }

    Err(miette!(
        "{}\ncommand: {}\nstdout: {}\nstderr: {}{}",
        context,
        debug_cmd,
        tail_stdout,
        tail_stderr,
        log_dump
    ))
}

fn truncate_tail(input: &str, max: usize) -> String {
    if input.len() <= max {
        return input.to_string();
    }
    let start = input.len() - max;
    format!("...{}", &input[start..])
}

fn resolve_postgres_bin(bin: &str) -> Result<String> {
    let env_key = format!("PILOT_{}_BIN", bin.to_ascii_uppercase());
    if let Ok(path) = std::env::var(&env_key) {
        if !path.trim().is_empty() && PathBuf::from(&path).exists() {
            return Ok(path);
        }
    }
    if let Some(path_hit) = resolve_from_path(bin) {
        return Ok(path_hit);
    }
    if let Some(path_hit) = resolve_from_ubuntu_layout(bin) {
        return Ok(path_hit);
    }
    Err(miette!(
        "Postgres binary '{}' not found. Install local server tools (Ubuntu: `sudo apt-get install postgresql`) or set {}=/absolute/path/{}",
        bin,
        env_key,
        bin
    ))
}

fn resolve_from_path(bin: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.exists() {
            return Some(candidate.display().to_string());
        }
    }
    None
}

fn resolve_from_ubuntu_layout(bin: &str) -> Option<String> {
    let root = PathBuf::from("/usr/lib/postgresql");
    let entries = fs::read_dir(root).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("bin").join(bin))
        .filter(|p| p.exists())
        .collect();
    candidates.sort();
    candidates.pop().map(|p| p.display().to_string())
}
