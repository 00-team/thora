use actix_web::web::{Data, Json, Query};
use actix_web::{get, post, HttpResponse, Scope};
use serde::Deserialize;
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::docs::UpdatePaths;
use crate::models::message::Message;
use crate::models::transaction::{
    Transaction, TransactionKind, TransactionStatus,
};
use crate::models::user::User;
use crate::models::{AppErr, ListInput, Response};
use crate::AppState;

#[derive(OpenApi)]
#[openapi(
    tags((name = "api::user")),
    paths(user_get, user_deposit, user_transactions, user_messages, user_message_seen),
    components(schemas(User, Transaction, TransactionKind, TransactionStatus, Message)),
    servers((url = "/user")),
    modifiers(&UpdatePaths)
)]
pub struct ApiDoc;

#[utoipa::path(get, responses((status = 200, body = User)))]
/// Get
#[get("/")]
async fn user_get(user: User) -> Response<User> {
    Ok(Json(user))
}

#[derive(Deserialize, ToSchema, IntoParams)]
struct DepositParams {
    amount: i64,
    auto_order: Option<bool>,
}

#[utoipa::path(
    get,
    params(DepositParams),
    responses((status = 200, body = String))
)]
/// Deposit
#[get("/deposit/")]
async fn user_deposit(
    user: User, q: Query<DepositParams>, state: Data<AppState>,
) -> Response<String> {
    let amount = q.amount.max(50_000).min(50_000_000);
    let wallet = user.wallet + amount;

    if let Some(true) = q.auto_order {
        todo!("impl this")
    }

    let tid = sqlx::query! {
        "insert into transactions(user, amount) values(?, ?)",
        user.id,
        amount
    }
    .execute(&state.sql)
    .await?
    .last_insert_rowid();

    sqlx::query! {
        "update users set wallet = ? where id = ?",
        wallet, user.id
    }
    .execute(&state.sql)
    .await?;

    sqlx::query! {
        "update transactions set status = ? where id = ?",
        TransactionStatus::Success, tid
    }
    .execute(&state.sql)
    .await?;

    Ok(Json(format!("amount is {amount}")))
}

#[utoipa::path(
    get,
    params(ListInput),
    responses((status = 200, body = Vec<Transaction>))
)]
/// List Transactions
#[get("/transactions/")]
async fn user_transactions(
    user: User, q: Query<ListInput>, state: Data<AppState>,
) -> Response<Vec<Transaction>> {
    let offset = q.page * 32;
    let result = sqlx::query_as! {
        Transaction,
        "select * from transactions where user = ? limit 32 offset ?",
        user.id, offset
    }
    .fetch_all(&state.sql)
    .await?;

    Ok(Json(result))
}

#[utoipa::path(
    get,
    params(ListInput),
    responses((status = 200, body = Vec<Message>))
)]
/// List Messages
#[get("/messages/")]
async fn user_messages(
    user: User, q: Query<ListInput>, state: Data<AppState>,
) -> Response<Vec<Message>> {
    let offset = q.page * 32;
    let result = sqlx::query_as! {
        Message,
        "select * from messages where user = ? order by id desc limit 32 offset ?",
        user.id, offset
    }
    .fetch_all(&state.sql)
    .await?;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    params(("id" = i64, Path,)),
    responses((status = 200))
)]
/// Message Seen
#[post("/messages/{id}/seen/")]
async fn user_message_seen(
    user: User, message: Message, state: Data<AppState>,
) -> Result<HttpResponse, AppErr> {
    sqlx::query! {
        "update messages set seen = true where id = ? and user = ?",
        message.id, user.id
    }
    .execute(&state.sql)
    .await?;

    Ok(HttpResponse::Ok().finish())
}

pub fn router() -> Scope {
    Scope::new("/user")
        .service(user_get)
        .service(user_deposit)
        .service(user_transactions)
        .service(user_messages)
        .service(user_message_seen)
}
