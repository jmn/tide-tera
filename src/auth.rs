use {
    crate::{Request, Session},
    http_types,
    serde::Deserialize,
    tide::{Redirect, Response, Result, StatusCode},
};

use oauth2::TokenResponse;
// Alternatively, this can be oauth2::curl::http_client or a custom.
use oauth2::reqwest::http_client;
use oauth2::{AuthorizationCode, CsrfToken, Scope};
use tide::log::info;

#[derive(Debug, Deserialize)]
struct AuthRequestQuery {
    code: String,
    state: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    email: String,
}

pub(super) async fn login(req: Request) -> Result<Redirect<String>> {
    let oauth_client = &req.state().google_oauth_client;
    // let oauth_client = &req.state().github_oauth_client;

    let (authorize_url, _csrf_state) = oauth_client
        .authorize_url(CsrfToken::new_random)
        // .add_scope(Scope::new("user:email".to_string()))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/userinfo.email".to_string(),
        ))
        .url();

    Ok(Redirect::new(authorize_url.to_string()))
}

pub(super) async fn login_authorized(req: Request) -> Result {
    let query: AuthRequestQuery = req.query()?;
    let code = AuthorizationCode::new(query.code);
    // let token_req = req.state().google_oauth_client.exchange_code(code);
    let token_res = req
        .state()
        .github_oauth_client
        .exchange_code(code)
        .request(http_client);

    // let access_token = token_res.access_token();

    info!("CHECKING TOKEN");

    match &token_res {
        Ok(token) => {
            // NB: Github returns a single comma-separated "scope" parameter instead of multiple
            // space-separated scopes. Github-specific clients can parse this scope into
            // multiple scopes by splitting at the commas. Note that it's not safe for the
            // library to do this by default because RFC 6749 allows scopes to contain commas.
            let scopes = if let Some(scopes_vec) = token.scopes() {
                scopes_vec
                    .iter()
                    .map(|comma_separated| comma_separated.split(','))
                    .flatten()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            info!("Github returned the following scopes:\n{:?}\n", scopes);

            // let userinfo: UserInfoResponse =
            //     surf::get("https://www.googleapis.com/oauth2/v2/userinfo")
            //         .header(
            //             http_types::headers::AUTHORIZATION,
            //             format!("Bearer {}", token.access_token().secret()),
            //         )
            //         .recv_json()
            //         .await?;
        }
        Err(e) => {
            info!("TOKEN FAILED: {:?} {:?} ", e, token_res)
        }
    }

    // if let Ok(token) = token_res {

    //     //return userinfo;
    // }

    let session = Session {
        email: String::from("Foo@bar.com"), //userinfo.email,
    };

    let resp: Response = Redirect::new("/").into();
    // FIXME
    // use Response::insert_ext
    // https://github.com/http-rs/tide/commit/7f946a9c9bee84c430dda62ebdf736b287fa0797
    let mut resp: tide::http::Response = resp.into();
    resp.ext_mut().insert(session);
    let resp = resp.into();

    Ok(resp)
}

pub(super) async fn logout(req: Request) -> Result {
    let cookie = req.cookie("session");
    let mut resp = Response::new(StatusCode::Ok);
    if let Some(mut cookie) = cookie {
        cookie.set_path("/");
        resp.remove_cookie(cookie);
    }
    resp.set_body("Goodbye!".to_string());
    Ok(resp)
}
