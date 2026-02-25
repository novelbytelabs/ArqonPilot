use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub commit_type: String,
    pub scope: Option<String>,
    pub description: String,
    pub is_breaking: bool,
}

pub struct CommitParser {
    root: PathBuf,
}

impl CommitParser {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Get commits since last tag
    pub fn get_commits_since_last_tag(&self) -> Result<Vec<Commit>> {
        // Get last tag
        let tag_output = Command::new("git")
            .args(["describe", "--tags", "--abbrev=0"])
            .current_dir(&self.root)
            .output()?;

        let last_tag = if tag_output.status.success() {
            String::from_utf8_lossy(&tag_output.stdout)
                .trim()
                .to_string()
        } else {
            "".to_string()
        };

        // Get log since tag (or all if no tag)
        let range = if last_tag.is_empty() {
            "HEAD".to_string()
        } else {
            format!("{}..HEAD", last_tag)
        };

        let log_output = Command::new("git")
            .args(["log", "--format=%H %s", &range])
            .current_dir(&self.root)
            .output()?;

        let log_str = String::from_utf8_lossy(&log_output.stdout);
        let commits = log_str
            .lines()
            .filter_map(|line| self.parse_commit_line(line))
            .collect();

        Ok(commits)
    }

    fn parse_commit_line(&self, line: &str) -> Option<Commit> {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return None;
        }

        let hash = parts[0].to_string();
        let message = parts[1];

        // Parse conventional commit: type(scope)!: description
        let is_breaking = message.contains("BREAKING CHANGE") || message.contains("!:");

        // Extract type and scope using pattern matching
        // Format: type(scope): description OR type: description
        let (commit_type, scope, description) = if let Some(colon_pos) = message.find(':') {
            let prefix = &message[..colon_pos];
            let desc = message[colon_pos + 1..].trim().to_string();

            // Check for scope: type(scope) or type(scope)!
            if let Some(paren_start) = prefix.find('(') {
                if let Some(paren_end) = prefix.find(')') {
                    let ctype = prefix[..paren_start].to_string();
                    let scope = prefix[paren_start + 1..paren_end].to_string();
                    (ctype, Some(scope), desc)
                } else {
                    (prefix.trim_end_matches('!').to_string(), None, desc)
                }
            } else {
                (prefix.trim_end_matches('!').to_string(), None, desc)
            }
        } else {
            // No colon, treat whole message as description with "other" type
            ("other".to_string(), None, message.to_string())
        };

        Some(Commit {
            hash,
            commit_type,
            scope,
            description,
            is_breaking,
        })
    }
}
