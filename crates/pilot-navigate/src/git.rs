use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

/// Repository information extracted from git remote
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub owner: String,
    pub repo: String,
}

/// Parse owner/repo from git remote URL
///
/// Supports:
/// - HTTPS: `https://github.com/owner/repo.git`
/// - SSH: `git@github.com:owner/repo.git`
pub fn parse_git_remote(root: &Path) -> Result<RepoInfo> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to get git remote URL. Is this a git repository?"
        ));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_remote_url(&url)
}

/// Parse a remote URL string into owner/repo
fn parse_remote_url(url: &str) -> Result<RepoInfo> {
    // Handle SSH format: git@github.com:owner/repo.git
    if url.starts_with("git@") {
        let parts: Vec<&str> = url.split(':').collect();
        if parts.len() >= 2 {
            return parse_path_segment(parts[1]);
        }
    }

    // Handle HTTPS format: https://github.com/owner/repo.git
    if url.starts_with("https://") || url.starts_with("http://") {
        // Extract path after host
        if let Some(path_start) = url.find("github.com/") {
            let path = &url[path_start + 11..]; // Skip "github.com/"
            return parse_path_segment(path);
        }
        if let Some(path_start) = url.find("gitlab.com/") {
            let path = &url[path_start + 11..]; // Skip "gitlab.com/"
            return parse_path_segment(path);
        }
    }

    Err(anyhow!("Could not parse git remote URL: {}", url))
}

/// Parse "owner/repo.git" or "owner/repo" into RepoInfo
fn parse_path_segment(path: &str) -> Result<RepoInfo> {
    let clean = path.trim_end_matches(".git").trim_end_matches('/');
    let parts: Vec<&str> = clean.split('/').collect();

    if parts.len() >= 2 {
        Ok(RepoInfo {
            owner: parts[0].to_string(),
            repo: parts[1].to_string(),
        })
    } else {
        Err(anyhow!("Invalid repository path: {}", path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_https_url() {
        let result = parse_remote_url("https://github.com/novelbytelabs/ArqonHPO.git").unwrap();
        assert_eq!(result.owner, "novelbytelabs");
        assert_eq!(result.repo, "ArqonHPO");
    }

    #[test]
    fn test_parse_https_url_no_git_suffix() {
        let result = parse_remote_url("https://github.com/owner/repo").unwrap();
        assert_eq!(result.owner, "owner");
        assert_eq!(result.repo, "repo");
    }

    #[test]
    fn test_parse_ssh_url() {
        let result = parse_remote_url("git@github.com:novelbytelabs/ArqonHPO.git").unwrap();
        assert_eq!(result.owner, "novelbytelabs");
        assert_eq!(result.repo, "ArqonHPO");
    }
}
