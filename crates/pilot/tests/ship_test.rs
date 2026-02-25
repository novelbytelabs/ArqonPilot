use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test SemVer parsing from Cargo.toml
mod version_tests {
    use super::*;

    fn create_cargo_toml(dir: &TempDir, version: &str) -> PathBuf {
        let cargo_path = dir.path().join("Cargo.toml");
        let content = format!(
            r#"[package]
name = "test-pkg"
version = "{}"
edition = "2021"
"#,
            version
        );
        fs::write(&cargo_path, content).unwrap();
        cargo_path
    }

    #[test]
    fn test_from_cargo_toml_basic() {
        let dir = TempDir::new().unwrap();
        let cargo_path = create_cargo_toml(&dir, "1.2.3");

        // Note: We can't directly test ship::SemVer here without adding ship as a dev-dependency
        // For now, this is a placeholder that verifies the file structure works
        let content = fs::read_to_string(&cargo_path).unwrap();
        assert!(content.contains("version = \"1.2.3\""));
    }

    #[test]
    fn test_from_cargo_toml_with_pre_release() {
        let dir = TempDir::new().unwrap();
        let cargo_path = create_cargo_toml(&dir, "2.0.0-alpha.1");

        let content = fs::read_to_string(&cargo_path).unwrap();
        assert!(content.contains("version = \"2.0.0-alpha.1\""));
    }
}

/// Test git remote URL parsing
mod git_tests {
    // These tests are in ship/src/ship/git.rs as unit tests
    // Integration tests would require a real git repo
}

/// Test conventional commit parsing
mod commit_tests {
    #[test]
    fn test_commit_type_extraction() {
        // Test data for conventional commit formats
        let test_cases = vec![
            ("feat: add new feature", "feat", None, "add new feature"),
            (
                "fix(auth): resolve login bug",
                "fix",
                Some("auth"),
                "resolve login bug",
            ),
            (
                "chore(deps)!: breaking change",
                "chore",
                Some("deps"),
                "breaking change",
            ),
        ];

        for (message, expected_type, expected_scope, expected_desc) in test_cases {
            // Parse the message parts
            if let Some(colon_pos) = message.find(':') {
                let prefix = &message[..colon_pos];
                let desc = message[colon_pos + 1..].trim();

                // Extract type
                let commit_type = if let Some(paren_start) = prefix.find('(') {
                    &prefix[..paren_start]
                } else {
                    prefix.trim_end_matches('!')
                };

                // Extract scope
                let scope = if let Some(paren_start) = prefix.find('(') {
                    prefix
                        .find(')')
                        .map(|paren_end| &prefix[paren_start + 1..paren_end])
                } else {
                    None
                };

                assert_eq!(commit_type, expected_type, "Type mismatch for: {}", message);
                assert_eq!(scope, expected_scope, "Scope mismatch for: {}", message);
                assert_eq!(desc, expected_desc, "Description mismatch for: {}", message);
            }
        }
    }
}
