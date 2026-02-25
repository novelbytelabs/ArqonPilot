use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;

pub struct GitHubClient {
    client: Client,
    token: String,
    owner: String,
    repo: String,
}

#[derive(Serialize)]
struct CreatePRRequest {
    title: String,
    body: String,
    head: String,
    base: String,
}

#[derive(Deserialize)]
struct CreatePRResponse {
    html_url: String,
}

impl GitHubClient {
    pub fn new(owner: &str, repo: &str) -> Result<Self> {
        let token = env::var("GITHUB_TOKEN")
            .map_err(|_| anyhow!("GITHUB_TOKEN environment variable not set"))?;

        let client = Client::new();

        Ok(Self {
            client,
            token,
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }

    pub fn create_release_pr(
        &self,
        title: &str,
        body: &str,
        head_branch: &str,
        base_branch: &str,
    ) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls",
            self.owner, self.repo
        );

        let request_body = CreatePRRequest {
            title: title.to_string(),
            body: body.to_string(),
            head: head_branch.to_string(),
            base: base_branch.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("User-Agent", "arqon-pilot")
            .header("Accept", "application/vnd.github.v3+json")
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            let pr: CreatePRResponse = response.json()?;
            Ok(pr.html_url)
        } else {
            let error_text = response.text()?;
            Err(anyhow!("Failed to create PR: {}", error_text))
        }
    }
}
