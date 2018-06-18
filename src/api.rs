use super::AppState;
use actix_web::{Error, HttpResponse, Json, Responder, State, HttpRequest};
use db::Query;
use futures::{future::result, Future};
use models::{Friendship, User};
use postgres::types::ToSql;
use std::sync::Mutex;

#[derive(Debug, Clone, Deserialize)]
pub struct NewUser {
    username: String,
}

/// Register a new account
pub fn register(
    (user, state): (Json<NewUser>, State<AppState>),
) -> impl Future<Item = impl Responder, Error = Error> {
    let user = user.into_inner();
    let query = "INSERT INTO users (username) VALUES ($1) RETURNING username, token, last_location, last_online, completion";
    let params = Mutex::new(vec![Box::new(user.username) as Box<ToSql + Send>]);
    state
        .db
        .send(Query { query, params })
        .from_err()
        .and_then(result)
        .map_err(From::from)
        .map(|rows| {
            if rows.is_empty() {
                HttpResponse::Conflict().json(
                    json! ({
                        "error": "Could not insert user"
                    })
                )
            } else {
                let row = rows.get(0);
                HttpResponse::Ok().json(User {
                    username: row.get(0),
                    token: row.get(1),
                    last_location: None,
                    last_online: row.get(3),
                    completion: row.get(4),
                })
            }
        })
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
    let query = r#"
        insert into friendship (source, target) values ($1, $2)
        on conflict (greatest(source, target), least(source, target)) do update
        set accepted_at = least(now(), friendship.accepted_at)
    "#;
    let params = Mutex::new(vec![Box::new(source.username) as Box<ToSql + Send>, Box::new(target.username)]);

    state
        .db
        .send(Query { query, params })
        .from_err()
        .and_then(result)
        .map(|_| HttpResponse::Ok().json(
            json!({
                "msg": "Success"
            })
        ))
        .map_err(From::from)
}

/// Get all friends (accepted and unaccepted)
pub fn friends(
    (state, mut req): (State<AppState>, HttpRequest<AppState>)
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
        .map(|rows| rows.into_iter().map(|row| Friendship {
            source: row.get(0),
            target: row.get(1),
            accepted: row.get_opt(2).and_then(Result::ok)
        }).collect::<Vec<_>>())
        .map(|r| HttpResponse::Ok().json(
            json!({
                "friends": r
            })
        ))
        .map_err(From::from)
}