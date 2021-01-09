// use google_oauth::make_client_github;
mod auth;
mod google_oauth;
mod types;
mod controllers;
mod handlers;

pub use {
    types::{AppConfig, Session, Request, AppState, RestEntity, Dino}
};

use {
    crate::{google_oauth::make_client},
    types::GoogleOAuthConfig,
    anyhow::Context,
    dotenv,
    sqlx::{Pool, PgPool},
    std::env,
    tera::Tera,
    tide::{Error, listener::Listener, Redirect, Server},
    tide_secure_cookie_session::SecureCookieSessionMiddleware,
    tide_tera::prelude::*,
    tide::security::{CorsMiddleware, Origin},
    http_types::headers::HeaderValue,
    // controllers::dino,
    controllers::views,
    uuid::Uuid
   
};

#[async_std::main]
async fn main() -> tide::Result<()> {
    dotenv::dotenv().ok();
    tide::log::with_level(tide::log::LevelFilter::Debug);
    // tide::log::start();

    let config = AppConfig {
        secret_key: env::var("APP_SECRET_KEY")
            .context("get APP_SECRET_KEY environment variable")?,
        google_oauth: GoogleOAuthConfig {
            client_id: env::var("APP_GOOGLE_OAUTH_CLIENT_ID")
                .context("get APP_GOOGLE_OAUTH_CLIENT_ID environment variable")?,
            client_secret: env::var("APP_GOOGLE_OAUTH_CLIENT_SECRET")
                .context("get APP_GOOGLE_OAUTH_CLIENT_SECRET environment variable")?,
            redirect_url: env::var("APP_GOOGLE_OAUTH_REDIRECT_URL")
                .context("get APP_GOOGLE_OAUTH_REDIRECT_URL environment variable")?,
        },
        // github_oauth: config::GitHubOauthConfig {
        //     client_id: env::var("APP_GITHUB_OAUTH_CLIENT_ID")
        //         .context("get APP_GITHUB_OAUTH_CLIENT_ID environment variable")?,
        //     client_secret: env::var("APP_GITHUB_OAUTH_CLIENT_SECRET")
        //         .context("get APP_GITHUB_OAUTH_CLIENT_SECRET environment variable")?,
        //     redirect_url: env::var("APP_GITHUB_OAUTH_REDIRECT_URL")
        //         .context("get APP_GITHUB_OAUTH_REDIRECT_URL environment variable")?,
        // },
    };

    let db_url = env::var("DATABASE_URL").context("get DATABASE_URL environment variable")?;
    let port = env::var("PORT").context("get PORT environment variable")?;

    let db_pool = make_db_pool(&db_url).await;
    let app = server(db_pool, config).await;

    let mut listener = app
        .bind(format!("http://0.0.0.0:{}", port))
        .await
        .expect("can't bind the port");

    for info in listener.info().iter() {
        println!("Server listening on {}", info);
    }

    listener.accept().await.unwrap();

    Ok(())
}

fn register_rest_entity(app: &mut Server<AppState>, entity: RestEntity) {
    app.at(&entity.base_path)
        .get(RestEntity::list)
        .post(RestEntity::create);

    app.at(&format!("{}/:id", entity.base_path))
        .get(RestEntity::get)
        .put(RestEntity::update)
        .delete(RestEntity::delete);
}

pub async fn make_db_pool(db_url: &str) -> PgPool {
    Pool::connect(&db_url).await.unwrap()
}

async fn server(db_pool: PgPool, config: AppConfig) -> Server<AppState> {
    let mut tera = Tera::new("templates/**/*").expect("Error parsing templates directory");
    tera.autoescape_on(vec!["html"]);

    let google_oauth_client = make_client(&config.google_oauth).unwrap();
    // let github_oauth_client = make_client_github(&config.github_oauth).unwrap();

    let session_middleware =
        SecureCookieSessionMiddleware::<Session>::new(config.secret_key.as_bytes().to_vec());

    let cors_middleware = CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false);

    let state = AppState {
        db_pool,
        tera,
        config,
        google_oauth_client,
        // github_oauth_client,
    };

    let mut app = tide::with_state(state);
    app.with(tide::log::LogMiddleware::new());
    app.with(session_middleware);
    app.with(cors_middleware);

    // index page
    app.at("/").get(views::index);

    app.at("/public")
        .serve_dir("./public/")
        .expect("Invalid static file directory");

    // Auth routes
    app.at("logout/").get(auth::logout);

    let mut login = app.at("login/");
    login.at("/").get(auth::login);
    login.at("authorized/").get(auth::login_authorized);

    app.at("/dinos/new").get(views::new);
    app.at("/dinos/:id/edit").get(views::edit);

    let dinos_endpoint = RestEntity {
        base_path: String::from("/dinos"),
    };

    register_rest_entity(&mut app, dinos_endpoint);

    app
}
