use anyhow::{anyhow, Context, Result};
use pilot_oracle::query::QueryEngine;
use pilot_oracle::query::QueryResult;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    pub id: i64,
    pub name: String,
    pub path: PathBuf,
    pub group_name: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoFilter {
    pub group: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub repo: RepoEntry,
    pub path_exists: bool,
    pub is_git_repo: bool,
    pub git_clean: Option<bool>,
    pub pilot_initialized: bool,
    pub oracle_ready: bool,
}

#[derive(Debug)]
pub struct MultiQueryResult {
    pub repo: String,
    pub repo_path: PathBuf,
    pub results: Vec<QueryResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedRepoPlan {
    pub repo: String,
    pub path: PathBuf,
    pub base_branch: String,
    pub head_branch: String,
    pub order: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedPrPlan {
    pub generated_at: String,
    pub repos: Vec<LinkedRepoPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoDependencyEdge {
    pub repo: String,
    pub depends_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyDagReport {
    pub generated_at: String,
    pub repos: Vec<RepoEntry>,
    pub edges: Vec<RepoDependencyEdge>,
    pub stages: Vec<Vec<String>>,
}

pub struct MultiRegistry {
    conn: Connection,
}

impl MultiRegistry {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create registry dir: {}", parent.display()))?;
        }

        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open registry DB: {}", db_path.display()))?;

        let registry = Self { conn };
        registry.init_db()?;
        Ok(registry)
    }

    pub fn default_db_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".pilot").join("workspace.db")
    }

    fn init_db(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS repos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                group_name TEXT
            );

            CREATE TABLE IF NOT EXISTS repo_tags (
                repo_id INTEGER NOT NULL,
                tag TEXT NOT NULL,
                UNIQUE(repo_id, tag),
                FOREIGN KEY(repo_id) REFERENCES repos(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS repo_deps (
                repo_id INTEGER NOT NULL,
                depends_on_repo_id INTEGER NOT NULL,
                UNIQUE(repo_id, depends_on_repo_id),
                FOREIGN KEY(repo_id) REFERENCES repos(id) ON DELETE CASCADE,
                FOREIGN KEY(depends_on_repo_id) REFERENCES repos(id) ON DELETE CASCADE
            );
            ",
        )?;
        Ok(())
    }

    pub fn register_repo(
        &self,
        path: &Path,
        name: Option<&str>,
        group_name: Option<&str>,
        tags: &[String],
    ) -> Result<RepoEntry> {
        let canonical = path
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;

        if !canonical.is_dir() {
            return Err(anyhow!("Not a directory: {}", canonical.display()));
        }

        let repo_name = name
            .map(ToOwned::to_owned)
            .or_else(|| {
                canonical
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "repo".to_string());

        self.conn.execute(
            "INSERT INTO repos(name, path, group_name) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET name=excluded.name, group_name=excluded.group_name",
            params![repo_name, canonical.to_string_lossy(), group_name],
        )?;

        let repo_id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM repos WHERE path = ?1",
                [canonical.to_string_lossy().to_string()],
                |row| row.get(0),
            )
            .with_context(|| {
                format!(
                    "Failed to fetch registered repo ID: {}",
                    canonical.display()
                )
            })?;

        self.conn
            .execute("DELETE FROM repo_tags WHERE repo_id = ?1", [repo_id])?;

        for tag in dedup(tags) {
            self.conn.execute(
                "INSERT OR IGNORE INTO repo_tags(repo_id, tag) VALUES (?1, ?2)",
                params![repo_id, tag],
            )?;
        }

        self.get_repo_by_id(repo_id)
            .ok_or_else(|| anyhow!("Registered repo not found after insert"))
    }

