use super::AppState;
use actix_web::http::header;
use actix_web::middleware::{Middleware, Started};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Result};
use db::Query;
use futures::{future::result, Future};
use models::User;
use postgres::types::ToSql;
use uuid::Uuid;

pub struct AuthMiddleware;

impl Middleware<AppState> for AuthMiddleware {
    fn start(&self, req: &mut HttpRequest<AppState>) -> Result<Started> {
        if let Some(token) = req.headers().get(header::AUTHORIZATION) {
            info!("Token: {:?}", token);
            let token = token
                .to_str()
                .map_err(|e| -> ::failure::Error { e.into() })?;
            if !token.starts_with("Bearer ") {
                return Err(format_err!("Invalid Authorization Header").into());
            }
            let token = token[7..]
                .parse::<Uuid>()
                .map_err(|e| -> ::failure::Error { e.into() })?; // needs copy otherwise does not live long enough

            let query = "update users set last_online = now() where token = $1 returning username, token, last_location, last_online, completion";
            let params = vec![Box::new(token) as Box<ToSql + Send>];
            let mut req = req.clone();
            let future = req.state()
                .db
                .send(Query { query, params })
                .from_err()
                .and_then(result)
                .map_err(From::from)
                .map(move |rows| {
                    if rows.is_empty() {
                        return Some(HttpResponse::Unauthorized().json(json!({
                                "error": "Token not found"
                            })));
                    }
                    let row = rows.get(0);
                    let u = User {
                        username: row.get(0),
                        token: row.get(1),
                        last_location: row.get(2),
                        last_online: row.get(3),
                        completion: row.get(4),
                    };
                    req.extensions_mut().insert(u);
                    None
                });

            Ok(Started::Future(Box::new(future) as Box<Future<Item = _, Error = _>>))
        } else {
            Err(format_err!("Not logged in").into()) // TODO: Change to 401 unauthorized
        }
    }
}
