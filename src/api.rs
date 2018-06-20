use super::AppState;
use actix_web::{client, Error, HttpMessage, HttpRequest, HttpResponse, Json,
                Query as Q, Responder, State};
use db::Query;
use futures::{future::{err, result},
              Future};
use models::{Friendship, Nearby, User};
use postgis::ewkb::Point;
use postgres::types::ToSql;
use std::sync::Mutex;

#[derive(Debug, Clone, Deserialize)]
pub struct NewUser {
    username: String,
    loc: Option<Loc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Loc {
    lon: f64,
    lat: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Update {
    loc: Loc,
    completion: i32,
}

/// Register a new account
/// Maybe add location to registration aswell
pub fn register(
    (user, state): (Json<NewUser>, State<AppState>),
) -> Box<Future<Item = impl Responder, Error = Error>> {
    let user = user.into_inner();
    if user.loc.is_none() {
        return Box::new(err(format_err!("No Location Provided").into()));
    }
    let pos = user.loc.unwrap();
    let query = "INSERT INTO users (username, last_location) VALUES ($1, $2) RETURNING username, token, last_location, last_online, completion";
    let params = Mutex::new(vec![
        Box::new(user.username) as Box<ToSql + Send>,
        Box::new(Point::new(pos.lon, pos.lat, None)),
    ]);
    Box::new(
        state
            .db
            .send(Query { query, params })
            .from_err()
            .and_then(result)
            .map_err(From::from)
            .map(|rows| {
                if rows.is_empty() {
                    HttpResponse::Conflict().json(json! ({
                        "error": "Could not insert user"
                    }))
                } else {
                    let row = rows.get(0);
                    HttpResponse::Ok().json(User {
                        username: row.get(0),
                        token: row.get(1),
                        last_location: row.get(2),
                        last_online: row.get(3),
                        completion: row.get(4),
                    })
                }
            }),
    )
}

/// Add or accept a friendrequest
pub fn addfriend(
    (user, state, mut req): (Json<NewUser>, State<AppState>, HttpRequest<AppState>),
) -> impl Future<Item = impl Responder, Error = Error> {
    // unwrap is ok here because this can only get reached after the middleware was successful
    // remove because we get an owned value not a borrow
    let source = req.extensions_mut().remove::<User>().unwrap();
    let target = user.into_inner();
    // 3 different cases here:
    // either there is already a friend request and we can accept it
    // or there is no friend request and we need to create one
    // or they are already friends and this will do nothing
    // will error if source == target
    // will do nothing if source == already existing source
    let query = r#"
        insert into friendship (source, target) values ($1, $2)
        on conflict (greatest(source, target), least(source, target)) do update
        set accepted_at = least(now(), friendship.accepted_at)
        where friendship.source <> $1
    "#;
    let params = Mutex::new(vec![
        Box::new(source.username) as Box<ToSql + Send>,
        Box::new(target.username),
    ]);

    state
        .db
        .send(Query { query, params })
        .from_err()
        .and_then(result)
        .map(|_| {
            HttpResponse::Ok().json(json!({
                "msg": "Success"
            }))
        })
        .map_err(From::from)
}

/// Get all friends (accepted and unaccepted)
pub fn friends(
    (state, mut req): (State<AppState>, HttpRequest<AppState>),
) -> impl Future<Item = impl Responder, Error = Error> {
    let user = req.extensions_mut().remove::<User>().unwrap();

    let query = r#"
        select source, target, accepted_at from friendship
        where source = $1
           or target = $1
    "#;
    let params = Mutex::new(vec![Box::new(user.username) as Box<ToSql + Send>]);

    state
        .db
        .send(Query { query, params })
        .from_err()
        .and_then(result)
        .map(|rows| {
            rows.into_iter()
                .map(|row| Friendship {
                    source: row.get(0),
                    target: row.get(1),
                    accepted: row.get_opt(2).and_then(Result::ok),
                })
                .collect::<Vec<_>>()
        })
        .map(|r| HttpResponse::Ok().json(json!({ "friends": r })))
        .map_err(From::from)
}

pub fn weather(loc: Q<Loc>) -> impl Future<Item = impl Responder, Error = Error> {
    let api = ::std::env::var("OWA_API_KEY").expect("No Weather API Key set");

    client::ClientRequest::get(format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&APPID={}",
        loc.lat, loc.lon, api
    )).finish()
        .unwrap()
        .send()
        .map_err(From::from)
        .and_then(|resp| {
            resp.body().from_err().and_then(|body| {
                Ok(HttpResponse::Ok()
                    .header("Content-Type", "application/json")
                    .body(body))
            })
        })
}

pub fn nearby(
    (state, mut req): (State<AppState>, HttpRequest<AppState>),
) -> impl Future<Item = impl Responder, Error = Error> {
    let user = req.extensions_mut().remove::<User>().unwrap();
    let query = r#"
        select u1.username, u1.last_location, u1.last_online
        from users u1, users u2
        where u2.token = $1
          and ST_DWithin(u1.last_location, u2.last_location, 1000)
          and u1.last_online + '5 minutes'::interval > now()
        order by ST_Distance(u1.last_location, u2.last_location)
    "#;
    let params = Mutex::new(vec![Box::new(user.token) as Box<ToSql + Send>]);

    state
        .db
        .send(Query { query, params })
        .from_err()
        .and_then(result)
        .map_err(From::from)
        .map(|rows| {
            rows.into_iter()
                .map(|row| Nearby {
                    username: row.get(0),
                    last_location: row.get(1),
                    last_online: row.get(2),
                })
                .collect::<Vec<_>>()
        })
        .map(|res| HttpResponse::Ok().json(json!({ "nearby": res })))
}

pub fn update(
    (state, upd, mut req): (State<AppState>, Json<Update>, HttpRequest<AppState>),
) -> impl Future<Item = impl Responder, Error = Error> {
    let user = req.extensions_mut().remove::<User>().unwrap();
    let upd = upd.into_inner();

    let query = r#"
        update users set
            last_location = $1,
            completion = $2
            last_online = now()
        where token = $3;
    "#;
    let params = Mutex::new(vec![
        Box::new(Point::new(upd.loc.lon, upd.loc.lat, None)) as Box<ToSql + Send>,
        Box::new(upd.completion),
        Box::new(user.token),
    ]);
    state
        .db
        .send(Query { query, params })
        .from_err()
        .and_then(result)
        .map_err(From::from)
        .map(|_| HttpResponse::Ok().json(json!({ "msg": "success" })))
}
