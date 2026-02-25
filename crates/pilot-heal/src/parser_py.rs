use crate::parser_rust::TestFailure;
use anyhow::Result;

pub struct PythonLogParser;

impl PythonLogParser {
    /// Parse pytest plain text output for failures
    pub fn parse_pytest_output(text_output: &str) -> Result<Vec<TestFailure>> {
        let mut failures = Vec::new();
        let mut current_test: Option<String> = None;
        let mut current_error = String::new();
        let mut in_failure = false;

        for line in text_output.lines() {
            // Detect test failure start (FAILED test_name.py::test_func)
            if line.starts_with("FAILED ") {
                if let Some(test_path) = line.strip_prefix("FAILED ") {
                    let test_path = test_path.split_whitespace().next().unwrap_or("");
                    current_test = Some(test_path.to_string());
                    in_failure = true;
                    current_error.clear();
                }
            }
            // Collect error lines
            else if in_failure && !line.starts_with("===") {
                current_error.push_str(line);
                current_error.push('\n');
            }
            // End of failure section
            else if in_failure && line.contains("short test summary") {
                if let Some(test_name) = current_test.take() {
                    let parts: Vec<&str> = test_name.split("::").collect();
                    let file_path = parts.first().unwrap_or(&"").to_string();
                    let func_name = parts.get(1).unwrap_or(&"").to_string();

                    failures.push(TestFailure {
                        file_path,
                        line: None, // Pytest doesn't always give line in plain output
                        error_message: current_error.trim().to_string(),
                        test_name: func_name,
                    });
                }
                in_failure = false;
            }
        }

        Ok(failures)
    }
}
