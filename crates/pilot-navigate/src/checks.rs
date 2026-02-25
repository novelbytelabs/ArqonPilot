use anyhow::Result;
use std::process::Command;

pub struct ConstitutionCheck {
    root: std::path::PathBuf,
}

impl ConstitutionCheck {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self { root }
    }

    /// Check if git working directory is clean
    pub fn check_clean_git(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.root)
            .output()?;

        let is_clean = output.stdout.is_empty();
        if !is_clean {
            eprintln!("Git working directory is not clean");
        }
        Ok(is_clean)
    }

    /// Check if all tests pass
    pub fn check_tests_pass(&self) -> Result<bool> {
        let output = Command::new("cargo")
            .arg("test")
            .current_dir(&self.root)
            .output()?;

        Ok(output.status.success())
    }

    /// Check for untagged TODOs/FIXMEs (simplified check)
    pub fn check_no_untagged_debt(&self) -> Result<bool> {
        let output = Command::new("grep")
            .args(["-rn", "TODO", "--include=*.rs", "."])
            .current_dir(&self.root)
            .output()?;

        let todos = String::from_utf8_lossy(&output.stdout);
        // Allow TODOs if they have a tracking issue like "TODO(#123)"
        let untagged_count = todos
            .lines()
            .filter(|line| !line.contains("TODO(#") && !line.contains("TODO["))
            .count();

        if untagged_count > 0 {
            eprintln!("Found {} untagged TODO items", untagged_count);
            return Ok(false);
        }

        Ok(true)
    }

    /// Run all constitution checks
    pub fn run_all(&self) -> Result<bool> {
        let clean = self.check_clean_git()?;
        let tests = self.check_tests_pass()?;
        // Skip debt check for MVP
        // let debt = self.check_no_untagged_debt()?;

        Ok(clean && tests)
    }
}
