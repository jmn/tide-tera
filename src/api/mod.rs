mod auth;

use {
    crate::{app::Session, App, Request},
    tide::{Redirect, Response, Result, StatusCode},
};

macro_rules! session {
    ($req:expr) => {{
        let session = $req.ext::<Session>();
        if session.is_none() {
            return Ok(Redirect::new("/login/").into());
        }
        session.unwrap()
    }};
}