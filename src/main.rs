use anyhow::Context;
use dotenv;

use serde::{Deserialize, Serialize};
use sqlx::Pool;
use sqlx::{query, query_as, PgPool};
use std::env;
use tide::{listener::Listener, Body, Request, Response, Server};
use uuid::Uuid;

use tera::Tera;
use tide_tera::prelude::*;

#[derive(Clone, Debug)]
struct State {
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
    async fn create(mut req: Request<State>) -> tide::Result {
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

    async fn list(req: tide::Request<State>) -> tide::Result {
        let db_pool = req.state().db_pool.clone();
        let rows = query_as!(
            Dino,
            r#"
            SELECT id, name, weight, diet from dinos as "id!, name, weight, diet"
            "#
        )
        .fetch_all(&db_pool)
        .await?;
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&rows)?);
        Ok(res)
    }

    async fn get(req: tide::Request<State>) -> tide::Result {
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

    async fn update(mut req: tide::Request<State>) -> tide::Result {
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

    async fn delete(req: tide::Request<State>) -> tide::Result {
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

    let db_url = env::var("DATABASE_URL").context("get DATABASE_URL environment variable")?;
    let port = env::var("PORT").context("get PORT environment variable")?;

    let db_pool = make_db_pool(&db_url).await;
    let app = server(db_pool).await;

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

fn register_rest_entity(app: &mut Server<State>, entity: RestEntity) {
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

async fn server(db_pool: PgPool) -> Server<State> {
    let mut tera = Tera::new("templates/**/*").expect("Error parsing templates directory");
    tera.autoescape_on(vec!["html"]);

    let state = State { db_pool, tera };

    let mut app = tide::with_state(state);

    // index page
    app.at("/").get(|req: tide::Request<State>| async move {
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
               "dinos" => rows
            },
        )
    });

    app.at("/public")
        .serve_dir("./public/")
        .expect("Invalid static file directory");

    // new dino
    app.at("/dinos/new")
        .get(|req: tide::Request<State>| async move {
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
        .get(|req: tide::Request<State>| async move {
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