    pub fn list_repos(&self, filter: &RepoFilter) -> Result<Vec<RepoEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, group_name FROM repos
             ORDER BY COALESCE(group_name, ''), name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(RepoEntry {
                id: row.get(0)?,
                name: row.get(1)?,
                path: PathBuf::from(row.get::<_, String>(2)?),
                group_name: row.get(3)?,
                tags: vec![],
            })
        })?;

        let mut repos = Vec::new();
        for row in rows {
            let mut repo = row?;
            repo.tags = self.tags_for_repo(repo.id)?;
            repos.push(repo);
        }

        Ok(filter_repos(repos, filter))
    }

    pub fn set_dependencies(&self, repo_name: &str, depends_on_names: &[String]) -> Result<()> {
        let repo_id = self
            .repo_id_by_name(repo_name)?
            .ok_or_else(|| anyhow!("Repo not found: {}", repo_name))?;

        self.conn
            .execute("DELETE FROM repo_deps WHERE repo_id = ?1", [repo_id])?;

        for dep in depends_on_names {
            let dep_id = self
                .repo_id_by_name(dep)?
                .ok_or_else(|| anyhow!("Dependency repo not found: {}", dep))?;
            self.conn.execute(
                "INSERT OR IGNORE INTO repo_deps(repo_id, depends_on_repo_id) VALUES (?1, ?2)",
                params![repo_id, dep_id],
            )?;
        }

        Ok(())
    }

    pub fn dependency_order(&self, filter: &RepoFilter) -> Result<Vec<RepoEntry>> {
        let repos = self.list_repos(filter)?;
        let selected_ids: HashSet<i64> = repos.iter().map(|r| r.id).collect();
        let repo_by_id: HashMap<i64, RepoEntry> = repos.into_iter().map(|r| (r.id, r)).collect();

        let mut indegree: HashMap<i64, usize> = repo_by_id.keys().map(|id| (*id, 0usize)).collect();
        let mut graph: HashMap<i64, Vec<i64>> = HashMap::new();

        let mut stmt = self
            .conn
            .prepare("SELECT repo_id, depends_on_repo_id FROM repo_deps")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;

        for row in rows {
            let (repo_id, dep_id) = row?;
            if !(selected_ids.contains(&repo_id) && selected_ids.contains(&dep_id)) {
                continue;
            }
            graph.entry(dep_id).or_default().push(repo_id);
            *indegree.entry(repo_id).or_insert(0) += 1;
        }

        let mut queue: VecDeque<i64> = {
            let mut roots: Vec<i64> = indegree
                .iter()
                .filter_map(|(id, deg)| if *deg == 0 { Some(*id) } else { None })
                .collect();
            roots.sort_by(|a, b| {
                let an = repo_by_id.get(a).map(|r| r.name.as_str()).unwrap_or("");
                let bn = repo_by_id.get(b).map(|r| r.name.as_str()).unwrap_or("");
                an.cmp(bn).then(a.cmp(b))
            });
            roots.into_iter().collect()
        };
        let mut ordered_ids = Vec::new();

        while let Some(id) = queue.pop_front() {
            ordered_ids.push(id);
            if let Some(children) = graph.get(&id) {
                for child in children {
                    if let Some(deg) = indegree.get_mut(child) {
                        *deg -= 1;
                        if *deg == 0 {
                            let insert_at = queue
                                .iter()
                                .position(|existing| {
                                    let cn = repo_by_id
                                        .get(child)
                                        .map(|r| r.name.as_str())
                                        .unwrap_or("");
                                    let en = repo_by_id
                                        .get(existing)
                                        .map(|r| r.name.as_str())
                                        .unwrap_or("");
                                    cn < en || (cn == en && child < existing)
                                })
                                .unwrap_or(queue.len());
                            queue.insert(insert_at, *child);
                        }
                    }
                }
            }
        }

        if ordered_ids.len() != selected_ids.len() {
            return Err(anyhow!(
                "Dependency graph has a cycle or unresolved dependency references"
            ));
        }

        Ok(ordered_ids
            .into_iter()
            .filter_map(|id| repo_by_id.get(&id).cloned())
            .collect())
    }

    pub fn dependency_stages(&self, filter: &RepoFilter) -> Result<Vec<Vec<RepoEntry>>> {
        let repos = self.list_repos(filter)?;
        let selected_ids: HashSet<i64> = repos.iter().map(|r| r.id).collect();
        let repo_by_id: HashMap<i64, RepoEntry> = repos.into_iter().map(|r| (r.id, r)).collect();

        let mut indegree: HashMap<i64, usize> = repo_by_id.keys().map(|id| (*id, 0usize)).collect();
        let mut graph: HashMap<i64, Vec<i64>> = HashMap::new();

        let mut stmt = self
            .conn
            .prepare("SELECT repo_id, depends_on_repo_id FROM repo_deps")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;

        for row in rows {
            let (repo_id, dep_id) = row?;
            if !(selected_ids.contains(&repo_id) && selected_ids.contains(&dep_id)) {
                continue;
            }
            graph.entry(dep_id).or_default().push(repo_id);
            *indegree.entry(repo_id).or_insert(0) += 1;
        }

        let mut queue: Vec<i64> = indegree
            .iter()
            .filter_map(|(id, deg)| if *deg == 0 { Some(*id) } else { None })
            .collect();
        queue.sort_by(|a, b| {
            let an = repo_by_id.get(a).map(|r| r.name.as_str()).unwrap_or("");
            let bn = repo_by_id.get(b).map(|r| r.name.as_str()).unwrap_or("");
            an.cmp(bn).then(a.cmp(b))
        });

        let mut visited = 0usize;
        let mut stages = Vec::new();

        while !queue.is_empty() {
            let current = queue.clone();
            queue.clear();

            let mut stage: Vec<RepoEntry> = current
                .iter()
                .filter_map(|id| repo_by_id.get(id).cloned())
                .collect();
            stage.sort_by(|a, b| a.name.cmp(&b.name));
            visited += stage.len();
            stages.push(stage);

            let mut next = Vec::new();
            for id in current {
                if let Some(children) = graph.get(&id) {
                    for child in children {
                        if let Some(deg) = indegree.get_mut(child) {
                            *deg -= 1;
                            if *deg == 0 {
                                next.push(*child);
                            }
                        }
                    }
                }
            }
            next.sort_by(|a, b| {
                let an = repo_by_id.get(a).map(|r| r.name.as_str()).unwrap_or("");
                let bn = repo_by_id.get(b).map(|r| r.name.as_str()).unwrap_or("");
                an.cmp(bn).then(a.cmp(b))
            });
            next.dedup();
            queue = next;
        }

        if visited != selected_ids.len() {
            return Err(anyhow!(
                "Dependency graph has a cycle or unresolved dependency references"
            ));
        }

        Ok(stages)
    }

    pub fn dependency_dag_report(&self, filter: &RepoFilter) -> Result<DependencyDagReport> {
        let repos = self.list_repos(filter)?;
        let selected_ids: HashSet<i64> = repos.iter().map(|r| r.id).collect();
        let name_by_id: HashMap<i64, String> =
            repos.iter().map(|r| (r.id, r.name.clone())).collect();

        let mut edges = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT repo_id, depends_on_repo_id FROM repo_deps")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;

        for row in rows {
            let (repo_id, dep_id) = row?;
            if !(selected_ids.contains(&repo_id) && selected_ids.contains(&dep_id)) {
                continue;
            }
            let repo_name = name_by_id.get(&repo_id).cloned().unwrap_or_default();
            let dep_name = name_by_id.get(&dep_id).cloned().unwrap_or_default();
            edges.push(RepoDependencyEdge {
                repo: repo_name,
                depends_on: dep_name,
            });
        }
        edges.sort_by(|a, b| a.depends_on.cmp(&b.depends_on).then(a.repo.cmp(&b.repo)));

        let stages = self
            .dependency_stages(filter)?
            .into_iter()
            .map(|s| s.into_iter().map(|r| r.name).collect())
            .collect();

        Ok(DependencyDagReport {
            generated_at: chrono::Utc::now().to_rfc3339(),
            repos,
            edges,
            stages,
        })
    }

    pub fn write_dependency_dag_report(
        &self,
        filter: &RepoFilter,
        output: Option<&Path>,
    ) -> Result<PathBuf> {
        let report = self.dependency_dag_report(filter)?;
        let out_path = if let Some(p) = output {
            p.to_path_buf()
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".pilot").join(format!(
                "dependency_dag_{}.json",
                chrono::Utc::now().timestamp()
            ))
        };
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = serde_json::to_string_pretty(&report)?;
        std::fs::write(&out_path, body)?;
        Ok(out_path)
    }

    pub fn generate_linked_pr_plan(
        &self,
        filter: &RepoFilter,
        head_branch: &str,
        base_branch: &str,
        output: Option<&Path>,
    ) -> Result<PathBuf> {
        let ordered = self.dependency_order(filter)?;

        let repos = ordered
            .into_iter()
            .enumerate()
            .map(|(i, r)| LinkedRepoPlan {
                repo: r.name,
                path: r.path,
                base_branch: base_branch.to_string(),
                head_branch: head_branch.to_string(),
                order: i + 1,
            })
            .collect();

        let plan = LinkedPrPlan {
            generated_at: chrono::Utc::now().to_rfc3339(),
            repos,
        };

        let out_path = if let Some(p) = output {
            p.to_path_buf()
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".pilot").join(format!(
                "linked_prs_{}.json",
                chrono::Utc::now().timestamp()
            ))
        };

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let body = serde_json::to_string_pretty(&plan)?;
        std::fs::write(&out_path, body)?;

        Ok(out_path)
    }

    pub fn status_repos(&self, filter: &RepoFilter) -> Result<Vec<RepoStatus>> {
        let repos = self.list_repos(filter)?;
        let mut statuses = Vec::with_capacity(repos.len());

        for repo in repos {
            let path_exists = repo.path.exists();
            let is_git_repo = repo.path.join(".git").exists();
            let git_clean = if is_git_repo {
                Some(is_git_clean(&repo.path).unwrap_or(false))
            } else {
                None
            };

            let pilot_initialized = repo.path.join(".pilot/config.toml").exists();
            let oracle_ready = repo.path.join(".pilot/graph.db").exists()
                && repo.path.join(".pilot/vectors.lance").exists();

            statuses.push(RepoStatus {
                repo,
                path_exists,
                is_git_repo,
                git_clean,
                pilot_initialized,
                oracle_ready,
            });
        }

        Ok(statuses)
    }

    pub async fn query_across_repos(
        &self,
        filter: &RepoFilter,
        query: &str,
        per_repo_limit: usize,
    ) -> Result<Vec<MultiQueryResult>> {
        let repos = self.list_repos(filter)?;
        let mut out = Vec::with_capacity(repos.len());

        for repo in repos {
            let db_path = repo.path.join(".pilot/graph.db");
            let vector_path = repo.path.join(".pilot/vectors.lance");

            if !db_path.exists() || !vector_path.exists() {
                out.push(MultiQueryResult {
                    repo: repo.name,
                    repo_path: repo.path,
                    results: vec![],
                    error: Some("Oracle index not initialized".to_string()),
                });
                continue;
            }

            let repo_name = repo.name;
            let repo_path = repo.path;

            let engine = QueryEngine::new(
                db_path.to_string_lossy().as_ref(),
                vector_path.to_string_lossy().as_ref(),
            )
            .await;

            match engine {
                Ok(mut engine) => match engine.query(query).await {
                    Ok(mut results) => {
                        results.truncate(per_repo_limit);
                        out.push(MultiQueryResult {
                            repo: repo_name,
                            repo_path,
                            results,
                            error: None,
                        });
                    }
                    Err(err) => {
                        out.push(MultiQueryResult {
                            repo: repo_name,
                            repo_path,
                            results: vec![],
                            error: Some(format!("Query failed: {:?}", err)),
                        });
                    }
                },
                Err(err) => {
                    out.push(MultiQueryResult {
                        repo: repo_name,
                        repo_path,
                        results: vec![],
                        error: Some(format!("Engine init failed: {:?}", err)),
                    });
                }
            }
        }

        Ok(out)
    }

    fn repo_id_by_name(&self, name: &str) -> Result<Option<i64>> {
        self.conn
            .query_row("SELECT id FROM repos WHERE name = ?1", [name], |row| {
                row.get(0)
            })
            .optional()
            .map_err(Into::into)
    }

    fn get_repo_by_id(&self, id: i64) -> Option<RepoEntry> {
        self.conn
            .query_row(
                "SELECT id, name, path, group_name FROM repos WHERE id = ?1",
                [id],
                |row| {
                    Ok(RepoEntry {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        path: PathBuf::from(row.get::<_, String>(2)?),
                        group_name: row.get(3)?,
                        tags: vec![],
                    })
                },
            )
            .optional()
            .ok()
            .flatten()
            .map(|mut repo| {
                repo.tags = self.tags_for_repo(repo.id).unwrap_or_default();
                repo
            })
    }

    fn tags_for_repo(&self, repo_id: i64) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM repo_tags WHERE repo_id = ?1 ORDER BY tag")?;
        let rows = stmt.query_map([repo_id], |row| row.get(0))?;
        let mut tags = Vec::new();
        for r in rows {
            tags.push(r?);
        }
        Ok(tags)
    }
}

