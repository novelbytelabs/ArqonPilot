use crate::commits::Commit;
use miette::{IntoDiagnostic, Result, WrapErr};
use std::fmt;
use std::path::Path;
use toml_edit::{value, DocumentMut};

#[derive(Debug, Clone)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl SemVer {
    pub fn parse(version_str: &str) -> Result<Self> {
        let clean = version_str.trim_start_matches('v');
        let parts: Vec<&str> = clean.split('.').collect();

        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    pub fn bump_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
        }
    }

    pub fn bump_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    pub fn bump_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
        }
    }

    /// Read version from a Cargo.toml file
    ///
    /// Handles both regular packages and workspace roots:
    /// - Checks `package.version` first (regular package)
    /// - Falls back to `workspace.package.version` (workspace with shared version)
    pub fn from_cargo_toml(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to read Cargo.toml: {:?}", path))?;

        let parsed: toml::Value = toml::from_str(&content)
            .into_diagnostic()
            .wrap_err("Failed to parse Cargo.toml")?;

        // Try package.version first (regular package)
        if let Some(version) = parsed
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Self::parse(version);
        }

        // Try workspace.package.version (workspace with shared version)
        if let Some(version) = parsed
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Self::parse(version);
        }

        Err(miette::miette!(
            "No version found in Cargo.toml or workspace"
        ))
    }

    /// Write version to a Cargo.toml file
    pub fn write_to_cargo_toml(&self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to read Cargo.toml at {:?}", path))?;

        let mut doc = content
            .parse::<DocumentMut>()
            .into_diagnostic()
            .wrap_err("Failed to parse Cargo.toml")?;

        if let Some(package) = doc.get_mut("package") {
            if let Some(item) = package.get_mut("version") {
                *item = value(self.to_string());
            }
        } else if let Some(workspace) = doc.get_mut("workspace") {
            if let Some(package) = workspace.get_mut("package") {
                if let Some(item) = package.get_mut("version") {
                    *item = value(self.to_string());
                }
            }
        }

        std::fs::write(path, doc.to_string())
            .into_diagnostic()
            .wrap_err("Failed to write Cargo.toml")?;

        Ok(())
    }
}

/// Calculate next version based on conventional commits
pub fn calculate_next_version(current: &SemVer, commits: &[Commit]) -> SemVer {
    let has_breaking = commits.iter().any(|c| c.is_breaking);
    let has_feat = commits.iter().any(|c| c.commit_type == "feat");

    if has_breaking {
        current.bump_major()
    } else if has_feat {
        current.bump_minor()
    } else {
        current.bump_patch()
    }
}

/// Generate changelog from commits
pub fn generate_changelog(version: &SemVer, commits: &[Commit]) -> String {
    let mut changelog = format!("## v{}\n\n", version);

    // Group by type
    let features: Vec<_> = commits.iter().filter(|c| c.commit_type == "feat").collect();
    let fixes: Vec<_> = commits.iter().filter(|c| c.commit_type == "fix").collect();
    let others: Vec<_> = commits
        .iter()
        .filter(|c| c.commit_type != "feat" && c.commit_type != "fix")
        .collect();

    if !features.is_empty() {
        changelog.push_str("### Features\n\n");
        for commit in features {
            changelog.push_str(&format!("- {}\n", commit.description));
        }
        changelog.push('\n');
    }

    if !fixes.is_empty() {
        changelog.push_str("### Bug Fixes\n\n");
        for commit in fixes {
            changelog.push_str(&format!("- {}\n", commit.description));
        }
        changelog.push('\n');
    }

    if !others.is_empty() {
        changelog.push_str("### Other Changes\n\n");
        for commit in others {
            changelog.push_str(&format!("- {}\n", commit.description));
        }
        changelog.push('\n');
    }

    changelog
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_write_to_cargo_toml() -> Result<()> {
        let dir = tempfile::tempdir().into_diagnostic()?;
        let path = dir.path().join("Cargo.toml");

        // 1. Setup basic Cargo.toml
        fs::write(
            &path,
            r#"
[package]
name = "test"
version = "0.1.0"
"#,
        )
        .into_diagnostic()?;

        // 2. Write update
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
        };
        v.write_to_cargo_toml(&path)?;

        let content = fs::read_to_string(&path).into_diagnostic()?;
        assert!(content.contains("version = \"1.2.3\""));

        // 3. Test workspace format
        fs::write(
            &path,
            r#"
[workspace.package]
version = "0.0.0"
"#,
        )
        .into_diagnostic()?;

        v.write_to_cargo_toml(&path)?;
        let content = fs::read_to_string(&path).into_diagnostic()?;
        assert!(content.contains("version = \"1.2.3\""));

        Ok(())
    }
}
