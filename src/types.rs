use {
    uuid::Uuid,
    oauth2::basic::BasicClient,
    serde::{Deserialize, Serialize},
    sqlx::PgPool,
    tera::Tera,
};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dino {
    pub id: Uuid,
    pub name: Option<String>,
    pub weight: Option<i32>,
    pub diet: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    pub email: String,
}
pub type Request = tide::Request<AppState>;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub google_oauth_client: BasicClient,
    // github_oauth_client: BasicClient,
    pub db_pool: PgPool,
    pub tera: Tera,
}
