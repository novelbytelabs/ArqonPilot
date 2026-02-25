use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

pub struct VerificationGate {
    root: PathBuf,
}

impl VerificationGate {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Check if the project compiles
    pub fn check_compile(&self) -> Result<bool> {
        let output = Command::new("cargo")
            .arg("build")
            .current_dir(&self.root)
            .output()?;

        Ok(output.status.success())
    }

    /// Check if linting passes (cargo clippy)
    pub fn check_lint(&self) -> Result<bool> {
        let output = Command::new("cargo")
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .current_dir(&self.root)
            .output()?;

        Ok(output.status.success())
    }

    /// Check if tests pass
    pub fn check_tests(&self) -> Result<bool> {
        let output = Command::new("cargo")
            .arg("test")
            .current_dir(&self.root)
            .output()?;

        Ok(output.status.success())
    }
}
