use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub secret_key: String,
    pub google_oauth: GoogleOAuthConfig,
    // pub github_oauth: GitHubOauthConfig
}

#[derive(Clone, Debug, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

// #[derive(Clone, Debug, Deserialize)]
// pub struct GitHubOauthConfig {
//     pub client_id: String,
//     pub client_secret: String,
//     pub redirect_url: String,
// }
