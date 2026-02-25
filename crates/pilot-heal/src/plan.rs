use crate::parser_rust::TestFailure;
use anyhow::Result;
use pilot_oracle::OracleStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiFileRepairPlan {
    pub primary_file: String,
    pub candidate_files: Vec<String>,
    pub related_signatures: Vec<String>,
    pub notes: Vec<String>,
}

pub fn build_multifile_repair_plan(
    root: &Path,
    store: &OracleStore,
    failure: &TestFailure,
    max_files: usize,
) -> Result<MultiFileRepairPlan> {
    let mut candidates = BTreeSet::new();
    candidates.insert(failure.file_path.clone());

    let failure_path = PathBuf::from(&failure.file_path);
    if let Some(parent) = failure_path.parent() {
        let dir = root.join(parent);
        if dir.is_dir() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                if p.extension().and_then(|s| s.to_str())
                    != failure_path.extension().and_then(|s| s.to_str())
                {
                    continue;
                }
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .to_string();
                candidates.insert(rel);
            }
        }
    }

    if let Some(stem) = failure_path.file_stem().and_then(|s| s.to_str()) {
        let conventional = [
            format!("tests/{}.rs", stem),
            format!("tests/test_{}.py", stem),
            format!("src/{}_test.rs", stem),
            format!("src/test_{}.py", stem),
        ];
        for rel in conventional {
            if root.join(&rel).exists() {
                candidates.insert(rel);
            }
        }
    }

    let related_signatures = store.get_related_signatures(&failure.file_path, failure.line);
    let mut candidate_files: Vec<String> = candidates.into_iter().collect();
    candidate_files.truncate(max_files.max(1));

    let mut notes = vec![
        "Primary failing file included by default".to_string(),
        "Sibling source files added from same directory for cross-file impact".to_string(),
        "Conventional test companions included when present".to_string(),
    ];
    if !related_signatures.is_empty() {
        notes.push("Oracle signatures near failure included for repair context".to_string());
    }

    Ok(MultiFileRepairPlan {
        primary_file: failure.file_path.clone(),
        candidate_files,
        related_signatures,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_multifile_repair_plan_includes_primary() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::create_dir_all(root.join(".pilot")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(root.join("src/util.rs"), "pub fn util() {}").unwrap();
        std::fs::write(root.join("tests/main.rs"), "#[test] fn t(){}").unwrap();

        let store = OracleStore::open(root.join(".pilot/graph.db")).unwrap();
        let failure = TestFailure {
            file_path: "src/main.rs".to_string(),
            line: Some(1),
            error_message: "boom".to_string(),
            test_name: "x".to_string(),
        };

        let plan = build_multifile_repair_plan(root, &store, &failure, 5).unwrap();
        assert!(plan.candidate_files.iter().any(|f| f == "src/main.rs"));
    }
}
