use google_oauth::make_client_github;

use {
    crate::{config::AppConfig, google_oauth::make_client},
    anyhow::Context,
    dotenv,
    oauth2::basic::BasicClient,
    serde::{Deserialize, Serialize},
    sqlx::Pool,
    sqlx::{query, query_as, PgPool},
    std::env,
    tera::Tera,
    tide::{listener::Listener, Body, Redirect, Response, Server},
    tide_secure_cookie_session::SecureCookieSessionMiddleware,
    tide_tera::prelude::*,
    uuid::Uuid,
};

mod auth;
mod config;
mod google_oauth;

// #[derive(Debug, Serialize, Deserialize)]
// struct MySession {
//     name: String,
//     count: usize,
// }

macro_rules! session {
    ($req:expr) => {{
        let session = $req.ext::<Session>();
        if session.is_none() {
            return Ok(Redirect::new("/login/").into());
        }
        session.unwrap()
    }};
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    pub email: String,
}
pub type Request = tide::Request<AppState>;

#[derive(Clone, Debug)]
pub struct AppState {
    config: AppConfig,
    google_oauth_client: BasicClient,
    github_oauth_client: BasicClient,
    db_pool: PgPool,
    tera: Tera,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Dino {
    id: Uuid,
    name: String,
    weight: i32,
    diet: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Dinos {
    dinos: Vec<Dino>,
}

struct RestEntity {
    base_path: String,
}

impl RestEntity {
    async fn create(mut req: Request) -> tide::Result {
        let dino: Dino = req.body_json().await?;
        let db_pool = req.state().db_pool.clone();
        let row = query_as!(
            Dino,
            r#"
            INSERT INTO dinos (id, name, weight, diet) VALUES
            ($1, $2, $3, $4) returning id, name, weight, diet
            "#,
            dino.id,
            dino.name,
            dino.weight,
            dino.diet
        )
        .fetch_one(&db_pool)
        .await?;

        let mut res = Response::new(201);
        res.set_body(Body::from_json(&row)?);
        Ok(res)
    }

    async fn list(req: tide::Request<AppState>) -> tide::Result {
        let db_pool = req.state().db_pool.clone();
        let rows = query_as!(
            Dino,
            r#"
            SELECT id, name, weight, diet from dinos as "id, name, weight, diet"
            "#
        )
        .fetch_all(&db_pool)
        .await?;
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&rows)?);
        Ok(res)
    }

    async fn get(req: tide::Request<AppState>) -> tide::Result {
        let db_pool = req.state().db_pool.clone();
        let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
        let row = query_as!(
            Dino,
            r#"
            SELECT  id, name, weight, diet from dinos
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&db_pool)
        .await?;

        let res = match row {
            None => Response::new(404),
            Some(row) => {
                let mut r = Response::new(200);
                r.set_body(Body::from_json(&row)?);
                r
            }
        };

        Ok(res)
    }

    async fn update(mut req: tide::Request<AppState>) -> tide::Result {
        let dino: Dino = req.body_json().await?;
        let db_pool = req.state().db_pool.clone();
        let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
        let row = query_as!(
            Dino,
            r#"
            UPDATE dinos SET name = $2, weight = $3, diet = $4
            WHERE id = $1
            returning id, name, weight, diet
            "#,
            id,
            dino.name,
            dino.weight,
            dino.diet
        )
        .fetch_optional(&db_pool)
        .await?;

        let res = match row {
            None => Response::new(404),
            Some(row) => {
                let mut r = Response::new(200);
                r.set_body(Body::from_json(&row)?);
                r
            }
        };

        Ok(res)
    }

