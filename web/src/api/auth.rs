use actix_web::cookie::{time::Duration, Cookie, SameSite};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Query};
use actix_web::{get, FromRequest, HttpRequest, HttpResponse, Scope};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256, Sha512};
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::config::config;
use crate::config::Config;
use crate::docs::UpdatePaths;
use crate::models::user::User;
use crate::models::{AppErr, AppErrBadRequest};
use crate::utils::{get_random_string, now, save_photo, send_after_login};
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
pub struct ApiDoc;

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
/// Login with Telegram
#[get("/login-telegram/")]
async fn login_telegram(
    rq: HttpRequest, state: Data<AppState>,
) -> Result<HttpResponse, AppErr> {
    let q = Query::<LoginTelQuery>::extract(&rq).await?;

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

    let config = config();
    let mut mac = Hmac256::new_from_slice(&config.bot_token_hash).unwrap();
    mac.update(msg.as_bytes());
    let result = mac.finalize();

    if !cfg!(debug_assertions) && hex::encode(result.into_bytes()) != q.hash {
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
    let is_admin = config.admins.contains(&(q.id as u64));

    match result {
        Ok(user) => {
            if user.auth_date == q.auth_date {
                return Err(AppErrBadRequest("invalid login info"));
            }

            sqlx::query_as! {
                User,
                "update users set name = ?, username = ?, auth_date = ?,
                token = ?, photo = ?, admin = ? where id = ?",
                name, q.username, q.auth_date, token_hashed,
                has_photo, is_admin, user.id,
            }
            .execute(&state.sql)
            .await?;
        }
        Err(_) => {
            sqlx::query_as! {
                User,
                "insert into users (id, name, username, auth_date, token, photo, admin)
                values(?, ?, ?, ?, ?, ?, ?)",
                q.id, name, q.username, q.auth_date, token_hashed, has_photo, is_admin
            }
            .execute(&state.sql)
            .await?;
        }
    };

    send_after_login(q.id).await;

    Ok(HttpResponse::build(StatusCode::FOUND)
        .cookie(
            Cookie::build("authorization", format!("user {}:{token}", q.id))
                // .domain("thora.dozar.bid")
                .path("/")
                .secure(true)
                .same_site(SameSite::Lax)
                .http_only(true)
                .max_age(Duration::weeks(12))
                .finish(),
        )
        .insert_header(("location", "/"))
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
