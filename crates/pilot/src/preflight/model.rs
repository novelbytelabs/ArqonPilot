use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreflightStatus {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResult {
    pub status: PreflightStatus,
    pub failure_code: Option<String>,
    pub hint: Option<String>,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreflightStepType {
    Policy,
    Hook,
    Drift,
    Gate,
    Push,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    pub steps: Vec<PreflightStepItem>,
    pub final_status: PreflightStatus,
    pub evidence_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightStepItem {
    pub step: PreflightStepType,
    pub result: PreflightResult,
}

impl PreflightReport {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            final_status: PreflightStatus::Pass,
            evidence_path: None,
        }
    }

    pub fn add(&mut self, step: PreflightStepType, result: PreflightResult) {
        if result.status == PreflightStatus::Fail {
            self.final_status = PreflightStatus::Fail;
        }
        self.steps.push(PreflightStepItem { step, result });
    }

    pub fn is_pass(&self) -> bool {
        self.final_status == PreflightStatus::Pass
    }
}

impl Default for PreflightReport {
    fn default() -> Self {
        Self::new()
    }
}
