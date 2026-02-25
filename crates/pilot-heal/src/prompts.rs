use crate::context::HealContext;

pub struct PromptTemplates;

impl PromptTemplates {
    /// Generate a repair prompt for Rust code
    pub fn rust_repair(ctx: &HealContext) -> String {
        format!(
            r#"You are a Rust expert. Fix the following code error.

## Error
File: {}
Line: {}
Message: {}

## Current Code
```rust
{}
```

## Instructions
1. Analyze the error message
2. Provide the corrected code block
3. Only output the fixed code, no explanations
4. Preserve the original function signature

## Fixed Code
```rust
"#,
            ctx.failure.file_path,
            ctx.failure
                .line
                .map(|l| l.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            ctx.failure.error_message,
            ctx.source_snippet
        )
    }

    /// Generate a repair prompt for Python code
    pub fn python_repair(ctx: &HealContext) -> String {
        format!(
            r#"You are a Python expert. Fix the following test failure.

## Error
File: {}
Test: {}
Message: {}

## Current Code
```python
{}
```

## Instructions
1. Analyze the error message
2. Provide the corrected code block
3. Only output the fixed code, no explanations

## Fixed Code
```python
"#,
            ctx.failure.file_path,
            ctx.failure.test_name,
            ctx.failure.error_message,
            ctx.source_snippet
        )
    }
}
