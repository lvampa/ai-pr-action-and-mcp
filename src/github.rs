use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use reqwest::Client as HttpClient;

pub struct Client {
    octocrab: Octocrab,
    http: HttpClient,
    token: String,
    owner: String,
    repo: String,
    pr_number: u64,
}

impl Client {
    pub fn new(token: &str, full_repo: &str, pr_number: u64) -> Result<Self> {
        let (owner, repo) = full_repo
            .split_once('/')
            .ok_or_else(|| anyhow!("GITHUB_REPOSITORY must be in owner/repo format"))?;
        let octocrab = Octocrab::builder()
            .personal_token(token.to_string())
            .build()?;
        Ok(Self {
            octocrab,
            http: HttpClient::new(),
            token: token.to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            pr_number,
        })
    }

    /// Fetches the PR diff. Uses reqwest directly because octocrab doesn't support
    /// the `application/vnd.github.v3.diff` Accept header natively.
    pub async fn diff(&self) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}",
            self.owner, self.repo, self.pr_number
        );
        let diff = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3.diff")
            .header("User-Agent", "ai-cr-action")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(diff)
    }

    pub async fn post_comment(&self, body: &str) -> Result<()> {
        self.octocrab
            .issues(&self.owner, &self.repo)
            .create_comment(self.pr_number, body)
            .await?;
        Ok(())
    }
}
