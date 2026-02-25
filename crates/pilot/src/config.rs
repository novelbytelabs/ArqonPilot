use miette::{Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub meta: MetaConfig,
    pub oracle: OracleConfig,
    pub heal: HealConfig,
    pub navigate: NavigateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConfig {
    pub config_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleConfig {
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
    pub model_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealConfig {
    pub max_attempts: u32,
    pub model_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigateConfig {
    pub require_branches: Vec<String>,
    pub version_scheme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            meta: MetaConfig { config_version: 1 },
            oracle: OracleConfig {
                include_globs: vec!["src/**/*.rs".to_string(), "src/**/*.py".to_string()],
                exclude_globs: vec![
                    "target/".to_string(),
                    "venv/".to_string(),
                    ".git/".to_string(),
                ],
                model_path: "~/.pilot/models/".to_string(),
            },
            heal: HealConfig {
                max_attempts: 2,
                model_id: "deepseek-coder-1.3b-instruct".to_string(),
                enabled: true,
            },
            navigate: NavigateConfig {
                require_branches: vec!["main".to_string()],
                version_scheme: "semver".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            miette::bail!("Config file not found at {:?}", path);
        }
        let content = fs::read_to_string(path)
            .into_diagnostic()
            .with_context(|| format!("Failed to read config file at {:?}", path))?;

        let config: Config = toml::from_str(&content)
            .into_diagnostic()
            .context("Failed to parse config TOML")?;

        Ok(config)
    }

    pub fn load_default() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();
        assert_eq!(config.meta.config_version, 1);
        assert_eq!(config.heal.max_attempts, 2);
        assert!(config.heal.enabled);
        assert_eq!(config.navigate.version_scheme, "semver");
    }

    #[test]
    fn test_oracle_config_default_globs() {
        let config = Config::default();
        assert!(config
            .oracle
            .include_globs
            .contains(&"src/**/*.rs".to_string()));
        assert!(config
            .oracle
            .include_globs
            .contains(&"src/**/*.py".to_string()));
        assert!(config.oracle.exclude_globs.contains(&"target/".to_string()));
    }

    #[test]
    fn test_heal_config_default() {
        let config = Config::default();
        assert_eq!(config.heal.model_id, "deepseek-coder-1.3b-instruct");
        assert_eq!(config.heal.max_attempts, 2);
    }

    #[test]
    fn test_navigate_config_default() {
        let config = Config::default();
        assert!(config
            .navigate
            .require_branches
            .contains(&"main".to_string()));
    }

    #[test]
    fn test_load_default_returns_default() {
        let config = Config::load_default();
        assert_eq!(config.meta.config_version, 1);
    }

    #[test]
    fn test_load_from_file_not_found() {
        let result = Config::load_from_file("/nonexistent/path/config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_file_valid_toml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[meta]
config_version = 2

[oracle]
include_globs = ["*.rs"]
exclude_globs = ["target/"]
model_path = "/tmp/models"

[heal]
max_attempts = 5
model_id = "test-model"
enabled = false

[navigate]
require_branches = ["main", "develop"]
version_scheme = "calver"
"#
        )
        .unwrap();

        let config = Config::load_from_file(file.path()).unwrap();
        assert_eq!(config.meta.config_version, 2);
        assert_eq!(config.heal.max_attempts, 5);
        assert!(!config.heal.enabled);
        assert_eq!(config.navigate.version_scheme, "calver");
    }

    #[test]
    fn test_load_from_file_invalid_toml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "this is not valid toml {{{{").unwrap();

        let result = Config::load_from_file(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.meta.config_version, config.meta.config_version);
        assert_eq!(parsed.heal.max_attempts, config.heal.max_attempts);
    }
}
