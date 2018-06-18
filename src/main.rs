extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate dotenv;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate uuid;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate num_cpus;
extern crate postgis;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

use actix::prelude::*;
use actix_web::{http::Method, middleware, server, App, HttpRequest, Responder};

use futures::{future::ok, Future};
use r2d2_postgres::{PostgresConnectionManager, TlsMode};

mod api;
mod auth;
mod db;
mod models;

pub struct AppState {
    pub db: Addr<Syn, db::DbExecutor>,
}

fn index(
    req: HttpRequest<AppState>,
) -> impl Future<Item = impl Responder, Error = actix_web::Error> {
    let _ = req.state().db;
    ok("Hello World")
}

fn main() {
    std::env::set_var("RUST_LOG", "INFO");
    env_logger::init();
    dotenv::dotenv().ok();

    let sys = actix::System::new("sg_g6_backend");

    let manager =
        PostgresConnectionManager::new(std::env::var("DATABASE_URL").unwrap(), TlsMode::None)
            .expect("Could not connect to Database");
    let pool = db::Pool::new(manager).expect("Could not connect to Database 2");

    let addr = SyncArbiter::start(num_cpus::get(), move || db::DbExecutor(pool.clone()));

    server::new(move || {
        App::with_state(AppState { db: addr.clone() })
            .middleware(middleware::Logger::default())
            .resource("/register", |r| r.method(Method::POST).with_async(api::register))
            // To accept just addfriend again
            .resource("/addfriend", |r| {
                r.middleware(auth::AuthMiddleware);
                r.method(Method::POST).with_async(api::addfriend)
            })
            .resource("/friends", |r| {
                r.middleware(auth::AuthMiddleware);
                r.method(Method::GET).with_async(api::friends)
            })
            .resource("/weather", |r| r.method(Method::GET).a(index))
            .resource("/nearby", |r| r.method(Method::GET).a(index))
            .resource("/update", |r| r.method(Method::POST).a(index))
    }).bind("127.0.0.1:8000")
        .unwrap()
        .start();

    sys.run();
}
