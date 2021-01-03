use crate::config::GitHubOauthConfig;

use {
    crate::config::GoogleOAuthConfig,
    anyhow::Result,
    oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl},
};

static AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
static TOKEN_URL: &str = "https://www.googleapis.com/oauth2/v3/token";

static GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
static GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub fn make_client(config: &GoogleOAuthConfig) -> Result<BasicClient> {
    Ok(BasicClient::new(
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
        AuthUrl::new(AUTH_URL.to_string())?,
        Some(TokenUrl::new(TOKEN_URL.to_string())?),
    )
    .set_redirect_url(RedirectUrl::new(config.redirect_url.clone())?))
}

pub fn make_client_github(config: &GitHubOauthConfig) -> Result<BasicClient> {
    Ok(BasicClient::new(
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
        AuthUrl::new(GITHUB_AUTH_URL.to_string())?,
        Some(TokenUrl::new(GITHUB_TOKEN_URL.to_string())?),
    ))
}