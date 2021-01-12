use super::*;
use tide::{Request, Response};
use uuid::Uuid;

macro_rules! session {
    ($req:expr) => {{
        let session = $req.ext::<Session>();
        if session.is_none() {
            return Ok(Redirect::new("/login/").into());
        }
        session.unwrap()
    }};
}

pub async fn index(req: Request<AppState>) -> tide::Result {
    let session = session!(req);
    let tera = req.state().tera.clone();
    let db_pool = req.state().db_pool.clone();
    // let rows = handlers::dino::list(&db_pool).await?;
    let (rows, execution_time) = handlers::dino::list_timed(&db_pool).await?;

    tera.render_response(
        "index.html",
        &context! {
           "title" => String::from("Tide basic CRUD"),
           "dinos" => rows,
           "user_email" => session.email,
           "db_execution_time_ms" => format!("{}", execution_time.as_millis())
        },
    )
}

pub async fn new(req: Request<AppState>) -> tide::Result {
    let tera = req.state().tera.clone();

    tera.render_response(
        "form.html",
        &context! {
            "title" => String::from("Create new dino")
        },
    )
}

pub async fn edit(req: Request<AppState>) -> tide::Result {
    let tera = req.state().tera.clone();
    let db_pool = req.state().db_pool.clone();
    let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
    let row = handlers::dino::get(id, &db_pool).await?;

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
}