    async fn delete(req: tide::Request<AppState>) -> tide::Result {
        let db_pool = req.state().db_pool.clone();
        let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
        let row = query!(
            r#"
            delete from dinos
            WHERE id = $1
            returning id
            "#,
            id
        )
        .fetch_optional(&db_pool)
        .await?;

        let res = match row {
            None => Response::new(404),
            Some(_) => Response::new(204),
        };

        Ok(res)
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    dotenv::dotenv().ok();
    tide::log::start();

    let config = AppConfig {
        secret_key: env::var("APP_SECRET_KEY")
            .context("get APP_SECRET_KEY environment variable")?,
        google_oauth: config::GoogleOAuthConfig {
            client_id: env::var("APP_GOOGLE_OAUTH_CLIENT_ID")
                .context("get APP_GOOGLE_OAUTH_CLIENT_ID environment variable")?,
            client_secret: env::var("APP_GOOGLE_OAUTH_CLIENT_SECRET")
                .context("get APP_GOOGLE_OAUTH_CLIENT_SECRET environment variable")?,
            redirect_url: env::var("APP_GOOGLE_OAUTH_REDIRECT_URL")
                .context("get APP_GOOGLE_OAUTH_REDIRECT_URL environment variable")?,
        },
        github_oauth: config::GitHubOauthConfig {
            client_id: env::var("APP_GITHUB_OAUTH_CLIENT_ID")
                .context("get APP_GITHUB_OAUTH_CLIENT_ID environment variable")?,
            client_secret: env::var("APP_GITHUB_OAUTH_CLIENT_SECRET")
                .context("get APP_GITHUB_OAUTH_CLIENT_SECRET environment variable")?,
            redirect_url: env::var("APP_GITHUB_OAUTH_REDIRECT_URL")
                .context("get APP_GITHUB_OAUTH_REDIRECT_URL environment variable")?,
        },
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

pub async fn make_db_pool(db_url: &String) -> PgPool {
    Pool::connect(&db_url).await.unwrap()
}

async fn server(db_pool: PgPool, config: AppConfig) -> Server<AppState> {
    let mut tera = Tera::new("templates/**/*").expect("Error parsing templates directory");
    tera.autoescape_on(vec!["html"]);

    let google_oauth_client = make_client(&config.google_oauth).unwrap();
    let github_oauth_client = make_client_github(&config.github_oauth).unwrap();
    
    let middleware =
        SecureCookieSessionMiddleware::<Session>::new(config.secret_key.as_bytes().to_vec());

    let state = AppState {
        db_pool,
        tera,
        config,
        google_oauth_client,
        github_oauth_client
    };

    let mut app = tide::with_state(state);
    app.with(middleware);

    // index page
    app.at("/").get(|req: tide::Request<AppState>| async move {
        let session = session!(req);
        let tera = req.state().tera.clone();
        let db_pool = req.state().db_pool.clone();
        let rows = query_as!(
            Dino,
            r#"
            SELECT id, name, weight, diet from dinos
            "#
        )
        .fetch_all(&db_pool)
        .await?;
        tera.render_response(
            "index.html",
            &context! {
               "title" => String::from("Tide basic CRUD"),
               "dinos" => rows,
               "user_email" => session.email
            },
        )
    });

    app.at("/public")
        .serve_dir("./public/")
        .expect("Invalid static file directory");

    // Auth routes
    app.at("logout/").get(auth::logout);

    let mut login = app.at("login/");
    login.at("/").get(auth::login);
    login.at("authorized/").get(auth::login_authorized);

    // new dino
    app.at("/dinos/new")
        .get(|req: tide::Request<AppState>| async move {
            let tera = req.state().tera.clone();

            tera.render_response(
                "form.html",
                &context! {
                   "title" => String::from("Create new dino")
                },
            )
        });

    // edit dino
    app.at("/dinos/:id/edit")
        .get(|req: tide::Request<AppState>| async move {
            let tera = req.state().tera.clone();
            let db_pool = req.state().db_pool.clone();
            let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
            let row = query_as!(
                Dino,
                r#"
            SELECT  id, name, weight, diet from dinos
            WHERE id = $1
            "#,
                id
            )
            .fetch_optional(&db_pool)
            .await?;

            let res = match row {
                None => Response::new(404),
                Some(row) => {
                    let mut r = Response::new(200);
                    let b = tera.render_body(
                        "form.html",
                        &context! {
                            "title" => String::from("Edit dino"),
                            "dino" => row
                        },
                    )?;
                    r.set_body(b);
                    r
                }
            };

            Ok(res)
        });

    let dinos_endpoint = RestEntity {
        base_path: String::from("/dinos"),
    };

    register_rest_entity(&mut app, dinos_endpoint);

    app
}
