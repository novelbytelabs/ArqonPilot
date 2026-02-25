use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanIssue {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub state: String,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredIssue {
    pub issue: PlanIssue,
    pub impact: u32,
    pub effort: u32,
    pub risk: u32,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roadmap {
    pub generated_at: String,
    pub items: Vec<ScoredIssue>,
}

#[derive(Debug, Deserialize)]
struct GitHubIssue {
    number: u64,
    title: String,
    #[serde(default)]
    body: Option<String>,
    state: String,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    labels: Vec<GitHubLabel>,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GitHubLabel {
    name: String,
}

pub fn load_issues_from_file(path: &Path) -> Result<Vec<PlanIssue>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed reading issues file {}", path.display()))?;
    let issues: Vec<PlanIssue> = serde_json::from_str(&raw)
        .with_context(|| format!("Failed parsing issues file {}", path.display()))?;
    Ok(issues)
}

pub fn fetch_issues_from_github(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<PlanIssue>> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues?state=open&per_page=100");
    let client = reqwest::blocking::Client::new();
    let mut req = client
        .get(&url)
        .header("User-Agent", "pilot-plan")
        .header("Accept", "application/vnd.github+json");
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let response = req.send()?.error_for_status()?;
    let data: Vec<GitHubIssue> = response.json()?;
    let issues = data
        .into_iter()
        .filter(|i| i.pull_request.is_none())
        .map(|i| PlanIssue {
            id: i.number,
            title: i.title,
            body: i.body.unwrap_or_default(),
            labels: i.labels.into_iter().map(|l| l.name).collect(),
            state: i.state,
            html_url: i.html_url,
        })
        .collect();
    Ok(issues)
}

pub fn write_issues(path: &Path, issues: &[PlanIssue]) -> Result<()> {
    write_json(path, issues)
}

pub fn load_scored(path: &Path) -> Result<Vec<ScoredIssue>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed reading scored file {}", path.display()))?;
    let items: Vec<ScoredIssue> = serde_json::from_str(&raw)
        .with_context(|| format!("Failed parsing scored file {}", path.display()))?;
    Ok(items)
}

pub fn score_issues(issues: Vec<PlanIssue>) -> Vec<ScoredIssue> {
    let mut out: Vec<ScoredIssue> = issues
        .into_iter()
        .filter(|i| i.state == "open")
        .map(|issue| {
            let labels_lower: Vec<String> = issue.labels.iter().map(|s| s.to_lowercase()).collect();
            let text = format!(
                "{} {}",
                issue.title.to_lowercase(),
                issue.body.to_lowercase()
            );

            let impact = if has_any(&labels_lower, &["critical", "security", "customer-impact"])
                || contains_any(&text, &["security", "outage", "data loss", "broken"])
            {
                5
            } else if has_any(&labels_lower, &["high", "p1"]) {
                4
            } else if has_any(&labels_lower, &["medium", "p2"]) {
                3
            } else {
                2
            };

            let effort = if has_any(&labels_lower, &["size/xl", "size/l", "epic"])
                || contains_any(&text, &["refactor", "migration", "cross-repo"])
            {
                5
            } else if has_any(&labels_lower, &["size/m"]) {
                3
            } else {
                2
            };

            let risk = if has_any(&labels_lower, &["security", "breaking-change", "risk/high"]) {
                5
            } else if contains_any(&text, &["auth", "crypto", "permission"]) {
                4
            } else {
                2
            };

            let score = (impact as i32 * 3) + (risk as i32 * 2) - effort as i32;
            ScoredIssue {
                issue,
                impact,
                effort,
                risk,
                score,
            }
        })
        .collect();

    out.sort_by(|a, b| b.score.cmp(&a.score).then(a.issue.id.cmp(&b.issue.id)));
    out
}

pub fn write_scored(path: &Path, issues: &[ScoredIssue]) -> Result<()> {
    write_json(path, issues)
}

pub fn build_roadmap(scored: Vec<ScoredIssue>, top_n: usize) -> Roadmap {
    let mut items = scored;
    items.truncate(top_n.max(1));
    Roadmap {
        generated_at: Utc::now().to_rfc3339(),
        items,
    }
}

pub fn write_roadmap_markdown(path: &Path, roadmap: &Roadmap) -> Result<()> {
    let mut body = String::new();
    body.push_str("# Pilot Roadmap\n\n");
    body.push_str(&format!("Generated at: `{}`\n\n", roadmap.generated_at));
    for (idx, item) in roadmap.items.iter().enumerate() {
        body.push_str(&format!(
            "{}. [{}] {} (score={}, impact={}, risk={}, effort={})\n",
            idx + 1,
            item.issue.id,
            item.issue.title,
            item.score,
            item.impact,
            item.risk,
            item.effort
        ));
        if let Some(url) = &item.issue.html_url {
            body.push_str(&format!("   - {}\n", url));
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)?;
    Ok(())
}

pub fn default_plan_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".pilot").join("plan")
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(value)?;
    std::fs::write(path, body)?;
    Ok(())
}

fn has_any(labels: &[String], expected: &[&str]) -> bool {
    expected.iter().any(|e| labels.iter().any(|l| l == e))
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|t| text.contains(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_ordering() {
        let issues = vec![
            PlanIssue {
                id: 1,
                title: "security outage".to_string(),
                body: "".to_string(),
                labels: vec!["security".to_string()],
                state: "open".to_string(),
                html_url: None,
            },
            PlanIssue {
                id: 2,
                title: "small cleanup".to_string(),
                body: "".to_string(),
                labels: vec!["size/s".to_string()],
                state: "open".to_string(),
                html_url: None,
            },
        ];

        let scored = score_issues(issues);
        assert_eq!(scored[0].issue.id, 1);
        assert!(scored[0].score >= scored[1].score);
    }
}
