use actix::prelude::*;
use failure::Error;
use postgres::{rows::Rows, types::ToSql};
use r2d2::{self, PooledConnection};
use r2d2_postgres::PostgresConnectionManager;

pub type Pool = r2d2::Pool<PostgresConnectionManager>;
pub type Connection = PooledConnection<PostgresConnectionManager>;

/// This is db executor actor. We are going to run 3 of them in parallel.
pub struct DbExecutor(pub Pool);

pub struct Query {
    pub query: &'static str,
    pub params: Vec<Box<ToSql + Send>>,
}

impl Message for Query {
    type Result = Result<Rows, Error>;
}

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

impl Handler<Query> for DbExecutor {
    type Result = Result<Rows, Error>;

    fn handle(&mut self, msg: Query, _: &mut Self::Context) -> Self::Result {
        let conn: Connection = self.0.get()?;
        conn.query(
            msg.query,
            &msg.params
                .iter()
                .map(|p| p.as_ref() as &ToSql)
                .collect::<Vec<_>>(),
        ).map_err(From::from)
    }
}
