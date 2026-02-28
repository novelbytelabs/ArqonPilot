use pilot_navigate::SemVer;
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
        let version = SemVer::from_cargo_toml(&cargo_path).expect("Failed to parse Cargo.toml");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_from_cargo_toml_workspace_version() {
        let dir = TempDir::new().unwrap();
        let cargo_path = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_path,
            r#"[workspace.package]
version = "2.3.4"
edition = "2021"
"#,
        )
        .unwrap();

        let version =
            SemVer::from_cargo_toml(&cargo_path).expect("Failed to parse workspace version");
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 3);
        assert_eq!(version.patch, 4);
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
