use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Scope};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256, Sha512};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::config::config;
use crate::config::Config;
use crate::docs::UpdatePaths;
use crate::models::{AppErr, AppErrBadRequest, User};
use crate::utils::{get_random_string, now, save_photo};
use crate::AppState;

type Hmac256 = Hmac<Sha256>;

#[derive(OpenApi)]
#[openapi(
    tags((name = "api::auth")),
    paths(login_telegram),
    components(schemas(LoginTelQuery)),
    servers((url = "/auth")),
    modifiers(&UpdatePaths)
)]
pub struct ApiAuthDoc;

// #[post("/login/")]
// async fn login(body: Json<LoginBody>, state: Data<AppState>) -> Response<User> {
//     verify(&body.phone, &body.code, Action::Login, &state.sql).await?;
//

//

//
//     Ok(Json(user))
// }

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct LoginTelQuery {
    auth_date: i64,
    first_name: String,
    id: i64,
    last_name: Option<String>,
    photo_url: Option<String>,
    username: Option<String>,
    hash: String,
}

#[utoipa::path(
    get,
    params(LoginTelQuery),
    responses((status = 302))
)]
#[get("/login-telegram/")]
async fn login_telegram(
    q: Query<LoginTelQuery>, state: Data<AppState>,
) -> Result<HttpResponse, AppErr> {
    if q.auth_date < now() - 6 * 3600 {
        return Err(AppErrBadRequest("login again"));
    }

    let mut msg = format!(
        "auth_date={}\nfirst_name={}\nid={}",
        q.auth_date, q.first_name, q.id
    );

    let mut name = q.first_name.clone();
    if let Some(last_name) = &q.last_name {
        msg += &("\nlast_name=".to_string() + last_name);
        name += &(" ".to_string() + last_name);
    }
    if let Some(photo_url) = &q.photo_url {
        msg += &("\nphoto_url=".to_string() + photo_url);
        save_photo(photo_url, q.id).await?;
    }
    if let Some(username) = &q.username {
        msg += &("\nusername=".to_string() + username)
    }

    let mut mac = Hmac256::new_from_slice(&config().bot_token_hash).unwrap();
    mac.update(msg.as_bytes());
    let result = mac.finalize();

    if hex::encode(result.into_bytes()) != q.hash {
        return Err(AppErrBadRequest("invalid login credentials ❌"));
    }

    let token = get_random_string(Config::TOKEN_ABC, 69);
    let token_hashed = hex::encode(Sha512::digest(&token));

    let result = sqlx::query_as! {
        User,
        "select * from users where id = ?",
        q.id
    }
    .fetch_one(&state.sql)
    .await;

    let has_photo = q.photo_url.is_some();

    match result {
        Ok(user) => {
            if user.auth_date == q.auth_date {
                return Err(AppErrBadRequest("invalid login info"));
            }

            sqlx::query_as! {
                User,
                "update users set name = ?, auth_date = ?,
                token = ?, photo = ? where id = ?",
                name, q.auth_date, token_hashed, has_photo, user.id
            }
            .execute(&state.sql)
            .await?;
        }
        Err(_) => {
            sqlx::query_as! {
                User,
                "insert into users (id, name, auth_date, token, photo)
                values(?, ?, ?, ?, ?)",
                q.id, name, q.auth_date, token_hashed, has_photo
            }
            .execute(&state.sql)
            .await?;
        }
    };

    Ok(HttpResponse::build(StatusCode::FOUND)
        .insert_header((header::LOCATION, "/"))
        .insert_header((header::AUTHORIZATION, "Bearer token"))
        .finish())
}

pub fn router() -> Scope {
    Scope::new("/auth").service(login_telegram)
    // .service(user_get)
    // .service(user_update)
    // .service(user_update_photo)
    // .service(user_delete_photo)
    // .service(user_wallet_test)
    // .service(user_transactions_list)
}