fn dedup(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for t in tags {
        let trimmed = t.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn filter_repos(mut repos: Vec<RepoEntry>, filter: &RepoFilter) -> Vec<RepoEntry> {
    if let Some(group) = &filter.group {
        repos.retain(|r| r.group_name.as_deref() == Some(group.as_str()));
    }

    if !filter.tags.is_empty() {
        repos.retain(|r| {
            let repo_tags: HashSet<_> = r.tags.iter().collect();
            filter.tags.iter().all(|t| repo_tags.contains(t))
        });
    }

    repos
}

fn is_git_clean(repo: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("Failed to run git status in {}", repo.display()))?;

    if !output.status.success() {
        return Ok(false);
    }

    Ok(output.stdout.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_list_with_filters() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("workspace.db");
        let registry = MultiRegistry::open(&db).unwrap();

        let r1 = dir.path().join("repo1");
        let r2 = dir.path().join("repo2");
        std::fs::create_dir_all(&r1).unwrap();
        std::fs::create_dir_all(&r2).unwrap();

        registry
            .register_repo(
                &r1,
                Some("repo1"),
                Some("core"),
                &["rust".to_string(), "service".to_string()],
            )
            .unwrap();
        registry
            .register_repo(&r2, Some("repo2"), Some("ml"), &["python".to_string()])
            .unwrap();

        let all = registry.list_repos(&RepoFilter::default()).unwrap();
        assert_eq!(all.len(), 2);

        let core_only = registry
            .list_repos(&RepoFilter {
                group: Some("core".to_string()),
                tags: vec![],
            })
            .unwrap();
        assert_eq!(core_only.len(), 1);
        assert_eq!(core_only[0].name, "repo1");

        let rust_only = registry
            .list_repos(&RepoFilter {
                group: None,
                tags: vec!["rust".to_string()],
            })
            .unwrap();
        assert_eq!(rust_only.len(), 1);
        assert_eq!(rust_only[0].name, "repo1");
    }

    #[test]
    fn test_dependency_order() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("workspace.db");
        let registry = MultiRegistry::open(&db).unwrap();

        let r1 = dir.path().join("repo1");
        let r2 = dir.path().join("repo2");
        let r3 = dir.path().join("repo3");
        std::fs::create_dir_all(&r1).unwrap();
        std::fs::create_dir_all(&r2).unwrap();
        std::fs::create_dir_all(&r3).unwrap();

        registry
            .register_repo(&r1, Some("repo1"), Some("core"), &[])
            .unwrap();
        registry
            .register_repo(&r2, Some("repo2"), Some("core"), &[])
            .unwrap();
        registry
            .register_repo(&r3, Some("repo3"), Some("core"), &[])
            .unwrap();

        registry
            .set_dependencies("repo3", &["repo2".to_string()])
            .unwrap();
        registry
            .set_dependencies("repo2", &["repo1".to_string()])
            .unwrap();

        let order = registry.dependency_order(&RepoFilter::default()).unwrap();
        let names: Vec<_> = order.into_iter().map(|r| r.name).collect();

        let i1 = names.iter().position(|n| n == "repo1").unwrap();
        let i2 = names.iter().position(|n| n == "repo2").unwrap();
        let i3 = names.iter().position(|n| n == "repo3").unwrap();

        assert!(i1 < i2 && i2 < i3);
    }

    #[test]
    fn test_dependency_stages_and_dag_report() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("workspace.db");
        let registry = MultiRegistry::open(&db).unwrap();

        let r1 = dir.path().join("repo1");
        let r2 = dir.path().join("repo2");
        let r3 = dir.path().join("repo3");
        std::fs::create_dir_all(&r1).unwrap();
        std::fs::create_dir_all(&r2).unwrap();
        std::fs::create_dir_all(&r3).unwrap();

        registry
            .register_repo(&r1, Some("repo1"), Some("core"), &[])
            .unwrap();
        registry
            .register_repo(&r2, Some("repo2"), Some("core"), &[])
            .unwrap();
        registry
            .register_repo(&r3, Some("repo3"), Some("core"), &[])
            .unwrap();
        registry
            .set_dependencies("repo2", &["repo1".to_string()])
            .unwrap();
        registry
            .set_dependencies("repo3", &["repo2".to_string()])
            .unwrap();

        let stages = registry.dependency_stages(&RepoFilter::default()).unwrap();
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0][0].name, "repo1");
        assert_eq!(stages[1][0].name, "repo2");
        assert_eq!(stages[2][0].name, "repo3");

        let dag = registry
            .dependency_dag_report(&RepoFilter::default())
            .unwrap();
        assert_eq!(dag.repos.len(), 3);
        assert_eq!(dag.edges.len(), 2);
        assert_eq!(dag.stages.len(), 3);
    }

    #[test]
    fn test_register_repo_idempotent_on_same_path() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("workspace.db");
        let registry = MultiRegistry::open(&db).unwrap();

        let r1 = dir.path().join("repo1");
        std::fs::create_dir_all(&r1).unwrap();

        let first = registry
            .register_repo(
                &r1,
                Some("repo1"),
                Some("core"),
                &["apply-pilot".to_string(), "operator".to_string()],
            )
            .unwrap();
        let second = registry
            .register_repo(
                &r1,
                Some("repo1"),
                Some("core"),
                &["operator".to_string(), "apply-pilot".to_string()],
            )
            .unwrap();
        assert_eq!(first.id, second.id);

        let repos = registry.list_repos(&RepoFilter::default()).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "repo1");
        assert_eq!(repos[0].group_name.as_deref(), Some("core"));

        let updated = registry
            .register_repo(
                &r1,
                Some("repo1-renamed"),
                Some("platform"),
                &["operator".to_string()],
            )
            .unwrap();
        assert_eq!(updated.id, first.id);
        let repos_after = registry.list_repos(&RepoFilter::default()).unwrap();
        assert_eq!(repos_after.len(), 1);
        assert_eq!(repos_after[0].name, "repo1-renamed");
        assert_eq!(repos_after[0].group_name.as_deref(), Some("platform"));
        assert_eq!(repos_after[0].tags, vec!["operator".to_string()]);
    }
}
