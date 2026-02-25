use crate::apply::{apply_fix, restore_backup};
use crate::audit::AuditLog;
use crate::context::ContextBuilder;
use crate::llm::{LlmClient, RemoteLlm};
use crate::parser_rust::TestFailure;
use crate::prompts::PromptTemplates;
use crate::verify::VerificationGate;
use anyhow::Result;
use pilot_oracle::OracleStore;
use std::path::PathBuf;

pub struct HealingLoop {
    context_builder: ContextBuilder,
    llm: RemoteLlm,
    max_attempts: u32,
    root: PathBuf,
    audit: Option<AuditLog>,
}

#[derive(Debug, Clone)]
pub enum HealOutcome {
    Success,
    CompileFailed,
    TestFailed,
    NoFixGenerated,
    MaxAttemptsExceeded,
}

impl HealingLoop {
    pub fn new(store: OracleStore, root: PathBuf, max_attempts: u32) -> Result<Self> {
        let context_builder = ContextBuilder::new(store, root.clone());
        let llm = RemoteLlm::new()?;

        // Initialize audit log
        let audit_path = root.join(".pilot/heal_audit.db");
        let audit = if let Some(parent) = audit_path.parent() {
            std::fs::create_dir_all(parent).ok();
            AuditLog::open(audit_path.to_str().unwrap_or(".pilot/heal_audit.db")).ok()
        } else {
            None
        };

        Ok(Self {
            context_builder,
            llm,
            max_attempts,
            root,
            audit,
        })
    }

    pub fn run(&mut self, failure: &TestFailure) -> Result<HealOutcome> {
        for attempt in 0..self.max_attempts {
            println!("Healing attempt {}/{}", attempt + 1, self.max_attempts);

            // 1. Build context
            let ctx = self.context_builder.build_context(failure)?;

            // 2. Generate prompt
            let prompt = if failure.file_path.ends_with(".rs") {
                PromptTemplates::rust_repair(&ctx)
            } else {
                PromptTemplates::python_repair(&ctx)
            };

            // 3. Generate fix from LLM
            let fix = self.llm.generate_fix(&prompt)?;
            if fix.is_empty() {
                println!("LLM generated no fix");
                let outcome = HealOutcome::NoFixGenerated;
                self.log_attempt(failure, &prompt, &fix, &outcome);
                return Ok(outcome);
            }

            // 4. Apply fix
            let file_path = self.root.join(&failure.file_path);
            apply_fix(&file_path, &fix)?;

            // 5. Verify
            let gate = VerificationGate::new(self.root.clone());
            if gate.check_compile()? && gate.check_lint()? && gate.check_tests()? {
                let outcome = HealOutcome::Success;
                self.log_attempt(failure, &prompt, &fix, &outcome);
                return Ok(outcome);
            }

            // 6. Rollback on failure - restore original before next attempt
            println!("Verification failed, rolling back...");
            restore_backup(&file_path)?;

            // Log unsuccessful attempt
            self.log_attempt(failure, &prompt, &fix, &HealOutcome::TestFailed);
        }

        Ok(HealOutcome::MaxAttemptsExceeded)
    }

    fn log_attempt(&self, failure: &TestFailure, prompt: &str, fix: &str, outcome: &HealOutcome) {
        if let Some(ref audit) = self.audit {
            if let Err(e) = audit.log_attempt(failure, prompt, fix, outcome) {
                eprintln!("Warning: Failed to log audit entry: {}", e);
            }
        }
    }
}
