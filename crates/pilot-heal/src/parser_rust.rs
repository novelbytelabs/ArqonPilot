use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub file_path: String,
    pub line: Option<u32>,
    pub error_message: String,
    pub test_name: String,
}

#[derive(Debug, Deserialize)]
struct CargoMessage {
    reason: String,
    #[serde(default)]
    message: Option<CompilerMessage>,
}

#[derive(Debug, Deserialize)]
struct CompilerMessage {
    message: String,
    code: Option<DiagnosticCode>,
    level: String,
    spans: Vec<Span>,
}

#[derive(Debug, Deserialize)]
struct DiagnosticCode {
    code: String,
}

#[derive(Debug, Deserialize)]
struct Span {
    file_name: String,
    line_start: u32,
    line_end: u32,
}

pub struct RustLogParser;

impl RustLogParser {
    pub fn parse_file(path: &std::path::Path) -> Result<Option<TestFailure>> {
        let content = std::fs::read_to_string(path)?;
        let failures = Self::parse_cargo_output(&content)?;
        Ok(failures.into_iter().next())
    }

    pub fn parse_cargo_output(json_output: &str) -> Result<Vec<TestFailure>> {
        let mut failures = Vec::new();

        for line in json_output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(msg) = serde_json::from_str::<CargoMessage>(line) {
                if msg.reason == "compiler-message" {
                    if let Some(compiler_msg) = msg.message {
                        if compiler_msg.level == "error" {
                            for span in &compiler_msg.spans {
                                failures.push(TestFailure {
                                    file_path: span.file_name.clone(),
                                    line: Some(span.line_start),
                                    error_message: compiler_msg.message.clone(),
                                    test_name: String::new(), // Not a test, a compile error
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(failures)
    }
}
