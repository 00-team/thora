use actix_files as af;
use actix_web::{
    dev::ServiceRequest,
    get,
    http::header::ContentType,
    middleware,
    web::{self, scope, Data},
    App, HttpResponse, HttpServer, Responder,
};
use config::Config;
use general::{general_get, General, PriceValue};
use sqlx::{Pool, Sqlite, SqlitePool};
use std::{
    collections::HashMap, env, fs::read_to_string,
    os::unix::fs::PermissionsExt, sync::Mutex,
};
use utoipa::OpenApi;

mod admin;
mod api;
mod config;
mod docs;
mod general;
mod models;
mod utils;
mod vendor;

pub struct AppState {
    pub sql: Pool<Sqlite>,
    pub general: Mutex<General>,
    pub prices: Mutex<HashMap<String, PriceValue>>,
    pub prices_update: Mutex<i64>,
}

#[get("/")]
async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(read_to_string("dist/index.html").expect("no index.html"))
}

#[get("/openapi.json")]
async fn openapi() -> impl Responder {
    let mut admin_doc = docs::ApiDoc::openapi();
    admin_doc.merge(admin::general::ApiDoc::openapi());
    admin_doc.merge(admin::stars::ApiDoc::openapi());
    admin_doc.merge(admin::users::ApiDoc::openapi());
    docs::doc_add_prefix(&mut admin_doc, "/admin", false);

    let mut doc = docs::ApiDoc::openapi();
    doc.merge(admin_doc);
    doc.merge(api::auth::ApiDoc::openapi());
    doc.merge(api::user::ApiDoc::openapi());
    doc.merge(api::vendor::ApiDoc::openapi());
    doc.merge(api::stars::ApiDoc::openapi());
    docs::doc_add_prefix(&mut doc, "/api", false);

    HttpResponse::Ok().json(doc)
}

#[get("/rapidoc")]
async fn rapidoc() -> impl Responder {
    HttpResponse::Ok().content_type(ContentType::html()).body(
        r###"<!doctype html>
    <html><head><meta charset="utf-8"><style>rapi-doc {
    --green: #00dc7d; --blue: #5199ff; --orange: #ff6b00;
    --red: #ec0f0f; --yellow: #ffd600; --purple: #782fef; }</style>
    <script type="module" src="/static/rapidoc.js"></script></head><body>
    <rapi-doc spec-url="/openapi.json" persist-auth="true"
    bg-color="#040404" text-color="#f2f2f2"
    header-color="#040404" primary-color="#ec0f0f"
    nav-text-color="#eee" font-size="largest"
    allow-spec-url-load="false" allow-spec-file-load="false"
    show-method-in-nav-bar="as-colored-block" response-area-height="500px"
    show-header="false" schema-expand-level="1" /></body> </html>"###,
    )
}

fn config_static(app: &mut web::ServiceConfig) {
    if cfg!(debug_assertions) {
        app.service(af::Files::new("/static", "./static"));
        app.service(af::Files::new("/assets", "./dist/assets"));
        app.service(af::Files::new("/record", Config::RECORD_DIR));
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_path(".env").expect("could not read .env");
    pretty_env_logger::init();

    let _ = std::fs::create_dir(Config::RECORD_DIR);

    let pool = SqlitePool::connect(
        &env::var("DATABASE_URL").expect("no DATABASE_URL in env"),
    )
    .await
    .expect("sqlite pool connect failed");

    let general = general_get(&pool).await.expect("general get failed");
    let data = Data::new(AppState {
        sql: pool,
        general: Mutex::new(general),
        prices: Default::default(),
        prices_update: Mutex::new(0),
    });

    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new("%s %r %Ts"))
            .app_data(data.clone())
            .configure(config_static)
            .service(openapi)
            .service(rapidoc)
            .service(index)
            .service(
                scope("/api")
                    .service(api::auth::router())
                    .service(api::user::router())
                    .service(api::vendor::router())
                    .service(api::stars::router())
                    .service(
                        scope("/admin")
                            .service(admin::general::router())
                            .service(admin::stars::router())
                            .service(admin::users::router()),
                    ),
            )
            .default_service(|r: ServiceRequest| {
                actix_utils::future::ok(
                    r.into_response(
                        HttpResponse::Ok()
                            .content_type(ContentType::html())
                            .body(
                                read_to_string("dist/index.html")
                                    .expect("no index.html"),
                            ),
                    ),
                )
            })
    });

    let server = if cfg!(debug_assertions) {
        server.bind(("0.0.0.0", 7000)).unwrap()
    } else {
        const PATH: &'static str = "/usr/share/nginx/sockets/thora.web.sock";
        let s = server.bind_uds(PATH).unwrap();
        std::fs::set_permissions(PATH, std::fs::Permissions::from_mode(0o777))?;
        s
    };

    server.run().await
}
