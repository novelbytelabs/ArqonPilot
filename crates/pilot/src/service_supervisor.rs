use miette::{IntoDiagnostic, Result};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub backoff_factor: f64,
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            backoff_factor: 2.0,
            max_delay_ms: 5000,
        }
    }
}

pub async fn supervised_start<F, Fut, T, E>(
    name: &str,
    mut action_fn: F,
    policy: RetryPolicy,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    let mut delay = policy.initial_delay_ms;
    let mut attempt = 1;

    loop {
        println!(
            "[Supervisor] Initiating {} (Attempt {}/{})",
            name, attempt, policy.max_attempts
        );
        match action_fn().await {
            Ok(val) => {
                println!(
                    "[Supervisor] {} started successfully on attempt {}",
                    name, attempt
                );
                return Ok(val);
            }
            Err(e) => {
                if attempt >= policy.max_attempts {
                    let err_msg = format!(
                        "Supervisor failed to start {} after {} attempts. Last error: {}",
                        name, attempt, e
                    );
                    eprintln!("[Supervisor] FATAL: {}", err_msg);
                    return Err(miette::miette!(err_msg));
                }

                eprintln!(
                    "[Supervisor] {} failed to start (Attempt {}/{}). Retrying in {}ms... Error: {}",
                    name, attempt, policy.max_attempts, delay, e
                );

                sleep(Duration::from_millis(delay)).await;

                delay = (delay as f64 * policy.backoff_factor) as u64;
                if delay > policy.max_delay_ms {
                    delay = policy.max_delay_ms;
                }

                attempt += 1;
            }
        }
    }
}
