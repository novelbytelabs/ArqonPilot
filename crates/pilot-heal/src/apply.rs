use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Apply a fix by replacing the entire file content
///
/// This is the "Whole Block Replacement" strategy per Constitution XVII.
/// In MVP, we replace the entire file. Future versions may do surgical edits.
pub fn apply_fix(file_path: &Path, fix_content: &str) -> Result<()> {
    // Create backup
    let backup_path = file_path.with_extension("rs.bak");
    if file_path.exists() {
        fs::copy(file_path, &backup_path)
            .with_context(|| format!("Failed to create backup at {:?}", backup_path))?;
    }

    // Write fix
    fs::write(file_path, fix_content)
        .with_context(|| format!("Failed to write fix to {:?}", file_path))?;

    Ok(())
}

/// Restore from backup if fix failed
pub fn restore_backup(file_path: &Path) -> Result<()> {
    let backup_path = file_path.with_extension("rs.bak");
    if backup_path.exists() {
        fs::copy(&backup_path, file_path)
            .with_context(|| format!("Failed to restore backup from {:?}", backup_path))?;
        fs::remove_file(&backup_path)?;
    }
    Ok(())
}
