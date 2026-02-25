use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAction {
    pub path: PathBuf,
    pub created: bool,
    pub message: String,
}

pub fn scaffold_feature(root: &Path, name: &str, dry_run: bool) -> Result<Vec<CreateAction>> {
    let mut actions = Vec::new();
    let module = sanitize(name);
    if module.is_empty() {
        return Err(anyhow!("Feature name is empty after sanitization"));
    }

    let src = root.join("src").join(format!("{module}.rs"));
    let test = root.join("tests").join(format!("{module}_test.rs"));

    actions.push(write_file(
        &src,
        &format!("pub fn {module}_feature() -> &'static str {{\n    \"{module}\"\n}}\n"),
        dry_run,
    )?);
    actions.push(write_file(
        &test,
        &format!(
            "#[test]\nfn test_{module}_feature_smoke() {{\n    assert_eq!(crate::{module}::{module}_feature(), \"{module}\");\n}}\n"
        ),
        dry_run,
    )?);

    Ok(actions)
}

pub fn scaffold_tests(root: &Path, target: &str, dry_run: bool) -> Result<CreateAction> {
    let module = sanitize(target);
    if module.is_empty() {
        return Err(anyhow!("Target name is empty after sanitization"));
    }
    let path = root
        .join("tests")
        .join(format!("{module}_generated_test.rs"));
    write_file(
        &path,
        &format!(
            "#[test]\nfn test_{module}_generated() {{\n    // TODO: replace with real assertions\n    assert!(true);\n}}\n"
        ),
        dry_run,
    )
}

fn write_file(path: &Path, content: &str, dry_run: bool) -> Result<CreateAction> {
    if path.exists() {
        return Ok(CreateAction {
            path: path.to_path_buf(),
            created: false,
            message: "File already exists, skipped".to_string(),
        });
    }

    if dry_run {
        return Ok(CreateAction {
            path: path.to_path_buf(),
            created: false,
            message: "[DRY RUN] Would create scaffold file".to_string(),
        });
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(CreateAction {
        path: path.to_path_buf(),
        created: true,
        message: "Created".to_string(),
    })
}

fn sanitize(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_lowercase()
}
