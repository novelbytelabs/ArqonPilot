use crate::db_runtime::PilotDbManager;
use miette::{miette, Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

const ACTIVE_AGORG_KEY: &str = "active_agorg_id";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agorg {
    pub id: Uuid,
    pub name: String,
    pub root_path: String,
    pub master_path: Option<String>,
    pub parent_agorg_id: Option<Uuid>,
    pub default_scope: bool,
    pub scan_depth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgoRecord {
    pub id: Uuid,
    pub agorg_id: Uuid,
    pub name: String,
    pub repo_path: String,
    pub relationship_parent: Option<String>,
    pub relationship_children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverCandidate {
    pub name: String,
    pub path: String,
    pub kind: String, // "agorg", "ago", "none"
    pub parent_hint: Option<String>,
    pub children_hints: Vec<String>,
    pub is_registered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverResult {
    pub root: String,
    pub depth: usize,
    pub candidates: Vec<DiscoverCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDiscoverySummary {
    pub upserted: usize,
    pub pruned: usize,
    pub final_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgorgTreeNode {
    pub agorg: Agorg,
    pub child_agorgs: Vec<AgorgTreeNode>,
    pub agos: Vec<AgoRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgorgReconcileIssue {
    pub repo_name: String,
    pub repo_path: String,
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgorgReconcileReport {
    pub agorg_id: Uuid,
    pub agorg_name: String,
    pub root_path: String,
    pub total_agos: usize,
    pub issue_count: usize,
    pub off_policy_count: usize,
    pub prune_candidate_paths: Vec<String>,
    pub issues: Vec<AgorgReconcileIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRelationships {
    pub parent: Option<String>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AgorgStore {
    dsn_override: Option<String>,
    db: PilotDbManager,
}

impl AgorgStore {
    pub fn from_env() -> Self {
        Self {
            dsn_override: std::env::var("PILOT_AGORG_DATABASE_URL").ok(),
            db: PilotDbManager::from_env(),
        }
    }

    pub fn dsn(&self) -> String {
        match &self.dsn_override {
            Some(s) => s.clone(),
            None => self.db.target_dsn(),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        if self.dsn_override.is_none() {
            self.db.ensure_ready().await?;
        }
        self.ensure_database_exists().await?;
        let client = self.connect().await?;
        self.ensure_schema(&client).await
    }

    pub async fn managed_db_status(&self) -> Result<Option<crate::db_runtime::DbStatus>> {
        if self.dsn_override.is_some() {
            return Ok(None);
        }
        Ok(Some(self.db.status().await?))
    }

    pub async fn ensure_managed_db(&self) -> Result<Option<crate::db_runtime::DbStatus>> {
        if self.dsn_override.is_some() {
            return Ok(None);
        }
        self.db.ensure_ready().await?;
        Ok(Some(self.db.status().await?))
    }

    pub async fn stop_managed_db(&self) -> Result<Option<crate::db_runtime::DbStatus>> {
        if self.dsn_override.is_some() {
            return Ok(None);
        }
        self.db.stop().await?;
        Ok(Some(self.db.status().await?))
    }

    pub async fn create_agorg(
        &self,
        name: &str,
        root_path: &Path,
        master_path: Option<&str>,
        parent_agorg_id: Option<Uuid>,
        scan_depth: i32,
        set_default: bool,
    ) -> Result<Agorg> {
        self.initialize().await?;
        let client = self.connect().await?;
        let canonical = canonicalize_or_input(root_path);

        if set_default {
            client
                .execute("UPDATE agorgs SET default_scope = FALSE", &[])
                .await
                .into_diagnostic()?;
        }

        let id = Uuid::new_v4();
        let row = client
            .query_one(
                "INSERT INTO agorgs (id, name, root_path, master_path, parent_agorg_id, default_scope, scan_depth)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (root_path) DO UPDATE SET
                    name = EXCLUDED.name,
                    master_path = EXCLUDED.master_path,
                    parent_agorg_id = EXCLUDED.parent_agorg_id,
                    default_scope = EXCLUDED.default_scope,
                    scan_depth = EXCLUDED.scan_depth,
                    updated_at = NOW()
                 RETURNING id, name, root_path, master_path, parent_agorg_id, default_scope, scan_depth",
                &[&id, &name, &canonical, &master_path, &parent_agorg_id, &set_default, &scan_depth],
            )
            .await
            .into_diagnostic()
            .with_context(|| format!("Failed creating/updating AGOrg '{}'", name))?;

        if set_default {
            self.set_active_agorg(id).await?;
        }

        Ok(row_to_agorg(&row))
    }

    pub async fn update_agorg(
        &self,
        id: Uuid,
        name: Option<String>,
        root_path: Option<PathBuf>,
        master_path: Option<String>,
        scan_depth: Option<i32>,
        set_default: Option<bool>,
    ) -> Result<Agorg> {
        self.initialize().await?;
        let client = self.connect().await?;
        if let Some(v) = set_default {
            if v {
                client
                    .execute("UPDATE agorgs SET default_scope = FALSE", &[])
                    .await
                    .into_diagnostic()?;
            }
        }
        let root = root_path
            .as_ref()
            .map(|p| canonicalize_or_input(p))
            .unwrap_or_default();
        let row = client
            .query_one(
                "UPDATE agorgs
                 SET name = COALESCE($2, name),
                     root_path = CASE WHEN $3 <> '' THEN $3 ELSE root_path END,
                     master_path = COALESCE($4, master_path),
                     scan_depth = COALESCE($5, scan_depth),
                     default_scope = COALESCE($6, default_scope),
                     updated_at = NOW()
                 WHERE id = $1
                 RETURNING id, name, root_path, master_path, parent_agorg_id, default_scope, scan_depth",
                &[&id, &name, &root, &master_path, &scan_depth, &set_default],
            )
            .await
            .into_diagnostic()
            .with_context(|| format!("Failed updating AGOrg {}", id))?;
        let ag = row_to_agorg(&row);
        if ag.default_scope {
            self.set_active_agorg(ag.id).await?;
        }
        Ok(ag)
    }

    pub async fn delete_agorg(&self, id: Uuid) -> Result<u64> {
        self.initialize().await?;
        let client = self.connect().await?;
        let deleted = client
            .execute("DELETE FROM agorgs WHERE id = $1", &[&id])
            .await
            .into_diagnostic()?;
        Ok(deleted)
    }

    pub async fn list_agorgs(&self) -> Result<Vec<Agorg>> {
        self.initialize().await?;
        let client = self.connect().await?;
        let rows = client
            .query(
                "SELECT id, name, root_path, master_path, parent_agorg_id, default_scope, scan_depth
                 FROM agorgs
                 ORDER BY name ASC",
                &[],
            )
            .await
            .into_diagnostic()?;
        Ok(rows.iter().map(row_to_agorg).collect())
    }

    pub async fn set_active_agorg(&self, agorg_id: Uuid) -> Result<()> {
        self.initialize().await?;
        let client = self.connect().await?;
        client
            .execute(
                "INSERT INTO app_state (key, value)
                 VALUES ($1, jsonb_build_object('agorg_id', $2::text))
                 ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
                &[&ACTIVE_AGORG_KEY, &agorg_id.to_string()],
            )
            .await
            .into_diagnostic()?;
        Ok(())
    }

    pub async fn get_active_agorg(&self) -> Result<Option<Agorg>> {
        self.initialize().await?;
        let client = self.connect().await?;
        let row = client
            .query_opt(
                "SELECT value FROM app_state WHERE key = $1",
                &[&ACTIVE_AGORG_KEY],
            )
            .await
            .into_diagnostic()?;
        let Some(row) = row else {
            return Ok(None);
        };
        let value: serde_json::Value = row.get(0);
        let Some(id_text) = value.get("agorg_id").and_then(|v| v.as_str()) else {
            return Ok(None);
        };
        let id = Uuid::parse_str(id_text).into_diagnostic()?;
        self.get_agorg(id).await
    }

    pub async fn get_agorg(&self, id: Uuid) -> Result<Option<Agorg>> {
        self.initialize().await?;
        let client = self.connect().await?;
        let row = client
            .query_opt(
                "SELECT id, name, root_path, master_path, parent_agorg_id, default_scope, scan_depth
                 FROM agorgs
                 WHERE id = $1",
                &[&id],
            )
            .await
            .into_diagnostic()?;
        Ok(row.as_ref().map(row_to_agorg))
    }

    pub async fn link_agorgs(&self, parent: Uuid, child: Uuid) -> Result<()> {
        if parent == child {
            return Err(miette!("Cannot link AGOrg to itself"));
        }
        self.initialize().await?;
        let client = self.connect().await?;
        let cycle = client
            .query_opt(
                "WITH RECURSIVE walk(id) AS (
                    SELECT $1::uuid
                    UNION
                    SELECT l.child_agorg_id
                    FROM agorg_links l
                    JOIN walk w ON l.parent_agorg_id = w.id
                 )
                 SELECT 1 FROM walk WHERE id = $2::uuid LIMIT 1",
                &[&child, &parent],
            )
            .await
            .into_diagnostic()?
            .is_some();
        if cycle {
            return Err(miette!(
                "Link would create circular AGOrg dependency ({} -> {})",
                parent,
                child
            ));
        }
        client
            .execute(
                "INSERT INTO agorg_links (parent_agorg_id, child_agorg_id)
                 VALUES ($1, $2)
                 ON CONFLICT (parent_agorg_id, child_agorg_id) DO NOTHING",
                &[&parent, &child],
            )
            .await
            .into_diagnostic()?;
        Ok(())
    }

    pub async fn upsert_ago(
        &self,
        agorg_id: Uuid,
        name: &str,
        repo_path: &Path,
        rel_parent: Option<&str>,
        rel_children: &[String],
    ) -> Result<AgoRecord> {
        self.initialize().await?;
        let client = self.connect().await?;
        let id = Uuid::new_v4();
        let canonical = canonicalize_or_input(repo_path);
        let children_json = serde_json::to_value(rel_children).into_diagnostic()?;
        let row = client
            .query_one(
                "INSERT INTO agos (id, agorg_id, name, repo_path, relationship_parent, relationship_children)
                 VALUES ($1, $2, $3, $4, $5, $6::jsonb)
                 ON CONFLICT (agorg_id, repo_path)
                 DO UPDATE SET
                    name = EXCLUDED.name,
                    relationship_parent = EXCLUDED.relationship_parent,
                    relationship_children = EXCLUDED.relationship_children,
                    updated_at = NOW()
                 RETURNING id, agorg_id, name, repo_path, relationship_parent, relationship_children",
                &[&id, &agorg_id, &name, &canonical, &rel_parent, &children_json],
            )
            .await
            .into_diagnostic()?;
        Ok(row_to_ago(&row))
    }

    pub async fn tree(&self, root: Option<Uuid>) -> Result<Vec<AgorgTreeNode>> {
        self.initialize().await?;
        let agorgs = self.list_agorgs().await?;
        let client = self.connect().await?;
        let link_rows = client
            .query(
                "SELECT parent_agorg_id, child_agorg_id FROM agorg_links ORDER BY parent_agorg_id",
                &[],
            )
            .await
            .into_diagnostic()?;
        let ago_rows = client
            .query(
                "SELECT id, agorg_id, name, repo_path, relationship_parent, relationship_children FROM agos",
                &[],
            )
            .await
            .into_diagnostic()?;
        let agos: Vec<AgoRecord> = ago_rows.iter().map(row_to_ago).collect();

        let mut children_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let mut all_children = HashSet::new();
        for row in link_rows {
            let p: Uuid = row.get(0);
            let c: Uuid = row.get(1);
            children_map.entry(p).or_default().push(c);
            all_children.insert(c);
        }
        let roots: Vec<Uuid> = if let Some(r) = root {
            vec![r]
        } else {
            agorgs
                .iter()
                .filter(|a| !all_children.contains(&a.id))
                .map(|a| a.id)
                .collect()
        };
        let agorg_by_id: HashMap<Uuid, Agorg> = agorgs.into_iter().map(|a| (a.id, a)).collect();
        let mut ago_map: HashMap<Uuid, Vec<AgoRecord>> = HashMap::new();
        for ago in agos {
            ago_map.entry(ago.agorg_id).or_default().push(ago);
        }

        fn build_node(
            id: Uuid,
            agorg_by_id: &HashMap<Uuid, Agorg>,
            children_map: &HashMap<Uuid, Vec<Uuid>>,
            ago_map: &HashMap<Uuid, Vec<AgoRecord>>,
            visiting: &mut HashSet<Uuid>,
        ) -> Option<AgorgTreeNode> {
            let agorg = agorg_by_id.get(&id)?.clone();
            if !visiting.insert(id) {
                return None;
            }
            let mut child_nodes = Vec::new();
            if let Some(children) = children_map.get(&id) {
                for child in children {
                    if let Some(node) =
                        build_node(*child, agorg_by_id, children_map, ago_map, visiting)
                    {
                        child_nodes.push(node);
                    }
                }
            }
            visiting.remove(&id);
            Some(AgorgTreeNode {
                agorg,
                child_agorgs: child_nodes,
                agos: ago_map.get(&id).cloned().unwrap_or_default(),
            })
        }

        let mut out = Vec::new();
        for root_id in roots {
            if let Some(node) = build_node(
                root_id,
                &agorg_by_id,
                &children_map,
                &ago_map,
                &mut HashSet::new(),
            ) {
                out.push(node);
            }
        }
        Ok(out)
    }

    pub async fn reconcile_agorg(&self, agorg_id: Uuid) -> Result<AgorgReconcileReport> {
        self.initialize().await?;
        let agorg = self
            .get_agorg(agorg_id)
            .await?
            .ok_or_else(|| miette!("AGOrg {} not found", agorg_id))?;
        let client = self.connect().await?;
        let rows = client
            .query(
                "SELECT id, agorg_id, name, repo_path, relationship_parent, relationship_children
                 FROM agos
                 WHERE agorg_id = $1
                 ORDER BY name ASC",
                &[&agorg_id],
            )
            .await
            .into_diagnostic()?;
        let agos: Vec<AgoRecord> = rows.iter().map(row_to_ago).collect();

        let root = PathBuf::from(&agorg.root_path);
        let mut issues: Vec<AgorgReconcileIssue> = Vec::new();
        let mut prune_candidates: HashSet<String> = HashSet::new();
        let mut name_counts: HashMap<String, usize> = HashMap::new();

        for ago in &agos {
            *name_counts.entry(ago.name.clone()).or_insert(0) += 1;

            let path = PathBuf::from(&ago.repo_path);
            if !path.exists() {
                issues.push(AgorgReconcileIssue {
                    repo_name: ago.name.clone(),
                    repo_path: ago.repo_path.clone(),
                    severity: "error".to_string(),
                    code: "repo_missing".to_string(),
                    message: "Repository path does not exist on disk".to_string(),
                });
                continue;
            }

            let rel_depth = path
                .strip_prefix(&root)
                .ok()
                .map(|p| p.components().count())
                .unwrap_or(0);

            let in_archive = path
                .components()
                .any(|c| c.as_os_str().to_string_lossy() == "archive");
            if in_archive {
                issues.push(AgorgReconcileIssue {
                    repo_name: ago.name.clone(),
                    repo_path: ago.repo_path.clone(),
                    severity: "warn".to_string(),
                    code: "archive_path".to_string(),
                    message: "Repository is under archive/; off-policy for active AGOrg fleet"
                        .to_string(),
                });
                prune_candidates.insert(ago.repo_path.clone());
            }

            if rel_depth > 1 {
                issues.push(AgorgReconcileIssue {
                    repo_name: ago.name.clone(),
                    repo_path: ago.repo_path.clone(),
                    severity: "warn".to_string(),
                    code: "nested_repo".to_string(),
                    message: format!(
                        "Repository is nested depth={} under AGOrg root; flat-fleet policy expects top-level",
                        rel_depth
                    ),
                });
                prune_candidates.insert(ago.repo_path.clone());
            }

            if !path.join("pyproject.toml").exists() {
                issues.push(AgorgReconcileIssue {
                    repo_name: ago.name.clone(),
                    repo_path: ago.repo_path.clone(),
                    severity: "warn".to_string(),
                    code: "missing_pyproject".to_string(),
                    message: "pyproject.toml not found".to_string(),
                });
            }
        }

        for (name, count) in name_counts {
            if count > 1 {
                issues.push(AgorgReconcileIssue {
                    repo_name: name.clone(),
                    repo_path: "".to_string(),
                    severity: "warn".to_string(),
                    code: "duplicate_name".to_string(),
                    message: format!("AGO name '{}' appears {} times in this AGOrg", name, count),
                });
            }
        }

        let prune_candidate_paths: Vec<String> = {
            let mut v: Vec<String> = prune_candidates.into_iter().collect();
            v.sort();
            v
        };

        Ok(AgorgReconcileReport {
            agorg_id: agorg.id,
            agorg_name: agorg.name,
            root_path: agorg.root_path,
            total_agos: agos.len(),
            issue_count: issues.len(),
            off_policy_count: prune_candidate_paths.len(),
            prune_candidate_paths,
            issues,
        })
    }

    pub async fn create_project(
        &self,
        name: &str,
        root: &Path,
        master: Option<&str>,
        parent: Option<Uuid>,
        scan_depth: usize,
        autoscan: bool,
        set_default: bool,
    ) -> Result<(Agorg, Option<DiscoverResult>)> {
        let agorg = self
            .create_agorg(name, root, master, parent, scan_depth as i32, set_default)
            .await?;
        let discovered = if autoscan {
            let scan = discover_hierarchy(root, scan_depth)?;
            self.import_discovery(agorg.id, &scan).await?;
            Some(scan)
        } else {
            None
        };
        Ok((agorg, discovered))
    }

    pub async fn import_discovery(
        &self,
        agorg_id: Uuid,
        discovery: &DiscoverResult,
    ) -> Result<ImportDiscoverySummary> {
        self.import_discovery_with_options(agorg_id, discovery, false)
            .await
    }

    pub async fn import_discovery_with_options(
        &self,
        agorg_id: Uuid,
        discovery: &DiscoverResult,
        prune_missing: bool,
    ) -> Result<ImportDiscoverySummary> {
        self.initialize().await?;
        let client = self.connect().await?;
        let mut upserted = 0usize;
        let mut keep_paths: Vec<String> = Vec::new();

        for c in &discovery.candidates {
            if c.kind == "ago" {
                let path = PathBuf::from(&c.path);
                keep_paths.push(canonicalize_or_input(&path));
                self.upsert_ago(
                    agorg_id,
                    &c.name,
                    &path,
                    c.parent_hint.as_deref(),
                    &c.children_hints,
                )
                .await?;
                upserted += 1;
            }
        }

        let pruned = if prune_missing {
            if keep_paths.is_empty() {
                client
                    .execute("DELETE FROM agos WHERE agorg_id = $1", &[&agorg_id])
                    .await
                    .into_diagnostic()? as usize
            } else {
                client
                    .execute(
                        "DELETE FROM agos
                         WHERE agorg_id = $1
                           AND NOT (repo_path = ANY($2::text[]))",
                        &[&agorg_id, &keep_paths],
                    )
                    .await
                    .into_diagnostic()? as usize
            }
        } else {
            0
        };

        let final_count: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM agos WHERE agorg_id = $1",
                &[&agorg_id],
            )
            .await
            .into_diagnostic()?
            .get(0);

        Ok(ImportDiscoverySummary {
            upserted,
            pruned,
            final_count: final_count as usize,
        })
    }

    pub async fn init_agorg_batch(
        &self,
        dest_parent: &Path,
        master_name: &str,
        siblings: &[String],
        use_git: bool,
    ) -> Result<Agorg> {
        let master_path = dest_parent.join(master_name);
        fs::create_dir_all(&master_path).into_diagnostic()?;

        // Primary AGOrg creation
        let agorg = self
            .create_agorg(master_name, &master_path, None, None, 4, true)
            .await?;

        for sib_name in siblings {
            let sib_path = master_path.join(sib_name);
            fs::create_dir_all(&sib_path).into_diagnostic()?;

            if use_git {
                let _ = tokio::process::Command::new("git")
                    .arg("init")
                    .current_dir(&sib_path)
                    .spawn();
            }

            // Initialize pyproject.toml as an indicator
            let pyproject = sib_path.join("pyproject.toml");
            if !pyproject.exists() {
                let content = format!(
                    "[tool.arqon.relationships]\nparent = \"{}\"\nchildren = []\n",
                    master_name
                );
                fs::write(pyproject, content).into_diagnostic()?;
            }

            self.upsert_ago(agorg.id, sib_name, &sib_path, Some(master_name), &[])
                .await?;
        }

        Ok(agorg)
    }

    async fn ensure_database_exists(&self) -> Result<()> {
        if self.dsn_override.is_none() {
            self.db.ensure_running().await?;
        }
        let mut cfg: tokio_postgres::Config = self
            .dsn()
            .parse()
            .into_diagnostic()
            .with_context(|| format!("Invalid PILOT_AGORG_DATABASE_URL: {}", self.dsn()))?;
        let db_name = cfg
            .get_dbname()
            .ok_or_else(|| miette!("Database name missing in DSN"))?
            .to_string();
        validate_db_identifier(&db_name)?;

        cfg.dbname("postgres");
        let (client, connection) = cfg.connect(NoTls).await.into_diagnostic()?;
        tokio::spawn(async move {
            let _ = connection.await;
        });

        let exists = client
            .query_opt("SELECT 1 FROM pg_database WHERE datname = $1", &[&db_name])
            .await
            .into_diagnostic()?
            .is_some();
        if !exists {
            let create = format!("CREATE DATABASE \"{}\"", db_name);
            client.execute(&create, &[]).await.into_diagnostic()?;
        }
        Ok(())
    }

    async fn connect(&self) -> Result<Client> {
        let cfg: tokio_postgres::Config = self
            .dsn()
            .parse()
            .into_diagnostic()
            .with_context(|| format!("Invalid DSN '{}'", self.dsn()))?;
        let (client, connection) = cfg.connect(NoTls).await.into_diagnostic()?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(client)
    }

    async fn ensure_schema(&self, client: &Client) -> Result<()> {
        client
            .batch_execute(
                "
                CREATE TABLE IF NOT EXISTS pilot_identity (
                  key TEXT PRIMARY KEY,
                  value TEXT NOT NULL,
                  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                CREATE TABLE IF NOT EXISTS agorg_schema_versions (
                  version INT PRIMARY KEY,
                  applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                CREATE TABLE IF NOT EXISTS agorgs (
                  id UUID PRIMARY KEY,
                  name TEXT NOT NULL,
                  root_path TEXT NOT NULL UNIQUE,
                  master_path TEXT NULL,
                  parent_agorg_id UUID NULL,
                  default_scope BOOLEAN NOT NULL DEFAULT FALSE,
                  scan_depth INT NOT NULL DEFAULT 4,
                  discovery_method TEXT NULL,
                  settings JSONB NOT NULL DEFAULT '{}'::jsonb,
                  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                CREATE TABLE IF NOT EXISTS agos (
                  id UUID PRIMARY KEY,
                  agorg_id UUID NOT NULL REFERENCES agorgs(id) ON DELETE CASCADE,
                  name TEXT NOT NULL,
                  repo_path TEXT NOT NULL,
                  relationship_parent TEXT NULL,
                  relationship_children JSONB NOT NULL DEFAULT '[]'::jsonb,
                  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                  UNIQUE (agorg_id, repo_path)
                );

                CREATE TABLE IF NOT EXISTS agorg_links (
                  parent_agorg_id UUID NOT NULL REFERENCES agorgs(id) ON DELETE CASCADE,
                  child_agorg_id UUID NOT NULL REFERENCES agorgs(id) ON DELETE CASCADE,
                  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                  PRIMARY KEY (parent_agorg_id, child_agorg_id),
                  CHECK (parent_agorg_id <> child_agorg_id)
                );

                CREATE TABLE IF NOT EXISTS app_state (
                  key TEXT PRIMARY KEY,
                  value JSONB NOT NULL
                );
                ",
            )
            .await
            .into_diagnostic()?;
        let row = client
            .query_opt("SELECT value FROM pilot_identity WHERE key = 'system'", &[])
            .await
            .into_diagnostic()?;
        match row {
            Some(r) => {
                let value: String = r.get(0);
                if value != "arqon_pilot" {
                    return Err(miette!(
                        "Refusing schema migration: connected DB identity '{}' != 'arqon_pilot'",
                        value
                    ));
                }
            }
            None => {
                client
                    .execute(
                        "INSERT INTO pilot_identity (key, value) VALUES ('system', 'arqon_pilot')
                         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
                        &[],
                    )
                    .await
                    .into_diagnostic()?;
            }
        }
        client
            .execute(
                "INSERT INTO agorg_schema_versions (version) VALUES (1) ON CONFLICT (version) DO NOTHING",
                &[],
            )
            .await
            .into_diagnostic()?;

        // Perform schema migrations here since IF NOT EXISTS doesn't update columns
        client
            .batch_execute(
                "
                DO $$
                BEGIN
                    IF NOT EXISTS (
                        SELECT 1
                        FROM information_schema.columns
                        WHERE table_name='agorgs' AND column_name='master_path'
                    ) THEN
                        ALTER TABLE agorgs ADD COLUMN master_path TEXT NULL;
                    END IF;
                END
                $$;
                ",
            )
            .await
            .into_diagnostic()?;

        Ok(())
    }
}

pub fn discover_hierarchy(root: &Path, depth: usize) -> Result<DiscoverResult> {
    let root = canonicalize_or_input(root);
    let root_path = PathBuf::from(&root);
    let root_name = root_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("AGOrg")
        .to_string();
    let mut repos: Vec<(String, PathBuf, Option<RepoRelationships>)> = Vec::new();
    // Flat-fleet default: discover top-level repositories and AGOrg roots only.
    // Nested repository discovery must be explicitly enabled.
    let allow_nested_repos = std::env::var("PILOT_AGORG_ALLOW_NESTED_REPOS")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);
    walk_dirs(&root_path, depth, &mut repos, allow_nested_repos)?;

    let mut parent_refs: HashMap<String, usize> = HashMap::new();
    for (_, _, rel) in &repos {
        if let Some(r) = rel {
            if let Some(p) = &r.parent {
                *parent_refs.entry(p.to_string()).or_insert(0) += 1;
            }
        }
    }

    let mut candidates = Vec::new();
    candidates.push(DiscoverCandidate {
        name: root_name,
        path: root.clone(),
        kind: "agorg".to_string(),
        parent_hint: None,
        children_hints: Vec::new(),
        is_registered: false,
    });

    for (name, path, rel) in repos {
        let mut kind = "ago".to_string();
        let mut parent_hint = None;
        let mut children = Vec::new();
        if let Some(r) = rel {
            parent_hint = r.parent;
            children = r.children;
            if !children.is_empty() || parent_refs.contains_key(&name) {
                kind = "agorg".to_string();
            }
        }
        candidates.push(DiscoverCandidate {
            name,
            path: path.display().to_string(),
            kind,
            parent_hint,
            children_hints: children,
            is_registered: false,
        });
    }

    Ok(DiscoverResult {
        root,
        depth,
        candidates,
    })
}

fn walk_dirs(
    root: &Path,
    depth: usize,
    repos: &mut Vec<(String, PathBuf, Option<RepoRelationships>)>,
    allow_nested_repos: bool,
) -> Result<()> {
    fn should_skip_dir(name: &str) -> bool {
        name.starts_with('.')
            || name == "target"
            || name == "node_modules"
            || name == "site"
            || name == "archive"
    }

    fn recurse(
        current: &Path,
        max_depth: usize,
        at_depth: usize,
        repos: &mut Vec<(String, PathBuf, Option<RepoRelationships>)>,
        allow_nested_repos: bool,
    ) -> Result<()> {
        if at_depth > max_depth {
            return Ok(());
        }
        let read = match fs::read_dir(current) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        for entry in read {
            let entry = match entry {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if should_skip_dir(name) {
                continue;
            }
            let has_repo_marker =
                path.join(".git").exists() || path.join("pyproject.toml").exists();
            if has_repo_marker {
                if at_depth > 1 && !allow_nested_repos {
                    // Flat-fleet rule: ignore nested repositories by default.
                    continue;
                }
                let rel = parse_relationships(&path).ok().flatten();
                repos.push((name.to_string(), path.clone(), rel));
            } else {
                recurse(&path, max_depth, at_depth + 1, repos, allow_nested_repos)?;
            }
        }
        Ok(())
    }
    recurse(root, depth, 1, repos, allow_nested_repos)
}

pub fn parse_relationships(repo_dir: &Path) -> Result<Option<RepoRelationships>> {
    let path = repo_dir.join("pyproject.toml");
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .into_diagnostic()
        .with_context(|| format!("Failed reading {}", path.display()))?;
    let value = content.parse::<toml::Value>().into_diagnostic()?;
    let rel = value
        .get("tool")
        .and_then(|v| v.get("arqon"))
        .and_then(|v| v.get("relationships"));
    let Some(rel) = rel else {
        return Ok(None);
    };
    let parent = rel
        .get("parent")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let children = rel
        .get("children")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(Some(RepoRelationships { parent, children }))
}

fn row_to_agorg(row: &tokio_postgres::Row) -> Agorg {
    Agorg {
        id: row.get(0),
        name: row.get(1),
        root_path: row.get(2),
        master_path: row.get(3),
        parent_agorg_id: row.get(4),
        default_scope: row.get(5),
        scan_depth: row.get(6),
    }
}

fn row_to_ago(row: &tokio_postgres::Row) -> AgoRecord {
    let children_value: serde_json::Value = row.get(5);
    let children = children_value
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    AgoRecord {
        id: row.get(0),
        agorg_id: row.get(1),
        name: row.get(2),
        repo_path: row.get(3),
        relationship_parent: row.get(4),
        relationship_children: children,
    }
}

fn canonicalize_or_input(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn validate_db_identifier(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(miette!("Database name cannot be empty"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(miette!(
            "Database name '{}' must contain only letters, digits, underscore",
            name
        ));
    }
    Ok(())
}

pub fn scan_master_directory(master_path: &Path) -> Result<Vec<DiscoverCandidate>> {
    let mut candidates = Vec::new();
    let read = fs::read_dir(master_path)
        .into_diagnostic()
        .with_context(|| format!("Failed to read master directory {}", master_path.display()))?;

    for entry in read {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if name.starts_with('.')
            || name == "target"
            || name == "node_modules"
            || name == "site"
            || name == "archive"
        {
            continue;
        }

        let has_repo = path.join(".git").exists() || path.join("pyproject.toml").exists();
        let mut kind = "none".to_string();
        let mut parent_hint = None;
        let mut children_hints = Vec::new();

        if has_repo {
            kind = "ago".to_string();
            if let Ok(Some(rel)) = parse_relationships(&path) {
                parent_hint = rel.parent;
                children_hints = rel.children;
                if !children_hints.is_empty() {
                    kind = "agorg".to_string();
                }
            }
        }

        candidates.push(DiscoverCandidate {
            name,
            path: path.display().to_string(),
            kind,
            parent_hint,
            children_hints,
            is_registered: false, // Updated by caller using DB
        });
    }

    candidates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(candidates)
}

pub fn edit_relationship(
    repo_path: &Path,
    parent: Option<String>,
    children: Vec<String>,
) -> Result<()> {
    let pyproject_path = repo_path.join("pyproject.toml");
    let content = if pyproject_path.exists() {
        fs::read_to_string(&pyproject_path).into_diagnostic()?
    } else {
        format!("[tool.arqon.relationships]\nparent = \"\"\nchildren = []\n")
    };

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .into_diagnostic()?;

    // Ensure tool.arqon.relationships exists
    if doc.get("tool").is_none() {
        doc.insert("tool", toml_edit::Item::Table(toml_edit::Table::new()));
    }
    let tool = doc["tool"].as_table_mut().unwrap();
    if tool.get("arqon").is_none() {
        tool.insert("arqon", toml_edit::Item::Table(toml_edit::Table::new()));
    }
    let arqon = tool["arqon"].as_table_mut().unwrap();
    if arqon.get("relationships").is_none() {
        arqon.insert(
            "relationships",
            toml_edit::Item::Table(toml_edit::Table::new()),
        );
    }
    let rel = arqon["relationships"].as_table_mut().unwrap();

    if let Some(p) = parent {
        rel.insert("parent", toml_edit::value(p));
    } else {
        rel.remove("parent");
    }

    let mut children_arr = toml_edit::Array::new();
    for child in children {
        children_arr.push(child);
    }
    rel.insert("children", toml_edit::value(children_arr));

    fs::write(&pyproject_path, doc.to_string()).into_diagnostic()?;
    Ok(())
}

pub fn upgrade_ago(repo_path: &Path, name: &str) -> Result<()> {
    let pyproject_path = repo_path.join("pyproject.toml");
    if pyproject_path.exists() {
        // Just ensure relationships exist
        edit_relationship(repo_path, None, Vec::new())?;
    } else {
        let content = format!(
            "[project]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[tool.arqon.relationships]\nparent = \"\"\nchildren = []\n",
            name
        );
        fs::write(&pyproject_path, content).into_diagnostic()?;
    }
    Ok(())
}
