use miette::{miette, Context, IntoDiagnostic, Result};
use serde::Serialize;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use tokio::process::Command;

const DEFAULT_DB_NAME: &str = "pilot_local";
const DEFAULT_PORT: u16 = 9132;

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
    pub mode: String,
    pub endpoint: String,
    pub dsn: String,
    pub data_dir: String,
    pub log_file: String,
}

impl PilotDbManager {
    pub fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let pilot_home = std::env::var("PILOT_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(home).join(".arqon").join("pilot"));
        let root_dir = pilot_home.join("db");
        let data_dir = root_dir.join("data");
        let run_dir = pilot_home.join("run");
        let log_file = root_dir.join("postgres.log");
        let socket_dir = run_dir.join("postgres.sock");
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
        if cfg!(windows) {
            "tcp"
        } else {
            "unix_socket"
        }
    }

    pub fn target_dsn(&self) -> String {
        if cfg!(windows) {
            format!(
                "host=127.0.0.1 port={} user={} dbname={}",
                self.port, self.user, self.db_name
            )
        } else {
            format!(
                "host={} user={} dbname={}",
                self.socket_dir.display(),
                self.user,
                self.db_name
            )
        }
    }

    pub fn maintenance_dsn(&self) -> String {
        if cfg!(windows) {
            format!(
                "host=127.0.0.1 port={} user={} dbname=postgres",
                self.port, self.user
            )
        } else {
            format!(
                "host={} user={} dbname=postgres",
                self.socket_dir.display(),
                self.user
            )
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
        let mut cmd = Command::new("pg_ctl");
        cmd.arg("-D")
            .arg(&self.data_dir)
            .arg("-l")
            .arg(&self.log_file)
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
            "Failed to start managed Postgres. Ensure `pg_ctl` exists.",
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
        let mut cmd = Command::new("pg_ctl");
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
        let running = if initialized {
            self.is_running().await.unwrap_or(false)
        } else {
            false
        };
        let endpoint = if cfg!(windows) {
            format!("127.0.0.1:{}", self.port)
        } else {
            self.socket_dir.display().to_string()
        };
        Ok(DbStatus {
            initialized,
            running,
            mode: self.endpoint_mode().to_string(),
            endpoint,
            dsn: self.target_dsn(),
            data_dir: self.data_dir.display().to_string(),
            log_file: self.log_file.display().to_string(),
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

    async fn run_initdb(&self) -> Result<()> {
        let mut cmd = Command::new("initdb");
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
        let mut cmd = Command::new("pg_ctl");
        cmd.arg("-D").arg(&self.data_dir).arg("status");
        let output = cmd.output().await.into_diagnostic()?;
        Ok(output.status.success())
    }

    fn postgres_opts(&self) -> String {
        if cfg!(windows) {
            // Keep local-only TCP on Windows, using deterministic high port.
            format!("-p {} -c listen_addresses=127.0.0.1", self.resolve_port())
        } else {
            format!(
                "-k {} -p {} -c listen_addresses='' -c unix_socket_permissions=0700",
                self.socket_dir.display(),
                self.port
            )
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
    Err(miette!(
        "{}\nstdout: {}\nstderr: {}",
        context,
        tail_stdout,
        tail_stderr
    ))
}

fn truncate_tail(input: &str, max: usize) -> String {
    if input.len() <= max {
        return input.to_string();
    }
    let start = input.len() - max;
    format!("...{}", &input[start..])
}
