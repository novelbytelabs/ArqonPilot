use super::model::{
    AgorgPolicyRecord, InheritanceStep, PolicyConflictTrace, PolicyException, PolicyOverrideRecord,
};
use chrono::{DateTime, Utc};
use miette::{IntoDiagnostic, Result};
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

pub struct GovernanceStore {
    dsn: String,
}

impl GovernanceStore {
    pub fn new(dsn: String) -> Self {
        Self { dsn }
    }

    pub async fn connect(&self) -> Result<Client> {
        use miette::Context;
        let cfg: tokio_postgres::Config = self
            .dsn
            .parse()
            .into_diagnostic()
            .with_context(|| format!("Invalid DSN '{}'", self.dsn))?;
        let (client, connection) = cfg.connect(NoTls).await.into_diagnostic()?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(client)
    }

    pub async fn get_policy(
        &self,
        agorg_id: Uuid,
        policy_kind: &str,
    ) -> Result<Option<AgorgPolicyRecord>> {
        let client = self.connect().await?;
        let row = client
            .query_opt(
                "SELECT id, agorg_id, ago_path, policy_kind, version, policy_json, status, updated_at, updated_by
                 FROM agorg_policies
                 WHERE agorg_id = $1 AND policy_kind = $2 AND ago_path IS NULL
                 ORDER BY version DESC LIMIT 1",
                &[&agorg_id, &policy_kind],
            )
            .await
            .into_diagnostic()?;

        match row {
            Some(r) => {
                let id: Uuid = r.get(0);
                let agorg_id: Uuid = r.get(1);
                let ago_path: Option<String> = r.get(2);
                let policy_kind: String = r.get(3);
                let version: i32 = r.get(4);
                let policy_json: serde_json::Value = r.get(5);
                let status: String = r.get(6);
                let updated_at: DateTime<Utc> = r.get(7);
                let updated_by: String = r.get(8);

                Ok(Some(AgorgPolicyRecord {
                    id,
                    agorg_id,
                    ago_path,
                    policy_kind,
                    version,
                    policy_json,
                    status,
                    updated_at,
                    updated_by,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_ago_policy_override(
        &self,
        agorg_id: Uuid,
        ago_path: &str,
        policy_kind: &str,
    ) -> Result<Option<AgorgPolicyRecord>> {
        let client = self.connect().await?;
        let row = client
            .query_opt(
                "SELECT id, agorg_id, ago_path, policy_kind, version, policy_json, status, updated_at, updated_by
                 FROM agorg_policies
                 WHERE agorg_id = $1 AND policy_kind = $2 AND ago_path = $3
                 ORDER BY version DESC LIMIT 1",
                &[&agorg_id, &policy_kind, &ago_path],
            )
            .await
            .into_diagnostic()?;

        match row {
            Some(r) => {
                let id: Uuid = r.get(0);
                let agorg_id: Uuid = r.get(1);
                let ago_path: Option<String> = r.get(2);
                let policy_kind: String = r.get(3);
                let version: i32 = r.get(4);
                let policy_json: serde_json::Value = r.get(5);
                let status: String = r.get(6);
                let updated_at: DateTime<Utc> = r.get(7);
                let updated_by: String = r.get(8);

                Ok(Some(AgorgPolicyRecord {
                    id,
                    agorg_id,
                    ago_path,
                    policy_kind,
                    version,
                    policy_json,
                    status,
                    updated_at,
                    updated_by,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_policy_by_version(
        &self,
        agorg_id: Uuid,
        ago_path: Option<&str>,
        policy_kind: &str,
        version: i32,
    ) -> Result<Option<AgorgPolicyRecord>> {
        let client = self.connect().await?;
        let row = client
            .query_opt(
                "SELECT id, agorg_id, ago_path, policy_kind, version, policy_json, status, updated_at, updated_by
                 FROM agorg_policies
                 WHERE agorg_id = $1
                   AND policy_kind = $2
                   AND ago_path IS NOT DISTINCT FROM $3
                   AND version = $4
                 LIMIT 1",
                &[&agorg_id, &policy_kind, &ago_path, &version],
            )
            .await
            .into_diagnostic()?;

        match row {
            Some(r) => Ok(Some(AgorgPolicyRecord {
                id: r.get(0),
                agorg_id: r.get(1),
                ago_path: r.get(2),
                policy_kind: r.get(3),
                version: r.get(4),
                policy_json: r.get(5),
                status: r.get(6),
                updated_at: r.get(7),
                updated_by: r.get(8),
            })),
            None => Ok(None),
        }
    }

    pub async fn update_policy_status(
        &self,
        policy_id: Uuid,
        status: &str,
        operator: &str,
    ) -> Result<Option<AgorgPolicyRecord>> {
        let client = self.connect().await?;
        let row = client
            .query_opt(
                "UPDATE agorg_policies
                 SET status = $2, updated_by = $3, updated_at = NOW()
                 WHERE id = $1
                 RETURNING id, agorg_id, ago_path, policy_kind, version, policy_json, status, updated_at, updated_by",
                &[&policy_id, &status, &operator],
            )
            .await
            .into_diagnostic()?;

        match row {
            Some(r) => Ok(Some(AgorgPolicyRecord {
                id: r.get(0),
                agorg_id: r.get(1),
                ago_path: r.get(2),
                policy_kind: r.get(3),
                version: r.get(4),
                policy_json: r.get(5),
                status: r.get(6),
                updated_at: r.get(7),
                updated_by: r.get(8),
            })),
            None => Ok(None),
        }
    }

    pub async fn save_policy(
        &self,
        agorg_id: Uuid,
        ago_path: Option<&str>,
        policy_kind: &str,
        policy_json: &serde_json::Value,
        status: &str,
        operator: &str,
    ) -> Result<AgorgPolicyRecord> {
        let client = self.connect().await?;
        let current_version: i32 = match client
            .query_opt(
                "SELECT version FROM agorg_policies WHERE agorg_id = $1 AND policy_kind = $2 AND ago_path IS NOT DISTINCT FROM $3 ORDER BY version DESC LIMIT 1",
                &[&agorg_id, &policy_kind, &ago_path],
            )
            .await
            .into_diagnostic()?
        {
            Some(row) => row.get(0),
            None => 0,
        };

        let next_version = current_version + 1;
        let id = Uuid::new_v4();

        client.execute(
            "INSERT INTO agorg_policies (id, agorg_id, ago_path, policy_kind, version, policy_json, status, updated_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[&id, &agorg_id, &ago_path, &policy_kind, &next_version, &policy_json, &status, &operator]
        ).await.into_diagnostic()?;

        Ok(AgorgPolicyRecord {
            id,
            agorg_id,
            ago_path: ago_path.map(|s| s.to_string()),
            policy_kind: policy_kind.to_string(),
            version: next_version,
            policy_json: policy_json.clone(),
            status: status.to_string(),
            updated_at: Utc::now(),
            updated_by: operator.to_string(),
        })
    }

    pub async fn list_overrides(
        &self,
        agorg_id: Uuid,
        policy_kind: &str,
    ) -> Result<Vec<PolicyOverrideRecord>> {
        let client = self.connect().await?;
        let rows = client
            .query(
                "SELECT id, agorg_id, ago_path, policy_kind, reason, ticket_ref, owner, created_at, expires_at, parent_policy_version, override_policy_version
                 FROM agorg_policy_overrides
                 WHERE agorg_id = $1 AND policy_kind = $2 AND revoked_at IS NULL
                 ORDER BY created_at DESC",
                &[&agorg_id, &policy_kind],
            )
            .await
            .into_diagnostic()?;

        let mut overrides = Vec::new();
        for r in rows {
            overrides.push(PolicyOverrideRecord {
                id: r.get(0),
                agorg_id: r.get(1),
                ago_path: r.get(2),
                policy_kind: r.get(3),
                reason: r.get(4),
                ticket_ref: r.get(5),
                owner: r.get(6),
                created_at: r.get(7),
                expires_at: r.get(8),
                parent_policy_version: r.get(9),
                override_policy_version: r.get(10),
            });
        }
        Ok(overrides)
    }

    pub async fn create_override_with_reason(
        &self,
        agorg_id: Uuid,
        ago_path: &str,
        policy_kind: &str,
        policy_json: &serde_json::Value,
        status: &str,
        operator: &str,
        reason: &str,
        ticket_ref: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<AgorgPolicyRecord> {
        let parent_policy = self.get_policy(agorg_id, policy_kind).await?;
        let parent_version = parent_policy.map(|p| p.version).unwrap_or(0);

        let saved = self
            .save_policy(
                agorg_id,
                Some(ago_path),
                policy_kind,
                policy_json,
                status,
                operator,
            )
            .await?;

        let client = self.connect().await?;
        client
            .execute(
                "INSERT INTO agorg_policy_overrides (agorg_id, ago_path, policy_kind, reason, ticket_ref, owner, expires_at, parent_policy_version, override_policy_version)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[
                    &agorg_id,
                    &ago_path,
                    &policy_kind,
                    &reason,
                    &ticket_ref,
                    &operator,
                    &expires_at,
                    &parent_version,
                    &saved.version,
                ],
            )
            .await
            .into_diagnostic()?;
        Ok(saved)
    }

    pub async fn revoke_override(
        &self,
        agorg_id: Uuid,
        ago_path: &str,
        policy_kind: &str,
    ) -> Result<()> {
        let mut client = self.connect().await?;
        let tx = client.transaction().await.into_diagnostic()?;
        tx.execute(
            "DELETE FROM agorg_policies WHERE agorg_id = $1 AND policy_kind = $2 AND ago_path = $3",
            &[&agorg_id, &policy_kind, &ago_path],
        )
        .await
        .into_diagnostic()?;
        tx.execute(
            "UPDATE agorg_policy_overrides SET revoked_at = NOW() WHERE agorg_id = $1 AND policy_kind = $2 AND ago_path = $3 AND revoked_at IS NULL",
            &[&agorg_id, &policy_kind, &ago_path],
        )
        .await
        .into_diagnostic()?;
        tx.commit().await.into_diagnostic()?;
        Ok(())
    }

    pub async fn get_agorg_name(&self, agorg_id: Uuid) -> Result<Option<String>> {
        let client = self.connect().await?;
        let row = client
            .query_opt("SELECT name FROM agorgs WHERE id = $1", &[&agorg_id])
            .await
            .into_diagnostic()?;
        Ok(row.map(|r| r.get(0)))
    }

    pub async fn get_effective_policy_record(
        &self,
        agorg_id: Uuid,
        ago_path: &str,
        policy_kind: &str,
    ) -> Result<Option<(AgorgPolicyRecord, String)>> {
        let client = self.connect().await?;

        // Recursive query to find the nearest policy (override or general) in the AGOrg hierarchy
        let row = client
            .query_opt(
                "WITH RECURSIVE hierarchy(agorg_id, parent_agorg_id, depth) AS (
                    SELECT id, parent_agorg_id, 0 FROM agorgs WHERE id = $1
                    UNION ALL
                    SELECT a.id, a.parent_agorg_id, h.depth + 1
                    FROM agorgs a
                    JOIN hierarchy h ON a.id = h.parent_agorg_id
                    WHERE h.depth < 10 -- Safety cap for recursion
                )
                SELECT p.id, p.agorg_id, p.ago_path, p.policy_kind, p.version, p.policy_json, p.status, p.updated_at, p.updated_by, a.name as source_name
                FROM hierarchy h
                JOIN agorgs a ON h.agorg_id = a.id
                CROSS JOIN LATERAL (
                    (SELECT id, agorg_id, ago_path, policy_kind, version, policy_json, status, updated_at, updated_by
                     FROM agorg_policies
                     WHERE agorg_id = h.agorg_id AND policy_kind = $2 AND ago_path = $3 AND status = 'active'
                     ORDER BY version DESC LIMIT 1)
                    UNION ALL
                    (SELECT id, agorg_id, ago_path, policy_kind, version, policy_json, status, updated_at, updated_by
                     FROM agorg_policies
                     WHERE agorg_id = h.agorg_id AND policy_kind = $2 AND ago_path IS NULL AND status = 'active'
                     ORDER BY version DESC LIMIT 1)
                ) p
                ORDER BY h.depth ASC, (p.ago_path IS NULL) ASC
                LIMIT 1",
                &[&agorg_id, &policy_kind, &ago_path],
            )
            .await
            .into_diagnostic()?;

        match row {
            Some(r) => {
                let rec = AgorgPolicyRecord {
                    id: r.get(0),
                    agorg_id: r.get(1),
                    ago_path: r.get(2),
                    policy_kind: r.get(3),
                    version: r.get(4),
                    policy_json: r.get(5),
                    status: r.get(6),
                    updated_at: r.get(7),
                    updated_by: r.get(8),
                };
                let source_name: String = r.get(9);
                Ok(Some((rec, source_name)))
            }
            None => Ok(None),
        }
    }

    pub async fn resolve_with_trace(
        &self,
        agorg_id: Uuid,
        ago_path: &str,
        policy_kind: &str,
    ) -> Result<PolicyConflictTrace> {
        let client = self.connect().await?;

        // 1. Get the hierarchy walk and check if each level has an override or a fleet policy.
        let rows = client
            .query(
                "WITH RECURSIVE hierarchy(agorg_id, parent_agorg_id, depth) AS (
                    SELECT id, parent_agorg_id, 0 FROM agorgs WHERE id = $1
                    UNION ALL
                    SELECT a.id, a.parent_agorg_id, h.depth + 1
                    FROM agorgs a
                    JOIN hierarchy h ON a.id = h.parent_agorg_id
                    WHERE h.depth < 10
                )
                SELECT h.agorg_id, a.name, h.depth,
                       EXISTS(SELECT 1 FROM agorg_policies WHERE agorg_id = h.agorg_id AND policy_kind = $2 AND ago_path = $3 AND status = 'active') as has_override,
                       EXISTS(SELECT 1 FROM agorg_policies WHERE agorg_id = h.agorg_id AND policy_kind = $2 AND ago_path IS NULL AND status = 'active') as has_fleet_policy
                FROM hierarchy h
                JOIN agorgs a ON h.agorg_id = a.id
                ORDER BY h.depth ASC",
                &[&agorg_id, &policy_kind, &ago_path],
            )
            .await
            .into_diagnostic()?;

        let mut chain = Vec::new();
        let mut winner_index = None;
        let mut winner_source = "fallback".to_string();

        for (i, r) in rows.iter().enumerate() {
            let step_agorg_id: Uuid = r.get(0);
            let step_name: String = r.get(1);
            let depth: i32 = r.get(2);
            let has_override: bool = r.get(3);
            let has_fleet_policy: bool = r.get(4);

            chain.push(InheritanceStep {
                agorg_id: step_agorg_id,
                agorg_name: step_name,
                depth,
                has_override,
                has_fleet_policy,
                is_winner: false,
            });

            // The first level (starting from depth 0) that has either an override or fleet policy wins.
            if winner_index.is_none() {
                if has_override {
                    winner_index = Some(i);
                    winner_source = "ago_override".to_string();
                } else if has_fleet_policy {
                    winner_index = Some(i);
                    winner_source = if depth == 0 {
                        "agorg_active".to_string()
                    } else {
                        "parent_agorg".to_string()
                    };
                }
            }
        }

        let mut resolved_agorg_id = Uuid::nil();
        let mut resolved_version = 0;

        if let Some(idx) = winner_index {
            chain[idx].is_winner = true;
            resolved_agorg_id = chain[idx].agorg_id;

            // Fetch the winning version
            let is_override = winner_source == "ago_override";
            let path_param = if is_override { Some(ago_path) } else { None };
            let policy_row = client.query_opt(
                "SELECT version FROM agorg_policies WHERE agorg_id = $1 AND policy_kind = $2 AND ago_path IS NOT DISTINCT FROM $3 AND status = 'active' ORDER BY version DESC LIMIT 1",
                &[&resolved_agorg_id, &policy_kind, &path_param]
            ).await.into_diagnostic()?;

            if let Some(p) = policy_row {
                resolved_version = p.get(0);
            }
        }

        Ok(PolicyConflictTrace {
            ago_path: ago_path.to_string(),
            policy_kind: policy_kind.to_string(),
            resolved_source: winner_source,
            resolved_agorg_id,
            resolved_version,
            chain,
        })
    }

    pub async fn get_exceptions(
        &self,
        agorg_id: Uuid,
        policy_kind: &str,
    ) -> Result<Vec<PolicyException>> {
        let client = self.connect().await?;
        let rows = client
            .query(
                "SELECT id, agorg_id, ago_path, policy_kind, rule_path, reason, ticket_ref, owner, expires_at, created_at
                 FROM policy_exceptions
                 WHERE agorg_id = $1 AND policy_kind = $2",
                &[&agorg_id, &policy_kind],
            )
            .await
            .into_diagnostic()?;

        let mut exceptions = Vec::new();
        for r in rows {
            exceptions.push(PolicyException {
                id: r.get(0),
                agorg_id: r.get(1),
                ago_path: r.get(2),
                policy_kind: r.get(3),
                rule_path: r.get(4),
                reason: r.get(5),
                ticket_ref: r.get(6),
                owner: r.get(7),
                expires_at: r.get(8),
                created_at: r.get(9),
            });
        }
        Ok(exceptions)
    }

    pub async fn get_effective_exceptions(
        &self,
        agorg_id: Uuid,
        policy_kind: &str,
    ) -> Result<Vec<PolicyException>> {
        let client = self.connect().await?;
        // Recursive query to find all relevant AGOrgs in the parent chain
        let rows = client
            .query(
                "WITH RECURSIVE hierarchy(agorg_id, parent_agorg_id, depth) AS (
                    SELECT id, parent_agorg_id, 0 FROM agorgs WHERE id = $1
                    UNION ALL
                    SELECT a.id, a.parent_agorg_id, h.depth + 1
                    FROM agorgs a
                    JOIN hierarchy h ON a.id = h.parent_agorg_id
                    WHERE h.depth < 10
                )
                SELECT e.id, e.agorg_id, e.ago_path, e.policy_kind, e.rule_path, e.reason, e.ticket_ref, e.owner, e.expires_at, e.created_at
                FROM policy_exceptions e
                JOIN hierarchy h ON e.agorg_id = h.agorg_id
                WHERE e.policy_kind = $2",
                &[&agorg_id, &policy_kind],
            )
            .await
            .into_diagnostic()?;

        let mut exceptions = Vec::new();
        for r in rows {
            exceptions.push(PolicyException {
                id: r.get(0),
                agorg_id: r.get(1),
                ago_path: r.get(2),
                policy_kind: r.get(3),
                rule_path: r.get(4),
                reason: r.get(5),
                ticket_ref: r.get(6),
                owner: r.get(7),
                expires_at: r.get(8),
                created_at: r.get(9),
            });
        }
        Ok(exceptions)
    }

    pub async fn add_exception(&self, exception: PolicyException) -> Result<()> {
        let mut client = self.connect().await?;
        let tx = client.transaction().await.into_diagnostic()?;
        let updated = tx
            .execute(
                "UPDATE policy_exceptions
                 SET reason = $6, ticket_ref = $7, owner = $8, expires_at = $9
                 WHERE agorg_id = $2
                   AND ago_path IS NOT DISTINCT FROM $3
                   AND policy_kind = $4
                   AND rule_path = $5",
                &[
                    &exception.id,
                    &exception.agorg_id,
                    &exception.ago_path,
                    &exception.policy_kind,
                    &exception.rule_path,
                    &exception.reason,
                    &exception.ticket_ref,
                    &exception.owner,
                    &exception.expires_at,
                ],
            )
            .await
            .into_diagnostic()?;
        if updated == 0 {
            tx.execute(
                "INSERT INTO policy_exceptions (id, agorg_id, ago_path, policy_kind, rule_path, reason, ticket_ref, owner, expires_at, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                &[
                    &exception.id,
                    &exception.agorg_id,
                    &exception.ago_path,
                    &exception.policy_kind,
                    &exception.rule_path,
                    &exception.reason,
                    &exception.ticket_ref,
                    &exception.owner,
                    &exception.expires_at,
                    &exception.created_at,
                ],
            )
            .await
            .into_diagnostic()?;
        }
        tx.commit().await.into_diagnostic()?;
        Ok(())
    }

    pub async fn delete_exception(&self, exception_id: Uuid) -> Result<()> {
        let client = self.connect().await?;
        client
            .execute(
                "DELETE FROM policy_exceptions WHERE id = $1",
                &[&exception_id],
            )
            .await
            .into_diagnostic()?;
        Ok(())
    }

    pub async fn record_decision(
        &self,
        agorg_id: Uuid,
        ago_path: String,
        policy_kind: &str,
        action: &str,
        result_status: &str,
        decision_json: &serde_json::Value,
    ) -> Result<()> {
        let client = self.connect().await?;
        let decision_id = Uuid::new_v4();
        client
            .execute(
                "INSERT INTO policy_decisions (decision_id, agorg_id, ago_path, policy_kind, action, result, decision_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                 &[&decision_id, &agorg_id, &ago_path, &policy_kind, &action, &result_status, &decision_json]
            )
            .await
            .into_diagnostic()?;
        Ok(())
    }

    pub async fn get_decisions(
        &self,
        agorg_id: Uuid,
        policy_kind: &str,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let client = self.connect().await?;
        let rows = client
            .query(
                "SELECT decision_json FROM policy_decisions
                 WHERE agorg_id = $1 AND policy_kind = $2
                 ORDER BY created_at DESC LIMIT $3",
                &[&agorg_id, &policy_kind, &(limit as i64)],
            )
            .await
            .into_diagnostic()?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.get(0));
        }
        Ok(results)
    }

    pub async fn get_idempotency_response(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<serde_json::Value>> {
        let client = self.connect().await?;
        let row = client
            .query_opt(
                "SELECT response_json FROM policy_idempotency WHERE idempotency_key = $1",
                &[&idempotency_key],
            )
            .await
            .into_diagnostic()?;
        Ok(row.map(|r| r.get(0)))
    }

    pub async fn save_idempotency_response(
        &self,
        idempotency_key: &str,
        response_json: &serde_json::Value,
    ) -> Result<()> {
        let client = self.connect().await?;
        client
            .execute(
                "INSERT INTO policy_idempotency (idempotency_key, response_json)
                 VALUES ($1, $2)
                 ON CONFLICT (idempotency_key) DO UPDATE SET response_json = EXCLUDED.response_json",
                &[&idempotency_key, &response_json],
            )
            .await
            .into_diagnostic()?;
        Ok(())
    }
}
